#![allow(clippy::unwrap_used, clippy::expect_used)]
#![cfg(unix)]
//! `vibe install` end to end.
mod cmd_support;
mod common;
use std::fs;
use std::path::Path;
use std::process::Command;

use cmd_support::*;
use common::{Home, Out};
use vibe_core::json::{self, Value};

const ALL: [&str; 2] = ["--harness", "claude,codex,opencode,droid"];

fn assert_ok(out: &Out) {
    assert_eq!(out.code, 0, "{}\n{}", out.text(), out.stderr);
}

fn is_link(home: &Home, rel: &str) -> bool {
    fs::symlink_metadata(home.path(rel)).is_ok_and(|m| m.file_type().is_symlink())
}

fn session_start_commands(home: &Home, rel: &str) -> Vec<String> {
    let doc = json::parse(&home.read(rel)).unwrap();
    let mut out = Vec::new();
    for group in doc
        .path(&["hooks", "SessionStart"])
        .unwrap()
        .as_array()
        .unwrap()
    {
        for h in group.path(&["hooks"]).unwrap().as_array().unwrap() {
            out.push(h.path(&["command"]).unwrap().as_str().unwrap().to_string());
        }
    }
    out
}

#[test]
fn install_fresh_home_wires_all_four() {
    let home = Home::with_store("install-fresh");
    common::fake_claude(&home);
    let kit = toolkit(&home);
    let out = install(&home, &kit, &ALL);
    assert_ok(&out);
    let text = out.text();
    assert!(text.lines().any(|l| l.starts_with("wrote ")), "{text}");
    assert!(text.lines().any(|l| l.starts_with("linked ")), "{text}");
    assert!(text.contains("claude mcp add local"), "{text}");
    assert!(
        text.lines()
            .any(|l| l == "note codex: trust the new hook with /hooks"),
        "{text}"
    );

    assert!(home.path(".config/vibe/store.json").is_file());
    assert!(home.path(".config/vibe/skills").is_dir());
    let claude_md = home.read(".claude/CLAUDE.md");
    assert!(claude_md.contains("vibe:store"), "{claude_md}");
    assert!(claude_md.contains("@"), "{claude_md}");
    assert!(
        home.read(".claude/settings.json")
            .contains("hook session-start --harness claude")
    );
    assert!(
        home.read(".codex/AGENTS.md")
            .contains("Use the shared tools.")
    );
    assert!(home.read(".codex/config.toml").contains("vibe:store"));
    assert!(
        home.read(".codex/hooks.json")
            .contains("hook session-start --harness codex")
    );
    assert!(
        home.read(".factory/AGENTS.md")
            .contains("Use the shared tools.")
    );
    assert!(home.read(".factory/mcp.json").contains("\"local\""));
    assert!(
        home.read(".factory/hooks.json")
            .contains("hook session-start --harness droid")
    );
    let plugin = home.read(".config/opencode/plugins/vibe-store.js");
    assert_eq!(plugin.lines().next(), Some("// vibe:store-plugin v1"));
    for rel in [
        ".claude/skills/spec",
        ".claude/skills/a",
        ".agents/skills/vibe",
        ".agents/skills/using-spec",
        ".agents/skills/a",
    ] {
        assert!(is_link(&home, rel), "{rel}");
    }
    let calls = home.claude_calls();
    assert_eq!(calls.len(), 2, "{calls:?}");
    for call in &calls {
        assert_eq!(call[..3], ["mcp", "add-json", "--scope"]);
    }
    let st = state(&home);
    assert_eq!(st.path(&["store_hash"]).unwrap().as_str().unwrap(), "");
}

#[test]
fn install_wires_only_detected_harnesses_and_stamps() {
    let home = Home::with_store("install-detect");
    common::fake_claude(&home);
    fs::create_dir_all(home.path(".codex")).unwrap();
    let kit = toolkit(&home);
    let out = install(&home, &kit, &[]);
    assert_ok(&out);
    let text = out.text();
    for id in ["opencode", "droid"] {
        assert!(
            text.lines()
                .any(|l| l
                    == format!("note {id}: not detected \u{2014} pass --harness {id} to wire it")),
            "{text}"
        );
    }
    assert!(!text.contains("note claude: not detected"), "{text}");
    assert!(!text.contains("note codex: not detected"), "{text}");
    assert!(home.read(".claude/CLAUDE.md").contains("vibe:store"));
    assert!(home.read(".codex/AGENTS.md").contains("vibe:store"));
    assert!(!home.path(".factory").exists());
    assert!(!home.path(".config/opencode").exists());
    let st = state(&home);
    assert_ne!(st.path(&["store_hash"]).unwrap().as_str().unwrap(), "");
    let forced = install(&home, &kit, &["--harness", "droid"]);
    assert_ok(&forced);
    assert!(home.read(".factory/AGENTS.md").contains("vibe:store"));
    assert!(!forced.text().contains("not detected"), "{}", forced.text());
}

