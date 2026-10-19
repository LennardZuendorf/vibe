//! Hook payload parsing and transcript skill detection (P4-R2).
#![allow(dead_code, clippy::unwrap_used, clippy::expect_used)]

#[path = "../src/hook.rs"]
mod hook;
/// `hook.rs` names `crate::root` for its guard, which this file never runs.
mod root {
    use std::path::{Path, PathBuf};

    pub struct Root {
        pub dir: PathBuf,
        pub project: PathBuf,
    }
    pub enum Miss {
        NotFound,
        Other,
    }
    impl Miss {
        pub fn message(&self) -> String {
            unimplemented!()
        }
    }
    pub enum Tooth {
        Block,
        Warn,
    }
    pub fn discover(_: &Path, _: Option<&Path>, _: Option<&std::ffi::OsStr>) -> Result<Root, Miss> {
        unimplemented!()
    }
    pub fn tooth(_: &Path) -> Result<Tooth, String> {
        unimplemented!()
    }
}
/// `hook::validate` names `crate::validate`, which this file never runs.
mod validate {
    pub enum Format {
        Hook,
    }
    pub struct Options {
        pub strict: bool,
        pub format: Format,
        pub changed: bool,
        pub file: Option<String>,
        pub event: Option<String>,
    }
    pub fn run_checked(
        _: &Result<crate::root::Root, crate::root::Miss>,
        _: &std::path::Path,
        _: &Options,
    ) -> Result<(u8, Vec<u8>), String> {
        unimplemented!()
    }
}
#[path = "../src/text.rs"]
mod text;
#[path = "../src/transcript.rs"]
mod transcript;

mod common;

use std::fs;
use std::path::{Path, PathBuf};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/hooks")
        .join(name)
}

/// `<tmp>/<sid>.jsonl` holding `main` (fixture name or literal text), plus an
/// optional `<tmp>/<sid>/subagents/agent-<id>.jsonl`. Returns the main path.
fn layout(name: &str, main: &str, sub: Option<(&str, &str)>) -> PathBuf {
    let dir = common::project(name);
    let sid = "11111111-2222-3333-4444-555555555555";
    let path = dir.join(format!("{sid}.jsonl"));
    fs::write(&path, main).unwrap();
    if let Some((id, body)) = sub {
        let sdir = dir.join(sid).join("subagents");
        fs::create_dir_all(&sdir).unwrap();
        fs::write(sdir.join(format!("agent-{id}.jsonl")), body).unwrap();
    }
    path
}

fn read(name: &str) -> String {
    fs::read_to_string(fixture(name)).unwrap()
}

#[test]
fn parses_every_captured_payload() {
    let mut n = 0;
    for entry in fs::read_dir(fixture("")).unwrap() {
        let path = entry.unwrap().path();
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        if !name.ends_with(".json") || name == "subagent-meta.json" {
            continue;
        }
        let p = hook::parse(&fs::read(&path).unwrap()).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(p.cwd, Path::new("/work/project"), "{name}");
        assert!(!p.event.is_empty(), "{name}");
        assert!(p.transcript.is_some(), "{name}");
        n += 1;
    }
    assert_eq!(n, 8);

    let p = hook::parse(&fs::read(fixture("agent-spec-health.json")).unwrap()).unwrap();
    assert_eq!(p.event, "PreToolUse");
    assert!(p.agent_id.is_some());
    assert_eq!(p.tool.as_deref(), Some("Bash"));

    let p = hook::parse(&fs::read(fixture("subagent-write-parent-skill.json")).unwrap()).unwrap();
    assert_eq!(p.event, "PreToolUse");
    assert_eq!(p.agent_id.as_deref(), Some("a315ef01f296b121d"));
    assert_eq!(p.tool.as_deref(), Some("Write"));
    assert!(p.file.as_deref().unwrap().ends_with("docs/spec/b.md"));
    assert_eq!(p.command, None);

    let p = hook::parse(&fs::read(fixture("main-write.json")).unwrap()).unwrap();
    assert_eq!(p.agent_id, None);
}

