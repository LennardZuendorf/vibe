#![allow(clippy::unwrap_used, clippy::expect_used)]
//! The Codex adapter: instructions block, `config.toml` MCP block, `hooks.json` entry.
mod common;
use common::Home;
use std::fs;
use std::path::Path;
use vibe::adapters::{Ctx, Mode, codex};
use vibe::exec::apply;
use vibe::hooks_json::hook_command;
use vibe::paths::{Harness, Paths};
use vibe::plan::{Note, PlanItem, Projection};
use vibe::state::State;
use vibe::store;
use vibe::translate::toml::scan_mcp_tables;
use vibe_core::dirs::Env;
use vibe_core::json::{self, Value};
use vibe_core::markers::{self, Style};

const BIN: &str = "/opt/vibe/bin/vibe";

fn paths(home: &Home) -> Paths {
    let vars = [("HOME".to_string(), home.dir.to_string_lossy().into_owned())];
    Paths::from_env(&Env::new(vars, false)).unwrap()
}

fn with<T>(home: &Home, state: &State, bin: &str, f: impl FnOnce(&Ctx) -> T) -> T {
    let paths = paths(home);
    let store = store::load(&paths).unwrap();
    let ctx = Ctx {
        paths: &paths,
        store: &store,
        state,
        vibe_bin: Path::new(bin),
        mode: Mode::Sync,
        harnesses: &[Harness::Codex],
    };
    f(&ctx)
}

fn project(home: &Home, state: &State) -> Projection {
    with(home, state, BIN, |c| codex::project(c, Harness::Codex))
}

fn install(home: &Home, state: &State, bin: &str) -> Projection {
    with(home, state, bin, |c| {
        codex::plan_hook_install(c, Harness::Codex)
    })
}

fn uninstall(home: &Home, state: &State) -> Projection {
    with(home, state, BIN, |c| {
        codex::plan_uninstall(c, Harness::Codex)
    })
}

fn run(home: &Home, p: Projection, state: State) -> State {
    let (out, state) = apply(
        &paths(home),
        p,
        state,
        false,
        "/nonexistent/vibe-test/claude",
    );
    assert!(out.failed.is_empty(), "{:?}", out.failed);
    state
}

fn set_store(home: &Home, agents: Option<&str>, mcp: &str) {
    if let Some(a) = agents {
        home.write(".config/vibe/AGENTS.md", a);
    }
    home.write_json(".config/vibe/mcp.json", mcp);
}

const LOCAL: &str = r#"{"mcpServers":{"local":{"command":"npx","args":["-y","srv"]}}}"#;
const NONE: &str = r#"{"mcpServers":{}}"#;
const RULES: &str = "# Global rules\n\nUse the shared tools.\n";

fn refused(p: &Projection) -> Vec<&str> {
    p.notes
        .iter()
        .filter_map(|n| match n {
            Note::Refused(s) => Some(s.as_str()),
            _ => None,
        })
        .collect()
}

fn outside_block(text: &str) -> String {
    let b = markers::find(text, Style::Hash, "store").unwrap().unwrap();
    format!("{}{}", &text[..b.range.start], &text[b.range.end..])
}

#[test]
fn codex_instructions_block_created_and_idempotent() {
    let home = Home::new("codex-instr");
    set_store(&home, Some(RULES), NONE);
    let p = project(&home, &State::default());
    assert_eq!(p.items.len(), 1);
    let state = run(&home, p, State::default());
    let text = home.read(".codex/AGENTS.md");
    assert!(text.starts_with("<!-- vibe:store:begin v=1 hash="));
    assert!(text.contains("Use the shared tools."));
    assert!(state.targets.contains_key("codex:instructions"));
    let again = project(&home, &state);
    assert!(again.items.is_empty(), "{:?}", again.items);
    assert!(again.edits.is_empty());
}

#[test]
fn codex_instructions_keep_user_text_outside_block() {
    let home = Home::real_shaped("codex-keep");
    set_store(&home, Some(RULES), NONE);
    let state = run(&home, project(&home, &State::default()), State::default());
    let drifted = "# Global rules\n\nA drifted copy of the rules.\n";
    assert!(home.read(".codex/AGENTS.md").starts_with(drifted));
    let text = format!("USER TOP\n{}USER BOTTOM\n", home.read(".codex/AGENTS.md"));
    home.write(".codex/AGENTS.md", &text);
    set_store(&home, Some("# New rules\n"), NONE);
    run(&home, project(&home, &state), state.clone());
    let now = home.read(".codex/AGENTS.md");
    assert!(now.starts_with("USER TOP\n# Global rules\n\nA drifted copy"));
    assert!(now.ends_with("USER BOTTOM\n"));
    assert!(now.contains("# New rules"));
    assert!(!now.contains("Use the shared tools."));
}

