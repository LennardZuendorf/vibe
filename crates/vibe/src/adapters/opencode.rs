//! OpenCode: the runtime plugin file; the plugin itself reads the store.
use std::path::{Path, PathBuf};

use vibe_core::json::{self, Value};

use super::Ctx;
use crate::paths::Harness;
use crate::plan::{Note, OwnedEdit, PlanItem, Projection, guard_target};

/// First line of the plugin file; a file without it is never overwritten.
pub const PLUGIN_MARKER: &str = "// vibe:store-plugin v1";
/// The plugin source; `__VIBE_BIN__` is replaced by the installing binary's path.
pub const PLUGIN_TEMPLATE: &str = include_str!("../templates/opencode-plugin.js");

const BIN_LINE: &str = "const VIBE_BIN = \"__VIBE_BIN__\";";

/// The plugin text for the installing binary `bin`; `None` when the path is not UTF-8.
pub fn plugin_bytes(bin: &Path) -> Option<String> {
    let bin = json::to_compact(&Value::str(bin.to_str()?));
    Some(PLUGIN_TEMPLATE.replacen(BIN_LINE, &format!("const VIBE_BIN = {bin};"), 1))
}

fn plugin_path(ctx: &Ctx) -> PathBuf {
    ctx.paths.opencode_dir.join("plugins").join("vibe-store.js")
}

fn plugin(ctx: &Ctx, h: Harness) -> Projection {
    let path = plugin_path(ctx);
    let Some(bytes) = plugin_bytes(ctx.vibe_bin) else {
        return Projection::refused(Note::Refused(format!(
            "opencode plugin: {} is not valid UTF-8 \u{2014} not written",
            ctx.vibe_bin.display()
        )));
    };
    let current = match guard_target(&path) {
        Ok(c) => c,
        Err(n) => return Projection::refused(n),
    };
    if let Some(text) = &current
        && text.lines().next() != Some(PLUGIN_MARKER)
    {
        return Projection::refused(Note::Refused(format!(
            "opencode plugin: {} exists and is not vibe's \u{2014} not overwritten",
            path.display()
        )));
    }
    let shown = path.display().to_string();
    let edit = OwnedEdit::Plugin {
        harness: h,
        path: Some(shown.clone()),
    };
    let mut p = Projection::default();
    if current.as_deref() == Some(bytes.as_str()) {
        if ctx.state.owned.plugins.get(h.id()) != Some(&shown) {
            p.edits.push((usize::MAX, edit));
        }
        return p;
    }
    p.items.push(PlanItem::FileWrite {
        harness: h,
        show: shown.clone(),
        path,
        bytes: bytes.into_bytes(),
    });
    p.edits.push((0, edit));
    if current.is_none() {
        p.edits.push((0, OwnedEdit::Created { path: shown }));
    }
    p
}

/// Plans the plugin file.
pub fn project(ctx: &Ctx, h: Harness) -> Projection {
    plugin(ctx, h)
}

/// Plans the plugin file, rewritten when the binary moved.
pub fn plan_hook_install(ctx: &Ctx, h: Harness) -> Projection {
    plugin(ctx, h)
}

/// Plans the removal of the plugin file when it still carries vibe's marker.
pub fn plan_uninstall(ctx: &Ctx, h: Harness) -> Projection {
    let Some(recorded) = ctx.state.owned.plugins.get(h.id()) else {
        return Projection::default();
    };
    let path = PathBuf::from(recorded);
    let forget = OwnedEdit::Plugin {
        harness: h,
        path: None,
    };
    let mut p = Projection::default();
    match guard_target(&path) {
        Err(n) => return Projection::refused(n),
        Ok(None) => p.edits.push((usize::MAX, forget)),
        Ok(Some(text)) if text.lines().next() != Some(PLUGIN_MARKER) => {
            p.notes.push(Note::Kept(format!(
                "opencode plugin: {recorded} is not vibe's \u{2014} kept"
            )));
            p.edits.push((usize::MAX, forget));
        }
        Ok(Some(_)) => {
            p.items.push(PlanItem::FileRemove {
                harness: h,
                show: recorded.clone(),
                path,
            });
            p.edits.push((0, forget));
        }
    }
    p
}
