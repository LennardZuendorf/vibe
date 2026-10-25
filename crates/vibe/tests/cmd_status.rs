#![allow(clippy::unwrap_used, clippy::expect_used)]
#![cfg(unix)]
//! `vibe store status` and `vibe doctor` end to end.
mod cmd_support;
mod common;
use cmd_support::*;
use vibe_core::json;

#[test]
fn status_reports_drift_and_exits_1_on_invalid_store() {
    let home = synced_home("status");
    let out = home.vibe(&["store", "status"], &[]);
    assert_eq!(out.code, 0, "{}", out.stderr);
    let text = out.text();
    assert!(text.lines().next().unwrap() == "store ok", "{text}");
    let codex = text.lines().find(|l| l.starts_with("codex: ")).unwrap();
    assert!(codex.contains("enabled=true"), "{codex}");
    assert!(codex.contains("block=intact"), "{codex}");
    assert!(codex.contains("drift=ok"), "{codex}");

    replace_in(
        &home,
        ".codex/AGENTS.md",
        "Use the shared tools.",
        "Hand edit.",
    );
    let out = home.vibe(&["store", "status"], &[]);
    assert_eq!(out.code, 0);
    let text = out.text();
    let codex = text.lines().find(|l| l.starts_with("codex: ")).unwrap();
    assert!(codex.contains("block=edited"), "{codex}");
    assert!(codex.contains("drift=drifted"), "{codex}");

    let out = home.vibe(&["store", "status", "--json"], &[]);
    assert_eq!(out.code, 0, "{}", out.stderr);
    let doc = json::parse(&out.text()).unwrap();
    assert_eq!(doc.path(&["store", "valid"]).unwrap().as_bool(), Some(true));
    assert_eq!(
        doc.path(&["harnesses", "codex", "drift"]).unwrap().as_str(),
        Some("drifted")
    );
    assert_eq!(
        doc.path(&["harnesses", "claude", "drift"])
            .unwrap()
            .as_str(),
        Some("ok")
    );
    assert_eq!(
        doc.path(&["harnesses", "droid", "enabled"])
            .unwrap()
            .as_bool(),
        Some(true)
    );

    home.write(".config/vibe/mcp.json", "{ nope");
    let out = home.vibe(&["store", "status"], &[]);
    assert_eq!(out.code, 1);
    assert!(out.text().contains("store invalid"), "{}", out.text());
    let out = home.vibe(&["store", "status", "--json"], &[]);
    assert_eq!(out.code, 1);
    let doc = json::parse(&out.text()).unwrap();
    assert_eq!(
        doc.path(&["store", "valid"]).unwrap().as_bool(),
        Some(false)
    );
}

#[test]
fn status_counts_skips_including_disabled_servers() {
    let home = synced_home("status-skips");
    replace_in(
        &home,
        ".config/vibe/mcp.json",
        "\"codex-only\"",
        "\"bad name!\":{\"command\":\"x\"},\n\"codex-only\"",
    );
    let out = home.vibe(&["store", "status", "--json"], &[]);
    let doc = json::parse(&out.text()).unwrap();
    let skips = doc.path(&["harnesses", "claude", "skips"]).unwrap();
    assert!(
        matches!(skips, json::Value::Number(n) if n.as_str() == "2"),
        "{skips:?}"
    );
}

#[test]
fn doctor_flags_missing_hook_binary() {
    let home = ready_home("doctor-hook");
    home.write(
        ".local/state/vibe/state.json",
        "{\"v\":1,\"store_hash\":\"\",\"mcp_hash\":\"\",\"targets\":{},\"owned\":{\"hooks\":{\"claude\":\"\\\"/nonexistent/dir/vibe\\\" hook session-start --harness claude\"}}}",
    );
    let out = home.vibe(&["doctor"], &[]);
    assert_eq!(out.code, 1, "{}", out.text());
    assert!(
        out.text()
            .contains("problem claude: hook binary missing \u{2014} run `vibe install`"),
        "{}",
        out.text()
    );
    assert!(out.text().contains("state ok"), "{}", out.text());
}

