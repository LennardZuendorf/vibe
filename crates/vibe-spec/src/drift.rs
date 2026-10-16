//! `vibe-spec check-drift` — port of `spec/scripts/check-drift.sh` (F18,
//! P3c-R3). For a `.spec` root the output is the oracle's, byte for byte:
//! stdout carries only the final OK line, everything else goes to stderr.
//! Paths print relative to the project through `Root::display()`.
use std::io::Write;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};
use vibe_core::exit;

use crate::root::{Miss, Root};
use crate::text::{find, lines, lossy, name_of, sub_dirs, walk_files};

/// Command options.
pub struct Options {
    /// Superpowers artifacts are errors, not warnings (D46).
    pub pre_merge: bool,
    /// Print Spec JSON v1 `check-drift` instead of text.
    pub json: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Sev {
    Error,
    Warning,
    Note,
}

struct Event {
    sev: Sev,
    msg: Vec<u8>,
    file: Option<String>,
}

fn event(sev: Sev, msg: Vec<u8>, file: Option<String>) -> Event {
    Event { sev, msg, file }
}

/// The oracle's `find_repo_root`, from the cwd: the nearest ancestor (not
/// `/`) holding `.spec/` or `.git`, else the cwd.
fn marker_root(cwd: &Path) -> PathBuf {
    let mut d = Some(cwd);
    while let Some(dir) = d {
        if dir.parent().is_none() {
            break;
        }
        if dir.join(".spec").is_dir() || dir.join(".git").exists() {
            return dir.to_path_buf();
        }
        d = dir.parent();
    }
    cwd.to_path_buf()
}

/// The first line holding `needle` (`grep -F … | head -n1`).
fn first_line_with<'a>(text: &'a [u8], needle: &[u8]) -> Option<&'a [u8]> {
    lines(text).into_iter().find(|l| find(l, needle).is_some())
}

/// The oracle's status-cell scan: split on `|`, keep each cell's ASCII
/// letters, and match the whole cell. The text after the last `|` is never
/// read.
fn row_status(row: &[u8]) -> &'static str {
    let mut status = "NOT STARTED";
    let mut rest = row;
    while let Some(i) = rest.iter().position(|&c| c == b'|') {
        let cell = rest.get(..i).unwrap_or_default();
        rest = rest.get(i + 1..).unwrap_or_default();
        let letters: Vec<u8> = cell
            .iter()
            .copied()
            .filter(u8::is_ascii_alphabetic)
            .collect();
        match letters.as_slice() {
            b"DONE" => return "DONE",
            b"INPROGRESS" => status = "IN PROGRESS",
            _ => {}
        }
    }
    status
}

/// `grep -E '[0-9]+ (passing|passed|assertions|green)'` on one line.
fn hand_count(line: &[u8]) -> bool {
    line.windows(2).enumerate().any(|(i, w)| {
        w.first().is_some_and(u8::is_ascii_digit)
            && w.get(1) == Some(&b' ')
            && line.get(i + 2..).is_some_and(|rest| {
                [&b"passing"[..], b"passed", b"assertions", b"green"]
                    .iter()
                    .any(|word| rest.starts_with(word))
            })
    })
}

// A symlinked spec root is scanned; the oracle's `find` without -H skips it
// (P3c Task 3 ruling: counts there are still drift).
/// `find "$SPEC_DIR" -type f -name '*.md' -not -path "$SPEC_DIR/archive/*"`,
/// as paths relative to the root, sorted. Dot files count; symlinks do not.
fn count_targets(dir: &Path) -> Vec<String> {
    fn walk(dir: &Path, prefix: &str, out: &mut Vec<String>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            let rel = format!("{prefix}{name}");
            let Ok(meta) = std::fs::symlink_metadata(entry.path()) else {
                continue;
            };
            if meta.is_dir() {
                if rel != "archive" {
                    walk(&entry.path(), &format!("{rel}/"), out);
                }
            } else if meta.is_file() && name.ends_with(".md") {
                out.push(rel);
            }
        }
    }
    let mut out = Vec::new();
    walk(dir, "", &mut out);
    out.sort();
    out
}

