//! `vibe-spec promote <feature> [--dry-run] [--target <file>] [--done]
//! [--base <ref>] [--json]` — port of
//! `spec/scripts/promote.sh` (F18, P3c-R6): `<!-- merge -->` blocks of a
//! fallback feature's tech.md are appended to the root tech doc. Byte
//! parity: messages, the `while read` loop (an unterminated last line is
//! dropped), whole-line markers, abort on the first marker error, empty
//! blocks always present, argument errors exit 1.
use std::path::{Path, PathBuf};

use serde_json::{Value, json};
use vibe_core::exit;
use vibe_core::fs::Write;

use crate::delta::{self, Change};
use crate::layered::{Layout, detect, lexical};
use crate::query::{rel, valid_name};
use crate::root::{Miss, Root};
use crate::text::{find, join_raw, read_lines};
use crate::{git, superpowers};

const USAGE: &str = "usage: vibe-spec promote <feature-name> [--dry-run] [--target <file>] [--done] [--base <ref>] [--json]\n  --done only deletes the bound superpowers artifacts and ignores --target; --base applies to --done only";

struct Args {
    feature: String,
    dry_run: bool,
    target: Option<String>,
    done: bool,
    base: Option<String>,
    json: bool,
}

fn parse(args: &[String]) -> Result<Args, String> {
    let feature = args
        .first()
        .filter(|f| !f.is_empty())
        .ok_or_else(|| USAGE.to_string())?
        .clone();
    let mut a = Args {
        feature,
        dry_run: false,
        target: None,
        done: false,
        base: None,
        json: false,
    };
    let mut i = 1;
    while let Some(arg) = args.get(i) {
        match arg.as_str() {
            "--dry-run" => a.dry_run = true,
            "--target" => {
                let t = args
                    .get(i + 1)
                    .filter(|t| !t.is_empty())
                    .ok_or_else(|| "promote: --target requires a path".to_string())?;
                a.target = Some(t.clone());
                i += 1;
            }
            "--done" => a.done = true,
            "--json" => a.json = true,
            "--base" => {
                let b = args
                    .get(i + 1)
                    .filter(|b| !b.is_empty() && !b.starts_with('-'))
                    .ok_or_else(|| "promote: --base requires a ref".to_string())?;
                a.base = Some(b.clone());
                i += 1;
            }
            other => return Err(format!("Unknown option: {other}")),
        }
        i += 1;
    }
    Ok(a)
}

/// The merge blocks of a feature tech doc, read the oracle's way.
fn merge_blocks(text: &[u8], show: &str) -> Result<Vec<Vec<u8>>, String> {
    let mut blocks = Vec::new();
    let mut open: Option<Vec<u8>> = None;
    for (i, line) in read_lines(text).into_iter().enumerate() {
        let n = i + 1;
        if line == b"<!-- merge -->" {
            if open.is_some() {
                return Err(format!(
                    "ERROR: nested <!-- merge --> at line {n} in {show}"
                ));
            }
            open = Some(Vec::new());
        } else if line == b"<!-- /merge -->" {
            match open.take() {
                Some(b) => blocks.push(b),
                None => {
                    return Err(format!(
                        "ERROR: <!-- /merge --> without opener at line {n} in {show}"
                    ));
                }
            }
        } else if let Some(b) = open.as_mut() {
            b.extend_from_slice(line);
            b.push(b'\n');
        }
    }
    if open.is_some() {
        return Err(format!("ERROR: unclosed <!-- merge --> block in {show}"));
    }
    Ok(blocks)
}

struct MergePlan {
    append: Vec<u8>,
    appended: usize,
    skipped: usize,
}

/// The oracle's idempotency: a block already in the target (or empty) is
/// skipped; each new one is appended after a newline.
fn merge_plan(target: &[u8], blocks: &[Vec<u8>]) -> MergePlan {
    let mut content = target.to_vec();
    let mut p = MergePlan {
        append: Vec::new(),
        appended: 0,
        skipped: 0,
    };
    for b in blocks {
        if b.is_empty() || find(&content, b).is_some() {
            p.skipped += 1;
            continue;
        }
        p.append.push(b'\n');
        p.append.extend_from_slice(b);
        content.push(b'\n');
        content.extend_from_slice(b);
        p.appended += 1;
    }
    p
}

fn fail(msg: &str) -> (u8, Vec<u8>) {
    eprintln!("{msg}");
    (exit::FINDINGS, Vec::new())
}