#[test]
fn install_creates_store_skeleton_and_projects_its_instructions() {
    let home = Home::new("install-skeleton");
    common::fake_claude(&home);
    let kit = toolkit(&home);
    let out = install(&home, &kit, &ALL);
    assert_ok(&out);
    for rel in ["store.json", "AGENTS.md", "mcp.json"] {
        assert!(home.path(&format!(".config/vibe/{rel}")).is_file(), "{rel}");
    }
    assert!(home.path(".config/vibe/skills").is_dir());
    assert!(
        home.read(".codex/AGENTS.md")
            .contains("vibe store: edit this file")
    );
    assert!(home.read(".claude/CLAUDE.md").contains("vibe:store"));
    let again = install(&home, &kit, &ALL);
    assert_ok(&again);
    assert!(again.text().contains("nothing to do"), "{}", again.text());
}

#[test]
fn install_second_run_changes_nothing() {
    let home = Home::with_store("install-twice");
    common::fake_claude(&home);
    let kit = toolkit(&home);
    assert_ok(&install(&home, &kit, &[]));
    mirror_claude_json(&home);
    clear_claude_log(&home);
    let before = snapshot(&home);
    let out = install(&home, &kit, &[]);
    assert_ok(&out);
    assert!(out.text().contains("nothing to do"), "{}", out.text());
    assert_eq!(snapshot(&home), before);
    assert!(home.claude_calls().is_empty());
}

#[test]
fn install_real_shape_preserves_everything() {
    let home = Home::real_shaped("install-real");
    common::fake_claude(&home);
    let kit = toolkit(&home);
    let before = snapshot(&home);
    let out = install(&home, &kit, &[]);
    assert_ok(&out);
    let after = snapshot(&home);
    let untouched = [
        ".claude/AGENTS.md",
        ".config/opencode/AGENTS.md",
        ".config/opencode/opencode.json",
        ".config/agent-stack/mcp.json",
        ".claude.json",
        ".claude/skills/hub-skill",
        ".claude/skills/foreign-link",
        ".claude/skills/own-skill/SKILL.md",
        ".claude/skills/other-real/SKILL.md",
        ".agents/skills/hub-skill/SKILL.md",
    ];
    for rel in untouched {
        assert_eq!(before.get(rel), after.get(rel), "{rel}");
    }
    assert!(is_link(&home, ".claude/AGENTS.md"));
    assert_eq!(
        home.read(".claude/AGENTS.md"),
        home.read(".config/opencode/AGENTS.md")
    );
    let claude = session_start_commands(&home, ".claude/settings.json");
    assert!(
        claude
            .iter()
            .any(|c| c == "/opt/thirdparty/bin/hook session-start")
    );
    assert_eq!(
        claude
            .iter()
            .filter(|c| c.contains("hook session-start --harness claude"))
            .count(),
        1
    );
    assert!(home.read(".claude/settings.json").contains("\"Stop\""));
    assert!(
        session_start_commands(&home, ".codex/hooks.json")
            .iter()
            .any(|c| c == "orca-hook session-start")
    );
    assert!(
        session_start_commands(&home, ".factory/hooks.json")
            .iter()
            .any(|c| c == "superset-hook notify")
    );
    assert!(home.read(".factory/mcp.json").contains("\"mesh\""));
    let toml = home.read(".codex/config.toml");
    assert!(toml.starts_with("model = \"gpt-5\"\n"), "{toml}");
    assert!(toml.contains("[mcp_servers.github]"), "{toml}");
    assert!(
        home.read(".codex/AGENTS.md")
            .starts_with("# Global rules\n\nA drifted copy of the rules.\n")
    );
    assert!(home.claude_calls().is_empty());
}

