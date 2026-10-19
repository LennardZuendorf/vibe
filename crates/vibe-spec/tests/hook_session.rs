//! `vibe-spec hook session-start`: the using-spec router (P4-R6).
#![allow(dead_code, clippy::unwrap_used, clippy::expect_used)]

mod common;

use std::fs;
use std::path::Path;

use serde_json::{Value, json};

fn payload(cwd: &Path) -> Vec<u8> {
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/hooks/session-start.json");
    let mut v: Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    v["cwd"] = json!(cwd);
    serde_json::to_vec(&v).unwrap()
}

fn run(p: &Path, env: &[(&str, &str)]) -> (i32, Value) {
    let out = common::spec_stdin(p, &["hook", "session-start"], env, &payload(p));
    let v = serde_json::from_slice(&out.stdout)
        .unwrap_or_else(|e| panic!("not JSON ({e}): {}\n{}", out.text(), out.stderr));
    (out.code, v)
}

fn rooted(name: &str) -> std::path::PathBuf {
    let p = common::project(name);
    common::write(&p, "docs/spec/README.md", "# Spec\n");
    p
}

#[test]
fn claude_shape() {
    let p = rooted("session-claude");
    let (code, v) = run(&p, &[("CLAUDE_PROJECT_DIR", "/x")]);
    assert_eq!(code, 0);
    let h = &v["hookSpecificOutput"];
    assert_eq!(h["hookEventName"], "SessionStart");
    let ctx = h["additionalContext"].as_str().unwrap();
    assert!(ctx.starts_with("<EXTREMELY_IMPORTANT>"), "{ctx}");
    assert!(ctx.contains("using-spec"), "{ctx}");
    assert!(ctx.ends_with("</EXTREMELY_IMPORTANT>"), "{ctx}");
    let (_, v) = run(&p, &[("CLAUDE_PLUGIN_ROOT", "/x")]);
    assert!(v["hookSpecificOutput"]["additionalContext"].is_string());
}

#[test]
fn cursor_and_other_shapes() {
    let p = rooted("session-shapes");
    let (code, v) = run(&p, &[("CURSOR_PLUGIN_ROOT", "/x")]);
    assert_eq!(code, 0);
    assert!(v["additional_context"].is_string(), "{v}");
    assert!(v.get("additionalContext").is_none());
    let (code, v) = run(&p, &[]);
    assert_eq!(code, 0);
    assert!(v["additionalContext"].is_string(), "{v}");
    assert!(v.get("additional_context").is_none());
}

#[test]
fn writes_the_session_marker() {
    let p = rooted("session-marker");
    assert!(!p.join(".vibe/spec-session").exists());
    run(&p, &[]);
    assert!(p.join(".vibe/spec-session").is_file());
}

#[test]
fn no_root_still_routes() {
    let p = common::project("session-noroot");
    let (code, v) = run(&p, &[]);
    assert_eq!(code, 0);
    assert!(
        v["additionalContext"]
            .as_str()
            .unwrap()
            .contains("using-spec")
    );
    assert!(!p.join(".vibe").exists());
}

#[test]
fn skill_is_within_budget() {
    let text = fs::read_to_string(common::repo().join("spec/using-spec/SKILL.md")).unwrap();
    let rest = text.strip_prefix("---\n").unwrap();
    let (front, body) = rest.split_once("\n---\n").unwrap();
    assert!(front.lines().any(|l| l == "name: using-spec"), "{front}");
    let words = body.split_whitespace().count();
    assert!(words <= 150, "{words} words");
}