/// Reads a file the oracle would read. `NotFound` keeps the oracle's empty
/// content; any other error refuses (exit 1) so nothing is truncated.
fn read_or_fail(path: &Path, show: &str) -> Result<Vec<u8>, (u8, Vec<u8>)> {
    match std::fs::read(path) {
        Ok(b) => Ok(b),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(e) => Err(fail(&format!("promote: ERROR: cannot read {show}: {e}"))),
    }
}

/// Whether two paths name one file: by identity when both exist (a symlinked
/// directory component gives the same file two spellings), else lexically.
fn same_file(a: &Path, b: &Path) -> bool {
    if let (Ok(ma), Ok(mb)) = (std::fs::metadata(a), std::fs::metadata(b)) {
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            return (ma.dev(), ma.ino()) == (mb.dev(), mb.ino());
        }
        #[cfg(not(unix))]
        {
            let _ = (ma, mb);
            return matches!(
                (std::fs::canonicalize(a), std::fs::canonicalize(b)),
                (Ok(x), Ok(y)) if x == y
            );
        }
    }
    lexical(a) == lexical(b)
}

/// The planned requirement changes for one feature.
struct DeltaPlan {
    dest: PathBuf,
    dest_show: String,
    product_show: String,
    bytes: Vec<u8>,
    changes: Vec<Change>,
    /// The target as read from disk; `None` when the plan read a queued merge.
    disk: Option<Vec<u8>>,
}

/// Plans the feature product's delta sections against their target, reading
/// `pending` instead of the disk when a merge write to the same file is
/// queued. Err holds the stderr lines of a refusal.
fn plan_delta(
    dir: &Path,
    display: &str,
    feature: &str,
    product: &[u8],
    pending: Option<(&Path, &[u8])>,
) -> Result<DeltaPlan, Vec<String>> {
    let product_show = format!("{display}/features/{feature}/product.md");
    let fdir = join_raw(&dir.join("features"), feature.as_bytes());
    let dest = delta::target(dir, &fdir, product)
        .map_err(|e| vec![format!("promote: ERROR: {e} ({product_show})")])?;
    let rel = lexical(&dest)
        .strip_prefix(lexical(dir))
        .map(|r| {
            r.components()
                .map(|c| c.as_os_str().to_string_lossy().into_owned())
                .collect::<Vec<_>>()
                .join("/")
        })
        .unwrap_or_default();
    let dest_show = format!("{display}/{rel}");
    let (base, disk) = match pending {
        Some((path, bytes)) if same_file(path, &dest) => (bytes.to_vec(), None),
        _ if dest.is_file() => {
            let b = std::fs::read(&dest)
                .map_err(|e| vec![format!("promote: ERROR: cannot read {dest_show}: {e}")])?;
            (b.clone(), Some(b))
        }
        _ => return Err(vec![format!("promote: ERROR: {dest_show} not found")]),
    };
    let (bytes, changes) = delta::apply(&base, product).map_err(|errs| {
        errs.into_iter()
            .map(|e| format!("promote: ERROR: {e} ({product_show})"))
            .collect::<Vec<_>>()
    })?;
    Ok(DeltaPlan {
        dest,
        dest_show,
        product_show,
        bytes,
        changes,
        disk,
    })
}

/// The stdout lines for a delta plan (P3c-R9).
fn delta_lines(p: &DeltaPlan, dry_run: bool) -> Vec<u8> {
    let applied = p.changes.iter().filter(|c| c.applied).count();
    let skipped = p.changes.len() - applied;
    let mut out = match (dry_run, applied) {
        (true, 0) => format!(
            "--- nothing to apply: all {} requirement change(s) already in {} ---\n",
            p.changes.len(),
            p.dest_show
        ),
        (true, _) => format!(
            "--- would apply {applied} requirement change(s) to {} ({skipped} already applied) ---\n",
            p.dest_show
        ),
        (false, 0) => format!(
            "No requirement changes to apply — all {} already in {}\n",
            p.changes.len(),
            p.dest_show
        ),
        (false, _) => format!(
            "APPLIED {applied} requirement change(s) from {} → {} ({skipped} already applied)\n",
            p.product_show, p.dest_show
        ),
    }
    .into_bytes();
    if dry_run {
        for c in p.changes.iter().filter(|c| c.applied) {
            out.extend_from_slice(c.kind.as_str().as_bytes());
            out.push(b' ');
            out.extend_from_slice(&c.name);
            out.push(b'\n');
        }
    }
    out
}