#[test]
fn install_dry_run_writes_nothing() {
    let home = Home::with_store("install-dry");
    common::fake_claude(&home);
    let kit = toolkit(&home);
    let before = snapshot(&home);
    let out = install(&home, &kit, &["--dry-run"]);
    assert_ok(&out);
    let text = out.text();
    assert!(
        text.lines().any(|l| l.starts_with("would write ")),
        "{text}"
    );
    assert!(text.lines().any(|l| l.starts_with("would link ")), "{text}");
    assert!(
        text.lines()
            .all(|l| !l.starts_with("wrote ") && !l.contains("would wrote")),
        "{text}"
    );
    assert_eq!(snapshot(&home), before);
    assert!(!home.path(".local/state/vibe").exists());
    assert!(!home.path(".claude").exists());
    assert!(home.claude_calls().is_empty());
}

#[test]
fn install_refused_target_does_not_block_others() {
    let home = Home::with_store("install-refused");
    common::fake_claude(&home);
    home.write(".factory/mcp.json", "{ nope");
    let kit = toolkit(&home);
    let out = install(&home, &kit, &ALL);
    assert_eq!(out.code, 1, "{}\n{}", out.text(), out.stderr);
    let text = out.text();
    assert!(
        text.lines()
            .any(|l| l.starts_with("refused ") && l.contains("mcp.json")),
        "{text}"
    );
    assert_eq!(home.read(".factory/mcp.json"), "{ nope");
    assert!(
        home.read(".codex/AGENTS.md")
            .contains("Use the shared tools.")
    );
    assert!(
        home.read(".claude/settings.json")
            .contains("hook session-start --harness claude")
    );
    assert!(
        home.path(".config/opencode/plugins/vibe-store.js")
            .is_file()
    );
}

#[test]
fn install_finds_toolkit_via_flag_and_env() {
    let home = Home::with_store("install-toolkit");
    common::fake_claude(&home);
    let kit = toolkit(&home);
    assert_ok(&install(&home, &kit, &["--harness", "codex"]));
    let sources = |home: &Home| -> Vec<String> {
        let doc = json::parse(&home.read(".config/vibe/store.json")).unwrap();
        doc.path(&["skills", "sources"])
            .unwrap()
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap().to_string())
            .collect()
    };
    assert_eq!(sources(&home), [kit.display().to_string()]);
    let again = home.vibe(
        &["install", ALL[0], ALL[1]],
        &[("VIBE_TOOLKIT", kit.to_str().unwrap())],
    );
    assert_ok(&again);
    assert_eq!(sources(&home), [kit.display().to_string()]);

    let other = Home::with_store("install-toolkit-env");
    common::fake_claude(&other);
    other.write_json(
        ".config/vibe/store.json",
        r#"{"v":1,"skills":{"sources":["/elsewhere"]}}"#,
    );
    let kit = toolkit(&other);
    let out = other.vibe(
        &["install", ALL[0], ALL[1]],
        &[("VIBE_TOOLKIT", kit.to_str().unwrap())],
    );
    assert_ok(&out);
    assert_eq!(
        sources(&other),
        ["/elsewhere".to_string(), kit.display().to_string()]
    );
    assert!(is_link(&other, ".agents/skills/spec"));
}

#[test]
fn install_toolkit_not_found_is_a_note_and_never_walks_ancestors() {
    let home = Home::with_store("install-no-toolkit");
    common::fake_claude(&home);
    let bad = non_toolkit(&home);
    let out = home.vibe(&["install"], &[("VIBE_TOOLKIT", bad.to_str().unwrap())]);
    assert_ok(&out);
    assert!(
        out.text()
            .lines()
            .any(|l| l == format!("note toolkit not found at {}", bad.display())),
        "{}",
        out.text()
    );
    let doc = json::parse(&home.read(".config/vibe/store.json")).unwrap();
    assert_eq!(
        doc.path(&["skills", "sources"])
            .unwrap()
            .as_array()
            .unwrap()
            .len(),
        0
    );
    assert!(!is_link(&home, ".agents/skills/spec"));
    let flag = home.vibe(&["install", "--toolkit", bad.to_str().unwrap()], &[]);
    assert_ok(&flag);
    assert!(
        flag.text().contains("note toolkit not found at"),
        "{}",
        flag.text()
    );
}

