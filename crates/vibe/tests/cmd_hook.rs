#![allow(clippy::unwrap_used, clippy::expect_used)]
#![cfg(unix)]
//! `vibe hook session-start` end to end.
mod cmd_support;
mod common;
use std::fs;
use std::path::Path;
use std::process::Command;
use std::time::{Duration, SystemTime};

use cmd_support::*;
use common::{Home, Out};
use vibe_core::lock::{Acquired, Lock, STALE};

fn hook(home: &Home) -> Out {
    home.vibe(&["hook", "session-start", "--harness", "claude"], &[])
}

/// The user-visible message of a hook run: the one JSON line's `systemMessage`.
fn message(out: &Out) -> String {
    let text = out.text();
    assert_eq!(text.lines().count(), 1, "{text}");
    let doc = vibe_core::json::parse(text.trim_end()).unwrap();
    let obj = doc.as_object().unwrap();
    assert_eq!(obj.len(), 1, "{text}");
    obj.get("systemMessage")
        .and_then(|v| v.as_str())
        .unwrap()
        .to_string()
}

fn raw_hook(home: &Home, path: &Path, with_home: bool) -> Out {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_vibe"));
    cmd.args(["hook", "session-start"])
        .current_dir(&home.dir)
        .env_clear()
        .env("PATH", path)
        .env("LC_ALL", "C");
    if with_home {
        cmd.env("HOME", &home.dir);
    }
    let o = cmd.output().unwrap();
    Out {
        code: o.status.code().unwrap_or(-1),
        stdout: o.stdout,
        stderr: String::from_utf8_lossy(&o.stderr).into_owned(),
    }
}

fn assert_fail_open(out: &Out) {
    assert_eq!(out.code, 0, "{}", out.stderr);
    assert!(out.stdout.is_empty(), "{}", out.text());
    assert!(out.stderr.lines().count() <= 1, "{}", out.stderr);
    assert!(!out.stderr.contains("panicked"), "{}", out.stderr);
}

fn mtime(path: &Path) -> SystemTime {
    fs::metadata(path).unwrap().modified().unwrap()
}

fn append_store_instructions(home: &Home) {
    let text = home.read(".config/vibe/AGENTS.md");
    home.write(".config/vibe/AGENTS.md", &format!("{text}\nNew rule.\n"));
}

#[test]
fn hook_unchanged_store_is_silent_and_writes_nothing() {
    let home = synced_home("hook-quiet");
    let state_before = fs::read(state_path(&home)).unwrap();
    let state_time = mtime(&state_path(&home));
    let codex_time = mtime(&home.path(".codex/AGENTS.md"));
    let out = hook(&home);
    assert_eq!(out.code, 0, "{}", out.stderr);
    assert!(out.stdout.is_empty(), "{}", out.text());
    assert!(out.stderr.is_empty(), "{}", out.stderr);
    assert_eq!(fs::read(state_path(&home)).unwrap(), state_before);
    assert_eq!(mtime(&state_path(&home)), state_time);
    assert_eq!(mtime(&home.path(".codex/AGENTS.md")), codex_time);
}

#[test]
fn hook_after_store_edit_reprojects_files_and_prints_one_notice() {
    let home = synced_home("hook-edit");
    append_store_instructions(&home);
    let out = hook(&home);
    assert_eq!(out.code, 0, "{}", out.stderr);
    let text = message(&out);
    assert!(
        text.starts_with("vibe: store changed \u{2014} refreshed "),
        "{text}"
    );
    assert!(text.contains("codex"), "{text}");
    assert!(!text.contains("vibe sync"), "{text}");
    assert!(home.read(".codex/AGENTS.md").contains("New rule."));
    assert!(home.claude_calls().is_empty());
    let again = hook(&home);
    assert!(again.stdout.is_empty(), "{}", again.text());
    assert_eq!(again.code, 0);

    replace_in(
        &home,
        ".config/vibe/mcp.json",
        "some-server",
        "other-server",
    );
    let out = hook(&home);
    let text = message(&out);
    assert!(text.contains("; run `vibe sync` for Claude MCP"), "{text}");
    assert!(home.claude_calls().is_empty());
    assert_eq!(
        message(&hook(&home)),
        "vibe: Claude MCP changed; run `vibe sync`"
    );
}

#[test]
fn hook_claude_only_mcp_change_prints_sync_hint() {
    let home = synced_home("hook-claude-mcp");
    replace_in(
        &home,
        ".config/vibe/mcp.json",
        "\"codex-only\"",
        "\"claude-only\":{\"command\":\"c-server\",\"harnesses\":[\"claude\"]},\n\"codex-only\"",
    );
    let out = hook(&home);
    assert_eq!(out.code, 0, "{}", out.stderr);
    assert_eq!(message(&out), "vibe: Claude MCP changed; run `vibe sync`");
    assert!(home.claude_calls().is_empty());
}

#[test]
fn hook_mcp_reminder_repeats_every_session_until_sync() {
    let home = synced_home("hook-remind");
    replace_in(
        &home,
        ".config/vibe/mcp.json",
        "\"codex-only\"",
        "\"claude-only\":{\"command\":\"c-server\",\"harnesses\":[\"claude\"]},\n\"codex-only\"",
    );
    for _ in 0..3 {
        let out = hook(&home);
        assert_eq!(out.code, 0, "{}", out.stderr);
        assert_eq!(message(&out), "vibe: Claude MCP changed; run `vibe sync`");
    }
    let status = home.vibe(&["store", "status"], &[]);
    assert!(
        status
            .text()
            .lines()
            .any(|l| l == "claude mcp: out of date \u{2014} run vibe sync"),
        "{}",
        status.text()
    );
    let sync = home.vibe(&["sync"], &[]);
    assert_eq!(sync.code, 0, "{}", sync.stderr);
    assert!(hook(&home).stdout.is_empty());
    let status = home.vibe(&["store", "status"], &[]);
    assert!(!status.text().contains("claude mcp:"), "{}", status.text());
}