#[test]
fn doctor_lists_found_and_missing_clis() {
    let home = ready_home("doctor-clis");
    let out = home.vibe(&["doctor"], &[]);
    assert_eq!(out.code, 0, "{}\n{}", out.text(), out.stderr);
    let text = out.text();
    let claude = home.path("bin/claude");
    assert!(
        text.lines()
            .any(|l| l == format!("found claude {}", claude.display())),
        "{text}"
    );
    for name in ["codex", "opencode", "droid", "vibe-spec"] {
        assert!(
            text.lines().any(|l| l == format!("missing {name}")),
            "{text}"
        );
    }
    assert!(text.lines().any(|l| l == "store ok"), "{text}");
    assert!(text.lines().any(|l| l == "lock free"), "{text}");
    assert!(!text.contains("problem"), "{text}");
}

#[test]
fn doctor_reports_invalid_store_and_held_lock() {
    let home = ready_home("doctor-bad");
    home.write(".config/vibe/store.json", "{ nope");
    home.write(".local/state/vibe/lock", "4242");
    let out = home.vibe(&["doctor"], &[]);
    assert_eq!(out.code, 1);
    let text = out.text();
    assert!(
        text.lines().any(|l| l.starts_with("problem store: ")),
        "{text}"
    );
    assert!(text.lines().any(|l| l == "lock held by 4242"), "{text}");
}

#[test]
fn status_reports_missing_config_dir_as_not_enabled() {
    let home = synced_home("status-nodir");
    std::fs::remove_dir_all(home.path(".factory")).unwrap();
    let out = home.vibe(&["store", "status"], &[]);
    assert_eq!(out.code, 0, "{}", out.stderr);
    let text = out.text();
    let droid = text.lines().find(|l| l.starts_with("droid: ")).unwrap();
    assert!(droid.contains("enabled=false"), "{droid}");
    let codex = text.lines().find(|l| l.starts_with("codex: ")).unwrap();
    assert!(codex.contains("enabled=true"), "{codex}");
}

#[cfg(unix)]
#[test]
fn doctor_ignores_non_executable_files_on_path() {
    use std::os::unix::fs::PermissionsExt;
    let home = ready_home("doctor-exec");
    let bin = home.bin_dir();
    home.write("bin/codex", "#!/bin/sh\n");
    std::fs::set_permissions(bin.join("codex"), std::fs::Permissions::from_mode(0o644)).unwrap();
    let out = home.vibe(&["doctor"], &[]);
    let text = out.text();
    assert!(text.lines().any(|l| l == "missing codex"), "{text}");
    assert!(
        text.lines().any(|l| l.starts_with("found claude ")),
        "{text}"
    );
}

#[test]
fn status_reports_codex_and_droid_mcp_drift() {
    let home = synced_home("status-mcp-drift");
    let out = home.vibe(&["store", "status"], &[]);
    for id in ["codex", "droid"] {
        let line = out
            .text()
            .lines()
            .find(|l| l.starts_with(&format!("{id}: ")))
            .unwrap()
            .to_string();
        assert!(
            line.contains("drift=ok") && line.contains("mcp=ok"),
            "{line}"
        );
    }
    replace_in(&home, ".factory/mcp.json", "some-server", "hacked-server");
    replace_in(&home, ".codex/config.toml", "codex-server", "hacked-server");
    let out = home.vibe(&["store", "status", "--json"], &[]);
    let doc = json::parse(&out.text()).unwrap();
    for id in ["codex", "droid"] {
        assert_eq!(
            doc.path(&["harnesses", id, "mcp"]).unwrap().as_str(),
            Some("drifted"),
            "{id}"
        );
        assert_eq!(
            doc.path(&["harnesses", id, "drift"]).unwrap().as_str(),
            Some("drifted"),
            "{id}"
        );
    }
}
