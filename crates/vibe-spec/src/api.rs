//! Explicit process inputs and per-call output.
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::PathBuf;

/// Immutable inputs for a spec invocation. Environment values retain their OS bytes.
#[derive(Clone, Debug)]
pub struct Runtime {
    pub cwd: PathBuf,
    pub env: BTreeMap<OsString, OsString>,
}
impl Runtime {
    /// Snapshot the process once, at a binary adapter boundary.
    pub fn from_process() -> std::io::Result<Self> {
        Ok(Self {
            cwd: std::env::current_dir()?,
            env: std::env::vars_os().collect(),
        })
    }
}
/// Complete compatibility command result; domain code never writes process stdio.
#[derive(Debug)]
pub struct CommandOutput {
    pub exit: u8,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

/// A stable domain error, independent of CLI exit codes or transport framing.
#[derive(Clone, Debug, PartialEq)]
pub struct SpecError {
    pub code: String,
    pub message: String,
    pub details: serde_json::Value,
}
impl SpecError {
    pub(crate) fn new(code: &str, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            details: serde_json::Value::Null,
        }
    }
}
impl std::fmt::Display for SpecError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}
impl std::error::Error for SpecError {}

/// Immutable configuration shared by independent domain API calls.
#[derive(Clone, Debug)]
pub struct SpecApi {
    pub(crate) runtime: Runtime,
    pub(crate) control: Option<crate::InvocationControl>,
}
impl SpecApi {
    pub fn new(runtime: Runtime) -> Result<Self, SpecError> {
        if !runtime.cwd.is_absolute() {
            return Err(SpecError::new(
                "invalid_context",
                "runtime cwd must be absolute",
            ));
        }
        Ok(Self {
            runtime,
            control: None,
        })
    }
    pub fn resolve_context(
        &self,
        selector: &crate::ContextSelector,
    ) -> Result<crate::SpecContext, SpecError> {
        crate::transaction::resolve_controlled(self, selector)
    }
}

/// A domain result with the exact selection that produced it.
#[derive(Clone, Debug)]
pub struct SpecResponse {
    pub context: Option<crate::SpecContext>,
    pub data: serde_json::Value,
}

pub(crate) use crate::execution::{Execution, executable};