/// Runs the command.
pub fn run(found: &Result<Root, Miss>, cwd: &Path, opts: &Options) -> (u8, Vec<u8>) {
    let root = match found {
        Ok(root) => root,
        Err(Miss::NotFound) if !opts.json => {
            eprintln!(
                "check-drift: no .spec/ under {}; nothing to check.",
                marker_root(cwd).to_string_lossy()
            );
            return (exit::OK, Vec::new());
        }
        Err(miss) => {
            eprintln!("{}", miss.message());
            return (miss.code(), Vec::new());
        }
    };
    let d = root.display();
    let plan_path = root.dir.join("plan.md");
    let plan = plan_path
        .is_file()
        .then(|| std::fs::read(&plan_path).unwrap_or_default());
    let features: Vec<PathBuf> = sub_dirs(&root.dir.join("features"));
    let mut ev: Vec<Event> = Vec::new();
    let mut statuses: Vec<(String, &'static str)> = Vec::new();

    // (a) every live feature folder must be named in the root plan.
    match &plan {
        Some(text) => {
            for dir in &features {
                let feat = name_of(dir);
                if find(text, feat.as_bytes()).is_none() {
                    ev.push(event(
                        Sev::Error,
                        cat!(
                            "feature '",
                            feat,
                            "' has no row in ",
                            d,
                            "/plan.md — a merged feature must be compounded into the root plan"
                        ),
                        None,
                    ));
                }
            }
        }
        None => ev.push(event(
            Sev::Warning,
            cat!(
                "root plan ",
                d,
                "/plan.md not found; skipping plan-row check."
            ),
            None,
        )),
    }

    // (d) status of each live feature's row in the root plan.
    if let Some(text) = &plan {
        for dir in &features {
            let feat = name_of(dir);
            let row = first_line_with(text, format!("| {feat} |").as_bytes())
                .or_else(|| first_line_with(text, format!("**{feat}**").as_bytes()));
            let Some(row) = row else {
                statuses.push((feat.clone(), "NO ROW"));
                ev.push(event(
                    Sev::Note,
                    cat!(
                        "check-drift: note: feature '",
                        feat,
                        "' has no Feature Sequence row in ",
                        d,
                        "/plan.md."
                    ),
                    None,
                ));
                continue;
            };
            let status = row_status(row);
            statuses.push((feat.clone(), status));
            ev.push(event(
                Sev::Note,
                cat!(
                    "check-drift: live feature '",
                    feat,
                    "' — root plan says: ",
                    status
                ),
                None,
            ));
            if status == "DONE" {
                ev.push(event(
                    Sev::Error,
                    cat!(
                        "feature '",
                        feat,
                        "' is DONE in ",
                        d,
                        "/plan.md but ",
                        d,
                        "/features/",
                        feat,
                        "/ still exists — compound promotes AND archives; move it to ",
                        d,
                        "/archive/"
                    ),
                    None,
                ));
            }
        }
    }

    // (b) NOT STARTED units left in a live feature plan.
    for dir in &features {
        let path = dir.join("plan.md");
        if !path.is_file() {
            continue;
        }
        let rel = format!("{d}/features/{}/plan.md", name_of(dir));
        let text = std::fs::read(&path).unwrap_or_default();
        for (i, line) in lines(&text).into_iter().enumerate() {
            if find(line, b"NOT STARTED").is_some() {
                ev.push(event(
                    Sev::Warning,
                    cat!(
                        "NOT STARTED unit in ",
                        rel,
                        ":",
                        (i + 1).to_string(),
                        ":",
                        line
                    ),
                    Some(rel.clone()),
                ));
            }
        }
    }

    // (c) hand-written test counts in README.md or the tree (archive excluded).
    let mut targets: Vec<(String, PathBuf)> = Vec::new();
    if root.project.join("README.md").is_file() {
        targets.push(("README.md".to_string(), root.project.join("README.md")));
    }
    for rel in count_targets(&root.dir) {
        let path = root.dir.join(&rel);
        targets.push((format!("{d}/{rel}"), path));
    }
    for (shown, path) in &targets {
        let text = std::fs::read(path).unwrap_or_default();
        for (i, line) in lines(&text).into_iter().enumerate() {
            if hand_count(line) {
                ev.push(event(
                    Sev::Error,
                    cat!(
                        "hand-written test count in ",
                        shown,
                        ":",
                        (i + 1).to_string(),
                        ":",
                        line
                    ),
                    Some(shown.clone()),
                ));
            }
        }
    }

    // D46 (P3c-R4): superpowers artifacts left in the tree.
    for rel in walk_files(&root.project.join("docs").join("superpowers")) {
        let shown = format!("docs/superpowers/{rel}");
        let sev = if opts.pre_merge {
            Sev::Error
        } else {
            Sev::Warning
        };
        ev.push(event(
            sev,
            cat!(
                "superpowers artifact remains: ",
                shown,
                " — promote its facts, then run vibe-spec promote <feature> --done"
            ),
            Some(shown.clone()),
        ));
    }

    let errors = ev.iter().filter(|e| e.sev == Sev::Error).count();
    let warnings = ev.iter().filter(|e| e.sev == Sev::Warning).count();
    if opts.json {
        let findings: Vec<Value> = ev
            .iter()
            .filter(|e| e.sev != Sev::Note)
            .map(|e| {
                let mut f = json!({
                    "severity": if e.sev == Sev::Error { "error" } else { "warning" },
                    "message": lossy(&e.msg),
                });
                if let Some(file) = &e.file {
                    f["file"] = json!(file);
                }
                f
            })
            .collect();
        let v = json!({
            "v": 1,
            "root": d,
            "pre_merge": opts.pre_merge,
            "errors": errors,
            "warnings": warnings,
            "features": statuses.iter().map(|(n, s)| json!({ "name": n, "status": s })).collect::<Vec<_>>(),
            "findings": findings,
        });
        let code = if errors > 0 { exit::FINDINGS } else { exit::OK };
        return (code, format!("{v}\n").into_bytes());
    }
    let mut err = std::io::stderr().lock();
    for e in &ev {
        let line = match e.sev {
            Sev::Error => cat!("check-drift: ERROR: ", e.msg, "\n"),
            Sev::Warning => cat!("check-drift: WARN: ", e.msg, "\n"),
            Sev::Note => cat!(e.msg, "\n"),
        };
        let _ = err.write_all(&line);
    }
    if errors > 0 {
        let _ = writeln!(
            err,
            "check-drift: FAIL — {errors} error(s), {warnings} warning(s)."
        );
        return (exit::FINDINGS, Vec::new());
    }
    (
        exit::OK,
        format!("check-drift: OK — 0 errors, {warnings} warning(s).\n").into_bytes(),
    )
}
