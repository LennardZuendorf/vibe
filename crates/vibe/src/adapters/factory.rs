//! Factory droid: instructions block in `AGENTS.md`, owned keys in `mcp.json`, hook in `hooks.json`.
use std::collections::BTreeSet;

use vibe_core::json::{self, Object, Value};
use vibe_core::markers::{Style, hash_bytes};

use super::{Ctx, shared};
use crate::paths::Harness;
use crate::plan::{Note, OwnedEdit, Projection, block_item, json_file_item};
use crate::translate::{Translated, factory as translate_factory};

pub(crate) const MCP_KEY: &str = "droid:mcp";

fn instructions(ctx: &Ctx, h: Harness) -> Projection {
    let Some(text) = &ctx.store.instructions else {
        return Projection::default();
    };
    block_item(
        ctx,
        h,
        "droid:instructions",
        &ctx.paths.factory_dir.join("AGENTS.md"),
        Style::Html,
        Some(text),
    )
}

pub(crate) fn owned_hash(owned: &[(String, Value)]) -> Option<String> {
    if owned.is_empty() {
        return None;
    }
    let mut o = Object::new();
    for (k, v) in owned {
        o.insert(k.as_str(), v.clone());
    }
    Some(hash_bytes(json::to_compact(&Value::Object(o)).as_bytes()))
}

fn mcp(ctx: &Ctx, h: Harness) -> Projection {
    let path = ctx.paths.factory_dir.join("mcp.json");
    let previous: BTreeSet<&str> = ctx
        .state
        .owned
        .mcp_names
        .get(h.id())
        .map(|v| v.iter().map(String::as_str).collect())
        .unwrap_or_default();
    let mut notes = Vec::new();
    let mut desired: Vec<(String, Value)> = Vec::new();
    for s in ctx.store.servers.iter().filter(|s| s.applies_to(h)) {
        match translate_factory(s) {
            Translated::Ok(v) => desired.push((s.name.clone(), v)),
            Translated::Skip(why) => {
                notes.push(Note::Skipped(format!("droid: {} ({why})", s.name)));
            }
        }
    }
    let mut owned: Vec<(String, Value)> = Vec::new();
    let mut shape_ok = true;
    let mut p = json_file_item(ctx, h, &path, |obj| {
        if obj
            .get("mcpServers")
            .is_some_and(|v| v.as_object().is_none())
        {
            shape_ok = false;
            return;
        }
        if obj.get("mcpServers").is_none() && desired.is_empty() {
            return;
        }
        if obj.get("mcpServers").is_none() {
            obj.insert("mcpServers", Value::Object(Object::new()));
        }
        let Some(file) = obj.get_mut("mcpServers").and_then(Value::as_object_mut) else {
            return;
        };
        if let Some(recorded) = ctx.state.targets.get(MCP_KEY) {
            let current: Vec<(String, Value)> = previous
                .iter()
                .filter_map(|n| file.get(n).map(|v| ((*n).to_string(), v.clone())))
                .collect();
            if current.len() != previous.len() || owned_hash(&current).as_ref() != Some(recorded) {
                notes.push(Note::Info(
                    "droid: mcp.json vibe entries edited outside vibe \u{2014} replaced"
                        .to_string(),
                ));
            }
        }
        for (name, value) in &desired {
            if file.get(name).is_some() && !previous.contains(name.as_str()) {
                notes.push(Note::Kept(format!("droid's own {name}")));
                continue;
            }
            file.insert(name.as_str(), value.clone());
            owned.push((name.clone(), value.clone()));
        }
        for name in &previous {
            if !owned.iter().any(|(n, _)| n == name) {
                file.remove(name);
            }
        }
    });
    if !shape_ok {
        notes.push(Note::Refused(format!(
            "{}: \"mcpServers\" is not an object",
            path.display()
        )));
        return Projection {
            notes,
            ..Projection::default()
        };
    }
    p.notes.append(&mut notes);
    if p.notes.iter().any(|n| matches!(n, Note::Refused(_))) {
        return p;
    }
    owned.sort_by(|a, b| a.0.cmp(&b.0));
    let names: Vec<String> = owned.iter().map(|(n, _)| n.clone()).collect();
    let hash = owned_hash(&owned);
    let at = if p.items.is_empty() { usize::MAX } else { 0 };
    let recorded: Vec<String> = ctx
        .state
        .owned
        .mcp_names
        .get(h.id())
        .cloned()
        .unwrap_or_default();
    if at == 0 || recorded != names {
        p.edits
            .push((at, OwnedEdit::McpNames { harness: h, names }));
    }
    if at == 0 || ctx.state.targets.get(MCP_KEY) != hash.as_ref() {
        p.edits.push((
            at,
            OwnedEdit::Target {
                key: MCP_KEY.to_string(),
                hash,
            },
        ));
    }
    p
}

/// Plans the instructions block and the owned keys of `mcp.json`.
pub fn project(ctx: &Ctx, h: Harness) -> Projection {
    let mut p = instructions(ctx, h);
    p.merge(mcp(ctx, h));
    p
}

/// Plans the SessionStart entry in `hooks.json`.
pub fn plan_hook_install(ctx: &Ctx, h: Harness) -> Projection {
    shared::hook_install(ctx, h, &ctx.paths.factory_dir.join("hooks.json"))
}

fn uninstall_mcp(ctx: &Ctx, h: Harness) -> Projection {
    let path = ctx.paths.factory_dir.join("mcp.json");
    let names = ctx
        .state
        .owned
        .mcp_names
        .get(h.id())
        .cloned()
        .unwrap_or_default();
    let created = ctx
        .state
        .owned
        .created
        .contains(&path.display().to_string());
    if names.is_empty() && !created {
        return Projection::default();
    }
    let mut shape_ok = true;
    let mut p = json_file_item(ctx, h, &path, |obj| {
        let Some(v) = obj.get_mut("mcpServers") else {
            return;
        };
        let Some(file) = v.as_object_mut() else {
            shape_ok = false;
            return;
        };
        for n in &names {
            file.remove(n);
        }
        if file.is_empty() {
            obj.remove("mcpServers");
        }
    });
    if !shape_ok {
        p.notes.push(Note::Refused(format!(
            "{}: \"mcpServers\" is not an object",
            path.display()
        )));
        return p;
    }
    if !p.notes.is_empty() {
        return p;
    }
    let at = if p.items.is_empty() { usize::MAX } else { 0 };
    p.edits.push((
        at,
        OwnedEdit::McpNames {
            harness: h,
            names: Vec::new(),
        },
    ));
    p.edits.push((
        at,
        OwnedEdit::Target {
            key: MCP_KEY.to_string(),
            hash: None,
        },
    ));
    shared::drop_empty_created(ctx, p, |bytes| {
        json::parse_object_or_empty(&String::from_utf8_lossy(bytes)).is_ok_and(|o| o.is_empty())
    })
}

/// Plans the removal of the instructions block, the owned MCP keys and the hook entry; files
/// vibe created that end up empty are removed.
pub fn plan_uninstall(ctx: &Ctx, h: Harness) -> Projection {
    let dir = &ctx.paths.factory_dir;
    let removal = block_item(
        ctx,
        h,
        "droid:instructions",
        &dir.join("AGENTS.md"),
        Style::Html,
        None,
    );
    let mut p = shared::drop_empty_created(ctx, removal, |bytes| {
        String::from_utf8_lossy(bytes).trim().is_empty()
    });
    p.merge(uninstall_mcp(ctx, h));
    p.merge(shared::hook_uninstall(ctx, h, &dir.join("hooks.json")));
    p
}
