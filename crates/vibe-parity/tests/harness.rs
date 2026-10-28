#![cfg(unix)]
#![allow(clippy::unwrap_used, clippy::expect_used)]
//! The harness passes a faithful candidate and fails each planted mutant.
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use serde_json::{Value, json};
use vibe_parity::harness::{Mode, Report, Status, execute, run_suite};
use vibe_parity::suite::Suite;

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixture")
}

fn work(name: &str) -> PathBuf {
    vibe_core::testscratch::scratch("vibe-parity-test", name)
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

/// A one-case suite over `case_json`, returning its self-check report.
fn one_case(name: &str, case_json: &str) -> Report {
    let dir = work(name);
    let case = dir.join("corpus/basic/one");
    fs::create_dir_all(case.join("tree")).unwrap();
    fs::write(case.join("case.json"), case_json).unwrap();
    let mut suite = base();
    suite["corpus"] = json!(dir.join("corpus"));
    suite["floors"]["basic"] = json!(1);
    run(&format!("{name}-run"), suite, &Mode::SelfCheck)
}

#[test]
fn a_cwd_outside_the_sandbox_is_refused() {
    for (name, cwd) in [("cwd-up", ".."), ("cwd-abs", "/")] {
        let report = one_case(name, &format!("{{ \"v\": 1, \"cwd\": {cwd:?} }}"));
        assert!(!report.ok(), "{cwd}: {report:#?}");
        assert!(
            report.errors.iter().any(|e| e.contains("cwd")),
            "{cwd}: {:?}",
            report.errors
        );
    }
}

#[test]
fn a_command_with_no_argument_lists_is_refused() {
    let report = one_case("noruns", "{ \"v\": 1, \"runs\": { \"echo\": [] } }");
    assert!(!report.ok(), "{report:#?}");
    assert!(
        report
            .errors
            .iter()
            .any(|e| e.contains("no argument lists")),
        "{:?}",
        report.errors
    );
}

#[test]
fn a_symlinked_tree_or_oracle_root_is_refused() {
    let dir = work("rootlink");
    let case = dir.join("corpus/basic/one");
    fs::create_dir_all(&case).unwrap();
    fs::write(case.join("case.json"), "{ \"v\": 1 }\n").unwrap();
    std::os::unix::fs::symlink(fixture().join("corpus/basic/one/tree"), case.join("tree")).unwrap();
    let mut suite = base();
    suite["corpus"] = json!(dir.join("corpus"));
    suite["floors"]["basic"] = json!(1);
    let report = run("rootlink-run", suite, &Mode::SelfCheck);
    assert!(!report.ok(), "{report:#?}");
    assert!(
        format!("{report:?}").contains("symlinks are not allowed"),
        "{report:#?}"
    );

    let link = dir.join("oracle-link");
    std::os::unix::fs::symlink(fixture().join("oracle"), &link).unwrap();
    let mut suite = base();
    suite["oracle_root"] = json!(link);
    let report = run("oraclelink-run", suite, &Mode::SelfCheck);
    assert!(!report.ok(), "{report:#?}");
    assert!(
        format!("{report:?}").contains("symlinks are not allowed"),
        "{report:#?}"
    );
}

#[test]
fn a_case_dir_without_case_json_fails() {
    let dir = work("nocasejson");
    let corpus = dir.join("corpus/basic");
    fs::create_dir_all(corpus.join("one/tree")).unwrap();
    fs::write(corpus.join("one/case.json"), "{ \"v\": 1 }\n").unwrap();
    fs::create_dir_all(corpus.join("two/tree")).unwrap();
    let mut suite = base();
    suite["corpus"] = json!(dir.join("corpus"));
    suite["floors"]["basic"] = json!(1);
    let report = run("nocasejson-run", suite, &Mode::SelfCheck);
    assert!(
        report.errors.iter().any(|e| e.contains("no case.json")),
        "{report:#?}"
    );
    assert!(!report.ok());
}

#[test]
fn a_runs_key_naming_no_command_fails() {
    let report = one_case(
        "unknownrun",
        "{ \"v\": 1, \"runs\": { \"ehco\": [[\"x\"]] } }",
    );
    assert!(
        report
            .errors
            .iter()
            .any(|e| e.contains("runs names command ehco, which is not in the suite")),
        "{report:#?}"
    );
    assert!(!report.ok());
}

#[test]
fn a_command_over_no_class_fails() {
    let mut suite = base();
    let mut spare = suite["commands"][0].clone();
    spare["name"] = json!("spare");
    spare["classes"] = json!([]);
    suite["commands"].as_array_mut().unwrap().push(spare);
    let report = run("noclasses", suite, &Mode::SelfCheck);
    assert!(
        report
            .errors
            .iter()
            .any(|e| e == "command spare runs over no class"),
        "{report:#?}"
    );
    assert!(!report.ok());
}

#[test]
fn a_pin_with_args_covers_only_that_run() {
    let mut suite = base();
    suite["divergences"] = json!([
        { "command": "echo", "case": "basic/one", "args": ["a"], "field": "stdout", "reason": "test" }
    ]);
    suite["commands"][0]["candidate"] = json!(["{candidate}", "--mutate=stdout"]);
    let report = run("pin-args", suite, &candidate());
    assert_eq!(report.pinned(), 1, "{report:#?}");
    assert_eq!(report.failed(), 2, "{report:#?}");
    assert!(report.errors.is_empty(), "{:?}", report.errors);
}

#[test]
fn a_pin_with_args_that_match_no_run_is_stale() {
    let mut suite = base();
    suite["divergences"] = json!([
        { "command": "echo", "case": "basic/one", "args": ["zzz"], "field": "stdout", "reason": "test" }
    ]);
    let report = run("pin-args-stale", suite, &candidate());
    assert!(
        report
            .errors
            .iter()
            .any(|e| e.starts_with("stale pin: echo basic/one [\"zzz\"]")),
        "{:?}",
        report.errors
    );
}

#[test]
fn an_os_pin_is_inactive_elsewhere_and_never_stale() {
    let mut suite = base();
    suite["divergences"] = json!([
        { "command": "echo", "case": "basic/one", "field": "stdout", "os": "plan9", "reason": "test" }
    ]);
    suite["commands"][0]["candidate"] = json!(["{candidate}", "--mutate=stdout"]);
    let report = run("pin-os-other", suite.clone(), &candidate());
    assert_eq!(report.failed(), 3, "{report:#?}");
    assert!(report.errors.is_empty(), "{:?}", report.errors);

    suite["divergences"][0]["os"] = json!(std::env::consts::OS);
    let report = run("pin-os-here", suite, &candidate());
    assert_eq!(report.pinned(), 2, "{report:#?}");
    assert_eq!(report.failed(), 1, "{report:#?}");

    let mut suite = base();
    suite["divergences"] = json!([
        { "command": "echo", "case": "basic/one", "field": "stdout", "os": std::env::consts::OS, "reason": "test" }
    ]);
    let report = run("pin-os-unhit", suite, &candidate());
    assert!(
        report.ok(),
        "an unhit os pin is not stale: {:?}",
        report.errors
    );
}

#[test]
fn a_hanging_run_fails_at_the_timeout() {
    let mut suite = base();
    suite["timeout_ms"] = json!(300);
    suite["commands"][0]["candidate"] = json!(["{candidate}", "--mutate=hang"]);
    let start = Instant::now();
    let report = run("hang", suite, &candidate());
    assert!(
        start.elapsed() < Duration::from_secs(20),
        "took {:?}",
        start.elapsed()
    );
    assert_eq!(report.failed(), 3, "{report:#?}");
    for r in &report.results {
        let Status::Fail(msgs) = &r.status else {
            panic!("{r:?}")
        };
        assert!(
            msgs.iter().any(|m| m.contains("timed out after 300 ms")),
            "{msgs:?}"
        );
    }
}

#[test]
fn an_extra_empty_directory_fails_on_files() {
    let mut suite = base();
    suite["commands"][0]["candidate"] = json!(["{candidate}", "--mutate=dir"]);
    let report = run("emptydir", suite, &candidate());
    assert_eq!(report.failed(), 3, "{report:#?}");
    for r in &report.results {
        let Status::Fail(msgs) = &r.status else {
            panic!("{r:?}")
        };
        assert!(msgs.iter().any(|m| m.contains("\"made/\"")), "{msgs:?}");
    }
}

#[test]
fn an_unported_command_is_skipped_in_candidate_mode_only() {
    let mut suite = base();
    suite["commands"][0]["candidate"] = Value::Null;
    let report = run("unported", suite.clone(), &candidate());
    assert_eq!(report.skipped, vec!["echo".to_string()]);
    assert!(report.results.is_empty());
    assert!(
        !report.ok(),
        "a candidate run that compared nothing must fail"
    );

    let report = run("unported-self", suite, &Mode::SelfCheck);
    assert!(report.ok(), "{report:#?}");
    assert_eq!(report.results.len(), 3);
}

#[test]
fn the_cli_prints_skipped_commands() {
    let dir = work("cli-skip");
    let mut suite = base();
    suite["commands"][0]["candidate"] = Value::Null;
    let path = dir.join("suite.json");
    fs::write(&path, serde_json::to_vec(&suite).unwrap()).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vibe-parity"))
        .arg("run")
        .arg(&path)
        .arg("--candidate")
        .arg(env!("CARGO_BIN_EXE_parity-fixture"))
        .output()
        .unwrap();
    let text = String::from_utf8_lossy(&out.stdout);
    assert_eq!(out.status.code(), Some(1), "{text}");
    assert!(
        text.contains("SKIP  echo (not ported; self-check only)"),
        "{text}"
    );
    assert!(text.contains(", 1 skipped ("), "{text}");
}
