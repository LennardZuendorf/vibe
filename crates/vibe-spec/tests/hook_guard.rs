//! `vibe-spec hook guard`: the PreToolUse spec tooth (P4-R3, P4-R4).
#![allow(dead_code, clippy::unwrap_used, clippy::expect_used)]

mod common;

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};

const SID: &str = "11111111-2222-3333-4444-555555555555";
const SKILL_SPEC: &str = r#"{"type":"assistant","message":{"role":"assistant","content":[{"type":"tool_use","name":"Skill","input":{"skill":"spec"}}]}}"#;

fn skill(name: &str) -> String {
    format!(
        r#"{{"type":"assistant","message":{{"role":"assistant","content":[{{"type":"tool_use","name":"Skill","input":{{"skill":"{name}"}}}}]}}}}"#
    )
}

fn fixture(name: &str) -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/hooks")
        .join(name);
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}

/// A project with a `docs/spec/` root and a main transcript holding `lines`.
fn setup(name: &str, lines: &[String]) -> (PathBuf, PathBuf) {
    let p = common::project(name);
    common::write(&p, "docs/spec/README.md", "# Spec\n");
    let t = p.join(format!("{SID}.jsonl"));
    let body: String = lines.iter().map(|l| format!("{l}\n")).collect();
    fs::write(&t, body).unwrap();
    (p, t)
}

/// A fixture payload with `cwd`, `transcript_path` and the target file rewritten.
fn payload(fixture_name: &str, cwd: &Path, transcript: &Path, file: &str) -> Vec<u8> {
    let mut v = fixture(fixture_name);
    v["cwd"] = json!(cwd);
    v["transcript_path"] = json!(transcript);
    v["tool_input"]["file_path"] = json!(cwd.join(file));
    v.to_string().into_bytes()
}

fn guard(cwd: &Path, stdin: &[u8]) -> common::Out {
    common::spec_stdin(cwd, &["hook", "guard"], &[], stdin)
}

fn edit(name: &str, lines: &[String], file: &str) -> common::Out {
    let (p, t) = setup(name, lines);
    guard(&p, &payload("main-write.json", &p, &t, file))
}

#[test]
fn unskilled_edit_is_blocked() {
    let out = edit("g-block", &[], "docs/spec/areas/a.md");
    assert_eq!(out.code, 2);
    assert_eq!(
        out.stderr,
        "blocked: invoke Skill(spec) before editing docs/spec/areas/a.md\n"
    );
}

#[test]
fn skilled_edit_is_allowed() {
    let out = edit("g-ok", &[SKILL_SPEC.to_string()], "docs/spec/areas/a.md");
    assert_eq!((out.code, out.stderr.as_str()), (0, ""));
}

#[test]
fn subagent_inherits_parent_load() {
    let (p, t) = setup("g-sub", &[SKILL_SPEC.to_string()]);
    let stdin = payload("subagent-write-parent-skill.json", &p, &t, "docs/spec/b.md");
    let out = guard(&p, &stdin);
    assert_eq!((out.code, out.stderr.as_str()), (0, ""));
}

#[test]
fn subagent_own_load_counts_and_absence_blocks() {
    let (p, t) = setup("g-sub2", &[]);
    let stdin = payload("subagent-write-own-skill.json", &p, &t, "docs/spec/b.md");
    assert_eq!(guard(&p, &stdin).code, 2);
    let v = fixture("subagent-write-own-skill.json");
    let id = v["agent_id"].as_str().unwrap();
    let dir = p.join(SID).join("subagents");
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join(format!("agent-{id}.jsonl")), skill("spec") + "\n").unwrap();
    assert_eq!(guard(&p, &stdin).code, 0);
}

#[test]
fn superpowers_targets() {
    let specs = "docs/superpowers/specs/2026-01-01-x-design.md";
    let out = edit("g-sp1", &[], specs);
    assert_eq!(out.code, 2);
    assert_eq!(
        out.stderr,
        format!(
            "blocked: invoke Skill(superpowers:brainstorming) or Skill(spec) before editing {specs}\n"
        )
    );
    assert_eq!(
        edit("g-sp2", &[skill("superpowers:brainstorming")], specs).code,
        0
    );
    assert_eq!(edit("g-sp3", &[skill("spec")], specs).code, 0);
    let plans = "docs/superpowers/plans/2026-01-01-x.md";
    assert_eq!(
        edit("g-sp4", &[skill("superpowers:writing-plans")], plans).code,
        0
    );
    assert_eq!(edit("g-sp5", &[], plans).code, 2);
    // A namespaced requirement matches exactly: brainstorming does not unlock plans.
    assert_eq!(
        edit("g-sp6", &[skill("superpowers:brainstorming")], plans).code,
        2
    );
}

#[test]
fn spec_json_is_a_target() {
    assert_eq!(edit("g-cfg", &[], ".vibe/spec.json").code, 2);
}

#[test]
fn other_paths_pass() {
    let out = edit("g-other", &[], "src/main.rs");
    assert_eq!((out.code, out.stderr.as_str()), (0, ""));
}

