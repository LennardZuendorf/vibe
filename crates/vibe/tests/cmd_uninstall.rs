#![allow(clippy::unwrap_used, clippy::expect_used)]
#![cfg(unix)]
//! `vibe uninstall` end to end.
mod cmd_support;
mod common;
use std::fs;

use cmd_support::*;
use common::{Home, Out};

fn assert_ok(out: &Out) {
    assert_eq!(out.code, 0, "{}\n{}", out.text(), out.stderr);
}

fn is_link(home: &Home, rel: &str) -> bool {
    fs::symlink_metadata(home.path(rel)).is_ok_and(|m| m.file_type().is_symlink())
}

#[test]
fn uninstall_restores_pre_install_bytes() {
    let home = Home::real_shaped("uninstall-restore");
    common::fake_claude(&home);
    let kit = toolkit(&home);
    let before = snapshot(&home);
    assert_ok(&install(&home, &kit, &[]));
    let out = home.vibe(&["uninstall"], &[]);
    assert_ok(&out);
    let after = snapshot(&home);
    for (rel, entry) in &before {
        match (entry, after.get(rel)) {
            (Entry::File(a, _), Some(Entry::File(b, _))) => {
                assert_eq!(a, b, "{rel} differs: {}", String::from_utf8_lossy(b));
            }
            (a, Some(b)) => assert_eq!(a, b, "{rel}"),
            (_, None) => panic!("{rel} is gone"),
        }
    }
    for rel in after.keys() {
        if before.contains_key(rel) || rel.starts_with(".config/vibe") || rel.starts_with(".local")
        {
            continue;
        }
        panic!("{rel} was created and not removed");
    }
    assert!(home.path(".config/vibe/store.json").is_file());
    assert!(home.path(".config/vibe/AGENTS.md").is_file());
    assert!(!state_path(&home).exists());
}

#[test]
fn uninstall_removes_created_directories_and_links() {
    let home = Home::with_store("uninstall-fresh");
    common::fake_claude(&home);
    let kit = toolkit(&home);
    let before = snapshot(&home);
    assert_ok(&install(&home, &kit, &[]));
    mirror_claude_json(&home);
    let out = home.vibe(&["uninstall"], &[]);
    assert_ok(&out);
    let text = out.text();
    assert!(text.lines().any(|l| l.starts_with("unlinked ")), "{text}");
    assert!(text.contains("claude mcp remove local"), "{text}");
    for rel in [
        ".claude",
        ".codex",
        ".factory",
        ".agents",
        ".config/opencode",
    ] {
        assert!(!home.path(rel).exists(), "{rel} left behind");
    }
    let after = snapshot(&home);
    for rel in after.keys() {
        assert!(
            before.contains_key(rel) || rel.starts_with(".local") || rel == ".claude.json",
            "{rel} was created and not removed"
        );
    }
    let calls = home.claude_calls();
    assert!(calls.iter().any(|c| c[1] == "remove"), "{calls:?}");
}

#[test]
fn uninstall_keeps_edited_claude_mcp() {
    let home = Home::with_store("uninstall-edited");
    common::fake_claude(&home);
    let kit = toolkit(&home);
    assert_ok(&install(&home, &kit, &[]));
    mirror_claude_json(&home);
    replace_in(&home, ".claude.json", "some-server", "hand-edited");
    clear_claude_log(&home);
    let out = home.vibe(&["uninstall"], &[]);
    assert_ok(&out);
    let text = out.text();
    assert!(
        text.lines()
            .any(|l| l == "kept claude: local edited outside vibe \u{2014} kept"),
        "{text}"
    );
    assert!(text.contains("claude mcp remove remote"), "{text}");
    let calls = home.claude_calls();
    assert_eq!(calls.len(), 1, "{calls:?}");
    assert_eq!(
        calls[0][..5],
        ["mcp", "remove", "--scope", "user", "remote"]
    );
}

