//! Serialized document transactions. A journal precedes effects and publication is an exact ref CAS.
use crate::{
    ContextSelector, SpecApi, SpecContext, SpecError, SpecResponse, StorageMode,
    documents::{self, Change, Files},
    registry, storage,
};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

fn error(code: &str, message: impl Into<String>) -> SpecError {
    SpecError::new(code, message)
}
fn io(e: impl std::fmt::Display) -> SpecError {
    documents::io(e)
}

struct OwnedDirectories(Vec<(PathBuf, fs::Metadata)>);
impl OwnedDirectories {
    fn create(anchor: &std::path::Path) -> Result<Self, SpecError> {
        let mut missing = Vec::new();
        let mut current = Some(anchor);
        while let Some(path) = current {
            match fs::symlink_metadata(path) {
                Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => break,
                Ok(_) => return Err(io("runtime directory is not a regular directory")),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    missing.push(path.to_owned());
                    current = path.parent();
                }
                Err(error) => return Err(io(error)),
            }
        }
        let mut owned = Self(Vec::new());
        for path in missing.into_iter().rev() {
            match fs::create_dir(&path) {
                Ok(()) => {
                    let metadata = fs::symlink_metadata(&path).map_err(io)?;
                    owned.0.push((path, metadata));
                }
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                    let metadata = fs::symlink_metadata(&path).map_err(io)?;
                    if !metadata.is_dir() || metadata.file_type().is_symlink() {
                        return Err(io("runtime directory changed during creation"));
                    }
                }
                Err(error) => return Err(io(error)),
            }
        }
        Ok(owned)
    }
}
impl Drop for OwnedDirectories {
    fn drop(&mut self) {
        for (path, original) in self.0.iter().rev() {
            let Ok(current) = fs::symlink_metadata(path) else {
                continue;
            };
            if !current.is_dir() || current.file_type().is_symlink() {
                continue;
            }
            #[cfg(unix)]
            let unchanged = {
                use std::os::unix::fs::MetadataExt;
                (original.dev(), original.ino()) == (current.dev(), current.ino())
            };
            #[cfg(not(unix))]
            let unchanged = original
                .created()
                .ok()
                .is_some_and(|created| current.created().ok() == Some(created));
            if unchanged {
                let _ = fs::remove_dir(path);
            }
        }
    }
}

pub(crate) struct Locked {
    // Fields drop in declaration order: release the lock before removing owned empty directories.
    _lock: vibe_core::lock::Lock,
    _directories: OwnedDirectories,
    lock_path: PathBuf,
    pub(crate) anchor: PathBuf,
    journal: PathBuf,
    pub(crate) history: Option<PathBuf>,
    pub(crate) spec_ref: Option<String>,
}
impl Locked {
    fn acquire(api: &SpecApi, initial: &SpecContext) -> Result<Self, SpecError> {
        let registry = registry::load(&api.runtime)?;
        let (anchor, lock_path, journal, history, spec_ref) =
            if initial.mode == StorageMode::GlobalFirst {
                let id = initial
                    .project_id
                    .as_ref()
                    .ok_or_else(|| error("invalid_context", "global project is missing"))?;
                let project = registry::project_dir(&registry, id);
                let state = registry::load_project(&registry, id)?;
                let branch = state
                    .branches
                    .iter()
                    .find(|b| Some(&b.code_ref) == initial.code_ref.as_ref())
                    .ok_or_else(|| error("branch_not_found", "selected branch is missing"))?;
                (
                    project.clone(),
                    project.join("project.lock"),
                    project.join(format!("transaction-{}.json", branch.id)),
                    Some(project.join("history.git")),
                    Some(branch.spec_ref.clone()),
                )
            } else {
                let checkout = initial
                    .checkout
                    .as_deref()
                    .ok_or_else(|| error("invalid_context", "repo-first checkout is missing"))?;
                let anchor = checkout.join(".vibe/run");
                // One lock per physical checkout, including explicit external spec roots.
                (
                    anchor.clone(),
                    anchor.join("spec-documents.lock"),
                    anchor.join("spec-transaction.json"),
                    None,
                    None,
                )
            };
        registry::confined(&anchor, &lock_path)?;
        registry::confined(&anchor, &journal)?;
        api.check_control()?;
        let directories = OwnedDirectories::create(&anchor)?;
        let lock = api.lock_storage(&anchor, &lock_path)?;
        Ok(Self {
            _lock: lock,
            _directories: directories,
            lock_path,
            anchor,
            journal,
            history,
            spec_ref,
        })
    }
    fn acquire_branch(
        api: &SpecApi,
        project: &crate::ProjectId,
        branch: &crate::BranchId,
    ) -> Result<Self, SpecError> {
        let registry = registry::load(&api.runtime)?;
        if !registry
            .projects
            .iter()
            .any(|p| &p.id == project && p.mode == StorageMode::GlobalFirst)
        {
            return Err(error(
                "unsupported_mode",
                "private branch lock requires global-first project",
            ));
        }
        let state = registry::load_project(&registry, project)?;
        let branch = state
            .branches
            .iter()
            .find(|b| &b.id == branch)
            .ok_or_else(|| error("branch_not_found", "stored target branch is missing"))?;
        let anchor = registry::project_dir(&registry, project);
        let lock_path = anchor.join("project.lock");
        let journal = anchor.join(format!("transaction-{}.json", branch.id));
        registry::confined(&registry.root, &anchor)?;
        registry::confined(&anchor, &lock_path)?;
        registry::confined(&anchor, &journal)?;
        api.check_control()?;
        let directories = OwnedDirectories::create(&anchor)?;
        let lock = api.lock_storage(&anchor, &lock_path)?;
        let locked = Self {
            _lock: lock,
            _directories: directories,
            lock_path,
            anchor: anchor.clone(),
            journal,
            history: Some(anchor.join("history.git")),
            spec_ref: Some(branch.spec_ref.clone()),
        };
        let fresh = registry::load_project(&registry, project)?;
        let current = fresh
            .branches
            .iter()
            .find(|b| b.id == branch.id)
            .ok_or_else(|| error("branch_not_found", "stored target branch disappeared"))?;
        if current.spec_ref != branch.spec_ref || current.code_ref != branch.code_ref {
            return Err(error(
                "context_changed",
                "stored target binding changed while waiting",
            ));
        }
        storage::ensure_existing(&api.runtime, &registry, project, current)?;
        Ok(locked)
    }
    pub(crate) fn admit_storage(
        &self,
        api: &SpecApi,
        context: &SpecContext,
    ) -> Result<(), SpecError> {
        if context.mode == StorageMode::GlobalFirst {
            let registry = registry::load(&api.runtime)?;
            let id = context
                .project_id
                .as_ref()
                .ok_or_else(|| error("invalid_context", "global project missing"))?;
            let state = registry::load_project(&registry, id)?;
            let branch = state
                .branches
                .iter()
                .find(|b| Some(&b.code_ref) == context.code_ref.as_ref())
                .ok_or_else(|| error("branch_not_found", "selected branch missing"))?;
            storage::ensure_existing(&api.runtime, &registry, id, branch)?;
        }
        Ok(())
    }
    pub(crate) fn verify(&self) -> Result<(), SpecError> {
        storage::verify_lock(&self.lock_path)
    }
    pub(crate) fn git(
        &self,
        api: &SpecApi,
        args: &[&str],
        input: Option<&[u8]>,
    ) -> Result<Vec<u8>, SpecError> {
        storage::spec_git_input(
            &api.runtime,
            self.history.as_deref().ok_or_else(|| {
                error(
                    "unsupported_mode",
                    "private history requires global storage",
                )
            })?,
            None,
            &args
                .iter()
                .map(std::ffi::OsString::from)
                .collect::<Vec<_>>(),
            input,
        )
    }
    pub(crate) fn text(
        &self,
        api: &SpecApi,
        args: &[&str],
        input: Option<&[u8]>,
    ) -> Result<String, SpecError> {
        String::from_utf8(self.git(api, args, input)?)
            .map(|s| s.trim().to_owned())
            .map_err(io)
    }
    pub(crate) fn revision(&self, api: &SpecApi) -> Result<String, SpecError> {
        self.text(
            api,
            &[
                "rev-parse",
                "--verify",
                self.spec_ref
                    .as_deref()
                    .ok_or_else(|| error("invalid_context", "spec ref missing"))?,
            ],
            None,
        )
    }
    fn record(&self, value: &Value) -> Result<(), SpecError> {
        self.verify()?;
        registry::write_json(&self.anchor, &self.journal, value)
    }
    fn clear(&self) -> Result<(), SpecError> {
        self.verify()?;
        registry::confined(&self.anchor, &self.journal)?;
        fs::remove_file(&self.journal).map_err(io)
    }
}

/// Read under the same lock as writes. Pending recovery never masquerades as a clean revision.
pub(crate) fn resolve_context(
    runtime: &crate::Runtime,
    selector: &ContextSelector,
) -> Result<SpecContext, SpecError> {
    let api = SpecApi::new(runtime.clone())?;
    resolve_controlled(&api, selector)
}
pub(crate) fn resolve_controlled(
    api: &SpecApi,
    selector: &ContextSelector,
) -> Result<SpecContext, SpecError> {
    api.check_control()?;
    let runtime = &api.runtime;
    let initial = crate::context::resolve_for_transaction(runtime, selector)?;
    let locked = Locked::acquire(api, &initial)?;
    let context = crate::context::resolve_locked(runtime, selector)?;
    same_selection(&initial, &context)?;
    let summary = crate::refresh::locked(api, &context, &locked)?;
    let mut context = crate::context::resolve_locked(runtime, selector)?;
    context.refresh = summary;
    Ok(context)
}
fn same_selection(initial: &SpecContext, current: &SpecContext) -> Result<(), SpecError> {
    if initial.project_id != current.project_id
        || initial.mode != current.mode
        || initial.checkout != current.checkout
        || initial.code_ref != current.code_ref
        || initial.spec_root != current.spec_root
        || initial.artifact_root != current.artifact_root
    {
        return Err(error(
            "context_changed",
            "selected checkout or spec branch changed while waiting",
        ));
    }
    Ok(())
}
pub(crate) fn read_typed<T>(
    api: &SpecApi,
    selector: &ContextSelector,
    f: impl FnOnce(&SpecContext) -> Result<T, SpecError>,
) -> Result<(SpecContext, T), SpecError> {
    read_selected(api, selector, None, f)
}
pub(crate) fn read_legacy<T>(
    api: &SpecApi,
    selector: &ContextSelector,
    roots: &crate::context::LegacyRoots,
    f: impl FnOnce(&SpecContext) -> Result<T, SpecError>,
) -> Result<(SpecContext, T), SpecError> {
    read_selected(api, selector, Some(roots), f)
}
fn resolve_selection(
    api: &SpecApi,
    selector: &ContextSelector,
    legacy: Option<&crate::context::LegacyRoots>,
    readable: bool,
) -> Result<SpecContext, SpecError> {
    if let Some(roots) = legacy {
        let context = crate::context::resolve_for_legacy(&api.runtime, selector, roots)?;
        if readable {
            ensure_readable(&context, &api.runtime)?;
        }
        Ok(context)
    } else if readable {
        crate::context::resolve_locked(&api.runtime, selector)
    } else {
        crate::context::resolve_for_transaction(&api.runtime, selector)
    }
}
fn read_selected<T>(
    api: &SpecApi,
    selector: &ContextSelector,
    legacy: Option<&crate::context::LegacyRoots>,
    f: impl FnOnce(&SpecContext) -> Result<T, SpecError>,
) -> Result<(SpecContext, T), SpecError> {
    let initial = resolve_selection(api, selector, legacy, false)?;
    let locked = Locked::acquire(api, &initial)?;
    let recovery_context = resolve_selection(api, selector, legacy, false)?;
    locked.admit_storage(api, &recovery_context)?;
    recover(api, &recovery_context, &locked)?;
    same_selection(&initial, &recovery_context)?;
    let context = resolve_selection(api, selector, legacy, true)?;
    let summary = if legacy.is_none() {
        crate::refresh::locked(api, &context, &locked)?
    } else {
        Default::default()
    };
    let context = resolve_selection(api, selector, legacy, true)?;
    let data = f(&context)?;
    let current = if legacy.is_some() {
        context.ensure_matches(resolve_selection(api, selector, legacy, true)?)?
    } else {
        context.ensure_current(&api.runtime, selector)?
    };
    let mut current = current;
    current.refresh = summary;
    Ok((current, data))
}

fn recover_authorized(
    api: &SpecApi,
    context: &SpecContext,
    locked: &Locked,
    selector: &ContextSelector,
    legacy: Option<&crate::context::LegacyRoots>,
) -> Result<(), SpecError> {
    if legacy.is_some() {
        let current = resolve_selection(api, selector, legacy, false)?;
        same_selection(context, &current)?;
        if context.code_head != current.code_head {
            return Err(error(
                "context_changed",
                "code authority changed before recovery",
            ));
        }
        recover(api, &current, locked)
    } else {
        recover(api, context, locked)
    }
}

/// Context lookup uses this without taking a second lock or performing recovery.
pub(crate) fn ensure_readable(
    context: &SpecContext,
    runtime: &crate::Runtime,
) -> Result<(), SpecError> {
    let journal = if context.mode == StorageMode::GlobalFirst {
        let snapshot = registry::load(runtime)?;
        let id = context
            .project_id
            .as_ref()
            .ok_or_else(|| error("invalid_context", "global project missing"))?;
        let state = registry::load_project(&snapshot, id)?;
        let branch = state
            .branches
            .iter()
            .find(|b| Some(&b.code_ref) == context.code_ref.as_ref())
            .ok_or_else(|| error("branch_not_found", "branch missing"))?;
        registry::project_dir(&snapshot, id).join(format!("transaction-{}.json", branch.id))
    } else {
        context
            .require_code()?
            .join(".vibe/run/spec-transaction.json")
    };
    registry::confined(
        journal
            .parent()
            .ok_or_else(|| io("journal has no parent"))?,
        &journal,
    )?;
    if journal.exists() {
        return Err(error(
            "recovery_required",
            "an interrupted document transaction needs recovery before reading",
        ));
    }
    Ok(())
}

