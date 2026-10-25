#![allow(clippy::unwrap_used, clippy::expect_used)]
//! The OpenCode adapter: the plugin file, and the node test of the plugin itself.
mod common;
use common::Home;
use std::path::Path;
use std::process::Command;
use vibe::adapters::opencode::{self, PLUGIN_MARKER, plugin_bytes};
use vibe::adapters::{Ctx, Mode};
use vibe::exec::apply;
use vibe::paths::{Harness, Paths};
use vibe::plan::{Note, PlanItem, Projection};
use vibe::state::State;
use vibe::store;
use vibe_core::dirs::Env;

const BIN: &str = "/opt/vibe/bin/vibe";
const REL: &str = ".config/opencode/plugins/vibe-store.js";

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
        mode: Mode::Hook,
        harnesses: &[Harness::Opencode],
    };
    f(&ctx)
}

fn project_bin(home: &Home, state: &State, bin: &str) -> Projection {
    with(home, state, bin, |c| {
        opencode::project(c, Harness::Opencode)
    })
}

fn project(home: &Home, state: &State) -> Projection {
    project_bin(home, state, BIN)
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

fn writes(p: &Projection) -> usize {
    p.items
        .iter()
        .filter(|i| matches!(i, PlanItem::FileWrite { .. }))
        .count()
}

#[test]
fn opencode_plugin_written_with_marker_and_bin() {
    let home = Home::new("oc-write");
    let state = run(&home, project(&home, &State::default()), State::default());
    let text = home.read(REL);
    assert_eq!(text.lines().next(), Some(PLUGIN_MARKER));
    assert!(
        text.contains(&format!("const VIBE_BIN = \"{BIN}\";")),
        "{text}"
    );
    assert!(!text.contains("__VIBE_BIN__"));
    let path = home.path(REL).display().to_string();
    assert_eq!(state.owned.plugins.get("opencode"), Some(&path));
    assert!(state.owned.created.contains(&path));
}

#[test]
fn opencode_plugin_bin_is_json_escaped() {
    let text = plugin_bytes(Path::new("/odd \\ dir/v\u{e9}\"x")).unwrap();
    assert!(
        text.contains(r#"const VIBE_BIN = "/odd \\ dir/v\u00e9\"x";"#)
            || text.contains("const VIBE_BIN = \"/odd \\\\ dir/v\u{e9}\\\"x\";"),
        "{text}"
    );
}

#[test]
fn opencode_plugin_foreign_file_refused() {
    let home = Home::new("oc-foreign");
    home.write(REL, "// mine\nexport default {};\n");
    let p = project(&home, &State::default());
    assert!(p.items.is_empty() && p.edits.is_empty());
    assert_eq!(
        p.notes,
        vec![Note::Refused(format!(
            "opencode plugin: {} exists and is not vibe's \u{2014} not overwritten",
            home.path(REL).display()
        ))]
    );
    assert_eq!(home.read(REL), "// mine\nexport default {};\n");
}

#[test]
fn opencode_plugin_rewritten_when_template_differs() {
    let home = Home::new("oc-rewrite");
    home.write(REL, &format!("{PLUGIN_MARKER}\nold body\n"));
    let p = project(&home, &State::default());
    assert_eq!(writes(&p), 1);
    run(&home, p, State::default());
    assert_eq!(home.read(REL), plugin_bytes(Path::new(BIN)).unwrap());
}

#[test]
fn opencode_plugin_idempotent() {
    let home = Home::new("oc-idem");
    let state = run(&home, project(&home, &State::default()), State::default());
    let again = project(&home, &state);
    assert!(
        again.items.is_empty() && again.edits.is_empty(),
        "{again:?}"
    );
    let moved = project_bin(&home, &state, "/elsewhere/vibe");
    assert_eq!(writes(&moved), 1);
}

#[cfg(unix)]
#[test]
fn opencode_plugin_symlink_refused() {
    let home = Home::new("oc-link");
    home.write("real.js", "x");
    std::fs::create_dir_all(home.path(".config/opencode/plugins")).unwrap();
    std::os::unix::fs::symlink(home.path("real.js"), home.path(REL)).unwrap();
    let p = project(&home, &State::default());
    assert!(p.items.is_empty());
    assert_eq!(p.notes.len(), 1);
    assert_eq!(home.read("real.js"), "x");
}

#[test]
fn opencode_uninstall_removes_own_plugin_only() {
    let home = Home::new("oc-un");
    let state = run(&home, project(&home, &State::default()), State::default());
    let p = with(&home, &state, BIN, |c| {
        opencode::plan_uninstall(c, Harness::Opencode)
    });
    let state = run(&home, p, state.clone());
    assert!(!home.path(REL).exists());
    assert!(state.owned.plugins.is_empty());

    let foreign = Home::new("oc-un2");
    foreign.write(REL, "// mine\n");
    let mut st = State::default();
    st.owned
        .plugins
        .insert("opencode".into(), foreign.path(REL).display().to_string());
    let p = with(&foreign, &st, BIN, |c| {
        opencode::plan_uninstall(c, Harness::Opencode)
    });
    assert!(p.items.is_empty());
    assert_eq!(foreign.read(REL), "// mine\n");
}

#[test]
fn opencode_plugin_node_test() {
    if Command::new("node").arg("--version").output().is_err() {
        assert!(
            std::env::var("VIBE_REQUIRE_NODE").as_deref() != Ok("1"),
            "node is required (VIBE_REQUIRE_NODE=1) but not found"
        );
        eprintln!("skipping opencode_plugin_node_test: node not found on PATH");
        return;
    }
    let home = Home::new("oc-node");
    let bin = if cfg!(unix) {
        "/usr/bin/true"
    } else {
        "C:\\nonexistent\\vibe.exe"
    };
    let replaced = home.path("plugin/replaced.js");
    let raw = home.path("plugin/raw.js");
    home.write("plugin/replaced.js", &plugin_bytes(Path::new(bin)).unwrap());
    home.write("plugin/raw.js", opencode::PLUGIN_TEMPLATE);
    let test = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/opencode_plugin.test.mjs"
    );
    let out = Command::new("node")
        .arg("--test")
        .arg(test)
        .env("VIBE_PLUGIN", &replaced)
        .env("VIBE_PLUGIN_RAW", &raw)
        .env("VIBE_PLUGIN_BIN", bin)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}
