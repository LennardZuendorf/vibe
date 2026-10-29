//! `vibe sync`: project the whole store, Claude MCP included.
use std::io::Write;

use super::common::{self, Loaded};
use crate::adapters::{Mode, enabled_harnesses};
use crate::exec::{self, Outcome};
use crate::paths::{Harness, Paths};
use crate::plan::Note;

fn run(
    paths: &Paths,
    filter: Option<&[Harness]>,
    dry_run: bool,
    claude_bin: &str,
) -> Result<Outcome, String> {
    let loaded = common::load(paths)?;
    let bin = common::vibe_bin()?;
    let harnesses = enabled_harnesses(paths, &loaded.store, filter, true);
    let mut proj = common::project_set(paths, &loaded, &bin, Mode::Sync, &harnesses);
    if filter.is_none() {
        proj.stamp = Some(common::stamp(paths, &loaded.store, common::mcp_hash(paths)));
    }
    let Loaded { state, store } = loaded;
    let (mut outcome, _) = exec::apply(paths, proj, state, dry_run, claude_bin);
    outcome
        .notes
        .extend(store.notes.into_iter().map(Note::Info));
    Ok(outcome)
}

/// Runs `vibe sync`. `harnesses` limits the run; a limited run does not stamp the store hash,
/// so the session hook still refreshes the others. Exit 1 when something failed, a target
/// was refused or another run holds the lock. A dry run reads only: it takes no lock and
/// creates no directory.
pub fn sync(
    paths: &Paths,
    harnesses: Option<Vec<Harness>>,
    dry_run: bool,
    json: bool,
    claude_bin: &str,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> u8 {
    let work = || run(paths, harnesses.as_deref(), dry_run, claude_bin);
    let result = if dry_run {
        Ok(Some(work()))
    } else {
        exec::with_lock(paths, work)
    };
    common::finish(result, dry_run, json, out, err)
}
