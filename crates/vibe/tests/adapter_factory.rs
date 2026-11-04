#![allow(clippy::unwrap_used, clippy::expect_used)]
//! The Factory droid adapter: instructions block, `mcp.json` owned keys, `hooks.json` entry.
mod common;
use common::Home;
use std::path::Path;
use vibe::adapters::{Ctx, Mode, factory};
use vibe::exec::apply;
use vibe::hooks_json::hook_command;
use vibe::paths::{Harness, Paths};
use vibe::plan::{Note, Projection};
use vibe::state::State;
use vibe::store;
use vibe_core::dirs::Env;
use vibe_core::json::{self, Value};

const BIN: &str = "/opt/vibe/bin/vibe";
const RULES: &str = "# Global rules\n\nUse the shared tools.\n";

fn paths(home: &Home) -> Paths {
    let vars = [("HOME".to_string(), home.dir.to_string_lossy().into_owned())];
    Paths::from_env(&Env::new(vars, false)).unwrap()
}

fn with<T>(home: &Home, state: &State, f: impl FnOnce(&Ctx) -> T) -> T {
    let paths = paths(home);
    let store = store::load(&paths).unwrap();
    let ctx = Ctx {
        paths: &paths,
        store: &store,
        state,
        vibe_bin: Path::new(BIN),
        mode: Mode::Sync,
        harnesses: &[Harness::Droid],
    };
    f(&ctx)
}

fn project(home: &Home, state: &State) -> Projection {
    with(home, state, |c| factory::project(c, Harness::Droid))
}

fn install(home: &Home, state: &State) -> Projection {
    with(home, state, |c| {
        factory::plan_hook_install(c, Harness::Droid)
    })
}

fn uninstall(home: &Home, state: &State) -> Projection {
    with(home, state, |c| factory::plan_uninstall(c, Harness::Droid))
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

fn servers(home: &Home) -> Vec<String> {
    let doc = json::parse(&home.read(".factory/mcp.json")).unwrap();
    let names: Vec<String> = doc
        .path(&["mcpServers"])
        .unwrap()
        .as_object()
        .unwrap()
        .keys()
        .map(str::to_string)
        .collect();
    names
}

fn owned(state: &State) -> Vec<String> {
    state
        .owned
        .mcp_names
        .get("droid")
        .cloned()
        .unwrap_or_default()
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

const TWO: &str = r#"{"mcpServers":{
  "local":{"type":"stdio","command":"npx","args":["-y","some-server"],"env":{"API_TOKEN":"${VIBE_LOCAL_API_TOKEN}"}},
  "remote":{"type":"http","url":"https://mcp.example.com/mcp","headers":{"Authorization":"${VIBE_REMOTE_AUTHORIZATION}"}},
  "codex-only":{"command":"c","harnesses":["codex"]}}}"#;

#[test]
fn factory_mcp_merge_keeps_mesh_and_adds_owned() {
    let home = Home::real_shaped("fac-merge");
    set_store(&home, None, TWO);
    let state = run(&home, project(&home, &State::default()), State::default());
    assert_eq!(servers(&home), vec!["mesh", "local", "remote"]);
    assert_eq!(owned(&state), vec!["local", "remote"]);
    assert!(state.targets.contains_key("droid:mcp"));
    let text = home.read(".factory/mcp.json");
    assert!(text.contains("${VIBE_LOCAL_API_TOKEN}"));
    let again = project(&home, &state);
    assert!(
        again.items.is_empty() && again.edits.is_empty(),
        "{again:?}"
    );
}

#[test]
fn factory_key_order_preserved() {
    let home = Home::real_shaped("fac-order");
    home.write_json(
        ".factory/mcp.json",
        r#"{"theme":"x","mcpServers":{"zeta":{"command":"z"},"mesh":{"type":"stdio","command":"mesh-mcp","args":["serve"]},"alpha":{"command":"a"}},"tail":1}"#,
    );
    let before = home.read(".factory/mcp.json");
    set_store(&home, None, TWO);
    run(&home, project(&home, &State::default()), State::default());
    let after = home.read(".factory/mcp.json");
    let unchanged =
        &before[before.find("\"theme\"").unwrap()..before.find("\n  },\n  \"tail\"").unwrap()];
    assert!(after.contains(unchanged), "{after}");
    assert!(after.find("\"alpha\"").unwrap() < after.find("\"local\"").unwrap());
    assert!(after.find("\"theme\"").unwrap() < after.find("\"tail\"").unwrap());
    assert!(after.find("\"local\"").unwrap() < after.find("\"tail\"").unwrap());
}

