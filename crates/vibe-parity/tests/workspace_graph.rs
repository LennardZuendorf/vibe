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
    let meta = metadata(&["--no-deps"]);
    let n = check_graph(&meta).unwrap_or_else(|e| panic!("{e:#?}"));
    assert_eq!(n, 3);
    let edges = meta["packages"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|p| matches!(p["name"].as_str(), Some("vibe-spec" | "vibe-parity")))
        .flat_map(|p| p["dependencies"].as_array().unwrap())
        .filter(|d| d["name"] == "vibe-core")
        .count();
    assert!(edges >= 2, "examined only {edges} edges to vibe-core");
}

#[test]
fn every_resolved_package_builds_at_msrv() {
    let n = check_msrv(&metadata(&[])).unwrap_or_else(|e| panic!("{e:#?}"));
    assert!(n >= 10, "examined only {n} packages");
}

/// Every package `Cargo.lock` resolves today: 3 members plus the reviewed
/// transitive closure of clap, serde and serde_json. Sorted; duplicates kept.
const REVIEWED_PACKAGES: [&str; 20] = [
    "anstyle",
    "clap",
    "clap_builder",
    "clap_derive",
    "clap_lex",
    "heck",
    "itoa",
    "memchr",
    "proc-macro2",
    "quote",
    "serde",
    "serde_core",
    "serde_derive",
    "serde_json",
    "syn",
    "unicode-ident",
    "vibe-core",
    "vibe-parity",
    "vibe-spec",
    "zmij",
];

#[test]
fn resolved_packages_are_the_reviewed_set() {
    let meta = metadata(&[]);
    let mut resolved: Vec<&str> = meta["packages"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["name"].as_str().unwrap())
        .collect();
    resolved.sort_unstable();
    let added: Vec<&str> = resolved
        .iter()
        .filter(|n| !REVIEWED_PACKAGES.contains(n))
        .copied()
        .collect();
    let removed: Vec<&str> = REVIEWED_PACKAGES
        .iter()
        .filter(|n| !resolved.contains(n))
        .copied()
        .collect();
    assert_eq!(
        resolved, REVIEWED_PACKAGES,
        "resolved packages differ from the reviewed set; added {added:?}, removed {removed:?}. \
         A dependency change must update REVIEWED_PACKAGES in a reviewed edit."
    );
}
