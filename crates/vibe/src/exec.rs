//! The transactional executor: files, then links, then `claude` calls.
use std::collections::BTreeSet;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use vibe_core::fs::{Write, commit, restore};
use vibe_core::lock::{Acquired, Lock, STALE, refresh_path};

use crate::links;
use crate::paths::Paths;
use crate::plan::{CliOp, Note, OwnedEdit, PlanItem, Projection, Stamp};
use crate::state::{self, State};

/// What a run did, as output lines.
#[derive(Debug, Default, PartialEq)]
pub struct Outcome {
    /// One line per applied item (`would ...` on a dry run).
    pub applied: Vec<String>,
    /// One line per failure; non-empty means exit 1.
    pub failed: Vec<String>,
    /// Notes from the plan and from apply-time checks.
    pub notes: Vec<Note>,
    /// A `claude` call found no CLI on `PATH`: the MCP stamp must not advance.
    pub cli_missing: bool,
}

/// Runs `f` while holding the projection lock. `None` means another run holds it.
pub fn with_lock<T>(paths: &Paths, f: impl FnOnce() -> T) -> Result<Option<T>, String> {
    match Lock::acquire(&paths.lock_file(), STALE).map_err(|e| e.to_string())? {
        Acquired::Got(_guard) => Ok(Some(f())),
        Acquired::Busy => Ok(None),
    }
}

enum LinkDone {
    Created { path: PathBuf, dirs: Vec<PathBuf> },
    Removed { path: PathBuf, target: PathBuf },
}

/// Everything needed to undo the file and link stages.
#[derive(Default)]
struct Undo {
    written: Vec<Write>,
    removed: Vec<(Write, Option<fs::Permissions>)>,
    removed_links: Vec<(PathBuf, PathBuf)>,
    dirs: Vec<PathBuf>,
    links: Vec<LinkDone>,
}

impl Undo {
    /// Undoes links in reverse, then restores files and removes directories this run made.
    fn run(&self) -> Vec<String> {
        for done in self.links.iter().rev() {
            match done {
                LinkDone::Created { path, dirs } => {
                    let _ = fs::remove_file(path);
                    remove_dirs(dirs);
                }
                LinkDone::Removed { path, target } => {
                    let _ = links::create_link(path, target);
                }
            }
        }
        let (_, mut failed) = restore(self.removed.iter().map(|(w, _)| w).chain(&self.written));
        for (w, mode) in &self.removed {
            if let Some(m) = mode
                && let Err(e) = fs::set_permissions(&w.path, m.clone())
            {
                failed.push(format!("{}: {e}", w.show));
            }
        }
        for (path, target) in &self.removed_links {
            if let Err(e) = links::create_link(path, target) {
                failed.push(format!("{}: {e}", path.display()));
            }
        }
        remove_dirs(&self.dirs);
        failed
    }
}

fn remove_dirs(dirs: &[PathBuf]) {
    let mut dirs = dirs.to_vec();
    dirs.sort_by_key(|d| std::cmp::Reverse(d.as_os_str().len()));
    for d in dirs {
        let _ = fs::remove_dir(d);
    }
}

fn read_original(path: &Path) -> Result<Option<Vec<u8>>, String> {
    match fs::read(path) {
        Ok(b) => Ok(Some(b)),
        Err(e) if matches!(e.kind(), ErrorKind::NotFound | ErrorKind::NotADirectory) => Ok(None),
        Err(e) => Err(format!("{}: {e}", path.display())),
    }
}

/// Creates the missing ancestors of `path`'s parent one by one, recording each directory in
/// `created` as it is made, so a failure part-way still reports what to remove.
fn make_parent(path: &Path, created: &mut Vec<PathBuf>) -> Result<(), String> {
    let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) else {
        return Ok(());
    };
    let mut missing: Vec<&Path> = parent
        .ancestors()
        .take_while(|a| !a.as_os_str().is_empty() && !a.exists())
        .collect();
    missing.reverse();
    for dir in missing {
        fs::create_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        created.push(dir.to_path_buf());
    }
    Ok(())
}

fn first_line(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes)
        .lines()
        .next()
        .unwrap_or("")
        .trim()
        .to_string()
}

/// What one `claude` call came to.
enum CliResult {
    Done,
    /// The `claude` executable does not exist.
    Missing,
}