#[test]
fn codex_hand_edit_inside_block_is_reported_and_replaced() {
    let home = Home::new("codex-hand");
    set_store(&home, Some(RULES), NONE);
    let state = run(&home, project(&home, &State::default()), State::default());
    let edited = home
        .read(".codex/AGENTS.md")
        .replace("shared tools", "hacked");
    home.write(".codex/AGENTS.md", &edited);
    let p = project(&home, &state);
    assert!(
        p.notes
            .iter()
            .any(|n| matches!(n, Note::Info(s) if s.contains("edited inside the vibe block")))
    );
    run(&home, p, state);
    assert!(home.read(".codex/AGENTS.md").contains("shared tools"));
}

#[test]
fn codex_no_instructions_no_block() {
    let home = Home::new("codex-none");
    set_store(&home, None, NONE);
    let p = project(&home, &State::default());
    assert!(p.items.is_empty());
    assert!(!home.path(".codex/AGENTS.md").exists());
}

#[test]
fn codex_override_note() {
    let home = Home::new("codex-override");
    set_store(&home, Some(RULES), NONE);
    home.write(".codex/AGENTS.override.md", "x\n");
    let p = project(&home, &State::default());
    let want = Note::Info("codex: AGENTS.override.md exists and shadows AGENTS.md".to_string());
    assert!(p.notes.contains(&want), "{:?}", p.notes);
}

#[test]
fn codex_32kib_note() {
    let home = Home::new("codex-big");
    let big = format!("{}\n", "x".repeat(40000));
    set_store(&home, Some(&big), NONE);
    let p = project(&home, &State::default());
    let want = Note::Info("codex: instructions exceed 32 KiB (40001 bytes)".to_string());
    assert!(p.notes.contains(&want), "{:?}", p.notes);
    assert_eq!(p.items.len(), 1);
    let small = Home::new("codex-small");
    set_store(&small, Some(RULES), NONE);
    let p = project(&small, &State::default());
    assert!(
        !p.notes
            .iter()
            .any(|n| matches!(n, Note::Info(s) if s.contains("KiB")))
    );
}

#[test]
fn codex_reversed_markers_refuse_and_file_untouched() {
    let home = Home::new("codex-reversed");
    set_store(&home, Some(RULES), LOCAL);
    let md = "<!-- vibe:store:end -->\nmine\n<!-- vibe:store:begin v=1 hash=0000000000000000 -->\n";
    let toml = "# vibe:store:end\nx = 1\n# vibe:store:begin v=1 hash=0000000000000000\n";
    home.write(".codex/AGENTS.md", md);
    home.write(".codex/config.toml", toml);
    let p = project(&home, &State::default());
    assert!(p.items.is_empty());
    assert_eq!(refused(&p).len(), 2, "{:?}", p.notes);
    assert!(refused(&p).iter().all(|r| r.contains("comes before")));
    run(&home, p, State::default());
    assert_eq!(home.read(".codex/AGENTS.md"), md);
    assert_eq!(home.read(".codex/config.toml"), toml);
}

#[test]
fn codex_toml_block_appended_at_end_after_user_tables() {
    let home = Home::real_shaped("codex-append");
    set_store(&home, None, LOCAL);
    let original = home.read(".codex/config.toml");
    let state = run(&home, project(&home, &State::default()), State::default());
    let text = home.read(".codex/config.toml");
    assert!(text.starts_with(&original));
    assert!(text.ends_with("# vibe:store:end\n"));
    assert!(text.contains("[mcp_servers.local]\ncommand = \"npx\"\nargs = [\"-y\", \"srv\"]\n"));
    assert!(state.targets.contains_key("codex:mcp"));
    assert!(project(&home, &state).items.is_empty());
}

