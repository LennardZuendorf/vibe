//! `vibe install`: store skeleton, hook registration, OpenCode plugin and the full projection.
use std::collections::BTreeSet;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use vibe_core::dirs::Env;
use vibe_core::json::{self, Object, Value};

use super::common::{self, Loaded};
use super::doctor::find_on_path;
use crate::adapters::{self, Ctx, Mode, enabled_harnesses};
use crate::exec::{self, Outcome};
use crate::paths::{Harness, Paths};
use crate::plan::{Note, OwnedEdit, PlanItem, Projection, json_file_item};
use crate::state::State;
use crate::store::{self, Store, init, skills};

/// What `vibe install` was asked to do.
#[derive(Clone, Debug, Default)]
pub struct InstallOpts {
    /// The harnesses to wire, detected or not; `None` means the detected ones (configuration
    /// directory present or CLI on `PATH`).
    pub harnesses: Option<Vec<Harness>>,
    /// The directory holding vibe's own skills; `None` falls back to `$VIBE_TOOLKIT`, then to
    /// a search around the binary.
    pub toolkit: Option<PathBuf>,
    /// Show the plan without writing.
    pub dry_run: bool,
}

const TOOLKIT_SKILLS: [&str; 3] = ["spec", "vibe", "using-spec"];

fn is_toolkit(dir: &Path) -> bool {
    TOOLKIT_SKILLS
        .iter()
        .all(|name| dir.join(name).join("SKILL.md").is_file())
}

fn resolve(dir: &Path, notes: &mut Vec<Note>) -> Option<PathBuf> {
    if is_toolkit(dir)
        && let Ok(canon) = fs::canonicalize(dir)
    {
        return Some(canon);
    }
    notes.push(Note::Info(format!(
        "toolkit not found at {}",
        dir.display()
    )));
    None
}

/// Finds the toolkit: the flag, else `$VIBE_TOOLKIT` (neither falls back to a search), else
/// the first match among the ancestors of the running binary.
fn find_toolkit(flag: Option<&Path>, env: &Env, notes: &mut Vec<Note>) -> Option<PathBuf> {
    if let Some(dir) = flag {
        return resolve(dir, notes);
    }
    if let Some(dir) = env.var("VIBE_TOOLKIT") {
        return resolve(Path::new(dir), notes);
    }
    let exe = std::env::current_exe().ok()?;
    let exe = fs::canonicalize(&exe).unwrap_or(exe);
    exe.ancestors().skip(1).take(4).find_map(|a| {
        [
            a.to_path_buf(),
            a.join("skills"),
            a.join(".agents").join("skills"),
        ]
        .into_iter()
        .find(|c| is_toolkit(c))
        .and_then(|c| fs::canonicalize(c).ok())
    })
}

fn add_source(obj: &mut Object, toolkit: &str) {
    if obj.get("skills").is_none() {
        obj.insert("skills", Value::Object(Object::new()));
    }
    let Some(skills) = obj.get_mut("skills").and_then(Value::as_object_mut) else {
        return;
    };
    if skills.get("sources").is_none() {
        skills.insert("sources", Value::Array(Vec::new()));
    }
    if let Some(Value::Array(list)) = skills.get_mut("sources") {
        list.push(Value::str(toolkit));
    }
}

/// Plans the `store.json` edit that lists `toolkit`; folded into the pending skeleton write
/// when the file does not exist yet.
fn plan_source(ctx: &Ctx, toolkit: &Path, pending: &mut [PlanItem], out: &mut Projection) {
    let shown = toolkit.display().to_string();
    let path = ctx.paths.store_json();
    for item in pending.iter_mut() {
        if let PlanItem::FileWrite { path: p, bytes, .. } = item
            && *p == path
        {
            let text = String::from_utf8_lossy(bytes).into_owned();
            if let Ok(mut obj) = json::parse_object_or_empty(&text) {
                add_source(&mut obj, &shown);
                *bytes = json::to_pretty(&Value::Object(obj)).into_bytes();
            }
            return;
        }
    }
    out.merge(json_file_item(ctx, Harness::Claude, &path, |obj| {
        add_source(obj, &shown);
    }));
}