/// Explicit checkpoints support cancellation before publication. Returning an error leaves a recoverable journal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Checkpoint {
    BeforeWrites,
    AfterWrites,
    BeforePublish,
    AfterPublish,
}

pub(crate) fn apply(
    api: &SpecApi,
    selector: &ContextSelector,
    expected: &str,
    changes: Vec<Change>,
) -> Result<SpecResponse, SpecError> {
    apply_planned(api, selector, expected, false, |_| Ok(changes), |_| Ok(()))
}
pub(crate) fn snapshot(
    api: &SpecApi,
    selector: &ContextSelector,
    expected: &str,
) -> Result<SpecResponse, SpecError> {
    apply_planned(
        api,
        selector,
        expected,
        true,
        |_| Ok(Vec::new()),
        |_| Ok(()),
    )
}
/// Build lifecycle write plans only after the lock and expected-context check.
pub(crate) fn apply_planned(
    api: &SpecApi,
    selector: &ContextSelector,
    expected: &str,
    snapshot: bool,
    plan: impl FnOnce(&SpecContext) -> Result<Vec<Change>, SpecError>,
    checkpoint: impl FnMut(Checkpoint) -> Result<(), SpecError>,
) -> Result<SpecResponse, SpecError> {
    apply_lifecycle_planned(api, selector, expected, snapshot, &[], plan, checkpoint)
}
/// Only explicit lifecycle completion supplies completion events; disappearance alone does not.
pub(crate) fn apply_lifecycle_planned(
    api: &SpecApi,
    selector: &ContextSelector,
    expected: &str,
    snapshot: bool,
    completed_features: &[String],
    plan: impl FnOnce(&SpecContext) -> Result<Vec<Change>, SpecError>,
    checkpoint: impl FnMut(Checkpoint) -> Result<(), SpecError>,
) -> Result<SpecResponse, SpecError> {
    apply_inner(
        api,
        selector,
        Some(expected),
        None,
        snapshot,
        completed_features,
        plan,
        checkpoint,
    )
}
/// The legacy repo CLI captures its revision only after acquiring the existing lock.
pub(crate) fn apply_captured_planned(
    api: &SpecApi,
    selector: &ContextSelector,
    roots: &crate::context::LegacyRoots,
    plan: impl FnOnce(&SpecContext) -> Result<Vec<Change>, SpecError>,
) -> Result<SpecResponse, SpecError> {
    apply_inner(api, selector, None, Some(roots), false, &[], plan, |_| {
        Ok(())
    })
}
pub(crate) fn apply_legacy_expected(
    api: &SpecApi,
    selector: &ContextSelector,
    roots: &crate::context::LegacyRoots,
    expected: &str,
    plan: impl FnOnce(&SpecContext) -> Result<Vec<Change>, SpecError>,
) -> Result<SpecResponse, SpecError> {
    apply_inner(
        api,
        selector,
        Some(expected),
        Some(roots),
        false,
        &[],
        plan,
        |_| Ok(()),
    )
}
#[allow(clippy::too_many_arguments)]
fn apply_inner(
    api: &SpecApi,
    selector: &ContextSelector,
    expected: Option<&str>,
    legacy: Option<&crate::context::LegacyRoots>,
    snapshot: bool,
    completed_features: &[String],
    plan: impl FnOnce(&SpecContext) -> Result<Vec<Change>, SpecError>,
    checkpoint: impl FnMut(Checkpoint) -> Result<(), SpecError>,
) -> Result<SpecResponse, SpecError> {
    apply_inner_reported(
        api,
        selector,
        expected,
        legacy,
        snapshot,
        completed_features,
        |context| plan(context).map(|changes| (changes, PlannedResponse::Receipt)),
        checkpoint,
    )
}
/// A complete operation response planned under the same lock as its documents.
pub(crate) enum PlannedResponse {
    Receipt,
    Fields(Value),
    Wrapped(Value),
}
impl PlannedResponse {
    fn value(&self, receipt: Value) -> Value {
        match self {
            Self::Receipt => receipt,
            Self::Fields(fields) => {
                let mut value = receipt;
                if let Some(fields) = fields.as_object() {
                    for (k, v) in fields {
                        value[k] = v.clone();
                    }
                }
                value
            }
            Self::Wrapped(prefix) => {
                let mut value = prefix.clone();
                value["receipt"] = receipt;
                value
            }
        }
    }
}
pub(crate) fn apply_reported_planned(
    api: &SpecApi,
    selector: &ContextSelector,
    expected: &str,
    completed_features: &[String],
    plan: impl FnOnce(&SpecContext) -> Result<(Vec<Change>, PlannedResponse), SpecError>,
) -> Result<SpecResponse, SpecError> {
    apply_inner_reported(
        api,
        selector,
        Some(expected),
        None,
        false,
        completed_features,
        plan,
        |_| Ok(()),
    )
}
#[allow(clippy::too_many_arguments)]
fn apply_inner_reported(
    api: &SpecApi,
    selector: &ContextSelector,
    expected: Option<&str>,
    legacy: Option<&crate::context::LegacyRoots>,
    snapshot: bool,
    completed_features: &[String],
    plan: impl FnOnce(&SpecContext) -> Result<(Vec<Change>, PlannedResponse), SpecError>,
    checkpoint: impl FnMut(Checkpoint) -> Result<(), SpecError>,
) -> Result<SpecResponse, SpecError> {
    // Internal resolution permits reaching the journal; public resolution refuses incomplete reads.
    let initial = resolve_selection(api, selector, legacy, false)?;
    let locked = Locked::acquire(api, &initial)?;
    let recovery_context = resolve_selection(api, selector, legacy, false)?;
    locked.admit_storage(api, &recovery_context)?;
    recover(api, &recovery_context, &locked)?;
    same_selection(&initial, &recovery_context)?;
    let context = resolve_selection(api, selector, legacy, true)?;
    // Preserve initial selection even if the user switches checkout while waiting.
    if initial.project_id != context.project_id
        || initial.mode != context.mode
        || initial.checkout != context.checkout
        || initial.code_ref != context.code_ref
        || initial.spec_root != context.spec_root
        || initial.artifact_root != context.artifact_root
    {
        return Err(error(
            "context_changed",
            "selected checkout or spec branch changed while waiting",
        ));
    }
    if let Some(expected) = expected {
        registry::expect(&context.revision_token, expected)?;
    } else if context.mode != StorageMode::RepoFirst {
        return Err(error(
            "expected_revision_required",
            "global mutations require an explicit expected revision",
        ));
    }
    apply_locked(
        api,
        selector,
        legacy,
        snapshot,
        completed_features,
        &context,
        &locked,
        None,
        plan,
        checkpoint,
    )
}

/// Record identity selects the project lock before code checkout discovery.
pub(crate) fn with_project_branch<T>(
    api: &SpecApi,
    project: &crate::ProjectId,
    branch: &crate::BranchId,
    action: impl FnOnce(&Locked) -> Result<T, SpecError>,
) -> Result<T, SpecError> {
    let locked = Locked::acquire_branch(api, project, branch)?;
    action(&locked)
}
pub(crate) fn recover_reconciliation_target(
    api: &SpecApi,
    locked: &Locked,
    plan: &crate::ReconcilePlan,
) -> Result<SpecContext, SpecError> {
    let selector = plan.apply_selector();
    let check = |context: &SpecContext| -> Result<(), SpecError> {
        if context.project_id.as_ref() != Some(&plan.project)
            || context.code_ref.as_deref() != Some(&plan.target_ref)
            || context.checkout != plan.target_checkout
        {
            return Err(error(
                "context_changed",
                "captured reconciliation checkout or branch changed",
            ));
        }
        Ok(())
    };
    let recovery = resolve_selection(api, &selector, None, false)?;
    check(&recovery)?;
    locked.admit_storage(api, &recovery)?;
    recover(api, &recovery, locked)?;
    let context = resolve_selection(api, &selector, None, true)?;
    check(&context)?;
    Ok(context)
}
/// Finish only a proven published receipt without requiring a still-current code checkout.
pub(crate) fn recover_reconciliation_receipt(
    api: &SpecApi,
    locked: &Locked,
    plan: &crate::ReconcilePlan,
    id: &crate::RecordId,
) -> Result<bool, SpecError> {
    registry::confined(&locked.anchor, &locked.journal)?;
    let bytes = match fs::read(&locked.journal) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(e) => return Err(io(e)),
    };
    let journal: Value = serde_json::from_slice(&bytes)
        .map_err(|_| error("invalid_transaction", "invalid reconciliation journal"))?;
    if journal["reconciliation"].as_str() != Some(id.as_str()) {
        return Ok(false);
    }
    let worktree = locked.anchor.join("branches").join(plan.target.as_str());
    if journal["v"] != 1
        || journal["mode"] != "global-first"
        || journal["base"].as_str() != Some(&plan.target_spec_oid)
        || journal["spec_ref"].as_str() != locked.spec_ref.as_deref()
        || journal["spec_root"] != registry::path_value(&worktree.join("spec"))
        || journal["artifact_root"] != registry::path_value(&worktree.join("superpowers"))
    {
        return Err(error(
            "invalid_transaction",
            "reconciliation journal binding differs",
        ));
    }
    let Some(planned) = journal["planned"].as_str() else {
        return Ok(false);
    };
    if planned.len() != 40 || !planned.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(error(
            "invalid_transaction",
            "invalid planned reconciliation OID",
        ));
    }
    let current = locked.revision(api)?;
    if current == plan.target_spec_oid {
        return Ok(false);
    }
    let output = storage::spec_git_output(
        &api.runtime,
        locked
            .history
            .as_deref()
            .ok_or_else(|| io("missing history"))?,
        None,
        &["merge-base", "--is-ancestor", planned, &current].map(std::ffi::OsString::from),
        None,
    )?;
    if !output.status.success() {
        return Err(error(
            "recovery_conflict",
            "target is not the recorded reconciliation result or descendant",
        ));
    }
    let _completion = api.completing();
    crate::reconcile::finish_receipt(api, &locked.anchor, id, planned)?;
    locked.clear()?;
    Ok(true)
}
/// Legacy formatter preview retains physical discovery authority without recovery effects.
pub(crate) fn read_legacy_preview<T>(
    api: &SpecApi,
    selector: &ContextSelector,
    roots: &crate::context::LegacyRoots,
    action: impl FnOnce(&SpecContext) -> Result<T, SpecError>,
) -> Result<(SpecContext, T), SpecError> {
    let initial = resolve_selection(api, selector, Some(roots), false)?;
    let locked = Locked::acquire(api, &initial)?;
    let context = resolve_selection(api, selector, Some(roots), true)?;
    same_selection(&initial, &context)?;
    locked.admit_storage(api, &context)?;
    let result = action(&context)?;
    let current = context.ensure_matches(resolve_selection(api, selector, Some(roots), true)?)?;
    Ok((current, result))
}

pub(crate) fn preview_locked<T>(
    api: &SpecApi,
    selector: &ContextSelector,
    action: impl FnOnce(&SpecContext, &Locked) -> Result<T, SpecError>,
) -> Result<T, SpecError> {
    let initial = resolve_selection(api, selector, None, false)?;
    let locked = Locked::acquire(api, &initial)?;
    let context = resolve_selection(api, selector, None, true)?;
    same_selection(&initial, &context)?;
    locked.admit_storage(api, &context)?;
    let result = action(&context, &locked)?;
    context.ensure_current(&api.runtime, selector)?;
    Ok(result)
}
pub(crate) fn read_preview<T>(
    api: &SpecApi,
    selector: &ContextSelector,
    action: impl FnOnce(&SpecContext) -> Result<T, SpecError>,
) -> Result<(SpecContext, T), SpecError> {
    preview_locked(api, selector, |context, _| {
        Ok((context.clone(), action(context)?))
    })
}

pub(crate) struct MergePublication<'a> {
    pub source: &'a str,
    pub record_id: &'a crate::RecordId,
}