#[test]
fn codex_duplicate_table_is_skipped_not_duplicated() {
    let home = Home::real_shaped("codex-dup");
    set_store(
        &home,
        None,
        r#"{"mcpServers":{"github":{"command":"other"},"local":{"command":"npx"}}}"#,
    );
    let p = project(&home, &State::default());
    assert!(
        p.notes
            .contains(&Note::Kept("codex's own github".to_string()))
    );
    run(&home, p, State::default());
    let text = home.read(".codex/config.toml");
    let outside = outside_block(&text);
    assert_eq!(
        scan_mcp_tables(&outside)
            .names
            .iter()
            .filter(|n| *n == "github")
            .count(),
        1
    );
    assert_eq!(text.matches("[mcp_servers.github]").count(), 1);
    assert!(text.contains("[mcp_servers.local]"));
}

#[test]
fn codex_content_after_block_refuses() {
    let home = Home::real_shaped("codex-after");
    set_store(&home, None, LOCAL);
    let state = run(&home, project(&home, &State::default()), State::default());
    let base = home.read(".codex/config.toml");
    set_store(
        &home,
        None,
        r#"{"mcpServers":{"local":{"command":"npx"},"two":{"command":"b"}}}"#,
    );

    home.write(".codex/config.toml", &format!("{base}\nkey = 1\n"));
    let p = project(&home, &state);
    assert!(p.items.is_empty());
    assert!(
        refused(&p).contains(&"config.toml has content after the vibe block \u{2014} fix by hand")
    );

    home.write(
        ".codex/config.toml",
        &format!("{base}\n# note\n\n[other]\nx = 1\n"),
    );
    let p = project(&home, &state);
    assert!(refused(&p).is_empty(), "{:?}", p.notes);
    assert_eq!(p.items.len(), 1);
    run(&home, p, state);
    let text = home.read(".codex/config.toml");
    assert!(text.contains("[mcp_servers.two]"));
    assert!(text.ends_with("[other]\nx = 1\n"));
}

#[test]
fn codex_translation_skips_reported() {
    let home = Home::new("codex-skip");
    set_store(
        &home,
        None,
        r#"{"mcpServers":{"bad":{"type":"http","url":"https://x/${VIBE_X_Y}/mcp"},"ok":{"command":"c"}}}"#,
    );
    let p = project(&home, &State::default());
    assert!(p.notes.contains(&Note::Skipped(
        "codex: bad (placeholder in url)".to_string()
    )));
    run(&home, p, State::default());
    let text = home.read(".codex/config.toml");
    assert!(text.contains("[mcp_servers.ok]") && !text.contains("bad"));
}

#[cfg(unix)]
#[test]
fn codex_symlinked_config_refused() {
    let home = Home::new("codex-symlink");
    set_store(&home, None, LOCAL);
    home.write("elsewhere.toml", "a = 1\n");
    fs::create_dir_all(home.path(".codex")).unwrap();
    std::os::unix::fs::symlink(home.path("elsewhere.toml"), home.path(".codex/config.toml"))
        .unwrap();
    let p = project(&home, &State::default());
    assert!(p.items.is_empty());
    assert!(refused(&p).iter().any(|r| r.contains("symlink")));
    assert_eq!(home.read("elsewhere.toml"), "a = 1\n");
}

#[test]
fn codex_invalid_utf8_refused() {
    let home = Home::new("codex-utf8");
    set_store(&home, None, LOCAL);
    fs::create_dir_all(home.path(".codex")).unwrap();
    fs::write(home.path(".codex/config.toml"), [0xff, 0xfe, b'a']).unwrap();
    let p = project(&home, &State::default());
    assert!(p.items.is_empty());
    assert!(refused(&p).iter().any(|r| r.contains("not valid UTF-8")));
}

#[test]
fn codex_root_inline_mcp_servers_refuses_block() {
    let home = Home::new("codex-inline");
    set_store(&home, Some(RULES), LOCAL);
    home.write(
        ".codex/config.toml",
        "mcp_servers = { a = { command = \"x\" } }\n",
    );
    let p = project(&home, &State::default());
    assert!(refused(&p).contains(
        &"codex: config.toml defines mcp_servers inline \u{2014} vibe MCP block not written"
    ));
    assert_eq!(p.items.len(), 1, "instructions still project");
    assert!(matches!(&p.items[0], PlanItem::FileWrite { path, .. } if path.ends_with("AGENTS.md")));
    run(&home, p, State::default());
    assert_eq!(
        home.read(".codex/config.toml"),
        "mcp_servers = { a = { command = \"x\" } }\n"
    );
}

