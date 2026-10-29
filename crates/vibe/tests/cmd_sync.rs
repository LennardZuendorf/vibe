#![allow(clippy::unwrap_used, clippy::expect_used)]
#![cfg(unix)]
//! `vibe sync` end to end.
mod cmd_support;
mod common;
use cmd_support::*;
use vibe_core::json;
use vibe_core::lock::{Acquired, Lock, STALE};

#[test]
fn sync_writes_blocks_links_and_claude_servers() {
    let home = ready_home("sync-fresh");
    let out = home.vibe(&["sync"], &[]);
    assert_eq!(out.code, 0, "{}\n{}", out.text(), out.stderr);
    let text = out.text();
    assert!(text.contains("wrote "), "{text}");
    assert!(text.contains("claude mcp add local"), "{text}");
    assert!(text.contains("claude mcp add remote"), "{text}");
    assert!(home.read(".claude/CLAUDE.md").contains("@"));
    assert!(
        home.read(".codex/AGENTS.md")
            .contains("Use the shared tools.")
    );
    let calls = home.claude_calls();
    assert_eq!(calls.len(), 2, "{calls:?}");
    assert_eq!(calls[0][..3], ["mcp", "add-json", "--scope"]);
    let st = state(&home);
    assert_ne!(st.path(&["store_hash"]).unwrap().as_str().unwrap(), "");
    assert_ne!(st.path(&["mcp_hash"]).unwrap().as_str().unwrap(), "");
}

#[test]
fn sync_second_run_has_nothing_to_do() {
    let home = synced_home("sync-twice");
    let out = home.vibe(&["sync"], &[]);
    assert_eq!(out.code, 0, "{}", out.stderr);
    assert!(out.text().contains("nothing to do"), "{}", out.text());
    assert!(home.claude_calls().is_empty());
}

#[test]
fn sync_dry_run_writes_nothing() {
    let home = ready_home("sync-dry");
    let out = home.vibe(&["sync", "--dry-run"], &[]);
    assert_eq!(out.code, 0, "{}", out.stderr);
    let text = out.text();
    assert!(
        text.lines().any(|l| l.starts_with("would write ")),
        "{text}"
    );
    assert!(
        text.lines().all(|l| !l.starts_with("wrote ")
            && !l.starts_with("linked ")
            && !l.starts_with("claude ")),
        "{text}"
    );
    assert!(!text.contains("would wrote"), "{text}");
    assert!(!home.path(".claude/CLAUDE.md").exists());
    assert!(!state_path(&home).exists());
    assert!(!home.path(".local/state/vibe").exists());
    assert!(home.claude_calls().is_empty());
}

#[test]
fn sync_after_store_edit_updates_files_and_claude_cli() {
    let home = synced_home("sync-edit");
    mirror_claude_json(&home);
    replace_in(
        &home,
        ".config/vibe/mcp.json",
        "some-server",
        "other-server",
    );
    let out = home.vibe(&["sync"], &[]);
    assert_eq!(out.code, 0, "{}\n{}", out.text(), out.stderr);
    let calls = home.claude_calls();
    assert_eq!(calls.len(), 2, "{calls:?}");
    assert_eq!(calls[0][..5], ["mcp", "remove", "--scope", "user", "local"]);
    assert_eq!(
        calls[1][..5],
        ["mcp", "add-json", "--scope", "user", "local"]
    );
    assert!(calls[1][5].contains("other-server"), "{calls:?}");
    let text = out.text();
    assert!(text.contains("claude mcp remove local"), "{text}");
    assert!(text.contains("claude mcp add local"), "{text}");
    let factory = home.read(".factory/mcp.json");
    assert!(factory.contains("other-server"), "{factory}");
    assert!(!factory.contains("some-server"), "{factory}");
}