#[allow(clippy::too_many_arguments)]
fn apply_locked(
    api: &SpecApi,
    selector: &ContextSelector,
    legacy: Option<&crate::context::LegacyRoots>,
    snapshot: bool,
    completed_features: &[String],
    context: &SpecContext,
    locked: &Locked,
    merge: Option<&MergePublication<'_>>,
    plan: impl FnOnce(&SpecContext) -> Result<(Vec<Change>, PlannedResponse), SpecError>,
    mut checkpoint: impl FnMut(Checkpoint) -> Result<(), SpecError>,
) -> Result<SpecResponse, SpecError> {
    let registry_before = registry::load(&api.runtime)?;
    let project_before = context
        .project_id
        .as_ref()
        .map(|id| registry::load_project(&registry_before, id))
        .transpose()?
        .map(|state| state.revision);
    if snapshot && context.mode != StorageMode::GlobalFirst {
        return Err(error(
            "unsupported_mode",
            "repo-first changes belong to the user's code history",
        ));
    }
    let before = documents::collect(context)?;
    let committed = if context.mode == StorageMode::GlobalFirst {
        documents::admit_global_tree(context)?;
        let head = locked.revision(api)?;
        if Some(&head) != context.spec_revision.as_ref() {
            return Err(error("revision_changed", "private spec ref changed"));
        }
        let files = committed_files(api, locked, &head)?;
        if !snapshot && files != before {
            return Err(error(
                "dirty_tree",
                "snapshot raw document edits before a managed mutation",
            ));
        }
        Some(files)
    } else {
        None
    };
    api.check_control()?;
    let (changes, response_shape) = plan(context)?;
    let mut seen = BTreeSet::new();
    let mut after = before.clone();
    let mut writes = Vec::new();
    for change in changes {
        if !seen.insert(change.path.clone()) {
            return Err(error("invalid_document", "duplicate write target"));
        }
        let target = documents::path(context, &change.path)?;
        if let Some(content) = &change.content {
            std::str::from_utf8(content)
                .map_err(|_| error("invalid_document", "managed Markdown must be UTF-8"))?;
        }
        let original = documents::bytes(context, &change.path)?;
        if change.content.is_none() && original.is_none() {
            return Err(error(
                "document_not_found",
                "cannot remove a missing document",
            ));
        }
        match &change.content {
            Some(content) => {
                after.insert(change.path.clone(), content.clone());
            }
            None => {
                after.remove(&change.path);
            }
        }
        let core = vibe_core::fs::Write {
            path: target,
            show: change.path.clone(),
            original: original.clone(),
            bytes: change.content.clone().unwrap_or_default(),
        };
        writes.push((core, change.content.is_none()));
    }
    // Both explicit changes and raw snapshot imports must be admitted Markdown bytes.
    for (name, content) in &after {
        documents::path(context, name)?;
        std::str::from_utf8(content)
            .map_err(|_| error("invalid_document", "snapshot Markdown must be UTF-8"))?;
    }
    api.admit_files(&after)?;
    let bound_after = features(&after);
    for name in after.keys() {
        for (at, _) in name.match_indices('/') {
            if after.contains_key(&name[..at]) {
                return Err(error(
                    "invalid_document",
                    "a planned document cannot also be another document's directory",
                ));
            }
        }
    }
    for feature in completed_features {
        if !crate::query::valid_name(feature) || bound_after.contains(feature) {
            return Err(error(
                "invalid_completion",
                "completion requires a valid feature with no remaining bound artifacts",
            ));
        }
    }
    let mut lifecycle_diagnostics = Vec::new();
    if let (Some(history), Some(project), Some(head)) =
        (&locked.history, &context.project_id, &context.spec_revision)
    {
        let mut heads = vec![head.as_str()];
        if let Some(merge) = merge {
            heads.push(merge.source);
        }
        for feature in features(&after) {
            if feature_evidence(api, history, project, &heads, &feature)?
                == FeatureEvidence::Ambiguous
            {
                lifecycle_diagnostics.push(json!({"code":"ambiguous_evidence","feature":feature,"message":"incomparable lifecycle events; completion requires unambiguous recorded evidence"}));
            }
        }
    }
    let unchanged = resolve_selection(api, selector, legacy, false)?;
    registry::expect(&unchanged.revision_token, &context.revision_token)?;
    let entries: Vec<_> = writes.iter().map(|(w, remove)| {
        let content = if *remove { None } else { Some(w.bytes.clone()) };
        json!({"path":w.show,"original":w.original,"original_hash":digest(&w.original),"written":content,"written_hash":digest(&content)})
    }).collect();
    let mut journal = json!({"v":1,"mode":context.mode.as_str(),"checkout":context.checkout.as_deref().map(physical_path_value).transpose()?,"code_ref":context.code_ref,"code_head":context.code_head,"spec_root":physical_path_value(&context.spec_root)?,"artifact_root":physical_path_value(&context.artifact_root)?,"spec_ref":locked.spec_ref,"base":context.spec_revision,"planned":null,"state":"prepared","entries":entries});
    if let Some(merge) = merge {
        journal["reconciliation"] = merge.record_id.as_str().into();
    }
    api.check_control()?;
    locked.record(&journal)?;
    api.check_control()?;
    checkpoint(Checkpoint::BeforeWrites)?;
    let mut effect_path = None;
    let effect = (|| {
        for (write, remove) in &writes {
            api.check_control()?;
            effect_path = Some(write.show.clone());
            locked.verify()?;
            let target = documents::path(context, &write.show)?;
            if documents::bytes(context, &write.show)? != write.original {
                return Err(error("revision_changed", "document changed before write"));
            }
            if *remove {
                fs::remove_file(target).map_err(io)?;
            } else {
                let mode = fs::metadata(&target).ok().map(|m| m.permissions());
                fs::create_dir_all(target.parent().ok_or_else(|| io("target has no parent"))?)
                    .map_err(io)?;
                vibe_core::fs::write_atomic(&target, &write.bytes).map_err(io)?;
                if let Some(mode) = mode {
                    fs::set_permissions(&target, mode).map_err(io)?;
                }
            }
        }
        Ok(())
    })();
    if let Err(mut cause) = effect {
        let restored: Vec<_> = writes
            .iter()
            .filter(|(write, remove)| {
                let written = if *remove {
                    None
                } else {
                    Some(write.bytes.clone())
                };
                written != write.original
                    && documents::bytes(context, &write.show)
                        .is_ok_and(|current| current == written)
            })
            .map(|(write, _)| write.show.clone())
            .collect();
        recover_authorized(api, context, locked, selector, legacy)?;
        cause.details = json!({"path":effect_path,"restored":restored});
        return Err(cause);
    }
    api.check_control()?;
    // The planned bytes are now present under the journal. Validate the complete
    // response before canonical publication; its opaque hash fields keep their size.
    let planned_diagnostics = documents::diagnostics(&api.runtime, context);
    api.admit_result(&json!({"context":storage::context_value(context),"data":response_shape.value(json!({"context":storage::context_value(context),"diagnostics":planned_diagnostics,"lifecycle_diagnostics":lifecycle_diagnostics,"recorded":true,"validated":false}))}))?;
    checkpoint(Checkpoint::AfterWrites)?;
    // Detect raw edits and code/registry retargeting before history publication.
    let recheck = || -> Result<(), SpecError> {
        let fresh = resolve_selection(api, selector, legacy, false)?;
        let registry_after = registry::load(&api.runtime)?;
        let project_after = context
            .project_id
            .as_ref()
            .map(|id| registry::load_project(&registry_after, id))
            .transpose()?
            .map(|state| state.revision);
        if registry_before.revision != registry_after.revision
            || project_before != project_after
            || context.project_id != fresh.project_id
            || context.checkout != fresh.checkout
            || context.code_ref != fresh.code_ref
            || context.code_head != fresh.code_head
            || context.spec_root != fresh.spec_root
            || context.artifact_root != fresh.artifact_root
            || context.spec_revision != fresh.spec_revision
            || documents::collect(&fresh)? != after
        {
            return Err(error(
                "revision_changed",
                "context or document bytes changed during transaction",
            ));
        }
        Ok(())
    };
    if let Err(cause) = recheck() {
        recover_authorized(api, context, locked, selector, legacy)?;
        return Err(cause);
    }
    if context.mode == StorageMode::GlobalFirst {
        let base = context
            .spec_revision
            .as_deref()
            .ok_or_else(|| error("invalid_storage", "spec revision missing"))?;
        let publication = (|| {
            api.check_control()?;
            let tree = write_tree(api, locked, &after)?;
            let events = if merge.is_some() {
                Vec::new()
            } else {
                introductions(
                    api,
                    locked,
                    committed
                        .as_ref()
                        .ok_or_else(|| io("missing committed tree"))?,
                    &after,
                    base,
                )?
            };
            let message = format!(
                "Record spec documents\n\nVibe-Spec-Metadata: {}",
                json!({"v":1,"project":context.project_id.as_ref().map(|id|id.as_str()),"branch":locked.spec_ref,"feature_introductions":events,"feature_completions":completed_features})
            );
            let message = if let Some(merge) = merge {
                format!("{message}\nVibe-Spec-Reconciliation: {}", merge.record_id)
            } else {
                message
            };
            let mut args = vec!["commit-tree", &tree, "-p", base];
            if let Some(merge) = merge {
                args.extend(["-p", merge.source]);
            }
            args.extend(["-m", &message]);
            let planned = if snapshot && committed.as_ref() == Some(&after) {
                base.to_owned()
            } else {
                locked.text(api, &args, None)?
            };
            if snapshot {
                journal["snapshot_metadata"] =
                    crate::refresh::capture(api, context, &planned)?.unwrap_or(Value::Null);
            }
            journal["planned"] = planned.clone().into();
            locked.record(&journal)?;
            api.check_control()?;
            checkpoint(Checkpoint::BeforePublish)?;
            api.check_control()?;
            locked.verify()?;
            recheck()?;
            if documents::collect(context)? != after {
                return Err(error(
                    "revision_changed",
                    "document bytes changed before publication",
                ));
            }
            locked.text(
                api,
                &[
                    "update-ref",
                    locked
                        .spec_ref
                        .as_deref()
                        .ok_or_else(|| io("spec ref missing"))?,
                    &planned,
                    base,
                ],
                None,
            )?;
            journal["state"] = "committed".into();
            locked.record(&journal)?;
            Ok(())
        })();
        if let Err(cause) = publication {
            // A checkpoint refusal is an interruption; leave its journal for deterministic recovery.
            if journal["planned"].is_null() {
                recover_authorized(api, context, locked, selector, legacy)?;
            }
            return Err(cause);
        }
    } else {
        api.check_control()?;
        checkpoint(Checkpoint::BeforePublish)?;
        api.check_control()?;
        recheck()?;
        journal["state"] = "committed".into();
        locked.record(&journal)?;
    }
    let _completion = api.completing();
    checkpoint(Checkpoint::AfterPublish)?;
    crate::refresh::finish_capture(api, context, &journal["snapshot_metadata"])?;
    if let Some(merge) = merge {
        crate::reconcile::finish_receipt(
            api,
            &locked.anchor,
            merge.record_id,
            journal["planned"]
                .as_str()
                .ok_or_else(|| io("planned result missing"))?,
        )?;
    }
    locked.clear()?;
    let context = resolve_selection(api, selector, legacy, true)?;
    Ok(SpecResponse {
        data: response_shape.value(json!({"context":storage::context_value(&context),"diagnostics":planned_diagnostics,"lifecycle_diagnostics":lifecycle_diagnostics,"recorded":true,"validated":false})),
        context: Some(context),
    })
}

/// Publish a completed isolated tree through the ordinary journal, ownership checks and CAS.
pub(crate) fn publish_merge(
    api: &SpecApi,
    selector: &ContextSelector,
    context: &SpecContext,
    locked: &Locked,
    files: Files,
    merge: &MergePublication<'_>,
    checkpoint: impl FnMut(Checkpoint) -> Result<(), SpecError>,
) -> Result<SpecResponse, SpecError> {
    apply_locked(
        api,
        selector,
        None,
        false,
        &[],
        context,
        locked,
        Some(merge),
        |context| {
            let before = documents::collect(context)?;
            let paths: BTreeSet<_> = before.keys().chain(files.keys()).cloned().collect();
            Ok((
                paths
                    .into_iter()
                    .filter(|p| before.get(p) != files.get(p))
                    .map(|path| Change {
                        content: files.get(&path).cloned(),
                        path,
                    })
                    .collect(),
                PlannedResponse::Receipt,
            ))
        },
        checkpoint,
    )
}

