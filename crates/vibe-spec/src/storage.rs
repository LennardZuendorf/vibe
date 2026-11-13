//! Locked project management. Setup stages remain recoverable without adopting foreign content.
use crate::api::Execution;
use crate::registry::{self, Branch, Checkout, Project, ProjectState, RegistrySnapshot};
use crate::{
    AttachProjectRequest, ContextSelector, InitProjectRequest, ProjectId, SpecApi, SpecContext,
    SpecError, SpecResponse, StorageMode,
};
use serde_json::{Value, json};
use std::fs;
use std::path::{Path, PathBuf};

fn error(code: &str, e: impl std::fmt::Display) -> SpecError {
    SpecError::new(code, e.to_string())
}
fn io(e: impl std::fmt::Display) -> SpecError {
    error("storage_write_failed", e)
}
pub(crate) fn context_value(c: &SpecContext) -> Value {
    json!({"project_id":c.project_id.as_ref().map(|p|p.as_str()),"mode":c.mode.as_str(),"checkout":c.checkout.as_deref().map(registry::path_value),"code_ref":c.code_ref,"code_head":c.code_head,"code_available":c.code_available,"spec_root":registry::path_value(&c.spec_root),"artifact_root":registry::path_value(&c.artifact_root),"spec_revision":c.spec_revision,"revision_token":c.revision_token,"refresh":c.refresh.value()})
}
pub(crate) fn projects(api: &SpecApi) -> Result<SpecResponse, SpecError> {
    let registry = registry::load(&api.runtime)?;
    Ok(SpecResponse {
        context: None,
        data: json!({"v":1,"revision":registry.revision,"projects":registry.projects.iter().map(registry::project_value).collect::<Vec<_>>()}),
    })
}
pub(crate) fn selected<'a>(
    api: &SpecApi,
    registry: &'a RegistrySnapshot,
    selector: &ContextSelector,
) -> Result<&'a Project, SpecError> {
    if let Some(id) = &selector.project {
        return registry
            .projects
            .iter()
            .find(|p| p.id == *id)
            .ok_or_else(|| error("project_not_found", "selected project is not registered"));
    }
    let clean = crate::git::context_runtime(&api.runtime);
    let cx = Execution::new(&clean);
    let location = api
        .runtime
        .cwd
        .join(selector.checkout.as_deref().unwrap_or(&api.runtime.cwd));
    let location = fs::canonicalize(location).map_err(|e| error("checkout_unavailable", e))?;
    let checkout = crate::context::checkout_top(&cx, &location)?.unwrap_or(location);
    match crate::context::matching_projects(registry, &cx, &checkout).as_slice() {
        [project] => Ok(*project),
        [] => Err(error("project_not_found", "checkout is not registered")),
        _ => Err(error(
            "ambiguous_project",
            "checkout matches multiple projects",
        )),
    }
}
pub(crate) fn show(api: &SpecApi, selector: &ContextSelector) -> Result<SpecResponse, SpecError> {
    let registry = registry::load(&api.runtime)?;
    let project = selected(api, &registry, selector)?;
    let state = registry::load_project(&registry, &project.id)?;
    Ok(SpecResponse {
        context: None,
        data: json!({"v":1,"revision":registry.revision,"project_revision":state.revision,"project":registry::project_value(project),"branches":state.branches.iter().map(registry::branch_value).collect::<Vec<_>>()}),
    })
}
/// Normalize the existing ancestor before effects, including a not-yet-created store.
pub(crate) fn future_physical(path: &Path) -> Result<PathBuf, SpecError> {
    let path = crate::init::normalise(path);
    let mut suffix = Vec::new();
    let mut current = path.as_path();
    loop {
        match fs::canonicalize(current) {
            Ok(mut p) => {
                for part in suffix.iter().rev() {
                    p.push(part);
                }
                return Ok(p);
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                let name = current.file_name().ok_or_else(|| io(&e))?;
                suffix.push(name.to_owned());
                current = current.parent().ok_or_else(|| io(e))?;
            }
            Err(e) => return Err(io(e)),
        }
    }
}
pub(crate) fn admit_global_root(api: &SpecApi) -> Result<PathBuf, SpecError> {
    let root = registry::store_root(&api.runtime)?;
    let physical = future_physical(&root)?;
    let mut existing = physical.as_path();
    while !existing.exists() {
        existing = existing
            .parent()
            .ok_or_else(|| io("missing storage ancestor"))?;
    }
    let clean = crate::git::context_runtime(&api.runtime);
    let cx = Execution::new(&clean);
    let mut enclosing = None;
    for ancestor in existing.ancestors() {
        if let Some(checkout) = crate::context::checkout_top(&cx, ancestor)? {
            enclosing = Some(checkout);
            break;
        }
    }
    // These reads only admit the storage location. Identity lookup stays under the registry lock.
    let registry = registry::load(&api.runtime)?;
    if enclosing
        .as_ref()
        .is_some_and(|code| physical.starts_with(code))
        || registry
            .projects
            .iter()
            .flat_map(|p| &p.checkouts)
            .any(|c| {
                c.path
                    .canonicalize()
                    .is_ok_and(|code| physical.starts_with(code))
            })
    {
        return Err(error(
            "store_inside_checkout",
            "configured global store is physically inside a code checkout",
        ));
    }
    registry::confined(&root, &root.join("spec/registry.json"))?;
    Ok(root)
}
pub(crate) fn admit_store(api: &SpecApi, checkout: &Path) -> Result<PathBuf, SpecError> {
    let root = admit_global_root(api)?;
    if future_physical(&root)?.starts_with(checkout) {
        return Err(error(
            "store_inside_checkout",
            "configured global store is physically inside the code checkout",
        ));
    }
    Ok(root)
}

