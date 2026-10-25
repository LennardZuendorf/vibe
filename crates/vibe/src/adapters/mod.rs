//! Adapters: one per harness, plus the shared skills links.
pub mod claude;
pub mod codex;
pub mod factory;
pub mod opencode;
pub(crate) mod shared;
pub mod skills;

use std::path::{Path, PathBuf};

use crate::paths::{Harness, Paths};
use crate::plan::Projection;
use crate::state::State;
use crate::store::Store;

/// What an adapter reads while planning.
pub struct Ctx<'a> {
    /// Every path the program touches.
    pub paths: &'a Paths,
    /// The loaded store.
    pub store: &'a Store,
    /// The state read under the lock.
    pub state: &'a State,
    /// The absolute path of the running binary.
    pub vibe_bin: &'a Path,
    /// Who is planning.
    pub mode: Mode,
    /// The effective harness set: enabled in `store.json`, inside any `--harness` filter,
    /// with a configuration directory when the caller requires one.
    pub harnesses: &'a [Harness],
}

/// The caller of the planner.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// The session hook: files and links only.
    Hook,
    /// `vibe sync`: everything.
    Sync,
}

/// The harness's configuration directory.
pub fn config_dir(paths: &Paths, h: Harness) -> &Path {
    match h {
        Harness::Claude => &paths.claude_dir,
        Harness::Codex => &paths.codex_dir,
        Harness::Opencode => &paths.opencode_dir,
        Harness::Droid => &paths.factory_dir,
    }
}

/// The harnesses to project into: not switched off in `store.json`, in `filter` when given,
/// and (when `require_dir`) with an existing configuration directory.
pub fn enabled_harnesses(
    paths: &Paths,
    store: &Store,
    filter: Option<&[Harness]>,
    require_dir: bool,
) -> Vec<Harness> {
    Harness::ALL
        .into_iter()
        .filter(|h| filter.is_none_or(|f| f.contains(h)))
        .filter(|h| store.config.enabled.get(h).copied().unwrap_or(true))
        .filter(|h| !require_dir || config_dir(paths, *h).is_dir())
        .collect()
}

/// Plans the projection into `h`; shared skills links are not part of it, see [`project_all`].
pub fn project(ctx: &Ctx, h: Harness) -> Projection {
    match h {
        Harness::Claude => claude::project(ctx, h),
        Harness::Codex => codex::project(ctx, h),
        Harness::Opencode => opencode::project(ctx, h),
        Harness::Droid => factory::project(ctx, h),
    }
}

/// The skills directories the effective set needs: the shared hub when any of Codex,
/// OpenCode or Factory is in the set, Claude's own directory when Claude is.
pub fn skill_dirs(ctx: &Ctx) -> Vec<PathBuf> {
    let has = |h: Harness| ctx.harnesses.contains(&h);
    let mut dirs = Vec::new();
    if has(Harness::Codex) || has(Harness::Opencode) || has(Harness::Droid) {
        dirs.push(ctx.paths.agents_skills.clone());
    }
    if has(Harness::Claude) {
        dirs.push(ctx.paths.claude_dir.join("skills"));
    }
    dirs
}

/// Plans every harness of the effective set, then the skills links once.
pub fn project_all(ctx: &Ctx) -> Projection {
    let mut p = Projection::default();
    for h in ctx.harnesses {
        p.merge(project(ctx, *h));
    }
    let dirs = skill_dirs(ctx);
    if !dirs.is_empty() {
        p.merge(skills::project(ctx, &dirs));
    }
    p
}

/// Plans the hook registration for `install` in `h`.
pub fn plan_hook_install(ctx: &Ctx, h: Harness) -> Projection {
    match h {
        Harness::Claude => claude::plan_hook_install(ctx, h),
        Harness::Codex => codex::plan_hook_install(ctx, h),
        Harness::Opencode => opencode::plan_hook_install(ctx, h),
        Harness::Droid => factory::plan_hook_install(ctx, h),
    }
}

/// Plans the removal of everything vibe owns in `h`.
pub fn plan_uninstall(ctx: &Ctx, h: Harness) -> Projection {
    match h {
        Harness::Claude => claude::plan_uninstall(ctx, h),
        Harness::Codex => codex::plan_uninstall(ctx, h),
        Harness::Opencode => opencode::plan_uninstall(ctx, h),
        Harness::Droid => factory::plan_uninstall(ctx, h),
    }
}
