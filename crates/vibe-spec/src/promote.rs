//! `vibe-spec promote <feature> [--dry-run] [--target <file>]` — port of
//! `spec/scripts/promote.sh` (F18, P3c-R6): `<!-- merge -->` blocks of a
//! fallback feature's tech.md are appended to the root tech doc. Byte
//! parity: messages, the `while read` loop (an unterminated last line is
//! dropped), whole-line markers, abort on the first marker error, empty
//! blocks always present, argument errors exit 1.
use std::path::{Path, PathBuf};

use vibe_core::exit;

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
    if blocks.is_empty() {
        eprintln!("WARN: no <!-- merge --> blocks found in {tech_show}");
        return (exit::OK, Vec::new());
    }
    let current = match read_or_fail(&target, &target_show) {
        Ok(b) => b,
        Err(r) => return r,
    };
    let plan = merge_plan(&current, &blocks);
    if a.dry_run {
        if plan.appended == 0 {
            let line = format!(
                "--- nothing to promote: all {} block(s) already present in {target_show} ---\n",
                blocks.len()
            );
            return (exit::OK, line.into_bytes());
        }
        let mut out = format!(
            "--- would promote {} block(s) to {target_show} ({} already present) ---\n",
            plan.appended, plan.skipped
        )
        .into_bytes();
        out.extend_from_slice(&plan.append);
        return (exit::OK, out);
    }
    if plan.appended == 0 {
        let line = format!(
            "No new blocks to promote — all {} already present in {target_show}\n",
            blocks.len()
        );
        return (exit::OK, line.into_bytes());
    }
    let mut bytes = current;
    bytes.extend_from_slice(&plan.append);
    if let Err(e) = vibe_core::fs::write_atomic(&target, &bytes) {
        return fail(&format!("ERROR: {target_show}: {e}"));
    }
    let line = format!(
        "PROMOTED {} block(s) from {tech_show} → {target_show} ({} already present)\n",
        plan.appended, plan.skipped
    );
    (exit::OK, line.into_bytes())
}
