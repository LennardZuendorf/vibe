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
    use literal::path_exempt as exempt;
    assert!(exempt("oracles/bash/x.sh"));
    assert!(exempt("flow/content/policy.json"));
    assert!(exempt("flow/reference/templates/AGENTS.md"));
    assert!(exempt("tests/parity/spec.json"));
    assert!(exempt("tests/parity/flow.json"));
    assert!(!exempt("tests/parity/spec.json.bak"));
    assert!(!exempt("tests/parity/flow.json.bak"));
    assert!(!exempt("tests/parity/run.sh"));
    // The materialized copies are exempt file-by-file, never by package prefix:
    // a blanket `plugins/vibe/skills/` rule would blind the sweep to a whole
    // subtree, and a new file there must be seen, not skipped.
    assert!(exempt("plugins/vibe/skills/vibe/content/policy.json"));
    assert!(exempt(
        "factory-plugin/skills/vibe/reference/templates/AGENTS.md"
    ));
    assert!(!exempt("plugins/vibe/skills/vibe/content/policy.json.bak"));
    assert!(!exempt("plugins/vibe/skills/vibe/verify.md"));
    assert!(!exempt("factory-plugin/skills/anything-else"));
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
        if literal::path_exempt(rel) {
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