/// The promotion checklist (spec section 5): what goes where, per layout.
fn checklist(root: &Root) -> Vec<(&'static str, String)> {
    let d = root.display();
    let layered = detect(&root.dir) == Layout::Layered;
    let to = |flat: &str, kinds: &str| {
        if layered {
            format!("{d}/{kinds}")
        } else {
            format!("{d}/{flat}")
        }
    };
    vec![
        ("chosen approach", to("plan.md", "decisions/")),
        ("new nouns", to("product.md", "concepts/")),
        ("built places (+code:)", to("tech.md", "areas/")),
        ("rules", to("tech.md", "tech/")),
        ("failures", to("lessons.md", "lessons.md")),
    ]
}

fn checklist_text(feature: &str, shown: &[String], items: &[(&str, String)]) -> Vec<u8> {
    let mut out = format!("superpowers artifacts for '{feature}':\n");
    for s in shown {
        out += &format!("  {s}\n");
    }
    out += &format!("Promote durable facts, then run: vibe-spec promote {feature} --done\n");
    for (what, target) in items {
        out += &format!("  [ ] {what} \u{2192} {target}\n");
    }
    out.into_bytes()
}

/// Shown artifact paths and the checklist items.
type SpOut<'a> = (Vec<String>, Vec<(&'a str, String)>);

fn refuse(msg: &str) -> (u8, Vec<u8>) {
    eprintln!("promote: refused \u{2014} {msg}; nothing deleted");
    (exit::FINDINGS, Vec::new())
}

/// The bound artifacts of a valid feature name; `Err` is a finished
/// refusal (an unreadable directory fails closed, P3c-R13).
fn bound(root: &Root, feature: &str) -> Result<Vec<PathBuf>, (u8, Vec<u8>)> {
    superpowers::artifacts(&root.project, feature).map_err(|e| {
        fail(&format!(
            "promote: ERROR: cannot read superpowers artifacts: {e}"
        ))
    })
}

/// `promote <f> --done` (D46, P3c-R13): delete the bound artifacts once the
/// spec root changed since the branch base; fail closed otherwise. The
/// command promotes nothing itself, so there are no spec writes to order
/// against: everything is checked first, then the files are removed.
fn done(found: &Result<Root, Miss>, a: &Args) -> (u8, Vec<u8>) {
    let root = match found {
        Ok(root) => root,
        Err(miss) => {
            eprintln!("{}", miss.message());
            return (miss.code(), Vec::new());
        }
    };
    if !valid_name(&a.feature) {
        return refuse(&format!(
            "invalid feature name {:?} (lowercase-kebab)",
            a.feature
        ));
    }
    let artifacts = match bound(root, &a.feature) {
        Ok(v) => v,
        Err(r) => return r,
    };
    if artifacts.is_empty() {
        return fail(&format!(
            "promote: ERROR: no superpowers artifacts for '{}'",
            a.feature
        ));
    }
    let Some(base) = git::branch_base(&root.project, a.base.as_deref()) else {
        return refuse("no branch base found (pass --base <ref>)");
    };
    let short = base.get(..7).unwrap_or(&base);
    match git::changed_since(&root.dir, &base) {
        Some(true) => {}
        Some(false) => {
            return refuse(&format!(
                "{}/ is unchanged since {short}; promote the artifacts' durable facts first",
                root.display()
            ));
        }
        None => {
            return refuse(&format!("cannot compare {}/ with {short}", root.display()));
        }
    }
    let shown: Vec<String> = artifacts.iter().map(|p| rel(&root.project, p)).collect();
    let mut deleted: Vec<String> = Vec::new();
    let mut failed = false;
    if !a.dry_run {
        for (path, s) in artifacts.iter().zip(&shown) {
            match std::fs::remove_file(path) {
                Ok(()) => deleted.push(s.clone()),
                Err(e) => {
                    failed = true;
                    eprintln!("promote: ERROR: not deleted {s}: {e}");
                }
            }
        }
    }
    let out = if a.json {
        let v = json!({
            "v": 1, "feature": a.feature, "dry_run": a.dry_run,
            "artifacts": shown, "deleted": deleted,
        });
        format!("{v}\n")
    } else if a.dry_run {
        shown
            .iter()
            .map(|s| format!("WOULD DELETE {s}\n"))
            .collect()
    } else {
        deleted.iter().map(|s| format!("DELETED {s}\n")).collect()
    };
    // Deleted files cannot come back: stdout names what is gone.
    let code = if failed { exit::FINDINGS } else { exit::OK };
    (code, out.into_bytes())
}