#[test]
fn install_no_literal_secrets_written() {
    let home = Home::with_store("install-secrets");
    common::fake_claude(&home);
    let kit = toolkit(&home);
    let args = [
        "install",
        "--toolkit",
        kit.to_str().unwrap(),
        ALL[0],
        ALL[1],
    ];
    let out = home.vibe(
        &args,
        &[
            ("VIBE_LOCAL_API_TOKEN", "SECRET-SENTINEL-123"),
            ("VIBE_REMOTE_AUTHORIZATION", "SECRET-SENTINEL-123"),
        ],
    );
    assert_ok(&out);
    assert!(!out.text().contains("SECRET-SENTINEL-123"));
    fn grep(dir: &std::path::Path, hits: &mut Vec<String>) {
        for e in fs::read_dir(dir).unwrap() {
            let path = e.unwrap().path();
            let meta = fs::symlink_metadata(&path).unwrap();
            if meta.is_dir() {
                grep(&path, hits);
            } else if meta.is_file()
                && String::from_utf8_lossy(&fs::read(&path).unwrap())
                    .contains("SECRET-SENTINEL-123")
            {
                hits.push(path.display().to_string());
            }
        }
    }
    let mut hits = Vec::new();
    grep(&home.dir, &mut hits);
    assert!(hits.is_empty(), "{hits:?}");
    assert!(
        home.read(".factory/mcp.json")
            .contains("${VIBE_LOCAL_API_TOKEN}")
    );
}

#[cfg(unix)]
#[test]
fn hook_binary_moved_install_replaces_entry() {
    let home = Home::with_store("install-moved");
    common::fake_claude(&home);
    let kit = toolkit(&home);
    assert_ok(&install(&home, &kit, &[]));
    mirror_claude_json(&home);
    let old = session_start_commands(&home, ".claude/settings.json");
    assert_eq!(old.len(), 1, "{old:?}");

    let moved_dir = home.path("moved");
    fs::create_dir_all(&moved_dir).unwrap();
    let moved = moved_dir.join("vibe");
    common::copy_exec(Path::new(env!("CARGO_BIN_EXE_vibe")), &moved);
    let mut cmd = Command::new(&moved);
    cmd.args([
        "install",
        "--toolkit",
        kit.to_str().unwrap(),
        ALL[0],
        ALL[1],
    ])
    .current_dir(&home.dir)
    .env_clear()
    .env(
        "PATH",
        format!("{}:/usr/bin:/bin", home.path("bin").display()),
    )
    .env("HOME", &home.dir)
    .env("LC_ALL", "C");
    let o = common::output(&mut cmd).unwrap();
    assert_eq!(
        o.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&o.stderr)
    );
    let moved_canon = fs::canonicalize(&moved).unwrap();
    for (rel, id) in [
        (".claude/settings.json", "claude"),
        (".codex/hooks.json", "codex"),
        (".factory/hooks.json", "droid"),
    ] {
        let cmds = session_start_commands(&home, rel);
        assert_eq!(cmds.len(), 1, "{rel}: {cmds:?}");
        assert_eq!(
            cmds[0],
            format!(
                "\"{}\" hook session-start --harness {id}",
                moved_canon.display()
            )
        );
    }
    let plugin = home.read(".config/opencode/plugins/vibe-store.js");
    assert!(
        plugin.contains(&moved_canon.display().to_string()),
        "{plugin}"
    );
    let st = state(&home);
    let hook = st.path(&["owned", "hooks", "claude"]).unwrap();
    assert!(
        matches!(hook, Value::String(s) if s.contains("/moved/vibe")),
        "{hook:?}"
    );
}

#[test]
fn install_without_claude_cli_skips_mcp_and_wires_the_rest() {
    let home = Home::with_store("install-noclaude");
    let kit = toolkit(&home);
    let out = install(&home, &kit, &ALL);
    assert_ok(&out);
    let text = out.text();
    assert!(
        text.lines()
            .any(|l| l == "skip claude: CLI not found \u{2014} MCP not projected"),
        "{text}"
    );
    assert!(home.read(".codex/config.toml").contains("vibe:store"));
    assert!(home.read(".factory/mcp.json").contains("\"local\""));
    assert!(home.read(".claude/CLAUDE.md").contains("vibe:store"));
    assert!(
        home.read(".claude/settings.json")
            .contains("hook session-start --harness claude")
    );
}

#[test]
fn install_without_claude_cli_leaves_claude_mcp_out_of_date() {
    let home = Home::with_store("install-noclaude-stale");
    for dir in [".claude", ".codex", ".factory", ".config/opencode"] {
        fs::create_dir_all(home.path(dir)).unwrap();
    }
    let kit = toolkit(&home);
    let out = install(&home, &kit, &[]);
    assert_ok(&out);
    assert!(
        out.text()
            .lines()
            .any(|l| l == "skip claude: CLI not found \u{2014} MCP not projected"),
        "{}",
        out.text()
    );
    let stale = "claude mcp: out of date \u{2014} run vibe sync";
    let status = home.vibe(&["store", "status"], &[]);
    assert!(
        status.text().lines().any(|l| l == stale),
        "{}",
        status.text()
    );

    common::fake_claude(&home);
    assert_ok(&home.vibe(&["sync"], &[]));
    assert!(
        home.claude_calls()
            .iter()
            .any(|c| c.get(1).is_some_and(|a| a == "add-json")),
        "{:?}",
        home.claude_calls()
    );
    let status = home.vibe(&["store", "status"], &[]);
    assert!(!status.text().contains("claude mcp:"), "{}", status.text());
}

