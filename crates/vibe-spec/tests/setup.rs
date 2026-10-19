#![cfg(unix)]
// vibe-spec: literal-file
#![allow(clippy::unwrap_used, clippy::expect_used)]
//! `vibe-spec setup`: git hook, OpenCode files, AGENTS.md router block (P4-R10).
mod common;

use common::{assert_spec_json, project, spec, write};
use std::path::{Path, PathBuf};
use std::process::Command;

const BAD: &str = "---\ntype: entry\nscope: product\nchildren:\nupdated: 2026-01-15\n---\n\n# P\n";
const GOOD: &str =
    "---\ntype: entrypoint\nscope: product\nchildren:\nupdated: 2026-01-15\n---\n\n# P\n";

fn repo(name: &str) -> PathBuf {
    let p = project(name);
    std::fs::remove_dir_all(p.join(".git")).unwrap();
    common::git(&p, "2026-03-01T12:00:00Z", &["init", "-q"]);
    p
}

fn git_commit(p: &Path, path_env: &str) -> (i32, String) {
    let o = Command::new("git")
        .args([
            "-c",
            "commit.gpgsign=false",
            "-c",
            "user.email=t@t",
            "-c",
            "user.name=t",
        ])
        .args(["commit", "-q", "-m", "x"])
        .current_dir(p)
        .env_clear()
        .env("PATH", path_env)
        .env("HOME", p)
        .output()
        .unwrap();
    (
        o.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&o.stderr).into_owned(),
    )
}

fn bin_dir() -> String {
    let bin = Path::new(env!("CARGO_BIN_EXE_vibe-spec")).parent().unwrap();
    format!("{}:/usr/bin:/bin", bin.display())
}

#[test]
fn no_flag_is_usage() {
    let p = repo("setup-none");
    let out = spec(&p, &["setup"], &[]);
    assert_eq!(out.code, 2, "{}", out.stderr);
    assert!(out.stderr.contains("--git-hooks"), "{}", out.stderr);
}

