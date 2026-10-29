#![allow(clippy::unwrap_used, clippy::expect_used)]
//! The `vibe` command surface.
mod common;
use common::Home;

#[test]
fn help_lists_every_command() {
    let home = Home::new("help");
    let out = home.vibe(&["--help"], &[]);
    assert_eq!(out.code, 0, "{}", out.stderr);
    let text = out.text();
    for cmd in [
        "store",
        "sync",
        "hook",
        "install",
        "uninstall",
        "doctor",
        "spec",
    ] {
        assert!(text.contains(cmd), "`{cmd}` missing from help:\n{text}");
    }
}

#[test]
fn unknown_command_exits_2() {
    let home = Home::new("unknown");
    let out = home.vibe(&["frobnicate"], &[]);
    assert_eq!(out.code, 2, "{}", out.stderr);
}

#[test]
fn hook_without_store_exits_0_silently() {
    let home = Home::new("hook");
    let out = home.vibe(&["hook", "session-start", "--harness", "claude"], &[]);
    assert_eq!(out.code, 0);
    assert!(out.stdout.is_empty());
    assert!(out.stderr.is_empty(), "{}", out.stderr);
}

#[test]
fn fixtures_are_canonical_json_and_real_shaped() {
    let home = Home::real_shaped("fixtures");
    for rel in [
        ".claude/settings.json",
        ".claude.json",
        ".codex/hooks.json",
        ".factory/mcp.json",
        ".factory/hooks.json",
        ".config/agent-stack/mcp.json",
        ".config/opencode/opencode.json",
    ] {
        let text = home.read(rel);
        let value = vibe_core::json::parse(&text).unwrap();
        assert_eq!(
            text,
            format!("{}\n", vibe_core::json::to_pretty(&value).trim_end()),
            "{rel} is not canonical"
        );
    }
    assert!(!home.path(".factory/AGENTS.md").exists());
    assert!(
        home.read(".config/agent-stack/mcp.json")
            .contains("sk-test-FAKE")
    );
    #[cfg(unix)]
    {
        let meta = std::fs::symlink_metadata(home.path(".claude/AGENTS.md")).unwrap();
        assert!(meta.file_type().is_symlink());
        assert_eq!(
            home.read(".claude/AGENTS.md"),
            home.read(".config/opencode/AGENTS.md")
        );
        let foreign = std::fs::symlink_metadata(home.path(".claude/skills/foreign-link")).unwrap();
        assert!(foreign.file_type().is_symlink());
        assert!(home.path(".claude/skills/own-skill/SKILL.md").is_file());
    }
    let store = Home::with_store("store-fixture");
    assert!(store.path(".config/vibe/skills/a/SKILL.md").is_file());
    let mcp = vibe_core::json::parse(&store.read(".config/vibe/mcp.json")).unwrap();
    assert!(vibe_core::json::to_compact(&mcp).contains("codex-only"));
}

#[cfg(unix)]
#[test]
fn fake_claude_logs_exact_argv_and_fails_on_the_named_server() {
    let home = Home::new("fake-claude");
    let bin = common::fake_claude(&home);
    assert_eq!(bin, home.path("bin"));
    let tricky = r#"{"a":"b\\c \"d\""}"#;
    let run = |args: &[&str], fail: &str| {
        std::process::Command::new(bin.join("claude"))
            .args(args)
            .env("FAKE_FAIL_NAME", fail)
            .env("FAKE_EXIT", "7")
            .status()
            .unwrap()
            .code()
            .unwrap()
    };
    assert_eq!(
        run(
            &["mcp", "add-json", "--scope", "user", "good name", tricky],
            "bad"
        ),
        0
    );
    assert_eq!(
        run(&["mcp", "add-json", "--scope", "user", "bad", "{}"], "bad"),
        7
    );
    assert_eq!(run(&["mcp", "remove", "--scope", "user", "bad"], "bad"), 7);
    let calls = home.claude_calls();
    assert_eq!(calls.len(), 3, "{calls:?}");
    assert_eq!(
        calls[0],
        ["mcp", "add-json", "--scope", "user", "good name", tricky]
    );
    assert_eq!(calls[2], ["mcp", "remove", "--scope", "user", "bad"]);
}

#[cfg(unix)]
#[test]
fn default_path_reaches_the_fake_and_not_the_real_path() {
    let home = Home::new("path");
    common::fake_claude(&home);
    let out = home.vibe(&["hook", "session-start"], &[]);
    assert_eq!(out.code, 0);
    let found = std::process::Command::new("/bin/sh")
        .args(["-c", "command -v claude"])
        .env_clear()
        .env("PATH", home.path("bin"))
        .output()
        .unwrap();
    assert!(found.status.success());
    let empty = Home::new("path-empty");
    let none = std::process::Command::new("/bin/sh")
        .args(["-c", "command -v claude"])
        .env_clear()
        .env(
            "PATH",
            format!("{}:/usr/bin:/bin", empty.path("bin").display()),
        )
        .output()
        .unwrap();
    assert!(!none.status.success(), "a real claude is on /usr/bin:/bin");
}
