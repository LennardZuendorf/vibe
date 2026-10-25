//! `vibe uninstall`: remove what the recorded state says vibe created.
use std::collections::BTreeSet;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use super::common;
use crate::adapters::{self, Ctx, Mode};
use crate::exec::{self, Outcome};
use crate::paths::{Harness, Paths};
use crate::plan::{Note, OwnedEdit, PlanItem, Projection};
use crate::state;
use crate::store::Store;

fn plan(paths: &Paths, state: &state::State, bin: &Path) -> Projection {
    let store = Store::empty();
    let ctx = Ctx {
        paths,
        store: &store,
        state,
        vibe_bin: bin,
        mode: Mode::Sync,
        harnesses: &Harness::ALL,
    };
    let mut proj = Projection::default();
    for h in Harness::ALL {
        proj.merge(adapters::plan_uninstall(&ctx, h));
    }
    for (path, target) in &state.owned.links {
        let at = proj.items.len();
        proj.items.push(PlanItem::LinkRemove {
            path: PathBuf::from(path),
            target: PathBuf::from(target),
        });
        proj.edits.push((
            at,
            OwnedEdit::Link {
                path: path.clone(),
                target: target.clone(),
                present: false,
            },
        ));
    }
    proj
}

/// The created directories (deepest first, never under the store) that exist.
fn created_dirs(paths: &Paths, created: &[String]) -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = created
        .iter()
        .map(PathBuf::from)
        .filter(|p| !p.starts_with(&paths.store))
        .filter(|p| fs::symlink_metadata(p).is_ok_and(|m| m.is_dir()))
        .collect();
    dirs.sort_by_key(|d| std::cmp::Reverse(d.as_os_str().len()));
    dirs
}

/// Removes the created directories that are now empty. Returns the removed paths.
fn remove_created_dirs(paths: &Paths, created: &[String]) -> Vec<String> {
    created_dirs(paths, created)
        .into_iter()
        .filter(|d| fs::remove_dir(d).is_ok())
        .map(|d| format!("remove {}", d.display()))
        .collect()
}

/// The created directories a run would remove: those whose entries the plan removes too.
fn dirs_to_remove(paths: &Paths, created: &[String], proj: &Projection) -> Vec<String> {
    let mut gone: BTreeSet<PathBuf> = proj
        .items
        .iter()
        .filter_map(|i| match i {
            PlanItem::FileRemove { path, .. } | PlanItem::LinkRemove { path, .. } => {
                Some(path.clone())
            }
            _ => None,
        })
        .collect();
    let mut lines = Vec::new();
    for dir in created_dirs(paths, created) {
        let empty = fs::read_dir(&dir)
            .is_ok_and(|mut rd| rd.all(|e| e.is_ok_and(|e| gone.contains(&e.path()))));
        if empty {
            lines.push(format!("remove {}", dir.display()));
            gone.insert(dir);
        }
    }
    lines
}

fn run(paths: &Paths, dry_run: bool, claude_bin: &str) -> Result<Outcome, String> {
    let state = state::load(paths)?;
    let bin = common::vibe_bin().unwrap_or_default();
    let proj = plan(paths, &state, &bin);
    let created = state.owned.created.clone();
    let would_remove = dry_run.then(|| dirs_to_remove(paths, &created, &proj));
    let (mut outcome, _) = exec::apply(paths, proj, state, dry_run, claude_bin);
    let state_line = format!("remove {}", paths.state_file().display());
    if dry_run {
        outcome.applied.extend(
            would_remove
                .unwrap_or_default()
                .into_iter()
                .map(|l| format!("would {l}")),
        );
        outcome.applied.push(format!("would {state_line}"));
        return Ok(outcome);
    }
    let refused = outcome.notes.iter().any(|n| matches!(n, Note::Refused(_)));
    if !outcome.failed.is_empty() || refused {
        return Ok(outcome);
    }
    outcome.applied.extend(remove_created_dirs(paths, &created));
    fs::remove_file(paths.state_file())
        .map_err(|e| format!("{}: {e}", paths.state_file().display()))?;
    outcome.applied.push(state_line);
    Ok(outcome)
}

/// Runs `vibe uninstall`. The plan comes from `state.json` alone; the store directory is
/// never touched, and neither is the state directory: it keeps its (released) lock file.
/// A failed or refused target keeps `state.json` and the created directories, so a re-run
/// finishes the job once the target is repaired. Exit 1 when something failed, a target was refused or another run holds
/// the lock. A dry run reads only: it takes no lock.
pub fn uninstall(
    paths: &Paths,
    dry_run: bool,
    claude_bin: &str,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> u8 {
    if fs::symlink_metadata(paths.state_file()).is_err() {
        let _ = writeln!(out, "note nothing installed");
        return 0;
    }
    let work = || run(paths, dry_run, claude_bin);
    let result = if dry_run {
        Ok(Some(work()))
    } else {
        exec::with_lock(paths, work)
    };
    common::finish(result, dry_run, false, out, err)
}