fn missing_dirs(path: &Path, seen: &mut BTreeSet<PathBuf>) -> Vec<PathBuf> {
    let Some(parent) = path.parent() else {
        return Vec::new();
    };
    let mut missing: Vec<PathBuf> = parent
        .ancestors()
        .take_while(|a| !a.as_os_str().is_empty() && !a.exists())
        .filter(|a| seen.insert(a.to_path_buf()))
        .map(Path::to_path_buf)
        .collect();
    missing.reverse();
    missing
}

/// Records every directory the file writes will create.
fn record_created_dirs(proj: &mut Projection) {
    let mut seen = BTreeSet::new();
    let mut edits = Vec::new();
    for (i, item) in proj.items.iter().enumerate() {
        if let PlanItem::FileWrite { path, .. } = item {
            for dir in missing_dirs(path, &mut seen) {
                edits.push((
                    i,
                    OwnedEdit::Created {
                        path: dir.display().to_string(),
                    },
                ));
            }
        }
    }
    proj.edits.extend(edits);
}

/// The harnesses `install` wires by default: the configuration directory exists or the CLI
/// (named like the harness id) is on `PATH`.
fn detect(paths: &Paths, env: &Env) -> Vec<Harness> {
    Harness::ALL
        .into_iter()
        .filter(|h| {
            adapters::config_dir(paths, *h).is_dir()
                || find_on_path(h.id(), env.var("PATH")).is_some()
        })
        .collect()
}

struct Planned {
    proj: Projection,
    made_dir: Option<PathBuf>,
    notes: Vec<Note>,
}

/// Plans the toolkit's `store.json` entry and returns the store with the toolkit's skills
/// discovered, so the projection sees them.
fn with_toolkit(
    paths: &Paths,
    state: &State,
    mut store: Store,
    toolkit: &Path,
    bin: &Path,
    proj: &mut Projection,
) -> Store {
    let listed = store
        .config
        .skill_sources
        .iter()
        .any(|s| s == toolkit || fs::canonicalize(s).ok().as_deref() == Some(toolkit));
    if listed {
        return store;
    }
    let json = paths.store_json();
    if fs::symlink_metadata(&json).is_ok_and(|m| m.file_type().is_symlink()) {
        proj.notes.push(Note::Info(format!(
            "toolkit not added: {} is a symlink \u{2014} add {} to skills.sources by hand",
            json.display(),
            toolkit.display()
        )));
        return store;
    }
    let mut source_plan = Projection::default();
    {
        let ctx = Ctx {
            paths,
            store: &store,
            state,
            vibe_bin: bin,
            mode: Mode::Sync,
            harnesses: &[],
        };
        plan_source(&ctx, toolkit, &mut proj.items, &mut source_plan);
    }
    proj.merge(source_plan);
    store.config.skill_sources.push(toolkit.to_path_buf());
    let mut sources = vec![paths.store_skills()];
    sources.extend(store.config.skill_sources.iter().cloned());
    (store.skills, store.shadowed) = skills::discover(&sources);
    store
}