/// Management writes consume the exact registry revision returned by Projects.
#[derive(Clone, Debug)]
pub struct InitProjectRequest {
    pub selector: crate::ContextSelector,
    pub mode: crate::StorageMode,
    pub name: Option<String>,
    pub expected_revision: String,
}
#[derive(Clone, Debug)]
pub struct AttachProjectRequest {
    pub project: crate::ProjectId,
    pub checkout: Option<PathBuf>,
    pub main: bool,
    pub expected_revision: String,
}
/// Branch writes bind both registry selection and per-project metadata revisions.
#[derive(Clone, Debug)]
pub struct InitBranchRequest {
    pub selector: crate::ContextSelector,
    pub name: String,
    pub from: String,
    pub expected_revision: String,
    pub expected_project_revision: String,
}
#[derive(Clone, Debug)]
pub enum SpecRequest {
    ReconcilePlan(crate::ReconcileRequest),
    ReconcileApply {
        plan: crate::ReconcilePlan,
        expected_revision: String,
    },
    ReconciliationStatus {
        project: crate::ProjectId,
        record_id: crate::RecordId,
    },
    ReconciliationRead {
        project: crate::ProjectId,
        record_id: crate::RecordId,
        path: String,
    },
    ReconciliationResolve(crate::ResolveRequest),
    Promote(crate::PromoteRequest),
    Init(crate::InitRequest),
    Export(crate::ExportRequest),
    Import(crate::ImportRequest),
    Trace(crate::TraceRequest),
    Drift(crate::DriftRequest),
    Feature(crate::FeatureRequest),
    Plan(crate::PlanRequest),
    Lessons(crate::LessonsRequest),
    ListSpecs(crate::ContextSelector),
    ScanMerges {
        selector: crate::ContextSelector,
        feature: Option<String>,
    },
    Validate(crate::ValidateRequest),
    Projects,
    ProjectShow {
        selector: crate::ContextSelector,
    },
    ProjectInit(InitProjectRequest),
    ProjectAttach(AttachProjectRequest),
    BranchInit(InitBranchRequest),
    Context(crate::ContextSelector),
    Documents(crate::ContextSelector),
    ReadDocument {
        selector: crate::ContextSelector,
        path: String,
    },
    WriteDocument(WriteDocumentRequest),
    RemoveDocument(RemoveDocumentRequest),
    Snapshot(SnapshotRequest),
}
#[derive(Clone, Debug)]
pub struct WriteDocumentRequest {
    pub selector: crate::ContextSelector,
    pub path: String,
    pub content: String,
    pub expected_revision: String,
}
#[derive(Clone, Debug)]
pub struct RemoveDocumentRequest {
    pub selector: crate::ContextSelector,
    pub path: String,
    pub expected_revision: String,
}
#[derive(Clone, Debug)]
pub struct SnapshotRequest {
    pub selector: crate::ContextSelector,
    pub expected_revision: String,
}
impl SpecApi {
    pub fn execute(&self, request: SpecRequest) -> Result<SpecResponse, SpecError> {
        self.check_control()?;
        match request {
            SpecRequest::ReconcilePlan(request) => Ok(SpecResponse {
                context: None,
                data: self.plan_reconciliation(request)?.value(),
            }),
            SpecRequest::ReconcileApply {
                plan,
                expected_revision,
            } => Ok(SpecResponse {
                context: None,
                data: self
                    .apply_reconciliation(&plan, &expected_revision)?
                    .value(),
            }),
            SpecRequest::ReconciliationStatus { project, record_id } => Ok(SpecResponse {
                context: None,
                data: self.reconciliation_status(&project, &record_id)?.value(),
            }),
            SpecRequest::ReconciliationRead {
                project,
                record_id,
                path,
            } => Ok(SpecResponse {
                context: None,
                data: self
                    .read_reconciliation_conflict(&project, &record_id, &path)?
                    .value(),
            }),
            SpecRequest::ReconciliationResolve(request) => Ok(SpecResponse {
                context: None,
                data: self.resolve_reconciliation(request)?.value(),
            }),
            SpecRequest::Promote(request) => crate::promote::managed(self, request),
            SpecRequest::Init(request) => crate::transaction::apply_planned(
                self,
                &request.selector,
                &request.expected_revision,
                false,
                crate::init::plan,
                |_| Ok(()),
            ),
            SpecRequest::Export(request) => self.read_domain(&request.selector, |context, _| {
                let (bundle, diagnostics) = crate::openspec::export_bundle(&self.runtime, context)?;
                let mut value = bundle.value();
                value["diagnostics"] = serde_json::json!(
                    diagnostics
                        .iter()
                        .map(|d| serde_json::json!({"code":d.code,"message":d.message}))
                        .collect::<Vec<_>>()
                );
                Ok(value)
            }),
            SpecRequest::Import(request) => crate::transaction::apply_reported_planned(
                self,
                &request.selector,
                &request.expected_revision,
                &[],
                |context| {
                    let (changes, diagnostics) = crate::openspec::import_plan(
                        &self.runtime,
                        context,
                        &request.bundle,
                        request.feature.as_deref(),
                    )?;
                    let written = changes.iter().map(|c| c.path.clone()).collect::<Vec<_>>();
                    Ok((
                        changes,
                        crate::transaction::PlannedResponse::Fields(
                            serde_json::json!({"written":written,"import_diagnostics":diagnostics.iter().map(|d|serde_json::json!({"code":d.code,"message":d.message})).collect::<Vec<_>>()}),
                        ),
                    ))
                },
            ),
            SpecRequest::Trace(request) => self.read_domain(&request.selector, |context, _| {
                crate::trace::data(&self.runtime, context, request.since.as_deref())
            }),
            SpecRequest::Drift(request) => self.read_domain(&request.selector, |context, _| {
                crate::drift::data(context, &request)
            }),
            SpecRequest::Feature(request) => self.read_domain(&request.selector, |_, files| {
                crate::query::feature_data(files, &request.feature)
            }),
            SpecRequest::Plan(request) => self.read_domain(&request.selector, |_, files| {
                crate::query::plan_data(files, &request.feature)
            }),
            SpecRequest::Lessons(request) => self.read_domain(&request.selector, |_, files| {
                crate::lessons::data(
                    files.get("spec/lessons.md").map_or(&[], Vec::as_slice),
                    &request.tags,
                )
            }),
            SpecRequest::ListSpecs(selector) => {
                self.read_domain(&selector, |_, files| Ok(crate::list_specs::data(files)))
            }
            SpecRequest::ScanMerges { selector, feature } => self
                .read_domain(&selector, |_, files| {
                    crate::scan_merges::data(files, feature.as_deref())
                }),
            SpecRequest::Validate(request) => {
                let (context, result) = self.validate(&request)?;
                Ok(SpecResponse {
                    context: Some(context),
                    data: result.value(),
                })
            }
            SpecRequest::Documents(selector) => crate::documents::list(self, &selector),
            SpecRequest::ReadDocument { selector, path } => {
                crate::documents::read(self, &selector, &path)
            }
            SpecRequest::WriteDocument(request) => crate::documents::write(self, request),
            SpecRequest::RemoveDocument(request) => crate::documents::remove(self, request),
            SpecRequest::Snapshot(request) => crate::storage::snapshot(self, request),
            SpecRequest::Projects => crate::storage::projects(self),
            SpecRequest::ProjectShow { selector } => crate::storage::show(self, &selector),
            SpecRequest::ProjectInit(request) => crate::storage::init_project(self, request),
            SpecRequest::ProjectAttach(request) => crate::storage::attach_project(self, request),
            SpecRequest::BranchInit(request) => crate::branch::init(self, request),
            SpecRequest::Context(selector) => {
                let context = self.resolve_context(&selector)?;
                Ok(SpecResponse {
                    data: crate::storage::context_value(&context),
                    context: Some(context),
                })
            }
        }
    }
}

impl SpecApi {
    pub(crate) fn read_domain(
        &self,
        selector: &crate::ContextSelector,
        f: impl FnOnce(
            &crate::SpecContext,
            &crate::documents::Files,
        ) -> Result<serde_json::Value, SpecError>,
    ) -> Result<SpecResponse, SpecError> {
        let (context, data) = crate::transaction::read_typed(self, selector, |context| {
            f(context, &crate::documents::collect(context)?)
        })?;
        Ok(SpecResponse {
            context: Some(context),
            data,
        })
    }
}
