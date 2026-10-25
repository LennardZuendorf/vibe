//! `vibe store status`: store validity and per-harness drift.
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use vibe_core::json::{self, Object, Value};
use vibe_core::markers::{self, Style};

use super::common;
use crate::adapters::{Ctx, Mode, enabled_harnesses, factory, opencode, project};
use crate::hooks_json::has_hook;
use crate::paths::{Harness, Paths};
use crate::plan::Note;
use crate::state::{self, State};
use crate::store::{self, Store};

struct Row {
    mcp: &'static str,
    enabled: bool,
    block: &'static str,
    hook: &'static str,
    drift: &'static str,
    skips: usize,
    conflicts: usize,
}

fn instruction_file(paths: &Paths, h: Harness) -> Option<(PathBuf, &'static str)> {
    match h {
        Harness::Claude => Some((paths.claude_dir.join("CLAUDE.md"), "claude:instructions")),
        Harness::Codex => Some((paths.codex_dir.join("AGENTS.md"), "codex:instructions")),
        Harness::Droid => Some((paths.factory_dir.join("AGENTS.md"), "droid:instructions")),
        Harness::Opencode => None,
    }
}

fn hooks_file(paths: &Paths, h: Harness) -> Option<PathBuf> {
    match h {
        Harness::Claude => Some(paths.claude_dir.join("settings.json")),
        Harness::Codex => Some(paths.codex_dir.join("hooks.json")),
        Harness::Droid => Some(paths.factory_dir.join("hooks.json")),
        Harness::Opencode => None,
    }
}

fn block_state(paths: &Paths, state: &State, h: Harness) -> (&'static str, &'static str) {
    let Some((path, key)) = instruction_file(paths, h) else {
        return ("n/a", "n/a");
    };
    let Ok(text) = fs::read_to_string(&path) else {
        return ("absent", "absent");
    };
    match markers::find(&text, Style::Html, "store") {
        Ok(None) => ("absent", "absent"),
        Err(_) => ("invalid", "drifted"),
        Ok(Some(b)) => {
            let intact = markers::intact(&b);
            let drift = if intact && state.targets.get(key) == Some(&b.hash) {
                "ok"
            } else {
                "drifted"
            };
            (if intact { "intact" } else { "edited" }, drift)
        }
    }
}

/// The state of the MCP projection: Claude's recorded `mcp.json` hash, the Codex block, the
/// Factory entries.
fn mcp_state(paths: &Paths, state: &State, h: Harness) -> &'static str {
    match h {
        Harness::Claude => {
            if common::mcp_out_of_date(paths, state) {
                "out-of-date"
            } else {
                "ok"
            }
        }
        Harness::Opencode => "n/a",
        Harness::Codex => {
            let recorded = state.targets.get("codex:mcp");
            let text = fs::read_to_string(paths.codex_dir.join("config.toml")).ok();
            match text
                .as_deref()
                .map(|t| markers::find(t, Style::Hash, "store"))
            {
                Some(Err(_)) => "drifted",
                Some(Ok(Some(b))) if markers::intact(&b) && recorded == Some(&b.hash) => "ok",
                Some(Ok(Some(_))) => "drifted",
                _ if recorded.is_some() => "drifted",
                _ => "absent",
            }
        }
        Harness::Droid => {
            let recorded = state.targets.get(factory::MCP_KEY);
            let names = state
                .owned
                .mcp_names
                .get(h.id())
                .cloned()
                .unwrap_or_default();
            if names.is_empty() && recorded.is_none() {
                return "absent";
            }
            let doc = fs::read_to_string(paths.factory_dir.join("mcp.json"))
                .ok()
                .and_then(|t| json::parse(&t).ok());
            let servers = doc
                .as_ref()
                .and_then(|d| d.path(&["mcpServers"]))
                .and_then(Value::as_object);
            let current: Option<Vec<(String, Value)>> = servers.and_then(|s| {
                names
                    .iter()
                    .map(|n| s.get(n).map(|v| (n.clone(), v.clone())))
                    .collect()
            });
            match current {
                Some(c) if factory::owned_hash(&c).as_ref() == recorded => "ok",
                _ => "drifted",
            }
        }
    }
}

fn hook_state(paths: &Paths, state: &State, h: Harness) -> &'static str {
    let present = if h == Harness::Opencode {
        plugin_text(paths).is_some_and(|t| t.lines().next() == Some(opencode::PLUGIN_MARKER))
    } else {
        let (Some(cmd), Some(file)) = (state.owned.hooks.get(h.id()), hooks_file(paths, h)) else {
            return "absent";
        };
        fs::read_to_string(file)
            .ok()
            .and_then(|t| json::parse(&t).ok())
            .is_some_and(|doc| has_hook(&doc, cmd))
    };
    if present { "present" } else { "absent" }
}