/// The always-present JSON fields, plus the checklist when artifacts exist.
fn json_base(a: &Args, shown: &[String], items: &[(&str, String)]) -> Value {
    let mut v = json!({
        "v": 1, "feature": a.feature, "dry_run": a.dry_run,
        "artifacts": shown, "deleted": Vec::<String>::new(),
    });
    if !items.is_empty() {
        v["checklist"] = Value::Array(
            items
                .iter()
                .map(|(w, t)| json!({ "what": w, "target": t }))
                .collect(),
        );
    }
    v
}

const HELP: &str = "Usage: vibe-spec promote <feature-name> [--dry-run] [--target <file>] [--done] [--base <ref>] [--json]\n\nMerge a feature's durable facts into the root spec.\n  --done only deletes the bound superpowers artifacts and ignores --target; --base applies to --done only\n";

/// Runs the command.
pub fn run(found: &Result<Root, Miss>, cwd: &Path, args: &[String]) -> (u8, Vec<u8>) {
    if crate::wants_help(args) {
        return (exit::OK, HELP.as_bytes().to_vec());
    }
    let a = match parse(args) {
        Ok(a) => a,
        Err(msg) => {
            eprintln!("{msg}");
            return (exit::USAGE, Vec::new());
        }
    };
    // The oracle's paths are cwd-relative under `.spec/`; the port uses the // literal
    // discovered root (a walk up from the cwd, shared by every subcommand: a
    // deliberate divergence) and prints it relative to the project.
    let (dir, display): (PathBuf, String) = match found {
        Ok(root) => (root.dir.clone(), root.display()),
        Err(Miss::NotFound) if !a.json => (cwd.join(".spec"), ".spec".to_string()), // literal
        Err(miss) => {
            eprintln!("{}", miss.message());
            return (miss.code(), Vec::new());
        }
    };
    if a.done {
        return done(found, &a);
    }
    let sp: Vec<PathBuf> = match found {
        Ok(root) if valid_name(&a.feature) => match bound(root, &a.feature) {
            Ok(v) => v,
            Err(r) => return r,
        },
        _ => Vec::new(),
    };
    let sp_out: Option<SpOut> = match found {
        Ok(root) if !sp.is_empty() => Some((
            sp.iter().map(|p| rel(&root.project, p)).collect(),
            checklist(root),
        )),
        _ => None,
    };
    let tech = join_raw(&dir.join("features"), a.feature.as_bytes()).join("tech.md");
    let tech_show = format!("{display}/features/{}/tech.md", a.feature);
    let (target, target_show) = match &a.target {
        Some(t) => (cwd.join(t), t.clone()),
        None => (dir.join("tech.md"), format!("{display}/tech.md")),
    };
    if !tech.is_file()
        && let Some((shown, items)) = &sp_out
    {
        // Superpowers artifacts only: no fallback folder, no writes.
        if a.json {
            let v = json_base(&a, shown, items);
            return (exit::OK, format!("{v}\n").into_bytes());
        }
        return (exit::OK, checklist_text(&a.feature, shown, items));
    }
    if !tech.is_file() {
        return fail(&format!("ERROR: {tech_show} not found"));
    }
    if !target.is_file() {
        return fail(&format!("ERROR: {target_show} not found"));
    }
    let tech_bytes = match read_or_fail(&tech, &tech_show) {
        Ok(b) => b,
        Err(r) => return r,
    };
    let blocks = match merge_blocks(&tech_bytes, &tech_show) {
        Ok(b) => b,
        Err(msg) => return fail(&msg),
    };
    let product_path = tech.with_file_name("product.md");
    let product_show = format!("{display}/features/{}/product.md", a.feature);
    let product = match read_or_fail(&product_path, &product_show) {
        Ok(b) => Some(b).filter(|t| delta::has_delta(t)),
        Err(r) => return r,
    };
    let mut out: Vec<u8> = Vec::new();
    let mut writes: Vec<Write> = Vec::new();
    let mut merged: Option<(usize, usize)> = None;
    if blocks.is_empty() {
        eprintln!("WARN: no <!-- merge --> blocks found in {tech_show}");
        if product.is_none() && sp_out.is_none() {
            if a.json {
                let v = json_base(&a, &[], &[]);
                return (exit::OK, format!("{v}\n").into_bytes());
            }
            return (exit::OK, Vec::new());
        }
    } else {
        let current = match read_or_fail(&target, &target_show) {
            Ok(b) => b,
            Err(r) => return r,
        };
        let plan = merge_plan(&current, &blocks);
        merged = Some((plan.appended, plan.skipped));
        out = match (a.dry_run, plan.appended) {
            (true, 0) => format!(
                "--- nothing to promote: all {} block(s) already present in {target_show} ---\n",
                blocks.len()
            )
            .into_bytes(),
            (true, _) => {
                let mut o = format!(
                    "--- would promote {} block(s) to {target_show} ({} already present) ---\n",
                    plan.appended, plan.skipped
                )
                .into_bytes();
                o.extend_from_slice(&plan.append);
                o
            }
            (false, 0) => format!(
                "No new blocks to promote — all {} already present in {target_show}\n",
                blocks.len()
            )
            .into_bytes(),
            (false, _) => format!(
                "PROMOTED {} block(s) from {tech_show} → {target_show} ({} already present)\n",
                plan.appended, plan.skipped
            )
            .into_bytes(),
        };
        if plan.appended > 0 {
            let mut bytes = current.clone();
            bytes.extend_from_slice(&plan.append);
            writes.push(Write {
                path: target.clone(),
                show: target_show.clone(),
                bytes,
                original: Some(current),
            });
        }
    }
    let delta_plan = match &product {
        Some(doc) => {
            let pending = writes
                .first()
                .map(|w| (w.path.as_path(), w.bytes.as_slice()));
            match plan_delta(&dir, &display, &a.feature, doc, pending) {
                Ok(p) => Some(p),
                Err(lines) => {
                    for l in lines {
                        eprintln!("{l}");
                    }
                    return (exit::FINDINGS, Vec::new());
                }
            }
        }
        None => None,
    };
    if let Some(p) = &delta_plan {
        if !out.is_empty() && !out.ends_with(b"\n") {
            out.push(b'\n');
        }
        out.extend_from_slice(&delta_lines(p, a.dry_run));
    }
    if let Some(p) = delta_plan
        .as_ref()
        .filter(|p| p.changes.iter().any(|c| c.applied))
    {
        match writes.first_mut() {
            // The delta was planned on the queued merge: one final write.
            Some(w) if p.disk.is_none() => w.bytes.clone_from(&p.bytes),
            _ => writes.push(Write {
                path: p.dest.clone(),
                show: p.dest_show.clone(),
                bytes: p.bytes.clone(),
                original: p.disk.clone(),
            }),
        }
    }
    if let Some((shown, items)) = &sp_out {
        if !out.is_empty() && !out.ends_with(b"\n") {
            out.push(b'\n');
        }
        out.extend_from_slice(&checklist_text(&a.feature, shown, items));
    }
    // Never write through a symlink; checked for dry runs too, so a dry run
    // predicts the real run.
    for w in &writes {
        if std::fs::symlink_metadata(&w.path).is_ok_and(|m| m.file_type().is_symlink()) {
            return fail(&format!(
                "promote: ERROR: {} is a symlink \u{2014} refusing to write through it",
                w.show
            ));
        }
    }
    if !a.dry_run
        && let Err(msg) = vibe_core::fs::commit(&writes)
    {
        return fail(&format!("promote: ERROR: {msg}"));
    }
    if a.json {
        let mut v = match &sp_out {
            Some((shown, items)) => json_base(&a, shown, items),
            None => json_base(&a, &[], &[]),
        };
        if let Some((appended, skipped)) = merged {
            v["merge"] = json!({
                "source": tech_show, "target": target_show,
                "appended": appended, "skipped": skipped,
            });
        }
        if let Some(p) = &delta_plan {
            v["delta"] = json!({
                "source": p.product_show, "target": p.dest_show,
                "changes": p.changes.iter().map(|c| json!({
                    "op": c.kind.as_str(),
                    "name": crate::text::lossy(&c.name),
                    "applied": c.applied,
                })).collect::<Vec<_>>(),
            });
        }
        return (exit::OK, format!("{v}\n").into_bytes());
    }
    (exit::OK, out)
}
