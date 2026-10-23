//! `vibe-spec trace [--since <ref>] [--json]` (P3b-R13): units are the
//! canonical `### <f>/<n>` headings of fallback feature plans; citations are
//! `<name>/<n>` tokens in commit messages and under `<project>/tests/`.
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use serde_json::json;
use vibe_core::exit;

use crate::git;
use crate::pattern::unit_citations;
use crate::root::{Miss, Root};
use crate::text::{Fences, lines, lossy, name_of, sub_dirs};

/// Feature names and their units.
fn units(dir: &Path) -> (BTreeSet<String>, BTreeSet<String>) {
    let mut features = BTreeSet::new();
    let mut units = BTreeSet::new();
    for fdir in sub_dirs(&dir.join("features")) {
        let f = name_of(&fdir);
        let text = std::fs::read(fdir.join("plan.md")).unwrap_or_default();
        let own = format!("{f}/");
        let mut fences = Fences::new();
        for line in lines(&text) {
            if fences.skip(line) {
                continue;
            }
            let Some(rest) = line.strip_prefix(b"### ") else {
                continue;
            };
            if let Some(tok) = unit_citations(rest).first()
                && rest.starts_with(tok)
                && tok.starts_with(own.as_bytes())
                && rest.get(tok.len()).is_none_or(|&c| c == b' ')
            {
                units.insert(lossy(tok));
            }
        }
        features.insert(f);
    }
    (features, units)
}

/// Largest `tests/` file scanned.
const MAX_TEST_FILE: u64 = 1024 * 1024;

/// Calls `f(relative path, text)` for each text file under `<project>/tests/`,
/// one at a time (hidden entries, symlinks, files over 1 MiB and files with a
/// NUL in their first 8 KiB are skipped).
fn each_test_file(project: &Path, f: &mut dyn FnMut(&str, &[u8])) {
    fn go(dir: &Path, rel: &str, f: &mut dyn FnMut(&str, &[u8])) {
        let Ok(read) = std::fs::read_dir(dir) else {
            return;
        };
        let mut entries: Vec<_> = read.flatten().collect();
        entries.sort_by_key(|e| e.file_name());
        for e in entries {
            let name = e.file_name().to_string_lossy().into_owned();
            if name.starts_with('.') {
                continue;
            }
            let here = format!("{rel}/{name}");
            match e.file_type() {
                Ok(t) if t.is_dir() => go(&e.path(), &here, f),
                Ok(t) if t.is_file() => {
                    if e.metadata().is_ok_and(|m| m.len() > MAX_TEST_FILE) {
                        continue;
                    }
                    let Ok(text) = std::fs::read(e.path()) else {
                        continue;
                    };
                    if text.iter().take(8192).any(|&b| b == 0) {
                        continue;
                    }
                    f(&here, &text);
                }
                _ => {}
            }
        }
    }
    go(&project.join("tests"), "tests", f);
}

/// Runs the command.
pub fn run(found: &Result<Root, Miss>, since: Option<&str>, json_out: bool) -> (u8, Vec<u8>) {
    let root = match found {
        Ok(root) => root,
        Err(miss) => {
            eprintln!("{}", miss.message());
            return (miss.code(), Vec::new());
        }
    };
    let history = match git::commits(&root.project, since) {
        Ok(h) => h,
        Err(why) => {
            eprintln!("vibe-spec trace: {why}");
            return (exit::USAGE, Vec::new());
        }
    };
    let (features, units) = units(&root.dir);
    if units.is_empty() {
        eprintln!(
            "vibe-spec trace: no units under {}/features/*/plan.md; nothing to trace (superpowers plans carry no unit ids)",
            root.display()
        );
        return (exit::FINDINGS, Vec::new());
    }
    if history.is_none() {
        if git::shallow(&root.project) {
            eprintln!(
                "vibe-spec trace: shallow clone: commit citations skipped; fetch full history"
            );
        } else {
            eprintln!("vibe-spec trace: no git history; citations from tests/ only");
        }
    }
    let mut cited: BTreeMap<String, usize> = units.iter().map(|u| (u.clone(), 0)).collect();
    let mut orphans: BTreeSet<(String, String)> = BTreeSet::new();
    let mut see = |token: &[u8], source: &str| {
        let t = lossy(token);
        let Some((name, _)) = t.split_once('/') else {
            return;
        };
        if !features.contains(name) {
            return;
        }
        match cited.get_mut(&t) {
            Some(n) => *n += 1,
            None => {
                orphans.insert((t, source.to_string()));
            }
        }
    };
    for (sha, body) in history.iter().flatten() {
        let source = format!("commit {}", sha.get(..7).unwrap_or(sha));
        for tok in unit_citations(body) {
            see(tok, &source);
        }
    }
    each_test_file(&root.project, &mut |rel, text| {
        if rel.ends_with(".md") {
            let mut fences = Fences::new();
            for line in lines(text) {
                if fences.skip(line) {
                    continue;
                }
                for tok in unit_citations(line) {
                    see(tok, rel);
                }
            }
        } else {
            for tok in unit_citations(text) {
                see(tok, rel);
            }
        }
    });
    let uncited: Vec<&String> = cited
        .iter()
        .filter(|(_, n)| **n == 0)
        .map(|(u, _)| u)
        .collect();
    let n_cited = cited.len() - uncited.len();
    let label = match (&history, since) {
        (None, _) => "none",
        (Some(_), Some(_)) => "since",
        (Some(_), None) => "all",
    };
    let code = if uncited.is_empty() && orphans.is_empty() {
        exit::OK
    } else {
        exit::FINDINGS
    };
    if json_out {
        let mut v = json!({
            "v": 1,
            "units": units.len(),
            "cited": n_cited,
            "history": label,
            "uncited": uncited,
            "orphans": orphans.iter().map(|(u, s)| json!({ "unit": u, "source": s })).collect::<Vec<_>>(),
        });
        if let (Some(s), "since") = (since, label) {
            v["since"] = json!(s);
        }
        return (code, format!("{v}\n").into_bytes());
    }
    let commits = match (label, since) {
        ("since", Some(s)) => format!("{s}..HEAD"),
        ("all", _) => "all".to_string(),
        _ => "unavailable".to_string(),
    };
    let mut out = format!(
        "trace: {} units, {n_cited} cited; commits: {commits}\n",
        units.len()
    );
    for u in &uncited {
        out.push_str(&format!("UNCITED  {u}\n"));
    }
    for (u, s) in &orphans {
        out.push_str(&format!("ORPHAN   {u}  {s}\n"));
    }
    (code, out.into_bytes())
}
