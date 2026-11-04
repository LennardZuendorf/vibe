#![cfg(unix)]
#![allow(clippy::unwrap_used, clippy::expect_used)]
//! The harness passes a faithful candidate and fails each planted mutant.
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use serde_json::{Value, json};
use std::collections::BTreeMap;
use vibe_parity::harness::{Mode, Report, Status, execute, run_suite};
use vibe_parity::suite::{Suite, normalize_bytes};

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

#[test]
fn cursor_updated_blanks_both_spellings() {
    let blanked = |bytes: &[u8]| normalize_bytes("cursor-updated", bytes).unwrap();
    assert_eq!(
        blanked(b"{\"updated\": \"2026-01-01T00:00:00Z\"}\n"),
        b"{\"updated\": \"T\"}\n"
    );
    assert_eq!(
        blanked(b"{\"updated\":\"2030-01-01T00:00:00Z\"}"),
        b"{\"updated\":\"T\"}"
    );
    assert_eq!(blanked(b"{\"updated\": null}"), b"{\"updated\": null}");
    assert_eq!(blanked(b"updated keys stay"), b"updated keys stay");
    // an escaped quote inside the value does not end it
    assert_eq!(blanked(b"x\"updated\":\"a\\\"b\"y"), b"x\"updated\":\"T\"y");
    // an unterminated value is left alone
    assert_eq!(blanked(b"{\"updated\": \"open"), b"{\"updated\": \"open");
    // a second value on the same file blanks too
    assert_eq!(
        blanked(b"{\"a\": \"x\"}\n{\"updated\": \"y\"}"),
        b"{\"a\": \"x\"}\n{\"updated\": \"T\"}"
    );
}

#[test]
fn an_unknown_normalizer_name_fails_suite_load() {
    let dir = work("unknown-normalizer");
    let mut suite = base();
    suite["normalize"] = json!(["bogus"]);
    let path = dir.join("suite.json");
    fs::write(&path, serde_json::to_vec(&suite).unwrap()).unwrap();
    let err = Suite::load(&path).unwrap_err();
    assert!(err.contains("unknown normalizer \"bogus\""), "{err}");
    assert!(err.contains("cursor-updated"), "{err}");
}

#[test]
fn cursor_updated_normalizes_written_files_on_both_sides() {
    let mut suite = base();
    suite["normalize"] = json!(["cursor-updated"]);
    suite["commands"][0]["candidate"] = json!(["{candidate}", "--mutate=updated"]);
    let report = run("normalize", suite.clone(), &candidate());
    assert!(report.ok(), "{report:#?}");
    assert_eq!(report.passed(), 3);

    let mut suite = base();
    suite["commands"][0]["candidate"] = json!(["{candidate}", "--mutate=updated"]);
    let report = run("normalize-control", suite, &candidate());
    assert_eq!(report.failed(), 3, "{report:#?}");
}

#[test]
fn a_file_alias_renames_the_key_on_both_sides() {
    let mut suite = base();
    suite["file_aliases"] = json!({ "out.txt": "renamed.txt" });
    suite["commands"][0]["candidate"] = json!(["{candidate}", "--mutate=path"]);
    let report = run("alias", suite.clone(), &Mode::SelfCheck);
    assert!(report.ok(), "{report:#?}");
    let report = run("alias-candidate", suite, &candidate());
    assert!(report.ok(), "{report:#?}");
    assert_eq!(report.passed(), 3);

    let mut suite = base();
    suite["commands"][0]["candidate"] = json!(["{candidate}", "--mutate=path"]);
    let report = run("alias-control", suite, &candidate());
    assert_eq!(report.failed(), 3, "{report:#?}");
    for r in &report.results {
        let Status::Fail(msgs) = &r.status else {
            panic!("{r:?}")
        };
        assert!(
            msgs.iter()
                .any(|m| m.contains("[\"out.txt\", \"renamed.txt\"]")),
            "{msgs:?}"
        );
    }
}

#[test]
fn an_alias_never_clobbers_a_key_the_side_already_holds() {
    let mut suite = base();
    suite["file_aliases"] = json!({ "out.txt": "seed.txt" });
    let dir = work("alias-clobber");
    let path = dir.join("suite.json");
    fs::write(&path, serde_json::to_vec(&suite).unwrap()).unwrap();
    let suite = Suite::load(&path).unwrap();
    let mut files = BTreeMap::new();
    files.insert("out.txt".to_string(), b"written".to_vec());
    files.insert("seed.txt".to_string(), b"staged".to_vec());
    let canon = suite.canonical_files(files).unwrap();
    assert_eq!(
        canon.get("out.txt").map(Vec::as_slice),
        Some(b"written".as_slice())
    );
    assert_eq!(
        canon.get("seed.txt").map(Vec::as_slice),
        Some(b"staged".as_slice())
    );
}

// ---------------------------------------------------------------------------
// Hook-shaped case fields: stdin, stderr, env, stamps, git. The mechanism is
// proven directly on run_child/apply_stamps/sandbox_git; the suite-level
// failure paths (a bad stamp, a failed setup commit) are proven through a
// one-case run — a green that cannot distinguish "checked" from "checked
// nothing" is not a green.
// ---------------------------------------------------------------------------

#[test]
fn run_child_feeds_stdin_from_a_file_and_captures_stderr() {
    let dir = work("run-child-io");
    let stdin = dir.join("stdin.bin");
    fs::write(&stdin, b"payload line\n").unwrap();
    let mut env = BTreeMap::new();
    env.insert("PROBE".to_string(), "yes".to_string());
    let argv = vec![
        "bash".to_string(),
        "-c".to_string(),
        "cat; printf '%s' \"$PROBE\" >&2".to_string(),
    ];
    let (exit, stdout, stderr) =
        vibe_parity::harness::run_child_for_tests(&argv, &dir, &dir, Some(&stdin), &env).unwrap();
    assert_eq!(exit, Some(0));
    assert_eq!(stdout, b"payload line\n");
    assert_eq!(stderr, b"yes");
}

