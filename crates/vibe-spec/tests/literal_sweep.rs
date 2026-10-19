// vibe-spec: literal-file
//! The zero-literal sweep (P4-R16a, R-P): no tracked file outside `oracles/`
//! carries the legacy root name, except through a tag, a file marker or one of
//! the path exemptions below. It reuses `literal::find`, the one regex source
//! `vibe-spec migrate` uses, so the sweep and the tool cannot disagree.
//!
//! Run through `tests/literals/run.sh`; `LITERALS_REPORT_ONLY=1` prints the
//! findings and passes (until the migration commit lands).
#![allow(dead_code, clippy::unwrap_used, clippy::expect_used)]

#[path = "../src/literal.rs"]
mod literal;
#[path = "../src/text.rs"]
mod text;

use std::path::{Path, PathBuf};
use std::process::Command;

/// Path exemptions: `(prefix or exact path, reason)`. A trailing `/` is a prefix.
const EXEMPT: [(&str, &str); 6] = [
    (
        "oracles/",
        "frozen oracles of the bash and JS originals; never rewritten",
    ),
    (
        "contracts/fixtures/",
        "legacy-tree fixtures; JSON cannot carry a comment",
    ),
    (
        "tests/parity/corpus/",
        "legacy-tree corpus the parity matrix compares; JSON cannot carry a comment",
    ),
    (
        "flow/content/policy.json",
        "JSON holding the dual-root write rules (P4-R16)",
    ),
    (
        "flow/reference/templates/AGENTS.md",
        "ships the rendered dual-root write rules; a marker line would break unmerge",
    ),
    (
        "docs/superpowers/",
        "closes in P5, once promoted and deleted per D46",
    ),
];

fn exempt(rel: &str) -> bool {
    EXEMPT.iter().any(|(p, _)| {
        if p.ends_with('/') {
            rel.starts_with(p)
        } else {
            rel == *p
        }
    })
}

/// The tree to sweep: `LITERALS_ROOT` (a probe checkout), else this repo.
fn repo_root() -> PathBuf {
    if let Some(dir) = std::env::var_os("LITERALS_ROOT") {
        return PathBuf::from(dir);
    }
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repo root")
}

#[test]
fn exemptions_are_prefixes_or_exact_paths() {
    assert!(exempt("oracles/bash/x.sh"));
    assert!(exempt("flow/content/policy.json"));
    assert!(!exempt("flow/content/policy.json.bak"));
    assert!(!exempt("docs/superpowers"));
    assert!(!exempt("crates/vibe-spec/src/main.rs"));
}

#[test]
#[ignore = "full-repo sweep; run through tests/literals/run.sh"]
fn no_tracked_file_carries_a_literal() {
    let root = repo_root();
    if let Some(dir) = std::env::var_os("LITERALS_ROOT") {
        println!(
            "literal sweep: root {} (LITERALS_ROOT)",
            dir.to_string_lossy()
        );
    }
    let out = Command::new("git")
        .args(["ls-files", "-z"])
        .current_dir(&root)
        .output()
        .expect("git ls-files");
    assert!(out.status.success(), "git ls-files failed");
    let mut scanned = 0usize;
    let mut findings: Vec<String> = Vec::new();
    for raw in out.stdout.split(|b| *b == 0).filter(|n| !n.is_empty()) {
        let Ok(rel) = std::str::from_utf8(raw) else {
            findings.push(format!("{}: non-UTF-8 path", String::from_utf8_lossy(raw)));
            continue;
        };
        if exempt(rel) {
            continue;
        }
        let path = root.join(rel);
        let Ok(meta) = std::fs::symlink_metadata(&path) else {
            continue; // listed but deleted in the working tree
        };
        if !meta.file_type().is_file() {
            continue;
        }
        let bytes = std::fs::read(&path).expect("read tracked file");
        scanned += 1;
        if literal::file_exempt(&bytes) {
            continue;
        }
        for (n, line) in bytes.split(|b| *b == b'\n').enumerate() {
            if !literal::find(line).is_empty() {
                findings.push(format!(
                    "{rel}:{}: {}",
                    n + 1,
                    String::from_utf8_lossy(line).trim()
                ));
            }
        }
    }
    assert!(
        scanned > 100,
        "swept only {scanned} files; is git ls-files empty?"
    );
    println!(
        "literal sweep: {scanned} file(s), {} literal(s)",
        findings.len()
    );
    for f in &findings {
        println!("{f}");
    }
    if std::env::var("LITERALS_REPORT_ONLY").is_ok_and(|v| v == "1") {
        println!("literal sweep: report-only (LITERALS_REPORT_ONLY=1); not failing");
        return;
    }
    assert!(findings.is_empty(), "{} literal(s) remain", findings.len());
}
