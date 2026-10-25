//! Plan items: what an adapter wants done, applied later by [`crate::exec`].
use std::fmt;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use vibe_core::json::{self, Object, Value};
use vibe_core::markers::{self, Style, hash_bytes};

use crate::adapters::Ctx;
use crate::paths::Harness;

/// Marker owner of every block vibe writes.
const OWNER: &str = "store";

/// One step of a plan.
#[derive(PartialEq)]
pub enum PlanItem {
    /// Write `bytes` to `path`; the executor reads the original at apply time.
    FileWrite {
        /// The harness the file belongs to.
        harness: Harness,
        /// How output and errors name the file.
        show: String,
        /// The file to write.
        path: PathBuf,
        /// The new content.
        bytes: Vec<u8>,
    },
    /// Remove `path` (uninstall).
    FileRemove {
        /// The harness the file belongs to.
        harness: Harness,
        /// How output and errors name the file.
        show: String,
        /// The file to remove.
        path: PathBuf,
    },
    /// Create the symlink `path` pointing at `target`.
    LinkCreate {
        /// The link to create.
        path: PathBuf,
        /// What it points at.
        target: PathBuf,
    },
    /// Remove the symlink `path` if it still points at `target`.
    LinkRemove {
        /// The link to remove.
        path: PathBuf,
        /// What vibe made it point at.
        target: PathBuf,
    },
    /// Run one `claude mcp` call.
    ClaudeCli {
        /// The call.
        op: CliOp,
    },
}

impl fmt::Debug for PlanItem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PlanItem::FileWrite {
                harness,
                show,
                path,
                bytes,
            } => f
                .debug_struct("FileWrite")
                .field("harness", harness)
                .field("show", show)
                .field("path", path)
                .field("bytes", &format_args!("<{} bytes redacted>", bytes.len()))
                .finish(),
            PlanItem::FileRemove {
                harness,
                show,
                path,
            } => f
                .debug_struct("FileRemove")
                .field("harness", harness)
                .field("show", show)
                .field("path", path)
                .finish(),
            PlanItem::LinkCreate { path, target } => f
                .debug_struct("LinkCreate")
                .field("path", path)
                .field("target", target)
                .finish(),
            PlanItem::LinkRemove { path, target } => f
                .debug_struct("LinkRemove")
                .field("path", path)
                .field("target", target)
                .finish(),
            PlanItem::ClaudeCli { op } => f.debug_struct("ClaudeCli").field("op", op).finish(),
        }
    }
}

impl PlanItem {
    /// One line naming what the item does, without a verb tense.
    pub fn describe(&self) -> String {
        match self {
            PlanItem::FileWrite { show, .. } => format!("write {show}"),
            PlanItem::FileRemove { show, .. } => format!("remove {show}"),
            PlanItem::LinkCreate { path, target } => {
                format!("link {} -> {}", path.display(), target.display())
            }
            PlanItem::LinkRemove { path, .. } => format!("unlink {}", path.display()),
            PlanItem::ClaudeCli { op } => match op {
                CliOp::Add { name, .. } => format!("claude mcp add {name}"),
                CliOp::Remove { name } => format!("claude mcp remove {name}"),
            },
        }
    }
}

/// A `claude mcp` call.
#[derive(PartialEq)]
pub enum CliOp {
    /// `claude mcp add-json --scope user <name> <json>`.
    Add {
        /// The server name.
        name: String,
        /// The compact canonical JSON (placeholders only, never secrets).
        json: String,
    },
    /// `claude mcp remove --scope user <name>`.
    Remove {
        /// The server name.
        name: String,
    },
}

impl fmt::Debug for CliOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CliOp::Add { name, .. } => f
                .debug_struct("Add")
                .field("name", name)
                .field("json", &"<redacted>")
                .finish(),
            CliOp::Remove { name } => f.debug_struct("Remove").field("name", name).finish(),
        }
    }
}

/// A line of output that is not an applied item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Note {
    /// Left alone on purpose.
    Kept(String),
    /// Not projected, with the reason.
    Skipped(String),
    /// Refused; the text is the whole line body.
    Refused(String),
    /// Information for the user.
    Info(String),
}

/// A change to the state, applied only when its item succeeded.
#[derive(PartialEq)]
pub enum OwnedEdit {
    /// Claude MCP ownership of `name`; `None` means no longer owned.
    McpClaude {
        /// The server name.
        name: String,
        /// The canonical JSON vibe added.
        canonical: Option<Value>,
    },
    /// The names vibe owns in a harness config (Codex, Factory).
    McpNames {
        /// The harness.
        harness: Harness,
        /// The owned names.
        names: Vec<String>,
    },
    /// The hook command vibe registered; `None` removes it.
    Hook {
        /// The harness.
        harness: Harness,
        /// The command string.
        command: Option<String>,
    },
    /// The plugin file vibe installed; `None` removes it.
    Plugin {
        /// The harness.
        harness: Harness,
        /// The plugin path.
        path: Option<String>,
    },
    /// A link vibe owns (`present`) or no longer owns.
    Link {
        /// The link path.
        path: String,
        /// The recorded target.
        target: String,
        /// Whether the link is owned after the edit.
        present: bool,
    },
    /// A file or directory vibe created; uninstall deletes it when empty or `{}`.
    Created {
        /// The path.
        path: String,
    },
    /// The hash vibe last wrote into a target; `None` forgets it.
    Target {
        /// `claude:instructions`, `codex:instructions`, `codex:mcp`, `droid:instructions` or `droid:mcp`.
        key: String,
        /// The hash.
        hash: Option<String>,
    },
}

