#![cfg(unix)]
#![allow(clippy::unwrap_used, clippy::expect_used)]
//! The spec peer's absence is a fixture, not an assumption (F8, issue #28).
//!
//! `orders`/`emit` enrich the payload's edge from a *soft* `vibe-spec` probe.
//! The contract is that a missing binary degrades to a path pointer and no
//! lessons — never a failed session and never a silent omission of the pointer
//! itself. Testing it in-process would read the ambient PATH, which is exactly
//! the assumption this fixture exists to remove, so the case runs the real
//! binary with a PATH of our choosing.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

use serde_json::Value;

static COUNTER: AtomicU64 = AtomicU64::new(0);

fn scratch(name: &str) -> PathBuf {
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "vibe-flow-degrade-{name}-{}-{n}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn copy(src: &str, dest: &Path) {
    if let Some(dir) = dest.parent() {
        fs::create_dir_all(dir).unwrap();
    }
    fs::copy(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../")
            .join(src),
        dest,
    )
    .unwrap();
}

/// A project mid-flow: the shipped machine and skill, and a cursor in
/// `feature.impl` with a feature name.
fn install(name: &str) -> PathBuf {
    let root = scratch(name);
    copy(
        "flow/state-machine.json",
        &root.join(".agents/skills/vibe/state-machine.json"),
    );
    copy("flow/SKILL.md", &root.join(".agents/skills/vibe/SKILL.md"));
    let run = root.join(".vibe/run/flow");
    fs::create_dir_all(&run).unwrap();
    fs::write(
        run.join("state.json"),
        "{\"flow\":\"feature\",\"phase\":\"impl\",\"feature\":\"auth\",\"updated\":\"2026-01-01T00:00:00Z\"}\n",
    )
    .unwrap();
    root
}

/// `vibe-flow --root <root> emit` with exactly `path` on PATH.
fn emit(root: &Path, path: &Path) -> Value {
    let home = scratch("home");
    let output = Command::new(env!("CARGO_BIN_EXE_vibe-flow"))
        .arg("--root")
        .arg(root)
        .arg("emit")
        .env_clear()
        .env("PATH", path)
        .env("HOME", &home)
        .env("LC_ALL", "C")
        .env("TZ", "UTC")
        .output()
        .expect("spawn vibe-flow");
    assert!(
        output.status.success(),
        "emit failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("emit prints the payload JSON")
}

#[test]
fn without_vibe_spec_on_path_the_edge_is_the_path_pointer_and_no_lessons() {
    let root = install("absent");
    // A spec tree that a reachable peer would have reported: its presence must
    // not tempt an in-process read. Only the binary decides, and the binary is
    // not on this PATH.
    fs::create_dir_all(root.join("spec")).unwrap();
    fs::write(root.join("spec/lessons.md"), "# lessons\n").unwrap();
    let empty_path = scratch("empty-path");

    let payload = emit(&root, &empty_path);
    let edge = payload["edge"].as_str().expect("edge is a string");
    assert!(
        edge.contains("spec: docs/spec/"),
        "the pointer must survive the degrade: {edge}"
    );
    assert!(
        edge.contains("vibe-spec is unavailable, so no tagged lessons."),
        "the degrade line must name the reason: {edge}"
    );
    assert!(
        !edge.contains("lessons ("),
        "no peer answered, so no lessons may be cited: {edge}"
    );
}

#[test]
fn a_reachable_vibe_spec_contributes_its_root_and_tagged_lessons() {
    let root = install("present");
    let bin = scratch("stub-bin");
    let stub = bin.join("vibe-spec");
    fs::write(
        &stub,
        "#!/bin/sh\nset -eu\ncase \"${1:-}\" in\n  root) printf '{\"root\":\"docs/spec\"}\\n' ;;\n  lessons-for) printf '{\"lessons\":[{\"title\":\"T\",\"rule\":\"R\"}]}\\n' ;;\n  *) exit 2 ;;\nesac\n",
    )
    .unwrap();
    fs::set_permissions(&stub, fs::Permissions::from_mode(0o755)).unwrap();

    let payload = emit(&root, &bin);
    let edge = payload["edge"].as_str().expect("edge is a string");
    assert!(edge.contains("spec: docs/spec"), "{edge}");
    assert!(edge.contains("lessons (feature.impl):"), "{edge}");
    assert!(edge.contains("- T: R"), "{edge}");
    assert!(!edge.contains("vibe-spec is unavailable"), "{edge}");
}
