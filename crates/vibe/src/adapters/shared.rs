//! Helpers shared by the adapters that edit a harness `hooks.json`.
use std::path::Path;

use vibe_core::json::{Object, Value};

use super::Ctx;
use crate::hooks_json::{add_hook, hook_command, remove_hook};
use crate::paths::Harness;
use crate::plan::{Note, OwnedEdit, PlanItem, Projection, json_file_item};

fn shape_note(h: Harness, path: &Path) -> Note {
    let file = path.file_name().map_or_else(
        || path.display().to_string(),
        |n| n.to_string_lossy().into_owned(),
    );
    Note::Refused(format!(
        "{}: {file} hooks has an unexpected shape \u{2014} hook not written",
        h.id()
    ))
}

/// Runs `edit` on the whole document held in `obj`; the object is restored on error.
fn on_document(obj: &mut Object, edit: impl FnOnce(&mut Value) -> Result<(), String>) -> bool {
    let mut doc = Value::Object(std::mem::take(obj));
    let ok = edit(&mut doc).is_ok();
    if let Value::Object(o) = doc {
        *obj = o;
    }
    ok
}

/// Plans the SessionStart entry in `path`: a recorded command that differs from the new one
/// is removed, then the new command is added.
pub(crate) fn hook_install(ctx: &Ctx, h: Harness, path: &Path) -> Projection {
    let cmd = match hook_command(ctx.vibe_bin, h) {
        Ok(c) => c,
        Err(e) => return Projection::refused(Note::Refused(format!("{}: {e}", h.id()))),
    };
    let old = ctx
        .state
        .owned
        .hooks
        .get(h.id())
        .filter(|o| **o != cmd)
        .cloned();
    let mut shape_ok = true;
    let mut p = json_file_item(ctx, h, path, |obj| {
        shape_ok = on_document(obj, |doc| {
            if let Some(o) = &old {
                remove_hook(doc, o)?;
            }
            add_hook(doc, &cmd).map(|_| ())
        });
    });
    if !shape_ok {
        return Projection::refused(shape_note(h, path));
    }
    let edit = OwnedEdit::Hook {
        harness: h,
        command: Some(cmd.clone()),
    };
    if !p.items.is_empty() {
        p.edits.push((0, edit));
    } else if p.notes.is_empty() && ctx.state.owned.hooks.get(h.id()) != Some(&cmd) {
        p.edits.push((usize::MAX, edit));
    }
    p
}

/// Plans the removal of the recorded SessionStart entry from `path`.
pub(crate) fn hook_uninstall(ctx: &Ctx, h: Harness, path: &Path) -> Projection {
    let Some(cmd) = ctx.state.owned.hooks.get(h.id()).cloned() else {
        return Projection::default();
    };
    let mut shape_ok = true;
    let mut p = json_file_item(ctx, h, path, |obj| {
        shape_ok = on_document(obj, |doc| remove_hook(doc, &cmd).map(|_| ()));
    });
    if !shape_ok {
        return Projection::refused(shape_note(h, path));
    }
    let edit = OwnedEdit::Hook {
        harness: h,
        command: None,
    };
    if !p.items.is_empty() {
        p.edits.push((0, edit));
    } else if p.notes.is_empty() {
        p.edits.push((usize::MAX, edit));
    }
    drop_empty_created(ctx, p, |bytes| {
        vibe_core::json::parse_object_or_empty(&String::from_utf8_lossy(bytes))
            .is_ok_and(|o| o.is_empty())
    })
}

/// Turns a write of an empty result into a removal when vibe created the file.
pub(crate) fn drop_empty_created(
    ctx: &Ctx,
    mut p: Projection,
    is_empty: impl Fn(&[u8]) -> bool,
) -> Projection {
    for item in &mut p.items {
        let PlanItem::FileWrite {
            harness,
            show,
            path,
            bytes,
        } = item
        else {
            continue;
        };
        let created = ctx
            .state
            .owned
            .created
            .contains(&path.display().to_string());
        if created && is_empty(bytes) {
            *item = PlanItem::FileRemove {
                harness: *harness,
                show: std::mem::take(show),
                path: std::mem::take(path),
            };
        }
    }
    p
}
