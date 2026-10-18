//! `vibe-spec promote <feature> [--dry-run] [--target <file>]` — port of
//! `spec/scripts/promote.sh` (F18, P3c-R6): `<!-- merge -->` blocks of a
//! fallback feature's tech.md are appended to the root tech doc. Byte
//! parity: messages, the `while read` loop (an unterminated last line is
//! dropped), whole-line markers, abort on the first marker error, empty
//! blocks always present, argument errors exit 1.
use std::path::{Path, PathBuf};

use vibe_core::exit;

use crate::delta::{self, Change};
use crate::layered::lexical;
use crate::root::{Miss, Root};
use crate::text::{find, join_raw, read_lines};

const USAGE: &str = "usage: vibe-spec promote <feature-name> [--dry-run] [--target <file>]";

struct Args {
    feature: String,
    dry_run: bool,
    target: Option<String>,
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
        .map_err(|e| vec![format!("ERROR: {e} ({product_show})")])?;
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
        Some((path, bytes)) if lexical(path) == lexical(&dest) => (bytes.to_vec(), None),
        _ if dest.is_file() => {
            let b = std::fs::read(&dest)
                .map_err(|e| vec![format!("promote: ERROR: cannot read {dest_show}: {e}")])?;
            (b.clone(), Some(b))
        }
        _ => return Err(vec![format!("promote: ERROR: {dest_show} not found")]),
    };
    let (bytes, changes) = delta::apply(&base, product).map_err(|errs| {
        errs.into_iter()
            .map(|e| format!("ERROR: {e} ({product_show})"))
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

/// One planned write with the bytes to put back if a later write fails.
struct Write {
    path: PathBuf,
    show: String,
    bytes: Vec<u8>,
    original: Option<Vec<u8>>,
}

/// Performs the writes in order. If write k fails, writes 0..k are put back
/// (or removed when the file did not exist) and the refusal is returned.
fn commit(writes: &[Write]) -> Result<(), String> {
    for (k, w) in writes.iter().enumerate() {
        let Err(e) = vibe_core::fs::write_atomic(&w.path, &w.bytes) else {
            continue;
        };
        let mut msg = format!("promote: ERROR: {}: {e}", w.show);
        let mut restored = Vec::new();
        let mut failed = Vec::new();
        for done in &writes[..k] {
            let back = match &done.original {
                Some(b) => vibe_core::fs::write_atomic(&done.path, b).map_err(|e| e.to_string()),
                None => std::fs::remove_file(&done.path).map_err(|e| e.to_string()),
            };
            match back {
                Ok(()) => restored.push(done.show.as_str()),
                Err(err) => failed.push(format!("{}: {err}", done.show)),
            }
        }
        if !restored.is_empty() {
            msg.push_str(&format!(" — restored {}", restored.join(", ")));
        }
        for f in failed {
            msg.push_str(&format!(" — restore failed for {f}"));
        }
        return Err(msg);
    }
    Ok(())
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

/// Runs the command.
pub fn run(found: &Result<Root, Miss>, cwd: &Path, args: &[String]) -> (u8, Vec<u8>) {
    let a = match parse(args) {
        Ok(a) => a,
        Err(msg) => return fail(&msg),
    };
    // The oracle's paths are cwd-relative under `.spec/`; the port uses the
    // discovered root (a walk up from the cwd, shared by every subcommand: a
    // deliberate divergence) and prints it relative to the project.
    let (dir, display): (PathBuf, String) = match found {
        Ok(root) => (root.dir.clone(), root.display()),
        Err(Miss::NotFound) => (cwd.join(".spec"), ".spec".to_string()),
        Err(miss) => {
            eprintln!("{}", miss.message());
            return (miss.code(), Vec::new());
        }
    };
    let tech = join_raw(&dir.join("features"), a.feature.as_bytes()).join("tech.md");
    let tech_show = format!("{display}/features/{}/tech.md", a.feature);
    let (target, target_show) = match &a.target {
        Some(t) => (cwd.join(t), t.clone()),
        None => (dir.join("tech.md"), format!("{display}/tech.md")),
    };
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
    if blocks.is_empty() {
        eprintln!("WARN: no <!-- merge --> blocks found in {tech_show}");
        if product.is_none() {
            return (exit::OK, Vec::new());
        }
    } else {
        let current = match read_or_fail(&target, &target_show) {
            Ok(b) => b,
            Err(r) => return r,
        };
        let plan = merge_plan(&current, &blocks);
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
    if !a.dry_run
        && let Err(msg) = commit(&writes)
    {
        return fail(&msg);
    }
    (exit::OK, out)
}