#[test]
fn uninstall_keeps_replaced_links() {
    let home = Home::with_store("uninstall-links");
    common::fake_claude(&home);
    let kit = toolkit(&home);
    assert_ok(&install(&home, &kit, &[]));
    mirror_claude_json(&home);
    fs::remove_file(home.path(".claude/skills/spec")).unwrap();
    fs::create_dir_all(home.path(".claude/skills/spec")).unwrap();
    home.write(".claude/skills/spec/SKILL.md", "mine\n");
    let out = home.vibe(&["uninstall"], &[]);
    assert_ok(&out);
    let text = out.text();
    let dir = home.path(".claude/skills/spec");
    assert!(
        text.lines()
            .any(|l| l.starts_with("kept ") && l.contains(&dir.display().to_string())),
        "{text}"
    );
    assert_eq!(home.read(".claude/skills/spec/SKILL.md"), "mine\n");
    assert!(!is_link(&home, ".claude/skills/vibe"));
    assert!(!is_link(&home, ".agents/skills/spec"));
}

#[test]
fn uninstall_without_state_is_noop() {
    let home = Home::with_store("uninstall-nostate");
    common::fake_claude(&home);
    let before = snapshot(&home);
    let out = home.vibe(&["uninstall"], &[]);
    assert_ok(&out);
    assert_eq!(out.text(), "note nothing installed\n");
    assert_eq!(snapshot(&home), before);
}

#[test]
fn uninstall_dry_run_changes_nothing() {
    let home = Home::with_store("uninstall-dry");
    common::fake_claude(&home);
    let kit = toolkit(&home);
    assert_ok(&install(&home, &kit, &[]));
    mirror_claude_json(&home);
    clear_claude_log(&home);
    let before = snapshot(&home);
    let out = home.vibe(&["uninstall", "--dry-run"], &[]);
    assert_ok(&out);
    let text = out.text();
    assert!(text.lines().any(|l| l.starts_with("would ")), "{text}");
    assert!(
        text.lines()
            .all(|l| l.starts_with("would ") || l.starts_with("note ")),
        "{text}"
    );
    assert_eq!(snapshot(&home), before);
    assert!(home.claude_calls().is_empty());
}

#[test]
fn uninstall_refused_target_keeps_state_and_rerun_finishes() {
    let home = Home::with_store("uninstall-refused");
    common::fake_claude(&home);
    let kit = toolkit(&home);
    assert_ok(&install(
        &home,
        &kit,
        &["--harness", "claude,codex,opencode,droid"],
    ));
    mirror_claude_json(&home);
    let good = home.read(".factory/mcp.json");
    home.write(".factory/mcp.json", "{ nope");
    let out = home.vibe(&["uninstall"], &[]);
    assert_eq!(out.code, 1, "{}\n{}", out.text(), out.stderr);
    assert!(
        out.text()
            .lines()
            .any(|l| l.starts_with("refused ") && l.contains("mcp.json")),
        "{}",
        out.text()
    );
    assert!(state_path(&home).exists());
    assert!(home.path(".factory").is_dir());

    home.write(".factory/mcp.json", &good);
    let out = home.vibe(&["uninstall"], &[]);
    assert_ok(&out);
    assert!(!state_path(&home).exists());
    assert!(!home.path(".factory").exists());
    assert!(!home.path(".claude").exists());
}

#[test]
fn uninstall_keeps_preexisting_empty_config_dir() {
    let home = Home::with_store("uninstall-empty-dir");
    common::fake_claude(&home);
    fs::create_dir_all(home.path(".factory")).unwrap();
    let kit = toolkit(&home);
    assert_ok(&install(&home, &kit, &[]));
    mirror_claude_json(&home);
    assert_ok(&home.vibe(&["uninstall"], &[]));
    assert!(home.path(".factory").is_dir());
    assert_eq!(fs::read_dir(home.path(".factory")).unwrap().count(), 0);
    assert!(!home.path(".codex").exists());
}
