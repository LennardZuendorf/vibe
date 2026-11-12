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
    assert_eq!(n, 5);
    let edges = meta["packages"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|p| {
            matches!(
                p["name"].as_str(),
                Some("vibe" | "vibe-spec" | "vibe-parity" | "vibe-flow")
            )
        })
        .flat_map(|p| p["dependencies"].as_array().unwrap())
        .filter(|d| d["name"] == "vibe-core")
        .count();
    assert!(edges >= 4, "examined only {edges} edges to vibe-core");
}

#[test]
fn resolved_msrv_declarations_and_exact_reviewed_exceptions_hold() {
    let n = check_msrv(&metadata(&[])).unwrap_or_else(|e| panic!("{e:#?}"));
    assert!(n >= 10, "examined only {n} packages");
}

#[test]
fn resolved_packages_are_the_reviewed_set() {
    let meta = metadata(&[]);
    vibe_parity::workspace::check_resolved_graph(&meta).unwrap();
    for feature in ["client", "auth", "transport-streamable-http-server"] {
        let mut bad = meta.clone();
        let id = bad["packages"]
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["name"] == "rmcp")
            .unwrap()["id"]
            .clone();
        bad["resolve"]["nodes"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|n| n["id"] == id)
            .unwrap()["features"]
            .as_array_mut()
            .unwrap()
            .push(serde_json::json!(feature));
        assert!(vibe_parity::workspace::check_resolved_graph(&bad).is_err());
    }
    let mut bad = meta.clone();
    bad["packages"][0]["version"] = serde_json::json!("999.0.0");
    assert!(vibe_parity::workspace::check_resolved_graph(&bad).is_err());
}

#[test]
fn spec_mcp_exception_cannot_spread_or_broaden_features() {
    // spec-projects-mcp/8: already-reviewed names cannot hide new capability edges.
    let original = metadata(&["--no-deps"]);
    check_graph(&original).unwrap();
    for member in ["vibe", "vibe-core", "vibe-flow", "vibe-parity"] {
        let mut bad = original.clone();
        let packages = bad["packages"].as_array_mut().unwrap();
        let spec = packages.iter().find(|p| p["name"] == "vibe-spec").unwrap();
        let dep = spec["dependencies"]
            .as_array()
            .unwrap()
            .iter()
            .find(|d| d["name"] == "tokio")
            .unwrap()
            .clone();
        packages.iter_mut().find(|p| p["name"] == member).unwrap()["dependencies"]
            .as_array_mut()
            .unwrap()
            .push(dep);
        assert!(
            check_graph(&bad).is_err(),
            "async dependency escaped into {member}"
        );
    }
    for (dependency, feature) in [
        ("rmcp", "client"),
        ("rmcp", "transport-streamable-http-server"),
        ("rmcp", "auth"),
        ("tokio", "process"),
        ("tokio", "full"),
    ] {
        let mut bad = original.clone();
        let spec = bad["packages"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|p| p["name"] == "vibe-spec")
            .unwrap();
        spec["dependencies"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|d| d["name"] == dependency)
            .unwrap()["features"]
            .as_array_mut()
            .unwrap()
            .push(serde_json::json!(feature));
        assert!(
            check_graph(&bad).is_err(),
            "extra {dependency}/{feature} capability escaped"
        );
    }
}

#[test]
fn reviewed_package_names_cannot_hide_renamed_target_build_edges() {
    let mut meta = metadata(&["--no-deps"]);
    check_graph(&meta).unwrap();
    let packages = meta["packages"].as_array_mut().unwrap();
    let mut dep = packages.iter().find(|p| p["name"] == "vibe-spec").unwrap()["dependencies"]
        .as_array()
        .unwrap()
        .iter()
        .find(|d| d["name"] == "tokio")
        .unwrap()
        .clone();
    dep["rename"] = serde_json::json!("innocent_alias");
    dep["kind"] = serde_json::json!("build");
    dep["target"] = serde_json::json!("cfg(unix)");
    packages
        .iter_mut()
        .find(|p| p["name"] == "vibe-core")
        .unwrap()["dependencies"]
        .as_array_mut()
        .unwrap()
        .push(dep);
    assert!(check_graph(&meta).is_err());
    let original = metadata(&[]);
    for field in ["source", "version"] {
        let mut bad = original.clone();
        let pkg = bad["packages"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|p| p["name"] == "ident_case")
            .unwrap();
        pkg[field] = serde_json::json!(if field == "source" {
            "registry+https://example.invalid/index"
        } else {
            "1.0.2"
        });
        assert!(check_msrv(&bad).is_err());
        assert!(vibe_parity::workspace::check_resolved_graph(&bad).is_err());
    }
    let mut bad = original.clone();
    bad["packages"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|p| p["name"] == "ident_case")
        .unwrap()["rust_version"] = serde_json::json!("1.89");
    assert!(check_msrv(&bad).is_err());
    let mut bad = original.clone();
    let target = bad["resolve"]["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|n| n["deps"].as_array().is_some_and(|a| !a.is_empty()))
        .unwrap()["deps"][0]
        .clone();
    bad["resolve"]["nodes"].as_array_mut().unwrap()[0]["deps"]
        .as_array_mut()
        .unwrap()
        .push(target);
    assert!(vibe_parity::workspace::check_resolved_graph(&bad).is_err());
}
