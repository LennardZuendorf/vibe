// vibe-spec: literal-file
#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Policy parity: the shipped policy as a pinned fixture, a matrix over every
//! rule, all 13 machine states and the path spellings, and one mutation per
//! rule class (each mutation must move a verdict, or the test fails).
//!
//! The fixture (`fixtures/policy.json`) is the shipped policy with every
//! occurrence of the legacy root name replaced by `@LEGACY@`: a tracked JSON
//! file cannot spell that name, because the literal sweep's file marker has no
//! JSON comment form. The first test reconstructs the name and asserts the
//! result is byte-identical to `flow/content/policy.json`, so the fixture can
//! never drift from the shipped rules.

mod support;

use std::path::{Path, PathBuf};

use serde_json::Value;
use vibe_flow::machine::Machine;
use vibe_flow::policy::{Loaded, Verdict, decide, load_policy};

const TOKEN: &str = "@LEGACY@";
const LEGACY: &str = ".spec";

fn fixture_text() -> String {
    include_str!("fixtures/policy.json").replace(TOKEN, LEGACY)
}

fn base() -> Value {
    serde_json::from_str(&fixture_text()).expect("fixture is JSON")
}

fn data_dir(root: &Path) -> PathBuf {
    root.join(".agents").join("flow")
}

fn shipped_machine() -> Machine {
    Machine::load(&support::manifest().join("../../flow")).expect("shipped machine")
}

/// A rule's own path: its pattern with every wildcard replaced by a plain
/// segment that matches no other rule.
fn canonical(pattern: &str) -> String {
    pattern.replace('*', "zz").replace('?', "q")
}

/// Every spelling of a canonical path that must answer identically.
fn spellings(path: &str) -> Vec<String> {
    vec![
        format!("./{path}"),
        format!("././{path}"),
        format!("/abs/root/{path}"),
        format!("a/b/{path}"),
        path.replace('/', "//"),
        path.replace('/', "/./"),
        format!("x/../{path}"),
    ]
}

fn admits(arm: &Value, state: &str) -> bool {
    match arm.get("states") {
        Some(Value::String(text)) if text == "*" => true,
        Some(Value::Array(list)) => list.iter().any(|entry| entry.as_str() == Some(state)),
        _ => false,
    }
}

fn verdict_of(text: Option<&str>) -> Verdict {
    match text {
        Some("allow") => Verdict::Allow,
        Some("warn") => Verdict::Warn,
        _ => Verdict::Block,
    }
}

/// The verdicts the FILE prescribes for one rule across every state: the first
/// arm whose `states` admits the state, else allow.
fn expected_verdicts(raw_rule: &Value, states: &[String]) -> Vec<(String, Verdict)> {
    let arms = raw_rule["arms"].as_array().expect("arms array");
    states
        .iter()
        .map(|state| {
            let verdict = arms
                .iter()
                .find(|arm| admits(arm, state))
                .map(|arm| verdict_of(arm.get("verdict").and_then(Value::as_str)))
                .unwrap_or(Verdict::Allow);
            (state.clone(), verdict)
        })
        .collect()
}

fn load(text: &str, name: &str) -> Loaded {
    let dir = support::scratch(name);
    support::install_policy(&dir, text);
    load_policy(&data_dir(&dir))
}

fn verdict_text(text: &str, name: &str, path: &str, state: &str) -> Verdict {
    decide(&load(text, name).policy, path, state).verdict
}

fn verdict_value(value: &Value, name: &str, path: &str, state: &str) -> Verdict {
    verdict_text(
        &serde_json::to_string(value).expect("encode policy"),
        name,
        path,
        state,
    )
}

#[test]
fn the_fixture_reconstructs_the_shipped_policy_byte_for_byte() {
    assert_eq!(fixture_text(), support::shipped_policy());
    assert!(!fixture_text().contains(TOKEN));
    assert!(!include_str!("fixtures/policy.json").contains(LEGACY));
}

