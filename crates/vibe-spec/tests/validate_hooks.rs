#![cfg(unix)]
#![allow(clippy::unwrap_used, clippy::expect_used)]
//! `validate --file/--changed/--format hook` and `hook validate` (P4-R5).
mod common;

use common::{git, project, spec, spec_stdin, write};
use std::fs;
use std::path::Path;
use std::time::{Duration, SystemTime};

const OK_HUB: &str = "---\ntype: hub\nupdated: 2026-01-15\n---\n\n# H\n";

/// A layered tree whose stray a.md and b.md are SF30 errors.
fn tree(name: &str) -> std::path::PathBuf {
    let p = project(name);
    write(
        &p,
        ".vibe/spec.json",
        "{\"strictness\":{\"SF-30\":\"error\"}}",
    );
    write(&p, "docs/spec/README.md", OK_HUB);
    write(&p, "docs/spec/a.md", "# no frontmatter a\n");
    write(&p, "docs/spec/b.md", "# no frontmatter b\n");
    p
}

fn files_of(v: &serde_json::Value) -> Vec<String> {
    v["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|f| f["file"].as_str().map(str::to_string))
        .collect()
}

fn json(p: &Path, extra: &[&str]) -> (i32, serde_json::Value) {
    common::validate_json(p, extra, &[])
}

fn payload(fixture: &str, p: &Path, file: Option<&str>) -> Vec<u8> {
    let path = format!(
        "{}/tests/fixtures/hooks/{fixture}",
        env!("CARGO_MANIFEST_DIR")
    );
    let text = fs::read_to_string(path).unwrap();
    let mut v: serde_json::Value = serde_json::from_str(&text).unwrap();
    v["cwd"] = p.to_str().unwrap().into();
    v["transcript_path"] = p.join("t.jsonl").to_str().unwrap().into();
    if let Some(f) = file {
        v["tool_input"]["file_path"] = p.join(f).to_str().unwrap().into();
    }
    serde_json::to_vec(&v).unwrap()
}

#[test]
fn file_filters_to_one_file() {
    let p = tree("vh-file");
    let (_, all) = json(&p, &[]);
    assert!(files_of(&all).iter().any(|f| f.ends_with("b.md")));
    let (code, v) = json(&p, &["--file", "docs/spec/a.md"]);
    let files = files_of(&v);
    assert!(!files.is_empty());
    assert!(files.iter().all(|f| f == "docs/spec/a.md"), "{files:?}");
    assert_eq!(code, 1);
    assert!(v["errors"].as_u64().unwrap() >= 1);
    common::assert_spec_json("validate", &v.to_string());
    let (code, v) = json(&p, &["--file", "docs/spec/README.md"]);
    assert_eq!((code, files_of(&v).len()), (0, 0));
}

#[test]
fn changed_uses_git_status() {
    let p = tree("vh-changed");
    fs::remove_dir_all(p.join(".git")).unwrap();
    git(&p, "2026-01-01T00:00:00Z", &["init", "-q"]);
    git(&p, "2026-01-01T00:00:00Z", &["add", "-A"]);
    git(&p, "2026-01-01T00:00:00Z", &["commit", "-q", "-m", "init"]);
    let out = spec(&p, &["validate", "--changed"], &[]);
    assert_eq!(
        (out.code, out.text().as_str()),
        (0, "validate: no spec changes\n")
    );
    write(&p, "docs/spec/b.md", "# still no frontmatter, edited\n");
    let (_, v) = json(&p, &["--changed"]);
    let files = files_of(&v);
    assert!(!files.is_empty());
    assert!(files.iter().all(|f| f == "docs/spec/b.md"), "{files:?}");
    common::assert_spec_json("validate", &v.to_string());
}

#[test]
fn changed_without_git_uses_the_session_marker() {
    let p = tree("vh-marker");
    let t = |s: u64| SystemTime::UNIX_EPOCH + Duration::from_secs(1_700_000_000 + s);
    let set = |rel: &str, s: u64| {
        fs::File::options()
            .write(true)
            .open(p.join(rel))
            .unwrap()
            .set_modified(t(s))
            .unwrap();
    };
    write(&p, ".vibe/spec-session", "");
    set("docs/spec/a.md", 0);
    set(".vibe/spec-session", 100);
    set("docs/spec/b.md", 200);
    set("docs/spec/README.md", 0);
    let (_, v) = json(&p, &["--changed"]);
    let files = files_of(&v);
    assert!(!files.is_empty());
    assert!(files.iter().all(|f| f == "docs/spec/b.md"), "{files:?}");
}

#[test]
fn hook_validate_post_tool_use() {
    let p = tree("vh-post");
    let out = spec_stdin(
        &p,
        &["hook", "validate"],
        &[],
        &payload("post-write.json", &p, Some("docs/spec/a.md")),
    );
    assert_eq!(out.code, 0, "{}", out.stderr);
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["hookSpecificOutput"]["hookEventName"], "PostToolUse");
    let ctx = v["hookSpecificOutput"]["additionalContext"]
        .as_str()
        .unwrap();
    assert!(ctx.contains("a.md"), "{ctx}");
    assert!(!ctx.contains("b.md"), "{ctx}");
}

