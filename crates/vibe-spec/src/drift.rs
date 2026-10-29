//! `vibe-spec check-drift` — port of `spec/scripts/check-drift.sh` (F18,
//! P3c-R3). For a `.spec` root the output is the oracle's, byte for byte: // literal
//! stdout carries only the final OK line, everything else goes to stderr.
//! Paths print relative to the project through `Root::display()`.
use std::io::Write;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};
use vibe_core::exit;

use crate::frontmatter;
use crate::root::{Miss, Root};
use crate::text::{find, join_raw, lines, lossy, name_of, sub_dirs, walk_all};

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
/// `/`) holding `docs/spec/`, the legacy root or `.git`, else the cwd.
fn marker_root(cwd: &Path) -> PathBuf {
    let mut d = Some(cwd);
    while let Some(dir) = d {
        if dir.parent().is_none() {
            break;
        }
        if dir.join("docs/spec").is_dir()
            || dir.join(crate::literal::LEGACY).is_dir()
            || dir.join(".git").exists()
        {
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
/// An unreadable directory is returned as `(shown path, error)`, never skipped.
fn count_targets(dir: &Path, shown: &str) -> (Vec<String>, Vec<(String, std::io::Error)>) {
    fn walk(
        dir: &Path,
        prefix: &str,
        shown: &str,
        out: &mut Vec<String>,
        errs: &mut Vec<(String, std::io::Error)>,
    ) {
        let entries = match std::fs::read_dir(dir) {
            Ok(entries) => entries,
            Err(e) => {
                errs.push((format!("{shown}/{}", prefix.trim_end_matches('/')), e));
                return;
            }
        };
        for entry in entries {
            let entry = match entry {
                Ok(entry) => entry,
                Err(e) => {
                    errs.push((format!("{shown}/{}", prefix.trim_end_matches('/')), e));
                    continue;
                }
            };
            let name = entry.file_name().to_string_lossy().into_owned();
            let rel = format!("{prefix}{name}");
            let meta = match std::fs::symlink_metadata(entry.path()) {
                Ok(meta) => meta,
                Err(e) => {
                    errs.push((format!("{shown}/{rel}"), e));
                    continue;
                }
            };
            if meta.is_dir() {
                if rel != "archive" {
                    walk(&entry.path(), &format!("{rel}/"), shown, out, errs);
                }
            } else if meta.is_file() && name.ends_with(".md") {
                out.push(rel);
            }
        }
    }
    let mut out = Vec::new();
    let mut errs = Vec::new();
    walk(dir, "", shown, &mut out, &mut errs);
    out.sort();
    (out, errs)
}

/// Reads a file that feeds the verdict. NotFound reads as empty (the file
/// vanished after the `is_file` check); any other error is an ERROR finding.
fn read_closed(path: &Path, shown: &str, ev: &mut Vec<Event>) -> Option<Vec<u8>> {
    match std::fs::read(path) {
        Ok(bytes) => Some(bytes),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Some(Vec::new()),
        Err(e) => {
            ev.push(unreadable(shown, &e));
            None
        }
    }
}

fn unreadable(shown: &str, e: &std::io::Error) -> Event {
    event(
        Sev::Error,
        cat!("cannot read ", shown, ": ", e.to_string()),
        Some(shown.to_string()),
    )
}

/// Relative `.md` link targets of `text`: `](target)` with the anchor cut,
/// no scheme, not absolute.
fn md_link_targets(text: &[u8]) -> Vec<String> {
    let s = String::from_utf8_lossy(text);
    let mut out = Vec::new();
    let mut rest: &str = &s;
    while let Some((_, tail)) = rest.split_once("](") {
        let Some((target, after)) = tail.split_once(')') else {
            break;
        };
        rest = after;
        let target = target.split('#').next().unwrap_or_default().trim();
        if target.ends_with(".md") && !target.contains("://") && !target.starts_with('/') {
            out.push(target.to_string());
        }
    }
    out
}

/// The plan hub and the notes it lists (`children:` or a relative `.md`
/// link), one level, each kept only when it resolves to a file inside the
/// root. Display path and bytes; the hub comes first. A missing or
/// unreadable note is skipped.
fn plan_texts(root: &Root) -> Vec<(String, Vec<u8>)> {
    let d = root.display();
    let Ok(plan) = std::fs::read(root.dir.join("plan.md")) else {
        return Vec::new();
    };
    let mut candidates: Vec<String> = Vec::new();
    if let Some(fm) = frontmatter::parse(&plan) {
        candidates.extend(fm.list("children"));
    }
    candidates.extend(md_link_targets(&plan));
    let mut out = vec![(format!("{d}/plan.md"), plan)];
    let Ok(canon_root) = std::fs::canonicalize(&root.dir) else {
        return out;
    };
    let mut seen = vec![root.dir.join("plan.md")];
    for rel in candidates {
        let rel = rel.trim().trim_start_matches("./").to_string();
        if rel.is_empty() || rel.contains('{') {
            continue;
        }
        let path = join_raw(&root.dir, rel.as_bytes());
        let Ok(canon) = std::fs::canonicalize(&path) else {
            continue;
        };
        if !canon.starts_with(&canon_root) || !canon.is_file() || seen.contains(&canon) {
            continue;
        }
        let Ok(bytes) = std::fs::read(&canon) else {
            continue;
        };
        seen.push(canon);
        out.push((format!("{d}/{rel}"), bytes));
    }
    out
}

/// Runs the command.
pub fn run(found: &Result<Root, Miss>, cwd: &Path, opts: &Options) -> (u8, Vec<u8>) {
    let root = match found {
        Ok(root) => root,
        Err(Miss::NotFound) if !opts.json => {
            eprintln!(
                "check-drift: no .spec/ under {}; nothing to check.", // literal
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
    let mut ev: Vec<Event> = Vec::new();
    let plan_present = plan_path.is_file();
    let plan = if plan_present {
        read_closed(&plan_path, &format!("{d}/plan.md"), &mut ev).map(|hub| {
            // (a) and (d) read the hub and the notes it lists (skill-revision/16).
            let mut text = hub;
            for (_, bytes) in plan_texts(root).into_iter().skip(1) {
                text.push(b'\n');
                text.extend(bytes);
            }
            text
        })
    } else {
        None
    };
    let features: Vec<PathBuf> = sub_dirs(&root.dir.join("features"));
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
        None if plan_present => {}
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
        let Some(text) = read_closed(&path, &rel, &mut ev) else {
            continue;
        };
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
    let (rels, walk_errs) = count_targets(&root.dir, &d);
    for (shown, e) in &walk_errs {
        ev.push(unreadable(shown, e));
    }
    for rel in rels {
        let path = root.dir.join(&rel);
        targets.push((format!("{d}/{rel}"), path));
    }
    for (shown, path) in &targets {
        let Some(text) = read_closed(path, shown, &mut ev) else {
            continue;
        };
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

    // D46 (P3c-R4): superpowers artifacts left in the tree. An unreadable
    // tree is a finding of the same severity (fail closed).
    let sev = if opts.pre_merge {
        Sev::Error
    } else {
        Sev::Warning
    };
    match walk_all(&root.project.join("docs").join("superpowers")) {
        Ok((files, links)) => {
            for rel in files {
                let shown = format!("docs/superpowers/{rel}");
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
            // Never followed or deleted: still an artifact left behind.
            for rel in links {
                let shown = format!("docs/superpowers/{rel}");
                ev.push(event(
                    sev,
                    cat!("superpowers artifact is a symlink: ", shown),
                    Some(shown.clone()),
                ));
            }
        }
        Err(e) => ev.push(event(
            sev,
            cat!("cannot read docs/superpowers: ", e.to_string()),
            Some("docs/superpowers".to_string()),
        )),
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

#[cfg(test)]
mod tests {
    use super::marker_root;

    #[test]
    #[expect(clippy::unwrap_used, reason = "test scratch tree")]
    fn marker_root_accepts_a_docs_spec_only_project() {
        let base = vibe_core::testscratch::scratch("vibe-drift-marker", "docs-spec");
        let nested = base.join("a/b");
        std::fs::create_dir_all(&nested).unwrap();
        std::fs::create_dir_all(base.join("docs/spec")).unwrap();
        let got = marker_root(&nested);
        std::fs::remove_dir_all(&base).unwrap();
        assert_eq!(got, base);
    }
}