#[test]
fn hook_output_is_json_for_claude_codex_droid_and_plain_otherwise() {
    for (name, plain) in [
        ("claude", false),
        ("codex", false),
        ("droid", false),
        ("opencode", true),
    ] {
        let home = synced_home(&format!("hook-fmt-{name}"));
        append_store_instructions(&home);
        let out = home.vibe(&["hook", "session-start", "--harness", name], &[]);
        assert_eq!(out.code, 0, "{}", out.stderr);
        let text = out.text();
        if plain {
            assert!(text.starts_with("vibe: store changed"), "{text}");
        } else {
            assert!(
                message(&out).starts_with("vibe: store changed"),
                "{name}: {text}"
            );
            assert!(text.starts_with("{\"systemMessage\":\""), "{text}");
        }
    }
}

#[test]
fn hook_fail_open_cases() {
    let bin = home_path_for_raw();
    // unset HOME
    let home = Home::new("hook-nohome");
    assert_fail_open(&raw_hook(&home, &bin, false));

    // corrupt state
    let home = synced_home("hook-corrupt");
    fs::write(state_path(&home), "{ nope").unwrap();
    append_store_instructions(&home);
    let out = hook(&home);
    assert_fail_open(&out);
    assert!(out.stderr.starts_with("vibe hook: "), "{}", out.stderr);
    assert!(out.stderr.trim_end().ends_with("skipped"), "{}", out.stderr);

    // missing store
    let home = Home::new("hook-nostore");
    assert_fail_open(&hook(&home));

    // invalid store
    let home = synced_home("hook-badstore");
    home.write(".config/vibe/store.json", "{ nope");
    assert_fail_open(&hook(&home));

    // held lock
    let home = synced_home("hook-held");
    append_store_instructions(&home);
    let _held = match Lock::acquire(&home.path(".local/state/vibe/lock"), STALE).unwrap() {
        Acquired::Got(l) => l,
        Acquired::Busy => panic!("busy"),
    };
    let out = hook(&home);
    assert_fail_open(&out);
    assert!(out.stderr.is_empty(), "{}", out.stderr);
}

fn home_path_for_raw() -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("vibe-test-emptybin-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[cfg(unix)]
#[test]
fn hook_read_only_target_fails_open_then_retries() {
    use std::os::unix::fs::PermissionsExt;
    let home = synced_home("hook-readonly");
    append_store_instructions(&home);
    let dir = home.path(".codex");
    fs::set_permissions(&dir, fs::Permissions::from_mode(0o555)).unwrap();
    let writable = fs::File::create(dir.join("probe")).is_ok();
    let out = hook(&home);
    fs::set_permissions(&dir, fs::Permissions::from_mode(0o755)).unwrap();
    assert_fail_open(&out);
    if !writable {
        assert_eq!(out.stderr.lines().count(), 1, "{}", out.stderr);
        assert!(out.stderr.starts_with("vibe hook: "), "{}", out.stderr);
        assert!(!home.read(".codex/AGENTS.md").contains("New rule."));
        let retry = hook(&home);
        assert_eq!(retry.code, 0, "{}", retry.stderr);
        assert!(
            message(&retry).starts_with("vibe: store changed"),
            "{}",
            retry.text()
        );
        assert!(home.read(".codex/AGENTS.md").contains("New rule."));
    }
}

#[test]
fn hook_second_concurrent_run_silent() {
    let home = synced_home("hook-concurrent");
    append_store_instructions(&home);
    let _held = match Lock::acquire(&home.path(".local/state/vibe/lock"), STALE).unwrap() {
        Acquired::Got(l) => l,
        Acquired::Busy => panic!("busy"),
    };
    let out = hook(&home);
    assert_eq!(out.code, 0);
    assert!(out.stdout.is_empty());
    assert!(out.stderr.is_empty(), "{}", out.stderr);
    assert!(!home.read(".codex/AGENTS.md").contains("New rule."));
}

#[test]
fn hook_after_crash_stale_lock_recovered() {
    let home = synced_home("hook-stale");
    append_store_instructions(&home);
    let lock = home.path(".local/state/vibe/lock");
    fs::write(&lock, "99999").unwrap();
    let old = SystemTime::now() - Duration::from_secs(120);
    fs::File::options()
        .write(true)
        .open(&lock)
        .unwrap()
        .set_modified(old)
        .unwrap();
    let out = hook(&home);
    assert_eq!(out.code, 0, "{}", out.stderr);
    assert!(
        message(&out).starts_with("vibe: store changed"),
        "{}",
        out.text()
    );
    assert!(home.read(".codex/AGENTS.md").contains("New rule."));
}

#[test]
fn hook_does_not_spawn_subprocesses() {
    let home = synced_home("hook-nospawn");
    append_store_instructions(&home);
    replace_in(
        &home,
        ".config/vibe/mcp.json",
        "some-server",
        "other-server",
    );
    let out = raw_hook(&home, &home.bin_dir(), true);
    assert_eq!(out.code, 0, "{}", out.stderr);
    assert!(out.stderr.is_empty(), "{}", out.stderr);
    let text = out.text();
    assert_eq!(text.lines().count(), 1, "{text}");
    assert!(
        text.starts_with("vibe: store changed \u{2014} refreshed "),
        "{text}"
    );
    assert!(home.read(".codex/AGENTS.md").contains("New rule."));
    assert!(home.claude_calls().is_empty());
}