fn plan(
    paths: &Paths,
    opts: &InstallOpts,
    state: &State,
    mut store: Store,
    bin: &Path,
) -> Result<Planned, String> {
    let files = init::init_files(paths, None)?;
    let mut notes = Vec::new();
    let toolkit = find_toolkit(opts.toolkit.as_deref(), &Env::from_process(), &mut notes);
    let mut proj = Projection::default();
    let mut made_dir = None;
    for f in &files {
        let Some(bytes) = &f.bytes else {
            made_dir = Some(f.path.clone());
            continue;
        };
        if f.path == paths.store_agents()
            && let Ok(text) = String::from_utf8(bytes.clone())
            && !text.trim().is_empty()
        {
            store.instructions = Some(text);
        }
        let shown = f.path.display().to_string();
        proj.edits.push((
            proj.items.len(),
            OwnedEdit::Created {
                path: shown.clone(),
            },
        ));
        proj.items.push(PlanItem::FileWrite {
            harness: Harness::Claude,
            show: shown,
            path: f.path.clone(),
            bytes: bytes.clone(),
        });
    }
    if let Some(t) = &toolkit {
        store = with_toolkit(paths, state, store, t, bin, &mut proj);
    }
    let detected = detect(paths, &Env::from_process());
    let filter = opts.harnesses.as_deref().unwrap_or(&detected);
    let harnesses = enabled_harnesses(paths, &store, Some(filter), false);
    if opts.harnesses.is_none() {
        for h in Harness::ALL {
            if !detected.contains(&h) && store.config.enabled.get(&h).copied().unwrap_or(true) {
                notes.push(Note::Info(format!(
                    "{id}: not detected \u{2014} pass --harness {id} to wire it",
                    id = h.id()
                )));
            }
        }
    }
    let loaded = Loaded {
        state: state.clone(),
        store,
    };
    {
        let ctx = Ctx {
            paths,
            store: &loaded.store,
            state: &loaded.state,
            vibe_bin: bin,
            mode: Mode::Sync,
            harnesses: &harnesses,
        };
        for h in harnesses.iter().filter(|h| **h != Harness::Opencode) {
            proj.merge(adapters::plan_hook_install(&ctx, *h));
        }
    }
    proj.merge(common::project_set(
        paths,
        &loaded,
        bin,
        Mode::Sync,
        &harnesses,
    ));
    record_created_dirs(&mut proj);
    notes.extend(loaded.store.notes.iter().cloned().map(Note::Info));
    Ok(Planned {
        proj,
        made_dir,
        notes,
    })
}

fn run(paths: &Paths, opts: &InstallOpts, claude_bin: &str) -> Result<Outcome, String> {
    let Loaded { state, store } = common::load(paths)?;
    let bin = common::vibe_bin()?;
    let Planned {
        mut proj,
        made_dir,
        notes,
    } = plan(paths, opts, &state, store, &bin)?;
    let mut notes = notes;
    if common::in_build_dir(&bin) {
        notes.push(Note::Info(format!(
            "hook binary {} is inside a build directory \u{2014} re-run vibe install after installing vibe on PATH",
            bin.display()
        )));
    }
    if let Some(dir) = &made_dir
        && !opts.dry_run
    {
        fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        proj.edits.push((
            usize::MAX,
            OwnedEdit::Created {
                path: dir.display().to_string(),
            },
        ));
    }
    let (mut outcome, new_state) = exec::apply(paths, proj, state, opts.dry_run, claude_bin);
    if !outcome.failed.is_empty()
        && let Some(dir) = &made_dir
    {
        let _ = fs::remove_dir(dir);
    }
    if !opts.dry_run && outcome.failed.is_empty() && opts.harnesses.is_none() {
        let store = store::load(paths)?;
        let mcp_hash = if outcome.cli_missing {
            new_state.mcp_hash.clone()
        } else {
            common::mcp_hash(paths)
        };
        let stamp = common::stamp(paths, &store, mcp_hash);
        exec::record_stamp(paths, new_state, stamp, &mut outcome);
    }
    outcome.notes.extend(notes);
    Ok(outcome)
}

/// Runs `vibe install`. Exit 1 when something failed, a target was refused or another run
/// holds the lock. A dry run reads only: it takes no lock and creates no directory.
pub fn install(
    paths: &Paths,
    opts: InstallOpts,
    claude_bin: &str,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> u8 {
    let work = || run(paths, &opts, claude_bin);
    let result = if opts.dry_run {
        Ok(Some(work()))
    } else {
        exec::with_lock(paths, work)
    };
    common::finish(result, opts.dry_run, false, out, err)
}