fn code_selection(
    api: &SpecApi,
    selector: &ContextSelector,
) -> Result<(crate::Runtime, PathBuf), SpecError> {
    if selector.branch.is_some() {
        return Err(error(
            "invalid_context",
            "project setup selects a checkout, not a stored branch",
        ));
    }
    let clean = crate::git::context_runtime(&api.runtime);
    let cx = Execution::new(&clean);
    let location = fs::canonicalize(
        api.runtime
            .cwd
            .join(selector.checkout.as_deref().unwrap_or(&api.runtime.cwd)),
    )
    .map_err(|e| error("checkout_unavailable", e))?;
    let checkout = crate::context::checkout_top(&cx, &location)?.unwrap_or(location);
    Ok((clean, checkout))
}
fn add_checkout(project: &mut Project, cx: &Execution<'_>, checkout: &Path, main: bool) -> bool {
    let mut changed = false;
    let common_dir = crate::context::common_dir(cx, checkout);
    if let Some(existing) = project.checkouts.iter_mut().find(|c| c.path == checkout) {
        if existing.common_dir != common_dir {
            existing.common_dir = common_dir;
            changed = true;
        }
    } else {
        project.checkouts.push(Checkout {
            path: checkout.into(),
            common_dir,
        });
        changed = true;
    }
    if main && project.main_checkout != checkout {
        project.main_checkout = checkout.into();
        changed = true;
    }
    if let Some(alias) = registry::origin_alias(cx, checkout)
        && !project.remote_aliases.contains(&alias)
    {
        project.remote_aliases.push(alias);
        project.remote_aliases.sort();
        changed = true;
    }
    changed
}
fn managed_response(
    api: &SpecApi,
    registry: &RegistrySnapshot,
    id: &ProjectId,
    selector: ContextSelector,
) -> Result<SpecResponse, SpecError> {
    let project = registry
        .projects
        .iter()
        .find(|p| p.id == *id)
        .ok_or_else(|| error("project_not_found", "project registration missing"))?;
    let state = registry::load_project(registry, id)?;
    let mut data = json!({"v":1,"revision":registry.revision,"project_revision":state.revision,"project":registry::project_value(project),"branches":state.branches.iter().map(registry::branch_value).collect::<Vec<_>>()});
    let prefix = data.clone();
    let projected = api.project_results(move |candidate| {
        let context = candidate.get("context").filter(|v| v.is_object())?;
        let mut data = prefix.clone();
        data["context"] = context.clone();
        Some(json!({"context":context,"data":data}))
    });
    // The project guard has dropped. Any new refresh publication must admit the
    // complete management result, including the metadata returned around Context.
    let context = projected.resolve_context(&selector)?;
    data["context"] = context_value(&context);
    Ok(SpecResponse {
        data,
        context: Some(context),
    })
}
struct CodeObservation {
    checkout: PathBuf,
    common_dir: Option<PathBuf>,
    code_ref: Option<String>,
    code_head: Option<String>,
}
impl CodeObservation {
    fn capture(cx: &Execution<'_>, checkout: &Path) -> Result<Self, SpecError> {
        let physical = fs::canonicalize(checkout).map_err(|e| error("context_changed", e))?;
        let top = crate::context::checkout_top(cx, checkout)?;
        if top.as_ref().is_some_and(|top| top != &physical) {
            return Err(error("context_changed", "code checkout identity changed"));
        }
        let code_ref = crate::context::current_ref(cx, checkout);
        let code_head = crate::context::head(cx, checkout);
        let common_dir = crate::context::common_dir(cx, checkout);
        if crate::context::current_ref(cx, checkout) != code_ref {
            return Err(error(
                "context_changed",
                "code branch changed while observing setup context",
            ));
        }
        Ok(Self {
            checkout: physical,
            common_dir,
            code_ref,
            code_head,
        })
    }
    fn revalidate(&self, cx: &Execution<'_>, checkout: &Path) -> Result<(), SpecError> {
        let fresh = Self::capture(cx, checkout)?;
        if self.checkout != fresh.checkout
            || self.common_dir != fresh.common_dir
            || self.code_ref != fresh.code_ref
        {
            return Err(error(
                "context_changed",
                "code checkout or branch changed during setup",
            ));
        }
        if self.code_head != fresh.code_head {
            return Err(error("revision_changed", "code head changed during setup"));
        }
        Ok(())
    }
}
pub(crate) fn init_project(
    api: &SpecApi,
    request: InitProjectRequest,
) -> Result<SpecResponse, SpecError> {
    api.check_control()?;
    let mut _published = None;
    let (clean, checkout) = code_selection(api, &request.selector)?;
    let cx = Execution::new(&clean);
    let root = admit_store(api, &checkout)?;
    // Detect committed/local roots and invalid configuration before lock creation.
    if request.mode == StorageMode::GlobalFirst {
        match crate::root::discover_context(&checkout, None) {
            Ok(root) if future_physical(&root.dir)?.starts_with(&checkout) => {
                return Err(error(
                    "mode_migration_required",
                    "existing repository specs require explicit mode migration; setup leaves them untouched",
                ));
            }
            Err(e) if e.code == "invalid_config" => return Err(e),
            _ => (),
        }
    }
    let registry_lock = root.join("spec/registry.lock");
    let _registry_lock = api.lock_storage(&root, &registry_lock)?;
    let mut registry = registry::load(&api.runtime)?;
    registry::expect(&registry.revision, &request.expected_revision)?;
    let observed = CodeObservation::capture(&cx, &checkout)?;
    let branch = observed.code_ref.clone();
    if request.mode == StorageMode::GlobalFirst && branch.is_none() {
        return Err(error(
            "named_branch_required",
            "global-first setup requires a named Git branch",
        ));
    }
    let matches = crate::context::matching_projects(&registry, &cx, &checkout);
    if matches.len() > 1 {
        return Err(error(
            "ambiguous_project",
            "checkout matches multiple projects",
        ));
    }
    let existing = if let Some(id) = &request.selector.project {
        let project = registry
            .projects
            .iter()
            .find(|p| p.id == *id)
            .ok_or_else(|| error("project_not_found", "selected project is not registered"))?;
        if !matches.iter().any(|p| p.id == project.id) {
            return Err(error(
                "context_mismatch",
                "use project attach to admit an unrelated checkout",
            ));
        }
        Some(project.id.clone())
    } else {
        matches.first().map(|p| p.id.clone())
    };
    let id = if let Some(id) = existing {
        let project = registry
            .projects
            .iter_mut()
            .find(|p| p.id == id)
            .ok_or_else(|| io("missing project"))?;
        if project.mode != request.mode {
            return Err(error(
                "mode_migration_required",
                "project storage mode cannot change during setup",
            ));
        }
        let project_dir = registry::project_dir(&registry, &id);
        let _project_lock = api.lock_storage(&root, &project_dir.join("project.lock"))?;
        observed.revalidate(&cx, &checkout)?;
        api.check_control()?;
        let project = registry
            .projects
            .iter_mut()
            .find(|p| p.id == id)
            .ok_or_else(|| io("missing project"))?;
        let changed = add_checkout(project, &cx, &checkout, false);
        let state = registry::load_project(&registry, &id)?;
        admit_management(
            api,
            &registry,
            registry
                .projects
                .iter()
                .find(|p| p.id == id)
                .ok_or_else(|| io("project missing"))?,
            &state,
            &checkout,
            true,
        )?;
        if request.mode == StorageMode::GlobalFirst {
            let code_ref = branch
                .as_ref()
                .ok_or_else(|| error("named_branch_required", "named branch required"))?;
            let record = state
                .branches
                .iter()
                .find(|b| &b.code_ref == code_ref)
                .ok_or_else(|| {
                    error(
                        "branch_not_found",
                        "create this branch explicitly with branch init --from BASE",
                    )
                })?;
            let spec_root = project_dir
                .join("branches")
                .join(record.id.as_str())
                .join("spec");
            crate::context::check_global_override(&api.runtime, &checkout, &spec_root)?;
            ensure_existing(&api.runtime, &registry, &id, record)?;
        } else {
            let target = repo_root(&api.runtime, &checkout)?;
            api.check_control()?;
            observed.revalidate(&cx, &checkout)?;
            api.check_control()?;
            _published = api.completing();
            crate::init::seed(&checkout, &target)?;
        }
        observed.revalidate(&cx, &checkout)?;
        if changed {
            verify_lock(&registry_lock)?;
            api.check_control()?;
            registry::write_registry(&registry)?;
            _published = api.completing();
        }
        id
    } else {
        let alias = registry::origin_alias(&cx, &checkout);
        let identity = alias
            .as_ref()
            .map(|a| a.as_bytes())
            .unwrap_or(checkout.as_os_str().as_encoded_bytes());
        let id = ProjectId::new(format!("p-{}", vibe_core::markers::hash_bytes(identity)))?;
        if registry.projects.iter().any(|p| p.id == id) {
            return Err(error(
                "project_id_collision",
                "project storage identity collides with another registration",
            ));
        }
        let project = Project {
            id: id.clone(),
            name: request.name.clone().unwrap_or_else(|| {
                checkout
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned()
            }),
            mode: request.mode,
            main_checkout: checkout.clone(),
            checkouts: vec![Checkout {
                path: checkout.clone(),
                common_dir: crate::context::common_dir(&cx, &checkout),
            }],
            remote_aliases: alias.into_iter().collect(),
        };
        if project.name.trim().is_empty() {
            return Err(error("invalid_name", "project name must not be empty"));
        }
        let project_dir = registry::project_dir(&registry, &id);
        registry::confined(&root, &project_dir)?;
        let _project_lock = api.lock_storage(&root, &project_dir.join("project.lock"))?;
        observed.revalidate(&cx, &checkout)?;
        api.check_control()?;
        let record = branch
            .as_ref()
            .map(|code_ref| {
                new_branch(
                    code_ref,
                    None,
                    observed.code_head.clone(),
                    observed.code_ref.clone(),
                )
            })
            .transpose()?;
        if request.mode == StorageMode::GlobalFirst {
            let record = record
                .as_ref()
                .ok_or_else(|| error("named_branch_required", "named branch required"))?;
            let spec_root = project_dir
                .join("branches")
                .join(record.id.as_str())
                .join("spec");
            crate::context::check_global_override(&api.runtime, &checkout, &spec_root)?;
        }
        let state = ProjectState {
            branches: record.clone().into_iter().collect(),
            snapshots: Vec::new(),
            observations: Vec::new(),
            revision: "absent".into(),
        };
        admit_management(api, &registry, &project, &state, &checkout, true)?;
        let stage = project_dir.join("setup.json");
        let planned = json!({"v":1,"project":registry::project_value(&project),"branch":record.as_ref().map(registry::branch_value)});
        if stage.exists() {
            registry::confined(&root, &stage)?;
            let actual: Value = serde_json::from_slice(&fs::read(&stage).map_err(io)?)
                .map_err(|e| error("invalid_registry", e))?;
            if actual != planned {
                return Err(error(
                    "setup_conflict",
                    "interrupted setup has different admitted inputs; preserve it for manual recovery",
                ));
            }
        } else {
            if project_dir.join("project.json").exists()
                || project_dir.join("history.git").exists()
                || project_dir.join("branches").exists()
            {
                return Err(error(
                    "setup_conflict",
                    "unowned project content exists without a setup record",
                ));
            }
            api.check_control()?;
            registry::write_json(&root, &stage, &planned)?;
        }
        if project_dir.join("project.json").exists() {
            let existing = registry::load_project(&registry, &id).map_err(|e| {
                error(
                    "setup_conflict",
                    format!("interrupted setup metadata is invalid; preserve it for recovery: {e}"),
                )
            })?;
            if existing
                .branches
                .iter()
                .map(registry::branch_value)
                .collect::<Vec<_>>()
                != state
                    .branches
                    .iter()
                    .map(registry::branch_value)
                    .collect::<Vec<_>>()
                || existing.snapshots != state.snapshots
                || existing.observations != state.observations
            {
                return Err(error(
                    "setup_conflict",
                    "project metadata diverged from interrupted setup; preserve it for recovery",
                ));
            }
        }
        if request.mode == StorageMode::GlobalFirst {
            let record = record
                .as_ref()
                .ok_or_else(|| error("named_branch_required", "named branch required"))?;
            initialize_global(api, &registry, &id, record, None)?;
            _published = api.completing();
        } else {
            let target = repo_root(&api.runtime, &checkout)?;
            api.check_control()?;
            observed.revalidate(&cx, &checkout)?;
            api.check_control()?;
            _published = api.completing();
            crate::init::seed(&checkout, &target)?;
        }
        observed.revalidate(&cx, &checkout)?;
        verify_lock(&project_dir.join("project.lock"))?;
        registry::write_project(&registry, &id, &state)?;
        registry.projects.push(project);
        observed.revalidate(&cx, &checkout)?;
        verify_lock(&registry_lock)?;
        api.check_control()?;
        registry::write_registry(&registry)?;
        _published = api.completing();
        id
    };
    let registry = registry::load(&api.runtime)?;
    // Registration bookkeeping is complete. Context refresh is a separate effect.
    drop(_published);
    managed_response(
        api,
        &registry,
        &id,
        ContextSelector {
            project: Some(id.clone()),
            checkout: Some(checkout),
            branch: None,
        },
    )
}
fn repo_root(runtime: &crate::Runtime, checkout: &Path) -> Result<PathBuf, SpecError> {
    let env = Execution::new(runtime)
        .var_os("VIBE_SPEC_ROOT")
        .map(|p| runtime.cwd.join(p));
    let target = match crate::root::discover(checkout, None, env.as_deref().map(Path::as_os_str)) {
        Ok(root) => root.dir,
        Err(crate::root::Miss::NotFound) => checkout.join("docs/spec"),
        Err(crate::root::Miss::Missing { path, .. }) => path,
        Err(e) => return Err(error("invalid_config", e.message())),
    };
    if !target.starts_with(checkout) || target == checkout {
        return Err(error(
            "unsafe_path",
            "repo-first spec root must be below the code project",
        ));
    }
    Ok(target)
}
pub(crate) fn verify_lock(path: &Path) -> Result<(), SpecError> {
    vibe_core::lock::verify_path(path).map_err(|e| error("lock_lost", e))
}
pub(crate) fn new_branch(
    code_ref: &str,
    parent: Option<String>,
    code_head: Option<String>,
    observed_code_ref: Option<String>,
) -> Result<Branch, SpecError> {
    let id = crate::BranchId::new(format!(
        "b-{}",
        vibe_core::markers::hash_bytes(code_ref.as_bytes())
    ))?;
    Ok(Branch {
        id: id.clone(),
        code_ref: code_ref.into(),
        spec_ref: format!("refs/heads/spec/{id}"),
        parent_spec_revision: parent,
        last_code_head: code_head,
        last_code_ref: observed_code_ref,
    })
}
pub(crate) fn attach_project(
    api: &SpecApi,
    request: AttachProjectRequest,
) -> Result<SpecResponse, SpecError> {
    api.check_control()?;
    let mut _published = None;
    let (clean, checkout) = code_selection(
        api,
        &ContextSelector {
            checkout: request.checkout,
            ..Default::default()
        },
    )?;
    let cx = Execution::new(&clean);
    let root = admit_store(api, &checkout)?;
    let lock_path = root.join("spec/registry.lock");
    let _registry_lock = api.lock_storage(&root, &lock_path)?;
    let mut registry = registry::load(&api.runtime)?;
    registry::expect(&registry.revision, &request.expected_revision)?;
    let matches = crate::context::matching_projects(&registry, &cx, &checkout);
    if matches.iter().any(|p| p.id != request.project) {
        return Err(error(
            "context_mismatch",
            "checkout already belongs to another project",
        ));
    }
    let project = registry
        .projects
        .iter_mut()
        .find(|p| p.id == request.project)
        .ok_or_else(|| error("project_not_found", "selected project is not registered"))?;
    if project.mode == StorageMode::GlobalFirst
        && crate::context::common_dir(&cx, &checkout).is_none()
    {
        return Err(error(
            "named_branch_required",
            "global project attachment requires a Git checkout",
        ));
    }
    let changed = add_checkout(project, &cx, &checkout, request.main);
    let project_dir = registry::project_dir(&registry, &request.project);
    let _project_lock = api.lock_storage(&root, &project_dir.join("project.lock"))?;
    let state = registry::load_project(&registry, &request.project)?;
    let planned = registry
        .projects
        .iter()
        .find(|p| p.id == request.project)
        .ok_or_else(|| io("project missing"))?;
    admit_management(api, &registry, planned, &state, &checkout, false)?;
    if changed {
        verify_lock(&lock_path)?;
        api.check_control()?;
        registry::write_registry(&registry)?;
        _published = api.completing();
    }
    let registry = registry::load(&api.runtime)?;
    show(
        api,
        &ContextSelector {
            project: Some(request.project.clone()),
            ..Default::default()
        },
    )
    .map(|mut result| {
        result.data["revision"] = registry.revision.into();
        result
    })
}

