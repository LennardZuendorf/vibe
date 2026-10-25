//! Fixtures shared by the command-level integration tests (`cmd_*.rs`).
#![cfg(unix)]
#![allow(dead_code, clippy::unwrap_used, clippy::expect_used)]
use std::fs;
use std::path::PathBuf;

use super::common::{Home, fake_claude};
use vibe_core::json::{self, Value};

/// A home with a valid store, one configuration directory per harness and a fake `claude`.
pub fn ready_home(name: &str) -> Home {
    let home = Home::with_store(name);
    for dir in [".claude", ".codex", ".factory", ".config/opencode"] {
        fs::create_dir_all(home.path(dir)).unwrap();
    }
    fake_claude(&home);
    home
}

/// A ready home after one successful `vibe sync`, with the `claude` log cleared.
pub fn synced_home(name: &str) -> Home {
    let home = ready_home(name);
    let out = home.vibe(&["sync"], &[]);
    assert_eq!(out.code, 0, "{}\n{}", out.text(), out.stderr);
    mirror_claude_json(&home);
    clear_claude_log(&home);
    home
}

/// Empties the fake `claude` argv log.
pub fn clear_claude_log(home: &Home) {
    let _ = fs::remove_file(home.path("claude-argv.log"));
}

/// The state file of the home, parsed.
pub fn state(home: &Home) -> Value {
    json::parse(&home.read(".local/state/vibe/state.json")).unwrap()
}

/// The state file path.
pub fn state_path(home: &Home) -> PathBuf {
    home.path(".local/state/vibe/state.json")
}

/// Writes `.claude.json` the way a real `claude` would after `sync`: the owned servers.
pub fn mirror_claude_json(home: &Home) {
    let st = state(home);
    let owned = st.path(&["owned", "mcp", "claude"]).unwrap().clone();
    let mut doc = json::Object::new();
    doc.insert("mcpServers", owned);
    home.write(".claude.json", &json::to_pretty(&Value::Object(doc)));
}

/// Replaces `from` with `to` in `rel` under the home.
pub fn replace_in(home: &Home, rel: &str, from: &str, to: &str) {
    let text = home.read(rel);
    assert!(text.contains(from), "{rel} lacks {from:?}:\n{text}");
    home.write(rel, &text.replace(from, to));
}

/// One file-system entry as `snapshot` records it.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Entry {
    /// Bytes and modification time.
    File(Vec<u8>, std::time::SystemTime),
    /// Link target.
    Link(PathBuf),
    /// A directory.
    Dir,
}

/// Every entry under the home (the fake `claude` log and the fake binary aside), keyed by
/// the path relative to the home.
pub fn snapshot(home: &Home) -> std::collections::BTreeMap<String, Entry> {
    fn walk(
        root: &std::path::Path,
        dir: &std::path::Path,
        out: &mut std::collections::BTreeMap<String, Entry>,
    ) {
        let mut names: Vec<_> = fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().path())
            .collect();
        names.sort();
        for path in names {
            let rel = path.strip_prefix(root).unwrap().display().to_string();
            if rel == "claude-argv.log" || rel == "bin" {
                continue;
            }
            let meta = fs::symlink_metadata(&path).unwrap();
            if meta.file_type().is_symlink() {
                out.insert(rel, Entry::Link(fs::read_link(&path).unwrap()));
            } else if meta.is_dir() {
                out.insert(rel, Entry::Dir);
                walk(root, &path, out);
            } else {
                out.insert(
                    rel,
                    Entry::File(fs::read(&path).unwrap(), meta.modified().unwrap()),
                );
            }
        }
    }
    let mut out = std::collections::BTreeMap::new();
    walk(&home.dir, &home.dir, &mut out);
    out
}

/// A valid toolkit (`spec`, `vibe`, `using-spec`) at `<home>/kit`.
pub fn toolkit(home: &Home) -> PathBuf {
    for name in ["spec", "vibe", "using-spec"] {
        home.write(
            &format!("kit/{name}/SKILL.md"),
            &format!("---\nname: {name}\ndescription: Toolkit {name}.\n---\n"),
        );
    }
    home.path("kit")
}

/// A directory that is not a toolkit.
pub fn non_toolkit(home: &Home) -> PathBuf {
    let dir = home.path("not-a-kit");
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// Runs `vibe install --toolkit <kit>` plus `extra` arguments.
pub fn install(home: &Home, kit: &std::path::Path, extra: &[&str]) -> super::common::Out {
    let kit = kit.to_str().unwrap();
    let mut args = vec!["install", "--toolkit", kit];
    args.extend_from_slice(extra);
    home.vibe(&args, &[])
}