impl fmt::Debug for OwnedEdit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OwnedEdit::McpClaude { name, canonical } => f
                .debug_struct("McpClaude")
                .field("name", name)
                .field("canonical", &canonical.as_ref().map(|_| "<redacted>"))
                .finish(),
            OwnedEdit::McpNames { harness, names } => f
                .debug_struct("McpNames")
                .field("harness", harness)
                .field("names", names)
                .finish(),
            OwnedEdit::Hook { harness, command } => f
                .debug_struct("Hook")
                .field("harness", harness)
                .field("command", command)
                .finish(),
            OwnedEdit::Plugin { harness, path } => f
                .debug_struct("Plugin")
                .field("harness", harness)
                .field("path", path)
                .finish(),
            OwnedEdit::Link {
                path,
                target,
                present,
            } => f
                .debug_struct("Link")
                .field("path", path)
                .field("target", target)
                .field("present", present)
                .finish(),
            OwnedEdit::Created { path } => f.debug_struct("Created").field("path", path).finish(),
            OwnedEdit::Target { key, hash } => f
                .debug_struct("Target")
                .field("key", key)
                .field("hash", hash)
                .finish(),
        }
    }
}

/// The cache stamps the executor records when a run fully succeeds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stamp {
    /// Hash of everything the store feeds into a projection.
    pub store_hash: String,
    /// Hash of the bytes of `mcp.json`.
    pub mcp_hash: String,
}

/// Everything an adapter plans: items, notes and the state edits tied to them.
#[derive(Debug, Default, PartialEq)]
pub struct Projection {
    /// The steps, in order.
    pub items: Vec<PlanItem>,
    /// Output lines that are not steps.
    pub notes: Vec<Note>,
    /// State edits; the index names the item that must succeed, `usize::MAX` ties the
    /// edit to the file stage as a whole.
    pub edits: Vec<(usize, OwnedEdit)>,
    /// Stamps recorded when the run has no failure; set by the caller of the dispatcher.
    pub stamp: Option<Stamp>,
}

impl Projection {
    /// Appends `other`, shifting its item indexes. The first stamp wins.
    pub fn merge(&mut self, other: Projection) {
        let offset = self.items.len();
        self.items.extend(other.items);
        self.notes.extend(other.notes);
        self.edits.extend(other.edits.into_iter().map(|(i, e)| {
            let i = if i == usize::MAX { i } else { i + offset };
            (i, e)
        }));
        if self.stamp.is_none() {
            self.stamp = other.stamp;
        }
    }

    /// Removes every `claude` call and the state edits that depend on one, re-indexing the
    /// rest. The session hook runs a plan through this so it can never spawn a process.
    pub fn drop_cli(&mut self) {
        let mut map: Vec<Option<usize>> = Vec::with_capacity(self.items.len());
        let mut kept = 0;
        for item in &self.items {
            if matches!(item, PlanItem::ClaudeCli { .. }) {
                map.push(None);
            } else {
                map.push(Some(kept));
                kept += 1;
            }
        }
        self.items
            .retain(|i| !matches!(i, PlanItem::ClaudeCli { .. }));
        self.edits = std::mem::take(&mut self.edits)
            .into_iter()
            .filter_map(|(i, e)| {
                if i == usize::MAX {
                    Some((i, e))
                } else {
                    map.get(i).copied().flatten().map(|n| (n, e))
                }
            })
            .collect();
    }

    /// A projection holding only the refusal `note`.
    pub(crate) fn refused(note: Note) -> Projection {
        Projection {
            notes: vec![note],
            ..Projection::default()
        }
    }
}

/// Reads a target file as text. `Ok(None)` when it does not exist; a symlink, a file that
/// is not valid UTF-8 and any read error are refusals.
pub fn guard_target(path: &Path) -> Result<Option<String>, Note> {
    let refused = |why: &dyn fmt::Display| Note::Refused(format!("{}: {why}", path.display()));
    match fs::symlink_metadata(path) {
        Ok(m) if m.file_type().is_symlink() => Err(Note::Refused(format!(
            "{} is a symlink \u{2014} not written",
            path.display()
        ))),
        Ok(_) => match fs::read(path) {
            Ok(bytes) => String::from_utf8(bytes).map(Some).map_err(|_| {
                Note::Refused(format!(
                    "{} is not valid UTF-8 \u{2014} not written",
                    path.display()
                ))
            }),
            Err(e) => Err(refused(&e)),
        },
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(None),
        Err(e) => Err(refused(&e)),
    }
}