#[test]
fn factory_same_name_unowned_is_kept() {
    let home = Home::real_shaped("fac-kept");
    set_store(
        &home,
        None,
        r#"{"mcpServers":{"mesh":{"command":"other"},"local":{"command":"npx"}}}"#,
    );
    let p = project(&home, &State::default());
    assert!(
        p.notes
            .contains(&Note::Kept("droid's own mesh".to_string()))
    );
    let state = run(&home, p, State::default());
    let doc = json::parse(&home.read(".factory/mcp.json")).unwrap();
    assert_eq!(
        doc.path(&["mcpServers", "mesh", "command"])
            .and_then(Value::as_str),
        Some("mesh-mcp")
    );
    assert_eq!(owned(&state), vec!["local"]);
}

#[test]
fn factory_owned_key_removed_when_store_drops_it() {
    let home = Home::real_shaped("fac-drop");
    set_store(&home, None, TWO);
    let state = run(&home, project(&home, &State::default()), State::default());
    set_store(
        &home,
        None,
        r#"{"mcpServers":{"remote":{"type":"http","url":"https://mcp.example.com/mcp"}}}"#,
    );
    let state = run(&home, project(&home, &state), state);
    assert_eq!(servers(&home), vec!["mesh", "remote"]);
    assert_eq!(owned(&state), vec!["remote"]);
    set_store(&home, None, r#"{"mcpServers":{}}"#);
    let state = run(&home, project(&home, &state), state);
    assert_eq!(servers(&home), vec!["mesh"]);
    assert!(owned(&state).is_empty());
    assert!(!state.targets.contains_key("droid:mcp"));
}

#[test]
fn factory_untranslatable_previously_owned_is_deleted_and_reported() {
    let home = Home::real_shaped("fac-untrans");
    set_store(&home, None, r#"{"mcpServers":{"x":{"command":"c"}}}"#);
    let state = run(&home, project(&home, &State::default()), State::default());
    set_store(
        &home,
        None,
        r#"{"mcpServers":{"x":{"command":"c${VIBE_X_Y}"}}}"#,
    );
    let p = project(&home, &state);
    assert!(p.notes.contains(&Note::Skipped(
        "droid: x (placeholder in command)".to_string()
    )));
    run(&home, p, state);
    assert_eq!(servers(&home), vec!["mesh"]);
}

#[test]
fn factory_hand_edited_owned_key_is_overwritten() {
    let home = Home::real_shaped("fac-edit");
    set_store(&home, None, r#"{"mcpServers":{"x":{"command":"c"}}}"#);
    let state = run(&home, project(&home, &State::default()), State::default());
    let edited = home
        .read(".factory/mcp.json")
        .replace("\"c\"", "\"hacked\"");
    home.write(".factory/mcp.json", &edited);
    run(&home, project(&home, &state), state);
    assert!(
        home.read(".factory/mcp.json")
            .contains("\"command\": \"c\"")
    );
}

#[test]
fn factory_disabled_server_gets_disabled_true() {
    let home = Home::new("fac-disabled");
    set_store(
        &home,
        None,
        r#"{"mcpServers":{"off":{"command":"o","enabled":false}}}"#,
    );
    run(&home, project(&home, &State::default()), State::default());
    let doc = json::parse(&home.read(".factory/mcp.json")).unwrap();
    assert_eq!(
        doc.path(&["mcpServers", "off", "disabled"])
            .and_then(Value::as_bool),
        Some(true)
    );
}

#[test]
fn factory_invalid_json_refused_other_targets_unaffected() {
    let home = Home::new("fac-badjson");
    set_store(&home, Some(RULES), TWO);
    home.write(".factory/mcp.json", "{not json");
    let p = project(&home, &State::default());
    assert_eq!(refused(&p).len(), 1, "{:?}", p.notes);
    assert_eq!(p.items.len(), 1);
    assert!(
        p.edits
            .iter()
            .all(|(_, e)| !matches!(e, vibe::plan::OwnedEdit::McpNames { .. }))
    );
    run(&home, p, State::default());
    assert_eq!(home.read(".factory/mcp.json"), "{not json");
    assert!(
        home.read(".factory/AGENTS.md")
            .contains("Use the shared tools.")
    );
}

#[test]
fn factory_wrong_typed_mcp_servers_refused() {
    let home = Home::new("fac-shape");
    set_store(&home, None, TWO);
    home.write(".factory/mcp.json", "{\"mcpServers\":[]}\n");
    let p = project(&home, &State::default());
    assert!(p.items.is_empty());
    assert_eq!(refused(&p).len(), 1, "{:?}", p.notes);
}

#[test]
fn factory_missing_file_created_with_only_mcp_servers() {
    let home = Home::new("fac-create");
    set_store(&home, None, r#"{"mcpServers":{"local":{"command":"npx"}}}"#);
    let state = run(&home, project(&home, &State::default()), State::default());
    assert_eq!(
        home.read(".factory/mcp.json"),
        "{\n  \"mcpServers\": {\n    \"local\": {\n      \"type\": \"stdio\",\n      \"command\": \"npx\"\n    }\n  }\n}\n"
    );
    assert!(state.owned.created.iter().any(|p| p.ends_with("mcp.json")));
}

#[test]
fn factory_instructions_block_when_agents_file_absent() {
    let home = Home::real_shaped("fac-instr");
    set_store(&home, Some(RULES), r#"{"mcpServers":{}}"#);
    assert!(!home.path(".factory/AGENTS.md").exists());
    let state = run(&home, project(&home, &State::default()), State::default());
    let text = home.read(".factory/AGENTS.md");
    assert!(text.starts_with("<!-- vibe:store:begin v=1 hash="));
    assert!(text.ends_with("<!-- vibe:store:end -->\n"));
    assert!(text.contains("Use the shared tools."));
    assert!(state.targets.contains_key("droid:instructions"));
    assert!(project(&home, &state).items.is_empty());
}

fn session_start(home: &Home) -> Vec<String> {
    let doc = json::parse(&home.read(".factory/hooks.json")).unwrap();
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
fn factory_hooks_merge_keeps_superset_group() {
    let home = Home::real_shaped("fac-hooks");
    let p = install(&home, &State::default());
    assert!(
        !p.notes
            .iter()
            .any(|n| matches!(n, Note::Info(s) if s.contains("/hooks")))
    );
    let state = run(&home, p, State::default());
    let cmd = hook_command(Path::new(BIN), Harness::Droid).unwrap();
    assert_eq!(
        session_start(&home),
        vec!["superset-hook notify".to_string(), cmd.clone()]
    );
    assert_eq!(state.owned.hooks.get("droid"), Some(&cmd));
    let again = install(&home, &state);
    assert!(again.items.is_empty() && again.edits.is_empty());
}

#[test]
fn factory_hooks_wrong_shape_refused_file_untouched() {
    let home = Home::new("fac-hookshape");
    let text = "{\"hooks\":[]}\n";
    home.write(".factory/hooks.json", text);
    let p = install(&home, &State::default());
    assert!(p.items.is_empty() && p.edits.is_empty());
    assert_eq!(
        refused(&p),
        vec!["droid: hooks.json hooks has an unexpected shape \u{2014} hook not written"]
    );
    assert_eq!(home.read(".factory/hooks.json"), text);
}

#[test]
fn factory_uninstall_removes_only_vibe_content() {
    let home = Home::real_shaped("fac-uninstall");
    set_store(&home, Some(RULES), TWO);
    let mut state = run(&home, project(&home, &State::default()), State::default());
    state = run(&home, install(&home, &state), state);
    let state = run(&home, uninstall(&home, &state), state);
    assert_eq!(servers(&home), vec!["mesh"]);
    assert_eq!(
        session_start(&home),
        vec!["superset-hook notify".to_string()]
    );
    assert!(!home.path(".factory/AGENTS.md").exists());
    assert!(state.owned.hooks.is_empty() && owned(&state).is_empty());
    assert!(state.targets.is_empty());
    assert!(uninstall(&home, &state).items.is_empty());
}

#[test]
fn factory_uninstall_deletes_files_vibe_created() {
    let home = Home::new("fac-uninstall-created");
    set_store(&home, Some(RULES), TWO);
    let mut state = run(&home, project(&home, &State::default()), State::default());
    state = run(&home, install(&home, &state), state);
    run(&home, uninstall(&home, &state), state);
    for f in ["AGENTS.md", "mcp.json", "hooks.json"] {
        assert!(!home.path(&format!(".factory/{f}")).exists(), "{f}");
    }
}

#[test]
fn factory_missing_instructions_leave_existing_block_untouched_uninstall_removes() {
    let home = Home::real_shaped("fac-instr-none");
    set_store(&home, Some(RULES), r#"{"mcpServers":{}}"#);
    let state = run(&home, project(&home, &State::default()), State::default());
    let before = home.read(".factory/AGENTS.md");
    std::fs::remove_file(home.path(".config/vibe/AGENTS.md")).unwrap();
    let p = project(&home, &state);
    assert!(p.items.is_empty() && p.edits.is_empty(), "{p:?}");
    assert_eq!(home.read(".factory/AGENTS.md"), before);
    run(&home, uninstall(&home, &state), state);
    assert!(!home.path(".factory/AGENTS.md").exists());
}

#[test]
fn factory_sync_dropping_all_keys_leaves_empty_object_uninstall_removes_file() {
    let home = Home::new("fac-empty-created");
    set_store(&home, None, r#"{"mcpServers":{"local":{"command":"npx"}}}"#);
    let state = run(&home, project(&home, &State::default()), State::default());
    set_store(&home, None, r#"{"mcpServers":{}}"#);
    let state = run(&home, project(&home, &state), state);
    assert_eq!(
        home.read(".factory/mcp.json"),
        "{\n  \"mcpServers\": {}\n}\n"
    );
    assert!(owned(&state).is_empty());
    run(&home, uninstall(&home, &state), state);
    assert!(!home.path(".factory/mcp.json").exists());
}
