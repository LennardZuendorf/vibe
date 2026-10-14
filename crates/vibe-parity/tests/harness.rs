#![cfg(unix)]
#![allow(clippy::unwrap_used, clippy::expect_used)]
//! The harness passes a faithful candidate and fails each planted mutant.
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::{Value, json};
use vibe_parity::harness::{Mode, Report, Status, execute, run_suite};
use vibe_parity::suite::Suite;

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixture")
}

fn work(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("vibe-parity-test-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn candidate() -> Mode {
    Mode::Candidate(PathBuf::from(env!("CARGO_BIN_EXE_parity-fixture")))
}

/// The fixture suite as JSON, with absolute corpus and oracle paths.
fn base() -> Value {
    let mut v: Value =
        serde_json::from_str(&fs::read_to_string(fixture().join("suite.json")).unwrap()).unwrap();
    v["corpus"] = json!(fixture().join("corpus"));
    v["oracle_root"] = json!(fixture().join("oracle"));
    v
}

fn run(name: &str, suite: Value, mode: &Mode) -> Report {
    let dir = work(name);
    let path = dir.join("suite.json");
    fs::write(&path, serde_json::to_vec(&suite).unwrap()).unwrap();
    let suite = Suite::load(&path).unwrap();
    run_suite(&suite, mode, &dir.join("work"))
}

#[test]
fn self_check_passes_over_every_run() {
    let report = run("self", base(), &Mode::SelfCheck);
    assert!(report.ok(), "{report:#?}");
    assert_eq!(report.results.len(), 3);
}

#[test]
fn a_faithful_candidate_passes() {
    let report = run("faithful", base(), &candidate());
    assert!(report.ok(), "{report:#?}");
    assert_eq!(report.passed(), 3);
}

#[test]
fn each_mutant_fails_on_its_field() {
    for (flag, word) in [
        ("--mutate=stdout", "stdout"),
        ("--mutate=exit", "exit"),
        ("--mutate=file", "files"),
    ] {
        let mut suite = base();
        suite["commands"][0]["candidate"] = json!(["{candidate}", flag]);
        let report = run(&format!("mutant-{word}"), suite, &candidate());
        assert!(!report.ok(), "{flag} passed");
        assert_eq!(report.failed(), 3, "{flag}: {report:#?}");
        for r in &report.results {
            let Status::Fail(msgs) = &r.status else {
                panic!("{flag}: {r:?}")
            };
            assert!(msgs.iter().any(|m| m.starts_with(word)), "{flag}: {msgs:?}");
        }
    }
}

#[test]
fn a_floor_above_the_corpus_fails() {
    let mut suite = base();
    suite["floors"]["basic"] = json!(3);
    let report = run("floor", suite, &Mode::SelfCheck);
    assert!(!report.ok());
    assert!(
        report
            .errors
            .iter()
            .any(|e| e == "class basic: 2 case(s), floor 3"),
        "{:?}",
        report.errors
    );
}

#[test]
fn a_class_without_a_floor_or_with_floor_zero_fails() {
    let mut suite = base();
    suite["floors"] = json!({});
    assert!(
        run("nofloor", suite, &Mode::SelfCheck)
            .errors
            .iter()
            .any(|e| e.contains("has no floor"))
    );
    let mut suite = base();
    suite["floors"]["basic"] = json!(0);
    assert!(
        run("zerofloor", suite, &Mode::SelfCheck)
            .errors
            .iter()
            .any(|e| e.contains("asserts nothing"))
    );
}

#[test]
fn an_empty_class_fails() {
    let mut suite = base();
    suite["commands"][0]["classes"] = json!(["absent"]);
    suite["floors"] = json!({ "absent": 1 });
    let report = run("empty", suite, &Mode::SelfCheck);
    assert!(!report.ok());
    assert!(report.results.is_empty());
}

#[test]
fn pinned_divergences_pass_and_stale_pins_fail() {
    let pins = json!([
        { "command": "echo", "case": "basic/one", "field": "stdout", "reason": "test" },
        { "command": "echo", "case": "basic/two", "field": "stdout", "reason": "test" }
    ]);
    let mut suite = base();
    suite["divergences"] = pins.clone();
    suite["commands"][0]["candidate"] = json!(["{candidate}", "--mutate=stdout"]);
    let report = run("pinned", suite, &candidate());
    assert!(report.ok(), "{report:#?}");
    assert_eq!(report.pinned(), 3);

    let mut suite = base();
    suite["divergences"] = pins;
    let report = run("stale", suite, &candidate());
    assert!(!report.ok());
    assert_eq!(
        report
            .errors
            .iter()
            .filter(|e| e.starts_with("stale pin"))
            .count(),
        2
    );
}

#[test]
fn the_cli_reports_and_exits_by_verdict() {
    let bin = env!("CARGO_BIN_EXE_vibe-parity");
    let suite = fixture().join("suite.json");
    let ok = Command::new(bin)
        .arg("run")
        .arg(&suite)
        .arg("--self")
        .output()
        .unwrap();
    assert_eq!(
        ok.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&ok.stdout)
    );
    assert!(
        String::from_utf8_lossy(&ok.stdout)
            .contains("parity[fixture]: 3 passed, 0 pinned, 0 failed")
    );
    let bad = Command::new(bin).arg("run").arg(&suite).output().unwrap();
    assert_eq!(bad.status.code(), Some(2));
}

#[test]
fn runs_see_only_path_home_locale_and_timezone() {
    let dir = work("env");
    let (code, out) = execute(&["env".to_string()], &dir, &dir).unwrap();
    assert_eq!(code, Some(0));
    let mut keys: Vec<String> = String::from_utf8(out)
        .unwrap()
        .lines()
        .filter_map(|l| l.split_once('=').map(|(k, _)| k.to_string()))
        .collect();
    keys.sort();
    assert_eq!(keys, ["HOME", "LC_ALL", "PATH", "TZ"]);
}

#[test]
fn a_symlink_in_the_corpus_is_refused() {
    let dir = work("symlink");
    let corpus = dir.join("corpus/basic/one");
    fs::create_dir_all(corpus.join("tree")).unwrap();
    fs::write(corpus.join("case.json"), "{ \"v\": 1 }\n").unwrap();
    std::os::unix::fs::symlink("/etc/hosts", corpus.join("tree/link")).unwrap();
    let mut suite = base();
    suite["corpus"] = json!(dir.join("corpus"));
    suite["floors"]["basic"] = json!(1);
    let report = run("symlink-run", suite, &Mode::SelfCheck);
    assert!(!report.ok());
    let Status::Fail(msgs) = &report.results[0].status else {
        panic!("{report:#?}")
    };
    assert!(msgs[0].contains("symlinks are not allowed"), "{msgs:?}");
}