const HISTORY_CONFIG: &[u8] =
    b"[core]\n\trepositoryformatversion = 0\n\tbare = true\n\tfilemode = false\n";
/// Private Git owns no caller-provided locators/configuration and is always bound
/// to the admitted history plus (when needed) its exact persistent worktree.
pub(crate) fn spec_git(
    runtime: &crate::Runtime,
    history: &Path,
    worktree: Option<&Path>,
    args: &[std::ffi::OsString],
) -> Result<Vec<u8>, SpecError> {
    spec_git_input(runtime, history, worktree, args, None)
}
pub(crate) fn spec_git_input(
    runtime: &crate::Runtime,
    history: &Path,
    worktree: Option<&Path>,
    args: &[std::ffi::OsString],
    input: Option<&[u8]>,
) -> Result<Vec<u8>, SpecError> {
    let out = spec_git_output(runtime, history, worktree, args, input)?;
    if !out.status.success() {
        return Err(error(
            "spec_git_failed",
            format!(
                "managed Git command failed: {}",
                String::from_utf8_lossy(&out.stderr)
            ),
        ));
    }
    Ok(out.stdout)
}
/// Structured exit status: callers distinguish a Git conflict from process failure.
pub(crate) fn spec_git_output(
    runtime: &crate::Runtime,
    history: &Path,
    worktree: Option<&Path>,
    args: &[std::ffi::OsString],
    input: Option<&[u8]>,
) -> Result<std::process::Output, SpecError> {
    let project = history
        .parent()
        .ok_or_else(|| io("history has no project anchor"))?;
    inspect_tree(project, history)?;
    if fs::read(history.join("config")).map_err(io)? != HISTORY_CONFIG {
        return Err(error(
            "invalid_storage",
            "managed history configuration changed; preserve it for manual recovery",
        ));
    }
    for rel in [
        "objects/info/alternates",
        "objects/info/http-alternates",
        "info/grafts",
    ] {
        if history.join(rel).exists() {
            return Err(error(
                "invalid_storage",
                "external object or replacement history is not admitted",
            ));
        }
    }
    let mut clean = crate::git::context_runtime(runtime);
    clean.cwd = project.into();
    clean.env.retain(|key, _| {
        !key.to_string_lossy()
            .to_ascii_uppercase()
            .starts_with("GIT_")
    });
    let empty = project.join("git-empty-config");
    registry::confined(project, &empty)?;
    registry::confined(project, &project.join("disabled-hooks"))?;
    if fs::read_dir(project.join("disabled-hooks"))
        .map_err(io)?
        .next()
        .is_some()
    {
        return Err(error(
            "invalid_storage",
            "disabled hooks directory must remain empty",
        ));
    }
    if fs::read(&empty).map_err(io)? != b"" {
        return Err(error(
            "invalid_storage",
            "managed empty Git configuration changed",
        ));
    }
    for (key, value) in [
        ("GIT_CONFIG_NOSYSTEM", std::ffi::OsString::from("1")),
        (
            "GIT_CONFIG_GLOBAL",
            crate::git::argument_path(&empty).into_os_string(),
        ),
        (
            "GIT_CONFIG_SYSTEM",
            crate::git::argument_path(&empty).into_os_string(),
        ),
        ("GIT_ATTR_NOSYSTEM", "1".into()),
        ("GIT_TERMINAL_PROMPT", "0".into()),
        ("GIT_ALLOW_PROTOCOL", "".into()),
        ("GIT_NO_REPLACE_OBJECTS", "1".into()),
        ("GIT_AUTHOR_DATE", "2000-01-01T00:00:00Z".into()),
        ("GIT_COMMITTER_DATE", "2000-01-01T00:00:00Z".into()),
    ] {
        clean.env.insert(key.into(), value);
    }
    let cx = Execution::new(&clean);
    let mut command = cx.command("git");
    let gitdir = if let Some(worktree) = worktree {
        bind_worktree(history, worktree)?;
        worktree_gitdir(history, worktree)?
    } else {
        history.to_path_buf()
    };
    command
        .arg("--git-dir")
        .arg(crate::git::argument_path(&gitdir));
    if let Some(worktree) = worktree {
        command
            .arg("--work-tree")
            .arg(crate::git::argument_path(worktree))
            .current_dir(worktree);
    }
    command.args([
        "--literal-pathspecs",
        "-c",
        "core.fsmonitor=false",
        "-c",
        "commit.gpgsign=false",
        "-c",
        "tag.gpgsign=false",
        "-c",
        "user.name=vibe spec",
        "-c",
        "user.email=spec@vibe.local",
        "-c",
        "protocol.allow=never",
        "-c",
        "core.autocrlf=false",
        "-c",
        "core.safecrlf=false",
    ]);
    let mut hooks = std::ffi::OsString::from("core.hooksPath=");
    hooks.push(crate::git::argument_path(&project.join("disabled-hooks")));
    let mut attributes = std::ffi::OsString::from("core.attributesFile=");
    attributes.push(crate::git::argument_path(&empty));
    command
        .arg("-c")
        .arg(hooks)
        .arg("-c")
        .arg(attributes)
        .args(args);
    let out = if let Some(input) = input {
        use std::io::Write;
        let mut child = command
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(io)?;
        let written = child
            .stdin
            .take()
            .ok_or_else(|| io("missing Git input pipe"))?
            .write_all(input);
        let out = child.wait_with_output().map_err(io)?;
        written.map_err(io)?;
        out
    } else {
        command
            .stdin(std::process::Stdio::null())
            .output()
            .map_err(io)?
    };
    Ok(out)
}

