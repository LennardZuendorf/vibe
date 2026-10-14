#![cfg(unix)]
#![allow(clippy::unwrap_used, clippy::expect_used)]
//! The committed spec corpus is exactly what the generator writes, and the
//! frozen spec oracles are deterministic over it.
use std::fs;
use std::path::{Path, PathBuf};

use vibe_parity::corpus_spec::{MARKER, generate};
use vibe_parity::harness::{Mode, execute, run_suite};
use vibe_parity::sandbox::{ORACLE_DIR, snapshot, stage};
use vibe_parity::suite::Suite;

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn scratch(name: &str) -> PathBuf {
    let dir =
        std::env::temp_dir().join(format!("vibe-parity-corpus-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

const TEMPLATES: &str = "oracles/bash/spec/reference/templates";

#[test]
fn committed_corpus_matches_the_generator() {
    let out = scratch("regen").join("spec");
    let n = generate(&repo().join(TEMPLATES), &out).unwrap();
    assert!(n >= 12, "generated only {n} cases");
    let committed = snapshot(&repo().join("tests/parity/corpus/spec")).unwrap();
    let fresh = snapshot(&out).unwrap();
    assert_eq!(
        committed.keys().collect::<Vec<_>>(),
        fresh.keys().collect::<Vec<_>>(),
        "file sets differ: run `cargo run -p vibe-parity -- gen-spec-corpus {TEMPLATES} tests/parity/corpus/spec`"
    );
    for (path, bytes) in &committed {
        assert_eq!(
            Some(bytes),
            fresh.get(path),
            "{path} differs from the generator output"
        );
    }
}

#[test]
fn the_generator_refuses_a_directory_it_did_not_write() {
    let out = scratch("refuse");
    fs::write(out.join("keep.txt"), b"mine").unwrap();
    let err = generate(&repo().join(TEMPLATES), &out).unwrap_err();
    assert!(err.contains(MARKER), "{err}");
    assert_eq!(fs::read(out.join("keep.txt")).unwrap(), b"mine");
}

#[test]
fn spec_oracles_are_deterministic_over_the_corpus() {
    let suite = Suite::load(&repo().join("tests/parity/spec.json")).unwrap();
    let report = run_suite(&suite, &Mode::SelfCheck, &scratch("self"));
    assert!(report.ok(), "{:#?}", report.errors);
    assert!(
        report.results.len() >= 60,
        "only {} runs",
        report.results.len()
    );
}

#[test]
fn no_root_cases_stop_at_the_sandbox_git_dir() {
    let dir = scratch("escape");
    let dest = dir.join("sandbox");
    let case = repo().join("tests/parity/corpus/spec/no-root/subdir");
    stage(&case.join("tree"), &repo().join("oracles/bash/spec"), &dest).unwrap();
    let script = dest.join(ORACLE_DIR).join("scripts/list-specs.sh");
    let argv = ["bash".to_string(), script.to_string_lossy().into_owned()];
    let (code, out) = execute(&argv, &dest.join("src/deep"), &dir).unwrap();
    assert_eq!(code, Some(0));
    assert_eq!(
        String::from_utf8(out).unwrap(),
        "No .spec/ directory found in current project\n"
    );
}