#[test]
fn hook_validate_stop() {
    let p = tree("vh-stop");
    write(&p, ".vibe/spec-session", "");
    let old = SystemTime::UNIX_EPOCH + Duration::from_secs(1_700_000_000);
    for f in [
        "docs/spec/a.md",
        "docs/spec/README.md",
        ".vibe/spec-session",
    ] {
        fs::File::options()
            .write(true)
            .open(p.join(f))
            .unwrap()
            .set_modified(old)
            .unwrap();
    }
    let out = spec_stdin(
        &p,
        &["hook", "validate"],
        &[],
        &payload("stop.json", &p, None),
    );
    assert_eq!(out.code, 0, "{}", out.stderr);
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let msg = v["systemMessage"].as_str().unwrap();
    assert!(msg.contains("b.md") && !msg.contains("a.md"), "{msg}");
    assert!(v.get("hookSpecificOutput").is_none());
}

#[test]
fn hook_validate_is_capped() {
    let p = tree("vh-cap");
    for i in 0..2000 {
        write(&p, &format!("docs/spec/notes/n{i}.md"), "# n\n");
    }
    let out = spec_stdin(
        &p,
        &["hook", "validate"],
        &[("VIBE_SPEC_HOOK_CAP_MS", "1")],
        &payload("post-write.json", &p, Some("docs/spec/a.md")),
    );
    assert_eq!(out.code, 0);
    assert!(out.stdout.is_empty());
    assert!(
        out.stderr
            .starts_with("vibe-spec hook validate: skipped — over 1 ms"),
        "{}",
        out.stderr
    );
}

#[test]
fn hook_validate_never_fails() {
    let p = tree("vh-never");
    let bare = project("vh-never-bare");
    for (cwd, input) in [
        (&p, b"not json".to_vec()),
        (
            &bare,
            payload("post-write.json", &bare, Some("docs/spec/a.md")),
        ),
        (
            &p,
            String::from_utf8(payload("stop.json", &p, None))
                .unwrap()
                .replace("\"Stop\"", "\"Bogus\"")
                .into_bytes(),
        ),
    ] {
        let out = spec_stdin(cwd, &["hook", "validate"], &[], &input);
        assert_eq!(out.code, 0, "{}", out.stderr);
        assert!(out.stdout.is_empty(), "{}", out.text());
    }
}

fn committed(name: &str) -> std::path::PathBuf {
    let p = tree(name);
    write(&p, "sub/y.txt", "y\n");
    fs::remove_dir_all(p.join(".git")).unwrap();
    git(&p, "2026-01-01T00:00:00Z", &["init", "-q"]);
    git(&p, "2026-01-01T00:00:00Z", &["add", "-A"]);
    git(&p, "2026-01-01T00:00:00Z", &["commit", "-q", "-m", "init"]);
    p
}

#[test]
fn changed_ignores_files_outside_the_spec_root() {
    let p = committed("vh-outside");
    write(&p, "sub/y.txt", "changed\n");
    let out = spec(&p, &["validate", "--changed"], &[]);
    assert_eq!(
        (out.code, out.text().as_str()),
        (0, "validate: no spec changes\n")
    );
}

#[test]
fn file_resolves_against_cwd_and_dotdot() {
    let p = committed("vh-paths");
    let only_a = |cwd: &Path, arg: &str| {
        let (_, v) = common::validate_json(cwd, &["--file", arg], &[]);
        let files = files_of(&v);
        assert!(!files.is_empty(), "{arg} from {}", cwd.display());
        assert!(files.iter().all(|f| f == "docs/spec/a.md"), "{files:?}");
    };
    only_a(&p, "docs/spec/../spec/a.md");
    only_a(&p.join("sub"), "../docs/spec/a.md");
    only_a(&p.join("docs/spec"), "a.md");
    let out = spec(&p, &["validate", "--file", "/etc/passwd"], &[]);
    assert_eq!(
        (out.code, out.text().as_str()),
        (0, "validate: /etc/passwd is not under the spec root\n")
    );
    let out = spec(&p, &["validate", "--file", "sub/y.txt"], &[]);
    assert_eq!(
        out.text(),
        "validate: sub/y.txt is not under the spec root\n"
    );
}
