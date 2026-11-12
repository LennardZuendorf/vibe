#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Document confinement and revision controls (spec-projects-mcp/4).
mod common;
use serde_json::Value;
use std::path::Path;

fn call(code: &Path, store: &Path, args: &[&str]) -> common::Out {
    // <stdin>, TEXT supplies explicit fixture bytes, not a CLI argument.
    let mut args = args.to_vec();
    let input = if let Some(at) = args.iter().position(|arg| *arg == "<stdin>") {
        let input = args[at + 1].as_bytes().to_vec();
        args.drain(at..at + 2);
        input
    } else {
        Vec::new()
    };
    let argv: Vec<std::ffi::OsString> = std::iter::once("vibe-spec")
        .chain(args)
        .map(std::ffi::OsString::from)
        .collect();
    let runtime = vibe_spec::Runtime {
        cwd: code.to_path_buf(),
        env: std::collections::BTreeMap::from([
            ("PATH".into(), std::env::var_os("PATH").unwrap_or_default()),
            ("HOME".into(), code.as_os_str().to_owned()),
            ("VIBE_STORE".into(), store.as_os_str().to_owned()),
        ]),
    };
    let out = vibe_spec::run_cli(&argv, &runtime, &mut std::io::Cursor::new(input));
    common::Out {
        code: i32::from(out.exit),
        stdout: out.stdout,
        stderr: String::from_utf8(out.stderr).unwrap(),
    }
}
fn setup(
    global: bool,
) -> (
    vibe_core::testscratch::Scratch,
    std::path::PathBuf,
    std::path::PathBuf,
) {
    let owner = common::project("documents");
    let code = owner.join("code");
    std::fs::create_dir(&code).unwrap();
    common::git(&code, "2026-01-01T00:00:00Z", &["init", "-b", "main"]);
    common::git(
        &code,
        "2026-01-01T00:00:00Z",
        &["commit", "--allow-empty", "-m", "initial"],
    );
    let store = owner.join("store");
    let out = call(
        &code,
        &store,
        &[
            "project",
            "init",
            "--mode",
            if global { "global-first" } else { "repo-first" },
        ],
    );
    assert_eq!(out.code, 0, "{}", out.stderr);
    (owner, code, store)
}
fn context(code: &Path, store: &Path) -> Value {
    let out = call(code, store, &["context"]);
    assert_eq!(out.code, 0, "{}", out.stderr);
    serde_json::from_slice(&out.stdout).unwrap()
}
#[test]
fn stale_writer_cannot_overwrite() {
    let (_owner, code, store) = setup(true);
    let before = context(&code, &store);
    let token = before["revision_token"].as_str().unwrap();
    let out = call(
        &code,
        &store,
        &[
            "document",
            "write",
            "spec/new.md",
            "<stdin>",
            "first\n",
            "--expect",
            token,
        ],
    );
    assert_eq!(out.code, 0, "{}", out.stderr);
    let stale = call(
        &code,
        &store,
        &[
            "document",
            "write",
            "spec/new.md",
            "<stdin>",
            "stale\n",
            "--expect",
            token,
        ],
    );
    assert_eq!(stale.code, 1);
    assert!(
        stale.stderr.contains("revision_changed"),
        "{}",
        stale.stderr
    );
    let read = call(&code, &store, &["document", "read", "spec/new.md"]);
    assert_eq!(read.code, 0, "{}", read.stderr);
    let read: Value = serde_json::from_slice(&read.stdout).unwrap();
    assert_eq!(read["content"], "first\n");
    let after = context(&code, &store);
    assert_ne!(before["spec_revision"], after["spec_revision"]);
    assert!(!code.join("docs").exists());
}
#[test]
fn dirty_tree_needs_snapshot() {
    let (_owner, code, store) = setup(true);
    let before = context(&code, &store);
    let root = before["spec_root"].as_str().unwrap();
    std::fs::write(Path::new(root).join("raw.md"), b"raw editor bytes\r\n").unwrap();
    let dirty = context(&code, &store);
    let token = dirty["revision_token"].as_str().unwrap();
    let out = call(
        &code,
        &store,
        &[
            "document",
            "write",
            "spec/new.md",
            "<stdin>",
            "new",
            "--expect",
            token,
        ],
    );
    assert_eq!(out.code, 1);
    assert!(out.stderr.contains("dirty_tree"), "{}", out.stderr);
    let out = call(&code, &store, &["snapshot", "--expect", token]);
    assert_eq!(out.code, 0, "{}", out.stderr);
    assert_eq!(
        std::fs::read(Path::new(root).join("raw.md")).unwrap(),
        b"raw editor bytes\r\n"
    );
    assert_ne!(
        before["spec_revision"],
        context(&code, &store)["spec_revision"]
    );
}
#[test]
fn path_escape_writes_nothing() {
    let (_owner, code, store) = setup(true);
    let before = context(&code, &store);
    for path in [
        "spec/../escape.md",
        "/tmp/escape.md",
        "spec/a.txt",
        "spec/.git/config.md",
        "superpowers/../../escape.md",
        "spec/a\\b.md",
    ] {
        let out = call(
            &code,
            &store,
            &[
                "document",
                "write",
                path,
                "<stdin>",
                "bad",
                "--expect",
                before["revision_token"].as_str().unwrap(),
            ],
        );
        assert_eq!(out.code, 1, "{path}: {}", out.stderr);
        assert!(
            out.stderr.contains("invalid_document"),
            "{path}: {}",
            out.stderr
        );
    }
    assert_eq!(before, context(&code, &store));
}
#[test]
fn repo_first_tokens_detect_raw_edits_and_never_touch_index() {
    let (_owner, code, store) = setup(false);
    common::git(&code, "2026-01-01T00:00:00Z", &["add", "docs/spec"]);
    let index = std::fs::read(code.join(".git/index")).unwrap();
    let head = common::git_out(&code, &["rev-parse", "HEAD"]);
    let before = context(&code, &store);
    std::fs::write(code.join("docs/spec/raw.md"), b"raw").unwrap();
    let stale = call(
        &code,
        &store,
        &[
            "document",
            "write",
            "spec/raw.md",
            "<stdin>",
            "overwrite",
            "--expect",
            before["revision_token"].as_str().unwrap(),
        ],
    );
    assert_eq!(stale.code, 1);
    assert!(
        stale.stderr.contains("revision_changed"),
        "{}",
        stale.stderr
    );
    let fresh = context(&code, &store);
    let out = call(
        &code,
        &store,
        &[
            "document",
            "remove",
            "spec/raw.md",
            "--expect",
            fresh["revision_token"].as_str().unwrap(),
        ],
    );
    assert_eq!(out.code, 0, "{}", out.stderr);
    assert!(!code.join("docs/spec/raw.md").exists());
    assert_eq!(std::fs::read(code.join(".git/index")).unwrap(), index);
    assert_eq!(common::git_out(&code, &["rev-parse", "HEAD"]), head);
}
#[test]
fn same_branch_clones_share_content() {
    let (owner, code, store) = setup(true);
    common::git(
        &code,
        "2026-01-01T00:00:00Z",
        &[
            "remote",
            "add",
            "origin",
            "https://example.invalid/team/shared.git",
        ],
    );
    // Re-register after adding the alias so a second physical clone can discover it.
    assert_eq!(
        call(
            &code,
            &store,
            &["project", "init", "--mode", "global-first"]
        )
        .code,
        0
    );
    let clone = owner.join("clone");
    common::git(
        &owner,
        "2026-01-01T00:00:00Z",
        &["clone", code.to_str().unwrap(), clone.to_str().unwrap()],
    );
    common::git(
        &clone,
        "2026-01-01T00:00:00Z",
        &[
            "remote",
            "set-url",
            "origin",
            "git@example.invalid:team/shared.git",
        ],
    );
    let before = context(&code, &store);
    let out = call(
        &code,
        &store,
        &[
            "document",
            "write",
            "spec/shared.md",
            "<stdin>",
            "shared bytes\n",
            "--expect",
            before["revision_token"].as_str().unwrap(),
        ],
    );
    assert_eq!(out.code, 0, "{}", out.stderr);
    let other = context(&clone, &store);
    assert_eq!(other["spec_root"], before["spec_root"]);
    let out = call(&clone, &store, &["document", "read", "spec/shared.md"]);
    assert_eq!(out.code, 0, "{}", out.stderr);
    assert_eq!(
        serde_json::from_slice::<Value>(&out.stdout).unwrap()["content"],
        "shared bytes\n"
    );
}
#[test]
fn different_branches_are_isolated() {
    let (_owner, code, store) = setup(true);
    let before = context(&code, &store);
    let out = call(
        &code,
        &store,
        &["branch", "init", "topic", "--from", "main"],
    );
    assert_eq!(out.code, 0, "{}", out.stderr);
    common::git(&code, "2026-01-01T00:00:00Z", &["checkout", "-b", "topic"]);
    let topic = context(&code, &store);
    let out = call(
        &code,
        &store,
        &[
            "document",
            "write",
            "spec/isolated.md",
            "<stdin>",
            "topic bytes",
            "--expect",
            topic["revision_token"].as_str().unwrap(),
        ],
    );
    assert_eq!(out.code, 0, "{}", out.stderr);
    assert!(
        !Path::new(before["spec_root"].as_str().unwrap())
            .join("isolated.md")
            .exists()
    );
    common::git(&code, "2026-01-01T00:00:00Z", &["checkout", "main"]);
    assert_eq!(
        context(&code, &store)["spec_revision"],
        before["spec_revision"]
    );
}
#[cfg(unix)]
#[test]
fn intermediate_and_leaf_symlinks_cannot_be_read_or_written() {
    use std::os::unix::fs::symlink;
    let (owner, code, store) = setup(true);
    let before = context(&code, &store);
    let root = Path::new(before["spec_root"].as_str().unwrap());
    let outside = owner.join("outside");
    std::fs::create_dir(&outside).unwrap();
    std::fs::write(outside.join("secret.md"), b"external exact bytes").unwrap();
    symlink(&outside, root.join("linked")).unwrap();
    for args in [
        vec!["document", "read", "spec/linked/secret.md"],
        vec![
            "document",
            "write",
            "spec/linked/new.md",
            "<stdin>",
            "bad",
            "--expect",
            before["revision_token"].as_str().unwrap(),
        ],
    ] {
        assert_eq!(call(&code, &store, &args).code, 1);
    }
    assert!(!outside.join("new.md").exists());
    assert_eq!(
        std::fs::read(outside.join("secret.md")).unwrap(),
        b"external exact bytes"
    );
    std::fs::remove_file(root.join("linked")).unwrap();
    symlink(outside.join("secret.md"), root.join("leaf.md")).unwrap();
    assert_eq!(
        call(&code, &store, &["document", "read", "spec/leaf.md"]).code,
        1
    );
}
#[test]
fn foreign_staged_files_and_code_git_are_untouched() {
    let (_owner, code, store) = setup(true);
    let before = context(&code, &store);
    let anchor = Path::new(before["spec_root"].as_str().unwrap())
        .parent()
        .unwrap();
    std::fs::write(anchor.join("foreign.txt"), b"must not enter history").unwrap();
    common::git(anchor, "2026-01-01T00:00:00Z", &["add", "foreign.txt"]);
    let gitdir = common::git_out(anchor, &["rev-parse", "--absolute-git-dir"]);
    let private_index = std::fs::read(Path::new(&gitdir).join("index")).unwrap();
    let code_head = common::git_out(&code, &["rev-parse", "HEAD"]);
    let out = call(
        &code,
        &store,
        &[
            "document",
            "write",
            "spec/owned.md",
            "<stdin>",
            "owned",
            "--expect",
            before["revision_token"].as_str().unwrap(),
        ],
    );
    assert_eq!(out.code, 0, "{}", out.stderr);
    assert_eq!(
        std::fs::read(Path::new(&gitdir).join("index")).unwrap(),
        private_index
    );
    assert_eq!(
        common::git_out(anchor, &["ls-tree", "HEAD", "foreign.txt"]),
        ""
    );
    assert_eq!(
        common::git_out(anchor, &["show", "HEAD:spec/owned.md"]),
        "owned"
    );
    assert_eq!(common::git_out(&code, &["rev-parse", "HEAD"]), code_head);
}
#[cfg(unix)]
#[test]
fn literal_git_pathspec_characters_keep_exact_document_bytes() {
    let (_owner, code, store) = setup(true);
    let before = context(&code, &store);
    let out = call(
        &code,
        &store,
        &[
            "document",
            "write",
            "spec/[brackets]*.md",
            "<stdin>",
            "literal\r\n",
            "--expect",
            before["revision_token"].as_str().unwrap(),
        ],
    );
    assert_eq!(out.code, 0, "{}", out.stderr);
    let anchor = Path::new(before["spec_root"].as_str().unwrap())
        .parent()
        .unwrap();
    assert_eq!(
        common::git_out(anchor, &["show", "HEAD:spec/[brackets]*.md"]),
        "literal"
    );
    assert_eq!(
        std::fs::read(anchor.join("spec/[brackets]*.md")).unwrap(),
        b"literal\r\n"
    );
}
#[test]
fn artifact_removal_without_completion_retains_first_baseline() {
    let (_owner, code, store) = setup(true);
    let initial = context(&code, &store);
    let artifact = "superpowers/plans/2026-11-11-feature-one.md";
    let out = call(
        &code,
        &store,
        &[
            "document",
            "write",
            artifact,
            "<stdin>",
            "draft",
            "--expect",
            initial["revision_token"].as_str().unwrap(),
        ],
    );
    assert_eq!(out.code, 0, "{}", out.stderr);
    let anchor = Path::new(initial["spec_root"].as_str().unwrap())
        .parent()
        .unwrap();
    let message = common::git_out(anchor, &["log", "-1", "--format=%B"]);
    let metadata: Value =
        serde_json::from_str(message.split("Vibe-Spec-Metadata: ").nth(1).unwrap()).unwrap();
    assert_eq!(
        metadata["feature_introductions"],
        serde_json::json!([{"feature":"feature-one","baseline":initial["spec_revision"]}])
    );
    let after = context(&code, &store);
    assert_eq!(
        call(
            &code,
            &store,
            &[
                "document",
                "remove",
                artifact,
                "--expect",
                after["revision_token"].as_str().unwrap()
            ]
        )
        .code,
        0
    );
    let removed = context(&code, &store);
    assert_eq!(
        call(
            &code,
            &store,
            &[
                "document",
                "write",
                artifact,
                "<stdin>",
                "returning artifact",
                "--expect",
                removed["revision_token"].as_str().unwrap()
            ]
        )
        .code,
        0
    );
    let message = common::git_out(anchor, &["log", "-1", "--format=%B"]);
    let metadata: Value =
        serde_json::from_str(message.split("Vibe-Spec-Metadata: ").nth(1).unwrap()).unwrap();
    assert_eq!(metadata["feature_introductions"], serde_json::json!([]));
}
#[test]
fn snapshot_rejects_non_markdown_inside_managed_roots() {
    let (_owner, code, store) = setup(true);
    let before = context(&code, &store);
    let path = Path::new(before["spec_root"].as_str().unwrap()).join("foreign.bin");
    std::fs::write(&path, [0, 255, 17]).unwrap();
    let dirty = context(&code, &store);
    let out = call(
        &code,
        &store,
        &[
            "snapshot",
            "--expect",
            dirty["revision_token"].as_str().unwrap(),
        ],
    );
    assert_eq!(out.code, 1);
    assert!(out.stderr.contains("invalid_document"), "{}", out.stderr);
    assert_eq!(std::fs::read(path).unwrap(), [0, 255, 17]);
    assert_eq!(
        context(&code, &store)["spec_revision"],
        before["spec_revision"]
    );
}
