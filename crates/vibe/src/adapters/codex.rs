//! Codex: instructions block in `AGENTS.md`, MCP block in `config.toml`, hook in `hooks.json`.
use vibe_core::markers::{self, Style};

use super::{Ctx, shared};
use crate::paths::Harness;
use crate::plan::{Note, PlanItem, Projection, block_item, guard_target};
use crate::translate::toml::{codex_tables, scan_mcp_tables};
use crate::translate::{Translated, codex as translate_codex};

const KIB32: usize = 32 * 1024;

fn instructions(ctx: &Ctx, h: Harness) -> Projection {
    let Some(text) = &ctx.store.instructions else {
        return Projection::default();
    };
    let dir = &ctx.paths.codex_dir;
    let mut p = block_item(
        ctx,
        h,
        "codex:instructions",
        &dir.join("AGENTS.md"),
        Style::Html,
        Some(text),
    );
    if dir.join("AGENTS.override.md").exists() {
        p.notes.push(Note::Info(
            "codex: AGENTS.override.md exists and shadows AGENTS.md".to_string(),
        ));
    }
    if text.len() > KIB32 {
        p.notes.push(Note::Info(format!(
            "codex: instructions exceed 32 KiB ({} bytes)",
            text.len()
        )));
    }
    p
}

/// Before the first table header after the block only blank lines and comments may stand:
/// any other line would become a key of the last table inside the block.
fn tail_is_safe(tail: &str) -> bool {
    tail.lines()
        .map(str::trim)
        .take_while(|t| !t.starts_with('['))
        .all(|t| t.is_empty() || t.starts_with('#'))
}

fn mcp(ctx: &Ctx, h: Harness) -> Projection {
    let path = ctx.paths.codex_dir.join("config.toml");
    let text = match guard_target(&path) {
        Ok(t) => t.unwrap_or_default(),
        Err(n) => return Projection::refused(n),
    };
    let found = match markers::find(&text, Style::Hash, "store") {
        Ok(b) => b,
        Err(e) => return Projection::refused(Note::Refused(format!("{}: {e}", path.display()))),
    };
    let outside = match &found {
        Some(b) => format!("{}{}", &text[..b.range.start], &text[b.range.end..]),
        None => text.clone(),
    };
    let scan = scan_mcp_tables(&outside);
    let mut notes = Vec::new();
    let mut servers = Vec::new();
    for s in ctx.store.servers.iter().filter(|s| s.applies_to(h)) {
        match translate_codex(s) {
            Translated::Skip(why) => {
                notes.push(Note::Skipped(format!("codex: {} ({why})", s.name)));
            }
            Translated::Ok(_) if scan.names.contains(&s.name) => {
                notes.push(Note::Kept(format!("codex's own {}", s.name)));
            }
            Translated::Ok(server) => servers.push(server),
        }
    }
    if scan.root_inline {
        if found.is_some() {
            notes.push(Note::Info(
                "codex: config.toml defines mcp_servers inline \u{2014} vibe MCP block removed"
                    .to_string(),
            ));
        } else if !servers.is_empty() {
            notes.push(Note::Refused(
                "codex: config.toml defines mcp_servers inline \u{2014} vibe MCP block not written"
                    .to_string(),
            ));
            return Projection {
                notes,
                ..Projection::default()
            };
        }
        servers.clear();
    }
    let body = (!servers.is_empty()).then(|| codex_tables(&servers));
    if body.is_some()
        && let Some(b) = &found
        && !tail_is_safe(&text[b.range.end..])
    {
        notes.push(Note::Refused(
            "config.toml has content after the vibe block \u{2014} fix by hand".to_string(),
        ));
        return Projection {
            notes,
            ..Projection::default()
        };
    }
    let mut p = block_item(ctx, h, "codex:mcp", &path, Style::Hash, body.as_deref());
    notes.append(&mut p.notes);
    p.notes = notes;
    p
}

/// Plans the instructions block and the MCP block.
pub fn project(ctx: &Ctx, h: Harness) -> Projection {
    let mut p = instructions(ctx, h);
    p.merge(mcp(ctx, h));
    p
}

/// Plans the SessionStart entry in `hooks.json`; a new entry needs the user's trust.
pub fn plan_hook_install(ctx: &Ctx, h: Harness) -> Projection {
    let mut p = shared::hook_install(ctx, h, &ctx.paths.codex_dir.join("hooks.json"));
    if !p.items.is_empty() {
        p.notes.push(Note::Info(
            "codex: trust the new hook with /hooks".to_string(),
        ));
    }
    p
}

fn text_is_empty(bytes: &[u8]) -> bool {
    String::from_utf8_lossy(bytes).trim().is_empty()
}

/// Plans the removal of both blocks and the hook entry; files vibe created that end up
/// empty are removed.
pub fn plan_uninstall(ctx: &Ctx, h: Harness) -> Projection {
    let dir = &ctx.paths.codex_dir;
    let mut p = Projection::default();
    for (key, file, style) in [
        ("codex:instructions", "AGENTS.md", Style::Html),
        ("codex:mcp", "config.toml", Style::Hash),
    ] {
        let removal = block_item(ctx, h, key, &dir.join(file), style, None);
        p.merge(shared::drop_empty_created(ctx, removal, text_is_empty));
    }
    p.merge(shared::hook_uninstall(ctx, h, &dir.join("hooks.json")));
    debug_assert!(
        p.items
            .iter()
            .all(|i| !matches!(i, PlanItem::ClaudeCli { .. }))
    );
    p
}