#[test]
fn the_matrix_covers_every_rule_every_state_and_every_spelling() {
    let machine = shipped_machine();
    let states = machine.state_keys();
    assert_eq!(states.len(), 13);

    let text = fixture_text();
    let loaded = load(&text, "matrix");
    assert!(loaded.errors.is_empty(), "{:?}", loaded.errors);

    let raw = base();
    let raw_rules = raw["rules"].as_array().expect("rules array");
    assert_eq!(loaded.policy.rules.len(), 11);
    assert_eq!(loaded.policy.rules.len(), raw_rules.len());

    let mut cases = 0usize;
    let mut spelled = 0usize;
    let mut patterns = 0usize;
    for (index, raw_rule) in raw_rules.iter().enumerate() {
        let rule = &loaded.policy.rules[index];
        let expected = expected_verdicts(raw_rule, &states);
        assert_eq!(expected.len(), 13);
        for pattern in &rule.patterns {
            patterns += 1;
            let path = canonical(pattern);
            for (state, want) in &expected {
                let decision = decide(&loaded.policy, &path, state);
                assert_eq!(
                    decision.verdict, *want,
                    "rule '{}' pattern {pattern:?} at {state}",
                    rule.id
                );
                assert_eq!(
                    decision.rule_id.as_deref(),
                    Some(rule.id.as_str()),
                    "rule '{}' did not answer for {path:?} at {state}",
                    rule.id
                );
                cases += 1;
                for spelling in spellings(&path) {
                    let got = decide(&loaded.policy, &spelling, state).verdict;
                    assert_eq!(
                        got, *want,
                        "spelling {spelling:?} of {path:?} at {state} (rule '{}')",
                        rule.id
                    );
                    spelled += 1;
                }
            }
        }
    }
    assert_eq!(patterns, 20);
    assert_eq!(cases, 20 * 13);
    assert_eq!(spelled, 20 * 13 * 7);
}

#[test]
fn a_dropped_allow_arm_changes_the_verdict() {
    let mut mutated = base();
    mutated["rules"][1]["arms"]
        .as_array_mut()
        .expect("arms")
        .remove(0);
    let (path, state) = (".spec/lessons.md", "feature.compound");
    assert_eq!(
        verdict_text(&fixture_text(), "base", path, state),
        Verdict::Allow
    );
    assert_eq!(
        verdict_value(&mutated, "drop-allow", path, state),
        Verdict::Block
    );
}

#[test]
fn an_inverted_verdict_changes_the_verdict() {
    let mut mutated = base();
    mutated["rules"][9]["arms"][0]["verdict"] = Value::String("block".to_string());
    let (path, state) = ("CLAUDE.md", "idle");
    assert_eq!(
        verdict_text(&fixture_text(), "base", path, state),
        Verdict::Warn
    );
    assert_eq!(
        verdict_value(&mutated, "invert", path, state),
        Verdict::Block
    );
}

#[test]
fn an_unknown_state_in_an_arm_changes_the_verdict() {
    let mut mutated = base();
    mutated["rules"][1]["arms"][0]["states"] = serde_json::json!(["nope"]);
    let (path, state) = (".spec/lessons.md", "feature.compound");
    assert_eq!(
        verdict_text(&fixture_text(), "base", path, state),
        Verdict::Allow
    );
    assert_eq!(
        verdict_value(&mutated, "unknown-state", path, state),
        Verdict::Block
    );
}

#[test]
fn a_glob_that_must_span_an_extra_segment_changes_the_verdict() {
    // `.planning/*` matches `.planning/a` only because `*` crosses `/`; the
    // mutation requires a second `/`, so the rule stops matching.
    let mut mutated = base();
    mutated["rules"][5]["match"] = serde_json::json!([".planning/*/*"]);
    let (path, state) = (".planning/a", "setup.apply");
    assert_eq!(
        verdict_text(&fixture_text(), "base", path, state),
        Verdict::Block
    );
    assert_eq!(
        verdict_value(&mutated, "glob-deep", path, state),
        Verdict::Allow
    );
}

