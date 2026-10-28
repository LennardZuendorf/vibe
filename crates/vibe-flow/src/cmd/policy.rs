//! `vibe-flow policy` (mirrors `commands/policy.mjs`).
//!
//! `decide` is the byte-parity alias: an answered verdict exits 0, a refused
//! policy exits 2 (the oracle's refusal). `check` prints the same output but
//! exits 1 on refusal (FV-R5, contract-clean).

use crate::cmd::Outcome;
use crate::cursor::read_cursor;
use crate::paths::FlowPaths;
use crate::policy::{Arm, Rule, States, Verdict, decide, load_policy, render_invariants};

const USAGE: &str = "vibe policy decide: usage: vibe policy decide <path> [state]";

fn line(text: &str) -> String {
    format!("{text}\n")
}

/// Which policy subcommand to run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sub {
    /// Answer a verdict; a refusal exits 2.
    Decide,
    /// Answer a verdict; a refusal exits 1.
    Check,
    /// Print the loaded rules.
    List,
    /// Print the `{{invariants}}` prose.
    Render,
}

/// Runs one policy subcommand.
pub fn run(paths: &FlowPaths, sub: Sub, args: &[String]) -> Outcome {
    match sub {
        Sub::Decide => verdict(paths, args, 2),
        Sub::Check => verdict(paths, args, 1),
        Sub::List => list(paths),
        Sub::Render => render(paths),
    }
}

fn verdict(paths: &FlowPaths, args: &[String], refused_code: u8) -> Outcome {
    let target = args.first().map(String::as_str).unwrap_or("");
    if target.is_empty() {
        return Outcome::new(1, String::new(), line(USAGE));
    }
    let loaded = load_policy(paths.data_dir());
    let mut stderr = String::new();
    for err in &loaded.errors {
        stderr.push_str(&line(&format!("vibe policy: WARN — {err}")));
    }
    if !loaded.errors.is_empty() || loaded.policy.rules.is_empty() {
        let suffix = if loaded.errors.is_empty() {
            " (the policy loaded to an empty rule set)"
        } else {
            ""
        };
        stderr.push_str(&line(&format!(
            "vibe policy decide: refusing to answer — no usable rules loaded{suffix}"
        )));
        return Outcome::new(refused_code, String::new(), stderr);
    }
    let state = match args.get(1).filter(|text| !text.is_empty()) {
        Some(state) => state.clone(),
        None => current_state(paths),
    };
    let decision = decide(&loaded.policy, strip_leading_dot_slash(target), &state);
    Outcome::new(
        0,
        line(&verdict_line(&decision.verdict, &decision.reason)),
        stderr,
    )
}

/// The cursor's state key, degrading to idle on any failure.
fn current_state(paths: &FlowPaths) -> String {
    read_cursor(paths)
        .map(|cursor| cursor.state)
        .unwrap_or_else(|_| "idle".to_string())
}

/// The oracle's `path="${path#./}"`, applied before the matcher.
fn strip_leading_dot_slash(path: &str) -> &str {
    path.strip_prefix("./").unwrap_or(path)
}

fn verdict_line(verdict: &Verdict, reason: &str) -> String {
    match verdict {
        Verdict::Allow => "allow".to_string(),
        other => format!("{}:{reason}", other.as_str()),
    }
}

fn list(paths: &FlowPaths) -> Outcome {
    let loaded = load_policy(paths.data_dir());
    let mut stderr = String::new();
    for err in &loaded.errors {
        stderr.push_str(&line(&format!("vibe policy: WARN — {err}")));
    }
    let mut stdout = line(&format!("policy: {} rule(s)", loaded.policy.rules.len()));
    for rule in &loaded.policy.rules {
        stdout.push_str(&line(&rule_line(rule)));
    }
    let code = if loaded.errors.is_empty() { 0 } else { 1 };
    Outcome::new(code, stdout, stderr)
}

fn rule_line(rule: &Rule) -> String {
    let arms = rule
        .arms
        .iter()
        .map(arm_summary)
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "{}  match={}  arms=[{arms}]",
        rule.id,
        rule.patterns.join(", ")
    )
}