fn digest(bytes: &Option<Vec<u8>>) -> Option<String> {
    bytes.as_deref().map(vibe_core::markers::hash_bytes)
}
fn journal_bytes(value: &Value) -> Result<Option<Vec<u8>>, SpecError> {
    if value.is_null() {
        return Ok(None);
    }
    let values = value
        .as_array()
        .ok_or_else(|| error("invalid_transaction", "invalid journal bytes"))?;
    values
        .iter()
        .map(|v| {
            v.as_u64()
                .and_then(|n| u8::try_from(n).ok())
                .ok_or_else(|| error("invalid_transaction", "invalid journal byte"))
        })
        .collect::<Result<Vec<_>, _>>()
        .map(Some)
}
fn physical_path_value(path: &Path) -> Result<Value, SpecError> {
    registry::confined(path, path)?;
    Ok(registry::path_value(&storage::future_physical(path)?))
}
fn journal_root_matches(saved: &Value, current: &Path) -> Result<bool, SpecError> {
    registry::confined(current, current)?;
    let physical = storage::future_physical(current)?;
    let Ok(saved) = registry::path(saved) else {
        return Ok(false);
    };
    // Admit old ordinary Windows spellings lexically, never follow a saved alias.
    if registry::confined(&saved, &saved).is_err() {
        return Ok(false);
    }
    Ok(crate::git::argument_path(&saved) == crate::git::argument_path(&physical))
}
fn recover(api: &SpecApi, context: &SpecContext, locked: &Locked) -> Result<(), SpecError> {
    api.check_control()?;
    registry::confined(&locked.anchor, &locked.journal)?;
    let journal = match fs::read(&locked.journal) {
        Ok(bytes) => serde_json::from_slice::<Value>(&bytes)
            .map_err(|e| error("invalid_transaction", e.to_string()))?,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(io(e)),
    };
    if journal["v"] != 1
        || journal["mode"] != context.mode.as_str()
        || !journal_root_matches(&journal["spec_root"], &context.spec_root)?
        || !journal_root_matches(&journal["artifact_root"], &context.artifact_root)?
        || journal["spec_ref"]
            != locked
                .spec_ref
                .clone()
                .map(Value::from)
                .unwrap_or(Value::Null)
    {
        return Err(error(
            "recovery_conflict",
            "journal belongs to a different document context",
        ));
    }
    // Repo-first roots are reused by every code branch. Absence after an
    // interrupted deletion is transaction-owned only in the exact saved context.
    if context.mode == StorageMode::RepoFirst
        && (match context.checkout.as_deref() {
            Some(checkout) => !journal_root_matches(&journal["checkout"], checkout)?,
            None => !journal["checkout"].is_null(),
        } || journal.get("code_ref")
            != Some(
                &context
                    .code_ref
                    .clone()
                    .map(Value::from)
                    .unwrap_or(Value::Null),
            )
            || journal.get("code_head")
                != Some(
                    &context
                        .code_head
                        .clone()
                        .map(Value::from)
                        .unwrap_or(Value::Null),
                ))
    {
        return Err(error(
            "recovery_conflict",
            "the selected checkout, code branch or code head differs from the interrupted transaction",
        ));
    }
    let published = if context.mode == StorageMode::GlobalFirst {
        let current = locked.revision(api)?;
        if journal["planned"].as_str() == Some(&current) {
            journal["planned"] != journal["base"] || journal["state"] == "committed"
        } else if journal["base"].as_str() == Some(&current) {
            false
        } else if journal["reconciliation"].is_string() && journal["planned"].is_string() {
            let planned = journal["planned"]
                .as_str()
                .ok_or_else(|| error("invalid_transaction", "missing result"))?;
            let output = storage::spec_git_output(
                &api.runtime,
                locked
                    .history
                    .as_deref()
                    .ok_or_else(|| error("invalid_storage", "history missing"))?,
                None,
                &["merge-base", "--is-ancestor", planned, &current].map(std::ffi::OsString::from),
                None,
            )?;
            if !output.status.success() {
                return Err(error(
                    "recovery_conflict",
                    "spec ref diverged from reconciliation result",
                ));
            }
            true
        } else {
            return Err(error(
                "recovery_conflict",
                "spec ref diverged from the interrupted transaction",
            ));
        }
    } else {
        journal["state"] == "committed"
    };
    if published {
        let _completion = api.completing();
        crate::refresh::finish_capture(api, context, &journal["snapshot_metadata"])?;
        if let Some(record) = journal["reconciliation"].as_str() {
            let id = record.parse::<crate::RecordId>()?;
            crate::reconcile::finish_receipt(
                api,
                &locked.anchor,
                &id,
                journal["planned"]
                    .as_str()
                    .ok_or_else(|| error("invalid_transaction", "missing reconciliation result"))?,
            )?;
        }
        return locked.clear();
    }
    // Validate every entry before any restoration. Corrupt journals cannot direct partial recovery.
    let mut entries = Vec::new();
    let mut seen = BTreeSet::new();
    for entry in journal["entries"]
        .as_array()
        .ok_or_else(|| error("invalid_transaction", "journal entries missing"))?
    {
        let path = entry["path"]
            .as_str()
            .ok_or_else(|| error("invalid_transaction", "journal path missing"))?;
        documents::path(context, path)?;
        if !seen.insert(path.to_owned()) {
            return Err(error("invalid_transaction", "duplicate recovery path"));
        }
        let original = journal_bytes(&entry["original"])?;
        let written = journal_bytes(&entry["written"])?;
        if entry["original_hash"] != digest(&original).map(Value::from).unwrap_or(Value::Null)
            || entry["written_hash"] != digest(&written).map(Value::from).unwrap_or(Value::Null)
        {
            return Err(error(
                "invalid_transaction",
                "journal content hash mismatch",
            ));
        }
        entries.push((path.to_owned(), original, written));
    }
    let mut conflicts = Vec::new();
    for (logical, original, written) in entries {
        api.check_control()?;
        let current = documents::bytes(context, &logical)?;
        if current == original {
            continue;
        }
        if current != written {
            conflicts.push(logical);
            continue;
        }
        locked.verify()?;
        let target = documents::path(context, &logical)?;
        match original {
            Some(bytes) => {
                fs::create_dir_all(target.parent().ok_or_else(|| io("target parent missing"))?)
                    .map_err(io)?;
                vibe_core::fs::write_atomic(&target, &bytes).map_err(io)?;
            }
            None => fs::remove_file(target).map_err(io)?,
        }
    }
    if !conflicts.is_empty() {
        let mut err = error(
            "recovery_conflict",
            "external document edits were preserved; restore or reconcile the listed paths before retrying recovery",
        );
        err.details = json!({"paths":conflicts});
        return Err(err);
    }
    api.check_control()?;
    locked.clear()
}

pub(crate) fn committed_files(
    api: &SpecApi,
    locked: &Locked,
    revision: &str,
) -> Result<Files, SpecError> {
    let listing = locked.git(api, &["ls-tree", "-rz", "--full-tree", revision], None)?;
    let mut files = Files::new();
    for record in listing.split(|b| *b == 0).filter(|r| !r.is_empty()) {
        let (header, path) = record
            .split_once_byte(b'\t')
            .ok_or_else(|| io("invalid Git tree record"))?;
        let path = std::str::from_utf8(path).map_err(io)?;
        let fields: Vec<_> = std::str::from_utf8(header)
            .map_err(io)?
            .split(' ')
            .collect();
        if fields.len() != 3 || fields[0] != "100644" || fields[1] != "blob" {
            return Err(error(
                "invalid_storage",
                "unsupported entry in managed history",
            ));
        }
        if !(path.starts_with("spec/") || path.starts_with("superpowers/"))
            || !path.ends_with(".md")
        {
            return Err(error("invalid_storage", "foreign file in managed history"));
        }
        files.insert(
            path.into(),
            locked.git(api, &["cat-file", "blob", fields[2]], None)?,
        );
    }
    Ok(files)
}
// Slice split_once is not available on the project's MSRV.
trait SplitByte {
    fn split_once_byte(&self, byte: u8) -> Option<(&[u8], &[u8])>;
}
impl SplitByte for [u8] {
    fn split_once_byte(&self, byte: u8) -> Option<(&[u8], &[u8])> {
        let at = self.iter().position(|b| *b == byte)?;
        Some((&self[..at], &self[at + 1..]))
    }
}
fn write_tree(api: &SpecApi, locked: &Locked, files: &Files) -> Result<String, SpecError> {
    #[derive(Default)]
    struct Tree {
        files: BTreeMap<String, String>,
        dirs: BTreeMap<String, Tree>,
    }
    fn emit(api: &SpecApi, locked: &Locked, tree: Tree) -> Result<String, SpecError> {
        let mut records = BTreeMap::new();
        api.check_control()?;
        for (name, oid) in tree.files {
            records.insert(name.clone(), format!("100644 blob {oid}\t{name}\0"));
        }
        for (name, subtree) in tree.dirs {
            let oid = emit(api, locked, subtree)?;
            records.insert(format!("{name}/"), format!("040000 tree {oid}\t{name}\0"));
        }
        let bytes = records.into_values().collect::<String>();
        locked.text(api, &["mktree", "-z"], Some(bytes.as_bytes()))
    }
    let mut tree = Tree::default();
    for (path, bytes) in files {
        api.check_control()?;
        let parts: Vec<_> = path.split('/').collect();
        let mut at = &mut tree;
        for part in &parts[..parts.len() - 1] {
            at = at.dirs.entry((*part).into()).or_default();
        }
        let oid = locked.text(
            api,
            &["hash-object", "-w", "--stdin", "--no-filters"],
            Some(bytes),
        )?;
        at.files.insert(parts[parts.len() - 1].into(), oid);
    }
    emit(api, locked, tree)
}
fn features(files: &Files) -> BTreeSet<String> {
    files
        .keys()
        .filter_map(|path| {
            if let Some(path) = path.strip_prefix("spec/features/") {
                let (feature, _) = path.split_once('/')?;
                return crate::query::valid_name(feature).then(|| feature.into());
            }
            let path = path.strip_prefix("superpowers/")?;
            let (dir, name) = path.split_once('/')?;
            let suffix = match dir {
                "specs" => "-design.md",
                "plans" => ".md",
                _ => return None,
            };
            let stem = name.strip_suffix(suffix)?;
            let date = stem.as_bytes().get(..10)?;
            if !crate::text::is_date(date) || stem.as_bytes().get(10) != Some(&b'-') {
                return None;
            }
            let feature = stem.get(11..)?;
            crate::query::valid_name(feature).then(|| feature.into())
        })
        .collect()
}
#[derive(Clone, Debug, PartialEq, Eq)]
enum FeatureEvidence {
    Missing,
    Active(String),
    Completed,
    Ambiguous,
}

/// All-parent ancestry-maximal relevant-event frontier; timestamps and parent order carry no authority.
fn feature_evidence(
    api: &SpecApi,
    history: &std::path::Path,
    project: &crate::ProjectId,
    heads: &[&str],
    feature: &str,
) -> Result<FeatureEvidence, SpecError> {
    let git = |args: &[&str]| {
        storage::spec_git_input(
            &api.runtime,
            history,
            None,
            &args
                .iter()
                .map(std::ffi::OsString::from)
                .collect::<Vec<_>>(),
            None,
        )
    };
    let mut args = vec!["log", "-z", "--format=%H%n%P%n%B"];
    args.extend_from_slice(heads);
    let log = git(&args)?;
    let mut events = BTreeMap::<String, Option<String>>::new();
    for entry in log.split(|b| *b == 0).filter(|e| !e.is_empty()) {
        let text = std::str::from_utf8(entry).map_err(io)?;
        let mut lines = text.lines();
        let oid = lines
            .next()
            .ok_or_else(|| error("invalid_history", "missing event commit"))?;
        let parents = lines.next().unwrap_or_default();
        let records: Vec<_> = lines
            .filter_map(|l| l.strip_prefix("Vibe-Spec-Metadata: "))
            .collect();
        if records.len() > 1 {
            return Err(error("invalid_history", "multiple lifecycle records"));
        }
        let Some(record) = records.first() else {
            continue;
        };
        let value: Value = serde_json::from_str(record)
            .map_err(|_| error("invalid_history", "malformed lifecycle metadata"))?;
        validate_lifecycle_metadata(&value)?;
        if value["project"].as_str() != Some(project.as_str()) {
            return Err(error("invalid_history", "lifecycle project differs"));
        }
        for event in value["feature_introductions"]
            .as_array()
            .into_iter()
            .flatten()
        {
            let baseline = event["baseline"]
                .as_str()
                .ok_or_else(|| error("invalid_history", "missing baseline"))?;
            if parents.split_whitespace().next() != Some(baseline) {
                return Err(error(
                    "invalid_history",
                    "baseline does not bind the introduction parent",
                ));
            }
            if event["feature"].as_str() == Some(feature) {
                events.insert(oid.into(), Some(baseline.into()));
            }
        }
        if value["feature_completions"]
            .as_array()
            .is_some_and(|e| e.iter().any(|f| f.as_str() == Some(feature)))
        {
            events.insert(oid.into(), None);
        }
    }
    let mut frontier = Vec::new();
    for (oid, event) in &events {
        let mut dominated = false;
        for other in events.keys().filter(|o| *o != oid) {
            let out = storage::spec_git_output(
                &api.runtime,
                history,
                None,
                &["merge-base", "--is-ancestor", oid, other].map(std::ffi::OsString::from),
                None,
            )?;
            if out.status.success() {
                dominated = true;
                break;
            }
            if out.status.code() != Some(1) {
                return Err(error(
                    "spec_git_failed",
                    String::from_utf8_lossy(&out.stderr),
                ));
            }
        }
        if !dominated {
            frontier.push(event.clone());
        }
    }
    if frontier.is_empty() {
        return Ok(FeatureEvidence::Missing);
    }
    let completed = frontier.iter().any(Option::is_none);
    let active: BTreeSet<_> = frontier.into_iter().flatten().collect();
    Ok(if active.is_empty() {
        FeatureEvidence::Completed
    } else if completed || active.len() != 1 {
        FeatureEvidence::Ambiguous
    } else {
        FeatureEvidence::Active(
            active
                .into_iter()
                .next()
                .ok_or_else(|| error("invalid_history", "empty active frontier"))?,
        )
    })
}
fn introductions(
    api: &SpecApi,
    locked: &Locked,
    before: &Files,
    after: &Files,
    base: &str,
) -> Result<Vec<Value>, SpecError> {
    let project = locked
        .anchor
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| error("invalid_history", "missing project identity"))?
        .parse::<crate::ProjectId>()?;
    let history = locked
        .history
        .as_deref()
        .ok_or_else(|| error("invalid_history", "missing history"))?;
    let mut events = Vec::new();
    for feature in features(after).difference(&features(before)) {
        match feature_evidence(api, history, &project, &[base], feature)? {
            FeatureEvidence::Missing | FeatureEvidence::Completed => {
                events.push(json!({"feature":feature,"baseline":base}))
            }
            FeatureEvidence::Active(_) | FeatureEvidence::Ambiguous => (),
        }
    }
    Ok(events)
}
/// The caller holds the project lock. Durable proof excludes every feature artifact.
pub(crate) fn feature_baseline(
    api: &SpecApi,
    context: &SpecContext,
    feature: &str,
) -> Result<String, SpecError> {
    let registry = registry::load(&api.runtime)?;
    let id = context
        .project_id
        .as_ref()
        .ok_or_else(|| error("invalid_context", "project missing"))?;
    let history = registry::project_dir(&registry, id).join("history.git");
    let head = context
        .spec_revision
        .as_deref()
        .ok_or_else(|| error("missing_evidence", "spec history unavailable"))?;
    let baseline = match feature_evidence(api, &history, id, &[head], feature)? {
        FeatureEvidence::Active(baseline) => baseline,
        FeatureEvidence::Ambiguous => {
            return Err(error(
                "ambiguous_evidence",
                "incomparable lifecycle evidence",
            ));
        }
        FeatureEvidence::Missing | FeatureEvidence::Completed => {
            return Err(error(
                "missing_evidence",
                "no active recorded introduction baseline",
            ));
        }
    };
    let git = |args: &[&str]| {
        storage::spec_git_input(
            &api.runtime,
            &history,
            None,
            &args
                .iter()
                .map(std::ffi::OsString::from)
                .collect::<Vec<_>>(),
            None,
        )
    };
    git(&["merge-base", "--is-ancestor", &baseline, head])
        .map_err(|_| error("missing_evidence", "feature baseline unreachable"))?;
    git(&["rev-parse", "--verify", &format!("{baseline}:spec")])
        .map_err(|_| error("missing_evidence", "baseline has no durable spec tree"))?;
    git(&["rev-parse", "--verify", &format!("{head}:spec")]).map_err(|_| {
        error(
            "missing_evidence",
            "selected revision has no durable spec tree",
        )
    })?;
    let durable = |revision: &str| -> Result<Vec<Vec<u8>>, SpecError> {
        let entries = git(&["ls-tree", "-r", "-z", revision, "--", "spec"])?;
        Ok(entries
            .split(|b| *b == 0)
            .filter(|e| !e.is_empty())
            .filter_map(|entry| {
                let at = entry.iter().position(|b| *b == b'\t')?;
                (!entry[at + 1..].starts_with(b"spec/features/")).then(|| entry.to_vec())
            })
            .collect())
    };
    if durable(&baseline)? == durable(head)? {
        return Err(error(
            "promotion_required",
            "durable spec is unchanged since feature introduction",
        ));
    }
    Ok(baseline)
}