fn run_cli(paths: &Paths, claude_bin: &str, op: &CliOp) -> Result<CliResult, String> {
    let mut cmd = Command::new(claude_bin);
    cmd.env("HOME", &paths.home);
    #[cfg(windows)]
    cmd.env("USERPROFILE", &paths.home);
    match &paths.claude_config_env {
        Some(dir) => cmd.env("CLAUDE_CONFIG_DIR", dir),
        None => cmd.env_remove("CLAUDE_CONFIG_DIR"),
    };
    let name = match op {
        CliOp::Add { name, json } => {
            cmd.args(["mcp", "add-json", "--scope", "user", name, json]);
            name
        }
        CliOp::Remove { name } => {
            cmd.args(["mcp", "remove", "--scope", "user", name]);
            name
        }
    };
    let verb = if matches!(op, CliOp::Add { .. }) {
        "add"
    } else {
        "remove"
    };
    let out = match cmd.stdin(Stdio::null()).output() {
        Ok(o) => o,
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(CliResult::Missing),
        Err(e) => return Err(format!("claude mcp {verb} {name}: {e}")),
    };
    if out.status.success() {
        return Ok(CliResult::Done);
    }
    let mut why = first_line(&out.stderr);
    if why.is_empty() {
        why = out.status.to_string();
    }
    Err(format!("claude mcp {verb} {name}: {why}"))
}

/// Applies a plan: all file writes in one commit, then removals, then links, then `claude`
/// calls. A failure in files or links rolls both back and leaves the state as loaded; a
/// failed `claude` call rolls nothing back and is not recorded. Returns the outcome and the
/// state to carry on with: the old state when `state.json` could not be written. A dry run
/// changes nothing.
///
/// The cache stamps are recorded whenever no item failed. `Refused` and `Skipped` notes do not
/// suppress them, so the session hook stays quiet; `vibe store status` and `sync` print those
/// notes on every run. A missing `claude` CLI is the exception for the MCP stamp: it stays at its
/// old value so `status` and the hook keep reporting Claude MCP out of date.
pub fn apply(
    paths: &Paths,
    mut proj: Projection,
    state: State,
    dry_run: bool,
    claude_bin: &str,
) -> (Outcome, State) {
    let mut out = Outcome {
        notes: std::mem::take(&mut proj.notes),
        ..Outcome::default()
    };
    if dry_run {
        out.applied = proj
            .items
            .iter()
            .map(|i| format!("would {}", i.describe()))
            .collect();
        return (out, state);
    }
    let mut undo = Undo::default();
    let mut ok: BTreeSet<usize> = BTreeSet::new();
    let lock = paths.lock_file();

    refresh_path(&lock);
    if let Err(e) = file_stage(&proj.items, &mut undo, &mut ok, &mut out) {
        out.failed.push(e);
        out.failed.extend(undo.run());
        return (out, state);
    }
    refresh_path(&lock);
    if let Err(e) = link_stage(&proj, &mut undo, &mut ok, &mut out) {
        out.failed.push(e);
        out.failed.extend(undo.run());
        return (out, state);
    }

    let mut missing_noted = false;
    for (i, item) in proj.items.iter().enumerate() {
        let PlanItem::ClaudeCli { op } = item else {
            continue;
        };
        refresh_path(&lock);
        match run_cli(paths, claude_bin, op) {
            Ok(CliResult::Done) => {
                ok.insert(i);
                out.applied.push(item.describe());
            }
            Ok(CliResult::Missing) => {
                if !missing_noted {
                    missing_noted = true;
                    out.cli_missing = true;
                    out.notes.push(Note::Skipped(
                        "claude: CLI not found \u{2014} MCP not projected".to_string(),
                    ));
                }
            }
            Err(line) => out.failed.push(line),
        }
    }

    let loaded = state::to_value(&state);
    let old = state.clone();
    let mut new = state;
    for (i, edit) in proj.edits {
        if i == usize::MAX || ok.contains(&i) {
            new.apply(edit);
        }
    }
    if out.failed.is_empty()
        && let Some(stamp) = proj.stamp
    {
        new.store_hash = stamp.store_hash;
        if !out.cli_missing {
            new.mcp_hash = stamp.mcp_hash;
        }
    }
    if state::to_value(&new) != loaded && !save(paths, &new, &mut out) {
        return (out, old);
    }
    (out, new)
}

/// Records `stamp` in `state` and saves it when that changes anything. Returns the state to
/// carry on with: the old one when `state.json` could not be written.
pub fn record_stamp(paths: &Paths, state: State, stamp: Stamp, out: &mut Outcome) -> State {
    let mut new = state.clone();
    new.store_hash = stamp.store_hash;
    new.mcp_hash = stamp.mcp_hash;
    if state::to_value(&new) == state::to_value(&state) || !save(paths, &new, out) {
        return state;
    }
    new
}

fn save(paths: &Paths, new: &State, out: &mut Outcome) -> bool {
    let PlanItem::FileWrite {
        path, bytes, show, ..
    } = state::save_plan(paths, new)
    else {
        return false;
    };
    let mut dirs = Vec::new();
    if let Err(e) = make_parent(&path, &mut dirs) {
        remove_dirs(&dirs);
        out.failed.push(format!("{show}: {e}"));
        return false;
    }
    let original = read_original(&path).unwrap_or(None);
    let write = Write {
        path,
        show,
        bytes,
        original,
    };
    if let Err(e) = commit(std::slice::from_ref(&write)) {
        remove_dirs(&dirs);
        out.failed.push(e);
        return false;
    }
    true
}