fn arm_summary(arm: &Arm) -> String {
    let states = match &arm.states {
        States::CatchAll => "*".to_string(),
        States::List(list) if list.is_empty() => "(none)".to_string(),
        States::List(list) => list.join("|"),
    };
    format!("{states}->{}", arm.verdict.as_str())
}

fn render(paths: &FlowPaths) -> Outcome {
    let loaded = load_policy(paths.data_dir());
    let mut stderr = String::new();
    for err in &loaded.errors {
        stderr.push_str(&line(&format!("vibe policy: WARN — {err}")));
    }
    let prose = render_invariants(&loaded.policy);
    let stdout = if prose.is_empty() {
        String::new()
    } else {
        format!("{prose}\n")
    };
    let code = if loaded.errors.is_empty() { 0 } else { 1 };
    Outcome::new(code, stdout, stderr)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::{Sub, run};
    use crate::paths::{Env, FlowPaths};
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};

    static COUNTER: AtomicU64 = AtomicU64::new(0);

    struct NoEnv;

    impl Env for NoEnv {
        fn get(&self, _key: &str) -> Option<String> {
            None
        }
    }

    fn scratch(name: &str) -> PathBuf {
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "vibe-flow-policycmd-{name}-{}-{n}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("create scratch");
        dir
    }

    fn root_with_policy(name: &str) -> (PathBuf, FlowPaths) {
        let root = scratch(name);
        let content = root.join(".agents").join("flow").join("content");
        fs::create_dir_all(&content).expect("content dir");
        fs::copy(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../flow/content/policy.json"),
            content.join("policy.json"),
        )
        .expect("copy policy");
        let paths = FlowPaths::resolve(Some(&root), None, &root, &NoEnv);
        (root, paths)
    }

    #[test]
    fn decide_prints_a_verdict_line_and_exits_zero() {
        let (_, paths) = root_with_policy("decide");
        let out = run(
            &paths,
            Sub::Decide,
            &[".planning/x".to_string(), "idle".to_string()],
        );
        assert_eq!(out.code, 0);
        assert_eq!(out.stdout, "allow\n");
        assert!(out.stderr.is_empty());
    }

    #[test]
    fn decide_reports_a_block_with_the_interpolated_reason() {
        let (_, paths) = root_with_policy("decide-block");
        let out = run(
            &paths,
            Sub::Decide,
            &["docs/spec/tech.md".to_string(), "idle".to_string()],
        );
        assert_eq!(out.code, 0);
        assert!(
            out.stdout
                .starts_with("block:root docs/spec specs are writable only during"),
            "{}",
            out.stdout
        );
        assert!(out.stdout.ends_with("(current: idle)\n"), "{}", out.stdout);
    }

    #[test]
    fn decide_requires_a_path() {
        let (_, paths) = root_with_policy("decide-usage");
        let out = run(&paths, Sub::Decide, &[]);
        assert_eq!(out.code, 1);
        assert_eq!(
            out.stderr,
            "vibe policy decide: usage: vibe policy decide <path> [state]\n"
        );
    }

    #[test]
    fn decide_strips_one_leading_dot_slash() {
        let (_, paths) = root_with_policy("decide-dot");
        let out = run(
            &paths,
            Sub::Decide,
            &["./.planning/x".to_string(), "idle".to_string()],
        );
        assert_eq!(out.stdout, "allow\n");
    }

    #[test]
    fn a_refused_policy_exits_two_for_decide_and_one_for_check() {
        let root = scratch("refused");
        let paths = FlowPaths::resolve(Some(&root), None, &root, &NoEnv);
        let args = ["x.txt".to_string(), "idle".to_string()];
        let decide = run(&paths, Sub::Decide, &args);
        assert_eq!(decide.code, 2);
        assert!(decide.stdout.is_empty());
        assert_eq!(
            decide.stderr,
            "vibe policy decide: refusing to answer — no usable rules loaded (the policy loaded to an empty rule set)\n"
        );
        let check = run(&paths, Sub::Check, &args);
        assert_eq!(check.code, 1);
        assert_eq!(check.stderr, decide.stderr);
    }

    #[test]
    fn a_refused_policy_with_errors_prints_one_warning_per_error() {
        let root = scratch("refused-errors");
        let content = root.join(".agents").join("flow").join("content");
        fs::create_dir_all(&content).expect("content dir");
        fs::write(content.join("policy.json"), "{ broken").expect("write policy");
        let paths = FlowPaths::resolve(Some(&root), None, &root, &NoEnv);
        let out = run(&paths, Sub::Decide, &["x.txt".to_string()]);
        assert_eq!(out.code, 2);
        assert!(
            out.stderr.starts_with("vibe policy: WARN — policy: "),
            "{}",
            out.stderr
        );
        assert!(
            out.stderr
                .ends_with("vibe policy decide: refusing to answer — no usable rules loaded\n"),
            "{}",
            out.stderr
        );
        assert!(!out.stderr.contains("empty rule set"), "{}", out.stderr);
    }

    #[test]
    fn the_state_argument_defaults_to_the_cursor() {
        let (root, paths) = root_with_policy("state-default");
        let dir = root.join(".vibe").join("run").join("flow");
        fs::create_dir_all(&dir).expect("cursor dir");
        fs::write(
            dir.join("state.json"),
            "{\"flow\":\"setup\",\"phase\":\"apply\"}",
        )
        .expect("write");
        let out = run(&paths, Sub::Decide, &["docs/spec/tech.md".to_string()]);
        assert_eq!(out.stdout, "allow\n");

        fs::write(dir.join("state.json"), "{ corrupt").expect("write");
        let out = run(&paths, Sub::Decide, &["docs/spec/tech.md".to_string()]);
        assert!(out.stdout.starts_with("block:"), "{}", out.stdout);
    }

    #[test]
    fn list_prints_the_loaded_rules() {
        let (_, paths) = root_with_policy("list");
        let out = run(&paths, Sub::List, &[]);
        assert_eq!(out.code, 0);
        assert!(
            out.stdout.starts_with("policy: 11 rule(s)\n"),
            "{}",
            out.stdout
        );
        assert!(
            out.stdout
                .contains("cursor  match=.agents/skills/vibe/state.json  arms=[*->block]"),
            "{}",
            out.stdout
        );
        assert!(
            out.stdout.contains("arms=[idle->allow, *->block]"),
            "{}",
            out.stdout
        );
    }

    #[test]
    fn render_prints_the_invariants() {
        let (_, paths) = root_with_policy("render");
        let out = run(&paths, Sub::Render, &[]);
        assert_eq!(out.code, 0);
        assert_eq!(out.stdout.lines().count(), 11);
        assert!(
            out.stdout
                .starts_with("- `.agents/skills/vibe/state.json` — otherwise: block"),
            "{}",
            out.stdout
        );
        assert!(out.stdout.ends_with('\n'));
    }

    #[test]
    fn render_prints_nothing_for_an_empty_policy() {
        let root = scratch("render-empty");
        let paths = FlowPaths::resolve(Some(&root), None, &root, &NoEnv);
        let out = run(&paths, Sub::Render, &[]);
        assert_eq!(out.code, 0);
        assert!(out.stdout.is_empty());
    }

    #[test]
    fn list_exits_one_when_the_policy_reported_errors() {
        let root = scratch("list-errors");
        let content = root.join(".agents").join("flow").join("content");
        fs::create_dir_all(&content).expect("content dir");
        fs::write(content.join("policy.json"), "{}").expect("write policy");
        let paths = FlowPaths::resolve(Some(&root), None, &root, &NoEnv);
        let out = run(&paths, Sub::List, &[]);
        assert_eq!(out.code, 1);
        assert_eq!(out.stdout, "policy: 0 rule(s)\n");
        assert!(
            out.stderr.contains("has no 'rules' array"),
            "{}",
            out.stderr
        );
    }
}