/// Changed managed paths in the selected spec commit; caller owns the project lock.
pub(crate) fn changed_documents(
    api: &SpecApi,
    context: &SpecContext,
) -> Result<BTreeSet<String>, SpecError> {
    if context.mode != StorageMode::GlobalFirst {
        return Ok(documents::collect(context)?.into_keys().collect());
    }
    let registry = registry::load(&api.runtime)?;
    let id = context
        .project_id
        .as_ref()
        .ok_or_else(|| error("invalid_context", "project is missing"))?;
    let history = registry::project_dir(&registry, id).join("history.git");
    let head = context
        .spec_revision
        .as_deref()
        .ok_or_else(|| error("invalid_history", "spec revision is missing"))?;
    let git = |args: &[&str]| {
        storage::spec_git_input(
            &api.runtime,
            &history,
            None,
            &args
                .iter()
                .map(std::ffi::OsString::from)
                .collect::<Vec<_>>(),
            None,
        )
    };
    let parents = git(&["rev-list", "--parents", "--max-count=1", head])?;
    let parents = std::str::from_utf8(&parents).map_err(io)?;
    let Some(parent) = parents.split_whitespace().nth(1) else {
        return Ok(documents::collect(context)?.into_keys().collect());
    };
    let changed = git(&[
        "diff",
        "--name-only",
        "-z",
        parent,
        head,
        "--",
        "spec",
        "superpowers",
    ])?;
    changed
        .split(|b| *b == 0)
        .filter(|b| !b.is_empty())
        .map(|p| std::str::from_utf8(p).map(str::to_owned).map_err(io))
        .collect()
}

fn validate_lifecycle_metadata(value: &Value) -> Result<(), SpecError> {
    let invalid = || error("invalid_history", "malformed lifecycle metadata");
    let object = value.as_object().ok_or_else(invalid)?;
    if object.len() != 5
        || value["v"] != 1
        || value["project"]
            .as_str()
            .and_then(|s| s.parse::<crate::ProjectId>().ok())
            .is_none()
        || value["branch"].as_str().is_none_or(|s| {
            crate::context::code_ref(s).ok().as_deref() != Some(s)
                || s.strip_prefix("refs/heads/spec/")
                    .and_then(|id| id.parse::<crate::BranchId>().ok())
                    .is_none()
        })
    {
        return Err(invalid());
    }
    let mut names = BTreeSet::new();
    for event in value["feature_introductions"]
        .as_array()
        .ok_or_else(invalid)?
    {
        let feature = event["feature"]
            .as_str()
            .filter(|f| crate::query::valid_name(f))
            .ok_or_else(invalid)?;
        if event.as_object().is_none_or(|o| o.len() != 2)
            || !event["baseline"]
                .as_str()
                .is_some_and(|b| b.len() == 40 && b.bytes().all(|c| c.is_ascii_hexdigit()))
            || !names.insert(feature)
        {
            return Err(invalid());
        }
    }
    for feature in value["feature_completions"]
        .as_array()
        .ok_or_else(invalid)?
    {
        let feature = feature
            .as_str()
            .filter(|f| crate::query::valid_name(f))
            .ok_or_else(invalid)?;
        if !names.insert(feature) {
            return Err(invalid());
        }
    }
    Ok(())
}

