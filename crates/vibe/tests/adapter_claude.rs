#![allow(clippy::unwrap_used, clippy::expect_used)]
//! The Claude adapter (instructions block, hook, MCP via the CLI) and the skills links.
mod common;
use common::Home;
use std::path::{Path, PathBuf};
use vibe::adapters::{Ctx, Mode, claude, skills};
use vibe::exec::apply;
use vibe::hooks_json::hook_command;
use vibe::paths::{Harness, Paths};
use vibe::plan::{CliOp, Note, OwnedEdit, PlanItem, Projection};
use vibe::state::State;
use vibe::store;
use vibe_core::dirs::Env;
use vibe_core::json::{self, Value};

const BIN: &str = "/opt/vibe/bin/vibe";
const NO_CLAUDE: &str = "/nonexistent/vibe-test/claude";

fn paths(home: &Home) -> Paths {
    let vars = [("HOME".to_string(), home.dir.to_string_lossy().into_owned())];
    Paths::from_env(&Env::new(vars, false)).unwrap()
}

fn with<T>(home: &Home, state: &State, mode: Mode, f: impl FnOnce(&Ctx) -> T) -> T {
    let paths = paths(home);
    let store = store::load(&paths).unwrap();
    let ctx = Ctx {
        paths: &paths,
        store: &store,
        state,
        vibe_bin: Path::new(BIN),
        mode,
        harnesses: &[Harness::Claude],
    };
    f(&ctx)
}

fn project_in(home: &Home, state: &State, mode: Mode) -> Projection {
    with(home, state, mode, |c| claude::project(c, Harness::Claude))
}

fn project(home: &Home, state: &State) -> Projection {
    project_in(home, state, Mode::Sync)
}

fn install(home: &Home, state: &State) -> Projection {
    with(home, state, Mode::Sync, |c| {
        claude::plan_hook_install(c, Harness::Claude)
    })
}

fn uninstall(home: &Home, state: &State) -> Projection {
    with(home, state, Mode::Sync, |c| {
        claude::plan_uninstall(c, Harness::Claude)
    })
}

fn run(home: &Home, p: Projection, state: State) -> State {
    let (out, state) = apply(&paths(home), p, state, false, NO_CLAUDE);
    assert!(out.failed.is_empty(), "{:?}", out.failed);
    state
}

/// A store with root instructions and a skill but no MCP servers: no `claude` call is planned.
fn plain_store(name: &str) -> Home {
    let home = Home::with_store(name);
    home.write_json(".config/vibe/mcp.json", r#"{"mcpServers":{}}"#);
    home
}

fn refused(p: &Projection) -> Vec<&str> {
    p.notes
        .iter()
        .filter_map(|n| match n {
            Note::Refused(s) => Some(s.as_str()),
            _ => None,
        })
        .collect()
}

fn slashed(p: &Path) -> String {
    p.to_string_lossy().replace('\\', "/")
}

#[test]
fn claude_block_is_single_import_line() {
    let home = plain_store("cl-block");
    let state = run(&home, project(&home, &State::default()), State::default());
    let text = home.read(".claude/CLAUDE.md");
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 3, "{text}");
    assert!(lines[0].starts_with("<!-- vibe:store:begin v=1 hash="));
    assert_eq!(
        lines[1],
        format!("@{}", slashed(&home.path(".config/vibe/AGENTS.md")))
    );
    assert_eq!(lines[2], "<!-- vibe:store:end -->");
    assert!(state.targets.contains_key("claude:instructions"));
    let again = project(&home, &state);
    assert!(
        again.items.is_empty() && again.edits.is_empty(),
        "{again:?}"
    );
}

#[test]
fn claude_no_block_without_instructions_and_existing_block_kept() {
    let home = plain_store("cl-none");
    let state = run(&home, project(&home, &State::default()), State::default());
    let before = home.read(".claude/CLAUDE.md");
    home.write(".config/vibe/AGENTS.md", "   \n");
    let p = project(&home, &state);
    assert!(p.items.is_empty(), "{p:?}");
    assert_eq!(home.read(".claude/CLAUDE.md"), before);
    let empty = Home::new("cl-none2");
    assert!(project(&empty, &State::default()).items.is_empty());
}

