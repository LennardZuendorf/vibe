#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Global history isolation controls (spec-projects-mcp/4).
mod common;

#[test]
fn snapshot_requires_an_expected_revision() {
    let owner = common::project("snapshot-required");
    let out = common::spec(&owner, &["snapshot"], &[]);
    assert_eq!(out.code, 2);
    assert!(out.stderr.contains("--expect"), "{}", out.stderr);
}
#[test]
fn document_commands_accept_stdin_and_explicit_input_files() {
    use std::{collections::BTreeMap, ffi::OsString, io::Cursor};
    use vibe_spec::{Runtime, run_cli};
    let owner = common::project("document-cli-inputs");
    let code = owner.join("code");
    std::fs::create_dir(&code).unwrap();
    let runtime = Runtime {
        cwd: code.clone(),
        env: BTreeMap::from([
            ("PATH".into(), std::env::var_os("PATH").unwrap_or_default()),
            ("HOME".into(), owner.as_os_str().to_owned()),
            ("VIBE_STORE".into(), owner.join("store").into_os_string()),
        ]),
    };
    let invoke = |args: &[&str], input: &[u8]| {
        let args: Vec<OsString> = std::iter::once("vibe-spec")
            .chain(args.iter().copied())
            .map(OsString::from)
            .collect();
        run_cli(&args, &runtime, &mut Cursor::new(input))
    };
    assert_eq!(
        invoke(&["project", "init", "--mode", "repo-first"], b"").exit,
        0
    );
    let context: serde_json::Value =
        serde_json::from_slice(&invoke(&["context"], b"").stdout).unwrap();
    let out = invoke(
        &[
            "document",
            "write",
            "spec/stdin.md",
            "--expect",
            context["revision_token"].as_str().unwrap(),
        ],
        b"stdin bytes\r\n",
    );
    assert_eq!(out.exit, 0, "{}", String::from_utf8_lossy(&out.stderr));
    assert_eq!(
        std::fs::read(code.join("docs/spec/stdin.md")).unwrap(),
        b"stdin bytes\r\n"
    );
    std::fs::write(code.join("source.txt"), b"input file bytes\n").unwrap();
    let context: serde_json::Value =
        serde_json::from_slice(&invoke(&["context"], b"").stdout).unwrap();
    let out = invoke(
        &[
            "document",
            "write",
            "spec/file.md",
            "--input",
            "source.txt",
            "--expect",
            context["revision_token"].as_str().unwrap(),
        ],
        b"must not consume stdin",
    );
    assert_eq!(out.exit, 0, "{}", String::from_utf8_lossy(&out.stderr));
    assert_eq!(
        std::fs::read(code.join("docs/spec/file.md")).unwrap(),
        b"input file bytes\n"
    );
    let listed = invoke(&["document", "list"], b"");
    assert_eq!(listed.exit, 0);
    let listed: serde_json::Value = serde_json::from_slice(&listed.stdout).unwrap();
    assert!(
        listed["documents"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["path"] == "spec/stdin.md")
    );
    let read = invoke(&["document", "read", "spec/file.md"], b"");
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&read.stdout).unwrap()["content"],
        "input file bytes\n"
    );
}