fn args(values: &[&str]) -> Vec<std::ffi::OsString> {
    values.iter().map(std::ffi::OsString::from).collect()
}
pub(crate) fn git_text(
    runtime: &crate::Runtime,
    history: &Path,
    worktree: Option<&Path>,
    values: &[&str],
) -> Result<String, SpecError> {
    let bytes = spec_git(runtime, history, worktree, &args(values))?;
    String::from_utf8(bytes)
        .map(|s| s.trim().into())
        .map_err(io)
}
pub(crate) fn snapshot(
    api: &SpecApi,
    request: crate::SnapshotRequest,
) -> Result<SpecResponse, SpecError> {
    crate::transaction::snapshot(api, &request.selector, &request.expected_revision)
}
fn inspect_tree(anchor: &Path, path: &Path) -> Result<(), SpecError> {
    registry::confined(anchor, path)?;
    if !path.exists() {
        return Ok(());
    }
    if path.is_dir() {
        for item in fs::read_dir(path).map_err(io)? {
            inspect_tree(anchor, &item.map_err(io)?.path())?;
        }
    }
    Ok(())
}
fn worktree_gitdir(history: &Path, worktree: &Path) -> Result<PathBuf, SpecError> {
    registry::confined(
        history
            .parent()
            .ok_or_else(|| io("missing project anchor"))?,
        &worktree.join(".git"),
    )?;
    let bytes = fs::read(worktree.join(".git")).map_err(io)?;
    let path = bytes
        .strip_prefix(b"gitdir: ")
        .and_then(crate::git::output_path)
        .ok_or_else(|| error("invalid_storage", "invalid spec worktree Git binding"))?;
    let path = fs::canonicalize(path).map_err(io)?;
    let history = fs::canonicalize(history).map_err(io)?;
    if path.parent() != Some(history.join("worktrees").as_path()) {
        return Err(error(
            "invalid_storage",
            "spec worktree points outside its admitted history",
        ));
    }
    registry::confined(&history, &path)?;
    let common = fs::read(path.join("commondir")).map_err(io)?;
    let common = crate::git::output_path(&common)
        .ok_or_else(|| error("invalid_storage", "invalid worktree common directory"))?;
    if fs::canonicalize(path.join(common)).map_err(io)? != history {
        return Err(error(
            "invalid_storage",
            "spec worktree common directory changed",
        ));
    }
    let reverse = crate::git::output_path(&fs::read(path.join("gitdir")).map_err(io)?)
        .ok_or_else(|| error("invalid_storage", "invalid reverse worktree binding"))?;
    if fs::canonicalize(reverse).map_err(io)?
        != fs::canonicalize(worktree.join(".git")).map_err(io)?
    {
        return Err(error(
            "invalid_storage",
            "spec worktree reverse binding changed",
        ));
    }
    Ok(path)
}
fn bind_worktree(history: &Path, worktree: &Path) -> Result<(), SpecError> {
    worktree_gitdir(history, worktree).map(|_| ())
}
/// An admission-only detached tree. A crash leaves unique scratch that is never
/// adopted. Cleanup removes only this owner and its exact reverse-bound entry.
pub(crate) struct AdmissionWorktree {
    root: PathBuf,
    root_metadata: fs::Metadata,
    history: PathBuf,
    binding: Option<(PathBuf, fs::Metadata, fs::Metadata)>,
    cleaned: bool,
    #[cfg(unix)]
    _root_handle: fs::File,
}
fn same_directory(path: &Path, original: &fs::Metadata) -> bool {
    fs::symlink_metadata(path).is_ok_and(|metadata| {
        if !metadata.is_dir() || metadata.file_type().is_symlink() {
            return false;
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            (metadata.dev(), metadata.ino()) == (original.dev(), original.ino())
        }
        #[cfg(not(unix))]
        {
            original
                .created()
                .ok()
                .is_some_and(|time| metadata.created().ok() == Some(time))
        }
    })
}