#[cfg(unix)]
#[test]
fn claude_symlinked_claude_md_refused() {
    let home = Home::real_shaped("cl-symlink");
    home.write(".config/vibe/AGENTS.md", "# Rules\n");
    std::os::unix::fs::symlink(
        home.path(".config/opencode/AGENTS.md"),
        home.path(".claude/CLAUDE.md"),
    )
    .unwrap();
    let agents_before = std::fs::read(home.path(".claude/AGENTS.md")).unwrap();
    let target_before = std::fs::read_link(home.path(".claude/AGENTS.md")).unwrap();
    let oc_before = home.read(".config/opencode/AGENTS.md");
    let p = project(&home, &State::default());
    assert!(
        refused(&p).iter().any(|r| r.contains("is a symlink")),
        "{p:?}"
    );
    run(&home, p, State::default());
    assert_eq!(
        std::fs::read(home.path(".claude/AGENTS.md")).unwrap(),
        agents_before
    );
    assert_eq!(
        std::fs::read_link(home.path(".claude/AGENTS.md")).unwrap(),
        target_before
    );
    assert_eq!(home.read(".config/opencode/AGENTS.md"), oc_before);
}

#[test]
fn claude_md_user_text_survives() {
    let home = plain_store("cl-user");
    home.write(".claude/CLAUDE.md", "# Mine\nkeep me\n");
    run(&home, project(&home, &State::default()), State::default());
    let text = home.read(".claude/CLAUDE.md");
    assert!(text.starts_with("# Mine\nkeep me\n"), "{text}");
    assert!(text.contains("@"));
}

#[test]
fn claude_hook_added_once_and_third_party_group_survives() {
    let home = Home::real_shaped("cl-hook");
    let cmd = hook_command(Path::new(BIN), Harness::Claude).unwrap();
    let state = run(&home, install(&home, &State::default()), State::default());
    let text = home.read(".claude/settings.json");
    assert_eq!(text.matches(&cmd.replace('"', "\\\"")).count(), 1, "{text}");
    assert!(text.contains("/opt/thirdparty/bin/hook session-start"));
    assert!(text.contains("/opt/thirdparty/bin/hook stop"));
    assert!(text.contains("\"model\": \"opus\""));
    assert_eq!(state.owned.hooks.get("claude"), Some(&cmd));
    let again = install(&home, &state);
    assert!(
        again.items.is_empty() && again.edits.is_empty(),
        "{again:?}"
    );
}

#[test]
fn claude_hook_repairs_moved_binary() {
    let home = Home::new("cl-moved");
    let state = run(&home, install(&home, &State::default()), State::default());
    let paths = paths(&home);
    let store = store::load(&paths).unwrap();
    let ctx = Ctx {
        paths: &paths,
        store: &store,
        state: &state,
        vibe_bin: Path::new("/new/place/vibe"),
        mode: Mode::Sync,
        harnesses: &[Harness::Claude],
    };
    let p = claude::plan_hook_install(&ctx, Harness::Claude);
    let state = run(&home, p, state.clone());
    let text = home.read(".claude/settings.json");
    assert!(
        text.contains("/new/place/vibe") && !text.contains(BIN),
        "{text}"
    );
    assert!(state.owned.hooks["claude"].contains("/new/place/vibe"));
}

#[test]
fn claude_settings_invalid_json_refused() {
    let home = Home::new("cl-badjson");
    home.write(".claude/settings.json", "{nope");
    let p = install(&home, &State::default());
    assert!(p.items.is_empty());
    assert_eq!(refused(&p).len(), 1, "{p:?}");
    assert_eq!(home.read(".claude/settings.json"), "{nope");
}