#[test]
fn sync_json_shape() {
    let home = ready_home("sync-json");
    let out = home.vibe(&["sync", "--json"], &[]);
    assert_eq!(out.code, 0, "{}", out.stderr);
    let doc = json::parse(&out.text()).unwrap();
    assert_eq!(doc.path(&["v"]).unwrap().as_str(), None);
    assert!(matches!(doc.path(&["v"]), Some(json::Value::Number(n)) if n.as_str() == "1"));
    assert_eq!(doc.path(&["dry_run"]).unwrap().as_bool(), Some(false));
    let changes = doc.path(&["changes"]).unwrap().as_array().unwrap();
    assert!(
        changes
            .iter()
            .any(|c| c.as_str().unwrap().starts_with("wrote "))
    );
    assert!(doc.path(&["notes"]).unwrap().as_array().is_some());
    assert_eq!(doc.path(&["failed"]).unwrap().as_array().unwrap().len(), 0);
    let dry = home.vibe(&["sync", "--json", "--dry-run"], &[]);
    let doc = json::parse(&dry.text()).unwrap();
    assert_eq!(doc.path(&["dry_run"]).unwrap().as_bool(), Some(true));
}

#[test]
fn sync_reports_edit_inside_block() {
    let home = synced_home("sync-drift");
    let before = home.read(".codex/AGENTS.md");
    replace_in(
        &home,
        ".codex/AGENTS.md",
        "Use the shared tools.",
        "Edited by hand.",
    );
    let out = home.vibe(&["sync"], &[]);
    assert_eq!(out.code, 0, "{}", out.stderr);
    let text = out.text();
    assert!(
        text.lines().any(|l| l.starts_with("note codex: ")
            && l.ends_with("edited inside the vibe block \u{2014} replaced")),
        "{text}"
    );
    assert_eq!(home.read(".codex/AGENTS.md"), before);
}

#[test]
fn sync_lock_busy_exits_1() {
    let home = ready_home("sync-busy");
    let _held = match Lock::acquire(&home.path(".local/state/vibe/lock"), STALE).unwrap() {
        Acquired::Got(l) => l,
        Acquired::Busy => panic!("lock busy"),
    };
    let out = home.vibe(&["sync"], &[]);
    assert_eq!(out.code, 1, "{}", out.stderr);
    assert!(
        out.text().contains("note another vibe run holds the lock"),
        "{}",
        out.text()
    );
    assert!(!home.path(".claude/CLAUDE.md").exists());
}

#[test]
fn sync_filter_harness_only_touches_it() {
    let home = ready_home("sync-filter");
    let out = home.vibe(&["sync", "--harness", "codex"], &[]);
    assert_eq!(out.code, 0, "{}\n{}", out.text(), out.stderr);
    assert!(home.path(".codex/AGENTS.md").exists());
    assert!(!home.path(".claude/CLAUDE.md").exists());
    assert!(!home.path(".factory/AGENTS.md").exists());
    assert!(home.claude_calls().is_empty());
    let st = state(&home);
    assert_eq!(st.path(&["store_hash"]).unwrap().as_str().unwrap(), "");
    let bad = home.vibe(&["sync", "--harness", "nope"], &[]);
    assert_eq!(bad.code, 2, "{}", bad.stderr);
}

#[test]
fn sync_invalid_store_is_refused() {
    let home = ready_home("sync-invalid");
    home.write(".config/vibe/mcp.json", "{ not json");
    let out = home.vibe(&["sync"], &[]);
    assert_eq!(out.code, 1);
    assert!(
        out.text().lines().any(|l| l.starts_with("refused ")),
        "{}",
        out.text()
    );
}

#[test]
fn sync_failed_claude_call_exits_1_without_stamp() {
    let home = ready_home("sync-claude-fail");
    let out = home.vibe(&["sync"], &[("FAKE_FAIL_NAME", "remote")]);
    assert_eq!(out.code, 1, "{}", out.text());
    assert!(
        out.stderr.contains("claude mcp add remote"),
        "{}",
        out.stderr
    );
    let st = state(&home);
    assert_eq!(st.path(&["store_hash"]).unwrap().as_str().unwrap(), "");
}

