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

const LINE_SYMLINK: &str = "vibe-spec hook session-start: marker skipped — ";

fn injected(v: &Value) -> bool {
    v["additionalContext"]
        .as_str()
        .is_some_and(|c| c.contains("using-spec"))
}

#[cfg(unix)]
#[test]
fn symlinked_vibe_dir_is_not_followed() {
    let p = rooted("session-link-dir");
    let outside = common::project("session-outside");
    std::os::unix::fs::symlink(&outside, p.join(".vibe")).unwrap();
    let out = common::spec_stdin(&p, &["hook", "session-start"], &[], &payload(&p));
    assert_eq!(out.code, 0);
    assert!(injected(&serde_json::from_slice(&out.stdout).unwrap()));
    assert!(!outside.join("spec-session").exists());
    let want = format!("{LINE_SYMLINK}.vibe is a symlink");
    assert_eq!(
        out.stderr.lines().filter(|l| *l == want).count(),
        1,
        "{}",
        out.stderr
    );
}

#[cfg(unix)]
#[test]
fn symlinked_marker_is_not_followed() {
    let p = rooted("session-link-file");
    common::write(&p, "target.txt", "keep");
    fs::create_dir_all(p.join(".vibe")).unwrap();
    std::os::unix::fs::symlink(p.join("target.txt"), p.join(".vibe/spec-session")).unwrap();
    let out = common::spec_stdin(&p, &["hook", "session-start"], &[], &payload(&p));
    assert_eq!(out.code, 0);
    assert!(injected(&serde_json::from_slice(&out.stdout).unwrap()));
    assert_eq!(fs::read(p.join("target.txt")).unwrap(), b"keep");
    let want = format!("{LINE_SYMLINK}.vibe/spec-session is a symlink");
    assert_eq!(
        out.stderr.lines().filter(|l| *l == want).count(),
        1,
        "{}",
        out.stderr
    );
}

#[test]
fn file_named_vibe_is_skipped() {
    let p = rooted("session-vibe-file");
    common::write(&p, ".vibe", "x");
    let out = common::spec_stdin(&p, &["hook", "session-start"], &[], &payload(&p));
    assert_eq!(out.code, 0);
    assert!(
        out.stderr
            .contains(&format!("{LINE_SYMLINK}.vibe is not a directory")),
        "{}",
        out.stderr
    );
}

#[test]
fn bad_input_still_routes() {
    let p = common::project("session-bad");
    let out = common::spec_stdin(&p, &["hook", "session-start"], &[], b"junk");
    assert_eq!(out.code, 0);
    assert!(injected(&serde_json::from_slice(&out.stdout).unwrap()));
    let gone = p.join("does-not-exist");
    let out = common::spec_stdin(&p, &["hook", "session-start"], &[], &payload(&gone));
    assert_eq!(out.code, 0);
    assert!(injected(&serde_json::from_slice(&out.stdout).unwrap()));
}

#[test]
fn second_run_refreshes_the_marker() {
    let p = rooted("session-mtime");
    run(&p, &[]);
    let marker = p.join(".vibe/spec-session");
    let old = std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_000_000);
    fs::File::options()
        .write(true)
        .open(&marker)
        .unwrap()
        .set_modified(old)
        .unwrap();
    run(&p, &[]);
    assert!(fs::metadata(&marker).unwrap().modified().unwrap() > old);
}