#[test]
fn claude_settings_wrong_hooks_shape_refused() {
    let home = Home::new("cl-shape");
    home.write(".claude/settings.json", r#"{"hooks":[]}"#);
    let p = install(&home, &State::default());
    assert!(p.items.is_empty());
    assert_eq!(
        refused(&p),
        vec!["claude: settings.json hooks has an unexpected shape \u{2014} hook not written"]
    );
}

#[test]
fn claude_uninstall_removes_block_hook_and_created_files() {
    let home = plain_store("cl-uninstall");
    let mut p = project(&home, &State::default());
    p.merge(install(&home, &State::default()));
    let state = run(&home, p, State::default());
    assert!(home.path(".claude/CLAUDE.md").exists());
    let state = run(&home, uninstall(&home, &state), state.clone());
    assert!(!home.path(".claude/CLAUDE.md").exists());
    assert!(!home.path(".claude/settings.json").exists());
    assert!(state.owned.hooks.is_empty());
    assert!(!state.targets.contains_key("claude:instructions"));
}

fn set_mcp(home: &Home, servers: &str) {
    home.write_json(
        ".config/vibe/mcp.json",
        &format!(r#"{{"mcpServers":{servers}}}"#),
    );
}

fn set_claude_json(home: &Home, servers: &str) {
    home.write_json(
        ".claude.json",
        &format!(r#"{{"numStartups":3,"mcpServers":{servers}}}"#),
    );
}

fn val(s: &str) -> Value {
    json::parse(s).unwrap()
}

fn owned_state(name: &str, record: &str) -> State {
    let mut st = State::default();
    st.owned.mcp_claude = vec![(name.to_string(), val(record))];
    st
}

fn cli(p: &Projection) -> Vec<String> {
    p.items
        .iter()
        .filter_map(|i| match i {
            PlanItem::ClaudeCli { op } => Some(match op {
                CliOp::Add { name, json } => format!("add {name} {json}"),
                CliOp::Remove { name } => format!("remove {name}"),
            }),
            _ => None,
        })
        .collect()
}

fn notes(p: &Projection) -> Vec<&Note> {
    p.notes.iter().collect()
}

fn mcp_edit(p: &Projection, name: &str) -> Option<Option<Value>> {
    p.edits.iter().find_map(|(_, e)| match e {
        OwnedEdit::McpClaude { name: n, canonical } if n == name => Some(canonical.clone()),
        _ => None,
    })
}

const EXA: &str = r#"{"exa":{"command":"npx","args":["x"]}}"#;
const EXA_CANON: &str = r#"{"type":"stdio","command":"npx","args":["x"]}"#;

#[test]
fn claude_mcp_add_new() {
    let home = Home::new("cl-add");
    set_mcp(&home, EXA);
    set_claude_json(&home, "{}");
    let p = project(&home, &State::default());
    assert_eq!(cli(&p), vec![format!("add exa {EXA_CANON}")]);
    assert_eq!(mcp_edit(&p, "exa"), Some(Some(val(EXA_CANON))));
    assert!(
        p.items.iter().all(
            |i| !matches!(i, PlanItem::FileWrite { path, .. } if path.ends_with(".claude.json"))
        )
    );
}

#[test]
fn claude_mcp_unowned_same_name_kept() {
    let home = Home::new("cl-kept");
    set_mcp(&home, EXA);
    set_claude_json(&home, r#"{"exa":{"type":"http","url":"https://x"}}"#);
    let p = project(&home, &State::default());
    assert!(p.items.is_empty());
    assert!(notes(&p).contains(&&Note::Kept("claude's own exa".to_string())));
    assert_eq!(mcp_edit(&p, "exa"), None);
}

#[test]
fn claude_mcp_changed_store_remove_then_add() {
    let home = Home::new("cl-changed");
    set_mcp(&home, r#"{"exa":{"command":"npx","args":["y"]}}"#);
    set_claude_json(&home, &format!(r#"{{"exa":{EXA_CANON}}}"#));
    let p = project(&home, &owned_state("exa", EXA_CANON));
    let new = r#"{"type":"stdio","command":"npx","args":["y"]}"#;
    assert_eq!(
        cli(&p),
        vec!["remove exa".to_string(), format!("add exa {new}")]
    );
    assert_eq!(mcp_edit(&p, "exa"), Some(Some(val(new))));
}

#[test]
fn claude_mcp_edited_outside_kept() {
    let home = Home::new("cl-edited");
    set_mcp(&home, r#"{"exa":{"command":"npx","args":["y"]}}"#);
    set_claude_json(&home, r#"{"exa":{"type":"stdio","command":"hacked"}}"#);
    let p = project(&home, &owned_state("exa", EXA_CANON));
    assert!(p.items.is_empty());
    assert!(notes(&p).contains(&&Note::Kept(
        "claude: exa edited outside vibe \u{2014} kept".to_string()
    )));
    assert_eq!(mcp_edit(&p, "exa"), None);
}

#[test]
fn claude_mcp_absent_owned_readded() {
    let home = Home::new("cl-absent");
    set_mcp(&home, EXA);
    set_claude_json(&home, "{}");
    let p = project(&home, &owned_state("exa", EXA_CANON));
    assert_eq!(cli(&p), vec![format!("add exa {EXA_CANON}")]);
    assert!(notes(&p).contains(&&Note::Info("claude: re-added exa".to_string())));
}

#[test]
fn claude_mcp_dropped_removed_when_equal() {
    let home = Home::new("cl-drop");
    set_mcp(&home, "{}");
    set_claude_json(&home, &format!(r#"{{"exa":{EXA_CANON}}}"#));
    let p = project(&home, &owned_state("exa", EXA_CANON));
    assert_eq!(cli(&p), vec!["remove exa"]);
    assert_eq!(mcp_edit(&p, "exa"), Some(None));
}

#[test]
fn claude_mcp_dropped_kept_when_edited() {
    let home = Home::new("cl-drop-edit");
    set_mcp(&home, "{}");
    set_claude_json(&home, r#"{"exa":{"type":"stdio","command":"hacked"}}"#);
    let p = project(&home, &owned_state("exa", EXA_CANON));
    assert!(p.items.is_empty());
    assert!(notes(&p).contains(&&Note::Kept(
        "claude: exa edited outside vibe \u{2014} kept".to_string()
    )));
    assert_eq!(mcp_edit(&p, "exa"), Some(None));
}

#[test]
fn claude_mcp_equal_ignores_key_order() {
    let home = Home::new("cl-order");
    set_mcp(&home, EXA);
    set_claude_json(
        &home,
        r#"{"exa":{"args":["x"],"command":"npx","type":"stdio"}}"#,
    );
    let p = project(&home, &owned_state("exa", EXA_CANON));
    assert!(p.items.is_empty() && p.edits.is_empty(), "{p:?}");
}

#[test]
fn claude_mcp_hook_mode_emits_no_cli_items() {
    let home = Home::new("cl-hookmode");
    set_mcp(&home, EXA);
    set_claude_json(&home, "{}");
    let p = project_in(&home, &State::default(), Mode::Hook);
    assert!(
        p.items.is_empty() && p.edits.is_empty() && p.notes.is_empty(),
        "{p:?}"
    );
}

#[test]
fn claude_mcp_missing_claude_json_is_empty() {
    let home = Home::new("cl-nojson");
    set_mcp(&home, EXA);
    let p = project(&home, &State::default());
    assert_eq!(cli(&p), vec![format!("add exa {EXA_CANON}")]);
    assert!(
        p.notes
            .iter()
            .any(|n| matches!(n, Note::Info(s) if s.contains(".claude.json")))
    );
}

#[test]
fn claude_mcp_invalid_claude_json_is_empty_with_info() {
    let home = Home::new("cl-badclaudejson");
    set_mcp(&home, EXA);
    home.write(".claude.json", "{broken");
    let p = project(&home, &State::default());
    assert_eq!(cli(&p), vec![format!("add exa {EXA_CANON}")]);
    assert!(
        p.notes
            .iter()
            .any(|n| matches!(n, Note::Info(s) if s.contains(".claude.json")))
    );
    assert_eq!(home.read(".claude.json"), "{broken");
}

#[test]
fn claude_mcp_bad_name_skipped() {
    let home = Home::new("cl-badname");
    set_mcp(&home, r#"{"bad name":{"command":"x"}}"#);
    set_claude_json(&home, "{}");
    let p = project(&home, &State::default());
    assert!(p.items.is_empty());
    assert!(notes(&p).contains(&&Note::Skipped(
        "claude: bad name (name not accepted by claude)".to_string()
    )));
}

#[test]
fn claude_mcp_disabled_not_added() {
    let home = Home::new("cl-disabled");
    set_mcp(&home, r#"{"off":{"command":"x","enabled":false}}"#);
    set_claude_json(&home, "{}");
    let p = project(&home, &State::default());
    assert!(p.items.is_empty());
    assert!(notes(&p).contains(&&Note::Skipped("claude: off (disabled)".to_string())));
}

#[test]
fn claude_mcp_placeholders_pass_through() {
    let home = Home::new("cl-ph");
    set_mcp(
        &home,
        r#"{"exa":{"command":"npx","args":["--key","${EXA_KEY}"],"env":{"K":"${EXA_KEY}"}}}"#,
    );
    set_claude_json(&home, "{}");
    let p = project(&home, &State::default());
    let calls = cli(&p);
    assert_eq!(calls.len(), 1);
    assert!(calls[0].contains("${EXA_KEY}"), "{calls:?}");
}

#[test]
fn claude_plan_debug_never_prints_cli_json() {
    let home = Home::new("cl-debug");
    set_mcp(
        &home,
        r#"{"exa":{"command":"npx","env":{"K":"sk-secret-value"}}}"#,
    );
    set_claude_json(&home, "{}");
    let p = project(&home, &State::default());
    assert!(!format!("{p:?}").contains("sk-secret-value"));
}

#[test]
fn claude_mcp_not_applicable_harness_filter_not_added() {
    let home = Home::new("cl-filter");
    set_mcp(&home, r#"{"x":{"command":"c","harnesses":["codex"]}}"#);
    set_claude_json(&home, "{}");
    let p = project(&home, &State::default());
    assert!(p.items.is_empty() && p.notes.is_empty(), "{p:?}");
}

#[test]
fn claude_uninstall_removes_owned_mcp_when_equal_and_keeps_edited() {
    let home = Home::new("cl-un-mcp");
    set_claude_json(
        &home,
        &format!(r#"{{"exa":{EXA_CANON},"two":{{"type":"stdio","command":"edited"}}}}"#),
    );
    let mut state = owned_state("exa", EXA_CANON);
    state.owned.mcp_claude.push((
        "two".to_string(),
        val(r#"{"type":"stdio","command":"orig"}"#),
    ));
    let p = uninstall(&home, &state);
    assert_eq!(cli(&p), vec!["remove exa"]);
    assert_eq!(mcp_edit(&p, "exa"), Some(None));
    assert_eq!(mcp_edit(&p, "two"), Some(None));
    assert!(notes(&p).contains(&&Note::Kept(
        "claude: two edited outside vibe \u{2014} kept".to_string()
    )));
}

#[cfg(unix)]
#[test]
fn claude_mcp_roundtrip_with_fake_cli() {
    let home = Home::new("cl-fake");
    let bin = common::fake_claude(&home);
    set_mcp(&home, EXA);
    set_claude_json(&home, "{}");
    let p = project(&home, &State::default());
    let claude_bin = bin.join("claude");
    let (out, state) = apply(
        &paths(&home),
        p,
        State::default(),
        false,
        &claude_bin.to_string_lossy(),
    );
    assert!(out.failed.is_empty(), "{:?}", out.failed);
    let calls = home.claude_calls();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0][..4], ["mcp", "add-json", "--scope", "user"]);
    assert_eq!(calls[0][4], "exa");
    assert_eq!(state.owned.mcp_claude.len(), 1);
}

fn skill(home: &Home, name: &str) {
    home.write(
        &format!(".config/vibe/skills/{name}/SKILL.md"),
        &format!("---\nname: {name}\ndescription: d.\n---\n"),
    );
}

fn skills_plan(home: &Home, state: &State, dirs: &[PathBuf]) -> Projection {
    with(home, state, Mode::Sync, |c| skills::project(c, dirs))
}

fn both_dirs(home: &Home) -> Vec<PathBuf> {
    vec![home.path(".agents/skills"), home.path(".claude/skills")]
}

fn creates(p: &Projection) -> usize {
    p.items
        .iter()
        .filter(|i| matches!(i, PlanItem::LinkCreate { .. }))
        .count()
}

#[cfg(unix)]
#[test]
fn skills_link_created_for_both_dirs() {
    let home = Home::new("sk-both");
    skill(&home, "a");
    skill(&home, "b");
    let dirs = both_dirs(&home);
    let p = skills_plan(&home, &State::default(), &dirs);
    assert_eq!(creates(&p), 4);
    let state = run(&home, p, State::default());
    for d in [".agents/skills", ".claude/skills"] {
        for n in ["a", "b"] {
            assert_eq!(
                std::fs::read_link(home.path(&format!("{d}/{n}"))).unwrap(),
                home.path(&format!(".config/vibe/skills/{n}"))
            );
        }
    }
    assert_eq!(state.owned.links.len(), 4);
    assert!(
        state
            .owned
            .created
            .contains(&home.path(".agents/skills").display().to_string())
    );
    assert!(
        state
            .owned
            .created
            .contains(&home.path(".agents").display().to_string())
    );
}

#[cfg(unix)]
#[test]
fn skills_real_dir_collision_skipped() {
    let home = Home::real_shaped("sk-collide");
    for n in ["own-skill", "foreign-link", "good"] {
        skill(&home, n);
    }
    let dirs = vec![home.path(".claude/skills")];
    let foreign_before = std::fs::read_link(home.path(".claude/skills/foreign-link")).unwrap();
    let own_before = home.read(".claude/skills/own-skill/SKILL.md");
    let p = skills_plan(&home, &State::default(), &dirs);
    let skipped: Vec<&Note> = p
        .notes
        .iter()
        .filter(|n| matches!(n, Note::Skipped(s) if s.contains("exists, not a vibe link")))
        .collect();
    assert_eq!(skipped.len(), 2, "{p:?}");
    assert!(skipped.contains(&&Note::Skipped(format!(
        "skills: {} exists, not a vibe link",
        home.path(".claude/skills/own-skill").display()
    ))));
    assert_eq!(creates(&p), 1);
    run(&home, p, State::default());
    assert_eq!(
        std::fs::read_link(home.path(".claude/skills/foreign-link")).unwrap(),
        foreign_before
    );
    assert_eq!(home.read(".claude/skills/own-skill/SKILL.md"), own_before);
    assert!(
        std::fs::symlink_metadata(home.path(".claude/skills/good"))
            .unwrap()
            .file_type()
            .is_symlink()
    );
}

#[cfg(unix)]
#[test]
fn skills_idempotent_second_run_empty() {
    let home = Home::new("sk-idem");
    skill(&home, "a");
    let dirs = both_dirs(&home);
    let state = run(
        &home,
        skills_plan(&home, &State::default(), &dirs),
        State::default(),
    );
    let again = skills_plan(&home, &state, &dirs);
    assert!(
        again.items.is_empty() && again.edits.is_empty(),
        "{again:?}"
    );
}

#[cfg(unix)]
#[test]
fn skills_stale_owned_link_removed() {
    let home = Home::new("sk-stale");
    skill(&home, "a");
    skill(&home, "b");
    let dirs = both_dirs(&home);
    let state = run(
        &home,
        skills_plan(&home, &State::default(), &dirs),
        State::default(),
    );
    std::fs::remove_dir_all(home.path(".config/vibe/skills/b")).unwrap();
    let p = skills_plan(&home, &state, &dirs);
    assert_eq!(
        p.items
            .iter()
            .filter(|i| matches!(i, PlanItem::LinkRemove { .. }))
            .count(),
        2
    );
    let state = run(&home, p, state.clone());
    assert!(std::fs::symlink_metadata(home.path(".claude/skills/b")).is_err());
    assert!(std::fs::symlink_metadata(home.path(".agents/skills/a")).is_ok());
    assert_eq!(state.owned.links.len(), 2);
}

#[cfg(unix)]
#[test]
fn skills_owned_link_replaced_by_user_is_released_once() {
    let home = Home::new("sk-replaced");
    skill(&home, "a");
    skill(&home, "b");
    let dirs = vec![home.path(".claude/skills")];
    let state = run(
        &home,
        skills_plan(&home, &State::default(), &dirs),
        State::default(),
    );
    std::fs::remove_file(home.path(".claude/skills/a")).unwrap();
    home.write(".claude/skills/a/SKILL.md", "mine");
    std::fs::remove_file(home.path(".claude/skills/b")).unwrap();
    std::os::unix::fs::symlink("/elsewhere", home.path(".claude/skills/b")).unwrap();
    let p = skills_plan(&home, &state, &dirs);
    let kept: Vec<&Note> = p
        .notes
        .iter()
        .filter(|n| matches!(n, Note::Kept(s) if s.contains("replaced by user \u{2014} no longer managed")))
        .collect();
    assert_eq!(kept.len(), 2, "{p:?}");
    assert!(p.notes.iter().all(|n| !matches!(n, Note::Skipped(_))));
    assert!(p.items.is_empty());
    let state = run(&home, p, state);
    assert!(state.owned.links.is_empty());
    let again = skills_plan(&home, &state, &dirs);
    assert!(again.notes.iter().all(|n| !matches!(n, Note::Kept(_))));
    assert_eq!(home.read(".claude/skills/a/SKILL.md"), "mine");
}

#[cfg(unix)]
#[test]
fn skills_owned_link_to_moved_source_is_retargeted() {
    let home = Home::new("sk-move");
    skill(&home, "a");
    let dirs = vec![home.path(".claude/skills")];
    let state = run(
        &home,
        skills_plan(&home, &State::default(), &dirs),
        State::default(),
    );
    home.write("moved/a/SKILL.md", "---\nname: a\ndescription: d.\n---\n");
    std::fs::remove_dir_all(home.path(".config/vibe/skills/a")).unwrap();
    home.write_json(
        ".config/vibe/store.json",
        &format!(
            r#"{{"v":1,"skills":{{"sources":[{:?}]}}}}"#,
            home.path("moved").to_string_lossy()
        ),
    );
    let p = skills_plan(&home, &state, &dirs);
    assert!(
        p.notes.iter().all(|n| !matches!(n, Note::Skipped(_))),
        "{p:?}"
    );
    run(&home, p, state);
    assert_eq!(
        std::fs::read_link(home.path(".claude/skills/a")).unwrap(),
        home.path("moved/a")
    );
}

#[test]
fn skills_shadowed_reported() {
    let home = Home::new("sk-shadow");
    skill(&home, "a");
    home.write("extra/a/SKILL.md", "---\nname: a\ndescription: dup.\n---\n");
    home.write_json(
        ".config/vibe/store.json",
        &format!(
            r#"{{"v":1,"skills":{{"sources":[{:?}]}}}}"#,
            home.path("extra").to_string_lossy()
        ),
    );
    let p = skills_plan(&home, &State::default(), &[home.path(".claude/skills")]);
    let kept = home.path(".config/vibe/skills/a");
    let dropped = home.path("extra/a");
    assert!(p.notes.contains(&Note::Info(format!(
        "skill a: kept {}, shadowed {}",
        kept.display(),
        dropped.display()
    ))));
}

#[cfg(not(unix))]
#[test]
fn skills_unix_only_failure_message_once() {
    let home = Home::new("sk-win");
    skill(&home, "a");
    skill(&home, "b");
    let p = skills_plan(&home, &State::default(), &both_dirs(&home));
    let lines: Vec<&Note> = p
        .notes
        .iter()
        .filter(|n| matches!(n, Note::Info(s) if s == "skills: links unsupported on this OS \u{2014} skipped"))
        .collect();
    assert_eq!(lines.len(), 1, "{p:?}");
    assert!(p.items.is_empty());
}
