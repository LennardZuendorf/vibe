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

fn fail(cx: &crate::api::Execution<'_>, msg: &str) -> (u8, Vec<u8>) {
    diag!(cx, "{msg}");
    (exit::FINDINGS, Vec::new())
}

/// Reads a file the oracle would read. `NotFound` keeps the oracle's empty
/// content; any other error refuses (exit 1) so nothing is truncated.
fn read_or_fail(
    cx: &crate::api::Execution<'_>,
    path: &Path,
    show: &str,
) -> Result<Vec<u8>, (u8, Vec<u8>)> {
    match std::fs::read(path) {
        Ok(b) => Ok(b),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(e) => Err(fail(
            cx,
            &format!("promote: ERROR: cannot read {show}: {e}"),
        )),
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

fn refuse(cx: &crate::api::Execution<'_>, msg: &str) -> (u8, Vec<u8>) {
    diag!(cx, "promote: refused \u{2014} {msg}; nothing deleted");
    (exit::FINDINGS, Vec::new())
}

/// The bound artifacts of a valid feature name; `Err` is a finished
/// refusal (an unreadable directory fails closed, P3c-R13).
fn bound(
    cx: &crate::api::Execution<'_>,
    root: &Root,
    feature: &str,
) -> Result<Vec<PathBuf>, (u8, Vec<u8>)> {
    superpowers::artifacts(&root.project, feature).map_err(|e| {
        fail(
            cx,
            &format!("promote: ERROR: cannot read superpowers artifacts: {e}"),
        )
    })
}

/// `promote <f> --done` (D46, P3c-R13): delete the bound artifacts once the
/// spec root changed since the branch base; fail closed otherwise. The
/// command promotes nothing itself, so there are no spec writes to order
/// against: everything is checked first, then the files are removed.
fn done(
    cx: &crate::api::Execution<'_>,
    found: &Result<Root, Miss>,
    a: &Args,
    planned: Option<&mut Vec<PathBuf>>,
) -> (u8, Vec<u8>) {
    let root = match found {
        Ok(root) => root,
        Err(miss) => {
            diag!(cx, "{}", miss.message());
            return (miss.code(), Vec::new());
        }
    };
    if !valid_name(&a.feature) {
        return refuse(
            cx,
            &format!("invalid feature name {:?} (lowercase-kebab)", a.feature),
        );
    }
    let artifacts = match bound(cx, root, &a.feature) {
        Ok(v) => v,
        Err(r) => return r,
    };
    if artifacts.is_empty() {
        return fail(
            cx,
            &format!(
                "promote: ERROR: no superpowers artifacts for '{}'",
                a.feature
            ),
        );
    }
    let Some(base) = git::branch_base(cx, &root.project, a.base.as_deref()) else {
        return refuse(cx, "no branch base found (pass --base <ref>)");
    };
    let short = base.get(..7).unwrap_or(&base);
    match git::changed_since(cx, &root.dir, &base) {
        Some(true) => {}
        Some(false) => {
            return refuse(
                cx,
                &format!(
                    "{}/ is unchanged since {short}; promote the artifacts' durable facts first",
                    root.display()
                ),
            );
        }
        None => {
            return refuse(
                cx,
                &format!("cannot compare {}/ with {short}", root.display()),
            );
        }
    }
    let shown: Vec<String> = artifacts.iter().map(|p| rel(&root.project, p)).collect();
    let mut deleted: Vec<String> = Vec::new();
    let mut failed = false;
    if !a.dry_run
        && let Some(planned) = planned
    {
        planned.extend(artifacts.clone());
        deleted = shown.clone();
    } else if !a.dry_run {
        for (path, s) in artifacts.iter().zip(&shown) {
            match std::fs::remove_file(path) {
                Ok(()) => deleted.push(s.clone()),
                Err(e) => {
                    failed = true;
                    diag!(cx, "promote: ERROR: not deleted {s}: {e}");
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
pub fn run(
    cx: &crate::api::Execution<'_>,
    found: &Result<Root, Miss>,
    cwd: &Path,
    args: &[String],
) -> (u8, Vec<u8>) {
    run_inner(cx, found, cwd, args, None)
}
fn run_inner(
    cx: &crate::api::Execution<'_>,
    found: &Result<Root, Miss>,
    cwd: &Path,
    args: &[String],
    mut planned: Option<(&mut Vec<Write>, &mut Vec<PathBuf>)>,
) -> (u8, Vec<u8>) {
    if crate::wants_help(args) {
        return (exit::OK, HELP.as_bytes().to_vec());
    }
    let a = match parse(args) {
        Ok(a) => a,
        Err(msg) => {
            diag!(cx, "{msg}");
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
            diag!(cx, "{}", miss.message());
            return (miss.code(), Vec::new());
        }
    };
    if a.done {
        return done(cx, found, &a, planned.map(|(_, deletes)| deletes));
    }
    let sp: Vec<PathBuf> = match found {
        Ok(root) if valid_name(&a.feature) => match bound(cx, root, &a.feature) {
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
        return fail(cx, &format!("ERROR: {tech_show} not found"));
    }
    if !target.is_file() {
        return fail(cx, &format!("ERROR: {target_show} not found"));
    }
    let tech_bytes = match read_or_fail(cx, &tech, &tech_show) {
        Ok(b) => b,
        Err(r) => return r,
    };
    let blocks = match merge_blocks(&tech_bytes, &tech_show) {
        Ok(b) => b,
        Err(msg) => return fail(cx, &msg),
    };
    let product_path = tech.with_file_name("product.md");
    let product_show = format!("{display}/features/{}/product.md", a.feature);
    let product = match read_or_fail(cx, &product_path, &product_show) {
        Ok(b) => Some(b).filter(|t| delta::has_delta(t)),
        Err(r) => return r,
    };
    let mut out: Vec<u8> = Vec::new();
    let mut writes: Vec<Write> = Vec::new();
    let mut merged: Option<(usize, usize)> = None;
    if blocks.is_empty() {
        diag!(cx, "WARN: no <!-- merge --> blocks found in {tech_show}");
        if product.is_none() && sp_out.is_none() {
            if a.json {
                let v = json_base(&a, &[], &[]);
                return (exit::OK, format!("{v}\n").into_bytes());
            }
            return (exit::OK, Vec::new());
        }
    } else {
        let current = match read_or_fail(cx, &target, &target_show) {
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
                        diag!(cx, "{l}");
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
            return fail(
                cx,
                &format!(
                    "promote: ERROR: {} is a symlink \u{2014} refusing to write through it",
                    w.show
                ),
            );
        }
    }
    if !a.dry_run {
        if let Some((planned, _)) = planned.as_mut() {
            planned.append(&mut writes);
        } else if let Err(msg) = vibe_core::fs::commit(&writes) {
            return fail(cx, &format!("promote: ERROR: {msg}"));
        }
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

#[derive(Clone, Debug)]
pub struct PromoteRequest {
    pub selector: crate::ContextSelector,
    pub feature: String,
    pub expected_revision: String,
    pub dry_run: bool,
    pub done: bool,
    /// A logical spec/... target. Only the legacy repo adapter accepts host targets.
    pub target: Option<String>,
    /// Legacy repo-first code-history base; global history never consumes it.
    pub base: Option<String>,
}
fn managed_plan(
    api: &crate::SpecApi,
    context: &crate::SpecContext,
    request: &PromoteRequest,
) -> Result<(Vec<crate::documents::Change>, Value), crate::SpecError> {
    let err = |message: String| crate::SpecError::new("invalid_promotion", message);
    if !valid_name(&request.feature) {
        return Err(err("invalid feature name".into()));
    }
    let files = crate::documents::collect(context)?;
    let artifacts = crate::query::bound_files(&files, &request.feature);
    if request.done {
        let code = context.require_code()?;
        if artifacts.is_empty() {
            return Err(err("no bound feature artifacts".into()));
        }
        let baseline = if context.mode == crate::StorageMode::GlobalFirst {
            crate::transaction::feature_baseline(api, context, &request.feature)?
        } else {
            let cx = crate::api::Execution::new(&api.runtime);
            let base = git::branch_base(&cx, code, request.base.as_deref()).ok_or_else(|| {
                crate::SpecError::new(
                    "missing_evidence",
                    "no branch base found (pass --base <ref>)",
                )
            })?;
            if git::changed_since(&cx, &context.spec_root, &base) != Some(true) {
                return Err(crate::SpecError::new(
                    "promotion_required",
                    "durable spec is unchanged since branch base",
                ));
            }
            base
        };
        let validation = crate::validate::context_result(
            &api.runtime,
            context,
            crate::ValidationScope::WithCode,
            false,
        )?;
        if validation.errors > 0 {
            let mut e =
                crate::SpecError::new("invalid_spec", "completion requires valid spec data");
            e.details = validation.value();
            return Err(e);
        }
        let changes = artifacts
            .iter()
            .map(|p| crate::documents::Change {
                path: p.clone(),
                content: None,
            })
            .collect();
        return Ok((
            changes,
            json!({"v":1,"feature":request.feature,"dry_run":request.dry_run,"artifacts":artifacts,"deleted":if request.dry_run{vec![]}else{artifacts.clone()},"baseline":baseline}),
        ));
    }
    let tech_path = format!("spec/features/{}/tech.md", request.feature);
    let Some(tech) = files.get(&tech_path) else {
        if artifacts.is_empty() {
            return Err(err("no feature to promote".into()));
        }
        return Ok((
            Vec::new(),
            json!({"v":1,"feature":request.feature,"dry_run":request.dry_run,"artifacts":artifacts,"deleted":[],"checklist":[{"what":"chosen approach","target":"spec/decisions/"},{"what":"new nouns","target":"spec/concepts/"},{"what":"built places (+code:)","target":"spec/areas/"},{"what":"rules","target":"spec/tech/"},{"what":"failures","target":"spec/lessons.md"}]}),
        ));
    };
    let target = request.target.as_deref().unwrap_or("spec/tech.md");
    if !target.starts_with("spec/") {
        return Err(err(
            "promotion target must be a logical spec document".into()
        ));
    }
    let target_path = crate::documents::path(context, target)?;
    let current = files
        .get(target)
        .ok_or_else(|| err("promotion target is missing".into()))?;
    let blocks = merge_blocks(tech, &tech_path).map_err(err)?;
    let merge = merge_plan(current, &blocks);
    let mut writes = Vec::new();
    let mut pending = current.clone();
    pending.extend(&merge.append);
    if merge.appended > 0 {
        writes.push(crate::documents::Change {
            path: target.into(),
            content: Some(pending.clone()),
        });
    }
    let mut result = json!({"v":1,"feature":request.feature,"dry_run":request.dry_run,"artifacts":artifacts,"deleted":[],"merge":{"source":tech_path,"target":target,"appended":merge.appended,"skipped":merge.skipped}});
    if let Some(product) = files
        .get(&format!("spec/features/{}/product.md", request.feature))
        .filter(|p| delta::has_delta(p))
    {
        let plan = plan_delta(
            &context.spec_root,
            "spec",
            &request.feature,
            product,
            Some((&target_path, &pending)),
        )
        .map_err(|e| err(e.join("\n")))?;
        let logical = format!(
            "spec/{}",
            plan.dest
                .strip_prefix(&context.spec_root)
                .map_err(|_| err("delta target escapes spec".into()))?
                .to_string_lossy()
                .replace('\\', "/")
        );
        crate::documents::path(context, &logical)?;
        if plan.changes.iter().any(|c| c.applied) {
            writes.retain(|w| w.path != logical);
            writes.push(crate::documents::Change {
                path: logical.clone(),
                content: Some(plan.bytes),
            });
        }
        result["delta"] = json!({"source":plan.product_show,"target":logical,"changes":plan.changes.iter().map(|c|json!({"op":c.kind.as_str(),"name":crate::text::lossy(&c.name),"applied":c.applied})).collect::<Vec<_>>()});
    }
    Ok((writes, result))
}
pub(crate) fn managed(
    api: &crate::SpecApi,
    request: PromoteRequest,
) -> Result<crate::SpecResponse, crate::SpecError> {
    if request.dry_run {
        let (context, data) =
            crate::transaction::read_preview(api, &request.selector, |context| {
                Ok(managed_plan(api, context, &request)?.1)
            })?;
        return Ok(crate::SpecResponse {
            context: Some(context),
            data,
        });
    }
    let completed = if request.done {
        vec![request.feature.clone()]
    } else {
        Vec::new()
    };
    crate::transaction::apply_reported_planned(
        api,
        &request.selector,
        &request.expected_revision,
        &completed,
        |context| {
            let (changes, data) = managed_plan(api, context, &request)?;
            Ok((changes, crate::transaction::PlannedResponse::Wrapped(data)))
        },
    )
}

pub(crate) fn managed_cli(
    api: &crate::SpecApi,
    selector: &crate::ContextSelector,
    args: &[String],
) -> Result<(u8, Vec<u8>), crate::SpecError> {
    if crate::wants_help(args) {
        return Ok((0, HELP.as_bytes().to_vec()));
    }
    let mut legacy = Vec::new();
    let mut expected = None;
    let mut i = 0;
    while let Some(arg) = args.get(i) {
        if arg == "--expect" {
            i += 1;
            expected = Some(
                args.get(i)
                    .ok_or_else(|| {
                        crate::SpecError::new("invalid_request", "--expect requires a revision")
                    })?
                    .clone(),
            );
        } else {
            legacy.push(arg.clone());
        }
        i += 1;
    }
    let a = parse(&legacy).map_err(|e| crate::SpecError::new("invalid_request", e))?;
    let expected = match expected {
        Some(v) => v,
        None if a.dry_run => String::new(),
        None => {
            return Err(crate::SpecError::new(
                "expected_revision_required",
                "global promotion requires --expect REVISION",
            ));
        }
    };
    let response = managed(
        api,
        PromoteRequest {
            selector: selector.clone(),
            feature: a.feature,
            expected_revision: expected,
            dry_run: a.dry_run,
            done: a.done,
            target: a.target,
            base: a.base,
        },
    )?;
    let mut data = response.data;
    if let Some(map) = data.as_object_mut() {
        map.remove("receipt");
        map.remove("baseline");
    }
    if a.json {
        return Ok((0, format!("{data}\n").into_bytes()));
    }
    let mut out = String::new();
    if a.done {
        for path in data["artifacts"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
        {
            out += &format!(
                "{} {path}\n",
                if a.dry_run { "WOULD DELETE" } else { "DELETED" }
            );
        }
    } else {
        if let Some(merge) = data.get("merge") {
            out += &format!(
                "{} {} block(s) from {} → {} ({} already present)\n",
                if a.dry_run {
                    "WOULD PROMOTE"
                } else {
                    "PROMOTED"
                },
                merge["appended"],
                merge["source"].as_str().unwrap_or_default(),
                merge["target"].as_str().unwrap_or_default(),
                merge["skipped"]
            );
        }
        for item in data["checklist"].as_array().into_iter().flatten() {
            out += &format!(
                "  [ ] {} → {}\n",
                item["what"].as_str().unwrap_or_default(),
                item["target"].as_str().unwrap_or_default()
            );
        }
    }
    Ok((0, out.into_bytes()))
}

pub(crate) fn repo_cli(
    api: &crate::SpecApi,
    selector: &crate::ContextSelector,
    cx: &crate::api::Execution<'_>,
    found: &Result<Root, Miss>,
    cwd: &Path,
    args: &[String],
) -> Result<(u8, Vec<u8>), crate::SpecError> {
    let parsed = match parse(args) {
        Ok(a) => a,
        Err(_) => return Ok(run(cx, found, cwd, args)),
    };
    let Ok(root) = found else {
        return Ok(run(cx, found, cwd, args));
    };
    let roots = crate::context::LegacyRoots::from_root(root)?;
    // Diagnostic-only admission preserves legacy refusals. The transaction discards
    // these bytes and recomputes every successful write plan under its lock.
    if !roots
        .code_anchor
        .ancestors()
        .any(|p| p.join(".vibe/run/spec-transaction.json").exists())
    {
        let preflight = crate::api::Execution::new(cx.runtime);
        let mut ignored = Vec::new();
        let mut deleted = Vec::new();
        let answer = run_inner(
            &preflight,
            found,
            cwd,
            args,
            Some((&mut ignored, &mut deleted)),
        );
        if answer.0 != 0 {
            cx.diagnostic_bytes(&preflight.take_diagnostics());
            return Ok(answer);
        }
    }
    if parsed.done && !parsed.dry_run {
        return repo_done(api, selector, &roots, cx, found, cwd, args, &parsed);
    }
    if parsed.dry_run || crate::wants_help(args) {
        return Ok(
            crate::transaction::read_legacy_preview(api, selector, &roots, |_| {
                Ok(run(cx, found, cwd, args))
            })?
            .1,
        );
    }
    if let Some(target) = parsed.target.as_ref().filter(|_| !parsed.done) {
        let context = crate::context::resolve_for_legacy(&api.runtime, selector, &roots)?;
        if crate::documents::logical_legacy(&context, &crate::layered::lexical(&cwd.join(target)))
            .is_err()
        {
            return Ok(run(cx, found, cwd, args));
        }
    }
    let mut output = None;
    let result = crate::transaction::apply_captured_planned(api, selector, &roots, |context| {
        let mut writes = Vec::new();
        let mut deletes = Vec::new();
        let answer = run_inner(cx, found, cwd, args, Some((&mut writes, &mut deletes)));
        let failed = answer.0 != 0;
        output = Some(answer);
        if failed {
            return Err(crate::SpecError::new(
                "legacy_refusal",
                "legacy promotion refused",
            ));
        }
        let mut changes: Vec<_> = writes
            .into_iter()
            .map(|w| {
                Ok(crate::documents::Change {
                    path: crate::documents::logical_legacy(context, &w.path)?,
                    content: Some(w.bytes),
                })
            })
            .collect::<Result<_, crate::SpecError>>()?;
        for path in deletes {
            changes.push(crate::documents::Change {
                path: crate::documents::logical_legacy(context, &path)?,
                content: None,
            });
        }
        Ok(changes)
    });
    match result {
        Ok(_) => Ok(output.unwrap_or((0, Vec::new()))),
        Err(error) if error.code == "legacy_refusal" => Ok(output.unwrap_or((1, Vec::new()))),
        Err(error) if error.code == "document_io" => {
            let show = |logical: &str| -> String {
                if let Some(relative) = logical.strip_prefix("spec/") {
                    format!("{}/{relative}", root.display())
                } else if let Some(relative) = logical.strip_prefix("superpowers/") {
                    format!("docs/superpowers/{relative}")
                } else {
                    logical.into()
                }
            };
            let target = error.details["path"]
                .as_str()
                .map(show)
                .unwrap_or_else(|| "managed documents".into());
            let restored: Vec<_> = error.details["restored"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .map(show)
                .collect();
            let suffix = if restored.is_empty() {
                String::new()
            } else {
                format!(" — restored {}", restored.join(", "))
            };
            Ok(fail(
                cx,
                &format!("promote: ERROR: {target}: {}{suffix}", error.message),
            ))
        }
        Err(error) => Err(error),
    }
}

#[allow(clippy::too_many_arguments)]
fn repo_done(
    api: &crate::SpecApi,
    selector: &crate::ContextSelector,
    roots: &crate::context::LegacyRoots,
    cx: &crate::api::Execution<'_>,
    found: &Result<Root, Miss>,
    cwd: &Path,
    args: &[String],
    parsed: &Args,
) -> Result<(u8, Vec<u8>), crate::SpecError> {
    let root = found
        .as_ref()
        .map_err(|miss| crate::SpecError::new("spec_root_unavailable", miss.message()))?;
    let (context, (answer, deletes)) =
        crate::transaction::read_legacy(api, selector, roots, |_| {
            let mut writes = Vec::new();
            let mut deletes = Vec::new();
            let answer = run_inner(cx, found, cwd, args, Some((&mut writes, &mut deletes)));
            Ok((answer, deletes))
        })?;
    if answer.0 != 0 {
        return Ok(answer);
    }
    let shown: Vec<_> = deletes.iter().map(|p| rel(&root.project, p)).collect();
    let mut deleted = Vec::new();
    let mut expected = context.revision_token;
    let mut failed = false;
    for (index, (path, show)) in deletes.iter().zip(&shown).enumerate() {
        let result =
            crate::transaction::apply_legacy_expected(api, selector, roots, &expected, |context| {
                Ok(vec![crate::documents::Change {
                    path: crate::documents::logical_legacy(context, path)?,
                    content: None,
                }])
            });
        match result {
            Ok(receipt) => {
                deleted.push(show.clone());
                expected = receipt
                    .context
                    .ok_or_else(|| {
                        crate::SpecError::new(
                            "invalid_context",
                            "legacy removal receipt has no context",
                        )
                    })?
                    .revision_token;
            }
            Err(error) => {
                failed = true;
                diag!(cx, "promote: ERROR: not deleted {show}: {}", error.message);
                if error.code != "document_io" {
                    for remaining in shown.iter().skip(index + 1) {
                        diag!(
                            cx,
                            "promote: ERROR: not deleted {remaining}: context changed; re-run with current context"
                        );
                    }
                    break;
                }
            }
        }
    }
    let output = if parsed.json {
        format!("{}\n",json!({"v":1,"feature":parsed.feature,"dry_run":false,"artifacts":shown,"deleted":deleted})).into_bytes()
    } else {
        deleted
            .iter()
            .map(|p| format!("DELETED {p}\n"))
            .collect::<String>()
            .into_bytes()
    };
    Ok((if failed { 1 } else { 0 }, output))
}
