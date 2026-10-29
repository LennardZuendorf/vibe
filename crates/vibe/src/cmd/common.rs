//! Helpers the commands share: loading under the lock, planning a harness set, rendering.
use std::io::Write;
use std::path::PathBuf;

use vibe_core::json::{Object, Value};
use vibe_core::markers::hash_bytes;

use crate::adapters::{self, Ctx, Mode};
use crate::exec::Outcome;
use crate::paths::{Harness, Paths};
use crate::plan::{Note, Projection, Stamp};
use crate::state::{self, State};
use crate::store::{self, Store, read_optional};

/// The state and the store, read together while the lock is held.
pub struct Loaded {
    /// The recorded state.
    pub state: State,
    /// The validated store.
    pub store: Store,
}

/// Loads the state, then the store. `Err` is one refusal line.
pub fn load(paths: &Paths) -> Result<Loaded, String> {
    let state = state::load(paths)?;
    let store = store::load(paths)?;
    Ok(Loaded { state, store })
}

/// The path vibe records for itself in hooks and the OpenCode plugin: the `vibe` found on
/// `PATH` when it is the running binary (compared after canonicalizing both; the `PATH`
/// spelling is kept, so a package manager's stable shim survives upgrades), else the running
/// binary's own path with its symlinks left alone.
pub fn vibe_bin() -> Result<PathBuf, String> {
    let exe = std::env::current_exe().map_err(|e| format!("cannot locate vibe: {e}"))?;
    let canon = std::fs::canonicalize(&exe).map_err(|e| format!("{}: {e}", exe.display()))?;
    let env = vibe_core::dirs::Env::from_process();
    if let Some(found) = super::doctor::find_on_path("vibe", env.var("PATH"))
        && found.is_absolute()
        && std::fs::canonicalize(&found).is_ok_and(|c| c == canon)
    {
        return Ok(strip_verbatim(found));
    }
    Ok(strip_verbatim(exe))
}

/// Whether `bin` lies inside a cargo build directory.
pub fn in_build_dir(bin: &std::path::Path) -> bool {
    let text = bin.to_string_lossy();
    text.contains("/target/") || text.contains("\\target\\")
}

