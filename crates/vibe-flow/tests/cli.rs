#![allow(clippy::unwrap_used, clippy::expect_used)]
//! The CLI surface: exit codes and stdout/stderr bytes per subcommand.

mod support;

use std::ffi::OsString;
use std::fs;
use std::path::Path;

use support::{
    MapEnv, install_policy, run, run_argv, run_with, scratch, shipped_policy, write_cursor,
};

const IDLE_CURSOR: &str = "{\n  \"flow\": \"idle\",\n  \"phase\": \"idle\",\n  \"feature\": null,\n  \"updated\": null,\n  \"state\": \"idle\"\n}\n";

#[test]
fn state_get_prints_the_idle_cursor() {
    let root = support::root("get-idle");
    let (code, stdout, stderr) = run(&root, &["state", "get"]);
    assert_eq!(code, 0);
    assert_eq!(stdout, IDLE_CURSOR);
    assert!(stderr.is_empty());
}

#[test]
fn state_get_reports_a_corrupt_cursor_on_stderr() {
    let root = support::root("get-corrupt");
    write_cursor(&root, "{ nope");
    let (code, stdout, stderr) = run(&root, &["state", "get"]);
    assert_eq!(code, 1);
    assert!(stdout.is_empty());
    assert!(
        stderr.starts_with("vibe state: ERROR: cursor at "),
        "{stderr}"
    );
    assert!(stderr.contains("is malformed: "), "{stderr}");
    assert!(stderr.ends_with('\n'));
}

#[test]
fn state_set_without_a_target_is_a_usage_error() {
    let root = support::root("set-usage");
    let (code, stdout, stderr) = run(&root, &["state", "set"]);
    assert_eq!(code, 1);
    assert!(stdout.is_empty());
    assert_eq!(
        stderr,
        "vibe state: ERROR: no target state given. Usage: vibe state set <flow.phase> [feature]\n"
    );
}

#[test]
fn state_set_refuses_an_unknown_target_with_the_sorted_state_list() {
    let root = support::root("set-unknown");
    let (code, stdout, stderr) = run(&root, &["state", "set", "nope"]);
    assert_eq!(code, 1);
    assert!(stdout.is_empty());
    let lines: Vec<&str> = stderr.lines().collect();
    assert_eq!(lines.len(), 2, "{stderr}");
    assert_eq!(lines[0], "vibe state: ERROR: 'nope' is not a known state.");
    assert_eq!(
        lines[1],
        "vibe state: Known states: feature.compound, feature.design, feature.impl, feature.plan, feature.verify, idle, quick.fix, quick.triage, quick.verify, setup.apply, setup.detect, strategy.brainstorm, strategy.spec"
    );
}

#[test]
fn state_set_writes_and_echoes_the_next_states() {
    let root = support::root("set-ok");
    let (code, stdout, stderr) = run(&root, &["state", "set", "feature.plan", "auth"]);
    assert_eq!(code, 0);
    assert_eq!(
        stdout,
        "-> feature.plan\n   next: feature.impl, feature.design, idle\n"
    );
    assert!(stderr.is_empty());
    let cursor = fs::read_to_string(root.join(".vibe/run/flow/state.json")).expect("cursor");
    assert!(cursor.starts_with("{\n  \"flow\": \"feature\",\n  \"phase\": \"plan\",\n  \"feature\": \"auth\",\n  \"updated\": \""), "{cursor}");
}

#[test]
fn state_set_warns_about_a_feature_state_without_a_feature() {
    let root = support::root("set-warn");
    let (code, _, stderr) = run(&root, &["state", "set", "feature.impl"]);
    assert_eq!(code, 0);
    assert_eq!(
        stderr,
        "vibe state: WARN: entering 'feature.impl' with no feature set. Pass one: vibe state set feature.impl <feature>\n"
    );
}

#[test]
fn state_set_moving_to_idle_clears_the_feature() {
    let root = support::root("set-idle");
    assert_eq!(run(&root, &["state", "set", "quick.fix", "bug"]).0, 0);
    assert_eq!(run(&root, &["state", "set", "idle"]).0, 0);
    let (_, stdout, _) = run(&root, &["state", "get"]);
    assert!(stdout.contains("\"feature\": null"), "{stdout}");
}

#[test]
fn policy_decide_answers_a_verdict_with_exit_zero() {
    let root = support::root("decide");
    install_policy(&root, &shipped_policy());
    let (code, stdout, stderr) = run(&root, &["policy", "decide", "docs/spec/tech.md", "idle"]);
    assert_eq!(code, 0);
    assert!(
        stdout.starts_with("block:root docs/spec specs are writable only during"),
        "{stdout}"
    );
    assert!(stdout.ends_with("(current: idle)\n"), "{stdout}");
    assert!(stderr.is_empty());

    let (code, stdout, _) = run(&root, &["policy", "decide", ".planning/x", "idle"]);
    assert_eq!(code, 0);
    assert_eq!(stdout, "allow\n");
}

#[test]
fn policy_decide_refuses_with_exit_two() {
    let root = support::root("decide-refused");
    let (code, stdout, stderr) = run(&root, &["policy", "decide", "x.txt"]);
    assert_eq!(code, 2);
    assert!(stdout.is_empty());
    assert_eq!(
        stderr,
        "vibe policy decide: refusing to answer — no usable rules loaded (the policy loaded to an empty rule set)\n"
    );
}

