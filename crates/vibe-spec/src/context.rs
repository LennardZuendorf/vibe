//! Spec storage selection is independent of code checkout selection.
use crate::api::{Execution, SpecError};
use crate::registry::{self, Project, RegistrySnapshot};
use crate::{Runtime, git};
use std::path::{Path, PathBuf};

macro_rules! id {
    ($name:ident) => {
        #[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Hash)]
        pub struct $name(String);
        impl $name {
            pub fn new(value: impl Into<String>) -> Result<Self, SpecError> {
                let value = value.into();
                if value.len() > 128 || !value.as_bytes().first().is_some_and(u8::is_ascii_alphanumeric)
                    || !value.bytes().all(|c| c.is_ascii_alphanumeric() || matches!(c, b'-' | b'_')) {
                    return Err(SpecError::new("invalid_id", "IDs must be opaque ASCII letters, digits, hyphens or underscores, starting with a letter or digit"));
                }
                Ok(Self(value))
            }
            pub fn as_str(&self) -> &str { &self.0 }
        }
        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { f.write_str(&self.0) }
        }
        impl std::str::FromStr for $name {
            type Err = SpecError;
            fn from_str(value: &str) -> Result<Self, Self::Err> { Self::new(value) }
        }
    }
}
id!(ProjectId);
id!(BranchId);
id!(RecordId);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StorageMode {
    RepoFirst,
    GlobalFirst,
}
impl StorageMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::RepoFirst => "repo-first",
            Self::GlobalFirst => "global-first",
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ContextSelector {
    pub project: Option<ProjectId>,
    pub checkout: Option<PathBuf>,
    pub branch: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpecContext {
    pub project_id: Option<ProjectId>,
    pub mode: StorageMode,
    pub checkout: Option<PathBuf>,
    pub code_ref: Option<String>,
    pub code_head: Option<String>,
    pub code_available: bool,
    pub spec_root: PathBuf,
    /// The directory containing `specs/` and `plans/`.
    pub artifact_root: PathBuf,
    pub spec_revision: Option<String>,
    pub revision_token: String,
    pub refresh: crate::refresh::RefreshSummary,
}
impl SpecContext {
    /// Code-sensitive operations must call this before accessing code files.
    pub fn require_code(&self) -> Result<&Path, SpecError> {
        self.checkout
            .as_deref()
            .filter(|_| self.code_available)
            .ok_or_else(|| {
                SpecError::new(
                    "code_context_unavailable",
                    "the selected spec branch has no matching code checkout",
                )
            })
    }
    /// Call under the mutation lock. A checkout switch never retargets a write.
    pub fn ensure_current(
        &self,
        runtime: &Runtime,
        selector: &ContextSelector,
    ) -> Result<SpecContext, SpecError> {
        if let Some(checkout) = &self.checkout {
            let clean = git::context_runtime(runtime);
            let cx = Execution::new(&clean);
            if !checkout.is_dir() || current_ref(&cx, checkout) != self.code_ref {
                return Err(SpecError::new(
                    "context_changed",
                    "the selected checkout or branch changed",
                ));
            }
        }
        let fresh = resolve_locked(runtime, selector)?;
        self.ensure_matches(fresh)
    }
    pub(crate) fn ensure_matches(&self, fresh: SpecContext) -> Result<SpecContext, SpecError> {
        if self.project_id != fresh.project_id
            || self.mode != fresh.mode
            || self.checkout != fresh.checkout
            || self.code_ref != fresh.code_ref
            || self.spec_root != fresh.spec_root
            || self.artifact_root != fresh.artifact_root
        {
            return Err(SpecError::new(
                "context_changed",
                "the selected project, checkout or branch changed",
            ));
        }
        if self.revision_token != fresh.revision_token {
            return Err(SpecError::new(
                "revision_changed",
                "the selected code or spec revision changed",
            ));
        }
        Ok(fresh)
    }
}

pub(crate) fn valid_ref(value: &str) -> bool {
    value.starts_with("refs/heads/")
        && value.len() > "refs/heads/".len()
        && !value.ends_with('.')
        && !value.ends_with('/')
        && !value.contains("..")
        && !value.contains("@{")
        && !value
            .bytes()
            .any(|c| c <= b' ' || c == 127 || b"~^:?*[\\".contains(&c))
        && value
            .split('/')
            .all(|c| !c.is_empty() && !c.starts_with('.') && !c.ends_with(".lock"))
}
pub(crate) fn code_ref(value: &str) -> Result<String, SpecError> {
    let value = if value.starts_with("refs/") {
        value.to_owned()
    } else {
        format!("refs/heads/{value}")
    };
    if !valid_ref(&value) {
        return Err(SpecError::new(
            "invalid_branch",
            "expected a valid named code branch",
        ));
    }
    Ok(value)
}
fn physical(path: &Path) -> Result<PathBuf, SpecError> {
    std::fs::canonicalize(path).map_err(|e| {
        SpecError::new(
            "checkout_unavailable",
            format!("cannot access selected checkout: {e}"),
        )
    })
}
fn equal_path(a: &Path, b: &Path) -> bool {
    match (a.canonicalize(), b.canonicalize()) {
        (Ok(a), Ok(b)) => a == b,
        _ => false,
    }
}
pub(crate) fn current_ref(cx: &Execution<'_>, checkout: &Path) -> Option<String> {
    git::git(cx, checkout, &["symbolic-ref", "-q", "HEAD"])
        .and_then(|v| String::from_utf8(v).ok())
        .map(|s| s.trim_end_matches('\n').to_owned())
}
pub(crate) fn head(cx: &Execution<'_>, checkout: &Path) -> Option<String> {
    git::git(cx, checkout, &["rev-parse", "--verify", "-q", "HEAD"])
        .and_then(|v| String::from_utf8(v).ok())
        .map(|s| s.trim().to_owned())
}
pub(crate) fn common_dir(cx: &Execution<'_>, checkout: &Path) -> Option<PathBuf> {
    git::git(
        cx,
        checkout,
        &["rev-parse", "--path-format=absolute", "--git-common-dir"],
    )
    .and_then(|v| git::output_path(&v))
    .and_then(|p| p.canonicalize().ok())
}

pub(crate) fn checkout_top(
    cx: &Execution<'_>,
    location: &Path,
) -> Result<Option<PathBuf>, SpecError> {
    let Some(top) = git::top(cx, location) else {
        return Ok(None);
    };
    let top = physical(&top)?;
    if !physical(location)?.starts_with(&top) {
        return Err(SpecError::new(
            "context_mismatch",
            "Git top-level does not contain the selected discovery location",
        ));
    }
    Ok(Some(top))
}

/// Task 3 extends this single identity lookup with canonical remote aliases.
pub(crate) fn matching_projects<'a>(
    registry: &'a RegistrySnapshot,
    cx: &Execution<'_>,
    checkout: &Path,
) -> Vec<&'a Project> {
    let common = common_dir(cx, checkout);
    let physical: Vec<_> = registry
        .projects
        .iter()
        .filter(|p| {
            p.checkouts.iter().any(|c| {
                equal_path(&c.path, checkout)
                    || common.as_ref().is_some_and(|dir| {
                        c.common_dir
                            .as_deref()
                            .is_some_and(|registered| equal_path(registered, dir))
                    })
            })
        })
        .collect();
    if !physical.is_empty() {
        return physical;
    }
    let Some(alias) = registry::origin_alias(cx, checkout) else {
        return Vec::new();
    };
    registry
        .projects
        .iter()
        .filter(|p| p.remote_aliases.contains(&alias))
        .collect()
}

