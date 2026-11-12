//! Explicit private-spec reconciliation. Code repositories are read-only context.
use crate::{
    BranchId, ContextSelector, ProjectId, RecordId, SpecApi, SpecContext, SpecError, StorageMode,
    documents::{self, Files},
    registry, storage,
    transaction::{self, Locked},
};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};
fn err(code: &str, message: impl Into<String>) -> SpecError {
    SpecError::new(code, message)
}
fn invalid() -> SpecError {
    err("invalid_reconciliation", "malformed reconciliation record")
}
fn unrecorded_merge_diagnostic() -> Value {
    json!({"code":"unrecorded_merge_workspace","message":"interrupted merge material has no persisted index proof; preserve the workspace and replan from a new revision"})
}
fn exact_fields(value: &Value, fields: &[&str]) -> bool {
    value
        .as_object()
        .is_some_and(|o| o.len() == fields.len() && fields.iter().all(|name| o.contains_key(*name)))
}

#[derive(Clone, Debug)]
pub struct ReconcileRequest {
    pub selector: ContextSelector,
    pub source: String,
    pub target: String,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReconcilePlan {
    pub project: ProjectId,
    pub source: BranchId,
    pub target: BranchId,
    pub source_ref: String,
    pub target_ref: String,
    pub base_spec_oid: Option<String>,
    pub source_spec_oid: String,
    pub target_spec_oid: String,
    pub source_code_head: Option<String>,
    pub target_code_head: Option<String>,
    pub target_checkout: Option<PathBuf>,
    pub target_revision: String,
}
impl ReconcilePlan {
    pub fn value(&self) -> Value {
        json!({"v":1,"project":self.project.as_str(),"source":self.source.as_str(),"target":self.target.as_str(),"source_ref":self.source_ref,"target_ref":self.target_ref,"base_spec_oid":self.base_spec_oid,"source_spec_oid":self.source_spec_oid,"target_spec_oid":self.target_spec_oid,"source_code_head":self.source_code_head,"target_code_head":self.target_code_head,"target_checkout":self.target_checkout.as_deref().map(registry::path_value),"target_revision":self.target_revision})
    }
    pub fn from_value(v: &Value) -> Result<Self, SpecError> {
        let s = |k: &str| v[k].as_str().map(str::to_owned).ok_or_else(invalid);
        let optional = |k: &str| {
            if v[k].is_null() {
                Ok(None)
            } else {
                s(k).map(Some)
            }
        };
        if v["v"] != 1
            || !exact_fields(
                v,
                &[
                    "v",
                    "project",
                    "source",
                    "target",
                    "source_ref",
                    "target_ref",
                    "base_spec_oid",
                    "source_spec_oid",
                    "target_spec_oid",
                    "source_code_head",
                    "target_code_head",
                    "target_checkout",
                    "target_revision",
                ],
            )
        {
            return Err(invalid());
        }
        let plan = Self {
            project: s("project")?.parse()?,
            source: s("source")?.parse()?,
            target: s("target")?.parse()?,
            source_ref: s("source_ref")?,
            target_ref: s("target_ref")?,
            base_spec_oid: optional("base_spec_oid")?,
            source_spec_oid: s("source_spec_oid")?,
            target_spec_oid: s("target_spec_oid")?,
            source_code_head: optional("source_code_head")?,
            target_code_head: optional("target_code_head")?,
            target_checkout: if v["target_checkout"].is_null() {
                None
            } else {
                Some(registry::path(&v["target_checkout"])?)
            },
            target_revision: s("target_revision")?,
        };
        for oid in [&plan.source_spec_oid, &plan.target_spec_oid]
            .into_iter()
            .chain(plan.base_spec_oid.as_ref())
        {
            if oid.len() != 40 || !oid.bytes().all(|b| b.is_ascii_hexdigit()) {
                return Err(invalid());
            }
        }
        if crate::context::code_ref(&plan.source_ref)? != plan.source_ref
            || crate::context::code_ref(&plan.target_ref)? != plan.target_ref
            || plan
                .source_code_head
                .iter()
                .chain(plan.target_code_head.iter())
                .any(|oid| {
                    !matches!(oid.len(), 40 | 64) || !oid.bytes().all(|b| b.is_ascii_hexdigit())
                })
        {
            return Err(invalid());
        }
        Ok(plan)
    }
    fn selector(&self) -> ContextSelector {
        ContextSelector {
            project: Some(self.project.clone()),
            branch: Some(self.target_ref.clone()),
            checkout: None,
        }
    }
    pub(crate) fn apply_selector(&self) -> ContextSelector {
        let mut selector = self.selector();
        selector.checkout = self.target_checkout.clone();
        selector
    }
    pub(crate) fn id(&self) -> Result<RecordId, SpecError> {
        let bytes = serde_json::to_vec(&self.value()).map_err(documents::io)?;
        RecordId::new(format!(
            "reconcile-{}",
            vibe_core::markers::hash_bytes(&bytes)
        ))
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReconcileState {
    Pending,
    Applied,
    Conflict,
    NeedsInput,
}
impl ReconcileState {
    fn name(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Applied => "applied",
            Self::Conflict => "conflict",
            Self::NeedsInput => "needs-input",
        }
    }
}
#[derive(Clone, Debug)]
pub struct ReconcileRecord {
    pub id: RecordId,
    pub plan: ReconcilePlan,
    pub state: ReconcileState,
    pub result_spec_oid: Option<String>,
    pub conflicts: Vec<String>,
    pub diagnostics: Value,
    stages: BTreeMap<String, [Option<String>; 3]>,
    merge_index: Option<BTreeMap<String, String>>,
}
impl ReconcileRecord {
    pub fn value(&self) -> Value {
        json!({"v":1,"id":self.id.as_str(),"plan":self.plan.value(),"state":self.state.name(),"result_spec_oid":self.result_spec_oid,"conflicts":self.conflicts,"diagnostics":self.diagnostics,"stages":self.stages,"merge_index":self.merge_index})
    }
    fn parse(v: &Value) -> Result<Self, SpecError> {
        if v["v"] != 1
            || !exact_fields(
                v,
                &[
                    "v",
                    "id",
                    "plan",
                    "state",
                    "result_spec_oid",
                    "conflicts",
                    "diagnostics",
                    "stages",
                    "merge_index",
                ],
            )
        {
            return Err(invalid());
        }
        let plan = ReconcilePlan::from_value(&v["plan"])?;
        let id = v["id"].as_str().ok_or_else(invalid)?.parse::<RecordId>()?;
        if id != plan.id()? {
            return Err(invalid());
        }
        let state = match v["state"].as_str() {
            Some("pending") => ReconcileState::Pending,
            Some("applied") => ReconcileState::Applied,
            Some("conflict") => ReconcileState::Conflict,
            Some("needs-input") => ReconcileState::NeedsInput,
            _ => return Err(invalid()),
        };
        let result_spec_oid = if v["result_spec_oid"].is_null() {
            None
        } else {
            Some(v["result_spec_oid"].as_str().ok_or_else(invalid)?.into())
        };
        if result_spec_oid.as_ref().is_some_and(|oid: &String| {
            oid.len() != 40 || !oid.bytes().all(|b| b.is_ascii_hexdigit())
        }) {
            return Err(invalid());
        }
        if (state == ReconcileState::Applied) != result_spec_oid.is_some() {
            return Err(invalid());
        }
        let conflicts = v["conflicts"]
            .as_array()
            .ok_or_else(invalid)?
            .iter()
            .map(|p| p.as_str().map(str::to_owned).ok_or_else(invalid))
            .collect::<Result<Vec<_>, _>>()?;
        let stages: BTreeMap<String, [Option<String>; 3]> =
            serde_json::from_value(v["stages"].clone()).map_err(|_| invalid())?;
        if conflicts != stages.keys().cloned().collect::<Vec<_>>()
            || stages.values().any(|stages| {
                stages.iter().all(Option::is_none)
                    || stages
                        .iter()
                        .flatten()
                        .any(|oid| oid.len() != 40 || !oid.bytes().all(|b| b.is_ascii_hexdigit()))
            })
        {
            return Err(invalid());
        }
        let merge_index: Option<BTreeMap<String, String>> =
            serde_json::from_value(v["merge_index"].clone()).map_err(|_| invalid())?;
        if merge_index.as_ref().is_some_and(|files| {
            files
                .values()
                .any(|oid| oid.len() != 40 || !oid.bytes().all(|b| b.is_ascii_hexdigit()))
        }) {
            return Err(invalid());
        }
        if merge_index
            .as_ref()
            .is_some_and(|files| files.keys().any(|path| stages.contains_key(path)))
            || (state == ReconcileState::Conflict && stages.is_empty())
        {
            return Err(invalid());
        }
        Ok(Self {
            id,
            plan,
            state,
            result_spec_oid,
            conflicts,
            diagnostics: v["diagnostics"].clone(),
            stages,
            merge_index,
        })
    }
}
#[derive(Clone, Debug)]
pub struct ConflictContent {
    pub path: String,
    pub base: Option<String>,
    pub source: Option<String>,
    pub target: Option<String>,
    pub proposed: Option<String>,
    pub target_revision: String,
    pub resolution_revision: String,
}
impl ConflictContent {
    pub fn value(&self) -> Value {
        json!({"path":self.path,"base":self.base,"source":self.source,"target":self.target,"proposed":self.proposed,"target_revision":self.target_revision,"resolution_revision":self.resolution_revision})
    }
}
#[derive(Clone, Debug)]
pub struct ResolutionFile {
    pub path: String,
    pub content: Option<String>,
}
#[derive(Clone, Debug)]
pub struct ResolveRequest {
    pub project: ProjectId,
    pub record_id: RecordId,
    pub expected_target_revision: String,
    pub expected_resolution_revision: String,
    pub files: Vec<ResolutionFile>,
}
fn project_dir(api: &SpecApi, project: &ProjectId) -> Result<PathBuf, SpecError> {
    let r = registry::load(&api.runtime)?;
    if !r
        .projects
        .iter()
        .any(|p| &p.id == project && p.mode == StorageMode::GlobalFirst)
    {
        return Err(err(
            "unsupported_mode",
            "reconciliation requires a registered global-first project",
        ));
    }
    Ok(registry::project_dir(&r, project))
}
fn record_path(project: &Path, id: &RecordId) -> Result<PathBuf, SpecError> {
    let p = project
        .join("reconciliations")
        .join(id.as_str())
        .join("record.json");
    registry::confined(project, &p)?;
    Ok(p)
}
pub(crate) fn load(project: &Path, id: &RecordId) -> Result<ReconcileRecord, SpecError> {
    let bytes = fs::read(record_path(project, id)?).map_err(documents::io)?;
    let r = ReconcileRecord::parse(&serde_json::from_slice(&bytes).map_err(|_| invalid())?)?;
    if &r.id != id || project.file_name().and_then(|n| n.to_str()) != Some(r.plan.project.as_str())
    {
        return Err(invalid());
    }
    Ok(r)
}
fn save(project: &Path, r: &ReconcileRecord) -> Result<(), SpecError> {
    storage::verify_lock(&project.join("project.lock"))?;
    registry::write_json(project, &record_path(project, &r.id)?, &r.value())
}
fn select(project: &ProjectId, branch: &str) -> ContextSelector {
    ContextSelector {
        project: Some(project.clone()),
        branch: Some(branch.into()),
        checkout: None,
    }
}
fn clean(api: &SpecApi, c: &SpecContext, locked: &Locked) -> Result<(), SpecError> {
    documents::admit_global_tree(c)?;
    let oid = c.spec_revision.as_deref().ok_or_else(invalid)?;
    if transaction::committed_files(api, locked, oid)? != documents::collect(c)? {
        return Err(err(
            "dirty_tree",
            "snapshot raw document edits before reconciliation",
        ));
    }
    Ok(())
}
pub(crate) fn plan_locked(
    api: &SpecApi,
    request: &ReconcileRequest,
    target: &SpecContext,
    locked: &Locked,
) -> Result<ReconcilePlan, SpecError> {
    if target.mode != StorageMode::GlobalFirst {
        return Err(err(
            "unsupported_mode",
            "reconciliation requires global-first storage",
        ));
    }
    let project = target.project_id.as_ref().ok_or_else(invalid)?;
    let r = registry::load(&api.runtime)?;
    let state = registry::load_project(&r, project)?;
    let source_ref = crate::context::code_ref(&request.source)?;
    let target_ref = crate::context::code_ref(&request.target)?;
    let branch = |name: &str| {
        state
            .branches
            .iter()
            .find(|b| b.code_ref == name)
            .ok_or_else(|| err("branch_not_found", "stored spec branch is missing"))
    };
    let source = branch(&source_ref)?;
    let to = branch(&target_ref)?;
    if source.id == to.id {
        return Err(err(
            "invalid_reconciliation",
            "source and target must differ",
        ));
    }
    storage::ensure_existing(&api.runtime, &r, project, source)?;
    let source_context =
        crate::context::resolve_locked(&api.runtime, &select(project, &source_ref))?;
    clean(api, target, locked)?;
    clean(api, &source_context, locked)?;
    let source_oid = source_context.spec_revision.ok_or_else(invalid)?;
    let target_oid = target.spec_revision.clone().ok_or_else(invalid)?;
    let output = storage::spec_git_output(
        &api.runtime,
        locked.history.as_deref().ok_or_else(invalid)?,
        None,
        &["merge-base", "--all", &target_oid, &source_oid].map(Into::into),
        None,
    )?;
    let base = if output.status.success() {
        let text = String::from_utf8(output.stdout).map_err(documents::io)?;
        let bases = text.split_whitespace().collect::<Vec<_>>();
        if bases.len() == 1 {
            Some(bases[0].into())
        } else {
            None
        }
    } else if output.status.code() == Some(1) {
        None
    } else {
        return Err(err(
            "spec_git_failed",
            String::from_utf8_lossy(&output.stderr),
        ));
    };
    Ok(ReconcilePlan {
        project: project.clone(),
        source: source.id.clone(),
        target: to.id.clone(),
        source_ref,
        target_ref,
        base_spec_oid: base,
        source_spec_oid: source_oid,
        target_spec_oid: target_oid,
        source_code_head: source_context.code_head,
        target_code_head: target.code_head.clone(),
        target_checkout: target.checkout.clone(),
        target_revision: target.revision_token.clone(),
    })
}
pub fn plan(api: &SpecApi, request: ReconcileRequest) -> Result<ReconcilePlan, SpecError> {
    let mut selector = request.selector.clone();
    selector.branch = Some(crate::context::code_ref(&request.target)?);
    transaction::preview_locked(api, &selector, |target, locked| {
        plan_locked(api, &request, target, locked)
    })
}
fn workspace(project: &Path, r: &ReconcileRecord) -> Result<PathBuf, SpecError> {
    let p = project
        .join("reconciliations")
        .join(r.id.as_str())
        .join("workspace");
    registry::confined(project, &p)?;
    Ok(p)
}
fn workspace_context(target: &SpecContext, path: &Path) -> SpecContext {
    let mut c = target.clone();
    c.spec_root = path.join("spec");
    c.artifact_root = path.join("superpowers");
    c
}
fn git(api: &SpecApi, locked: &Locked, path: &Path, args: &[&str]) -> Result<Vec<u8>, SpecError> {
    storage::spec_git(
        &api.runtime,
        locked.history.as_deref().ok_or_else(invalid)?,
        Some(path),
        &args.iter().map(Into::into).collect::<Vec<_>>(),
    )
}
fn admit_workspace(
    api: &SpecApi,
    locked: &Locked,
    target: &SpecContext,
    path: &Path,
) -> Result<Files, SpecError> {
    registry::confined(&locked.anchor, path)?;
    for entry in fs::read_dir(path).map_err(documents::io)? {
        let e = entry.map_err(documents::io)?;
        let name = e.file_name();
        if name != ".git" && name != "spec" && name != "superpowers" {
            return Err(err(
                "invalid_storage",
                "foreign file in resolution workspace",
            ));
        }
    }
    let c = workspace_context(target, path);
    documents::admit_global_tree(&c)?;
    let files = documents::collect(&c)?;
    let staged = git(api, locked, path, &["ls-files", "--stage", "-z"])?;
    for entry in staged.split(|b| *b == 0).filter(|e| !e.is_empty()) {
        let text = std::str::from_utf8(entry).map_err(documents::io)?;
        let (header, p) = text.split_once('\t').ok_or_else(invalid)?;
        if !header.starts_with("100644 ") {
            return Err(err("invalid_storage", "unsupported resolution index entry"));
        }
        documents::path(&c, p)?;
    }
    Ok(files)
}
fn token(
    api: &SpecApi,
    locked: &Locked,
    target: &SpecContext,
    path: &Path,
    r: &ReconcileRecord,
) -> Result<String, SpecError> {
    let files = admit_workspace(api, locked, target, path)?;
    api.admit_files(&files)?;
    let index = git(api, locked, path, &["ls-files", "--stage", "-z"])?;
    let head = git(api, locked, path, &["rev-parse", "HEAD"])?;
    if std::str::from_utf8(&head).map_err(documents::io)?.trim() != r.plan.target_spec_oid {
        return Err(err("invalid_storage", "resolution workspace HEAD changed"));
    }
    let bytes = serde_json::to_vec(&json!([
        files,
        index,
        head,
        r.stages,
        r.merge_index,
        r.plan.value()
    ]))
    .map_err(documents::io)?;
    Ok(vibe_core::markers::hash_bytes(&bytes))
}
fn manifest(
    api: &SpecApi,
    locked: &Locked,
    target: &SpecContext,
    path: &Path,
) -> Result<BTreeMap<String, [Option<String>; 3]>, SpecError> {
    let mut result = BTreeMap::<String, [Option<String>; 3]>::new();
    let c = workspace_context(target, path);
    for entry in git(api, locked, path, &["ls-files", "--unmerged", "-z"])?
        .split(|b| *b == 0)
        .filter(|e| !e.is_empty())
    {
        let text = std::str::from_utf8(entry).map_err(documents::io)?;
        let (header, path) = text.split_once('\t').ok_or_else(invalid)?;
        documents::path(&c, path)?;
        let h = header.split_whitespace().collect::<Vec<_>>();
        if h.len() != 3 || h[0] != "100644" {
            return Err(invalid());
        }
        let stage = h[2].parse::<usize>().map_err(|_| invalid())?;
        if !(1..=3).contains(&stage) {
            return Err(invalid());
        }
        result.entry(path.into()).or_default()[stage - 1] = Some(h[1].into());
    }
    Ok(result)
}
fn recheck_source(api: &SpecApi, locked: &Locked, plan: &ReconcilePlan) -> Result<(), SpecError> {
    let registry = registry::load(&api.runtime)?;
    let state = registry::load_project(&registry, &plan.project)?;
    let branch = state
        .branches
        .iter()
        .find(|b| b.id == plan.source && b.code_ref == plan.source_ref)
        .ok_or_else(|| err("context_changed", "reconciliation source binding changed"))?;
    storage::ensure_existing(&api.runtime, &registry, &plan.project, branch)?;
    let source =
        crate::context::resolve_locked(&api.runtime, &select(&plan.project, &plan.source_ref))?;
    if source.spec_revision.as_deref() != Some(&plan.source_spec_oid)
        || source.code_head != plan.source_code_head
    {
        return Err(err(
            "revision_changed",
            "reconciliation source changed before publication",
        ));
    }
    clean(api, &source, locked).map_err(|e| {
        if e.code == "dirty_tree" {
            err(
                "revision_changed",
                "reconciliation source bytes changed before publication",
            )
        } else {
            e
        }
    })
}

fn index_state(
    api: &SpecApi,
    locked: &Locked,
    path: &Path,
) -> Result<BTreeMap<String, [Option<String>; 4]>, SpecError> {
    let mut entries = BTreeMap::<String, [Option<String>; 4]>::new();
    for entry in git(api, locked, path, &["ls-files", "--stage", "-z"])?
        .split(|b| *b == 0)
        .filter(|e| !e.is_empty())
    {
        let text = std::str::from_utf8(entry).map_err(documents::io)?;
        let (header, path) = text.split_once('\t').ok_or_else(invalid)?;
        let fields = header.split_whitespace().collect::<Vec<_>>();
        if fields.len() != 3 || fields[0] != "100644" {
            return Err(invalid());
        }
        let stage = fields[2].parse::<usize>().map_err(|_| invalid())?;
        if stage > 3 {
            return Err(invalid());
        }
        entries.entry(path.into()).or_default()[stage] = Some(fields[1].into());
    }
    Ok(entries)
}

fn clean_index(
    api: &SpecApi,
    locked: &Locked,
    path: &Path,
) -> Result<BTreeMap<String, String>, SpecError> {
    let mut files = BTreeMap::new();
    for entry in git(api, locked, path, &["ls-files", "--stage", "-z"])?
        .split(|b| *b == 0)
        .filter(|e| !e.is_empty())
    {
        let text = std::str::from_utf8(entry).map_err(documents::io)?;
        let (header, path) = text.split_once('\t').ok_or_else(invalid)?;
        let fields = header.split_whitespace().collect::<Vec<_>>();
        if fields.len() != 3 || fields[0] != "100644" {
            return Err(invalid());
        }
        if fields[2] == "0" {
            files.insert(path.into(), fields[1].into());
        }
    }
    Ok(files)
}
/// A fresh token authorizes only the manifest bundle, never unrelated workspace changes.
fn audit_merge(
    api: &SpecApi,
    locked: &Locked,
    target: &SpecContext,
    path: &Path,
    r: &ReconcileRecord,
) -> Result<(), SpecError> {
    let expected = r.merge_index.as_ref().ok_or_else(|| {
        err(
            "invalid_resolution",
            "merge workspace has no persisted clean-path evidence",
        )
    })?;
    let actual = admit_workspace(api, locked, target, path)?;
    let index = clean_index(api, locked, path)?
        .into_iter()
        .filter(|(p, _)| !r.stages.contains_key(p))
        .collect::<BTreeMap<_, _>>();
    if &index != expected {
        return Err(err(
            "invalid_resolution",
            "resolution index changed outside the conflict manifest",
        ));
    }
    let outside = actual
        .iter()
        .filter(|(p, _)| !r.stages.contains_key(*p))
        .map(|(p, b)| (p.clone(), b.clone()))
        .collect::<Files>();
    if outside.len() != expected.len() {
        return Err(err(
            "invalid_resolution",
            "resolution files changed outside the conflict manifest",
        ));
    }
    for (path, oid) in expected {
        if outside.get(path) != Some(&locked.git(api, &["cat-file", "blob", oid], None)?) {
            return Err(err(
                "invalid_resolution",
                "resolution bytes changed outside the conflict manifest",
            ));
        }
    }
    Ok(())
}

fn validate_publish(
    api: &SpecApi,
    target: &SpecContext,
    locked: &Locked,
    r: &mut ReconcileRecord,
    path: &Path,
    admitted: Option<&ReconcileRecord>,
    mut checkpoint: impl FnMut(transaction::Checkpoint) -> Result<(), SpecError>,
) -> Result<(), SpecError> {
    recheck_source(api, locked, &r.plan)?;
    audit_merge(api, locked, target, path, r)?;
    let expected = token(api, locked, target, path, r)?;
    let files = admit_workspace(api, locked, target, path)?;
    if !manifest(api, locked, target, path)?.is_empty() {
        return Err(err(
            "unresolved_conflicts",
            "resolution workspace still has unmerged entries",
        ));
    }
    let validation = crate::validate::completed_tree(
        &api.runtime,
        target,
        &path.join("spec"),
        &path.join("superpowers"),
    )?;
    if token(api, locked, target, path, r)? != expected {
        return Err(err(
            "revision_changed",
            "resolution workspace changed during validation",
        ));
    }
    r.diagnostics = validation.value();
    if admitted.is_some_and(|expected| expected.diagnostics != r.diagnostics) {
        return Err(err(
            "revision_changed",
            "merge validation differs from the admitted result",
        ));
    }
    let mut expected_result = r.clone();
    expected_result.state = ReconcileState::Applied;
    expected_result.result_spec_oid = Some("0".repeat(40));
    api.admit_result(&json!({"context":null,"data":expected_result.value()}))?;
    if validation.errors != 0 {
        r.state = ReconcileState::NeedsInput;
        api.check_control()?;
        save(&locked.anchor, r)?;
        return Ok(());
    }
    api.check_control()?;
    save(&locked.anchor, r)?;
    let original = r.clone();
    let selector = ContextSelector {
        project: target.project_id.clone(),
        branch: target.code_ref.clone(),
        checkout: target.checkout.clone(),
    };
    transaction::publish_merge(
        api,
        &selector,
        target,
        locked,
        files,
        &transaction::MergePublication {
            source: &r.plan.source_spec_oid,
            record_id: &r.id,
        },
        |point| {
            checkpoint(point)?;
            if point == transaction::Checkpoint::BeforePublish {
                recheck_source(api, locked, &original.plan)?;
            }
            if point == transaction::Checkpoint::BeforePublish
                && token(api, locked, target, path, &original)? != expected
            {
                return Err(err(
                    "revision_changed",
                    "resolution workspace changed before publication",
                ));
            }
            Ok(())
        },
    )?;
    *r = load(&locked.anchor, &r.id)?;
    Ok(())
}
fn admit_completed_receipt(
    api: &SpecApi,
    target: &SpecContext,
    path: &Path,
    record: &ReconcileRecord,
    conflicted: bool,
) -> Result<ReconcileRecord, SpecError> {
    let mut result = record.clone();
    if conflicted {
        result.state = ReconcileState::Conflict;
        api.admit_result(&json!({"context":null,"data":result.value()}))?;
    } else {
        result.diagnostics = crate::validate::completed_tree(
            &api.runtime,
            target,
            &path.join("spec"),
            &path.join("superpowers"),
        )?
        .value();
        result.state = ReconcileState::NeedsInput;
        api.admit_result(&json!({"context":null,"data":result.value()}))?;
        result.state = ReconcileState::Applied;
        result.result_spec_oid = Some(record.plan.target_spec_oid.clone());
        api.admit_result(&json!({"context":null,"data":result.value()}))?;
    }
    Ok(result)
}
fn admit_merge_receipt(
    api: &SpecApi,
    locked: &Locked,
    target: &SpecContext,
    record: &ReconcileRecord,
) -> Result<(ReconcileRecord, Files), SpecError> {
    let scratch = storage::AdmissionWorktree::new(
        api,
        locked.history.as_deref().ok_or_else(invalid)?,
        &record.plan.target_spec_oid,
    )?;
    let path = scratch.path();
    api.admit_files(&admit_workspace(api, locked, target, &path)?)?;
    api.check_control()?;
    let output = storage::spec_git_output(
        &api.runtime,
        locked.history.as_deref().ok_or_else(invalid)?,
        Some(&path),
        &[
            "merge",
            "--no-commit",
            "--no-ff",
            &record.plan.source_spec_oid,
        ]
        .map(Into::into),
        None,
    )?;
    // Admission scratch is disposable, unlike the durable resolution workspace.
    // Capture its completed proof before honoring cancellation, then release the
    // completion region before any canonical record or merge can begin.
    let completion = api.completing();
    let mut result = record.clone();
    result.stages = manifest(api, locked, target, &path)?;
    result.conflicts = result.stages.keys().cloned().collect();
    result.merge_index = Some(clean_index(api, locked, &path)?);
    scratch.save_proof(&result.value())?;
    drop(completion);
    api.check_control()?;
    if !output.status.success() && (output.status.code() != Some(1) || result.conflicts.is_empty())
    {
        return Err(err(
            "spec_git_failed",
            String::from_utf8_lossy(&output.stderr),
        ));
    }
    let files = admit_workspace(api, locked, target, &path)?;
    api.admit_files(&files)?;
    let result =
        admit_completed_receipt(api, target, &path, &result, !result.conflicts.is_empty())?;
    scratch.finish()?;
    api.check_control()?;
    Ok((result, files))
}
pub fn apply(
    api: &SpecApi,
    plan: &ReconcilePlan,
    expected_revision: &str,
) -> Result<ReconcileRecord, SpecError> {
    apply_checked(api, plan, expected_revision, |_| Ok(()))
}
pub(crate) fn apply_checked(
    api: &SpecApi,
    plan: &ReconcilePlan,
    expected_revision: &str,
    mut checkpoint: impl FnMut(transaction::Checkpoint) -> Result<(), SpecError>,
) -> Result<ReconcileRecord, SpecError> {
    // Round-trip rejects malformed public plans before any effect.
    let plan = ReconcilePlan::from_value(&plan.value())?;
    transaction::with_project_branch(api, &plan.project, &plan.target, |locked| {
        apply_locked(api, &plan, expected_revision, locked, &mut checkpoint)
    })
}
pub(crate) fn apply_locked(
    api: &SpecApi,
    plan: &ReconcilePlan,
    expected_revision: &str,
    locked: &Locked,
    mut checkpoint: impl FnMut(transaction::Checkpoint) -> Result<(), SpecError>,
) -> Result<ReconcileRecord, SpecError> {
    let id = plan.id()?;
    let location = record_path(&locked.anchor, &id)?;
    let existing = if location.exists() {
        let r = load(&locked.anchor, &id)?;
        if r.plan != *plan {
            return Err(invalid());
        }
        if r.state == ReconcileState::Applied {
            return Ok(r);
        }
        Some(r)
    } else {
        None
    };
    if transaction::recover_reconciliation_receipt(api, locked, plan, &id)? {
        return load(&locked.anchor, &id);
    }
    // Receipts survive checkout changes. New work still resolves the exact captured checkout.
    let selected_target = target_context(api, locked, plan)?;
    let target = &selected_target;
    registry::expect(&target.revision_token, expected_revision)?;
    registry::expect(&plan.target_revision, expected_revision)?;
    if let Some(record) = existing.as_ref().filter(|r| {
        r.state != ReconcileState::Pending
            && !(r.state == ReconcileState::NeedsInput
                && r.diagnostics["code"] == "automatic_evidence_unavailable")
    }) {
        return Ok(record.clone());
    }
    let actual = plan_locked(
        api,
        &ReconcileRequest {
            selector: plan.selector(),
            source: plan.source_ref.clone(),
            target: plan.target_ref.clone(),
        },
        target,
        locked,
    )?;
    if actual != *plan {
        return Err(err("revision_changed", "reconciliation preview changed"));
    }
    let mut record = existing.unwrap_or(ReconcileRecord {
        id,
        plan: plan.clone(),
        state: ReconcileState::Pending,
        result_spec_oid: None,
        conflicts: Vec::new(),
        diagnostics: Value::Null,
        stages: BTreeMap::new(),
        merge_index: None,
    });
    let integrated = if plan.base_spec_oid.is_some() {
        let ancestor = storage::spec_git_output(
            &api.runtime,
            locked.history.as_deref().ok_or_else(invalid)?,
            None,
            &[
                "merge-base",
                "--is-ancestor",
                &plan.source_spec_oid,
                &plan.target_spec_oid,
            ]
            .map(Into::into),
            None,
        )?;
        if !ancestor.status.success() && ancestor.status.code() != Some(1) {
            return Err(err(
                "spec_git_failed",
                String::from_utf8_lossy(&ancestor.stderr),
            ));
        }
        ancestor.status.success()
    } else {
        false
    };
    // The first durable Pending record must also fit its reachable final
    // receipt. A commit OID has the same wire size as the captured target OID.
    let mut final_receipt = record.clone();
    if plan.base_spec_oid.is_none() {
        final_receipt.state = ReconcileState::NeedsInput;
        final_receipt.diagnostics = json!({"code":"merge_base_unavailable"});
    } else {
        final_receipt.state = ReconcileState::Applied;
        final_receipt.result_spec_oid = Some(plan.target_spec_oid.clone());
    }
    api.admit_result(&json!({"context":null,"data":final_receipt.value()}))?;
    api.admit_result(&json!({"context":null,"data":record.value()}))?;
    let admitted = if api.limited() && plan.base_spec_oid.is_some() && !integrated {
        let path = workspace(&locked.anchor, &record)?;
        let result = if record.merge_index.is_some() {
            let files = admit_workspace(api, locked, target, &path)?;
            api.admit_files(&files)?;
            if manifest(api, locked, target, &path)?.is_empty() {
                let projected = admit_completed_receipt(api, target, &path, &record, false)?;
                (projected, files)
            } else {
                let mut result = record.clone();
                result.state = ReconcileState::Conflict;
                api.admit_result(&json!({"context":null,"data":result.value()}))?;
                (result, files)
            }
        } else {
            if path.exists() {
                let mut unrecorded = record.clone();
                unrecorded.state = ReconcileState::NeedsInput;
                unrecorded.diagnostics = unrecorded_merge_diagnostic();
                api.admit_result(&json!({"context":null,"data":unrecorded.value()}))?;
            }
            admit_merge_receipt(api, locked, target, &record)?
        };
        let fresh = target_context(api, locked, plan)?;
        registry::expect(&fresh.revision_token, expected_revision)?;
        if plan_locked(
            api,
            &ReconcileRequest {
                selector: plan.selector(),
                source: plan.source_ref.clone(),
                target: plan.target_ref.clone(),
            },
            &fresh,
            locked,
        )? != *plan
        {
            return Err(err(
                "revision_changed",
                "reconciliation inputs changed during result admission",
            ));
        }
        Some(result)
    } else {
        None
    };
    api.check_control()?;
    save(&locked.anchor, &record)?;
    if plan.base_spec_oid.is_none() {
        record.state = ReconcileState::NeedsInput;
        record.diagnostics = json!({"code":"merge_base_unavailable"});
        api.admit_result(&json!({"context":null,"data":record.value()}))?;
        api.check_control()?;
        save(&locked.anchor, &record)?;
        return Ok(record);
    }
    // An exact integrated source needs no new commit, but receives the same durable receipt.
    if integrated {
        let mut result = record.clone();
        result.state = ReconcileState::Applied;
        result.result_spec_oid = Some(plan.target_spec_oid.clone());
        api.admit_result(&json!({"context":null,"data":result.value()}))?;
        api.check_control()?;
        let _completion = api.completing();
        finish_receipt(api, &locked.anchor, &record.id, &plan.target_spec_oid)?;
        return load(&locked.anchor, &record.id);
    }
    let path = workspace(&locked.anchor, &record)?;
    api.check_control()?;
    checkpoint(transaction::Checkpoint::BeforeWrites)?;
    api.check_control()?;
    if record.merge_index.is_some() {
        let unresolved = manifest(api, locked, target, &path)?;
        if !unresolved.is_empty() {
            if unresolved != record.stages {
                return Err(err("invalid_resolution", "pending conflict stages changed"));
            }
            record.state = ReconcileState::Conflict;
            api.admit_result(&json!({"context":null,"data":record.value()}))?;
            api.check_control()?;
            save(&locked.anchor, &record)?;
            return Ok(record);
        }
        validate_publish(
            api,
            target,
            locked,
            &mut record,
            &path,
            admitted.as_ref().map(|(record, _)| record),
            &mut checkpoint,
        )?;
        return Ok(record);
    }
    if !path.exists() {
        storage::spec_git(
            &api.runtime,
            locked.history.as_deref().ok_or_else(invalid)?,
            None,
            &[
                std::ffi::OsString::from("worktree"),
                "add".into(),
                "--detach".into(),
                crate::git::argument_path(&path).into_os_string(),
                plan.target_spec_oid.clone().into(),
            ],
        )?;
    }
    admit_workspace(api, locked, target, &path)?;
    let merge_head = storage::spec_git_output(
        &api.runtime,
        locked.history.as_deref().ok_or_else(invalid)?,
        Some(&path),
        &["rev-parse", "--verify", "-q", "MERGE_HEAD"].map(Into::into),
        None,
    )?;
    if merge_head.status.success() {
        record.state = ReconcileState::NeedsInput;
        record.diagnostics = unrecorded_merge_diagnostic();
        api.admit_result(&json!({"context":null,"data":record.value()}))?;
        api.check_control()?;
        save(&locked.anchor, &record)?;
        return Ok(record);
    } else {
        if merge_head.status.code() != Some(1) {
            return Err(err(
                "spec_git_failed",
                String::from_utf8_lossy(&merge_head.stderr),
            ));
        }
        let baseline = transaction::committed_files(api, locked, &plan.target_spec_oid)?;
        let index =
            String::from_utf8(git(api, locked, &path, &["write-tree"])?).map_err(documents::io)?;
        let target_tree = locked.text(
            api,
            &["rev-parse", &format!("{}^{{tree}}", plan.target_spec_oid)],
            None,
        )?;
        if admit_workspace(api, locked, target, &path)? != baseline || index.trim() != target_tree {
            return Err(err(
                "dirty_tree",
                "pending resolution workspace changed before merge",
            ));
        }
        api.check_control()?;
        let output = storage::spec_git_output(
            &api.runtime,
            locked.history.as_deref().ok_or_else(invalid)?,
            Some(&path),
            &["merge", "--no-commit", "--no-ff", &plan.source_spec_oid].map(Into::into),
            None,
        )?;
        if !output.status.success()
            && (output.status.code() != Some(1) || manifest(api, locked, target, &path)?.is_empty())
        {
            return Err(err(
                "spec_git_failed",
                String::from_utf8_lossy(&output.stderr),
            ));
        }
    }
    // Persist the completed merge proof before honoring cooperative cancellation.
    let completion = api.completing();
    record.stages = manifest(api, locked, target, &path)?;
    record.conflicts = record.stages.keys().cloned().collect();
    record.merge_index = Some(clean_index(api, locked, &path)?);
    if !record.conflicts.is_empty() {
        record.state = ReconcileState::Conflict;
    }
    save(&locked.anchor, &record)?;
    drop(completion);
    api.check_control()?;
    if let Some((expected, files)) = admitted.as_ref()
        && (expected.stages != record.stages
            || expected.merge_index != record.merge_index
            || expected.conflicts != record.conflicts
            || admit_workspace(api, locked, target, &path)? != *files)
    {
        return Err(err(
            "revision_changed",
            "completed merge proof differs from the admitted result",
        ));
    }
    if record.state == ReconcileState::Conflict {
        api.admit_result(&json!({"context":null,"data":record.value()}))?;
        return Ok(record);
    }
    validate_publish(
        api,
        target,
        locked,
        &mut record,
        &path,
        admitted.as_ref().map(|(record, _)| record),
        &mut checkpoint,
    )?;
    Ok(record)
}

pub(crate) fn needs_input_locked(
    api: &SpecApi,
    locked: &Locked,
    plan: &ReconcilePlan,
    reason: &str,
) -> Result<ReconcileRecord, SpecError> {
    let id = plan.id()?;
    if record_path(&locked.anchor, &id)?.exists() {
        return load(&locked.anchor, &id);
    }
    let r = ReconcileRecord {
        id,
        plan: plan.clone(),
        state: ReconcileState::NeedsInput,
        result_spec_oid: None,
        conflicts: Vec::new(),
        diagnostics: json!({"code":reason}),
        stages: BTreeMap::new(),
        merge_index: None,
    };
    api.check_control()?;
    api.admit_result(&json!({"context":null,"data":r.value()}))?;
    save(&locked.anchor, &r)?;
    Ok(r)
}
/// Finish the receipt before the transaction journal is retired, including crash recovery.
pub(crate) fn finish_receipt(
    api: &SpecApi,
    project: &Path,
    id: &RecordId,
    result: &str,
) -> Result<(), SpecError> {
    let mut r = load(project, id)?;
    let history = project.join("history.git");
    let parents = storage::git_text(
        &api.runtime,
        &history,
        None,
        &["rev-list", "--parents", "--max-count=1", result],
    )?;
    let expected = format!(
        "{} {} {}",
        result, r.plan.target_spec_oid, r.plan.source_spec_oid
    );
    if result != r.plan.target_spec_oid && parents != expected {
        return Err(err(
            "invalid_history",
            "reconciliation result parent linkage differs",
        ));
    }
    if result != r.plan.target_spec_oid {
        let message = storage::git_text(
            &api.runtime,
            &history,
            None,
            &["show", "-s", "--format=%B", result],
        )?;
        if !message
            .lines()
            .any(|l| l == format!("Vibe-Spec-Reconciliation: {id}"))
        {
            return Err(err(
                "invalid_history",
                "reconciliation commit linkage is missing",
            ));
        }
    }
    if r.result_spec_oid
        .as_deref()
        .is_some_and(|oid| oid != result)
    {
        return Err(invalid());
    }
    r.result_spec_oid = Some(result.into());
    r.state = ReconcileState::Applied;
    let _completion = api.completing();
    save(project, &r)
}
fn target_context(
    api: &SpecApi,
    locked: &Locked,
    plan: &ReconcilePlan,
) -> Result<SpecContext, SpecError> {
    transaction::recover_reconciliation_target(api, locked, plan)
}

pub fn status(
    api: &SpecApi,
    project: &ProjectId,
    id: &RecordId,
) -> Result<ReconcileRecord, SpecError> {
    let root = project_dir(api, project)?;
    let initial = load(&root, id)?;
    if &initial.plan.project != project {
        return Err(invalid());
    }
    transaction::with_project_branch(api, &initial.plan.project, &initial.plan.target, |locked| {
        let record = load(&locked.anchor, id)?;
        if record.state == ReconcileState::Applied {
            return Ok(record);
        }
        if transaction::recover_reconciliation_receipt(api, locked, &record.plan, id)? {
            return load(&locked.anchor, id);
        }
        target_context(api, locked, &record.plan)?;
        load(&locked.anchor, id)
    })
}
pub fn read_conflict(
    api: &SpecApi,
    project: &ProjectId,
    id: &RecordId,
    logical: &str,
) -> Result<ConflictContent, SpecError> {
    let root = project_dir(api, project)?;
    let initial = load(&root, id)?;
    transaction::with_project_branch(api, &initial.plan.project, &initial.plan.target, |locked| {
        let r = load(&locked.anchor, id)?;
        transaction::recover_reconciliation_receipt(api, locked, &r.plan, id)?;
        let selected = target_context(api, locked, &r.plan)?;
        let target = &selected;
        let stages = r
            .stages
            .get(logical)
            .ok_or_else(|| err("invalid_resolution", "path is outside conflict manifest"))?;
        let path = workspace(&locked.anchor, &r)?;
        let c = workspace_context(target, &path);
        let text = |oid: &Option<String>| -> Result<Option<String>, SpecError> {
            oid.as_ref()
                .map(|oid| {
                    locked
                        .git(api, &["cat-file", "blob", oid], None)
                        .and_then(|b| String::from_utf8(b).map_err(documents::io))
                })
                .transpose()
        };
        Ok(ConflictContent {
            path: logical.into(),
            base: text(&stages[0])?,
            source: text(&stages[2])?,
            target: text(&stages[1])?,
            proposed: documents::bytes(&c, logical)?
                .map(String::from_utf8)
                .transpose()
                .map_err(documents::io)?,
            target_revision: target.revision_token.clone(),
            resolution_revision: token(api, locked, target, &path, &r)?,
        })
    })
}
pub fn resolve(api: &SpecApi, request: ResolveRequest) -> Result<ReconcileRecord, SpecError> {
    resolve_checked(api, request, |_| Ok(()))
}
pub(crate) fn resolve_checked(
    api: &SpecApi,
    request: ResolveRequest,
    mut checkpoint: impl FnMut(transaction::Checkpoint) -> Result<(), SpecError>,
) -> Result<ReconcileRecord, SpecError> {
    let root = project_dir(api, &request.project)?;
    let initial = load(&root, &request.record_id)?;
    transaction::with_project_branch(api, &initial.plan.project, &initial.plan.target, |locked| {
        transaction::recover_reconciliation_receipt(
            api,
            locked,
            &initial.plan,
            &request.record_id,
        )?;
        let mut r = load(&locked.anchor, &request.record_id)?;
        if r.state == ReconcileState::Applied {
            return Err(err("already_applied", "reconciliation already applied"));
        }
        let selected = target_context(api, locked, &r.plan)?;
        let target = &selected;
        if r.conflicts.is_empty() {
            return Err(err("invalid_resolution", "record has no conflict manifest"));
        }
        registry::expect(&target.revision_token, &request.expected_target_revision)?;
        if target.spec_revision.as_deref() != Some(&r.plan.target_spec_oid) {
            return Err(err(
                "revision_changed",
                "canonical target advanced; replan reconciliation",
            ));
        }
        clean(api, target, locked)?;
        let path = workspace(&locked.anchor, &r)?;
        registry::expect(
            &token(api, locked, target, &path, &r)?,
            &request.expected_resolution_revision,
        )?;
        audit_merge(api, locked, target, &path, &r)?;
        let c = workspace_context(target, &path);
        let mut names = BTreeSet::new();
        for file in &request.files {
            documents::path(&c, &file.path)
                .map_err(|_| err("invalid_resolution", "invalid resolution path"))?;
            if !names.insert(file.path.clone()) || !r.stages.contains_key(&file.path) {
                return Err(err(
                    "invalid_resolution",
                    "resolution paths must exactly cover the conflict manifest",
                ));
            }
        }
        if names != r.conflicts.iter().cloned().collect() {
            return Err(err("invalid_resolution", "resolution bundle is incomplete"));
        }
        let mut expected_files = admit_workspace(api, locked, target, &path)?;
        let mut expected_index = index_state(api, locked, &path)?;
        let mut expected_token = request.expected_resolution_revision.clone();
        checkpoint(transaction::Checkpoint::BeforeWrites)?;
        for file in request.files {
            api.check_control()?;
            registry::expect(&token(api, locked, target, &path, &r)?, &expected_token)?;
            if documents::bytes(&c, &file.path)?.as_ref() != expected_files.get(&file.path) {
                return Err(err(
                    "revision_changed",
                    "resolution path changed before write",
                ));
            }
            let intended = file.content.as_ref().map(|s| s.as_bytes().to_vec());
            let expected_oid = intended
                .as_ref()
                .map(|bytes| {
                    locked.text(
                        api,
                        &["hash-object", "--stdin", "--no-filters"],
                        Some(bytes),
                    )
                })
                .transpose()?;
            let p = documents::path(&c, &file.path)?;
            match file.content {
                Some(content) => {
                    fs::create_dir_all(p.parent().ok_or_else(invalid)?).map_err(documents::io)?;
                    vibe_core::fs::write_atomic(&p, content.as_bytes()).map_err(documents::io)?;
                }
                None => match fs::remove_file(&p) {
                    Ok(()) => (),
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
                    Err(e) => return Err(documents::io(e)),
                },
            }
            git(api, locked, &path, &["add", "-A", "--", &file.path])?;
            match intended {
                Some(bytes) => {
                    expected_files.insert(file.path.clone(), bytes);
                }
                None => {
                    expected_files.remove(&file.path);
                }
            }
            match expected_oid {
                Some(oid) => {
                    expected_index.insert(file.path.clone(), [Some(oid), None, None, None]);
                }
                None => {
                    expected_index.remove(&file.path);
                }
            }
            if admit_workspace(api, locked, target, &path)? != expected_files
                || index_state(api, locked, &path)? != expected_index
            {
                return Err(err(
                    "revision_changed",
                    "foreign resolution edits were preserved",
                ));
            }
            expected_token = token(api, locked, target, &path, &r)?;
            checkpoint(transaction::Checkpoint::AfterWrites)?;
        }
        registry::expect(&token(api, locked, target, &path, &r)?, &expected_token)?;
        validate_publish(api, target, locked, &mut r, &path, None, &mut checkpoint)?;
        Ok(r)
    })
}
impl SpecApi {
    pub fn plan_reconciliation(
        &self,
        request: ReconcileRequest,
    ) -> Result<ReconcilePlan, SpecError> {
        plan(self, request)
    }
    pub fn apply_reconciliation(
        &self,
        plan: &ReconcilePlan,
        expected: &str,
    ) -> Result<ReconcileRecord, SpecError> {
        apply(self, plan, expected)
    }
    pub fn reconciliation_status(
        &self,
        project: &ProjectId,
        id: &RecordId,
    ) -> Result<ReconcileRecord, SpecError> {
        status(self, project, id)
    }
    pub fn read_reconciliation_conflict(
        &self,
        project: &ProjectId,
        id: &RecordId,
        path: &str,
    ) -> Result<ConflictContent, SpecError> {
        read_conflict(self, project, id, path)
    }
    pub fn resolve_reconciliation(
        &self,
        request: ResolveRequest,
    ) -> Result<ReconcileRecord, SpecError> {
        resolve(self, request)
    }
}

/// Exact structured bundle parser shared by adapters; domain paths are admitted under the lock.
pub fn resolution_files(bundle: &Value) -> Result<Vec<ResolutionFile>, SpecError> {
    let invalid = || {
        err(
            "invalid_resolution",
            "expected {files:[{path,content}]} with text or null content",
        )
    };
    if bundle.as_object().is_none_or(|o| o.len() != 1) {
        return Err(invalid());
    }
    bundle["files"]
        .as_array()
        .ok_or_else(invalid)?
        .iter()
        .map(|file| {
            if file.as_object().is_none_or(|o| o.len() != 2) || file.get("content").is_none() {
                return Err(invalid());
            }
            Ok(ResolutionFile {
                path: file["path"].as_str().ok_or_else(invalid)?.into(),
                content: if file["content"].is_null() {
                    None
                } else {
                    Some(file["content"].as_str().ok_or_else(invalid)?.into())
                },
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;
    #[test]
    fn spec_projects_mcp_8_admission_cleanup_preserves_changed_binding() {
        let (_owner, api, context) = crate::transaction::tests::fixture();
        let registry = registry::load(&api.runtime).unwrap();
        let root = registry::project_dir(&registry, context.project_id.as_ref().unwrap());
        let history = root.join("history.git");
        let scratch = storage::AdmissionWorktree::new(
            &api,
            &history,
            context.spec_revision.as_ref().unwrap(),
        )
        .unwrap();
        let path = scratch.path();
        let binding = fs::read(path.join(".git")).unwrap();
        let registration =
            crate::git::output_path(binding.strip_prefix(b"gitdir: ").unwrap()).unwrap();
        fs::write(path.join(".git"), b"foreign binding\n").unwrap();
        assert_eq!(scratch.finish().unwrap_err().code, "invalid_storage");
        assert_eq!(fs::read(path.join(".git")).unwrap(), b"foreign binding\n");
        assert!(registration.is_dir());
    }
    fn setup(api: &SpecApi) -> ReconcilePlan {
        let show = api
            .execute(crate::SpecRequest::ProjectShow {
                selector: ContextSelector::default(),
            })
            .unwrap();
        api.execute(crate::SpecRequest::BranchInit(crate::InitBranchRequest {
            selector: ContextSelector::default(),
            name: "topic".into(),
            from: "main".into(),
            expected_revision: show.data["revision"].as_str().unwrap().into(),
            expected_project_revision: show.data["project_revision"].as_str().unwrap().into(),
        }))
        .unwrap();
        let selector = ContextSelector {
            branch: Some("topic".into()),
            ..Default::default()
        };
        let context = api.resolve_context(&selector).unwrap();
        api.execute(crate::SpecRequest::WriteDocument(
            crate::WriteDocumentRequest {
                selector,
                path: "spec/lessons.md".into(),
                content: "# Lessons\n\nSource.\n".into(),
                expected_revision: context.revision_token,
            },
        ))
        .unwrap();
        plan(
            api,
            ReconcileRequest {
                selector: ContextSelector::default(),
                source: "topic".into(),
                target: "main".into(),
            },
        )
        .unwrap()
    }
    #[test]
    fn spec_projects_mcp_8_admission_scratch_cancellation_and_changed_inputs_leave_no_record() {
        use std::sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
        };
        for change in ["cancel", "source", "target"] {
            let (_owner, api, _) = crate::transaction::tests::fixture();
            let p = setup(&api);
            let registry = registry::load(&api.runtime).unwrap();
            let root = registry::project_dir(&registry, &p.project);
            let original = fs::read(root.join("project.json")).unwrap();
            let registrations = || {
                fs::read_dir(root.join("history.git/worktrees"))
                    .unwrap()
                    .map(|entry| entry.unwrap().file_name())
                    .collect::<BTreeSet<_>>()
            };
            let before = registrations();
            let selected = crate::context::resolve_locked(
                &api.runtime,
                &select(
                    &p.project,
                    if change == "source" { "topic" } else { "main" },
                ),
            )
            .unwrap();
            let watched = root.clone();
            let cancelled = Arc::new(AtomicBool::new(false));
            let flag = cancelled.clone();
            let mut control = crate::InvocationControl::new(move || {
                if change == "cancel"
                    && fs::read_dir(&watched).unwrap().any(|e| {
                        let e = e.unwrap();
                        e.file_name()
                            .to_string_lossy()
                            .starts_with(".response-admission-")
                            && e.path().join("proof.json").is_file()
                    })
                {
                    flag.store(true, Ordering::SeqCst);
                }
                flag.load(Ordering::SeqCst)
            });
            let changed = Arc::new(AtomicBool::new(false));
            let flag = changed.clone();
            let path = selected.spec_root.join("lessons.md");
            control.limited = true;
            control.result_admission = Some(Arc::new(move |value| {
                if change != "cancel"
                    && value["data"]["merge_index"].is_object()
                    && !flag.swap(true, Ordering::SeqCst)
                {
                    fs::write(&path, b"foreign after admission\n").unwrap();
                }
                Ok(())
            }));
            let controlled = api.with_control(control);
            let error = apply(&controlled, &p, &p.target_revision).unwrap_err();
            if change == "cancel" {
                assert_eq!(error.code, "cancelled");
                assert_eq!(
                    controlled.check_control().unwrap_err().code,
                    "cancelled",
                    "scratch completion mask must be released"
                );
            } else {
                assert!(matches!(
                    error.code.as_str(),
                    "revision_changed" | "dirty_tree"
                ));
                assert!(changed.load(Ordering::SeqCst));
                assert_eq!(
                    fs::read(selected.spec_root.join("lessons.md")).unwrap(),
                    b"foreign after admission\n"
                );
            }
            assert!(!record_path(&root, &p.id().unwrap()).unwrap().exists());
            assert_eq!(fs::read(root.join("project.json")).unwrap(), original);
            assert!(!fs::read_dir(&root).unwrap().any(|e| {
                e.unwrap()
                    .file_name()
                    .to_string_lossy()
                    .starts_with(".response-admission-")
            }));
            assert_eq!(
                registrations(),
                before,
                "cleanup must remove only its exact Git registration"
            );
            if change == "cancel" {
                assert_eq!(
                    apply(&api, &p, &p.target_revision).unwrap().state,
                    ReconcileState::Applied
                );
            }
        }
    }
    #[test]
    fn spec_projects_mcp_8_retry_result_admission_preserves_existing_unproven_records() {
        use std::sync::Arc;
        for state in [ReconcileState::Pending, ReconcileState::NeedsInput] {
            let (_owner, api, _) = crate::transaction::tests::fixture();
            let p = setup(&api);
            let mut location = None;
            transaction::with_project_branch(&api, &p.project, &p.target, |locked| {
                let record = ReconcileRecord {
                    id: p.id()?,
                    plan: p.clone(),
                    state,
                    result_spec_oid: None,
                    conflicts: Vec::new(),
                    diagnostics: if state == ReconcileState::NeedsInput {
                        json!({"code":"automatic_evidence_unavailable"})
                    } else {
                        Value::Null
                    },
                    stages: BTreeMap::new(),
                    merge_index: None,
                };
                save(&locked.anchor, &record)?;
                location = Some(record_path(&locked.anchor, &record.id)?);
                Ok(())
            })
            .unwrap();
            let location = location.unwrap();
            let original = fs::read(&location).unwrap();
            let mut control = crate::InvocationControl::new(|| false);
            control.limited = true;
            control.result_admission = Some(Arc::new(|value| {
                if value["data"]["merge_index"].is_object() {
                    Err(err("size_limit", "test complete proof refusal"))
                } else {
                    Ok(())
                }
            }));
            let controlled = api.with_control(control);
            assert_eq!(
                apply(&controlled, &p, &p.target_revision).unwrap_err().code,
                "size_limit"
            );
            assert_eq!(fs::read(&location).unwrap(), original);
            assert!(!location.parent().unwrap().join("workspace").exists());
            assert_eq!(
                apply(&api, &p, &p.target_revision).unwrap().state,
                ReconcileState::Applied
            );
        }
    }
    #[test]
    fn spec_projects_mcp_6_source_changes_at_prepublication_checkpoint_refuse() {
        let (_owner, api, _) = crate::transaction::tests::fixture();
        let p = setup(&api);
        let source = api.resolve_context(&select(&p.project, "topic")).unwrap();
        let error = apply_checked(&api, &p, &p.target_revision, |point| {
            if point == transaction::Checkpoint::BeforePublish {
                fs::write(
                    source.spec_root.join("lessons.md"),
                    b"post-preview source bytes",
                )
                .unwrap();
            }
            Ok(())
        })
        .unwrap_err();
        assert_eq!(error.code, "revision_changed");
        let current = api.resolve_context(&p.selector());
        assert!(current.is_err());
        let record = status(&api, &p.project, &p.id().unwrap()).unwrap();
        assert_eq!(record.state, ReconcileState::Pending);
        let target = api.resolve_context(&p.selector()).unwrap();
        assert_eq!(
            target.spec_revision.as_deref(),
            Some(p.target_spec_oid.as_str())
        );
    }
    #[test]
    fn spec_projects_mcp_6_ref_before_receipt_recovers_once_and_preserves_later_bytes() {
        let (_owner, api, _) = crate::transaction::tests::fixture();
        let p = setup(&api);
        let error = apply_checked(&api, &p, &p.target_revision, |point| {
            if point == transaction::Checkpoint::AfterPublish {
                Err(err("cancelled", "interrupted after CAS"))
            } else {
                Ok(())
            }
        })
        .unwrap_err();
        assert_eq!(error.code, "cancelled");
        let registry = registry::load(&api.runtime).unwrap();
        let root = registry::project_dir(&registry, &p.project);
        let pending = load(&root, &p.id().unwrap()).unwrap();
        assert_eq!(pending.state, ReconcileState::Pending);
        let raw = crate::context::resolve_for_transaction(&api.runtime, &p.selector()).unwrap();
        fs::write(raw.spec_root.join("lessons.md"), b"later external bytes").unwrap();
        let moved = std::process::Command::new("git")
            .args(["checkout", "-b", "moved-after-cas"])
            .current_dir(&api.runtime.cwd)
            .env_clear()
            .env("PATH", std::env::var_os("PATH").unwrap_or_default())
            .output()
            .unwrap();
        assert!(moved.status.success());
        let r = status(&api, &p.project, &pending.id).unwrap();
        assert_eq!(r.state, ReconcileState::Applied);
        let count = storage::git_text(
            &api.runtime,
            &root.join("history.git"),
            None,
            &["rev-list", "--count", r.result_spec_oid.as_deref().unwrap()],
        )
        .unwrap();
        assert_eq!(
            apply(&api, &p, &p.target_revision).unwrap().result_spec_oid,
            r.result_spec_oid
        );
        assert_eq!(
            storage::git_text(
                &api.runtime,
                &root.join("history.git"),
                None,
                &["rev-list", "--count", r.result_spec_oid.as_deref().unwrap()]
            )
            .unwrap(),
            count
        );
        assert_eq!(
            fs::read(raw.spec_root.join("lessons.md")).unwrap(),
            b"later external bytes"
        );
        assert!(!root.join(format!("transaction-{}.json", p.target)).exists());
    }
    #[test]
    fn spec_projects_mcp_6_recovery_finishes_receipt_when_result_has_descendant() {
        let (_owner, api, _) = crate::transaction::tests::fixture();
        let p = setup(&api);
        apply_checked(&api, &p, &p.target_revision, |point| {
            if point == transaction::Checkpoint::AfterPublish {
                Err(err("cancelled", "after CAS"))
            } else {
                Ok(())
            }
        })
        .unwrap_err();
        let c = crate::context::resolve_for_transaction(&api.runtime, &p.selector()).unwrap();
        let root = project_dir(&api, &p.project).unwrap();
        let result = c.spec_revision.as_deref().unwrap();
        let history = root.join("history.git");
        let tree = storage::git_text(
            &api.runtime,
            &history,
            None,
            &["rev-parse", &format!("{result}^{{tree}}")],
        )
        .unwrap();
        let later = storage::git_text(
            &api.runtime,
            &history,
            None,
            &[
                "commit-tree",
                &tree,
                "-p",
                result,
                "-m",
                "later preserved history",
            ],
        )
        .unwrap();
        let registry = registry::load(&api.runtime).unwrap();
        let state = registry::load_project(&registry, &p.project).unwrap();
        let branch = state.branches.iter().find(|b| b.id == p.target).unwrap();
        storage::git_text(
            &api.runtime,
            &history,
            None,
            &["update-ref", &branch.spec_ref, &later, result],
        )
        .unwrap();
        let record = status(&api, &p.project, &p.id().unwrap()).unwrap();
        assert_eq!(record.state, ReconcileState::Applied);
        assert_eq!(record.result_spec_oid.as_deref(), Some(result));
        assert_eq!(
            api.resolve_context(&p.selector())
                .unwrap()
                .spec_revision
                .as_deref(),
            Some(later.as_str())
        );
    }
    #[test]
    fn spec_projects_mcp_6_prepublication_retry_reuses_completed_workspace() {
        let (_owner, api, _) = crate::transaction::tests::fixture();
        let p = setup(&api);
        apply_checked(&api, &p, &p.target_revision, |point| {
            if point == transaction::Checkpoint::BeforePublish {
                Err(err("cancelled", "before CAS"))
            } else {
                Ok(())
            }
        })
        .unwrap_err();
        let root = project_dir(&api, &p.project).unwrap();
        let r = apply(&api, &p, &p.target_revision).unwrap();
        assert_eq!(r.state, ReconcileState::Applied);
        assert_eq!(
            fs::read_dir(root.join("reconciliations")).unwrap().count(),
            1
        );
        assert_eq!(
            apply(&api, &p, &p.target_revision).unwrap().result_spec_oid,
            r.result_spec_oid
        );
    }
    #[test]
    fn followup_reconciliation_journal_uses_physical_root_identity() {
        let (owner, mut api, _) = crate::transaction::tests::fixture();
        let store = std::path::PathBuf::from(
            api.runtime
                .env
                .get(std::ffi::OsStr::new("VIBE_STORE"))
                .unwrap(),
        );
        api.runtime
            .env
            .insert("VIBE_STORE".into(), store.join(".").into_os_string());
        let plan = setup(&api);
        let interrupted = apply_checked(&api, &plan, &plan.target_revision, |point| {
            if point == transaction::Checkpoint::AfterPublish {
                Err(err("cancelled", "after CAS"))
            } else {
                Ok(())
            }
        })
        .unwrap_err();
        assert_eq!(interrupted.code, "cancelled");
        let root = project_dir(&api, &plan.project).unwrap();
        let path = root.join(format!("transaction-{}.json", plan.target));
        let bytes = fs::read(&path).unwrap();
        for key in ["spec_root", "artifact_root"] {
            let mut foreign: Value = serde_json::from_slice(&bytes).unwrap();
            foreign[key] = registry::path_value(&owner.join("foreign"));
            fs::write(&path, serde_json::to_vec(&foreign).unwrap()).unwrap();
            assert_eq!(
                status(&api, &plan.project, &plan.id().unwrap())
                    .unwrap_err()
                    .code,
                "invalid_transaction"
            );
        }
        #[cfg(unix)]
        {
            let mut alias: Value = serde_json::from_slice(&bytes).unwrap();
            let root = registry::path(&alias["spec_root"]).unwrap();
            let saved = owner.join("saved-root-alias");
            std::os::unix::fs::symlink(root, &saved).unwrap();
            alias["spec_root"] = registry::path_value(&saved);
            fs::write(&path, serde_json::to_vec(&alias).unwrap()).unwrap();
            assert_eq!(
                status(&api, &plan.project, &plan.id().unwrap())
                    .unwrap_err()
                    .code,
                "invalid_transaction"
            );
        }
        let mut equivalent: Value = serde_json::from_slice(&bytes).unwrap();
        for key in ["spec_root", "artifact_root"] {
            let physical = registry::path(&equivalent[key]).unwrap();
            equivalent[key] = registry::path_value(&physical.join("."));
        }
        fs::write(&path, serde_json::to_vec(&equivalent).unwrap()).unwrap();
        let recovered = status(&api, &plan.project, &plan.id().unwrap()).unwrap();
        assert_eq!(recovered.state, ReconcileState::Applied);
        assert!(!path.exists());
    }
    fn conflict_fixture(api: &SpecApi) -> (ReconcilePlan, ReconcileRecord, ConflictContent) {
        setup(api);
        let context = api.resolve_context(&ContextSelector::default()).unwrap();
        api.execute(crate::SpecRequest::WriteDocument(
            crate::WriteDocumentRequest {
                selector: ContextSelector::default(),
                path: "spec/lessons.md".into(),
                content: "# Lessons\n\nTarget.\n".into(),
                expected_revision: context.revision_token,
            },
        ))
        .unwrap();
        let p = plan(
            api,
            ReconcileRequest {
                selector: ContextSelector::default(),
                source: "topic".into(),
                target: "main".into(),
            },
        )
        .unwrap();
        let r = apply(api, &p, &p.target_revision).unwrap();
        let content = read_conflict(api, &p.project, &r.id, "spec/lessons.md").unwrap();
        (p, r, content)
    }
    #[test]
    fn spec_projects_mcp_6_review_beforewrites_foreign_resolution_edit_survives() {
        let (_owner, api, _) = crate::transaction::tests::fixture();
        let (p, r, c) = conflict_fixture(&api);
        let root = project_dir(&api, &p.project).unwrap();
        let path = workspace(&root, &r).unwrap();
        let request = ResolveRequest {
            project: p.project.clone(),
            record_id: r.id,
            expected_target_revision: c.target_revision,
            expected_resolution_revision: c.resolution_revision,
            files: vec![ResolutionFile {
                path: "spec/lessons.md".into(),
                content: Some("# Lessons\n\nResolved.\n".into()),
            }],
        };
        let mut injected = false;
        let error = resolve_checked(&api, request, |point| {
            if point == transaction::Checkpoint::BeforeWrites && !injected {
                injected = true;
                fs::write(path.join("spec/lessons.md"), b"foreign edit after token").unwrap();
            }
            Ok(())
        })
        .unwrap_err();
        assert_eq!(error.code, "revision_changed");
        assert_eq!(
            fs::read(path.join("spec/lessons.md")).unwrap(),
            b"foreign edit after token"
        );
        assert_eq!(
            api.resolve_context(&p.selector())
                .unwrap()
                .spec_revision
                .as_deref(),
            Some(p.target_spec_oid.as_str())
        );
    }
    #[test]
    fn spec_projects_mcp_6_review_proven_pending_conflict_recovers_as_conflict() {
        let (_owner, api, _) = crate::transaction::tests::fixture();
        let (p, mut r, _) = conflict_fixture(&api);
        let root = project_dir(&api, &p.project).unwrap();
        r.state = ReconcileState::Pending;
        let context = api.resolve_context(&p.selector()).unwrap();
        let _lock = crate::transaction::tests::record_lock(&api, &context);
        save(&root, &r).unwrap();
        drop(_lock);
        assert_eq!(
            apply(&api, &p, &p.target_revision).unwrap().state,
            ReconcileState::Conflict
        );
    }
    #[test]
    fn spec_projects_mcp_6_review_multipath_preserves_foreign_later_path_and_index() {
        let (_owner, api, _) = crate::transaction::tests::fixture();
        setup(&api);
        let original = api
            .read_document(&ContextSelector::default(), "spec/design.md")
            .unwrap()
            .document
            .content;
        for (branch, path, text) in [
            ("main", "spec/lessons.md", "# Lessons\n\nTarget.\n".into()),
            ("main", "spec/design.md", format!("{original}\nTarget.\n")),
            ("topic", "spec/design.md", format!("{original}\nSource.\n")),
        ] {
            let selector = ContextSelector {
                branch: Some(branch.into()),
                ..Default::default()
            };
            let c = api.resolve_context(&selector).unwrap();
            api.execute(crate::SpecRequest::WriteDocument(
                crate::WriteDocumentRequest {
                    selector,
                    path: path.into(),
                    content: text,
                    expected_revision: c.revision_token,
                },
            ))
            .unwrap();
        }
        let p = plan(
            &api,
            ReconcileRequest {
                selector: ContextSelector::default(),
                source: "topic".into(),
                target: "main".into(),
            },
        )
        .unwrap();
        let r = apply(&api, &p, &p.target_revision).unwrap();
        assert_eq!(r.conflicts, vec!["spec/design.md", "spec/lessons.md"]);
        let c = read_conflict(&api, &p.project, &r.id, "spec/design.md").unwrap();
        let root = project_dir(&api, &p.project).unwrap();
        let path = workspace(&root, &r).unwrap();
        let mut injected = false;
        let error = resolve_checked(
            &api,
            ResolveRequest {
                project: p.project.clone(),
                record_id: r.id,
                expected_target_revision: c.target_revision,
                expected_resolution_revision: c.resolution_revision,
                files: vec![
                    ResolutionFile {
                        path: "spec/design.md".into(),
                        content: Some(original),
                    },
                    ResolutionFile {
                        path: "spec/lessons.md".into(),
                        content: Some("# Lessons\n\nResolved.\n".into()),
                    },
                ],
            },
            |point| {
                if point == transaction::Checkpoint::AfterWrites && !injected {
                    injected = true;
                    assert!(
                        fs::read_to_string(path.join("spec/lessons.md"))
                            .unwrap()
                            .contains("<<<<<<<")
                    );
                    fs::write(path.join("spec/lessons.md"), b"foreign later path").unwrap();
                    storage::git_text(
                        &api.runtime,
                        &root.join("history.git"),
                        Some(&path),
                        &["add", "--", "spec/lessons.md"],
                    )
                    .unwrap();
                }
                Ok(())
            },
        )
        .unwrap_err();
        assert_eq!(error.code, "revision_changed");
        assert_eq!(
            fs::read(path.join("spec/lessons.md")).unwrap(),
            b"foreign later path"
        );
        assert_eq!(
            api.resolve_context(&p.apply_selector())
                .unwrap()
                .spec_revision
                .as_deref(),
            Some(p.target_spec_oid.as_str())
        );
    }
    #[test]
    fn spec_projects_mcp_8_reconcile_cooperative_cancel_keeps_durable_receipt() {
        use std::sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
        };
        for at in [
            transaction::Checkpoint::BeforeWrites,
            transaction::Checkpoint::AfterPublish,
        ] {
            let (_owner, api, _) = crate::transaction::tests::fixture();
            let plan = setup(&api);
            let flag = Arc::new(AtomicBool::new(false));
            let token = flag.clone();
            let controlled = api.with_control(crate::InvocationControl::new(move || {
                token.load(Ordering::SeqCst)
            }));
            let result = apply_checked(&controlled, &plan, &plan.target_revision, |point| {
                if point == at {
                    flag.store(true, Ordering::SeqCst);
                }
                Ok(())
            });
            if at == transaction::Checkpoint::AfterPublish {
                assert_eq!(result.unwrap().state, ReconcileState::Applied);
            } else {
                assert_eq!(result.unwrap_err().code, "cancelled");
            }
            let record = status(&api, &plan.project, &plan.id().unwrap()).unwrap();
            if at == transaction::Checkpoint::AfterPublish {
                assert_eq!(record.state, ReconcileState::Applied);
                assert!(record.result_spec_oid.is_some());
            }
            let result = apply(&api, &plan, &plan.target_revision).unwrap();
            assert_eq!(result.state, ReconcileState::Applied);
            assert_eq!(
                apply(&api, &plan, &plan.target_revision)
                    .unwrap()
                    .result_spec_oid,
                result.result_spec_oid
            );
        }
    }
    #[test]
    fn spec_projects_mcp_8_resolution_cancellation_keeps_partial_workspace_for_fresh_retry() {
        use std::sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
        };
        let (_owner, api, _) = crate::transaction::tests::fixture();
        setup(&api);
        let original = api
            .read_document(&ContextSelector::default(), "spec/design.md")
            .unwrap()
            .document
            .content;
        for (branch, path, content) in [
            ("main", "spec/lessons.md", "# Lessons\n\nTarget.\n".into()),
            ("main", "spec/design.md", format!("{original}\nTarget.\n")),
            ("topic", "spec/design.md", format!("{original}\nSource.\n")),
        ] {
            let selector = select(
                &api.resolve_context(&ContextSelector::default())
                    .unwrap()
                    .project_id
                    .unwrap(),
                branch,
            );
            let context = api.resolve_context(&selector).unwrap();
            api.execute(crate::SpecRequest::WriteDocument(
                crate::WriteDocumentRequest {
                    selector,
                    path: path.into(),
                    content,
                    expected_revision: context.revision_token,
                },
            ))
            .unwrap();
        }
        let plan = plan(
            &api,
            ReconcileRequest {
                selector: ContextSelector::default(),
                source: "topic".into(),
                target: "main".into(),
            },
        )
        .unwrap();
        let record = apply(&api, &plan, &plan.target_revision).unwrap();
        assert_eq!(record.conflicts.len(), 2);
        let before = read_conflict(&api, &plan.project, &record.id, "spec/design.md").unwrap();
        let request = ResolveRequest {
            project: plan.project.clone(),
            record_id: record.id.clone(),
            expected_target_revision: before.target_revision,
            expected_resolution_revision: before.resolution_revision,
            files: vec![
                ResolutionFile {
                    path: "spec/design.md".into(),
                    content: Some(format!("{original}\nResolved.\n")),
                },
                ResolutionFile {
                    path: "spec/lessons.md".into(),
                    content: Some("# Lessons\n\nResolved.\n".into()),
                },
            ],
        };
        let flag = Arc::new(AtomicBool::new(false));
        let token = flag.clone();
        let controlled = api.with_control(crate::InvocationControl::new(move || {
            token.load(Ordering::SeqCst)
        }));
        assert_eq!(
            resolve_checked(&controlled, request.clone(), |at| {
                if at == transaction::Checkpoint::AfterWrites {
                    flag.store(true, Ordering::SeqCst);
                }
                Ok(())
            })
            .unwrap_err()
            .code,
            "cancelled"
        );
        let first = read_conflict(&api, &plan.project, &record.id, "spec/design.md").unwrap();
        let second = read_conflict(&api, &plan.project, &record.id, "spec/lessons.md").unwrap();
        assert!(first.proposed.as_deref().unwrap().contains("Resolved."));
        assert!(second.proposed.as_deref().unwrap().contains("<<<<<<<"));
        assert_ne!(
            first.resolution_revision,
            request.expected_resolution_revision
        );
        assert_eq!(
            resolve(&api, request.clone()).unwrap_err().code,
            "revision_changed"
        );
        let mut fresh = request;
        fresh.expected_target_revision = first.target_revision;
        fresh.expected_resolution_revision = first.resolution_revision;
        assert_eq!(resolve(&api, fresh).unwrap().state, ReconcileState::Applied);
    }
}
