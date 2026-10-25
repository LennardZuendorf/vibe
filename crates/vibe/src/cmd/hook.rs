//! `vibe hook session-start`: the fail-open fast path.
use std::io::Write;
use std::panic::{AssertUnwindSafe, catch_unwind};

use vibe_core::dirs::Env;
use vibe_core::json::{self, Object, Value};

use super::common::{self, Loaded};
use crate::adapters::{Mode, enabled_harnesses};
use crate::exec::{self, with_lock};
use crate::paths::{Harness, Paths};
use crate::plan::{PlanItem, Stamp};
use crate::store;

/// The hook never runs `claude`; a path that cannot exist makes any slip fail loudly.
const NO_CLI: &str = "/nonexistent/vibe-hook/claude";

const STALE_MCP: &str = "vibe: Claude MCP changed; run `vibe sync`";

/// The one notice line, when there is something to say.
fn session(paths: &Paths) -> Result<Option<String>, String> {
    if !paths.store.is_dir() {
        return Ok(None);
    }
    match with_lock(paths, || refresh(paths))? {
        None => Ok(None),
        Some(result) => result,
    }
}

fn refresh(paths: &Paths) -> Result<Option<String>, String> {
    let loaded = common::load(paths)?;
    let hash = store::store_hash(paths, &loaded.store);
    let harnesses = enabled_harnesses(paths, &loaded.store, None, true);
    if hash == loaded.state.store_hash {
        let stale =
            harnesses.contains(&Harness::Claude) && common::mcp_out_of_date(paths, &loaded.state);
        return Ok(stale.then(|| STALE_MCP.to_string()));
    }
    let bin = common::vibe_bin()?;
    let mut proj = common::project_set(paths, &loaded, &bin, Mode::Hook, &harnesses);
    proj.drop_cli();
    proj.stamp = Some(Stamp {
        store_hash: hash,
        mcp_hash: loaded.state.mcp_hash.clone(),
    });
    let mut touched: Vec<Harness> = proj
        .items
        .iter()
        .filter_map(|i| match i {
            PlanItem::FileWrite { harness, .. } | PlanItem::FileRemove { harness, .. } => {
                Some(*harness)
            }
            _ => None,
        })
        .collect();
    touched.sort();
    touched.dedup();
    let links = proj
        .items
        .iter()
        .any(|i| matches!(i, PlanItem::LinkCreate { .. } | PlanItem::LinkRemove { .. }));
    let Loaded { state, .. } = loaded;
    let claude_on = harnesses.contains(&Harness::Claude);
    let (outcome, state) = exec::apply(paths, proj, state, false, NO_CLI);
    if let Some(first) = outcome.failed.first() {
        return Err(common::one_line(first));
    }
    let mcp_stale = claude_on && common::mcp_out_of_date(paths, &state);
    if outcome.applied.is_empty() {
        return Ok(mcp_stale.then(|| STALE_MCP.to_string()));
    }
    let mut names: Vec<&str> = touched.iter().map(|h| h.id()).collect();
    if links {
        names.push("skills");
    }
    let mut line = format!(
        "vibe: store changed \u{2014} refreshed {}",
        names.join(", ")
    );
    if mcp_stale {
        line.push_str("; run `vibe sync` for Claude MCP");
    }
    Ok(Some(line))
}

/// Runs the hook for resolved `paths`. Always returns 0. Output is at most one stdout notice
/// or one stderr line `vibe hook: <cause> — skipped`. For Claude, Codex and Factory the
/// notice is one line of JSON `{"systemMessage":"..."}`, which the harness shows the user;
/// for OpenCode and when no harness is named it is plain text.
pub fn session_start(
    paths: &Paths,
    harness: Option<Harness>,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> u8 {
    emit(
        catch_unwind(AssertUnwindSafe(|| session(paths)))
            .unwrap_or_else(|_| Err("internal error".into())),
        harness,
        out,
        err,
    )
}

fn notice(line: &str, harness: Option<Harness>) -> String {
    match harness {
        Some(Harness::Claude | Harness::Codex | Harness::Droid) => {
            let mut o = Object::new();
            o.insert("systemMessage", Value::str(line));
            json::to_compact(&Value::Object(o))
        }
        Some(Harness::Opencode) | None => line.to_string(),
    }
}

fn emit(
    result: Result<Option<String>, String>,
    harness: Option<Harness>,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> u8 {
    match result {
        Ok(Some(line)) => {
            let _ = writeln!(out, "{}", notice(&line, harness));
        }
        Ok(None) => {}
        Err(cause) => {
            let _ = writeln!(
                err,
                "vibe hook: {} \u{2014} skipped",
                common::one_line(&cause)
            );
        }
    }
    0
}

/// Runs the hook from the process environment; an unresolvable home is a skipped run.
pub fn run_from_env(harness: Option<Harness>, out: &mut dyn Write, err: &mut dyn Write) -> u8 {
    std::panic::set_hook(Box::new(|_| {}));
    let paths = match Paths::from_env(&Env::from_process()) {
        Ok(p) => p,
        Err(e) => return emit(Err(e.to_string()), harness, out, err),
    };
    session_start(&paths, harness, out, err)
}