#[test]
fn install_proceeds_with_symlinked_store_files() {
    let home = Home::new("install-symlinked-store");
    common::fake_claude(&home);
    home.write(
        "dotfiles/store.json",
        "{\"v\":1,\"skills\":{\"sources\":[]}}\n",
    );
    home.write("dotfiles/AGENTS.md", "# Linked rules\n");
    home.write("dotfiles/mcp.json", "{\"mcpServers\":{}}\n");
    fs::create_dir_all(home.path(".config/vibe")).unwrap();
    for f in ["store.json", "AGENTS.md", "mcp.json"] {
        std::os::unix::fs::symlink(
            home.path(&format!("dotfiles/{f}")),
            home.path(&format!(".config/vibe/{f}")),
        )
        .unwrap();
    }
    let kit = toolkit(&home);
    let out = install(&home, &kit, &["--harness", "claude,codex"]);
    assert_ok(&out);
    assert!(home.read(".codex/AGENTS.md").contains("# Linked rules"));
    assert!(
        out.text().contains("note toolkit not added: "),
        "{}",
        out.text()
    );
    assert_eq!(
        home.read("dotfiles/store.json"),
        "{\"v\":1,\"skills\":{\"sources\":[]}}\n"
    );
    for f in ["store.json", "AGENTS.md", "mcp.json"] {
        assert!(is_link(&home, &format!(".config/vibe/{f}")), "{f}");
    }
}

#[test]
fn install_notes_a_hook_binary_inside_a_build_directory() {
    let home = Home::with_store("install-build-note");
    common::fake_claude(&home);
    let kit = toolkit(&home);
    let out = install(&home, &kit, &["--harness", "codex"]);
    assert_ok(&out);
    let bin = env!("CARGO_BIN_EXE_vibe");
    assert!(bin.contains("/target/"), "{bin}");
    assert!(
        out.text().lines().any(|l| l.starts_with("note hook binary ")
            && l.ends_with(
                " is inside a build directory \u{2014} re-run vibe install after installing vibe on PATH"
            )),
        "{}",
        out.text()
    );
}

#[test]
fn hook_binary_prefers_the_path_spelling_of_the_running_binary() {
    use std::os::unix::fs::symlink;
    let home = Home::with_store("install-path-spelling");
    common::fake_claude(&home);
    let kit = toolkit(&home);
    let real_dir = home.path("pkg/real");
    fs::create_dir_all(&real_dir).unwrap();
    let real = real_dir.join("vibe");
    common::copy_exec(Path::new(env!("CARGO_BIN_EXE_vibe")), &real);
    let shim_dir = home.path("pkg/shim");
    fs::create_dir_all(&shim_dir).unwrap();
    symlink(&real, shim_dir.join("vibe")).unwrap();
    let run = |bin: &std::path::Path, path: String| {
        let mut cmd = Command::new(bin);
        cmd.args([
            "install",
            "--toolkit",
            kit.to_str().unwrap(),
            "--harness",
            "codex",
        ])
        .current_dir(&home.dir)
        .env_clear()
        .env("PATH", path)
        .env("HOME", &home.dir)
        .env("LC_ALL", "C");
        let o = common::output(&mut cmd).unwrap();
        assert_eq!(
            o.status.code(),
            Some(0),
            "{}",
            String::from_utf8_lossy(&o.stderr)
        );
        session_start_commands(&home, ".codex/hooks.json")
    };
    let on_path = run(
        &real,
        format!(
            "{}:{}:/usr/bin:/bin",
            home.path("bin").display(),
            shim_dir.display()
        ),
    );
    assert_eq!(
        on_path,
        [format!(
            "\"{}\" hook session-start --harness codex",
            shim_dir.join("vibe").display()
        )]
    );
    let off_path = run(&real, "/usr/bin:/bin".to_string());
    assert_eq!(
        off_path,
        [format!(
            "\"{}\" hook session-start --harness codex",
            fs::canonicalize(&real).unwrap().display()
        )]
    );
}
