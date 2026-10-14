#![allow(clippy::unwrap_used, clippy::expect_used)]
//! The real workspace satisfies the structural rules in `vibe_parity::workspace`.
use std::process::Command;

use serde_json::Value;
use vibe_parity::workspace::{check_graph, check_msrv};

fn metadata(extra: &[&str]) -> Value {
    let manifest = concat!(env!("CARGO_MANIFEST_DIR"), "/../../Cargo.toml");
    let out = Command::new(env!("CARGO"))
        .args([
            "metadata",
            "--format-version",
            "1",
            "--locked",
            "--manifest-path",
            manifest,
        ])
        .args(extra)
        .output()
        .expect("run cargo metadata");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).expect("parse cargo metadata")
}

#[test]
fn member_graph_holds() {
    let n = check_graph(&metadata(&["--no-deps"])).unwrap_or_else(|e| panic!("{e:#?}"));
    assert_eq!(n, 3);
}

#[test]
fn every_resolved_package_builds_at_msrv() {
    let n = check_msrv(&metadata(&[])).unwrap_or_else(|e| panic!("{e:#?}"));
    assert!(n >= 10, "examined only {n} packages");
}
