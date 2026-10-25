//! Claude Code: import block in `CLAUDE.md`, hook in `settings.json`, MCP through the CLI.
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use vibe_core::json::{self, Value};
use vibe_core::markers::Style;

use super::{Ctx, Mode, shared};
use crate::paths::Harness;
use crate::plan::{CliOp, Note, OwnedEdit, PlanItem, Projection, block_item};
use crate::translate::{Translated, claude as translate_claude};

const INSTRUCTIONS_KEY: &str = "claude:instructions";

fn instructions(ctx: &Ctx, h: Harness) -> Projection {
    if ctx.store.instructions.is_none() {
        return Projection::default();
    }
    let Some(agents) = ctx.paths.store_agents().to_str().map(|s| {
        if cfg!(windows) {
            s.replace('\\', "/")
        } else {
            s.to_string()
        }
    }) else {
        return Projection::default();
    };
    block_item(
        ctx,
        h,
        INSTRUCTIONS_KEY,
        &ctx.paths.claude_dir.join("CLAUDE.md"),
        Style::Html,
        Some(&format!("@{agents}\n")),
    )
}

/// The `mcpServers` object of `.claude.json`, read-only. A missing or unusable file counts
/// as empty and adds one note.
fn current_servers(path: &Path, notes: &mut Vec<Note>) -> Vec<(String, Value)> {
    let info = |why: &str| {
        Note::Info(format!(
            "claude: {} {why} \u{2014} treated as empty",
            path.display()
        ))
    };
    let text = match fs::read_to_string(path) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            notes.push(info("is missing"));
            return Vec::new();
        }
        Err(_) => {
            notes.push(info("is unreadable"));
            return Vec::new();
        }
    };
    let Ok(doc) = json::parse(&text) else {
        notes.push(info("is not valid JSON"));
        return Vec::new();
    };
    match doc.path(&["mcpServers"]) {
        None => Vec::new(),
        Some(v) => v.as_object().map_or_else(
            || {
                notes.push(info("has a non-object \"mcpServers\""));
                Vec::new()
            },
            |o| o.iter().map(|(k, v)| (k.to_string(), v.clone())).collect(),
        ),
    }
}

fn edited_note(name: &str) -> Note {
    Note::Kept(format!("claude: {name} edited outside vibe \u{2014} kept"))
}

fn push_cli(p: &mut Projection, op: CliOp) -> usize {
    p.items.push(PlanItem::ClaudeCli { op });
    p.items.len() - 1
}

/// Plans the `claude mcp` calls that bring `.claude.json` in line with `desired`
/// (translated store servers); `desired` empty plans only the removal of owned entries.
fn plan_mcp(ctx: &Ctx, desired: &[(String, Value)], mut p: Projection) -> Projection {
    let owned = &ctx.state.owned.mcp_claude;
    if desired.is_empty() && owned.is_empty() {
        return p;
    }
    let current = current_servers(&ctx.paths.claude_json, &mut p.notes);
    let entry = |n: &str| current.iter().find(|(k, _)| k == n).map(|(_, v)| v);
    let record = |n: &str| owned.iter().find(|(k, _)| k == n).map(|(_, v)| v);
    let drop_owned = |p: &mut Projection, name: &str, at: usize| {
        p.edits.push((
            at,
            OwnedEdit::McpClaude {
                name: name.to_string(),
                canonical: None,
            },
        ));
    };
    let record_new = |p: &mut Projection, name: &str, canonical: &Value, at: usize| {
        p.edits.push((
            at,
            OwnedEdit::McpClaude {
                name: name.to_string(),
                canonical: Some(canonical.clone()),
            },
        ));
    };
    let add = |p: &mut Projection, name: &str, canonical: &Value| {
        let at = push_cli(
            p,
            CliOp::Add {
                name: name.to_string(),
                json: json::to_compact(canonical),
            },
        );
        record_new(p, name, canonical, at);
    };
    for (name, want) in desired {
        match (record(name), entry(name)) {
            (Some(_), None) => {
                add(&mut p, name, want);
                p.notes.push(Note::Info(format!("claude: re-added {name}")));
            }
            (Some(rec), Some(cur)) if !cur.equal_ignoring_order(rec) => {
                p.notes.push(edited_note(name));
            }
            (Some(rec), Some(_)) => {
                if !want.equal_ignoring_order(rec) {
                    push_cli(&mut p, CliOp::Remove { name: name.clone() });
                    add(&mut p, name, want);
                }
            }
            (None, Some(_)) => p.notes.push(Note::Kept(format!("claude's own {name}"))),
            (None, None) => add(&mut p, name, want),
        }
    }
    let wanted: BTreeSet<&str> = desired.iter().map(|(n, _)| n.as_str()).collect();
    for (name, rec) in owned.iter().filter(|(n, _)| !wanted.contains(n.as_str())) {
        match entry(name) {
            None => drop_owned(&mut p, name, usize::MAX),
            Some(cur) if cur.equal_ignoring_order(rec) => {
                let at = push_cli(&mut p, CliOp::Remove { name: name.clone() });
                drop_owned(&mut p, name, at);
            }
            Some(_) => {
                p.notes.push(edited_note(name));
                drop_owned(&mut p, name, usize::MAX);
            }
        }
    }
    p
}

fn mcp(ctx: &Ctx, h: Harness) -> Projection {
    if ctx.mode != Mode::Sync {
        return Projection::default();
    }
    let mut notes = Vec::new();
    let mut desired = Vec::new();
    for s in ctx.store.servers.iter().filter(|s| s.applies_to(h)) {
        match translate_claude(s) {
            Translated::Ok(v) => desired.push((s.name.clone(), v)),
            Translated::Skip(why) => {
                notes.push(Note::Skipped(format!("claude: {} ({why})", s.name)));
            }
        }
    }
    plan_mcp(
        ctx,
        &desired,
        Projection {
            notes,
            ..Projection::default()
        },
    )
}

/// Plans the instructions import block and, in `sync` mode, the MCP calls.
pub fn project(ctx: &Ctx, h: Harness) -> Projection {
    let mut p = instructions(ctx, h);
    p.merge(mcp(ctx, h));
    p
}

/// Plans the SessionStart entry in `settings.json`.
pub fn plan_hook_install(ctx: &Ctx, h: Harness) -> Projection {
    shared::hook_install(ctx, h, &ctx.paths.claude_dir.join("settings.json"))
}

fn text_is_empty(bytes: &[u8]) -> bool {
    String::from_utf8_lossy(bytes).trim().is_empty()
}

/// Plans the removal of the import block, the hook entry and the owned MCP servers; files
/// vibe created that end up empty are removed.
pub fn plan_uninstall(ctx: &Ctx, h: Harness) -> Projection {
    let dir = &ctx.paths.claude_dir;
    let removal = block_item(
        ctx,
        h,
        INSTRUCTIONS_KEY,
        &dir.join("CLAUDE.md"),
        Style::Html,
        None,
    );
    let mut p = shared::drop_empty_created(ctx, removal, text_is_empty);
    p.merge(shared::hook_uninstall(ctx, h, &dir.join("settings.json")));
    p.merge(plan_mcp(ctx, &[], Projection::default()));
    p
}