#[test]
fn codex_block_removed_when_no_servers_remain() {
    let home = Home::real_shaped("codex-drop");
    set_store(&home, None, LOCAL);
    let original = home.read(".codex/config.toml");
    let state = run(&home, project(&home, &State::default()), State::default());
    set_store(&home, None, NONE);
    let state = run(&home, project(&home, &state), state);
    assert_eq!(home.read(".codex/config.toml"), original);
    assert!(!state.targets.contains_key("codex:mcp"));
}

fn hook_cmd(bin: &str) -> String {
    hook_command(Path::new(bin), Harness::Codex).unwrap()
}

fn session_start(home: &Home) -> Vec<String> {
    let doc = json::parse(&home.read(".codex/hooks.json")).unwrap();
    doc.path(&["hooks", "SessionStart"])
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|g| g.path(&["hooks"]).unwrap().as_array().unwrap().clone())
        .map(|h| {
            h.path(&["command"])
                .and_then(Value::as_str)
                .unwrap()
                .to_string()
        })
        .collect()
}

#[test]
fn codex_hooks_json_keeps_orca_group_and_adds_one_entry() {
    let home = Home::real_shaped("codex-hooks");
    let state = run(
        &home,
        install(&home, &State::default(), BIN),
        State::default(),
    );
    assert_eq!(
        session_start(&home),
        vec!["orca-hook session-start".to_string(), hook_cmd(BIN)]
    );
    assert_eq!(state.owned.hooks.get("codex"), Some(&hook_cmd(BIN)));
}

#[test]
fn codex_hook_second_run_no_item() {
    let home = Home::real_shaped("codex-hook2");
    let state = run(
        &home,
        install(&home, &State::default(), BIN),
        State::default(),
    );
    let p = install(&home, &state, BIN);
    assert!(
        p.items.is_empty() && p.edits.is_empty() && p.notes.is_empty(),
        "{p:?}"
    );
}

#[test]
fn hook_binary_moved_install_replaces_entry() {
    let home = Home::real_shaped("codex-moved");
    let state = run(
        &home,
        install(&home, &State::default(), BIN),
        State::default(),
    );
    let moved = "/new/place/vibe";
    let state = run(&home, install(&home, &state, moved), state);
    assert_eq!(
        session_start(&home),
        vec!["orca-hook session-start".to_string(), hook_cmd(moved)]
    );
    assert_eq!(state.owned.hooks.get("codex"), Some(&hook_cmd(moved)));
}

#[test]
fn codex_trust_notice() {
    let home = Home::real_shaped("codex-trust");
    set_store(&home, Some(RULES), LOCAL);
    let p = install(&home, &State::default(), BIN);
    let trust: Vec<_> = p
        .notes
        .iter()
        .filter(|n| matches!(n, Note::Info(s) if s.contains("/hooks")))
        .collect();
    assert_eq!(
        trust,
        vec![&Note::Info(
            "codex: trust the new hook with /hooks".to_string()
        )]
    );
    let q = project(&home, &State::default());
    assert!(
        !q.notes
            .iter()
            .any(|n| matches!(n, Note::Info(s) if s.contains("/hooks")))
    );
}

#[test]
fn codex_hooks_wrong_shape_refused_file_untouched() {
    let home = Home::new("codex-shape");
    let text = "{\"hooks\":{\"SessionStart\":{}}}\n";
    home.write(".codex/hooks.json", text);
    let p = install(&home, &State::default(), BIN);
    assert!(p.items.is_empty() && p.edits.is_empty());
    assert_eq!(
        refused(&p),
        vec!["codex: hooks.json hooks has an unexpected shape \u{2014} hook not written"]
    );
    assert_eq!(home.read(".codex/hooks.json"), text);
}

#[test]
fn codex_hooks_invalid_json_refused() {
    let home = Home::new("codex-badjson");
    home.write(".codex/hooks.json", "{nope");
    let p = install(&home, &State::default(), BIN);
    assert!(p.items.is_empty() && p.edits.is_empty());
    assert_eq!(refused(&p).len(), 1);
}

