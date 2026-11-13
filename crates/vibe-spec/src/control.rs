//! Per-invocation cooperative control; compatibility callers use the no-op default.
use crate::SpecError;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
type ResultAdmission = Arc<dyn Fn(&serde_json::Value) -> Result<(), SpecError> + Send + Sync>;
#[derive(Clone)]
pub struct InvocationControl {
    cancelled: Arc<dyn Fn() -> bool + Send + Sync>,
    completion: Arc<AtomicUsize>,
    pub(crate) limited: bool,
    pub(crate) result_admission: Option<ResultAdmission>,
}
impl std::fmt::Debug for InvocationControl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InvocationControl").finish_non_exhaustive()
    }
}
impl InvocationControl {
    pub fn new(cancelled: impl Fn() -> bool + Send + Sync + 'static) -> Self {
        Self {
            cancelled: Arc::new(cancelled),
            completion: Arc::new(AtomicUsize::new(0)),
            limited: false,
            result_admission: None,
        }
    }
    pub(crate) fn check(&self) -> Result<(), SpecError> {
        if self.completion.load(Ordering::SeqCst) == 0 && (self.cancelled)() {
            Err(SpecError::new(
                "cancelled",
                "operation cancelled before publication",
            ))
        } else {
            Ok(())
        }
    }
    pub(crate) fn completing(&self) -> Completion {
        self.completion.fetch_add(1, Ordering::SeqCst);
        Completion(self.completion.clone())
    }
}
pub(crate) struct Completion(Arc<AtomicUsize>);
impl Drop for Completion {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::SeqCst);
    }
}
impl crate::SpecApi {
    pub fn with_control(&self, control: InvocationControl) -> Self {
        Self {
            runtime: self.runtime.clone(),
            control: Some(control),
        }
    }
    pub(crate) fn project_results(
        &self,
        project: impl Fn(&serde_json::Value) -> Option<serde_json::Value> + Send + Sync + 'static,
    ) -> Self {
        let mut api = self.clone();
        if let Some(control) = &mut api.control
            && let Some(admit) = control.result_admission.clone()
        {
            control.result_admission = Some(Arc::new(move |value| {
                let full = project(value);
                admit(full.as_ref().unwrap_or(value))
            }));
        }
        api
    }
    pub(crate) fn check_control(&self) -> Result<(), SpecError> {
        self.control
            .as_ref()
            .map_or(Ok(()), InvocationControl::check)
    }
    pub(crate) fn completing(&self) -> Option<Completion> {
        self.control.as_ref().map(InvocationControl::completing)
    }
    pub(crate) fn lock_storage(
        &self,
        root: &std::path::Path,
        path: &std::path::Path,
    ) -> Result<vibe_core::lock::Lock, SpecError> {
        self.check_control()?;
        let lock = crate::registry::lock_controlled(root, path, self.control.as_ref())?;
        self.check_control()?;
        Ok(lock)
    }
    pub(crate) fn admit_result(&self, value: &serde_json::Value) -> Result<(), SpecError> {
        self.control
            .as_ref()
            .and_then(|c| c.result_admission.as_ref())
            .map_or(Ok(()), |admit| admit(value))
    }
    pub(crate) fn limited(&self) -> bool {
        self.control.as_ref().is_some_and(|c| c.limited)
    }
    pub(crate) fn admit_files(&self, files: &crate::documents::Files) -> Result<(), SpecError> {
        self.check_control()?;
        if self.control.as_ref().is_some_and(|c| c.limited)
            && files.values().any(|b| b.len() > 8 * 1024 * 1024)
        {
            return Err(SpecError::new(
                "size_limit",
                "planned document exceeds 8 MiB",
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use crate::*;
    #[cfg(unix)]
    #[test]
    fn spec_projects_mcp_8_resumed_setup_rechecks_before_cas_and_completes_published_ref() {
        use std::os::unix::fs::PermissionsExt;
        use std::sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
        };
        for published in [false, true] {
            let (owner, api, context) = crate::transaction::tests::fixture();
            let show = api
                .execute(SpecRequest::ProjectShow {
                    selector: ContextSelector::default(),
                })
                .unwrap();
            let registry = crate::registry::load(&api.runtime).unwrap();
            let project = context.project_id.unwrap();
            let root = crate::registry::project_dir(&registry, &project);
            let branch = crate::storage::new_branch("refs/heads/topic", None, None, None).unwrap();
            let marker = root.join(format!("setup-{}.json", branch.id));
            let watched = marker.clone();
            let controlled = api.with_control(InvocationControl::new(move || {
                std::fs::read(&watched)
                    .ok()
                    .and_then(|b| serde_json::from_slice::<serde_json::Value>(&b).ok())
                    .is_some_and(|v| v["planned_revision"].is_string())
            }));
            let request = InitBranchRequest {
                selector: ContextSelector::default(),
                name: "topic".into(),
                from: "main".into(),
                expected_revision: show.data["revision"].as_str().unwrap().into(),
                expected_project_revision: show.data["project_revision"].as_str().unwrap().into(),
            };
            assert_eq!(
                controlled
                    .execute(SpecRequest::BranchInit(request.clone()))
                    .unwrap_err()
                    .code,
                "cancelled"
            );
            let saved = std::fs::read(&marker).unwrap();
            let stage: serde_json::Value = serde_json::from_slice(&saved).unwrap();
            let spec_ref = stage["branch"]["spec_ref"].as_str().unwrap();
            let base = stage["base_revision"].as_str().unwrap();
            let planned = stage["planned_revision"].as_str().unwrap();
            let history = root.join("history.git");
            assert_eq!(
                crate::storage::git_text(
                    &api.runtime,
                    &history,
                    None,
                    &["rev-parse", "--verify", spec_ref]
                )
                .unwrap(),
                base
            );
            if published {
                crate::storage::git_text(
                    &api.runtime,
                    &history,
                    None,
                    &["update-ref", spec_ref, planned, base],
                )
                .unwrap();
            }
            let bin = owner.join("held-bin");
            std::fs::create_dir(&bin).unwrap();
            let gate = owner.join("hold");
            std::fs::write(&gate, b"held").unwrap();
            let entered = owner.join("entered");
            let quote =
                |p: &std::path::Path| format!("'{}'", p.to_str().unwrap().replace('\'', "'\\''"));
            let wrapper = bin.join("git");
            std::fs::write(&wrapper,format!("#!/bin/sh\ncase \" $* \" in *\" rev-parse --verify {spec_ref} \"*) : > {}; while [ -f {} ]; do /bin/sleep 0.01; done;; esac\nexec /usr/bin/git \"$@\"\n",quote(&entered),quote(&gate))).unwrap();
            std::fs::set_permissions(&wrapper, std::fs::Permissions::from_mode(0o755)).unwrap();
            let mut runtime = api.runtime.clone();
            let mut paths = vec![bin];
            paths.extend(std::env::split_paths(
                runtime.env.get(std::ffi::OsStr::new("PATH")).unwrap(),
            ));
            runtime
                .env
                .insert("PATH".into(), std::env::join_paths(paths).unwrap());
            let cancelled = Arc::new(AtomicBool::new(false));
            let flag = cancelled.clone();
            let resumed = SpecApi::new(runtime)
                .unwrap()
                .with_control(InvocationControl::new(move || flag.load(Ordering::SeqCst)));
            let retry = request.clone();
            let worker =
                std::thread::spawn(move || resumed.execute(SpecRequest::BranchInit(retry)));
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
            while !entered.exists() {
                assert!(
                    std::time::Instant::now() < deadline,
                    "resumed exact ref lookup was not reached"
                );
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
            cancelled.store(true, Ordering::SeqCst);
            std::fs::remove_file(gate).unwrap();
            let result = worker.join().unwrap();
            if published {
                result.unwrap();
            } else {
                assert_eq!(result.unwrap_err().code, "cancelled");
                assert_eq!(
                    crate::storage::git_text(
                        &api.runtime,
                        &history,
                        None,
                        &["rev-parse", "--verify", spec_ref]
                    )
                    .unwrap(),
                    base
                );
                assert_eq!(
                    crate::registry::load_project(&registry, &project)
                        .unwrap()
                        .revision,
                    show.data["project_revision"].as_str().unwrap()
                );
                assert_eq!(std::fs::read(&marker).unwrap(), saved);
                api.execute(SpecRequest::BranchInit(request)).unwrap();
            }
            assert_eq!(
                crate::storage::git_text(
                    &api.runtime,
                    &history,
                    None,
                    &["rev-parse", "--verify", spec_ref]
                )
                .unwrap(),
                planned
            );
            assert_eq!(
                crate::registry::load_project(&registry, &project)
                    .unwrap()
                    .branches
                    .len(),
                2
            );
        }
    }
    #[test]
    fn spec_projects_mcp_8_branch_setup_cancellation_resumes_exact_inputs() {
        let (_owner, api, context) = crate::transaction::tests::fixture();
        let show = api
            .execute(SpecRequest::ProjectShow {
                selector: ContextSelector::default(),
            })
            .unwrap();
        let registry = crate::registry::load(&api.runtime).unwrap();
        let id = context.project_id.as_ref().unwrap();
        let root = crate::registry::project_dir(&registry, id);
        let branch = crate::storage::new_branch("refs/heads/topic", None, None, None).unwrap();
        let marker = root.join(format!("setup-{}.json", branch.id));
        let watched = marker.clone();
        let controlled = api.with_control(InvocationControl::new(move || watched.exists()));
        let request = InitBranchRequest {
            selector: ContextSelector::default(),
            name: "topic".into(),
            from: "main".into(),
            expected_revision: show.data["revision"].as_str().unwrap().into(),
            expected_project_revision: show.data["project_revision"].as_str().unwrap().into(),
        };
        assert_eq!(
            controlled
                .execute(SpecRequest::BranchInit(request.clone()))
                .unwrap_err()
                .code,
            "cancelled"
        );
        assert!(marker.exists());
        assert_eq!(
            api.execute(SpecRequest::ProjectShow {
                selector: ContextSelector::default()
            })
            .unwrap()
            .data["project_revision"],
            show.data["project_revision"]
        );
        api.execute(SpecRequest::BranchInit(request)).unwrap();
        assert!(
            api.resolve_context(&ContextSelector {
                branch: Some("topic".into()),
                ..Default::default()
            })
            .is_ok()
        );
    }
    #[test]
    fn spec_projects_mcp_8_attach_cancels_after_project_lock_before_registration() {
        let (owner, api, context) = crate::transaction::tests::fixture();
        let extra = owner.join("extra");
        std::fs::create_dir(&extra).unwrap();
        let initialized = std::process::Command::new("git")
            .args(["init", "-b", "other"])
            .current_dir(&extra)
            .env_clear()
            .env("PATH", std::env::var_os("PATH").unwrap_or_default())
            .output()
            .unwrap();
        assert!(initialized.status.success());
        let registry = crate::registry::load(&api.runtime).unwrap();
        let project = context.project_id.unwrap();
        let path = crate::registry::project_dir(&registry, &project).join("project.lock");
        let controlled = api.with_control(InvocationControl::new(move || path.exists()));
        let result = controlled.execute(SpecRequest::ProjectAttach(AttachProjectRequest {
            project: project.clone(),
            checkout: Some(extra),
            main: false,
            expected_revision: registry.revision.clone(),
        }));
        assert_eq!(result.unwrap_err().code, "cancelled");
        let current = crate::registry::load(&api.runtime).unwrap();
        assert_eq!(current.revision, registry.revision);
        assert_eq!(current.projects[0].checkouts.len(), 1);
    }
    #[test]
    fn spec_projects_mcp_8_initial_setup_cancellation_retains_exact_resumable_stage() {
        let (owner, api, _) = crate::transaction::tests::fixture();
        let code = owner.join("new-code");
        std::fs::create_dir(&code).unwrap();
        let initialized = std::process::Command::new("git")
            .args(["init", "-b", "main"])
            .current_dir(&code)
            .env_clear()
            .env("PATH", std::env::var_os("PATH").unwrap_or_default())
            .output()
            .unwrap();
        assert!(initialized.status.success());
        let registry = crate::registry::load(&api.runtime).unwrap();
        let physical = std::fs::canonicalize(&code).unwrap();
        let id = ProjectId::new(format!(
            "p-{}",
            vibe_core::markers::hash_bytes(physical.as_os_str().as_encoded_bytes())
        ))
        .unwrap();
        let marker = crate::registry::project_dir(&registry, &id).join("setup.json");
        let watched = marker.clone();
        let controlled = api.with_control(InvocationControl::new(move || watched.exists()));
        let request = InitProjectRequest {
            selector: ContextSelector {
                checkout: Some(code),
                ..Default::default()
            },
            mode: StorageMode::GlobalFirst,
            name: Some("Resumable".into()),
            expected_revision: registry.revision.clone(),
        };
        assert_eq!(
            controlled
                .execute(SpecRequest::ProjectInit(request.clone()))
                .unwrap_err()
                .code,
            "cancelled"
        );
        assert!(marker.exists());
        assert_eq!(
            crate::registry::load(&api.runtime).unwrap().revision,
            registry.revision
        );
        let recovered = api.execute(SpecRequest::ProjectInit(request)).unwrap();
        assert_eq!(recovered.context.unwrap().project_id, Some(id));
    }
    #[test]
    fn spec_projects_mcp_8_registration_completion_does_not_mask_new_refresh() {
        let (owner, api, _) = crate::transaction::tests::fixture();
        let code = owner.join("publication-code");
        std::fs::create_dir(&code).unwrap();
        let initialized = std::process::Command::new("git")
            .args(["init", "-b", "main"])
            .current_dir(&code)
            .env_clear()
            .env("PATH", std::env::var_os("PATH").unwrap_or_default())
            .output()
            .unwrap();
        assert!(initialized.status.success());
        let committed = std::process::Command::new("git")
            .args([
                "-c",
                "commit.gpgsign=false",
                "-c",
                "core.hooksPath=/dev/null",
                "-c",
                "user.name=test",
                "-c",
                "user.email=test@example.invalid",
                "commit",
                "--allow-empty",
                "-m",
                "fixture",
            ])
            .current_dir(&code)
            .env_clear()
            .env("PATH", std::env::var_os("PATH").unwrap_or_default())
            .output()
            .unwrap();
        assert!(committed.status.success());

        let registry = crate::registry::load(&api.runtime).unwrap();
        let physical = std::fs::canonicalize(&code).unwrap();
        let id = ProjectId::new(format!(
            "p-{}",
            vibe_core::markers::hash_bytes(physical.as_os_str().as_encoded_bytes())
        ))
        .unwrap();
        let observed = id.as_str().to_owned();
        let path = registry.root.join("spec/registry.json");
        let controlled = api.with_control(InvocationControl::new(move || {
            std::fs::read_to_string(&path).is_ok_and(|text| text.contains(&observed))
        }));
        let result = controlled.execute(SpecRequest::ProjectInit(InitProjectRequest {
            selector: ContextSelector {
                checkout: Some(code),
                ..Default::default()
            },
            mode: StorageMode::GlobalFirst,
            name: Some("Committed registration".into()),
            expected_revision: registry.revision,
        }));
        let current = crate::registry::load(&api.runtime).unwrap();
        assert!(current.projects.iter().any(|p| p.id == id));
        let state = crate::registry::load_project(&current, &id).unwrap();
        assert!(
            state.observations.is_empty(),
            "cancellation must stop the independent observation publication"
        );
        assert_eq!(state.branches.len(), 1);
        assert_eq!(result.unwrap_err().code, "cancelled");
        let shown = api
            .execute(SpecRequest::ProjectShow {
                selector: ContextSelector {
                    project: Some(id.clone()),
                    ..Default::default()
                },
            })
            .unwrap();
        assert_eq!(shown.data["project"]["id"], id.as_str());
        assert!(
            api.resolve_context(&ContextSelector {
                project: Some(id.clone()),
                ..Default::default()
            })
            .is_ok()
        );
        assert_eq!(
            crate::registry::load_project(&crate::registry::load(&api.runtime).unwrap(), &id)
                .unwrap()
                .observations
                .len(),
            1
        );
    }
}