#[test]
fn fail_open_causes() {
    let allowing = "vibe-spec hook guard: allowing — ";
    let check = |out: common::Out| {
        assert_eq!(out.code, 0, "{}", out.stderr);
        assert_eq!(out.stderr.lines().count(), 1, "{}", out.stderr);
        assert!(out.stderr.starts_with(allowing), "{}", out.stderr);
    };
    let (p, t) = setup("g-fo", &[]);
    check(guard(&p, b""));
    check(guard(&p, b"{"));
    let gone = p.join("missing.jsonl");
    check(guard(
        &p,
        &payload("main-write.json", &p, &gone, "docs/spec/a.md"),
    ));
    // No spec root: a bare project.
    let bare = common::project("g-fo-bare");
    let tr = bare.join("t.jsonl");
    fs::write(&tr, "").unwrap();
    check(guard(
        &bare,
        &payload("main-write.json", &bare, &tr, "docs/spec/a.md"),
    ));
    // A target outside the project.
    let mut v = fixture("main-write.json");
    v["cwd"] = json!(p);
    v["transcript_path"] = json!(t);
    v["tool_input"]["file_path"] = json!("/elsewhere/docs/spec/a.md");
    check(guard(&p, v.to_string().as_bytes()));
}

#[test]
fn tooth_warn_override() {
    let (p, t) = setup("g-warn", &[]);
    common::write(&p, ".vibe/spec.json", r#"{"tooth":"warn"}"#);
    let out = guard(&p, &payload("main-write.json", &p, &t, "docs/spec/a.md"));
    assert_eq!(out.code, 0);
    assert_eq!(
        out.stderr,
        "vibe-spec hook guard: warn — blocked: invoke Skill(spec) before editing docs/spec/a.md\n"
    );
}

#[test]
fn tooth_config_without_root_still_discovers_and_bad_value_fails_open() {
    let (p, t) = setup("g-tooth", &[]);
    common::write(&p, ".vibe/spec.json", r#"{"tooth":"loud"}"#);
    let out = guard(&p, &payload("main-write.json", &p, &t, "docs/spec/a.md"));
    assert_eq!(out.code, 0);
    assert_eq!(out.stderr.lines().count(), 1);
    assert!(out.stderr.starts_with("vibe-spec hook guard: allowing — "));
}

#[test]
fn bash_writes_warn() {
    let (p, t) = setup("g-bash", &[]);
    let run = |command: &str| {
        let mut v = fixture("main-write.json");
        v["cwd"] = json!(p);
        v["transcript_path"] = json!(t);
        v["tool_name"] = json!("Bash");
        v["tool_input"] = json!({ "command": command });
        guard(&p, v.to_string().as_bytes())
    };
    let out = run("echo x > docs/spec/a.md");
    assert_eq!(out.code, 0);
    assert_eq!(
        out.stderr,
        "vibe-spec: warning: a shell write to docs/spec/a.md bypasses the spec tooth\n"
    );
    let out = run("cat docs/spec/a.md");
    assert_eq!((out.code, out.stderr.as_str()), (0, ""));
}

#[cfg(unix)]
#[test]
fn targets_survive_symlinked_cwd_and_dotdot() {
    let (p, t) = setup("g-link", &[]);
    let link = p
        .parent()
        .unwrap()
        .join(format!("{}-link", p.file_name().unwrap().to_string_lossy()));
    let _ = fs::remove_file(&link);
    std::os::unix::fs::symlink(&p, &link).unwrap();
    let mut v = fixture("main-write.json");
    v["transcript_path"] = json!(t);
    // Through a symlinked cwd, with the file under the real path.
    v["cwd"] = json!(link);
    v["tool_input"]["file_path"] = json!(p.join("docs/spec/a.md"));
    assert_eq!(guard(&link, v.to_string().as_bytes()).code, 2);
    // Through a symlinked cwd, with the file under the link.
    v["tool_input"]["file_path"] = json!(link.join("docs/spec/a.md"));
    assert_eq!(guard(&p, v.to_string().as_bytes()).code, 2);
    // `..` in the path.
    v["cwd"] = json!(p);
    v["tool_input"]["file_path"] = json!(p.join("docs/x/../spec/a.md"));
    assert_eq!(guard(&p, v.to_string().as_bytes()).code, 2);
    v["tool_input"]["file_path"] = json!("docs/x/../spec/a.md");
    assert_eq!(guard(&p, v.to_string().as_bytes()).code, 2);
}

fn big_transcript(name: &str) -> (PathBuf, PathBuf) {
    let (p, t) = setup(name, &[]);
    let filler = format!(
        r#"{{"type":"assistant","message":{{"role":"assistant","content":[{{"type":"text","text":"{}"}}]}}}}"#,
        "x".repeat(900)
    );
    let mut body = String::new();
    while body.len() < 20 * 1024 * 1024 {
        body.push_str(&filler);
        body.push('\n');
    }
    body.push_str(SKILL_SPEC);
    body.push('\n');
    fs::write(&t, body).unwrap();
    (p, t)
}

#[test]
fn guard_finds_a_load_on_the_last_line_of_a_large_transcript() {
    let (p, t) = big_transcript("g-big");
    let out = guard(&p, &payload("main-write.json", &p, &t, "docs/spec/a.md"));
    assert_eq!((out.code, out.stderr.as_str()), (0, ""));
}

#[test]
#[ignore = "wall-clock assert; the phase gate runs it with --release --ignored"]
fn guard_scans_a_large_transcript_in_time() {
    let (p, t) = big_transcript("g-big-time");
    let stdin = payload("main-write.json", &p, &t, "docs/spec/a.md");
    let start = std::time::Instant::now();
    let out = guard(&p, &stdin);
    let took = start.elapsed();
    assert_eq!(out.code, 0, "{}", out.stderr);
    assert!(took.as_millis() < 1000, "took {took:?}");
}