fn file_stage(
    items: &[PlanItem],
    undo: &mut Undo,
    ok: &mut BTreeSet<usize>,
    out: &mut Outcome,
) -> Result<(), String> {
    let mut writes: Vec<Write> = Vec::new();
    for (i, item) in items.iter().enumerate() {
        if let PlanItem::FileWrite {
            show, path, bytes, ..
        } = item
        {
            let staged =
                read_original(path).and_then(|o| make_parent(path, &mut undo.dirs).map(|()| o));
            let original = match staged {
                Ok(o) => o,
                Err(e) => {
                    remove_dirs(&undo.dirs);
                    undo.dirs.clear();
                    return Err(format!("{show}: {e}"));
                }
            };
            writes.push(Write {
                path: path.clone(),
                show: show.clone(),
                bytes: bytes.clone(),
                original,
            });
            ok.insert(i);
        }
    }
    if let Err(e) = commit(&writes) {
        remove_dirs(&undo.dirs);
        undo.dirs.clear();
        return Err(e);
    }
    undo.written = writes;
    for (i, item) in items.iter().enumerate() {
        let PlanItem::FileRemove { show, path, .. } = item else {
            continue;
        };
        match fs::symlink_metadata(path) {
            Err(e) if matches!(e.kind(), ErrorKind::NotFound | ErrorKind::NotADirectory) => {}
            Err(e) => return Err(format!("{show}: {e}")),
            Ok(m) if m.file_type().is_symlink() => {
                let target = fs::read_link(path).map_err(|e| format!("{show}: {e}"))?;
                fs::remove_file(path).map_err(|e| format!("{show}: {e}"))?;
                undo.removed_links.push((path.clone(), target));
            }
            Ok(m) => {
                let original = fs::read(path).map_err(|e| format!("{show}: {e}"))?;
                fs::remove_file(path).map_err(|e| format!("{show}: {e}"))?;
                undo.removed.push((
                    Write {
                        path: path.clone(),
                        show: show.clone(),
                        bytes: original.clone(),
                        original: Some(original),
                    },
                    Some(m.permissions()),
                ));
            }
        }
        ok.insert(i);
    }
    for (i, item) in items.iter().enumerate() {
        if matches!(
            item,
            PlanItem::FileWrite { .. } | PlanItem::FileRemove { .. }
        ) && ok.contains(&i)
        {
            out.applied.push(item.describe());
        }
    }
    Ok(())
}

fn link_stage(
    proj: &Projection,
    undo: &mut Undo,
    ok: &mut BTreeSet<usize>,
    out: &mut Outcome,
) -> Result<(), String> {
    for (i, item) in proj.items.iter().enumerate() {
        match item {
            PlanItem::LinkCreate { path, target } => {
                if links::is_link_to(path, target) {
                    ok.insert(i);
                    continue;
                }
                if fs::symlink_metadata(path).is_ok() {
                    out.notes.push(Note::Skipped(format!(
                        "{} exists, not a vibe link \u{2014} skipped",
                        path.display()
                    )));
                    continue;
                }
                let dirs = created_dirs(proj, i);
                if let Err(e) = links::create_link(path, target) {
                    remove_dirs(&dirs);
                    return Err(format!("link {}: {e}", path.display()));
                }
                undo.links.push(LinkDone::Created {
                    path: path.clone(),
                    dirs,
                });
                ok.insert(i);
                out.applied.push(item.describe());
            }
            PlanItem::LinkRemove { path, target } => {
                if links::is_link_to(path, target) {
                    if let Err(e) = fs::remove_file(path) {
                        return Err(format!("unlink {}: {e}", path.display()));
                    }
                    undo.links.push(LinkDone::Removed {
                        path: path.clone(),
                        target: target.clone(),
                    });
                    out.applied.push(item.describe());
                } else if fs::symlink_metadata(path).is_ok() {
                    out.notes.push(Note::Kept(format!(
                        "{} is not a link to {} \u{2014} kept",
                        path.display(),
                        target.display()
                    )));
                }
                ok.insert(i);
            }
            _ => {}
        }
    }
    Ok(())
}

/// The directories the plan says `create_link` makes for item `i`.
fn created_dirs(proj: &Projection, i: usize) -> Vec<PathBuf> {
    proj.edits
        .iter()
        .filter_map(|(at, e)| match e {
            OwnedEdit::Created { path } if *at == i => Some(PathBuf::from(path)),
            _ => None,
        })
        .collect()
}