fn select_project<'a>(
    registry: &'a RegistrySnapshot,
    cx: &Execution<'_>,
    selector: &ContextSelector,
    checkout: Option<&Path>,
) -> Result<Option<&'a Project>, SpecError> {
    let matches = checkout
        .map(|c| matching_projects(registry, cx, c))
        .unwrap_or_default();
    if let Some(id) = &selector.project {
        let project = registry
            .projects
            .iter()
            .find(|p| p.id == *id)
            .ok_or_else(|| {
                SpecError::new(
                    "project_not_found",
                    "the selected project is not registered",
                )
            })?;
        if selector.checkout.is_some()
            && checkout.is_some()
            && !matches.iter().any(|p| p.id == project.id)
        {
            return Err(SpecError::new(
                "context_mismatch",
                "the selected checkout does not belong to the selected project",
            ));
        }
        return Ok(Some(project));
    }
    match matches.as_slice() {
        [] => Ok(None),
        [p] => Ok(Some(*p)),
        _ => Err(SpecError::new(
            "ambiguous_project",
            "the checkout matches multiple projects",
        )),
    }
}

fn matching_checkout(
    cx: &Execution<'_>,
    project: &Project,
    preferred: Option<&Path>,
    branch: &str,
    explicit_checkout: bool,
) -> Result<Option<PathBuf>, SpecError> {
    if let Some(path) = preferred {
        if current_ref(cx, path).as_deref() == Some(branch) {
            return Ok(Some(path.to_owned()));
        }
        if explicit_checkout {
            return Err(SpecError::new(
                "context_mismatch",
                "the selected checkout is on a different branch",
            ));
        }
    }
    let mut matching = Vec::new();
    for registered in &project.checkouts {
        let Ok(path) = registered.path.canonicalize() else {
            continue;
        };
        if checkout_top(cx, &path)?.is_none() {
            continue;
        }
        // A stale registered path may have been replaced by an unrelated repository.
        if common_dir(cx, &path).is_none_or(|common| {
            registered
                .common_dir
                .as_deref()
                .is_none_or(|registered| !equal_path(&common, registered))
        }) {
            continue;
        }
        if current_ref(cx, &path).as_deref() == Some(branch) && !matching.contains(&path) {
            matching.push(path);
        }
    }
    match matching.len() {
        0 => Ok(None),
        1 => Ok(matching.pop()),
        _ => Err(ambiguous_checkout()),
    }
}