#[test]
fn codex_uninstall_removes_only_vibe_content() {
    let home = Home::real_shaped("codex-uninstall");
    set_store(&home, Some(RULES), LOCAL);
    let config = home.read(".codex/config.toml");
    let mut state = run(&home, project(&home, &State::default()), State::default());
    state = run(&home, install(&home, &state, BIN), state);
    let state = run(&home, uninstall(&home, &state), state);
    assert_eq!(home.read(".codex/config.toml"), config);
    assert_eq!(
        session_start(&home),
        vec!["orca-hook session-start".to_string()]
    );
    let agents = home.read(".codex/AGENTS.md");
    assert_eq!(agents, "# Global rules\n\nA drifted copy of the rules.\n");
    assert!(state.owned.hooks.is_empty());
    assert!(state.targets.is_empty());
    assert!(uninstall(&home, &state).items.is_empty());
}

#[test]
fn codex_uninstall_deletes_files_vibe_created() {
    let home = Home::new("codex-uninstall-created");
    set_store(&home, Some(RULES), LOCAL);
    let mut state = run(&home, project(&home, &State::default()), State::default());
    state = run(&home, install(&home, &state, BIN), state);
    assert!(home.path(".codex/hooks.json").exists());
    run(&home, uninstall(&home, &state), state);
    for f in ["AGENTS.md", "config.toml", "hooks.json"] {
        assert!(!home.path(&format!(".codex/{f}")).exists(), "{f}");
    }
}

#[test]
fn codex_root_inline_without_servers_is_silent() {
    let home = Home::new("codex-inline-quiet");
    set_store(&home, None, NONE);
    home.write(
        ".codex/config.toml",
        "mcp_servers = { a = { command = \"x\" } }\n",
    );
    let p = project(&home, &State::default());
    assert!(p.items.is_empty() && p.notes.is_empty(), "{p:?}");
}

#[test]
fn codex_root_inline_removes_existing_vibe_block() {
    let home = Home::new("codex-inline-remove");
    set_store(&home, None, LOCAL);
    let state = run(&home, project(&home, &State::default()), State::default());
    let inline = "mcp_servers = { a = { command = \"x\" } }\n";
    let text = format!("{inline}{}", home.read(".codex/config.toml"));
    home.write(".codex/config.toml", &text);
    let p = project(&home, &state);
    assert!(refused(&p).is_empty());
    assert!(p.notes.contains(&Note::Info(
        "codex: config.toml defines mcp_servers inline \u{2014} vibe MCP block removed".to_string()
    )));
    run(&home, p, state);
    assert_eq!(home.read(".codex/config.toml"), inline.to_string() + "");
}

#[test]
fn codex_owned_name_collides_later() {
    for after in [false, true] {
        let home = Home::new(&format!("codex-collide-{after}"));
        set_store(&home, None, LOCAL);
        let state = run(&home, project(&home, &State::default()), State::default());
        let base = home.read(".codex/config.toml");
        let user = "[mcp_servers.local]\ncommand = \"mine\"\n";
        let text = if after {
            format!("{base}\n{user}")
        } else {
            format!("{user}\n{base}")
        };
        home.write(".codex/config.toml", &text);
        set_store(
            &home,
            None,
            r#"{"mcpServers":{"local":{"command":"npx"},"two":{"command":"b"}}}"#,
        );
        let p = project(&home, &state);
        assert!(
            p.notes
                .contains(&Note::Kept("codex's own local".to_string()))
        );
        run(&home, p, state);
        let text = home.read(".codex/config.toml");
        assert_eq!(text.matches("[mcp_servers.local]").count(), 1, "{text}");
        assert!(text.contains("[mcp_servers.two]"));
        let outside = outside_block(&text);
        assert!(scan_mcp_tables(&outside).names.contains("local"));
    }
}

#[test]
fn codex_missing_instructions_leave_existing_block_untouched_uninstall_removes() {
    let home = Home::new("codex-instr-none");
    set_store(&home, Some(RULES), NONE);
    let state = run(&home, project(&home, &State::default()), State::default());
    let before = home.read(".codex/AGENTS.md");
    fs::remove_file(home.path(".config/vibe/AGENTS.md")).unwrap();
    let p = project(&home, &state);
    assert!(p.items.is_empty() && p.edits.is_empty(), "{p:?}");
    assert_eq!(home.read(".codex/AGENTS.md"), before);
    run(&home, uninstall(&home, &state), state);
    assert!(!home.path(".codex/AGENTS.md").exists());
}