#[test]
fn policy_check_refuses_with_exit_one() {
    let root = support::root("check-refused");
    let (code, stdout, stderr) = run(&root, &["policy", "check", "x.txt"]);
    assert_eq!(code, 1);
    assert!(stdout.is_empty());
    assert!(
        stderr.ends_with(
            "refusing to answer — no usable rules loaded (the policy loaded to an empty rule set)\n"
        ),
        "{stderr}"
    );
}

#[test]
fn policy_check_answers_when_the_policy_loads() {
    let root = support::root("check-ok");
    install_policy(&root, &shipped_policy());
    let (code, stdout, _) = run(&root, &["policy", "check", "src/x.rs", "feature.verify"]);
    assert_eq!(code, 0);
    assert!(stdout.starts_with("warn:verify writes no src"), "{stdout}");
}

#[test]
fn policy_decide_requires_a_path() {
    let root = support::root("decide-usage");
    for sub in ["decide", "check"] {
        let (code, stdout, stderr) = run(&root, &["policy", sub]);
        assert_eq!(code, 1);
        assert!(stdout.is_empty());
        assert_eq!(
            stderr,
            "vibe policy decide: usage: vibe policy decide <path> [state]\n"
        );
    }
}

#[test]
fn policy_list_prints_one_line_per_rule() {
    let root = support::root("list");
    install_policy(&root, &shipped_policy());
    let (code, stdout, stderr) = run(&root, &["policy", "list"]);
    assert_eq!(code, 0);
    assert!(stderr.is_empty());
    assert_eq!(stdout.lines().count(), 12);
    assert_eq!(stdout.lines().next(), Some("policy: 11 rule(s)"));
    assert!(
        stdout.contains("cursor  match=.agents/skills/vibe/state.json, .vibe/run/flow/state.json  arms=[*->block]"),
        "{stdout}"
    );
    assert!(
        stdout.contains("gsd-planning  match=.planning/*  arms=[idle->allow, *->block]"),
        "{stdout}"
    );
    assert!(stdout.contains("arms=[feature.verify->warn, quick.verify->warn, feature.impl|quick.fix|setup.apply->allow, *->warn]"), "{stdout}");
}

#[test]
fn policy_render_prints_the_invariants() {
    let root = support::root("render");
    install_policy(&root, &shipped_policy());
    let (code, stdout, stderr) = run(&root, &["policy", "render"]);
    assert_eq!(code, 0);
    assert!(stderr.is_empty());
    assert_eq!(stdout.lines().count(), 11);
    assert!(stdout.ends_with('\n'));
}

#[test]
fn policy_render_prints_nothing_for_an_empty_policy() {
    let root = support::root("render-empty");
    let (code, stdout, _) = run(&root, &["policy", "render"]);
    assert_eq!(code, 0);
    assert!(stdout.is_empty());
}

#[test]
fn a_leading_vibe_dir_pins_the_policy_data() {
    let root = support::root("pinned");
    let pinned = scratch("pinned-data");
    fs::create_dir_all(pinned.join("content")).expect("content dir");
    fs::write(
        pinned.join("content/policy.json"),
        r#"{"rules":[{"id":"pin","match":"x.txt","arms":[{"states":"*","verdict":"warn","reason":"pinned"}]}]}"#,
    )
    .expect("write pinned policy");
    let pinned = pinned.display().to_string();
    let (code, stdout, stderr) = run(
        &root,
        &["policy", "--vibe-dir", &pinned, "decide", "x.txt", "idle"],
    );
    assert_eq!(code, 0);
    assert_eq!(stdout, "warn:pinned\n");
    assert!(stderr.is_empty());

    // The same flag in its `=` form, and a missing value dropped rather than
    // treated as an error.
    let (code, stdout, _) = run(
        &root,
        &[
            "policy",
            &format!("--vibe-dir={pinned}"),
            "decide",
            "x.txt",
            "idle",
        ],
    );
    assert_eq!(code, 0);
    assert_eq!(stdout, "warn:pinned\n");
    let (code, _, _) = run(&root, &["policy", "--vibe-dir"]);
    assert_ne!(code, 0);
}

#[test]
fn the_root_flag_beats_the_environment() {
    let root = support::root("root-flag");
    let other = support::root("root-other");
    let other_root = other.display().to_string();
    let env = MapEnv::new(&[("CLAUDE_PROJECT_DIR", other_root.as_str())]);
    let (code, stdout, _) = run_with(&root, &["state", "get"], &env);
    assert_eq!(code, 0);
    assert_eq!(stdout, IDLE_CURSOR);
}

#[test]
fn claude_project_dir_and_vibe_root_resolve_the_root() {
    let root = support::root("env-root");
    let value = root.display().to_string();
    let argv: Vec<OsString> = ["vibe-flow", "state", "get"]
        .iter()
        .map(OsString::from)
        .collect();
    for key in ["CLAUDE_PROJECT_DIR", "VIBE_ROOT"] {
        let env = MapEnv::new(&[(key, value.as_str())]);
        let (code, stdout, stderr) = run_argv(Path::new("/"), &argv, &env);
        assert_eq!(code, 0, "{key}: {stderr}");
        assert_eq!(stdout, IDLE_CURSOR, "{key}");
    }
}

#[test]
fn json_is_accepted_and_ignored() {
    let root = support::root("json");
    let (code, stdout, _) = run(&root, &["--json", "state", "get"]);
    assert_eq!(code, 0);
    assert_eq!(stdout, IDLE_CURSOR);
    let (code, stdout, _) = run(&root, &["state", "get", "--json"]);
    assert_eq!(code, 0);
    assert_eq!(stdout, IDLE_CURSOR);
}