/// Resolve only from explicit inputs. Never change process cwd/env or code Git state.
pub(crate) fn ambiguous_checkout() -> SpecError {
    SpecError::new(
        "ambiguous_checkout",
        "multiple checkouts are on the selected branch",
    )
}

pub fn resolve(runtime: &Runtime, selector: &ContextSelector) -> Result<SpecContext, SpecError> {
    crate::transaction::resolve_context(runtime, selector)
}
/// Already-locked callers use this; it never acquires another project lock.
pub(crate) fn resolve_locked(
    runtime: &Runtime,
    selector: &ContextSelector,
) -> Result<SpecContext, SpecError> {
    let context = resolve_for_transaction(runtime, selector)?;
    crate::transaction::ensure_readable(&context, runtime)?;
    Ok(context)
}
pub(crate) fn resolve_for_transaction(
    runtime: &Runtime,
    selector: &ContextSelector,
) -> Result<SpecContext, SpecError> {
    resolve_selected(
        runtime,
        selector,
        true,
        None,
        CheckoutSelection::MatchingBranch,
    )
}
/// Branch management selects stored refs, using only its original matching code anchor.
pub(crate) fn resolve_branch_storage_locked(
    runtime: &Runtime,
    selector: &ContextSelector,
) -> Result<SpecContext, SpecError> {
    let context = resolve_selected(
        runtime,
        selector,
        true,
        None,
        CheckoutSelection::ManagementAnchor,
    )?;
    crate::transaction::ensure_readable(&context, runtime)?;
    Ok(context)
}
/// Identify managed ownership without requiring a spec root or stored code branch.
pub(crate) fn registered_identity(
    runtime: &Runtime,
    selector: &ContextSelector,
) -> Result<Option<StorageMode>, SpecError> {
    let registry = registry::load(runtime)?;
    if registry.projects.is_empty() && selector.project.is_none() {
        return Ok(None);
    }
    let clean = git::context_runtime(runtime);
    let cx = Execution::new(&clean);
    let location = physical(
        &selector
            .checkout
            .as_ref()
            .map_or_else(|| runtime.cwd.clone(), |p| runtime.cwd.join(p)),
    )?;
    let checkout = checkout_top(&cx, &location)?.or_else(|| {
        registry
            .projects
            .iter()
            .flat_map(|p| p.checkouts.iter())
            .filter_map(|c| c.path.canonicalize().ok())
            .filter(|p| location.starts_with(p))
            .max_by_key(|p| p.components().count())
    });
    Ok(select_project(&registry, &cx, selector, checkout.as_deref())?.map(|p| p.mode))
}
/// Legacy adapter admission only. This tokenless selection must never authorize a managed read/write.
pub(crate) fn resolve_for_routing(
    runtime: &Runtime,
    selector: &ContextSelector,
) -> Result<SpecContext, SpecError> {
    resolve_selected(
        runtime,
        selector,
        false,
        None,
        CheckoutSelection::MatchingBranch,
    )
}
#[derive(Clone, Debug)]
pub(crate) struct LegacyRoots {
    pub code_anchor: PathBuf,
    pub spec_root: PathBuf,
    pub artifact_root: PathBuf,
    pub initialization: bool,
    pub explicit_target: Option<PathBuf>,
}
impl LegacyRoots {
    pub(crate) fn from_root(root: &crate::root::Root) -> Result<Self, SpecError> {
        Ok(Self {
            code_anchor: root.project.clone(),
            spec_root: crate::storage::future_physical(&root.dir)?,
            artifact_root: crate::storage::future_physical(&root.project.join("docs/superpowers"))?,
            initialization: false,
            explicit_target: None,
        })
    }
}
pub(crate) fn resolve_for_legacy(
    runtime: &Runtime,
    selector: &ContextSelector,
    roots: &LegacyRoots,
) -> Result<SpecContext, SpecError> {
    resolve_selected(
        runtime,
        selector,
        true,
        Some(roots),
        CheckoutSelection::MatchingBranch,
    )
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum CheckoutSelection {
    MatchingBranch,
    ManagementAnchor,
}
fn resolve_selected(
    runtime: &Runtime,
    selector: &ContextSelector,
    include_content: bool,
    legacy: Option<&LegacyRoots>,
    selection: CheckoutSelection,
) -> Result<SpecContext, SpecError> {
    if !runtime.cwd.is_absolute() {
        return Err(SpecError::new(
            "invalid_context",
            "runtime cwd must be absolute",
        ));
    }
    let clean = git::context_runtime(runtime);
    let cx = Execution::new(&clean);
    let selected_path = selector.checkout.as_ref().map(|p| runtime.cwd.join(p));
    let mut checkout = if let Some(p) = &selected_path {
        let p = physical(p)?;
        Some(checkout_top(&cx, &p)?.unwrap_or(p))
    } else {
        checkout_top(&cx, &runtime.cwd)?
    };
    checkout = checkout.map(|p| physical(&p)).transpose()?;
    let registry = registry::load(runtime)?;
    if checkout.is_none() {
        let location = physical(&runtime.cwd)?;
        checkout = registry
            .projects
            .iter()
            .filter(|p| p.mode == StorageMode::RepoFirst)
            .flat_map(|p| p.checkouts.iter())
            .filter_map(|c| c.path.canonicalize().ok())
            .filter(|p| location.starts_with(p))
            .max_by_key(|p| p.components().count());
    }
    // Explicit project identity outranks an implicit cwd from another project.
    // An explicitly supplied checkout remains a binding and must match below.
    if selector.checkout.is_none()
        && let (Some(id), Some(ambient)) = (&selector.project, checkout.as_deref())
        && !matching_projects(&registry, &cx, ambient)
            .iter()
            .any(|p| &p.id == id)
    {
        checkout = None;
    }
    let project = select_project(&registry, &cx, selector, checkout.as_deref())?;
    if project.is_none() && selector.project.is_none() && selector.branch.is_some() {
        return Err(SpecError::new(
            "project_required",
            "branch selection requires a registered project",
        ));
    }
    let explicit_branch = selector.branch.as_deref().map(code_ref).transpose()?;
    if let Some(project) = project {
        if selection == CheckoutSelection::MatchingBranch
            && checkout.is_none()
            && explicit_branch.is_none()
        {
            let main = physical(&project.main_checkout)?;
            checkout_top(&cx, &main)?;
            if !matching_projects(&registry, &cx, &main)
                .iter()
                .any(|p| p.id == project.id)
            {
                return Err(SpecError::new(
                    "context_mismatch",
                    "the main checkout no longer belongs to the selected project",
                ));
            }
            checkout = Some(main);
        }
        let branch =
            explicit_branch.or_else(|| checkout.as_deref().and_then(|p| current_ref(&cx, p)));
        let management_anchor = checkout.clone();
        if let Some(branch) = &branch {
            if selection == CheckoutSelection::ManagementAnchor {
                if project.mode != StorageMode::GlobalFirst {
                    return Err(SpecError::new(
                        "unsupported_mode",
                        "stored branch management requires global-first storage",
                    ));
                }
                checkout = checkout.filter(|path| current_ref(&cx, path).as_ref() == Some(branch));
            } else {
                checkout = matching_checkout(
                    &cx,
                    project,
                    checkout.as_deref(),
                    branch,
                    selector.checkout.is_some(),
                )?;
            }
        }
        if project.mode == StorageMode::GlobalFirst {
            let branch = branch.ok_or_else(|| {
                SpecError::new(
                    "named_branch_required",
                    "global specs require a named code branch",
                )
            })?;
            let state = registry::load_project(&registry, &project.id)?;
            let record = state
                .branches
                .iter()
                .find(|b| b.code_ref == branch)
                .ok_or_else(|| {
                    SpecError::new(
                        "branch_not_found",
                        "no spec storage exists for the selected code branch",
                    )
                })?;
            let anchor = registry
                .root
                .join("spec/projects")
                .join(project.id.as_str())
                .join("branches")
                .join(record.id.as_str());
            let spec_root = anchor.join("spec");
            registry::confined(&registry.root, &spec_root)?;
            if !spec_root.is_dir() {
                return Err(SpecError::new(
                    "spec_root_unavailable",
                    "the selected global spec root is missing",
                ));
            }
            let override_anchor = if selection == CheckoutSelection::ManagementAnchor {
                management_anchor.as_deref()
            } else {
                checkout.as_deref()
            };
            check_branch_storage_override(runtime, override_anchor, &spec_root)?;

            let history = registry
                .root
                .join("spec/projects")
                .join(project.id.as_str())
                .join("history.git");
            registry::confined(&registry.root, &history)?;
            let spec_revision = if history.is_dir() {
                git::git(
                    &cx,
                    &history,
                    &["rev-parse", "--verify", "-q", &record.spec_ref],
                )
                .and_then(|v| String::from_utf8(v).ok())
                .map(|s| s.trim().to_owned())
            } else {
                None
            };
            #[cfg(test)]
            crate::transaction::tests::after_spec_oid(&spec_root);
            let artifact_root = crate::superpowers::context_root(StorageMode::GlobalFirst, &anchor);
            registry::confined(&registry.root, &artifact_root)?;
            let code_head = checkout.as_deref().and_then(|p| head(&cx, p));
            let mut result = SpecContext {
                project_id: Some(project.id.clone()),
                mode: project.mode,
                code_available: checkout.is_some(),
                checkout,
                code_ref: Some(branch),
                code_head,
                spec_root,
                artifact_root,
                spec_revision,
                revision_token: String::new(),
                refresh: Default::default(),
            };
            if include_content {
                result.revision_token = token(
                    &result,
                    &[registry.revision.as_bytes(), state.revision.as_bytes()],
                )?;
            }
            return Ok(result);
        }
        if checkout.is_none() {
            return Err(SpecError::new(
                "code_context_unavailable",
                "repo-first branch selection requires a matching code checkout",
            ));
        }
    }
    let origin = checkout
        .as_deref()
        .filter(|_| {
            selector.checkout.is_some() || selector.project.is_some() || selector.branch.is_some()
        })
        .unwrap_or(&runtime.cwd);
    let env_root = Execution::new(runtime)
        .var_os("VIBE_SPEC_ROOT")
        .filter(|v| !v.is_empty())
        .map(|p| runtime.cwd.join(p));
    let root = if let Some(roots) = legacy {
        let mut selected = if let Some(target) = roots.explicit_target.as_ref() {
            crate::root::Root {
                dir: target.clone(),
                project: roots.code_anchor.clone(),
                source: crate::root::Source::Flag,
                depth: 0,
            }
        } else {
            match if roots.initialization {
                crate::root::discover_initialization(
                    origin,
                    env_root.as_deref().map(Path::as_os_str),
                )
            } else {
                crate::root::discover(origin, None, env_root.as_deref().map(Path::as_os_str))
            } {
                Ok(root) => root,
                Err(crate::root::Miss::NotFound) if roots.initialization => {
                    let project = git::top(&cx, origin).unwrap_or_else(|| origin.to_owned());
                    crate::root::Root {
                        dir: project.join("docs/spec"),
                        project,
                        source: crate::root::Source::DocsSpec,
                        depth: 0,
                    }
                }
                Err(miss) => {
                    return Err(SpecError::new(
                        "context_changed",
                        format!("legacy spec discovery changed: {}", miss.message()),
                    ));
                }
            }
        };
        selected.dir = crate::storage::future_physical(&selected.dir)?;
        let artifact = crate::storage::future_physical(&selected.project.join("docs/superpowers"))?;
        if selected.dir != roots.spec_root || artifact != roots.artifact_root {
            return Err(SpecError::new(
                "context_changed",
                "legacy spec or artifact authority changed",
            ));
        }
        selected.project = crate::storage::future_physical(&selected.project)?;
        selected
    } else {
        crate::root::discover_context(origin, env_root.as_deref().map(Path::as_os_str))?
    };
    if legacy.is_some() && checkout.is_none() {
        checkout = Some(root.project.clone());
    }
    let code_head = checkout.as_deref().and_then(|p| head(&cx, p));
    let code_ref = checkout.as_deref().and_then(|p| current_ref(&cx, p));
    // Local repo-first files remain usable without Git or a Git history.
    let checkout = checkout.or_else(|| root.project.is_dir().then(|| root.project.clone()));
    let code_available = checkout.is_some();
    let artifact_root = if legacy.is_some() {
        crate::storage::future_physical(&root.project.join("docs/superpowers"))?
    } else {
        crate::superpowers::context_root(StorageMode::RepoFirst, &root.project)
    };
    let mut result = SpecContext {
        project_id: project.map(|p| p.id.clone()),
        mode: StorageMode::RepoFirst,
        checkout,
        code_ref,
        code_head,
        code_available,
        spec_root: root.dir,
        artifact_root,
        spec_revision: None,
        revision_token: String::new(),
        refresh: Default::default(),
    };
    if include_content {
        result.revision_token = token(&result, &[registry.revision.as_bytes()])?;
    }
    Ok(result)
}

pub(crate) fn check_branch_storage_override(
    runtime: &Runtime,
    anchor: Option<&Path>,
    root: &Path,
) -> Result<(), SpecError> {
    if let Some(anchor) = anchor {
        check_global_override(runtime, anchor, root)
    } else {
        check_global_environment(runtime, root).map(|_| ())
    }
}
fn check_global_environment(runtime: &Runtime, root: &Path) -> Result<bool, SpecError> {
    if let Some(override_root) = Execution::new(runtime)
        .var_os("VIBE_SPEC_ROOT")
        .filter(|v| !v.is_empty())
    {
        if crate::storage::future_physical(&runtime.cwd.join(override_root))?
            != crate::storage::future_physical(root)?
        {
            return Err(SpecError::new(
                "context_mismatch",
                "VIBE_SPEC_ROOT conflicts with registered global storage",
            ));
        }
        return Ok(true);
    }
    Ok(false)
}
pub(crate) fn check_global_override(
    runtime: &Runtime,
    origin: &Path,
    root: &Path,
) -> Result<(), SpecError> {
    if check_global_environment(runtime, root)? {
        return Ok(());
    }
    for dir in origin.ancestors() {
        let config = dir.join(".vibe/spec.json");
        if config.is_file() {
            let configured = crate::root::config_root(&config)
                .map_err(|e| SpecError::new("invalid_config", e.message()))?;
            if let Some(configured) = configured {
                if crate::storage::future_physical(&dir.join(configured))?
                    != crate::storage::future_physical(root)?
                {
                    return Err(SpecError::new(
                        "context_mismatch",
                        "configured spec root conflicts with registered global storage",
                    ));
                }
                break;
            }
        }
        if dir.join(".git").exists() {
            break;
        }
    }
    Ok(())
}

fn framed(out: &mut Vec<u8>, bytes: &[u8]) {
    out.extend_from_slice(&(bytes.len() as u64).to_be_bytes());
    out.extend_from_slice(bytes);
}
fn documents(root: &Path, tag: &[u8], out: &mut Vec<u8>) -> Result<(), SpecError> {
    framed(out, tag);
    registry::confined(root, root)?;
    if !root.exists() {
        framed(out, b"absent");
        return Ok(());
    }
    framed(out, b"root-begin");
    document_entries(root, root, out)?;
    framed(out, b"root-end");
    Ok(())
}
fn document_entries(root: &Path, directory: &Path, out: &mut Vec<u8>) -> Result<(), SpecError> {
    let mut entries = std::fs::read_dir(directory)
        .map_err(|e| SpecError::new("context_unreadable", e.to_string()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| SpecError::new("context_unreadable", e.to_string()))?;
    entries.sort_by_key(|e| e.file_name());
    for entry in entries {
        if entry.file_name() == ".git" {
            continue;
        }
        let path = entry.path();
        let relative = path
            .strip_prefix(root)
            .map_err(|_| SpecError::new("context_unreadable", "document escaped selected root"))?;
        framed(out, relative.as_os_str().as_encoded_bytes());
        let kind = entry
            .file_type()
            .map_err(|e| SpecError::new("context_unreadable", e.to_string()))?;
        if kind.is_symlink() {
            return Err(SpecError::new(
                "context_unreadable",
                "symlink in selected spec documents",
            ));
        }
        if kind.is_dir() {
            framed(out, b"dir-begin");
            document_entries(root, &path, out)?;
            framed(out, b"dir-end");
        } else if kind.is_file() {
            framed(out, b"file");
            framed(
                out,
                &std::fs::read(entry.path())
                    .map_err(|e| SpecError::new("context_unreadable", e.to_string()))?,
            );
        } else {
            return Err(SpecError::new(
                "context_unreadable",
                "non-regular spec document",
            ));
        }
    }
    Ok(())
}
fn token(context: &SpecContext, metadata: &[&[u8]]) -> Result<String, SpecError> {
    let mut bytes = Vec::new();
    for value in [
        context.project_id.as_ref().map(ProjectId::as_str),
        Some(context.mode.as_str()),
        context.code_ref.as_deref(),
        context.code_head.as_deref(),
        context.spec_revision.as_deref(),
    ] {
        framed(&mut bytes, if value.is_some() { b"some" } else { b"none" });
        framed(&mut bytes, value.unwrap_or_default().as_bytes());
    }
    for path in [
        context.checkout.as_deref(),
        Some(context.spec_root.as_path()),
        Some(context.artifact_root.as_path()),
    ] {
        let path = path
            .map(|path| {
                registry::confined(path, path)?;
                crate::storage::future_physical(path)
            })
            .transpose()?;
        framed(&mut bytes, if path.is_some() { b"some" } else { b"none" });
        framed(
            &mut bytes,
            path.as_deref()
                .map(|p| p.as_os_str().as_encoded_bytes())
                .unwrap_or_default(),
        );
    }
    for extra in metadata {
        framed(&mut bytes, extra);
    }
    documents(&context.spec_root, b"spec-root", &mut bytes)?;
    documents(&context.artifact_root, b"artifact-root", &mut bytes)?;
    Ok(format!("v1:{}", vibe_core::markers::hash_bytes(&bytes)))
}

#[cfg(all(test, unix))]
mod identity_tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    #[test]
    fn spec_ci_portability_alias_roots_share_revision_identity() {
        // spec-projects-mcp/2: display spelling cannot split physical authority.
        let owner = vibe_core::testscratch::scratch("spec-context", "alias-identity");
        let code = owner.join("code");
        let alias = owner.join("alias");
        std::fs::create_dir_all(code.join("docs/spec")).unwrap();
        std::fs::create_dir_all(code.join(".git")).unwrap();
        std::fs::write(code.join("docs/spec/tech.md"), b"retained").unwrap();
        std::os::unix::fs::symlink(&code, &alias).unwrap();
        let mut runtime = Runtime {
            cwd: alias.clone(),
            env: std::collections::BTreeMap::from([(
                "VIBE_STORE".into(),
                owner.join("store").into_os_string(),
            )]),
        };
        assert_eq!(
            resolve_for_transaction(&runtime, &ContextSelector::default())
                .unwrap_err()
                .code,
            "invalid_registry"
        );
        runtime.cwd = code.join(".");
        let roots =
            LegacyRoots::from_root(&crate::root::discover(&runtime.cwd, None, None).unwrap())
                .unwrap();
        let ordinary = resolve_for_transaction(&runtime, &ContextSelector::default()).unwrap();
        let legacy = resolve_for_legacy(&runtime, &ContextSelector::default(), &roots).unwrap();
        assert_eq!(
            ordinary.spec_root.as_os_str(),
            runtime.cwd.join("docs/spec").as_os_str()
        );
        assert_eq!(ordinary.revision_token, legacy.revision_token);
        let other = owner.join("other");
        std::fs::create_dir_all(&other).unwrap();
        let mut different = ordinary.clone();
        different.spec_root = other;
        assert_ne!(
            token(&ordinary, &[]).unwrap(),
            token(&different, &[]).unwrap()
        );
    }
}