#[test]
fn run_child_without_stdin_sees_an_empty_stream() {
    let dir = work("run-child-empty");
    let argv = vec!["bash".to_string(), "-c".to_string(), "cat".to_string()];
    let (exit, stdout, stderr) =
        vibe_parity::harness::run_child_for_tests(&argv, &dir, &dir, None, &BTreeMap::new())
            .unwrap();
    assert_eq!(exit, Some(0));
    assert!(stdout.is_empty());
    assert!(stderr.is_empty());
}

#[test]
fn rfc3339_parsing_accepts_real_timestamps_and_rejects_garbage() {
    use vibe_parity::suite::parse_rfc3339;
    assert_eq!(parse_rfc3339("1970-01-01T00:00:00Z").unwrap(), 0);
    assert_eq!(
        parse_rfc3339("2026-10-24T12:34:56Z").unwrap(),
        1_792_845_296
    );
    for bad in [
        "",
        "2026-10-24",
        "2026-13-01T00:00:00Z",
        "2026-00-10T00:00:00Z",
        "2026-10-32T00:00:00Z",
        "2026-10-24 12:00:00Z",
        "2026-10-24T12:00:00",
        "2026-10-24T25:00:00Z",
        "x026-10-24T12:00:00Z",
    ] {
        assert!(parse_rfc3339(bad).is_err(), "{bad:?} parsed");
    }
}

#[test]
fn stamps_set_the_mtime_and_a_missing_path_fails_loudly() {
    use vibe_parity::harness::apply_stamps_for_tests;
    let dir = work("stamps");
    let file = dir.join("receipt.md");
    fs::write(&file, b"x").unwrap();
    let mut stamps = BTreeMap::new();
    stamps.insert("receipt.md".to_string(), "2026-10-01T00:00:00Z".to_string());
    apply_stamps_for_tests(&dir, &stamps).unwrap();
    let secs = fs::metadata(&file)
        .unwrap()
        .modified()
        .unwrap()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    assert_eq!(secs, 1_790_812_800);
    let mut missing = BTreeMap::new();
    missing.insert("absent.md".to_string(), "2026-10-01T00:00:00Z".to_string());
    assert!(apply_stamps_for_tests(&dir, &missing).is_err());
}

#[test]
fn git_sandboxing_commits_and_a_commit_of_nothing_fails_loudly() {
    let dir = work("git-ok");
    fs::write(dir.join("seed.txt"), b"x").unwrap();
    vibe_parity::harness::sandbox_git_for_tests(&dir, true).unwrap();
    assert!(dir.join(".git").is_dir());
    let log = Command::new("git")
        .args(["log", "--oneline"])
        .current_dir(&dir)
        .env_clear()
        .env("PATH", std::env::var_os("PATH").unwrap_or_default())
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_SYSTEM", "/dev/null")
        .output()
        .unwrap();
    assert!(log.status.success());
    assert!(!log.stdout.is_empty());
    let empty = work("git-empty");
    fs::create_dir_all(&empty).unwrap();
    assert!(vibe_parity::harness::sandbox_git_for_tests(&empty, true).is_err());
}

#[test]
fn a_case_with_an_unparsable_stamp_fails_the_run() {
    // The stamp is validated at case-load time: a case whose stamp cannot
    // parse never runs — the error is a suite error naming the case file,
    // never a silently skipped or silently passing run.
    let report = one_case(
        "stamp-garbage",
        r#"{ "v": 1, "stamps": { "out.txt": "not-a-time" } }"#,
    );
    assert!(report.results.is_empty(), "the case must not run");
    assert!(
        report
            .errors
            .iter()
            .any(|e| e.contains("not-a-time") && e.contains("stamps")),
        "{:?}",
        report.errors
    );
}

#[test]
fn a_case_can_feed_stdin_and_compare_stderr_through_a_full_run() {
    // A one-case suite whose oracle copies stdin into out.txt and prints to
    // stderr: the self-check proves the plumbing delivers the payload to
    // both sides, byte-exactly.
    let dir = work("stdin-suite");
    let oracle = dir.join("oracle");
    fs::create_dir_all(&oracle).unwrap();
    fs::write(
        oracle.join("cat.sh"),
        "#!/usr/bin/env bash\nset -euo pipefail\ncat >out.txt\necho hook-noise >&2\n",
    )
    .unwrap();
    let case = dir.join("corpus/basic/one");
    fs::create_dir_all(case.join("tree")).unwrap();
    fs::write(
        case.join("case.json"),
        r#"{ "v": 1, "stdin": "{\"tool_name\":\"Bash\"}\n", "stderr": true }"#,
    )
    .unwrap();
    let suite = json!({
        "v": 1, "name": "stdin",
        "corpus": dir.join("corpus"),
        "oracle_root": dir.join("oracle"),
        "commands": [
            { "name": "cat", "oracle": ["bash", "{oracle}/cat.sh"], "candidate": ["bash", "{oracle}/cat.sh"], "classes": ["basic"] }
        ],
        "floors": { "basic": 1 }
    });
    let path = dir.join("suite.json");
    fs::write(&path, serde_json::to_vec(&suite).unwrap()).unwrap();
    let loaded = Suite::load(&path).unwrap();
    let report = run_suite(&loaded, &Mode::SelfCheck, &dir.join("work"));
    assert!(report.ok(), "{:?}", report.errors);
    assert_eq!(report.passed(), 1);
}
