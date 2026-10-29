#![allow(clippy::unwrap_used, clippy::expect_used)]
#![cfg(unix)]
//! The shapes `vibe` emits (`state.json`, `sync --json`) against the pinned contract.
mod cmd_support;
mod common;
use std::fs;

use cmd_support::{install, toolkit};
use common::Home;
use vibe::paths::Paths;
use vibe_core::dirs::Env;
use vibe_core::json::{self, Value};

fn is_hex16(s: &str) -> bool {
    s.len() == 16 && s.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

fn keys(v: &Value) -> Vec<String> {
    v.as_object()
        .unwrap()
        .iter()
        .map(|(k, _)| k.to_string())
        .collect()
}

#[test]
fn installed_state_matches_contract() {
    let home = Home::real_shaped("contract-state");
    common::fake_claude(&home);
    let kit = toolkit(&home);
    let out = install(&home, &kit, &[]);
    assert_eq!(out.code, 0, "{}\n{}", out.text(), out.stderr);

    let state = json::parse(&home.read(".local/state/vibe/state.json")).unwrap();
    assert!(matches!(state.path(&["v"]), Some(Value::Number(n)) if n.as_str() == "1"));
    let store_hash = state.path(&["store_hash"]).unwrap().as_str().unwrap();
    assert!(is_hex16(store_hash), "store_hash {store_hash:?}");
    if let Some(mcp_hash) = state.path(&["mcp_hash"]) {
        let mcp_hash = mcp_hash.as_str().unwrap();
        assert!(
            mcp_hash.is_empty() || is_hex16(mcp_hash),
            "mcp_hash {mcp_hash:?}"
        );
    }

    let targets = state.path(&["targets"]).unwrap();
    let target_keys = keys(targets);
    assert!(!target_keys.is_empty());
    for key in target_keys {
        let hash = targets
            .path(&[key.as_str(), "hash"])
            .and_then(Value::as_str)
            .unwrap_or_else(|| panic!("targets.{key}.hash missing"));
        assert!(is_hex16(hash), "targets.{key}.hash {hash:?}");
    }

    let owned = state.path(&["owned"]).unwrap();
    let allowed = ["mcp", "hooks", "plugins", "links", "created"];
    for key in keys(owned) {
        assert!(allowed.contains(&key.as_str()), "unexpected owned.{key}");
    }
    let claude_hook = owned
        .path(&["hooks", "claude"])
        .and_then(Value::as_str)
        .expect("owned.hooks.claude");
    assert!(claude_hook.starts_with('"'), "{claude_hook}");
    assert!(
        claude_hook.contains("hook session-start --harness claude"),
        "{claude_hook}"
    );
    for link in owned.path(&["links"]).unwrap().as_array().unwrap() {
        assert!(link.path(&["path"]).and_then(Value::as_str).is_some());
        assert!(link.path(&["target"]).and_then(Value::as_str).is_some());
    }
}

#[test]
fn contract_state_fixture_loads() {
    let home = Home::new("contract-fixture");
    let vars = [("HOME".to_string(), home.dir.to_string_lossy().into_owned())];
    let paths = Paths::from_env(&Env::new(vars, false)).unwrap();
    let fixture = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../contracts/fixtures/vibe-store/state/valid/full.json"
    );
    fs::create_dir_all(paths.state_file().parent().unwrap()).unwrap();
    fs::copy(fixture, paths.state_file()).unwrap();
    let state = vibe::state::load(&paths).unwrap();
    assert_eq!(state.store_hash, "0123456789abcdef");
    assert_eq!(state.targets.len(), 2);
    assert_eq!(state.owned.links.len(), 1);
}

#[test]
fn sync_json_has_exactly_the_contract_keys() {
    let home = cmd_support::ready_home("contract-sync");
    let out = home.vibe(&["sync", "--json"], &[]);
    assert_eq!(out.code, 0, "{}\n{}", out.text(), out.stderr);
    let doc = json::parse(&out.text()).unwrap();
    let mut got = keys(&doc);
    got.sort();
    assert_eq!(got, ["changes", "dry_run", "failed", "notes", "v"]);
}