fn plugin_text(paths: &Paths) -> Option<String> {
    fs::read_to_string(paths.opencode_dir.join("plugins").join("vibe-store.js")).ok()
}

fn opencode_drift(paths: &Paths, bin: Option<&Path>) -> &'static str {
    let Some(text) = plugin_text(paths) else {
        return "absent";
    };
    let Some(want) = bin.and_then(opencode::plugin_bytes) else {
        return "unknown";
    };
    if text == want { "ok" } else { "drifted" }
}

fn rows(paths: &Paths, store: &Store, state: &State) -> Vec<(Harness, Row)> {
    let found = common::vibe_bin().ok();
    let bin = found.clone().unwrap_or_default();
    let on = enabled_harnesses(paths, store, None, true);
    let ctx = Ctx {
        paths,
        store,
        state,
        vibe_bin: &bin,
        mode: Mode::Sync,
        harnesses: &on,
    };
    Harness::ALL
        .into_iter()
        .map(|h| {
            let enabled = on.contains(&h);
            let (block, mut drift) = block_state(paths, state, h);
            if h == Harness::Opencode {
                drift = opencode_drift(paths, found.as_deref());
            }
            let (mut skips, mut conflicts) = (0, 0);
            if enabled {
                for n in project(&ctx, h).notes {
                    match n {
                        Note::Skipped(_) => skips += 1,
                        Note::Kept(_) | Note::Refused(_) => conflicts += 1,
                        Note::Info(_) => {}
                    }
                }
            }
            let mcp = mcp_state(paths, state, h);
            if mcp == "drifted" {
                drift = "drifted";
            }
            let row = Row {
                mcp,
                enabled,
                block,
                hook: hook_state(paths, state, h),
                drift,
                skips,
                conflicts,
            };
            (h, row)
        })
        .collect()
}

fn number(n: usize) -> Value {
    json::parse(&n.to_string()).unwrap_or(Value::Null)
}

/// Prints the store's validity and each harness's state; `json` selects the JSON form.
/// Exit 1 only when the store is invalid.
pub fn status(paths: &Paths, json: bool, out: &mut dyn Write, _err: &mut dyn Write) -> u8 {
    let store = match store::load(paths) {
        Ok(s) => s,
        Err(e) => {
            let why = common::one_line(&e);
            if json {
                let mut s = Object::new();
                s.insert("valid", Value::Bool(false));
                s.insert("error", Value::str(&why));
                let mut doc = Object::new();
                doc.insert("v", number(1));
                doc.insert("store", Value::Object(s));
                doc.insert("harnesses", Value::Object(Object::new()));
                let _ = writeln!(out, "{}", json::to_pretty(&Value::Object(doc)));
            } else {
                let _ = writeln!(out, "store invalid: {why}");
            }
            return 1;
        }
    };
    let (state, state_error) = match state::load(paths) {
        Ok(s) => (s, None),
        Err(e) => (State::default(), Some(common::one_line(&e))),
    };
    let rows = rows(paths, &store, &state);
    if json {
        let mut harnesses = Object::new();
        for (h, r) in &rows {
            let mut o = Object::new();
            o.insert("enabled", Value::Bool(r.enabled));
            o.insert("block", Value::str(r.block));
            o.insert("hook", Value::str(r.hook));
            o.insert("drift", Value::str(r.drift));
            o.insert("mcp", Value::str(r.mcp));
            o.insert("skips", number(r.skips));
            o.insert("conflicts", number(r.conflicts));
            harnesses.insert(h.id(), Value::Object(o));
        }
        let mut s = Object::new();
        s.insert("valid", Value::Bool(true));
        s.insert("error", Value::Null);
        s.insert(
            "notes",
            Value::Array(store.notes.iter().map(|n| Value::str(n)).collect()),
        );
        s.insert(
            "state_error",
            state_error.as_deref().map_or(Value::Null, Value::str),
        );
        let mut doc = Object::new();
        doc.insert("v", number(1));
        doc.insert("store", Value::Object(s));
        doc.insert("harnesses", Value::Object(harnesses));
        let _ = writeln!(out, "{}", json::to_pretty(&Value::Object(doc)));
        return 0;
    }
    let _ = writeln!(out, "store ok");
    for n in &store.notes {
        let _ = writeln!(out, "note {n}");
    }
    if let Some(e) = state_error {
        let _ = writeln!(out, "state invalid: {e}");
    }
    if rows
        .iter()
        .any(|(h, r)| *h == Harness::Claude && r.enabled && r.mcp == "out-of-date")
    {
        let _ = writeln!(out, "claude mcp: out of date \u{2014} run vibe sync");
    }
    for (h, r) in rows {
        let _ = writeln!(
            out,
            "{}: enabled={} block={} hook={} drift={} skips={} conflicts={} mcp={}",
            h.id(),
            r.enabled,
            r.block,
            r.hook,
            r.drift,
            r.skips,
            r.conflicts,
            r.mcp
        );
    }
    0
}