/// Plans the managed block of `path`: `body` upserts it, `None` removes it. No item when the
/// text would not change; then only a stale target hash in the state is corrected.
pub fn block_item(
    ctx: &Ctx,
    harness: Harness,
    key: &str,
    path: &Path,
    style: Style,
    body: Option<&str>,
) -> Projection {
    let current = match guard_target(path) {
        Ok(c) => c,
        Err(n) => return Projection::refused(n),
    };
    let text = current.clone().unwrap_or_default();
    let refuse = |e: vibe_core::Error| {
        Projection::refused(Note::Refused(format!("{}: {e}", path.display())))
    };
    let existing = match markers::find(&text, style, OWNER) {
        Ok(b) => b,
        Err(e) => return refuse(e),
    };
    let next = match body {
        Some(b) => markers::upsert(&text, style, OWNER, b),
        None => markers::remove(&text, style, OWNER),
    };
    let next = match next {
        Ok(n) => n,
        Err(e) => return refuse(e),
    };
    let hash = body.and_then(|_| {
        markers::find(&next, style, OWNER)
            .ok()
            .flatten()
            .map(|b| hash_bytes(b.body.as_bytes()))
    });
    let mut p = Projection::default();
    if next == text {
        if ctx.state.targets.get(key) != hash.as_ref() {
            p.edits.push((
                usize::MAX,
                OwnedEdit::Target {
                    key: key.to_string(),
                    hash,
                },
            ));
        }
        return p;
    }
    if body.is_some()
        && let Some(b) = &existing
        && !markers::intact(b)
    {
        p.notes.push(Note::Info(format!(
            "{}: {} edited inside the vibe block \u{2014} replaced",
            harness.id(),
            path.display()
        )));
    }
    push_write(&mut p, harness, path, next.into_bytes(), current.is_none());
    p.edits.push((
        0,
        OwnedEdit::Target {
            key: key.to_string(),
            hash,
        },
    ));
    p
}

fn push_write(p: &mut Projection, harness: Harness, path: &Path, bytes: Vec<u8>, is_new: bool) {
    let idx = p.items.len();
    p.items.push(PlanItem::FileWrite {
        harness,
        show: path.display().to_string(),
        path: path.to_path_buf(),
        bytes,
    });
    if is_new {
        p.edits.push((
            idx,
            OwnedEdit::Created {
                path: path.display().to_string(),
            },
        ));
    }
}

/// Plans an edit of the JSON object in `path`. No item when the edit leaves the value
/// unchanged; otherwise the file is re-serialised with two-space indentation.
pub fn json_file_item(
    _ctx: &Ctx,
    harness: Harness,
    path: &Path,
    edit: impl FnOnce(&mut Object),
) -> Projection {
    let current = match guard_target(path) {
        Ok(c) => c,
        Err(n) => return Projection::refused(n),
    };
    let mut obj = match json::parse_object_or_empty(current.as_deref().unwrap_or("")) {
        Ok(o) => o,
        Err(e) => {
            return Projection::refused(Note::Refused(format!("{}: {e}", path.display())));
        }
    };
    let before = Value::Object(obj.clone());
    edit(&mut obj);
    let after = Value::Object(obj);
    let mut p = Projection::default();
    if after == before {
        return p;
    }
    push_write(
        &mut p,
        harness,
        path,
        json::to_pretty(&after).into_bytes(),
        current.is_none(),
    );
    p
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cli(name: &str) -> PlanItem {
        PlanItem::ClaudeCli {
            op: CliOp::Remove { name: name.into() },
        }
    }

    fn rm(path: &str) -> PlanItem {
        PlanItem::FileRemove {
            harness: Harness::Codex,
            show: path.into(),
            path: PathBuf::from(path),
        }
    }

    #[test]
    fn drop_cli_removes_calls_and_reindexes_edits() {
        let mut p = Projection {
            items: vec![cli("a"), rm("x"), cli("b"), rm("y")],
            edits: vec![
                (
                    0,
                    OwnedEdit::McpClaude {
                        name: "a".into(),
                        canonical: None,
                    },
                ),
                (3, OwnedEdit::Created { path: "y".into() }),
                (usize::MAX, OwnedEdit::Created { path: "z".into() }),
            ],
            ..Projection::default()
        };
        p.drop_cli();
        assert_eq!(p.items.len(), 2);
        assert!(
            p.items
                .iter()
                .all(|i| !matches!(i, PlanItem::ClaudeCli { .. }))
        );
        assert_eq!(p.edits.len(), 2);
        assert_eq!(p.edits[0].0, 1);
        assert_eq!(p.edits[1].0, usize::MAX);
    }
}