fn strip_verbatim(path: PathBuf) -> PathBuf {
    match path.to_str().and_then(|s| s.strip_prefix(r"\\?\")) {
        Some(rest) if !rest.starts_with(r"UNC\") => PathBuf::from(rest),
        _ => path,
    }
}

/// The hash of the bytes of `mcp.json`; empty bytes when the file is missing.
pub fn mcp_hash(paths: &Paths) -> String {
    let bytes = read_optional(&paths.store_mcp())
        .ok()
        .flatten()
        .unwrap_or_default();
    hash_bytes(&bytes)
}

/// True when the recorded `mcp_hash` no longer matches `mcp.json`. A state that never
/// synced next to a missing or empty `mcp.json` is up to date.
pub fn mcp_out_of_date(paths: &Paths, state: &State) -> bool {
    if state.mcp_hash.is_empty() {
        let bytes = read_optional(&paths.store_mcp()).ok().flatten();
        return bytes.is_some_and(|b| !b.is_empty());
    }
    state.mcp_hash != mcp_hash(paths)
}

/// Plans the effective harness set (and the skills links) with the store as loaded.
pub fn project_set(
    paths: &Paths,
    loaded: &Loaded,
    vibe_bin: &std::path::Path,
    mode: Mode,
    harnesses: &[Harness],
) -> Projection {
    let ctx = Ctx {
        paths,
        store: &loaded.store,
        state: &loaded.state,
        vibe_bin,
        mode,
        harnesses,
    };
    adapters::project_all(&ctx)
}

/// The stamp a fully successful run records.
pub fn stamp(paths: &Paths, store: &Store, mcp_hash: String) -> Stamp {
    Stamp {
        store_hash: store::store_hash(paths, store),
        mcp_hash,
    }
}

/// One output line for an applied item. `describe` names links with their target; the
/// output names the path only.
pub fn applied_line(described: &str, dry_run: bool) -> String {
    let line = if dry_run {
        described.strip_prefix("would ").unwrap_or(described)
    } else {
        described
    };
    let line = if let Some(rest) = line.strip_prefix("link ") {
        let path = rest.rfind(" -> ").map_or(rest, |at| &rest[..at]);
        if dry_run {
            format!("link {path}")
        } else {
            format!("linked {path}")
        }
    } else if dry_run {
        line.to_string()
    } else if let Some(rest) = line.strip_prefix("write ") {
        format!("wrote {rest}")
    } else if let Some(rest) = line.strip_prefix("remove ") {
        format!("removed {rest}")
    } else if let Some(rest) = line.strip_prefix("unlink ") {
        format!("unlinked {rest}")
    } else {
        line.to_string()
    };
    if dry_run {
        format!("would {line}")
    } else {
        line
    }
}

/// One output line for a note.
pub fn note_line(note: &Note) -> String {
    match note {
        Note::Kept(t) => format!("kept {t}"),
        Note::Skipped(t) => format!("skip {t}"),
        Note::Refused(t) => format!("refused {t}"),
        Note::Info(t) => format!("note {t}"),
    }
}

fn string_array(lines: &[String]) -> Value {
    Value::Array(lines.iter().map(|l| Value::str(l)).collect())
}

/// Prints an outcome in the stable grammar (or as JSON) and returns the exit code: 1 when
/// something failed or a target was refused.
pub fn render(
    outcome: &Outcome,
    dry_run: bool,
    json_out: bool,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> u8 {
    render_with(outcome, dry_run, json_out, true, out, err)
}

/// Like [`render`]; `say_idle` controls the `nothing to do` line of an empty run.
pub fn render_with(
    outcome: &Outcome,
    dry_run: bool,
    json_out: bool,
    say_idle: bool,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> u8 {
    let changes: Vec<String> = outcome
        .applied
        .iter()
        .map(|l| applied_line(l, dry_run))
        .collect();
    let notes: Vec<String> = outcome.notes.iter().map(note_line).collect();
    let refused = outcome.notes.iter().any(|n| matches!(n, Note::Refused(_)));
    let code = u8::from(!outcome.failed.is_empty() || refused);
    if json_out {
        let mut o = Object::new();
        o.insert("v", vibe_core::json::parse("1").unwrap_or(Value::Null));
        o.insert("dry_run", Value::Bool(dry_run));
        o.insert("changes", string_array(&changes));
        o.insert("notes", string_array(&notes));
        o.insert("failed", string_array(&outcome.failed));
        let _ = writeln!(out, "{}", vibe_core::json::to_pretty(&Value::Object(o)));
        return code;
    }
    for line in &changes {
        let _ = writeln!(out, "{line}");
    }
    for line in &notes {
        let _ = writeln!(out, "{line}");
    }
    if say_idle && changes.is_empty() && code == 0 && !dry_run {
        let _ = writeln!(out, "nothing to do");
    }
    for line in &outcome.failed {
        let _ = writeln!(err, "vibe: {}", one_line(line));
    }
    code
}

/// `text` with line breaks flattened, for single-line diagnostics.
pub fn one_line(text: &str) -> String {
    text.lines().map(str::trim).collect::<Vec<_>>().join(" ")
}

/// Reports the result of a locked run: a busy lock, a refusal or the outcome. Returns the
/// exit code.
pub fn finish(
    result: Result<Option<Result<Outcome, String>>, String>,
    dry_run: bool,
    json_out: bool,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> u8 {
    match result {
        Err(e) | Ok(Some(Err(e))) => {
            let refused = Outcome {
                notes: vec![Note::Refused(one_line(&e))],
                ..Outcome::default()
            };
            let mut sink = Vec::new();
            render_with(&refused, false, json_out, false, out, &mut sink);
            1
        }
        Ok(None) => {
            let busy = Outcome {
                notes: vec![Note::Info("another vibe run holds the lock".to_string())],
                ..Outcome::default()
            };
            render_with(&busy, dry_run, json_out, false, out, err);
            1
        }
        Ok(Some(Ok(outcome))) => render(&outcome, dry_run, json_out, out, err),
    }
}