#[test]
fn a_missing_id_changes_the_verdict_and_reports() {
    let mut mutated = base();
    mutated["rules"][0]
        .as_object_mut()
        .expect("rule")
        .remove("id");
    let (path, state) = (".agents/skills/vibe/state.json", "idle");
    assert_eq!(
        verdict_text(&fixture_text(), "base", path, state),
        Verdict::Block
    );
    assert_eq!(
        verdict_value(&mutated, "missing-id", path, state),
        Verdict::Allow
    );
    let loaded = load(
        &serde_json::to_string(&mutated).expect("encode"),
        "missing-id-errors",
    );
    assert!(
        loaded
            .errors
            .iter()
            .any(|err| err == "policy: rules[0] is missing an 'id'"),
        "{:?}",
        loaded.errors
    );
}

#[test]
fn an_empty_arms_array_changes_the_verdict_and_reports() {
    let mut mutated = base();
    mutated["rules"][0]["arms"] = serde_json::json!([]);
    let (path, state) = (".agents/skills/vibe/state.json", "idle");
    assert_eq!(
        verdict_text(&fixture_text(), "base", path, state),
        Verdict::Block
    );
    assert_eq!(
        verdict_value(&mutated, "no-arms", path, state),
        Verdict::Allow
    );
    let loaded = load(
        &serde_json::to_string(&mutated).expect("encode"),
        "no-arms-errors",
    );
    assert_eq!(loaded.errors.len(), 1, "{:?}", loaded.errors);
    assert!(
        loaded.errors[0].ends_with("has no usable 'arms'"),
        "{:?}",
        loaded.errors
    );

    // An arms key that is absent (not an array) is reported twice: once for
    // the missing array, once for the resulting empty arm list.
    let mut absent = base();
    absent["rules"][0]
        .as_object_mut()
        .expect("rule")
        .remove("arms");
    let loaded = load(
        &serde_json::to_string(&absent).expect("encode"),
        "absent-arms-errors",
    );
    assert_eq!(loaded.errors.len(), 2, "{:?}", loaded.errors);
    assert!(
        loaded.errors[0].ends_with("has no 'arms' array"),
        "{:?}",
        loaded.errors
    );
    assert!(
        loaded.errors[1].ends_with("has no usable 'arms'"),
        "{:?}",
        loaded.errors
    );
}

#[test]
fn an_unknown_verdict_changes_the_verdict_and_reports() {
    let mut mutated = base();
    mutated["rules"][1]["arms"][0]["verdict"] = Value::String("nope".to_string());
    let (path, state) = (".spec/lessons.md", "feature.compound");
    assert_eq!(
        verdict_text(&fixture_text(), "base", path, state),
        Verdict::Allow
    );
    assert_eq!(
        verdict_value(&mutated, "unknown-verdict", path, state),
        Verdict::Block
    );
    let loaded = load(
        &serde_json::to_string(&mutated).expect("encode"),
        "unknown-verdict-errors",
    );
    assert_eq!(
        loaded.errors,
        vec![
            "policy: rule 'lessons' arms[0] has an unknown verdict 'nope' — defaulting to 'block'"
                .to_string()
        ]
    );
}

#[test]
fn a_future_version_changes_the_verdict_and_refuses_every_rule() {
    let mut mutated = base();
    mutated["version"] = serde_json::json!(2);
    let (path, state) = (".agents/skills/vibe/state.json", "idle");
    assert_eq!(
        verdict_text(&fixture_text(), "base", path, state),
        Verdict::Block
    );
    assert_eq!(
        verdict_value(&mutated, "version", path, state),
        Verdict::Allow
    );
    let loaded = load(
        &serde_json::to_string(&mutated).expect("encode"),
        "version-errors",
    );
    assert!(loaded.policy.rules.is_empty());
    assert_eq!(loaded.errors.len(), 1);
    assert!(
        loaded.errors[0].ends_with(
            "declares version 2, but this engine only understands version 1 — refusing to load its rules"
        ),
        "{:?}",
        loaded.errors
    );
}