impl AdmissionWorktree {
    pub(crate) fn new(api: &SpecApi, history: &Path, target: &str) -> Result<Self, SpecError> {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let project = history
            .parent()
            .ok_or_else(|| io("missing project anchor"))?;
        let root = loop {
            let path = project.join(format!(
                ".response-admission-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            registry::confined(project, &path)?;
            let builder = fs::DirBuilder::new();
            #[cfg(unix)]
            let builder = {
                use std::os::unix::fs::DirBuilderExt;
                let mut builder = builder;
                builder.mode(0o700);
                builder
            };
            match builder.create(&path) {
                Ok(()) => break path,
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(e) => return Err(io(e)),
            }
        };
        let mut owner = Self {
            root_metadata: fs::symlink_metadata(&root).map_err(io)?,
            #[cfg(unix)]
            _root_handle: fs::File::open(&root).map_err(io)?,
            root,
            history: history.into(),
            binding: None,
            cleaned: false,
        };
        fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(owner.root.join("owner"))
            .map_err(io)?;
        api.check_control()?;
        spec_git(
            &api.runtime,
            history,
            None,
            &[
                "worktree".into(),
                "add".into(),
                "--detach".into(),
                crate::git::argument_path(&owner.path()).into_os_string(),
                target.into(),
            ],
        )?;
        let binding = worktree_gitdir(history, &owner.path())?;
        owner.binding = Some((
            binding.clone(),
            fs::symlink_metadata(binding).map_err(io)?,
            fs::symlink_metadata(owner.path()).map_err(io)?,
        ));
        Ok(owner)
    }
    pub(crate) fn path(&self) -> PathBuf {
        self.root.join("workspace")
    }
    pub(crate) fn save_proof(&self, value: &Value) -> Result<(), SpecError> {
        registry::write_json(&self.root, &self.root.join("proof.json"), value)
    }
    pub(crate) fn finish(mut self) -> Result<(), SpecError> {
        self.cleanup()
    }
    fn cleanup(&mut self) -> Result<(), SpecError> {
        if self.cleaned {
            return Ok(());
        }
        let changed = || {
            error(
                "invalid_storage",
                "admission scratch ownership changed; preserve it for inspection",
            )
        };
        if !same_directory(&self.root, &self.root_metadata)
            || !fs::symlink_metadata(self.root.join("owner"))
                .is_ok_and(|m| m.is_file() && !m.file_type().is_symlink() && m.len() == 0)
            || inspect_tree(&self.root, &self.root).is_err()
        {
            return Err(changed());
        }
        if let Some((binding, metadata, workspace_metadata)) = &self.binding {
            registry::confined(&self.history, &self.history).map_err(|_| changed())?;
            let physical_history = fs::canonicalize(&self.history).map_err(|_| changed())?;
            if !same_directory(binding, metadata)
                || !same_directory(&self.path(), workspace_metadata)
                || worktree_gitdir(&self.history, &self.path()).as_ref() != Ok(binding)
                || inspect_tree(&physical_history, binding).is_err()
            {
                return Err(changed());
            }
            fs::remove_dir_all(binding).map_err(io)?;
        } else if self.path().exists() {
            return Err(changed());
        }
        fs::remove_dir_all(&self.root).map_err(io)?;
        self.cleaned = true;
        Ok(())
    }
}
impl Drop for AdmissionWorktree {
    fn drop(&mut self) {
        let _ = self.cleanup();
    }
}
fn ensure_history(
    runtime: &crate::Runtime,
    registry: &RegistrySnapshot,
    id: &ProjectId,
) -> Result<PathBuf, SpecError> {
    let project = registry::project_dir(registry, id);
    let history = project.join("history.git");
    registry::confined(&registry.root, &history)?;
    let empty = project.join("git-empty-config");
    registry::confined(&registry.root, &empty)?;
    registry::confined(&registry.root, &project.join("disabled-hooks"))?;
    if !empty.exists() {
        fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&empty)
            .map_err(io)?;
    }
    fs::create_dir_all(project.join("disabled-hooks")).map_err(io)?;
    // Seed the minimal bare repository atomically, one independently repeatable
    // file at a time. No Git template, user configuration or half-written config
    // needs to be adopted during recovery.
    for dir in ["objects", "refs/heads"] {
        let path = history.join(dir);
        registry::confined(&registry.root, &path)?;
        fs::create_dir_all(path).map_err(io)?;
    }
    for (name, bytes) in [
        ("config", HISTORY_CONFIG),
        ("HEAD", b"ref: refs/heads/spec/bootstrap\n".as_slice()),
    ] {
        let path = history.join(name);
        registry::confined(&registry.root, &path)?;
        match fs::read(&path) {
            Ok(existing) if existing == bytes => (),
            Ok(_) => {
                return Err(error(
                    "invalid_storage",
                    "managed bare repository bootstrap changed",
                ));
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                vibe_core::fs::write_atomic(&path, bytes).map_err(io)?
            }
            Err(e) => return Err(io(e)),
        }
    }
    // Validate even when the next phase can return without invoking Git.
    git_text(
        runtime,
        &history,
        None,
        &["rev-parse", "--is-bare-repository"],
    )?;
    Ok(history)
}
pub(crate) fn ensure_existing(
    runtime: &crate::Runtime,
    registry: &RegistrySnapshot,
    id: &ProjectId,
    record: &Branch,
) -> Result<(), SpecError> {
    let project = registry::project_dir(registry, id);
    let history = project.join("history.git");
    let anchor = project.join("branches").join(record.id.as_str());
    if !history.is_dir() || !anchor.join("spec").is_dir() {
        return Err(error(
            "invalid_storage",
            "registered spec storage is incomplete",
        ));
    }
    inspect_tree(&registry.root, &anchor)?;
    bind_worktree(&history, &anchor)?;
    let head = fs::read_to_string(worktree_gitdir(&history, &anchor)?.join("HEAD")).map_err(io)?;
    if head.trim() != format!("ref: {}", record.spec_ref) {
        return Err(error("invalid_storage", "spec worktree HEAD changed"));
    }
    git_text(
        runtime,
        &history,
        None,
        &["rev-parse", "--verify", &record.spec_ref],
    )?;
    Ok(())
}
pub(crate) fn initialize_global(
    api: &SpecApi,
    registry: &RegistrySnapshot,
    id: &ProjectId,
    record: &Branch,
    base: Option<&str>,
) -> Result<(), SpecError> {
    api.check_control()?;
    let runtime = &api.runtime;
    let project = registry::project_dir(registry, id);
    let history = ensure_history(runtime, registry, id)?;
    api.check_control()?;
    let anchor = project.join("branches").join(record.id.as_str());
    let journal = project.join(format!("setup-{}.json", record.id));
    registry::confined(&registry.root, &anchor)?;
    registry::confined(&registry.root, &journal)?;
    let mut stage = if journal.exists() {
        let value: Value = serde_json::from_slice(&fs::read(&journal).map_err(io)?)
            .map_err(|e| error("invalid_registry", e))?;
        if value["v"] != 1
            || value["branch"] != registry::branch_value(record)
            || value["from"] != base.map(Value::from).unwrap_or(Value::Null)
        {
            return Err(error("setup_conflict", "branch setup inputs changed"));
        }
        value
    } else {
        if anchor.exists() {
            return Err(error(
                "setup_conflict",
                "unowned branch directory already exists",
            ));
        }
        let base_revision = if let Some(base) = base {
            base.to_owned()
        } else {
            let tree = git_text(runtime, &history, None, &["mktree"])?;
            git_text(
                runtime,
                &history,
                None,
                &["commit-tree", &tree, "-m", "Initialize spec history"],
            )?
        };
        let stage = json!({"v":1,"branch":registry::branch_value(record),"from":base,"base_revision":base_revision,"planned_revision":null});
        registry::write_json(&registry.root, &journal, &stage)?;
        stage
    };
    let base_revision = stage["base_revision"]
        .as_str()
        .ok_or_else(|| error("invalid_registry", "setup base revision missing"))?
        .to_owned();
    let current = git_text(
        runtime,
        &history,
        None,
        &["rev-parse", "--verify", "-q", &record.spec_ref],
    )
    .ok();
    api.check_control()?;
    if current.is_none() {
        git_text(
            runtime,
            &history,
            None,
            &[
                "update-ref",
                &record.spec_ref,
                &base_revision,
                "0000000000000000000000000000000000000000",
            ],
        )?;
    }
    api.check_control()?;
    if !anchor.join(".git").exists() {
        if anchor.exists() && fs::read_dir(&anchor).map_err(io)?.next().is_some() {
            return Err(error(
                "setup_conflict",
                "incomplete spec worktree contains files; preserve for manual recovery",
            ));
        }
        let mut command = args(&["worktree", "add", "--quiet"]);
        command.push(crate::git::argument_path(&anchor).into_os_string());
        command.push(
            record
                .spec_ref
                .strip_prefix("refs/heads/")
                .unwrap_or(&record.spec_ref)
                .into(),
        );
        spec_git(runtime, &history, None, &command)?;
    }
    bind_worktree(&history, &anchor)?;
    inspect_tree(&registry.root, &anchor)?;
    let head = fs::read_to_string(worktree_gitdir(&history, &anchor)?.join("HEAD")).map_err(io)?;
    if head.trim() != format!("ref: {}", record.spec_ref) {
        return Err(error("invalid_storage", "spec worktree HEAD changed"));
    }
    if let Some(planned) = stage["planned_revision"].as_str() {
        let current = git_text(
            runtime,
            &history,
            None,
            &["rev-parse", "--verify", &record.spec_ref],
        )?;
        if current == base_revision {
            api.check_control()?;
            git_text(
                runtime,
                &history,
                None,
                &["update-ref", &record.spec_ref, planned, &base_revision],
            )?;
        } else if current != planned {
            return Err(error(
                "setup_conflict",
                "spec ref diverged from interrupted setup",
            ));
        }
        return Ok(());
    }
    if git_text(
        runtime,
        &history,
        None,
        &["rev-parse", "--verify", &record.spec_ref],
    )? != base_revision
    {
        return Err(error(
            "setup_conflict",
            "spec ref changed before initial publication",
        ));
    }
    api.check_control()?;
    crate::init::seed(&anchor, &anchor.join("spec"))?;
    for rel in ["superpowers/specs", "superpowers/plans"] {
        fs::create_dir_all(anchor.join(rel)).map_err(io)?;
    }
    git_text(
        runtime,
        &history,
        Some(&anchor),
        &["add", "--all", "--", "spec", "superpowers"],
    )?;
    let tree = git_text(runtime, &history, Some(&anchor), &["write-tree"])?;
    let revision = git_text(
        runtime,
        &history,
        None,
        &[
            "commit-tree",
            &tree,
            "-p",
            &base_revision,
            "-m",
            "Initialize spec branch",
        ],
    )?;
    api.check_control()?;
    stage["planned_revision"] = revision.clone().into();
    registry::write_json(&registry.root, &journal, &stage)?;
    api.check_control()?;
    git_text(
        runtime,
        &history,
        None,
        &["update-ref", &record.spec_ref, &revision, &base_revision],
    )?;
    Ok(())
}

/// Conservative response admission for management before its publication. The
/// selected roots and native path encoding are bounded with the metadata, not
/// inferred later from a side-effectful Context call.
pub(crate) fn admit_management(
    api: &SpecApi,
    registry: &RegistrySnapshot,
    project: &Project,
    state: &ProjectState,
    checkout: &Path,
    with_refresh: bool,
) -> Result<(), SpecError> {
    if !api.limited() {
        return Ok(());
    }
    if with_refresh && project.mode == StorageMode::GlobalFirst {
        // Automatic merge validation uses no checkout-local configuration.
        // Reject an immutable invalid override before management publication.
        crate::check::load_data(&Execution::new(&api.runtime), None, false)
            .map_err(|message| error("invalid_config", message))?;
    }
    fn wire_cost(value: &Value) -> Result<usize, SpecError> {
        let encoded = serde_json::to_string(value).map_err(io)?;
        Ok(encoded.len() + serde_json::to_vec(&encoded).map_err(io)?.len())
    }
    let mut code_ref = None;
    let mut ref_cost = 0;
    let clean = crate::git::context_runtime(&api.runtime);
    let cx = Execution::new(&clean);
    for name in state
        .branches
        .iter()
        .map(|b| b.code_ref.clone())
        .chain(crate::context::current_ref(&cx, checkout))
    {
        let cost = wire_cost(&json!(name))?;
        if cost > ref_cost {
            ref_cost = cost;
            code_ref = Some(name);
        }
    }
    let mut selected_checkout = registry::path_value(checkout);
    for candidate in &project.checkouts {
        let candidate = registry::path_value(&candidate.path);
        if wire_cost(&candidate)? > wire_cost(&selected_checkout)? {
            selected_checkout = candidate;
        }
    }
    let (spec, artifacts) = if project.mode == StorageMode::GlobalFirst {
        let mut largest = None;
        for branch in &state.branches {
            let anchor = registry::project_dir(registry, &project.id)
                .join("branches")
                .join(branch.id.as_str());
            let spec = anchor.join("spec");
            let artifacts = anchor.join("superpowers");
            let cost = wire_cost(&registry::path_value(&spec))?
                + wire_cost(&registry::path_value(&artifacts))?;
            if largest.as_ref().is_none_or(|(old, _, _)| cost > *old) {
                largest = Some((cost, spec, artifacts));
            }
        }
        let (_, spec, artifacts) =
            largest.ok_or_else(|| error("branch_not_found", "project has no stored branch"))?;
        (spec, artifacts)
    } else {
        (
            repo_root(&api.runtime, checkout)?,
            checkout.join("docs/superpowers"),
        )
    };
    let refresh = if with_refresh && project.mode == StorageMode::GlobalFirst {
        crate::refresh::reserve_management(registry, project, state)?
    } else {
        crate::refresh::RefreshSummary::default()
    };
    let context = json!({"project_id":project.id.as_str(),"mode":project.mode.as_str(),"checkout":selected_checkout,"code_ref":code_ref,"code_head":"0".repeat(64),"code_available":false,"spec_root":registry::path_value(&spec),"artifact_root":registry::path_value(&artifacts),"spec_revision":"0".repeat(64),"revision_token":"v1:".to_owned()+&"0".repeat(64),"refresh":refresh.value()});
    let data = json!({"v":1,"revision":"0".repeat(64),"project_revision":"0".repeat(64),"project":registry::project_value(project),"branches":state.branches.iter().map(registry::branch_value).collect::<Vec<_>>(),"context":context});
    api.admit_result(&json!({"context":context,"data":data}))
}

#[cfg(test)]
mod admission_tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    #[test]
    fn followup_admission_cleanup_keeps_canonical_binding_and_foreign_ownership() {
        let (_owner, api, context) = crate::transaction::tests::fixture();
        let registry = registry::load(&api.runtime).unwrap();
        let project = registry::project_dir(&registry, context.project_id.as_ref().unwrap());
        let history =
            crate::git::argument_path(&project.join("history.git").canonicalize().unwrap());
        let target = context.spec_revision.as_deref().unwrap();
        let scratch = AdmissionWorktree::new(&api, &history, target).unwrap();
        let root = scratch.root.clone();
        scratch.finish().unwrap();
        assert!(!root.exists());

        let scratch = AdmissionWorktree::new(&api, &history, target).unwrap();
        let root = scratch.root.clone();
        let binding = scratch.binding.as_ref().unwrap().0.clone();
        drop(scratch);
        assert!(!root.exists());
        assert!(!binding.exists());

        let scratch = AdmissionWorktree::new(&api, &history, target).unwrap();
        let root = scratch.root.clone();
        let binding = scratch.binding.as_ref().unwrap().0.clone();
        fs::write(binding.join("gitdir"), b"foreign-binding\n").unwrap();
        assert_eq!(scratch.finish().unwrap_err().code, "invalid_storage");
        assert!(root.exists());
        assert_eq!(
            fs::read(binding.join("gitdir")).unwrap(),
            b"foreign-binding\n"
        );

        let scratch = AdmissionWorktree::new(&api, &history, target).unwrap();
        let root = scratch.root.clone();
        fs::write(root.join("owner"), b"foreign-owner").unwrap();
        drop(scratch);
        assert_eq!(fs::read(root.join("owner")).unwrap(), b"foreign-owner");

        let scratch = AdmissionWorktree::new(&api, &history, target).unwrap();
        let root = scratch.root.clone();
        let moved = root.with_extension("moved");
        fs::rename(&root, &moved).unwrap();
        fs::create_dir(&root).unwrap();
        fs::write(root.join("foreign"), b"replacement").unwrap();
        assert_eq!(scratch.finish().unwrap_err().code, "invalid_storage");
        assert_eq!(fs::read(root.join("foreign")).unwrap(), b"replacement");
        assert!(moved.join("owner").exists());
    }
}