#[test]
fn git_hook_installs_and_is_idempotent() {
    let p = repo("setup-hook");
    for _ in 0..2 {
        let out = spec(&p, &["setup", "--git-hooks"], &[]);
        assert_eq!(out.code, 0, "{}{}", out.text(), out.stderr);
    }
    let hook = p.join(".git/hooks/pre-commit");
    let text = std::fs::read_to_string(&hook).unwrap();
    assert!(text.starts_with("#!/usr/bin/env bash\n"));
    assert!(text.contains("# vibe-spec pre-commit v1"));
    assert!(text.contains(r#"VIBE_SPEC_STRICTNESS='{"SF29":"error"}'"#));
    use std::os::unix::fs::PermissionsExt;
    assert_eq!(
        std::fs::metadata(&hook).unwrap().permissions().mode() & 0o111,
        0o111
    );

    write(&p, ".spec/product.md", BAD);
    Command::new("git")
        .args(["add", "-A"])
        .current_dir(&p)
        .output()
        .unwrap();
    let (code, err) = git_commit(&p, &bin_dir());
    assert_ne!(code, 0, "hook must block: {err}");

    write(&p, ".spec/product.md", GOOD);
    Command::new("git")
        .args(["add", "-A"])
        .current_dir(&p)
        .output()
        .unwrap();
    let (code, err) = git_commit(&p, &bin_dir());
    assert_eq!(code, 0, "{err}");
}

#[test]
fn git_hook_skips_when_the_binary_is_absent() {
    let p = repo("setup-hook-absent");
    assert_eq!(spec(&p, &["setup", "--git-hooks"], &[]).code, 0);
    write(&p, ".spec/product.md", BAD);
    Command::new("git")
        .args(["add", "-A"])
        .current_dir(&p)
        .output()
        .unwrap();
    let (code, err) = git_commit(&p, "/usr/bin:/bin");
    assert_eq!(code, 0, "{err}");
    let lines: Vec<&str> = err.lines().filter(|l| l.contains("vibe-spec")).collect();
    assert_eq!(lines.len(), 1, "{err}");
}

#[test]
fn git_hook_refuses_a_foreign_hook() {
    let p = repo("setup-hook-foreign");
    std::fs::create_dir_all(p.join(".git/hooks")).unwrap();
    let hook = p.join(".git/hooks/pre-commit");
    std::fs::write(&hook, "#!/bin/sh\necho mine\n").unwrap();
    let out = spec(&p, &["setup", "--git-hooks"], &[]);
    assert_eq!(out.code, 1, "{}", out.stderr);
    assert_eq!(
        std::fs::read_to_string(&hook).unwrap(),
        "#!/bin/sh\necho mine\n"
    );
}

#[test]
fn git_hook_follows_a_worktree() {
    let p = repo("setup-hook-wt");
    write(&p, "a", "a");
    common::git(&p, "2026-03-01T12:00:00Z", &["add", "-A"]);
    common::git(&p, "2026-03-01T12:00:00Z", &["commit", "-q", "-m", "i"]);
    let wt = p.with_file_name(format!("{}-wt", p.file_name().unwrap().to_string_lossy()));
    common::git(
        &p,
        "2026-03-01T12:00:00Z",
        &["worktree", "add", "-q", wt.to_str().unwrap()],
    );
    let out = spec(&wt, &["setup", "--git-hooks", "--json"], &[]);
    assert_eq!(out.code, 0, "{}", out.stderr);
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let path = v["written"][0].as_str().unwrap();
    assert!(path.ends_with("hooks/pre-commit"), "{path}");
    assert!(
        Path::new(path).is_file() || wt.join(path).is_file(),
        "{path}"
    );
}

const OC: [&str; 3] = [
    ".opencode/agents/spec-promoter.md",
    ".opencode/agents/spec-health.md",
    ".opencode/plugins/vibe-spec.js",
];

#[test]
fn opencode_files() {
    let p = repo("setup-oc");
    for _ in 0..2 {
        let out = spec(&p, &["setup", "--opencode"], &[]);
        assert_eq!(out.code, 0, "{}{}", out.text(), out.stderr);
    }
    for rel in &OC[..2] {
        let t = std::fs::read_to_string(p.join(rel)).unwrap();
        assert!(t.starts_with("---\n"), "{t}");
        assert!(t.contains("<!-- vibe-spec:opencode v1 -->"));
        assert!(t.contains("mode: subagent"));
        assert!(!t.contains("name:"));
    }
    let promoter = std::fs::read_to_string(p.join(OC[0])).unwrap();
    assert!(promoter.contains("model: anthropic/claude-sonnet"));
    assert!(promoter.contains("  write: true"));
    let health = std::fs::read_to_string(p.join(OC[1])).unwrap();
    assert!(health.contains("  write: false"));
    let source =
        std::fs::read_to_string(common::repo().join("spec/agents/spec-health.md")).unwrap();
    let body = source.split("---\n").nth(2).unwrap();
    assert!(health.ends_with(body), "body must stay verbatim");
    let js = std::fs::read_to_string(p.join(OC[2])).unwrap();
    assert!(js.starts_with("// vibe-spec:opencode v1"));
    assert!(js.contains("tool.execute.after"));
    assert!(js.contains("hook"));
}

#[test]
fn opencode_refuses_a_foreign_file_and_writes_nothing() {
    let p = repo("setup-oc-foreign");
    write(&p, OC[1], "mine\n");
    let out = spec(&p, &["setup", "--opencode"], &[]);
    assert_eq!(out.code, 1, "{}", out.stderr);
    assert_eq!(std::fs::read_to_string(p.join(OC[1])).unwrap(), "mine\n");
    assert!(!p.join(OC[0]).exists());
    assert!(!p.join(OC[2]).exists());
}

#[test]
fn agents_md_block() {
    let p = repo("setup-agents");
    let before = "# Mine\n\n<!-- vibe:instructions:start -->\nkeep\n<!-- vibe:instructions:end -->\n\n<!-- vibe:rules -->\nr\n<!-- /vibe:rules -->\n";
    write(&p, "AGENTS.md", before);
    assert_eq!(spec(&p, &["setup", "--agents-md"], &[]).code, 0);
    let once = std::fs::read_to_string(p.join("AGENTS.md")).unwrap();
    assert!(
        once.starts_with(before),
        "outside text must be byte-identical"
    );
    assert_eq!(once.matches("<!-- vibe-spec:router:start -->").count(), 1);
    let again = spec(&p, &["setup", "--agents-md"], &[]);
    assert_eq!(again.code, 0);
    assert_eq!(std::fs::read_to_string(p.join("AGENTS.md")).unwrap(), once);

    let (s, e) = (
        once.find("<!-- vibe-spec:router:start -->\n").unwrap() + 31,
        once.find("<!-- vibe-spec:router:end -->").unwrap(),
    );
    assert!(once[s..e].lines().count() <= 6);
    assert!(once[s..e].contains("`spec` skill"));
    assert!(once[s..e].contains("vibe-spec"));

    // Replace in place: edit inside the markers, add text after; both sides keep.
    let edited = once
        .replace("Check the tree", "STALE")
        .replace("# Mine\n", "# Mine\nhead\n")
        + "tail\n";
    std::fs::write(p.join("AGENTS.md"), &edited).unwrap();
    assert_eq!(spec(&p, &["setup", "--agents-md"], &[]).code, 0);
    let fixed = std::fs::read_to_string(p.join("AGENTS.md")).unwrap();
    assert!(!fixed.contains("STALE"));
    assert!(fixed.starts_with("# Mine\nhead\n"));
    assert!(fixed.ends_with("tail\n"));
    assert_eq!(fixed.matches("<!-- vibe-spec:router:start -->").count(), 1);

    // A half-open block is refused, file unchanged.
    let broken = "x\n<!-- vibe-spec:router:start -->\nno end\n";
    std::fs::write(p.join("AGENTS.md"), broken).unwrap();
    assert_eq!(spec(&p, &["setup", "--agents-md"], &[]).code, 1);
    assert_eq!(
        std::fs::read_to_string(p.join("AGENTS.md")).unwrap(),
        broken
    );
}

#[test]
fn agents_md_is_created_when_absent() {
    let p = repo("setup-agents-new");
    assert_eq!(spec(&p, &["setup", "--agents-md"], &[]).code, 0);
    let t = std::fs::read_to_string(p.join("AGENTS.md")).unwrap();
    assert!(t.starts_with("<!-- vibe-spec:router:start -->\n"));
}

#[test]
fn json_shape() {
    let p = repo("setup-json");
    let out = spec(&p, &["setup", "--git-hooks", "--json"], &[]);
    assert_eq!(out.code, 0, "{}", out.stderr);
    let first = out.text();
    assert_spec_json("setup", &first);
    let v: serde_json::Value = serde_json::from_str(&first).unwrap();
    assert_eq!(v["written"].as_array().unwrap().len(), 1);
    assert_eq!(v["skipped"].as_array().unwrap().len(), 0);
    let second = spec(&p, &["setup", "--git-hooks", "--json"], &[]);
    assert_spec_json("setup", &second.text());
    let v: serde_json::Value = serde_json::from_str(&second.text()).unwrap();
    assert_eq!(v["written"].as_array().unwrap().len(), 0);
    assert_eq!(v["skipped"].as_array().unwrap().len(), 1);
}

#[test]
fn a_foreign_file_mentioning_the_marker_is_refused() {
    let p = repo("setup-mention");
    let foreign =
        "mine\nsee <!-- vibe-spec:opencode v1 --> in the docs\n<!-- vibe-spec:opencode v1 -->\n";
    write(&p, OC[1], foreign);
    let out = spec(&p, &["setup", "--opencode"], &[]);
    assert_eq!(out.code, 1, "{}", out.stderr);
    assert_eq!(std::fs::read_to_string(p.join(OC[1])).unwrap(), foreign);

    let q = repo("setup-mention-hook");
    std::fs::create_dir_all(q.join(".git/hooks")).unwrap();
    let hook = "#!/bin/sh\necho '# vibe-spec pre-commit v1'\n# vibe-spec pre-commit v1\n";
    std::fs::write(q.join(".git/hooks/pre-commit"), hook).unwrap();
    assert_eq!(spec(&q, &["setup", "--git-hooks"], &[]).code, 1);
    assert_eq!(
        std::fs::read_to_string(q.join(".git/hooks/pre-commit")).unwrap(),
        hook
    );
}

#[test]
fn two_router_blocks_are_refused() {
    let p = repo("setup-two-blocks");
    let two = "<!-- vibe-spec:router:start -->\na\n<!-- vibe-spec:router:end -->\nmid\n<!-- vibe-spec:router:start -->\nb\n<!-- vibe-spec:router:end -->\n";
    write(&p, "AGENTS.md", two);
    let out = spec(&p, &["setup", "--agents-md"], &[]);
    assert_eq!(out.code, 1, "{}", out.stderr);
    assert_eq!(std::fs::read_to_string(p.join("AGENTS.md")).unwrap(), two);
}

#[test]
fn git_hook_from_a_subdirectory() {
    let p = repo("setup-subdir");
    let sub = p.join("sub/deep");
    std::fs::create_dir_all(&sub).unwrap();
    let out = spec(&sub, &["setup", "--git-hooks", "--json"], &[]);
    assert_eq!(out.code, 0, "{}", out.stderr);
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["written"][0], ".git/hooks/pre-commit");

    let q = repo("setup-subdir-link");
    let real = q.join("elsewhere");
    std::fs::create_dir_all(&real).unwrap();
    std::fs::remove_dir_all(q.join(".git/hooks")).unwrap();
    std::os::unix::fs::symlink(&real, q.join(".git/hooks")).unwrap();
    let sub = q.join("sub");
    std::fs::create_dir_all(&sub).unwrap();
    let out = spec(&sub, &["setup", "--git-hooks"], &[]);
    assert_eq!(out.code, 1, "{}", out.stderr);
    assert!(out.stderr.contains("symlink"), "{}", out.stderr);
    assert!(!real.join("pre-commit").exists());
}

#[test]
fn a_failed_write_puts_earlier_writes_back() {
    use std::os::unix::fs::PermissionsExt;
    let p = repo("setup-rollback");
    let stale = "---\ndescription: x\n---\n<!-- vibe-spec:opencode v1 -->\nold\n";
    write(&p, OC[1], stale);
    let plugins = p.join(".opencode/plugins");
    std::fs::create_dir_all(&plugins).unwrap();
    std::fs::set_permissions(&plugins, std::fs::Permissions::from_mode(0o555)).unwrap();
    let out = spec(&p, &["setup", "--opencode"], &[]);
    std::fs::set_permissions(&plugins, std::fs::Permissions::from_mode(0o755)).unwrap();
    assert_eq!(out.code, 1, "{}", out.stderr);
    assert_eq!(std::fs::read_to_string(p.join(OC[1])).unwrap(), stale);
    assert!(!p.join(OC[0]).exists());
    assert!(!plugins.join("vibe-spec.js").exists());
}