#[test]
fn a_truncated_policy_changes_the_verdict_and_reports() {
    let text = fixture_text();
    let truncated = text.lines().take(10).collect::<Vec<_>>().join("\n");
    let (path, state) = (".agents/skills/vibe/state.json", "idle");
    assert_eq!(verdict_text(&text, "base", path, state), Verdict::Block);
    assert_eq!(
        verdict_text(&truncated, "truncated", path, state),
        Verdict::Allow
    );
    assert_eq!(load(&truncated, "truncated-errors").errors.len(), 1);
}

#[test]
fn an_empty_rule_list_changes_the_verdict() {
    let mut mutated = base();
    mutated["rules"] = serde_json::json!([]);
    let (path, state) = (".agents/skills/vibe/state.json", "idle");
    assert_eq!(
        verdict_text(&fixture_text(), "base", path, state),
        Verdict::Block
    );
    assert_eq!(
        verdict_value(&mutated, "empty-rules", path, state),
        Verdict::Allow
    );
}

#[test]
fn an_exact_rule_outranks_a_glob_rule_across_rules() {
    // The shipped `docs/spec/lessons.md` rule is exact and the superpowers
    // glob never reaches it; the cross-rule precedence itself is pinned with a
    // hand-built pair, since no shipped pair overlaps.
    let text = r#"{"rules":[
      {"id":"glob","match":"docs/*","arms":[{"states":"*","verdict":"block","reason":"glob"}]},
      {"id":"exact","match":"docs/spec/lessons.md","arms":[{"states":"*","verdict":"allow","reason":""}]}
    ]}"#;
    let loaded = load(text, "exact-first");
    assert!(loaded.errors.is_empty(), "{:?}", loaded.errors);
    assert_eq!(
        decide(&loaded.policy, "docs/spec/lessons.md", "idle")
            .rule_id
            .as_deref(),
        Some("exact")
    );
    assert_eq!(
        decide(&loaded.policy, "docs/other.md", "idle")
            .rule_id
            .as_deref(),
        Some("glob")
    );
}

#[test]
fn the_shipped_rules_answer_the_guarded_paths() {
    let loaded = load(&fixture_text(), "guarded");
    assert!(loaded.errors.is_empty(), "{:?}", loaded.errors);
    let policy = &loaded.policy;
    assert_eq!(
        decide(policy, ".agents/skills/vibe/state.json", "feature.impl").verdict,
        Verdict::Block
    );
    assert_eq!(
        decide(policy, ".spec/lessons.md", "quick.verify").verdict,
        Verdict::Allow
    );
    assert_eq!(
        decide(policy, "docs/spec/plan.md", "feature.compound").verdict,
        Verdict::Allow
    );
    assert_eq!(
        decide(policy, ".planning/x/y", "idle").verdict,
        Verdict::Allow
    );
    assert_eq!(
        decide(policy, ".planning/x/y", "quick.fix").verdict,
        Verdict::Block
    );
    assert_eq!(
        decide(policy, "docs/spec/features/a", "feature.impl").verdict,
        Verdict::Warn
    );
    assert_eq!(
        decide(policy, "docs/superpowers/specs/a.md", "quick.fix").verdict,
        Verdict::Warn
    );
    assert_eq!(
        decide(policy, "AGENTS.md", "setup.apply").verdict,
        Verdict::Warn
    );
    assert_eq!(
        decide(policy, "tests/x.rs", "feature.verify").verdict,
        Verdict::Warn
    );
    assert_eq!(
        decide(policy, "src/x.rs", "quick.fix").verdict,
        Verdict::Allow
    );
    assert_eq!(
        decide(policy, "src/x.rs", "strategy.spec").verdict,
        Verdict::Warn
    );
    assert_eq!(
        decide(policy, "unmatched/x", "idle").verdict,
        Verdict::Allow
    );
}