#[test]
fn payload_fields_and_errors() {
    let p = hook::parse(br#"{"hook_event_name":"PreToolUse","cwd":"/p","tool_name":"NotebookEdit","tool_input":{"notebook_path":"/p/n.ipynb"}}"#).unwrap();
    assert_eq!(p.file.as_deref(), Some("/p/n.ipynb"));
    assert_eq!(p.transcript, None);
    let p = hook::parse(br#"{"hook_event_name":"PreToolUse","cwd":"/p","tool_name":"Edit","tool_input":{"file_path":"/p/a","notebook_path":"/p/b"}}"#).unwrap();
    assert_eq!(p.file.as_deref(), Some("/p/a"));
    let p = hook::parse(br#"{"hook_event_name":"PreToolUse","cwd":"/p","tool_name":"Bash","tool_input":{"command":"echo hi > x"}}"#).unwrap();
    assert_eq!(p.command.as_deref(), Some("echo hi > x"));
    assert_eq!(p.file, None);

    assert!(hook::parse(b"").is_err());
    assert!(hook::parse(b"not json").is_err());
    assert!(hook::parse(b"[]").is_err());
    assert!(hook::parse(br#"{"cwd":"/p"}"#).is_err());
    assert!(hook::parse(br#"{"hook_event_name":"Stop"}"#).is_err());
}

#[test]
fn skill_loads_are_found() {
    let main = layout("t1-main", &read("transcript-main-skill.jsonl"), None);
    assert!(transcript::loaded(&main, None, &["spec"]).unwrap());
    assert!(!transcript::loaded(&main, None, &["brainstorming"]).unwrap());
    assert!(!transcript::loaded(&main, None, &["pec"]).unwrap());
    assert!(transcript::loaded(&main, None, &["brainstorming", "spec"]).unwrap());

    let slash = layout("t1-slash", &read("transcript-slash-command.jsonl"), None);
    assert!(transcript::loaded(&slash, None, &["spec"]).unwrap());

    let nosk = layout("t1-nosk", &read("transcript-subagent-no-skill.jsonl"), None);
    assert!(!transcript::loaded(&nosk, None, &["spec"]).unwrap());

    let parent = layout(
        "t1-parent",
        &read("transcript-parent-of-subagents.jsonl"),
        None,
    );
    assert!(transcript::loaded(&parent, Some("a6d9f9d932ee550ba"), &["spec"]).unwrap());

    let own = read("transcript-subagent-own-skill.jsonl");
    let p = layout(
        "t1-own",
        &read("transcript-subagent-no-skill.jsonl"),
        Some(("a6d9f9d932ee550ba", &own)),
    );
    assert!(transcript::loaded(&p, Some("a6d9f9d932ee550ba"), &["spec"]).unwrap());
    assert!(!transcript::loaded(&p, None, &["spec"]).unwrap());
    // A different agent id has no file: not loaded, not an error.
    assert!(!transcript::loaded(&p, Some("deadbeef"), &["spec"]).unwrap());
}

#[test]
fn namespaced_skills_count() {
    let tool = r#"{"type":"assistant","message":{"content":[{"type":"tool_use","name":"Skill","input":{"skill":"plugin:spec"}}]}}"#;
    let p = layout("t1-ns", tool, None);
    assert!(transcript::loaded(&p, None, &["spec"]).unwrap());
    assert!(!transcript::loaded(&p, None, &["other"]).unwrap());
    let cmd = r#"{"type":"user","message":{"content":"<command-name>/superpowers:brainstorming</command-name>"}}"#;
    let p = layout("t1-ns2", cmd, None);
    assert!(transcript::loaded(&p, None, &["brainstorming"]).unwrap());
    assert!(!transcript::loaded(&p, None, &["spec"]).unwrap());
    // `/xspec` is a different skill, not a suffix match without a namespace colon.
    let cmd = r#"{"type":"user","message":{"content":"<command-name>/xspec</command-name>"}}"#;
    let p = layout("t1-ns3", cmd, None);
    assert!(!transcript::loaded(&p, None, &["spec"]).unwrap());
    let tool = r#"{"type":"assistant","message":{"content":[{"type":"tool_use","name":"Skill","input":{"skill":"xspec"}}]}}"#;
    let p = layout("t1-ns4", tool, None);
    assert!(!transcript::loaded(&p, None, &["spec"]).unwrap());
}

#[test]
fn prose_and_tool_results_do_not_count() {
    let prose = r#"{"type":"user","message":{"role":"user","content":"please use Skill(spec) and \"name\":\"Skill\",\"input\":{\"skill\":\"spec\"}"}}"#;
    let p = layout("t1-prose", prose, None);
    assert!(!transcript::loaded(&p, None, &["spec"]).unwrap());

    let result = r#"{"type":"user","message":{"role":"user","content":[{"type":"tool_result","tool_use_id":"t","content":"{\"name\":\"Skill\",\"input\":{\"skill\":\"spec\"}}"}]}}"#;
    let p = layout("t1-result", result, None);
    assert!(!transcript::loaded(&p, None, &["spec"]).unwrap());

    // Embedded as raw JSON inside a tool_result, not as a content item.
    let nested = r#"{"type":"user","message":{"role":"user","content":[{"type":"tool_result","tool_use_id":"t","content":[{"type":"tool_use","name":"Skill","input":{"skill":"spec"}}]}]}}"#;
    let p = layout("t1-nested", nested, None);
    assert!(!transcript::loaded(&p, None, &["spec"]).unwrap());

    // A user-role tool_use and an assistant-role command tag do not count either.
    let wrong_role = concat!(
        r#"{"type":"user","message":{"content":[{"type":"tool_use","name":"Skill","input":{"skill":"spec"}}]}}"#,
        "\n",
        r#"{"type":"assistant","message":{"content":"<command-name>/spec</command-name>"}}"#,
        "\n",
        r#"{"type":"user","message":{"content":[{"type":"text","text":"<command-name>/spec</command-name>"}]}}"#,
        "\n"
    );
    let p = layout("t1-role", wrong_role, None);
    assert!(!transcript::loaded(&p, None, &["spec"]).unwrap());

    // A different tool named in the assistant record does not count.
    let other = r#"{"type":"assistant","message":{"content":[{"type":"tool_use","name":"Read","input":{"skill":"spec"}}]}}"#;
    let p = layout("t1-other", other, None);
    assert!(!transcript::loaded(&p, None, &["spec"]).unwrap());
}

#[test]
fn malformed_lines_are_skipped() {
    let good = r#"{"type":"assistant","message":{"content":[{"type":"tool_use","name":"Skill","input":{"skill":"spec"}}]}}"#;
    let text = format!(
        "{{\"type\":\"user\",\"message\":{{\"content\":\"hi\"}}}}\nSkill <command-name> not json\n{good}\n"
    );
    let p = layout("t1-bad", &text, None);
    assert!(transcript::loaded(&p, None, &["spec"]).unwrap());
    // Without a trailing newline, and with CRLF.
    let p = layout("t1-nonl", good, None);
    assert!(transcript::loaded(&p, None, &["spec"]).unwrap());
    let p = layout("t1-crlf", &format!("{good}\r\n"), None);
    assert!(transcript::loaded(&p, None, &["spec"]).unwrap());
}

#[test]
fn missing_main_transcript_is_an_error() {
    let dir = common::project("t1-missing");
    assert!(transcript::loaded(&dir.join("nope.jsonl"), None, &["spec"]).is_err());
}

#[test]
fn spec_stdin_pipes_input() {
    // `vibe-spec` with no subcommand reads nothing; the helper must still not hang or panic.
    let dir = common::project("t1-stdin");
    let out = common::spec_stdin(&dir, &["--help"], &[], b"{}");
    assert_eq!(out.code, 0);
}