#[test]
fn sync_without_claude_cli_skips_mcp_and_exits_zero() {
    let home = common::Home::with_store("sync-noclaude");
    for dir in [".claude", ".codex", ".factory", ".config/opencode"] {
        std::fs::create_dir_all(home.path(dir)).unwrap();
    }
    let out = home.vibe(&["sync"], &[]);
    assert_eq!(out.code, 0, "{}\n{}", out.text(), out.stderr);
    let text = out.text();
    assert!(
        text.lines()
            .any(|l| l == "skip claude: CLI not found \u{2014} MCP not projected"),
        "{text}"
    );
    assert!(out.stderr.is_empty(), "{}", out.stderr);
    assert!(
        home.read(".codex/AGENTS.md")
            .contains("Use the shared tools.")
    );
    assert!(home.read(".factory/mcp.json").contains("\"local\""));
    let st = state(&home);
    assert_ne!(st.path(&["store_hash"]).unwrap().as_str().unwrap(), "");
    assert!(
        st.path(&["owned", "mcp", "claude"])
            .and_then(|v| v.as_object())
            .is_none_or(|o| o.is_empty())
    );
}

#[test]
fn sync_without_claude_cli_leaves_claude_mcp_out_of_date() {
    let home = common::Home::with_store("sync-noclaude-stale");
    for dir in [".claude", ".codex", ".factory", ".config/opencode"] {
        std::fs::create_dir_all(home.path(dir)).unwrap();
    }
    let stale = "claude mcp: out of date \u{2014} run vibe sync";
    let hook = |h: &common::Home| h.vibe(&["hook", "session-start", "--harness", "claude"], &[]);

    let out = home.vibe(&["sync"], &[]);
    assert_eq!(out.code, 0, "{}\n{}", out.text(), out.stderr);
    let st = state(&home);
    assert_ne!(st.path(&["store_hash"]).unwrap().as_str().unwrap(), "");
    let status = home.vibe(&["store", "status"], &[]);
    assert!(
        status.text().lines().any(|l| l == stale),
        "{}",
        status.text()
    );
    let out = hook(&home);
    assert_eq!(out.code, 0, "{}", out.stderr);
    assert!(
        out.text()
            .contains("vibe: Claude MCP changed; run `vibe sync`"),
        "{}",
        out.text()
    );

    common::fake_claude(&home);
    let out = home.vibe(&["sync"], &[]);
    assert_eq!(out.code, 0, "{}\n{}", out.text(), out.stderr);
    assert!(
        home.claude_calls()
            .iter()
            .any(|c| c.get(1).is_some_and(|a| a == "add-json")),
        "{:?}",
        home.claude_calls()
    );
    let status = home.vibe(&["store", "status"], &[]);
    assert!(!status.text().contains("claude mcp:"), "{}", status.text());
    assert!(hook(&home).stdout.is_empty());
}

#[test]
fn sync_reports_factory_mcp_entries_edited_outside_vibe() {
    let home = synced_home("sync-droid-drift");
    replace_in(&home, ".factory/mcp.json", "some-server", "hacked-server");
    let out = home.vibe(&["sync"], &[]);
    assert_eq!(out.code, 0, "{}\n{}", out.text(), out.stderr);
    assert!(
        out.text()
            .lines()
            .any(|l| l == "note droid: mcp.json vibe entries edited outside vibe \u{2014} replaced"),
        "{}",
        out.text()
    );
    assert!(home.read(".factory/mcp.json").contains("some-server"));
    let again = home.vibe(&["sync"], &[]);
    assert!(again.text().contains("nothing to do"), "{}", again.text());
    assert!(!again.text().contains("edited outside"), "{}", again.text());
}

#[test]
fn imported_env_secret_projects_to_codex_env_vars() {
    let home = common::Home::new("sync-import-codex");
    std::fs::create_dir_all(home.path(".codex")).unwrap();
    assert_eq!(home.vibe(&["store", "init"], &[]).code, 0);
    home.write_json(
        "in.json",
        r#"{"mcpServers":{"exa":{"command":"exa-mcp","env":{"API_TOKEN":"sk-test-FAKE"}}}}"#,
    );
    let out = home.vibe(&["store", "import-mcp", "in.json"], &[]);
    assert_eq!(out.code, 0, "{}", out.stderr);
    let out = home.vibe(&["sync", "--harness", "codex"], &[]);
    assert_eq!(out.code, 0, "{}\n{}", out.text(), out.stderr);
    let toml = home.read(".codex/config.toml");
    assert!(toml.contains("[mcp_servers.exa]"), "{toml}");
    assert!(toml.contains("env_vars = [\"API_TOKEN\"]"), "{toml}");
    assert!(!out.text().contains("skip codex: exa"), "{}", out.text());
}
