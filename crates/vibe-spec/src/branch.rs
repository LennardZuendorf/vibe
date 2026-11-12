//! Explicit spec branches never create, checkout or mutate code branches.
use crate::registry;
use crate::{ContextSelector, InitBranchRequest, SpecApi, SpecError, SpecResponse, StorageMode};
use serde_json::json;

pub(crate) fn init(api: &SpecApi, request: InitBranchRequest) -> Result<SpecResponse, SpecError> {
    api.check_control()?;
    let target = crate::context::code_ref(&request.name)?;
    let base = crate::context::code_ref(&request.from)?;
    let root = crate::storage::admit_global_root(api)?;
    let registry_lock_path = root.join("spec/registry.lock");
    let _registry_lock = api.lock_storage(&root, &registry_lock_path)?;
    let registry = registry::load(&api.runtime)?;
    registry::expect(&registry.revision, &request.expected_revision)?;
    let project = crate::storage::selected(api, &registry, &request.selector)?;
    if project.mode != StorageMode::GlobalFirst {
        return Err(SpecError::new(
            "unsupported_mode",
            "managed spec branches require global-first storage",
        ));
    }
    let project_dir = registry::project_dir(&registry, &project.id);
    let project_lock_path = project_dir.join("project.lock");
    let _project_lock = api.lock_storage(&root, &project_lock_path)?;
    let mut state = registry::load_project(&registry, &project.id)?;
    registry::expect(&state.revision, &request.expected_project_revision)?;
    let clean = crate::git::context_runtime(&api.runtime);
    let cx = crate::api::Execution::new(&clean);
    let location = api.runtime.cwd.join(
        request
            .selector
            .checkout
            .as_deref()
            .unwrap_or(&api.runtime.cwd),
    );
    let mut checkout = crate::context::checkout_top(&cx, &location)?;
    if request.selector.checkout.is_some() && checkout.is_none() {
        return Err(SpecError::new(
            "checkout_unavailable",
            "explicit checkout is missing or is not a Git code checkout",
        ));
    }
    if let Some(candidate) = &checkout
        && !crate::context::matching_projects(&registry, &cx, candidate)
            .iter()
            .any(|p| p.id == project.id)
    {
        if request.selector.checkout.is_none() && request.selector.project.is_some() {
            checkout = None;
        } else {
            return Err(SpecError::new(
                "context_mismatch",
                "selected code checkout does not belong to the project",
            ));
        }
    }
    if state.branches.iter().any(|b| b.code_ref == target) {
        let existing = state
            .branches
            .iter()
            .find(|b| b.code_ref == target)
            .ok_or_else(|| SpecError::new("branch_not_found", "branch disappeared"))?;
        crate::storage::ensure_existing(&api.runtime, &registry, &project.id, existing)?;
        return response(api, &registry, project, &state, existing, &request.selector);
    }
    let base = state
        .branches
        .iter()
        .find(|b| b.code_ref == base)
        .ok_or_else(|| SpecError::new("branch_not_found", "explicit spec base is not registered"))?
        .clone();
    crate::storage::ensure_existing(&api.runtime, &registry, &project.id, &base)?;
    let base_context = crate::context::resolve_branch_storage_locked(
        &api.runtime,
        &ContextSelector {
            project: Some(project.id.clone()),
            branch: Some(base.code_ref.clone()),
            checkout: request.selector.checkout.clone(),
        },
    )?;
    let parent = base_context
        .spec_revision
        .ok_or_else(|| SpecError::new("invalid_storage", "base spec revision is missing"))?;
    let observed_base_ref = base_context
        .checkout
        .as_ref()
        .and(base_context.code_ref.clone());
    let mut record = crate::storage::new_branch(
        &target,
        Some(parent.clone()),
        base_context.code_head.clone(),
        observed_base_ref.clone(),
    )?;
    if let Some(head) = checkout
        .as_deref()
        .and_then(|code| crate::git::git(&cx, code, &["rev-parse", "--verify", "-q", &target]))
        .and_then(|b| String::from_utf8(b).ok())
        .map(|s| s.trim().to_owned())
    {
        record.last_code_head = Some(head);
        record.last_code_ref = Some(target);
    } else {
        record.last_code_head = base_context.code_head;
        record.last_code_ref = observed_base_ref;
    }
    let target_root = project_dir
        .join("branches")
        .join(record.id.as_str())
        .join("spec");
    crate::context::check_branch_storage_override(&api.runtime, checkout.as_deref(), &target_root)?;
    let mut planned = state.clone();
    planned.branches.push(record.clone());
    crate::storage::admit_management(
        api,
        &registry,
        project,
        &planned,
        checkout.as_deref().unwrap_or(&api.runtime.cwd),
        false,
    )?;
    crate::storage::initialize_global(api, &registry, &project.id, &record, Some(&parent))?;
    let _completion = api.completing();
    state.branches.push(record.clone());
    crate::storage::verify_lock(&registry_lock_path)?;
    crate::storage::verify_lock(&project_lock_path)?;
    registry::write_project(&registry, &project.id, &state)?;
    let fresh = registry::load_project(&registry, &project.id)?;
    response(api, &registry, project, &fresh, &record, &request.selector)
}
fn response(
    api: &SpecApi,
    registry: &registry::RegistrySnapshot,
    project: &registry::Project,
    state: &registry::ProjectState,
    record: &registry::Branch,
    selector: &ContextSelector,
) -> Result<SpecResponse, SpecError> {
    let context = crate::context::resolve_branch_storage_locked(
        &api.runtime,
        &ContextSelector {
            project: Some(project.id.clone()),
            branch: Some(record.code_ref.clone()),
            checkout: selector.checkout.clone(),
        },
    )?;
    Ok(SpecResponse {
        data: json!({"v":1,"revision":registry.revision,"project_revision":state.revision,"branch":registry::branch_value(record),"context":crate::storage::context_value(&context)}),
        context: Some(context),
    })
}