#[cfg(test)]
pub(crate) mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;
    use crate::{InitProjectRequest, Runtime, SpecRequest};
    use std::{
        path::Path,
        sync::{Mutex, mpsc},
    };
    static WAITING: Mutex<BTreeMap<PathBuf, mpsc::Sender<()>>> = Mutex::new(BTreeMap::new());
    type OidHook = (mpsc::Sender<()>, mpsc::Receiver<()>);
    static AFTER_OID: Mutex<BTreeMap<PathBuf, OidHook>> = Mutex::new(BTreeMap::new());
    pub(crate) fn waiting(path: &Path) {
        if let Some(sender) = WAITING.lock().unwrap().remove(path) {
            sender.send(()).unwrap();
        }
    }
    pub(crate) fn after_spec_oid(root: &Path) {
        let hook = AFTER_OID.lock().unwrap().remove(root);
        if let Some((entered, proceed)) = hook {
            entered.send(()).unwrap();
            proceed
                .recv_timeout(std::time::Duration::from_secs(10))
                .unwrap();
        }
    }
    pub(crate) fn record_lock(api: &SpecApi, context: &SpecContext) -> Locked {
        Locked::acquire(api, context).unwrap()
    }
    pub(crate) fn fixture() -> (vibe_core::testscratch::Scratch, SpecApi, SpecContext) {
        fixture_mode(StorageMode::GlobalFirst)
    }
    fn fixture_mode(mode: StorageMode) -> (vibe_core::testscratch::Scratch, SpecApi, SpecContext) {
        let owner = vibe_core::testscratch::scratch("spec-transactions", "failure-controls");
        let code = owner.join("code");
        fs::create_dir_all(&code).unwrap();
        for args in [
            vec!["init", "-b", "main"],
            vec![
                "-c",
                "user.name=test",
                "-c",
                "user.email=test@example.invalid",
                "-c",
                "commit.gpgsign=false",
                "commit",
                "--allow-empty",
                "-m",
                "initial",
            ],
        ] {
            let out = std::process::Command::new("git")
                .args(args)
                .current_dir(&code)
                .env_clear()
                .env("PATH", std::env::var_os("PATH").unwrap_or_default())
                .env("HOME", owner.as_os_str())
                .output()
                .unwrap();
            assert!(
                out.status.success(),
                "{}",
                String::from_utf8_lossy(&out.stderr)
            );
        }
        let api = SpecApi::new(Runtime {
            cwd: code,
            env: BTreeMap::from([
                ("PATH".into(), std::env::var_os("PATH").unwrap_or_default()),
                ("HOME".into(), owner.as_os_str().to_owned()),
                ("VIBE_STORE".into(), owner.join("store").into_os_string()),
            ]),
        })
        .unwrap();
        let response = api
            .execute(SpecRequest::ProjectInit(InitProjectRequest {
                selector: ContextSelector::default(),
                mode,
                name: None,
                expected_revision: "absent".into(),
            }))
            .unwrap();
        (owner, api, response.context.unwrap())
    }
    fn change(path: &str, content: Option<&[u8]>) -> Change {
        Change {
            path: path.into(),
            content: content.map(Vec::from),
        }
    }
    fn interrupted() -> SpecError {
        error("transaction_interrupted", "injected interruption")
    }
    fn execute_failure(api: &SpecApi, context: &SpecContext, changes: Vec<Change>, at: Checkpoint) {
        let err = apply_planned(
            api,
            &ContextSelector::default(),
            &context.revision_token,
            false,
            |_| Ok(changes),
            |point| {
                if point == at {
                    Err(interrupted())
                } else {
                    Ok(())
                }
            },
        )
        .unwrap_err();
        assert_eq!(err.code, "transaction_interrupted");
        assert_eq!(
            api.resolve_context(&ContextSelector::default())
                .unwrap_err()
                .code,
            "recovery_required"
        );
    }
    #[test]
    fn interrupted_writes_recover_exact_original_bytes_and_refs() {
        for checkpoint in [
            Checkpoint::BeforeWrites,
            Checkpoint::AfterWrites,
            Checkpoint::BeforePublish,
        ] {
            let (_owner, api, context) = fixture();
            let original = fs::read(context.spec_root.join("product.md")).unwrap();
            execute_failure(
                &api,
                &context,
                vec![
                    change("spec/product.md", Some(b"replacement\r\n")),
                    change("spec/new.md", Some(b"new")),
                ],
                checkpoint,
            );
            let response =
                documents::read(&api, &ContextSelector::default(), "spec/product.md").unwrap();
            assert_eq!(
                response.data["content"].as_str().unwrap().as_bytes(),
                original
            );
            assert!(!context.spec_root.join("new.md").exists());
            assert_eq!(
                response.context.unwrap().spec_revision,
                context.spec_revision
            );
        }
    }
    #[test]
    fn recovery_preserves_post_crash_edits_to_existing_and_created_files() {
        let (_owner, api, context) = fixture();
        execute_failure(
            &api,
            &context,
            vec![
                change("spec/product.md", Some(b"transaction product")),
                change("spec/created.md", Some(b"transaction new")),
                change("spec/owned.md", Some(b"transaction owned")),
            ],
            Checkpoint::AfterWrites,
        );
        fs::write(
            context.spec_root.join("product.md"),
            b"external existing\r\n",
        )
        .unwrap();
        fs::write(context.spec_root.join("created.md"), b"external new\n").unwrap();
        let err =
            documents::read(&api, &ContextSelector::default(), "spec/product.md").unwrap_err();
        assert_eq!(err.code, "recovery_conflict");
        assert_eq!(
            err.details["paths"],
            json!(["spec/product.md", "spec/created.md"])
        );
        assert_eq!(
            fs::read(context.spec_root.join("product.md")).unwrap(),
            b"external existing\r\n"
        );
        assert_eq!(
            fs::read(context.spec_root.join("created.md")).unwrap(),
            b"external new\n"
        );
        assert!(!context.spec_root.join("owned.md").exists());
        let locked = Locked::acquire(&api, &context).unwrap();
        assert_eq!(
            locked.revision(&api).unwrap(),
            context.spec_revision.unwrap()
        );
    }
    #[test]
    fn published_ref_recovery_keeps_commit_and_later_editor_bytes() {
        let (_owner, api, context) = fixture();
        execute_failure(
            &api,
            &context,
            vec![change("spec/new.md", Some(b"committed bytes"))],
            Checkpoint::AfterPublish,
        );
        fs::write(context.spec_root.join("new.md"), b"post-publication edit").unwrap();
        let response = documents::read(&api, &ContextSelector::default(), "spec/new.md").unwrap();
        assert_eq!(response.data["content"], "post-publication edit");
        let new_context = response.context.unwrap();
        assert_ne!(new_context.spec_revision, context.spec_revision);
        let locked = Locked::acquire(&api, &new_context).unwrap();
        let object = format!("{}:spec/new.md", new_context.spec_revision.unwrap());
        assert_eq!(
            locked.git(&api, &["show", &object], None).unwrap(),
            b"committed bytes"
        );
    }
    #[test]
    fn duplicate_plan_and_late_raw_edit_never_publish() {
        let (_owner, api, context) = fixture();
        let err = apply(
            &api,
            &ContextSelector::default(),
            &context.revision_token,
            vec![
                change("spec/a.md", Some(b"one")),
                change("spec/a.md", Some(b"two")),
            ],
        )
        .unwrap_err();
        assert_eq!(err.code, "invalid_document");
        assert!(!context.spec_root.join("a.md").exists());
        let err = apply_planned(
            &api,
            &ContextSelector::default(),
            &context.revision_token,
            false,
            |_| Ok(vec![change("spec/a.md", Some(b"one"))]),
            |at| {
                if at == Checkpoint::BeforePublish {
                    fs::write(context.spec_root.join("a.md"), b"late external").unwrap();
                }
                Ok(())
            },
        )
        .unwrap_err();
        assert_eq!(err.code, "revision_changed");
        assert_eq!(
            fs::read(context.spec_root.join("a.md")).unwrap(),
            b"late external"
        );
        let locked = Locked::acquire(&api, &context).unwrap();
        assert_eq!(
            locked.revision(&api).unwrap(),
            context.spec_revision.unwrap()
        );
    }
    #[test]
    fn concurrent_expected_revision_has_exactly_one_winner() {
        let (_owner, api, context) = fixture();
        let barrier = std::sync::Arc::new(std::sync::Barrier::new(3));
        let handles: Vec<_> = ["one", "two"]
            .into_iter()
            .map(|content| {
                let api = api.clone();
                let context = context.clone();
                let barrier = barrier.clone();
                std::thread::spawn(move || {
                    barrier.wait();
                    apply(
                        &api,
                        &ContextSelector::default(),
                        &context.revision_token,
                        vec![change("spec/race.md", Some(content.as_bytes()))],
                    )
                })
            })
            .collect();
        barrier.wait();
        let results: Vec<_> = handles
            .into_iter()
            .map(|handle| handle.join().unwrap())
            .collect();
        assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
        assert_eq!(
            results
                .iter()
                .filter_map(|result| result.as_ref().err())
                .next()
                .unwrap()
                .code,
            "revision_changed"
        );
        let winner = results
            .into_iter()
            .find_map(Result::ok)
            .unwrap()
            .context
            .unwrap();
        let bytes = fs::read(context.spec_root.join("race.md")).unwrap();
        assert!(bytes == b"one" || bytes == b"two");
        let locked = Locked::acquire(&api, &winner).unwrap();
        let object = format!("{}:spec/race.md", winner.spec_revision.unwrap());
        assert_eq!(locked.git(&api, &["show", &object], None).unwrap(), bytes);
    }
    #[test]
    fn raw_snapshot_and_explicit_completion_bind_exact_feature_baselines() {
        let (_owner, api, context) = fixture();
        let artifact = context.artifact_root.join("plans/2026-11-11-bound.md");
        fs::write(&artifact, b"raw draft").unwrap();
        let dirty = api.resolve_context(&ContextSelector::default()).unwrap();
        let snapped = snapshot(&api, &ContextSelector::default(), &dirty.revision_token)
            .unwrap()
            .context
            .unwrap();
        let locked = Locked::acquire(&api, &snapped).unwrap();
        let message = locked
            .text(
                &api,
                &[
                    "log",
                    "-1",
                    "--format=%B",
                    snapped.spec_revision.as_ref().unwrap(),
                ],
                None,
            )
            .unwrap();
        assert!(message.contains(&format!(
            "\"baseline\":\"{}\"",
            context.spec_revision.unwrap()
        )));
        drop(locked);
        let done = apply_lifecycle_planned(
            &api,
            &ContextSelector::default(),
            &snapped.revision_token,
            false,
            &["bound".into()],
            |_| Ok(vec![change("superpowers/plans/2026-11-11-bound.md", None)]),
            |_| Ok(()),
        )
        .unwrap()
        .context
        .unwrap();
        let next = apply(
            &api,
            &ContextSelector::default(),
            &done.revision_token,
            vec![change(
                "superpowers/plans/2026-11-11-bound.md",
                Some(b"new lifecycle"),
            )],
        )
        .unwrap()
        .context
        .unwrap();
        let locked = Locked::acquire(&api, &next).unwrap();
        let message = locked
            .text(
                &api,
                &[
                    "log",
                    "-1",
                    "--format=%B",
                    next.spec_revision.as_ref().unwrap(),
                ],
                None,
            )
            .unwrap();
        let metadata: Value =
            serde_json::from_str(message.split("Vibe-Spec-Metadata: ").nth(1).unwrap()).unwrap();
        assert_eq!(
            metadata["feature_introductions"],
            json!([{"feature":"bound","baseline":done.spec_revision}])
        );
    }
    #[test]
    fn private_ref_cas_preserves_an_external_publication() {
        let (_owner, api, context) = fixture();
        let locked = Locked::acquire(&api, &context).unwrap();
        let base = context.spec_revision.as_ref().unwrap();
        let tree = locked
            .text(&api, &["rev-parse", &format!("{base}^{{tree}}")], None)
            .unwrap();
        let external = locked
            .text(
                &api,
                &[
                    "commit-tree",
                    &tree,
                    "-p",
                    base,
                    "-m",
                    "external publication",
                ],
                None,
            )
            .unwrap();
        let history = locked.history.clone().unwrap();
        let spec_ref = locked.spec_ref.clone().unwrap();
        drop(locked);
        let err = apply_planned(
            &api,
            &ContextSelector::default(),
            &context.revision_token,
            false,
            |_| Ok(vec![change("spec/cas.md", Some(b"transaction"))]),
            |at| {
                if at == Checkpoint::BeforePublish {
                    storage::git_text(
                        &api.runtime,
                        &history,
                        None,
                        &["update-ref", &spec_ref, &external, base],
                    )?;
                }
                Ok(())
            },
        )
        .unwrap_err();
        assert_eq!(err.code, "revision_changed");
        assert_eq!(
            storage::git_text(&api.runtime, &history, None, &["rev-parse", &spec_ref]).unwrap(),
            external
        );
        assert_eq!(
            documents::read(&api, &ContextSelector::default(), "spec/cas.md")
                .unwrap_err()
                .code,
            "recovery_conflict"
        );
        assert_eq!(
            fs::read(context.spec_root.join("cas.md")).unwrap(),
            b"transaction"
        );
    }
    #[test]
    fn plan_rejects_file_ancestor_collisions_before_effects() {
        let (_owner, api, context) = fixture();
        let err = apply(
            &api,
            &ContextSelector::default(),
            &context.revision_token,
            vec![
                change("spec/a.md", Some(b"ancestor")),
                change("spec/a.md/b.md", Some(b"descendant")),
            ],
        )
        .unwrap_err();
        assert_eq!(err.code, "invalid_document");
        assert!(!context.spec_root.join("a.md").exists());
        assert_eq!(
            api.resolve_context(&ContextSelector::default())
                .unwrap()
                .spec_revision,
            context.spec_revision
        );
    }
    #[test]
    fn checkout_switch_at_publication_checkpoint_cannot_publish() {
        let (owner, api, context) = fixture();
        let err = apply_planned(
            &api,
            &ContextSelector::default(),
            &context.revision_token,
            false,
            |_| Ok(vec![change("spec/switch.md", Some(b"should roll back"))]),
            |at| {
                if at == Checkpoint::BeforePublish {
                    let out = std::process::Command::new("git")
                        .args(["checkout", "-b", "switched"])
                        .current_dir(context.checkout.as_ref().unwrap())
                        .env_clear()
                        .env("PATH", std::env::var_os("PATH").unwrap_or_default())
                        .env("HOME", owner.as_os_str())
                        .output()
                        .unwrap();
                    assert!(
                        out.status.success(),
                        "{}",
                        String::from_utf8_lossy(&out.stderr)
                    );
                }
                Ok(())
            },
        )
        .unwrap_err();
        assert!(matches!(
            err.code.as_str(),
            "context_changed" | "revision_changed" | "branch_not_found"
        ));
        let locked = Locked::acquire(&api, &context).unwrap();
        assert_eq!(
            locked.revision(&api).unwrap(),
            context.spec_revision.unwrap()
        );
    }
    #[test]
    fn repo_first_recovery_never_restores_branch_a_into_branch_b() {
        let (_owner, api, _) = fixture_mode(StorageMode::RepoFirst);
        let git = |args: &[&str]| {
            let cx = crate::api::Execution::new(&api.runtime);
            let out = cx
                .command("git")
                .args([
                    "-c",
                    "user.name=test",
                    "-c",
                    "user.email=test@example.invalid",
                    "-c",
                    "commit.gpgsign=false",
                ])
                .args(args)
                .current_dir(&api.runtime.cwd)
                .output()
                .unwrap();
            assert!(
                out.status.success(),
                "{}",
                String::from_utf8_lossy(&out.stderr)
            );
        };
        git(&["add", "docs/spec"]);
        git(&["commit", "-m", "branch a specs"]);
        let context = api.resolve_context(&ContextSelector::default()).unwrap();
        let err = apply_planned(
            &api,
            &ContextSelector::default(),
            &context.revision_token,
            false,
            |_| Ok(vec![change("spec/product.md", None)]),
            |at| {
                if at == Checkpoint::AfterWrites {
                    Err(interrupted())
                } else {
                    Ok(())
                }
            },
        )
        .unwrap_err();
        assert_eq!(err.code, "transaction_interrupted");
        git(&["checkout", "-b", "branch-b"]);
        git(&["add", "-u", "docs/spec/product.md"]);
        git(&["commit", "-m", "branch b removes product"]);
        assert!(!context.spec_root.join("product.md").exists());
        let err =
            documents::read(&api, &ContextSelector::default(), "spec/product.md").unwrap_err();
        assert_eq!(err.code, "recovery_conflict");
        assert!(!context.spec_root.join("product.md").exists());
        let locked = Locked::acquire(&api, &context).unwrap();
        assert!(locked.journal.exists());
    }
    #[test]
    fn repo_first_recovery_refuses_a_new_head_on_the_same_branch() {
        let (_owner, api, _) = fixture_mode(StorageMode::RepoFirst);
        let git = |args: &[&str]| {
            let cx = crate::api::Execution::new(&api.runtime);
            let out = cx
                .command("git")
                .args([
                    "-c",
                    "user.name=test",
                    "-c",
                    "user.email=test@example.invalid",
                    "-c",
                    "commit.gpgsign=false",
                ])
                .args(args)
                .current_dir(&api.runtime.cwd)
                .output()
                .unwrap();
            assert!(
                out.status.success(),
                "{}",
                String::from_utf8_lossy(&out.stderr)
            );
        };
        git(&["add", "docs/spec"]);
        git(&["commit", "-m", "initial specs"]);
        let context = api.resolve_context(&ContextSelector::default()).unwrap();
        execute_failure(
            &api,
            &context,
            vec![change("spec/product.md", None)],
            Checkpoint::AfterWrites,
        );
        git(&["add", "-u", "docs/spec/product.md"]);
        git(&["commit", "-m", "later code head"]);
        let err =
            documents::read(&api, &ContextSelector::default(), "spec/product.md").unwrap_err();
        assert_eq!(err.code, "recovery_conflict");
        assert!(!context.spec_root.join("product.md").exists());
        assert!(Locked::acquire(&api, &context).unwrap().journal.exists());
    }
    #[test]
    fn lock_wait_recovery_checks_fresh_checkout_before_restoring() {
        for mutation in [false, true] {
            let (_owner, api, _) = fixture_mode(StorageMode::RepoFirst);
            let git = |args: &[&str]| {
                let cx = crate::api::Execution::new(&api.runtime);
                let out = cx
                    .command("git")
                    .args([
                        "-c",
                        "user.name=test",
                        "-c",
                        "user.email=test@example.invalid",
                        "-c",
                        "commit.gpgsign=false",
                    ])
                    .args(args)
                    .current_dir(&api.runtime.cwd)
                    .output()
                    .unwrap();
                assert!(
                    out.status.success(),
                    "{}",
                    String::from_utf8_lossy(&out.stderr)
                );
            };
            git(&["add", "docs/spec"]);
            git(&["commit", "-m", "branch a specs"]);
            let context = api.resolve_context(&ContextSelector::default()).unwrap();
            execute_failure(
                &api,
                &context,
                vec![change("spec/product.md", None)],
                Checkpoint::AfterWrites,
            );
            let held = Locked::acquire(&api, &context).unwrap();
            let (entered, waiting) = mpsc::channel();
            WAITING
                .lock()
                .unwrap()
                .insert(held.lock_path.clone(), entered);
            let worker_api = api.clone();
            let expected = context.revision_token.clone();
            let worker = std::thread::spawn(move || {
                if mutation {
                    apply(
                        &worker_api,
                        &ContextSelector::default(),
                        &expected,
                        vec![change("spec/new.md", Some(b"must not write"))],
                    )
                } else {
                    documents::read(&worker_api, &ContextSelector::default(), "spec/product.md")
                }
            });
            waiting
                .recv_timeout(std::time::Duration::from_secs(10))
                .unwrap();
            git(&["checkout", "-b", "branch-b"]);
            git(&["add", "-u", "docs/spec/product.md"]);
            git(&["commit", "-m", "branch b removes product"]);
            let journal = held.journal.clone();
            drop(held);
            let err = worker.join().unwrap().unwrap_err();
            assert_eq!(err.code, "recovery_conflict");
            assert!(!context.spec_root.join("product.md").exists());
            assert!(!context.spec_root.join("new.md").exists());
            assert!(journal.exists());
        }
    }
    #[test]
    fn public_context_never_combines_old_oid_with_new_document_token() {
        let (_owner, api, context) = fixture();
        let (entered, paused) = mpsc::channel();
        let (resume, proceed) = mpsc::channel();
        AFTER_OID
            .lock()
            .unwrap()
            .insert(context.spec_root.clone(), (entered, proceed));
        let reader_api = api.clone();
        let reader =
            std::thread::spawn(move || reader_api.resolve_context(&ContextSelector::default()));
        paused
            .recv_timeout(std::time::Duration::from_secs(10))
            .unwrap();
        let written = apply(
            &api,
            &ContextSelector::default(),
            &context.revision_token,
            vec![change("spec/coherent.md", Some(b"new bytes"))],
        )
        .unwrap()
        .context
        .unwrap();
        resume.send(()).unwrap();
        let read = reader.join().unwrap().unwrap();
        assert_eq!(read.spec_revision, written.spec_revision);
        assert_eq!(read.revision_token, written.revision_token);
    }
    #[test]
    fn legacy_after_write_authority_change_retains_journal_until_authority_is_restored() {
        let (_owner, api, context) = fixture_mode(StorageMode::RepoFirst);
        fs::create_dir_all(api.runtime.cwd.join(".vibe")).unwrap();
        let config = api.runtime.cwd.join(".vibe/spec.json");
        fs::write(&config, b"{\"root\":\"docs/spec\"}").unwrap();
        let replacement = api.runtime.cwd.join("replacement");
        fs::create_dir(&replacement).unwrap();
        fs::write(replacement.join("tech.md"), b"replacement bytes").unwrap();
        let found = crate::root::discover(&api.runtime.cwd, None, None).unwrap();
        let roots = crate::context::LegacyRoots::from_root(&found).unwrap();
        let original = fs::read(context.spec_root.join("tech.md")).unwrap();
        let error = apply_inner(
            &api,
            &ContextSelector::default(),
            None,
            Some(&roots),
            false,
            &[],
            |_| {
                Ok(vec![Change {
                    path: "spec/tech.md".into(),
                    content: Some(b"transaction-owned bytes".to_vec()),
                }])
            },
            |point| {
                if point == Checkpoint::AfterWrites {
                    fs::write(&config, b"{\"root\":\"replacement\"}").unwrap();
                }
                Ok(())
            },
        )
        .unwrap_err();
        assert_eq!(error.code, "context_changed");
        assert_eq!(
            fs::read(context.spec_root.join("tech.md")).unwrap(),
            b"transaction-owned bytes"
        );
        assert_eq!(
            fs::read(replacement.join("tech.md")).unwrap(),
            b"replacement bytes"
        );
        assert!(
            api.runtime
                .cwd
                .join(".vibe/run/spec-transaction.json")
                .is_file()
        );
        fs::write(&config, b"{\"root\":\"docs/spec\"}").unwrap();
        read_legacy(&api, &ContextSelector::default(), &roots, |_| Ok(())).unwrap();
        assert_eq!(
            fs::read(context.spec_root.join("tech.md")).unwrap(),
            original
        );
        assert!(
            !api.runtime
                .cwd
                .join(".vibe/run/spec-transaction.json")
                .exists()
        );
    }
    #[test]
    fn legacy_promotion_rechecks_config_and_alias_authority_after_waiting() {
        for alias in [false, true] {
            let (_owner, api, context) = fixture_mode(StorageMode::RepoFirst);
            let replacement = api.runtime.cwd.join("replacement");
            fs::create_dir_all(context.spec_root.join("features/auth")).unwrap();
            fs::write(
                context.spec_root.join("features/auth/tech.md"),
                b"<!-- merge -->\nold feature fact\n<!-- /merge -->\n",
            )
            .unwrap();
            fs::write(context.spec_root.join("tech.md"), b"old durable").unwrap();
            fs::create_dir_all(replacement.join("features/auth")).unwrap();
            fs::write(
                replacement.join("features/auth/tech.md"),
                b"<!-- merge -->\nnew feature fact\n<!-- /merge -->\n",
            )
            .unwrap();
            fs::write(replacement.join("tech.md"), b"new durable").unwrap();
            fs::create_dir_all(api.runtime.cwd.join(".vibe")).unwrap();
            fs::write(
                api.runtime.cwd.join(".vibe/spec.json"),
                b"{\"root\":\"docs/spec\"}",
            )
            .unwrap();
            #[cfg(unix)]
            let alias_path = api.runtime.cwd.join("selected-root");
            #[cfg(unix)]
            if alias {
                std::os::unix::fs::symlink(&context.spec_root, &alias_path).unwrap();
            }
            #[cfg(not(unix))]
            if alias {
                continue;
            }
            let before =
                crate::context::resolve_for_transaction(&api.runtime, &ContextSelector::default())
                    .unwrap();
            let held = Locked::acquire(&api, &before).unwrap();
            let (send, noticed) = mpsc::channel();
            WAITING
                .lock()
                .unwrap()
                .insert(api.runtime.cwd.join(".vibe/run/spec-documents.lock"), send);
            let runtime = api.runtime.clone();
            let worker = std::thread::spawn(move || {
                let mut argv = vec!["vibe-spec".into()];
                if alias {
                    argv.extend(["--root".into(), "selected-root".into()]);
                }
                argv.extend(["promote".into(), "auth".into(), "--json".into()]);
                crate::run_cli(&argv, &runtime, &mut std::io::empty())
            });
            noticed
                .recv_timeout(std::time::Duration::from_secs(10))
                .unwrap();
            #[cfg(unix)]
            if alias {
                fs::remove_file(&alias_path).unwrap();
                std::os::unix::fs::symlink(&replacement, &alias_path).unwrap();
            } else {
                fs::write(
                    api.runtime.cwd.join(".vibe/spec.json"),
                    b"{\"root\":\"replacement\"}",
                )
                .unwrap();
            }
            #[cfg(not(unix))]
            fs::write(
                api.runtime.cwd.join(".vibe/spec.json"),
                b"{\"root\":\"replacement\"}",
            )
            .unwrap();
            drop(held);
            let output = worker.join().unwrap();
            assert_eq!(
                output.exit,
                1,
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            assert!(String::from_utf8_lossy(&output.stderr).contains("context_changed"));
            assert_eq!(
                fs::read(context.spec_root.join("tech.md")).unwrap(),
                b"old durable"
            );
            assert_eq!(
                fs::read(replacement.join("tech.md")).unwrap(),
                b"new durable"
            );
        }
    }
    #[test]
    fn owned_runtime_directories_cleanup_preserves_foreign_and_preexisting_paths() {
        let (_owner, api, context) = fixture_mode(StorageMode::RepoFirst);
        let runtime = api.runtime.cwd.join(".vibe/run");
        assert!(!runtime.exists());
        let lock = Locked::acquire(&api, &context).unwrap();
        assert!(runtime.is_dir());
        drop(lock);
        assert!(!runtime.exists());
        fs::create_dir_all(&runtime).unwrap();
        let lock = Locked::acquire(&api, &context).unwrap();
        drop(lock);
        assert!(runtime.is_dir());
        fs::remove_dir(&runtime).unwrap();
        let lock = Locked::acquire(&api, &context).unwrap();
        fs::write(runtime.join("foreign"), b"preserve").unwrap();
        drop(lock);
        assert_eq!(fs::read(runtime.join("foreign")).unwrap(), b"preserve");
        #[cfg(unix)]
        {
            fs::remove_file(runtime.join("foreign")).unwrap();
            fs::remove_dir(&runtime).unwrap();
            let lock = Locked::acquire(&api, &context).unwrap();
            let moved = api.runtime.cwd.join(".vibe/moved-run");
            fs::rename(&runtime, &moved).unwrap();
            fs::create_dir(&runtime).unwrap();
            drop(lock);
            assert!(runtime.is_dir());
            assert!(moved.join("spec-documents.lock").is_file());
        }
    }
    #[test]
    fn legacy_removal_chain_never_recaptures_a_competing_revision() {
        let (_owner, api, context) = fixture_mode(StorageMode::RepoFirst);
        let registry = registry::load(&api.runtime).unwrap();
        fs::remove_file(registry.root.join("spec/registry.json")).unwrap();
        let roots = crate::context::LegacyRoots {
            code_anchor: api.runtime.cwd.clone(),
            spec_root: context.spec_root.clone(),
            artifact_root: context.artifact_root.clone(),
            initialization: false,
            explicit_target: None,
        };
        fs::create_dir_all(roots.artifact_root.join("specs")).unwrap();
        for name in ["first.md", "second.md"] {
            fs::write(
                roots.artifact_root.join("specs").join(name),
                b"retained bytes",
            )
            .unwrap();
        }
        let expected =
            crate::context::resolve_for_legacy(&api.runtime, &ContextSelector::default(), &roots)
                .unwrap()
                .revision_token;
        let first =
            apply_legacy_expected(&api, &ContextSelector::default(), &roots, &expected, |_| {
                Ok(vec![Change {
                    path: "superpowers/specs/first.md".into(),
                    content: None,
                }])
            })
            .unwrap()
            .context
            .unwrap();
        api.execute(SpecRequest::WriteDocument(crate::WriteDocumentRequest {
            selector: ContextSelector::default(),
            path: "spec/tech.md".into(),
            content: "competing writer".into(),
            expected_revision: first.revision_token.clone(),
        }))
        .unwrap();
        let error = apply_legacy_expected(
            &api,
            &ContextSelector::default(),
            &roots,
            &first.revision_token,
            |_| {
                Ok(vec![Change {
                    path: "superpowers/specs/second.md".into(),
                    content: None,
                }])
            },
        )
        .unwrap_err();
        assert_eq!(error.code, "revision_changed");
        assert!(!roots.artifact_root.join("specs/first.md").exists());
        assert_eq!(
            fs::read(roots.artifact_root.join("specs/second.md")).unwrap(),
            b"retained bytes"
        );
        assert_eq!(
            fs::read(roots.spec_root.join("tech.md")).unwrap(),
            b"competing writer"
        );
    }
    #[test]
    fn unregistered_legacy_transactions_recover_through_the_existing_journal() {
        let (_owner, api, context) = fixture_mode(StorageMode::RepoFirst);
        let registry = registry::load(&api.runtime).unwrap();
        fs::remove_file(registry.root.join("spec/registry.json")).unwrap();
        let roots = crate::context::LegacyRoots {
            code_anchor: api.runtime.cwd.clone(),
            spec_root: context.spec_root.clone(),
            artifact_root: context.artifact_root.clone(),
            initialization: false,
            explicit_target: None,
        };
        let before = fs::read(context.spec_root.join("tech.md")).unwrap();
        let error = apply_inner(
            &api,
            &ContextSelector::default(),
            None,
            Some(&roots),
            false,
            &[],
            |_| {
                Ok(vec![Change {
                    path: "spec/tech.md".into(),
                    content: Some(b"interrupted legacy write".to_vec()),
                }])
            },
            |point| {
                if point == Checkpoint::AfterWrites {
                    Err(SpecError::new("cancelled", "test interruption"))
                } else {
                    Ok(())
                }
            },
        )
        .unwrap_err();
        assert_eq!(error.code, "cancelled");
        assert_eq!(
            fs::read(context.spec_root.join("tech.md")).unwrap(),
            b"interrupted legacy write"
        );
        let document = api
            .read_document(&ContextSelector::default(), "spec/tech.md")
            .unwrap();
        assert_eq!(document.document.content.as_bytes(), before);
        assert!(document.context.project_id.is_none());
        assert!(
            !api.runtime
                .cwd
                .join(".vibe/run/spec-transaction.json")
                .exists()
        );
    }
    #[test]
    fn spec_ci_portability_recovery_binds_physical_roots_without_changing_display() {
        // spec-projects-mcp/4: equivalent spellings recover; another root never does.
        let (_owner, mut api, context) = fixture_mode(StorageMode::RepoFirst);
        let registry = registry::load(&api.runtime).unwrap();
        fs::remove_file(registry.root.join("spec/registry.json")).unwrap();
        api.runtime.cwd.push(".");
        let roots = crate::context::LegacyRoots {
            code_anchor: api.runtime.cwd.clone(),
            spec_root: context.spec_root.clone(),
            artifact_root: context.artifact_root.clone(),
            initialization: false,
            explicit_target: None,
        };
        let before = fs::read(context.spec_root.join("tech.md")).unwrap();
        let error = apply_inner(
            &api,
            &ContextSelector::default(),
            None,
            Some(&roots),
            false,
            &[],
            |_| {
                Ok(vec![Change {
                    path: "spec/tech.md".into(),
                    content: Some(b"interrupted".to_vec()),
                }])
            },
            |point| {
                if point == Checkpoint::AfterWrites {
                    Err(SpecError::new("cancelled", "test interruption"))
                } else {
                    Ok(())
                }
            },
        )
        .unwrap_err();
        assert_eq!(error.code, "cancelled");
        let path = api.runtime.cwd.join(".vibe/run/spec-transaction.json");
        let bytes = fs::read(&path).unwrap();
        let mut journal: Value = serde_json::from_slice(&bytes).unwrap();
        #[cfg(unix)]
        {
            let alias = api.runtime.cwd.join("saved-alias");
            std::os::unix::fs::symlink(&context.spec_root, &alias).unwrap();
            assert!(
                !journal_root_matches(&registry::path_value(&alias), &context.spec_root).unwrap()
            );
        }
        assert_eq!(
            journal["spec_root"],
            registry::path_value(&fs::canonicalize(&context.spec_root).unwrap())
        );
        journal["spec_root"] = registry::path_value(&api.runtime.cwd.join("other-root"));
        fs::write(&path, serde_json::to_vec(&journal).unwrap()).unwrap();
        assert_eq!(
            api.read_document(&ContextSelector::default(), "spec/tech.md")
                .unwrap_err()
                .code,
            "recovery_conflict"
        );
        assert_eq!(
            fs::read(context.spec_root.join("tech.md")).unwrap(),
            b"interrupted"
        );
        fs::write(&path, &bytes).unwrap();
        let read = api
            .read_document(&ContextSelector::default(), "spec/tech.md")
            .unwrap();
        assert_eq!(read.document.content.as_bytes(), before);
        assert_eq!(
            read.context.spec_root.as_os_str(),
            api.runtime.cwd.join("docs/spec").as_os_str()
        );
    }
    #[cfg(windows)]
    #[test]
    fn spec_ci_portability_old_journal_paths_keep_representation_compatibility() {
        // spec-projects-mcp/4: old ordinary paths equal their verbatim spelling only.
        let owner = vibe_core::testscratch::scratch("spec-recovery", "old-path");
        let physical = fs::canonicalize(&owner).unwrap();
        let ordinary = crate::git::argument_path(&physical);
        assert!(journal_root_matches(&registry::path_value(&ordinary), &physical).unwrap());
        assert!(
            !journal_root_matches(&registry::path_value(&ordinary.join("other")), &physical)
                .unwrap()
        );
    }
    #[test]
    fn spec_projects_mcp_5_plan_read_waits_for_one_coherent_published_snapshot() {
        let (_owner, api, initial) = fixture();
        let selector = ContextSelector::default();
        let initial = apply(
            &api,
            &selector,
            &initial.revision_token,
            vec![Change {
                path: "superpowers/plans/2026-01-15-coherent.md".into(),
                content: Some(b"### Task 1: Old\n- [ ] Old\n".to_vec()),
            }],
        )
        .unwrap()
        .context
        .unwrap();
        let (entered, waiting) = mpsc::channel();
        let (release, proceed) = mpsc::channel();
        let writer_api = api.clone();
        let expected = initial.revision_token.clone();
        let writer = std::thread::spawn(move || {
            apply_planned(
                &writer_api,
                &ContextSelector::default(),
                &expected,
                false,
                |_| {
                    Ok(vec![Change {
                        path: "superpowers/plans/2026-01-15-coherent.md".into(),
                        content: Some(b"### Task 2: Published\n- [x] Published\n".to_vec()),
                    }])
                },
                |checkpoint| {
                    if checkpoint == Checkpoint::AfterWrites {
                        entered.send(()).unwrap();
                        proceed
                            .recv_timeout(std::time::Duration::from_secs(10))
                            .unwrap();
                    }
                    Ok(())
                },
            )
        });
        waiting
            .recv_timeout(std::time::Duration::from_secs(10))
            .unwrap();
        let registry = registry::load(&api.runtime).unwrap();
        let lock_path = registry::project_dir(&registry, initial.project_id.as_ref().unwrap())
            .join("project.lock");
        let (blocked, noticed) = mpsc::channel();
        WAITING.lock().unwrap().insert(lock_path, blocked);
        let reader = std::thread::spawn(move || {
            api.execute(SpecRequest::Plan(crate::PlanRequest {
                selector: ContextSelector::default(),
                feature: "coherent".into(),
            }))
        });
        noticed
            .recv_timeout(std::time::Duration::from_secs(10))
            .unwrap();
        release.send(()).unwrap();
        let written = writer.join().unwrap().unwrap();
        let read = reader.join().unwrap().unwrap();
        assert_eq!(
            read.context.as_ref().unwrap().spec_revision,
            written.context.as_ref().unwrap().spec_revision
        );
        assert_eq!(
            read.context.as_ref().unwrap().revision_token,
            written.context.as_ref().unwrap().revision_token
        );
        assert_eq!(read.data["rows"][0]["id"], "Task 2");
        assert_eq!(read.data["rows"][0]["status"], "done");
    }
    #[test]
    fn spec_projects_mcp_6_lifecycle_frontier_inherits_source_and_prunes_completed_ancestors() {
        let (_owner, api, context) = fixture();
        let locked = Locked::acquire(&api, &context).unwrap();
        let project = context.project_id.as_ref().unwrap();
        let base = context.spec_revision.as_deref().unwrap();
        let tree = locked
            .text(&api, &["rev-parse", &format!("{base}^{{tree}}")], None)
            .unwrap();
        let event = |parent: &str, introduction: bool| {
            let value = json!({"v":1,"project":project.as_str(),"branch":locked.spec_ref,"feature_introductions":if introduction {vec![json!({"feature":"sample","baseline":parent})]}else{vec![]},"feature_completions":if introduction {Vec::<String>::new()}else{vec!["sample".into()]}});
            locked
                .text(
                    &api,
                    &[
                        "commit-tree",
                        &tree,
                        "-p",
                        parent,
                        "-m",
                        &format!("event\n\nVibe-Spec-Metadata: {value}"),
                    ],
                    None,
                )
                .unwrap()
        };
        let intro = event(base, true);
        let completion = event(&intro, false);
        assert_eq!(
            feature_evidence(
                &api,
                locked.history.as_deref().unwrap(),
                project,
                &[base, &intro],
                "sample"
            )
            .unwrap(),
            FeatureEvidence::Active(base.into())
        );
        assert_eq!(
            feature_evidence(
                &api,
                locked.history.as_deref().unwrap(),
                project,
                &[&intro, &completion],
                "sample"
            )
            .unwrap(),
            FeatureEvidence::Completed
        );
        assert_eq!(
            feature_evidence(
                &api,
                locked.history.as_deref().unwrap(),
                project,
                &[&completion, &intro],
                "sample"
            )
            .unwrap(),
            FeatureEvidence::Completed
        );
        let reuse = event(&completion, true);
        assert_eq!(
            feature_evidence(
                &api,
                locked.history.as_deref().unwrap(),
                project,
                &[&intro, &reuse],
                "sample"
            )
            .unwrap(),
            FeatureEvidence::Active(completion.clone())
        );
        let other = locked
            .text(
                &api,
                &["commit-tree", &tree, "-p", base, "-m", "unrelated"],
                None,
            )
            .unwrap();
        let concurrent = event(&other, true);
        assert_eq!(
            feature_evidence(
                &api,
                locked.history.as_deref().unwrap(),
                project,
                &[&intro, &concurrent],
                "sample"
            )
            .unwrap(),
            FeatureEvidence::Ambiguous
        );
        assert_eq!(
            feature_evidence(
                &api,
                locked.history.as_deref().unwrap(),
                project,
                &[&completion, &concurrent],
                "sample"
            )
            .unwrap(),
            FeatureEvidence::Ambiguous
        );
    }
    #[test]
    fn spec_projects_mcp_6_preview_refuses_journal_without_recovery() {
        let (_owner, api, context) = fixture();
        execute_failure(
            &api,
            &context,
            vec![change("spec/product.md", Some(b"pending bytes"))],
            Checkpoint::AfterWrites,
        );
        let locked = Locked::acquire(&api, &context).unwrap();
        let journal = fs::read(&locked.journal).unwrap();
        let head = locked.revision(&api).unwrap();
        drop(locked);
        let promotion = api.execute(SpecRequest::Promote(crate::PromoteRequest {
            selector: ContextSelector::default(),
            feature: "sample".into(),
            expected_revision: context.revision_token.clone(),
            dry_run: true,
            done: false,
            target: None,
            base: None,
        }));
        assert_eq!(promotion.unwrap_err().code, "recovery_required");
        let preview = api.plan_reconciliation(crate::ReconcileRequest {
            selector: ContextSelector::default(),
            source: "unused".into(),
            target: "main".into(),
        });
        assert_eq!(preview.unwrap_err().code, "recovery_required");
        let result = read_preview(&api, &ContextSelector::default(), |_| Ok(()));
        assert_eq!(result.unwrap_err().code, "recovery_required");
        assert_eq!(
            fs::read(context.spec_root.join("product.md")).unwrap(),
            b"pending bytes"
        );
        let locked = Locked::acquire(&api, &context).unwrap();
        assert_eq!(fs::read(&locked.journal).unwrap(), journal);
        assert_eq!(locked.revision(&api).unwrap(), head);
    }
    #[test]
    fn spec_projects_mcp_6_completion_retains_missing_durable_subtree_refusal() {
        let (_owner, api, context) = fixture();
        let original = documents::collect(&context).unwrap();
        let empty = apply(
            &api,
            &ContextSelector::default(),
            &context.revision_token,
            original
                .keys()
                .map(|path| Change {
                    path: path.clone(),
                    content: None,
                })
                .collect(),
        )
        .unwrap()
        .context
        .unwrap();
        let introduced = apply(
            &api,
            &ContextSelector::default(),
            &empty.revision_token,
            vec![change(
                "superpowers/specs/2026-01-15-boundary-design.md",
                Some(b"# Boundary\n"),
            )],
        )
        .unwrap()
        .context
        .unwrap();
        let restored = apply(
            &api,
            &ContextSelector::default(),
            &introduced.revision_token,
            original
                .into_iter()
                .map(|(path, content)| Change {
                    path,
                    content: Some(content),
                })
                .collect(),
        )
        .unwrap()
        .context
        .unwrap();
        let _lock = Locked::acquire(&api, &restored).unwrap();
        assert_eq!(
            feature_baseline(&api, &restored, "boundary")
                .unwrap_err()
                .code,
            "missing_evidence"
        );
    }
    #[test]
    fn spec_projects_mcp_6_repo_cli_promotion_preview_never_recovers() {
        let (_owner, api, context) = fixture_mode(StorageMode::RepoFirst);
        execute_failure(
            &api,
            &context,
            vec![change("spec/product.md", Some(b"pending bytes"))],
            Checkpoint::AfterWrites,
        );
        let held = Locked::acquire(&api, &context).unwrap();
        let before = fs::read(&held.journal).unwrap();
        drop(held);
        let argv = ["vibe-spec", "promote", "sample", "--dry-run"].map(std::ffi::OsString::from);
        let out = crate::run_cli(&argv, &api.runtime, &mut std::io::empty());
        assert_eq!(out.exit, 1);
        assert!(
            String::from_utf8_lossy(&out.stderr).contains("recovery_required"),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        let held = Locked::acquire(&api, &context).unwrap();
        assert_eq!(fs::read(&held.journal).unwrap(), before);
        assert_eq!(
            fs::read(context.spec_root.join("product.md")).unwrap(),
            b"pending bytes"
        );
    }

    #[test]
    fn spec_projects_mcp_6_review_lifecycle_requires_exact_private_origin_ref() {
        for branch in ["main", "refs/heads/main", "refs/heads/spec/valid/escape"] {
            let value = json!({"v":1,"project":"p-example","branch":branch,"feature_introductions":[],"feature_completions":[]});
            assert_eq!(
                validate_lifecycle_metadata(&value).unwrap_err().code,
                "invalid_history"
            );
        }
    }
}

#[cfg(test)]
mod invocation_controls {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;
    use std::sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    };

    #[test]
    fn spec_projects_mcp_8_recovery_can_stop_between_owned_paths_and_resume() {
        let (_owner, api, _) = super::tests::fixture();
        for name in ["a", "b"] {
            let context = api.resolve_context(&ContextSelector::default()).unwrap();
            apply(
                &api,
                &ContextSelector::default(),
                &context.revision_token,
                vec![Change {
                    path: format!("spec/{name}.md"),
                    content: Some(b"# Original\n".to_vec()),
                }],
            )
            .unwrap();
        }
        let context = api.resolve_context(&ContextSelector::default()).unwrap();
        apply_planned(
            &api,
            &ContextSelector::default(),
            &context.revision_token,
            false,
            |_| {
                Ok(["a", "b"]
                    .iter()
                    .map(|name| Change {
                        path: format!("spec/{name}.md"),
                        content: Some(b"# Planned\n".to_vec()),
                    })
                    .collect())
            },
            |point| {
                if point == Checkpoint::BeforePublish {
                    Err(SpecError::new("interrupted", "fixture"))
                } else {
                    Ok(())
                }
            },
        )
        .unwrap_err();
        let a = context.spec_root.join("a.md");
        let watched = a.clone();
        let controlled = api.with_control(crate::InvocationControl::new(move || {
            fs::read(&watched).is_ok_and(|b| b == b"# Original\n")
        }));
        assert_eq!(
            controlled
                .read_document(&ContextSelector::default(), "spec/b.md")
                .unwrap_err()
                .code,
            "cancelled"
        );
        assert_eq!(fs::read(a).unwrap(), b"# Original\n");
        assert_eq!(
            fs::read(context.spec_root.join("b.md")).unwrap(),
            b"# Planned\n"
        );
        assert_eq!(
            api.read_document(&ContextSelector::default(), "spec/b.md")
                .unwrap()
                .document
                .content,
            "# Original\n"
        );
        assert_eq!(
            api.resolve_context(&ContextSelector::default())
                .unwrap()
                .spec_revision,
            context.spec_revision
        );
    }
    #[test]
    fn spec_projects_mcp_8_promotion_admits_its_complete_result_before_publication() {
        let (_owner, api, context) = super::tests::fixture();
        api.execute(crate::SpecRequest::WriteDocument(
            crate::WriteDocumentRequest {
                selector: ContextSelector::default(),
                path: "superpowers/specs/2026-11-12-size-check-design.md".into(),
                content: "# Feature design\n".into(),
                expected_revision: context.revision_token,
            },
        ))
        .unwrap();
        let before = api.resolve_context(&ContextSelector::default()).unwrap();
        let mut control = crate::InvocationControl::new(|| false);
        control.limited = true;
        control.result_admission = Some(Arc::new(|value| {
            if value["data"].get("feature").is_some() {
                Err(SpecError::new("size_limit", "test full promotion result"))
            } else {
                Ok(())
            }
        }));
        let result = api
            .with_control(control)
            .execute(crate::SpecRequest::Promote(crate::PromoteRequest {
                selector: ContextSelector::default(),
                feature: "size-check".into(),
                expected_revision: before.revision_token.clone(),
                dry_run: false,
                done: false,
                target: None,
                base: None,
            }));
        assert_eq!(result.unwrap_err().code, "size_limit");
        // Recovery observes the unchanged private ref and preserves caller drafts.
        api.list_documents(&ContextSelector::default()).unwrap();
        assert_eq!(
            api.resolve_context(&ContextSelector::default())
                .unwrap()
                .spec_revision,
            before.spec_revision
        );
    }
    #[test]
    fn spec_projects_mcp_8_cooperative_stop_preserves_publication_and_bookkeeping() {
        for stop in [
            Checkpoint::BeforeWrites,
            Checkpoint::AfterWrites,
            Checkpoint::BeforePublish,
            Checkpoint::AfterPublish,
        ] {
            let (_owner, api, context) = super::tests::fixture();
            let cancelled = Arc::new(AtomicBool::new(false));
            let token = cancelled.clone();
            let controlled = api.with_control(crate::InvocationControl::new(move || {
                token.load(Ordering::SeqCst)
            }));
            let result = apply_planned(
                &controlled,
                &ContextSelector::default(),
                &context.revision_token,
                false,
                |_| {
                    Ok(vec![Change {
                        path: "spec/control.md".into(),
                        content: Some(b"# Controlled\n".to_vec()),
                    }])
                },
                |point| {
                    if point == stop {
                        cancelled.store(true, Ordering::SeqCst);
                    }
                    Ok(())
                },
            );
            let recorded = stop == Checkpoint::AfterPublish;
            if recorded {
                assert!(result.is_ok(), "{result:?}");
            } else {
                assert_eq!(result.unwrap_err().code, "cancelled");
            }
            let fresh = api.resolve_context(&ContextSelector::default());
            if !recorded {
                assert_eq!(fresh.unwrap_err().code, "recovery_required");
            }
            let read = api.read_document(&ContextSelector::default(), "spec/control.md");
            if recorded {
                assert_eq!(read.unwrap().document.content, "# Controlled\n");
            } else {
                assert_eq!(read.unwrap_err().code, "document_not_found");
            }
            let current = api.resolve_context(&ContextSelector::default()).unwrap();
            assert_eq!(current.spec_revision == context.spec_revision, !recorded);
            let lock = super::tests::record_lock(&api, &current);
            assert!(!lock.journal.exists());
        }
    }
}
