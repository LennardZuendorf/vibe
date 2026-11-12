//! Conservative local merge detection. Historical evidence never authorizes a guessed merge.
use crate::{
    BranchId, ContextSelector, ProjectId, RecordId, SpecApi, SpecContext, SpecError, StorageMode,
};
use crate::{reconcile, registry, transaction};
use serde_json::{Value, json};
use std::collections::BTreeSet;

fn invalid() -> SpecError {
    SpecError::new("invalid_registry", "invalid refresh evidence")
}
fn fields(v: &Value, names: &[&str]) -> Result<(), SpecError> {
    let o = v.as_object().ok_or_else(invalid)?;
    if o.len() != names.len() || names.iter().any(|k| !o.contains_key(*k)) {
        return Err(invalid());
    }
    Ok(())
}
fn text(v: &Value, k: &str) -> Result<String, SpecError> {
    v[k].as_str().map(str::to_owned).ok_or_else(invalid)
}
fn oid(v: &Value, k: &str) -> Result<String, SpecError> {
    let s = text(v, k)?;
    if !matches!(s.len(), 40 | 64)
        || !s
            .bytes()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
    {
        return Err(invalid());
    }
    Ok(s)
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SnapshotBinding {
    project: ProjectId,
    source: BranchId,
    target: BranchId,
    code_head: String,
    observed_target_code_head: String,
    spec_revision: String,
}
impl SnapshotBinding {
    pub(crate) fn value(&self) -> Value {
        json!({"v":1,"project":self.project.as_str(),"source":self.source.as_str(),"target":self.target.as_str(),"code_head":self.code_head,"observed_target_code_head":self.observed_target_code_head,"spec_revision":self.spec_revision})
    }
    pub(crate) fn parse(v: &Value) -> Result<Self, SpecError> {
        fields(
            v,
            &[
                "v",
                "project",
                "source",
                "target",
                "code_head",
                "observed_target_code_head",
                "spec_revision",
            ],
        )?;
        if v["v"] != 1 {
            return Err(invalid());
        }
        Ok(Self {
            project: text(v, "project")?.parse()?,
            source: text(v, "source")?.parse()?,
            target: text(v, "target")?.parse()?,
            code_head: oid(v, "code_head")?,
            observed_target_code_head: oid(v, "observed_target_code_head")?,
            spec_revision: oid(v, "spec_revision")?,
        })
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Observation {
    project: ProjectId,
    target: BranchId,
    code_head: String,
    records: Vec<RecordId>,
}
impl Observation {
    pub(crate) fn value(&self) -> Value {
        json!({"v":1,"project":self.project.as_str(),"target":self.target.as_str(),"code_head":self.code_head,"records":self.records.iter().map(RecordId::as_str).collect::<Vec<_>>()})
    }
    pub(crate) fn parse(v: &Value) -> Result<Self, SpecError> {
        fields(v, &["v", "project", "target", "code_head", "records"])?;
        if v["v"] != 1 {
            return Err(invalid());
        }
        let records = v["records"]
            .as_array()
            .ok_or_else(invalid)?
            .iter()
            .map(|r| r.as_str().ok_or_else(invalid)?.parse())
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            project: text(v, "project")?.parse()?,
            target: text(v, "target")?.parse()?,
            code_head: oid(v, "code_head")?,
            records,
        })
    }
}
pub(crate) fn validate_identities(
    project: &ProjectId,
    branches: &[registry::Branch],
    snapshots: &[SnapshotBinding],
    observations: &[Observation],
) -> Result<(), SpecError> {
    let ids: BTreeSet<_> = branches.iter().map(|b| &b.id).collect();
    let mut targets = BTreeSet::new();
    for s in snapshots {
        if &s.project != project || !ids.contains(&s.source) || !ids.contains(&s.target) {
            return Err(invalid());
        }
    }
    for o in observations {
        if &o.project != project || !ids.contains(&o.target) || !targets.insert(&o.target) {
            return Err(invalid());
        }
    }
    Ok(())
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RefreshSummary {
    pub applied: Vec<RecordId>,
    pub conflict: Vec<RecordId>,
    pub needs_input: Vec<RecordId>,
    pub diagnostics: Vec<Value>,
}
impl RefreshSummary {
    pub fn value(&self) -> Value {
        json!({"applied":self.applied.iter().map(RecordId::as_str).collect::<Vec<_>>(),"conflict":self.conflict.iter().map(RecordId::as_str).collect::<Vec<_>>(),"needs_input":self.needs_input.iter().map(RecordId::as_str).collect::<Vec<_>>(),"diagnostics":self.diagnostics})
    }
    fn add(&mut self, r: &reconcile::ReconcileRecord) {
        match r.state {
            reconcile::ReconcileState::Applied => self.applied.push(r.id.clone()),
            reconcile::ReconcileState::Conflict => self.conflict.push(r.id.clone()),
            _ => self.needs_input.push(r.id.clone()),
        }
    }
}
#[derive(Clone, Debug)]
pub struct RefreshReport {
    pub applied: Vec<RecordId>,
    pub conflict: Vec<RecordId>,
    pub needs_input: Vec<RecordId>,
    pub diagnostics: Vec<Value>,
    pub context: Option<SpecContext>,
}
impl RefreshReport {
    pub fn value(&self) -> Value {
        json!({"applied":self.applied.iter().map(RecordId::as_str).collect::<Vec<_>>(),"conflict":self.conflict.iter().map(RecordId::as_str).collect::<Vec<_>>(),"needs_input":self.needs_input.iter().map(RecordId::as_str).collect::<Vec<_>>(),"diagnostics":self.diagnostics,"context":self.context.as_ref().map(crate::storage::context_value)})
    }
}
/// Pure upper bound for reports reachable after a management publication.
/// Existing records retain their exact encoded IDs; each source can add one new
/// fixed-format record. Each ID is reserved in every possible classification.
pub(crate) fn reserve_management(
    registry: &registry::RegistrySnapshot,
    project: &registry::Project,
    state: &registry::ProjectState,
) -> Result<RefreshSummary, SpecError> {
    let mut ids = state
        .observations
        .iter()
        .flat_map(|o| o.records.iter().cloned())
        .collect::<BTreeSet<_>>();
    let records = registry::project_dir(registry, &project.id).join("reconciliations");
    registry::confined(&registry.root, &records)?;
    match std::fs::read_dir(&records) {
        Ok(entries) => {
            for entry in entries {
                let name = entry
                    .map_err(crate::documents::io)?
                    .file_name()
                    .into_string()
                    .map_err(|_| invalid())?;
                ids.insert(RecordId::new(name)?);
            }
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(crate::documents::io(e)),
    }
    let mut ids = ids.into_iter().collect::<Vec<_>>();
    ids.extend(
        state
            .branches
            .iter()
            .map(|_| RecordId::new(format!("reconcile-{}", "0".repeat(16))))
            .collect::<Result<Vec<_>, _>>()?,
    );
    let diagnostics = state
        .branches
        .iter()
        .map(|branch| {
            let error = crate::context::ambiguous_checkout();
            json!({"code":error.code,"message":error.message,"source":branch.id.as_str()})
        })
        .collect();
    Ok(RefreshSummary {
        applied: ids.clone(),
        conflict: ids.clone(),
        needs_input: ids,
        diagnostics,
    })
}

/// Observe the precisely selected stored branch. All effects use its project lock.
impl SpecApi {
    pub fn refresh(&self, selector: &ContextSelector) -> Result<RefreshReport, SpecError> {
        let (context, ()) = transaction::read_typed(self, selector, |_| Ok(()))?;
        let s = context.refresh.clone();
        Ok(RefreshReport {
            applied: s.applied,
            conflict: s.conflict,
            needs_input: s.needs_input,
            diagnostics: s.diagnostics,
            context: Some(context),
        })
    }
}
/// Project refresh uses its registered main checkout, never an arbitrary branch.
pub fn run(api: &SpecApi, project: &ProjectId) -> Result<RefreshReport, SpecError> {
    let r = registry::load(&api.runtime)?;
    let p = r
        .projects
        .iter()
        .find(|p| &p.id == project)
        .ok_or_else(invalid)?;
    api.refresh(&ContextSelector {
        project: Some(project.clone()),
        checkout: Some(p.main_checkout.clone()),
        branch: None,
    })
}
fn tip(
    api: &SpecApi,
    r: &registry::RegistrySnapshot,
    p: &registry::Project,
    branch: &str,
) -> Option<String> {
    let runtime = crate::git::context_runtime(&api.runtime);
    let cx = crate::api::Execution::new(&runtime);
    let mut tips = BTreeSet::new();
    for c in &p.checkouts {
        if crate::context::checkout_top(&cx, &c.path)
            .ok()
            .flatten()
            .as_ref()
            != Some(&c.path)
            || !crate::context::matching_projects(r, &cx, &c.path)
                .iter()
                .any(|x| x.id == p.id)
        {
            continue;
        }
        if let Some(b) = crate::git::git(
            &cx,
            &c.path,
            &[
                "rev-parse",
                "--verify",
                "-q",
                &format!("{branch}^{{commit}}"),
            ],
        ) && let Ok(s) = String::from_utf8(b)
        {
            tips.insert(s.trim().to_owned());
        }
    }
    if tips.len() == 1 {
        tips.into_iter().next()
    } else {
        None
    }
}
/// Compute snapshot metadata before CAS, and place both old/new bytes in its journal.
pub(crate) fn capture(
    api: &SpecApi,
    c: &SpecContext,
    planned: &str,
) -> Result<Option<Value>, SpecError> {
    let Some(id) = &c.project_id else {
        return Ok(None);
    };
    let r = registry::load(&api.runtime)?;
    let p = r
        .projects
        .iter()
        .find(|p| &p.id == id)
        .ok_or_else(invalid)?;
    let mut state = registry::load_project(&r, id)?;
    let Some(source) = state
        .branches
        .iter()
        .find(|b| Some(&b.code_ref) == c.code_ref.as_ref())
    else {
        return Ok(None);
    };
    let Some(head) = tip(api, &r, p, &source.code_ref).filter(|h| Some(h) == c.code_head.as_ref())
    else {
        return Ok(None);
    };
    let before = registry::project_state_value(&state);
    for target in &state.branches {
        if let Some(target_head) = tip(api, &r, p, &target.code_ref) {
            let s = SnapshotBinding {
                project: id.clone(),
                source: source.id.clone(),
                target: target.id.clone(),
                code_head: head.clone(),
                observed_target_code_head: target_head,
                spec_revision: planned.into(),
            };
            if !state.snapshots.contains(&s) {
                state.snapshots.push(s);
            }
        }
    }
    let after = registry::project_state_value(&state);
    let after_hash = vibe_core::markers::hash_bytes(
        &serde_json::to_vec_pretty(&after).map_err(crate::documents::io)?,
    );
    Ok((before != after).then(|| json!({"project":id.as_str(),"before_hash":state.revision,"after_hash":after_hash,"before":before,"after":after})))
}
pub(crate) fn finish_capture(
    api: &SpecApi,
    context: &SpecContext,
    metadata: &Value,
) -> Result<(), SpecError> {
    if metadata.is_null() {
        return Ok(());
    }
    fields(
        metadata,
        &["project", "before", "after", "before_hash", "after_hash"],
    )?;
    let planned_hash = vibe_core::markers::hash_bytes(
        &serde_json::to_vec_pretty(&metadata["after"]).map_err(crate::documents::io)?,
    );
    if metadata["after_hash"] != planned_hash {
        return Err(SpecError::new(
            "invalid_transaction",
            "snapshot metadata planned hash mismatch",
        ));
    }
    let id: ProjectId = text(metadata, "project")?.parse()?;
    if context.project_id.as_ref() != Some(&id) {
        return Err(invalid());
    }
    let r = registry::load(&api.runtime)?;
    let state = registry::load_project(&r, &id)?;
    let current = registry::project_state_value(&state);
    if current == metadata["after"] && metadata["after_hash"] == state.revision {
        return Ok(());
    }
    if current != metadata["before"] || metadata["before_hash"] != state.revision {
        return Err(SpecError::new(
            "recovery_conflict",
            "snapshot project metadata changed before publication",
        ));
    }
    // Reuse the strict loader before publishing journal-supplied metadata.
    validate_metadata(&id, &state, &metadata["after"])?;
    registry::write_json(
        &r.root,
        &registry::project_dir(&r, &id).join("project.json"),
        &metadata["after"],
    )
}
fn validate_metadata(
    id: &ProjectId,
    old: &registry::ProjectState,
    v: &Value,
) -> Result<(), SpecError> {
    fields(v, &["v", "branches", "snapshots", "observations"])?;
    if v["v"] != 1
        || v["branches"] != registry::project_state_value(old)["branches"]
        || v["observations"] != registry::project_state_value(old)["observations"]
    {
        return Err(invalid());
    }
    let snapshots = v["snapshots"]
        .as_array()
        .ok_or_else(invalid)?
        .iter()
        .map(SnapshotBinding::parse)
        .collect::<Result<Vec<_>, _>>()?;
    validate_identities(id, &old.branches, &snapshots, &old.observations)
}

fn source_context(
    api: &SpecApi,
    project: &ProjectId,
    source: &registry::Branch,
    report: &mut RefreshSummary,
) -> Result<Option<SpecContext>, SpecError> {
    match crate::context::resolve_locked(
        &api.runtime,
        &ContextSelector {
            project: Some(project.clone()),
            branch: Some(source.code_ref.clone()),
            checkout: None,
        },
    ) {
        Ok(c) => Ok(Some(c)),
        Err(e) if e.code == "ambiguous_checkout" => {
            if !report
                .diagnostics
                .iter()
                .any(|d| d["code"] == e.code && d["source"] == source.id.as_str())
            {
                report
                    .diagnostics
                    .push(json!({"code":e.code,"message":e.message,"source":source.id.as_str()}));
            }
            Ok(None)
        }
        Err(e) => Err(e),
    }
}
fn integrated(
    api: &SpecApi,
    lock: &transaction::Locked,
    source: &str,
    target: &str,
) -> Result<bool, SpecError> {
    let output = crate::storage::spec_git_output(
        &api.runtime,
        lock.history.as_deref().ok_or_else(invalid)?,
        None,
        &["merge-base", "--is-ancestor", source, target].map(Into::into),
        None,
    )?;
    if output.status.success() {
        Ok(true)
    } else if output.status.code() == Some(1) {
        Ok(false)
    } else {
        Err(SpecError::new(
            "spec_git_failed",
            String::from_utf8_lossy(&output.stderr),
        ))
    }
}
fn current_records(
    api: &SpecApi,
    lock: &transaction::Locked,
    ids: Vec<RecordId>,
) -> Result<Vec<reconcile::ReconcileRecord>, SpecError> {
    let target = lock.revision(api)?;
    let mut seen = BTreeSet::new();
    let mut records = Vec::new();
    for id in ids {
        if !seen.insert(id.clone()) {
            continue;
        }
        let r = reconcile::load(&lock.anchor, &id)?;
        if r.state != reconcile::ReconcileState::Applied
            && integrated(api, lock, &r.plan.source_spec_oid, &target)?
        {
            continue;
        }
        records.push(r);
    }
    Ok(records)
}

pub(crate) fn locked(
    api: &SpecApi,
    c: &SpecContext,
    lock: &transaction::Locked,
) -> Result<RefreshSummary, SpecError> {
    locked_checked(api, c, lock, |_| Ok(()))
}
pub(crate) fn locked_checked(
    api: &SpecApi,
    c: &SpecContext,
    lock: &transaction::Locked,
    mut checkpoint: impl FnMut(transaction::Checkpoint) -> Result<(), SpecError>,
) -> Result<RefreshSummary, SpecError> {
    api.check_control()?;
    let mut report = RefreshSummary::default();
    if c.mode != StorageMode::GlobalFirst || c.spec_revision.is_none() {
        return Ok(report);
    }
    let id = c.project_id.as_ref().ok_or_else(invalid)?;
    let r = registry::load(&api.runtime)?;
    let p = r
        .projects
        .iter()
        .find(|p| &p.id == id)
        .ok_or_else(invalid)?;
    let mut state = registry::load_project(&r, id)?;
    let target = state
        .branches
        .iter()
        .find(|b| Some(&b.code_ref) == c.code_ref.as_ref())
        .ok_or_else(invalid)?
        .clone();
    let path = lock.anchor.join(format!("refresh-{}.json", target.id));
    registry::confined(&lock.anchor, &path)?;
    let Some(head) = tip(api, &r, p, &target.code_ref) else {
        return Ok(report);
    };
    let previous = state
        .observations
        .iter()
        .find(|o| o.target == target.id)
        .cloned();
    let mut pending = match std::fs::read(&path) {
        Ok(bytes) => Some(serde_json::from_slice::<Value>(&bytes).map_err(|_| invalid())?),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => return Err(crate::documents::io(e)),
    };
    if let Some(j) = &pending {
        fields(j, &["v", "project", "target", "from", "to", "candidates"])?;
        if j["v"] != 1 || j["project"] != id.as_str() || j["target"] != target.id.as_str() {
            return Err(invalid());
        }
        oid(j, "from")?;
        oid(j, "to")?;
        for item in j["candidates"].as_array().ok_or_else(invalid)? {
            fields(item, &["plan", "automatic"])?;
            let plan = reconcile::ReconcilePlan::from_value(&item["plan"])?;
            if plan.project != *id
                || plan.target != target.id
                || item["automatic"].as_bool().is_none()
                || !state
                    .branches
                    .iter()
                    .any(|b| b.id == plan.source && b.code_ref == plan.source_ref)
                || plan.target_ref != target.code_ref
            {
                return Err(invalid());
            }
        }
    }
    if let (Some(j), Some(o)) = (&pending, &previous)
        && j["to"] == o.code_head
    {
        let mut proved = BTreeSet::new();
        let target_oid = lock.revision(api)?;
        let mut consumed = true;
        for candidate in j["candidates"].as_array().ok_or_else(invalid)? {
            let plan = reconcile::ReconcilePlan::from_value(&candidate["plan"])?;
            let record_id = plan.id()?;
            let record = reconcile::load(&lock.anchor, &record_id)?;
            if record.plan != plan
                || (record.state == reconcile::ReconcileState::Pending
                    && (candidate["automatic"] != false || !o.records.contains(&record_id)))
            {
                consumed = false;
                break;
            }
            if o.records.contains(&record_id) {
                if record.state == reconcile::ReconcileState::Applied
                    && !integrated(
                        api,
                        lock,
                        record.result_spec_oid.as_deref().ok_or_else(invalid)?,
                        &target_oid,
                    )?
                {
                    consumed = false;
                    break;
                }
                proved.insert(record_id);
            } else if record.state == reconcile::ReconcileState::Applied
                || !integrated(api, lock, &record.plan.source_spec_oid, &target_oid)?
            {
                consumed = false;
                break;
            }
        }
        if consumed && o.records.iter().all(|id| proved.contains(id)) {
            api.check_control()?;
            lock.verify()?;
            std::fs::remove_file(&path).map_err(crate::documents::io)?;
            pending = None;
        } else {
            report.diagnostics.push(json!({"code":"refresh_pending","message":"refresh envelope has no complete observation-linked receipt proof"}));
            return Ok(report);
        }
    }
    if let Some(o) = &previous
        && o.code_head == head
    {
        let mut outstanding = false;
        let mut old_records = o
            .records
            .iter()
            .map(|id| reconcile::load(&lock.anchor, id))
            .collect::<Result<Vec<_>, _>>()?;
        let records_dir = lock.anchor.join("reconciliations");
        registry::confined(&lock.anchor, &records_dir)?;
        if records_dir.is_dir() {
            for entry in std::fs::read_dir(&records_dir).map_err(crate::documents::io)? {
                let entry = entry.map_err(crate::documents::io)?;
                let name = entry.file_name().into_string().map_err(|_| invalid())?;
                let record_id: RecordId = name.parse()?;
                let record = reconcile::load(&lock.anchor, &record_id)?;
                if record.plan.project == *id
                    && record.plan.target == target.id
                    && record.plan.target_spec_oid
                        == c.spec_revision.as_deref().ok_or_else(invalid)?
                    && record.plan.target_code_head == c.code_head
                    && record.plan.target_checkout == c.checkout
                    && record.plan.target_revision == c.revision_token
                    && !old_records.iter().any(|r| r.id == record.id)
                    && let Some(source) = state.branches.iter().find(|b| {
                        b.id == record.plan.source && b.code_ref == record.plan.source_ref
                    })
                {
                    let Some(source_context) = source_context(api, id, source, &mut report)? else {
                        continue;
                    };
                    if source_context.spec_revision.as_ref() == Some(&record.plan.source_spec_oid)
                        && source_context.code_head == record.plan.source_code_head
                    {
                        old_records.push(record);
                    }
                }
            }
        }
        for source in state.branches.iter().filter(|b| b.id != target.id) {
            let Some(sc) = source_context(api, id, source, &mut report)? else {
                continue;
            };
            let source_oid = sc.spec_revision.as_ref().ok_or_else(invalid)?;
            let target_oid = c.spec_revision.as_ref().ok_or_else(invalid)?;
            let integrated = crate::storage::spec_git_output(
                &api.runtime,
                lock.history.as_deref().ok_or_else(invalid)?,
                None,
                &["merge-base", "--is-ancestor", source_oid, target_oid].map(Into::into),
                None,
            )?;
            if !integrated.status.success() && integrated.status.code() != Some(1) {
                return Err(SpecError::new(
                    "spec_git_failed",
                    String::from_utf8_lossy(&integrated.stderr),
                ));
            }
            if !integrated.status.success()
                && !old_records.iter().any(|r| {
                    r.plan.source == source.id
                        && &r.plan.source_spec_oid == source_oid
                        && &r.plan.target_spec_oid == target_oid
                })
            {
                outstanding = true;
            }
        }
        if !outstanding {
            for record in
                current_records(api, lock, old_records.into_iter().map(|r| r.id).collect())?
            {
                report.add(&record);
            }
            return Ok(report);
        }
    }
    let same_head = previous.as_ref().is_some_and(|o| o.code_head == head);
    let mut candidates: Vec<(reconcile::ReconcilePlan, bool)> = Vec::new();
    if let Some(j) = pending {
        fields(&j, &["v", "project", "target", "from", "to", "candidates"])?;
        if j["v"] != 1
            || j["project"] != id.as_str()
            || j["target"] != target.id.as_str()
            || j["to"] != head
            || previous.as_ref().map(|o| o.code_head.as_str()) != j["from"].as_str()
        {
            report.diagnostics.push(json!({"code":"refresh_pending","message":"retained refresh belongs to a different code observation; explicit reconciliation is required"}));
            return Ok(report);
        }
        for item in j["candidates"].as_array().ok_or_else(invalid)? {
            fields(item, &["plan", "automatic"])?;
            let plan = reconcile::ReconcilePlan::from_value(&item["plan"])?;
            if plan.project != *id || plan.target != target.id {
                return Err(invalid());
            }
            candidates.push((plan, item["automatic"].as_bool().ok_or_else(invalid)?));
        }
    } else if let Some(old) = &previous {
        let runtime = crate::git::context_runtime(&api.runtime);
        let cx = crate::api::Execution::new(&runtime);
        let parents = p
            .checkouts
            .iter()
            .find_map(|checkout| {
                crate::git::git(
                    &cx,
                    &checkout.path,
                    &["rev-list", "--parents", "-n", "1", &head],
                )
            })
            .and_then(|v| String::from_utf8(v).ok())
            .map(|v| v.split_whitespace().map(str::to_owned).collect::<Vec<_>>())
            .unwrap_or_default();
        let possible_sources: BTreeSet<_> = state
            .snapshots
            .iter()
            .filter(|s| {
                s.target == target.id
                    && s.source != target.id
                    && parents.get(2) == Some(&s.code_head)
                    && s.observed_target_code_head == old.code_head
            })
            .map(|s| &s.source)
            .collect();
        for source in state.branches.iter().filter(|b| b.id != target.id) {
            let Some(source_context) = source_context(api, id, source, &mut report)? else {
                continue;
            };
            let Some(source_spec) = source_context.spec_revision.as_ref() else {
                continue;
            };
            let target_spec = c.spec_revision.as_ref().ok_or_else(invalid)?;
            let integrated = crate::storage::spec_git_output(
                &api.runtime,
                lock.history.as_deref().ok_or_else(invalid)?,
                None,
                &["merge-base", "--is-ancestor", source_spec, target_spec].map(Into::into),
                None,
            )?;
            if integrated.status.success() {
                continue;
            }
            if integrated.status.code() != Some(1) {
                return Err(SpecError::new(
                    "spec_git_failed",
                    String::from_utf8_lossy(&integrated.stderr),
                ));
            }
            let request = reconcile::ReconcileRequest {
                selector: ContextSelector {
                    project: Some(id.clone()),
                    branch: Some(target.code_ref.clone()),
                    checkout: None,
                },
                source: source.code_ref.clone(),
                target: target.code_ref.clone(),
            };
            let plan = reconcile::plan_locked(api, &request, c, lock);
            let source_tip = tip(api, &r, p, &source.code_ref);
            let source_bindings = state
                .snapshots
                .iter()
                .filter(|s| {
                    s.source == source.id
                        && s.target == target.id
                        && parents.get(2) == Some(&s.code_head)
                        && s.observed_target_code_head == old.code_head
                })
                .collect::<Vec<_>>();
            let target_bindings = state
                .snapshots
                .iter()
                .filter(|s| {
                    s.source == target.id
                        && s.target == target.id
                        && s.code_head == old.code_head
                        && s.observed_target_code_head == old.code_head
                })
                .collect::<Vec<_>>();
            let eligible = old.code_head != head
                && parents.len() == 3
                && parents[1] == old.code_head
                && possible_sources.len() == 1
                && source_bindings.len() == 1
                && target_bindings.len() == 1
                && source_tip.as_ref() == parents.get(2)
                && source_bindings[0].spec_revision == *source_spec
                && target_bindings[0].spec_revision == *target_spec
                && plan.as_ref().is_ok_and(|p| p.base_spec_oid.is_some());
            let plan = plan.unwrap_or_else(|_| reconcile::ReconcilePlan {
                project: id.clone(),
                source: source.id.clone(),
                target: target.id.clone(),
                source_ref: source.code_ref.clone(),
                target_ref: target.code_ref.clone(),
                base_spec_oid: None,
                source_spec_oid: source_spec.clone(),
                target_spec_oid: target_spec.clone(),
                source_code_head: source_context.code_head.clone(),
                target_code_head: c.code_head.clone(),
                target_checkout: c.checkout.clone(),
                target_revision: c.revision_token.clone(),
            });
            candidates.push((plan, eligible));
        }
        if !candidates.is_empty() && !same_head {
            api.check_control()?;
            checkpoint(transaction::Checkpoint::BeforeWrites)?;
            api.check_control()?;
            registry::write_json(
                &lock.anchor,
                &path,
                &json!({"v":1,"project":id.as_str(),"target":target.id.as_str(),"from":old.code_head,"to":head,"candidates":candidates.iter().map(|(p,a)|json!({"plan":p.value(),"automatic":a})).collect::<Vec<_>>()}),
            )?;
        }
    }
    let mut records = if same_head {
        previous
            .as_ref()
            .map(|o| o.records.clone())
            .unwrap_or_default()
    } else {
        Vec::new()
    };
    if api.limited() {
        let ids = candidates
            .iter()
            .map(|(p, _)| p.id())
            .collect::<Result<Vec<_>, _>>()?;
        // Reserve all three classifications before the first candidate effect.
        // Actual reports contain each candidate in only one classification.
        let reservation = RefreshSummary {
            applied: ids.clone(),
            conflict: ids.clone(),
            needs_input: ids,
            diagnostics: report.diagnostics.clone(),
        };
        let mut context = c.clone();
        context.refresh = reservation.clone();
        let report = RefreshReport {
            applied: reservation.applied,
            conflict: reservation.conflict,
            needs_input: reservation.needs_input,
            diagnostics: reservation.diagnostics,
            context: Some(context.clone()),
        };
        api.admit_result(
            &json!({"context":crate::storage::context_value(&context),"data":report.value()}),
        )?;
    }
    let mut failed = false;
    for (plan, automatic) in &candidates {
        api.check_control()?;
        let outcome = if *automatic {
            // The receipt path precedes current-code checks, so a committed publication remains observable.
            reconcile::apply_locked(api, plan, &plan.target_revision, lock, |at| {
                checkpoint(at)?;
                if matches!(
                    at,
                    transaction::Checkpoint::BeforeWrites | transaction::Checkpoint::BeforePublish
                ) && (tip(api, &r, p, &target.code_ref).as_ref() != Some(&head)
                    || tip(api, &r, p, &plan.source_ref) != tip(api, &r, p, &format!("{head}^2")))
                {
                    return Err(SpecError::new(
                        "revision_changed",
                        "code refs changed before refresh",
                    ));
                }
                Ok(())
            })
        } else {
            reconcile::needs_input_locked(api, lock, plan, "automatic_evidence_unavailable")
        };
        match outcome {
            Ok(record) => {
                records.push(record.id.clone());
            }
            Err(e) => {
                if e.code == "cancelled" {
                    return Err(e);
                }
                failed = true;
                report
                    .diagnostics
                    .push(json!({"code":e.code,"message":e.message,"source":plan.source.as_str()}));
            }
        }
    }
    let final_records = current_records(api, lock, records)?;
    let records = final_records.iter().map(|r| r.id.clone()).collect();
    for record in final_records {
        report.add(&record);
    }
    if !failed && !same_head && report.diagnostics.is_empty() {
        api.check_control()?;
        checkpoint(transaction::Checkpoint::BeforePublish)?;
        api.check_control()?;
        state = registry::load_project(&r, id)?;
        state.observations.retain(|o| o.target != target.id);
        state.observations.push(Observation {
            project: id.clone(),
            target: target.id,
            code_head: head,
            records,
        });
        lock.verify()?;
        if tip(api, &r, p, &target.code_ref).as_ref()
            != Some(&state.observations.last().ok_or_else(invalid)?.code_head)
        {
            return Err(SpecError::new(
                "revision_changed",
                "code ref changed before observation publication",
            ));
        }
        registry::write_project(&r, id, &state)?;
        let _completion = api.completing();
        checkpoint(transaction::Checkpoint::AfterPublish)?;
        if path.exists() {
            std::fs::remove_file(&path).map_err(crate::documents::io)?;
        }
    }
    Ok(report)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;
    use std::fs;
    fn git(api: &SpecApi, args: &[&str]) {
        let o = std::process::Command::new("git")
            .args([
                "-c",
                "user.name=test",
                "-c",
                "user.email=test@example.invalid",
                "-c",
                "commit.gpgsign=false",
                "-c",
                "core.hooksPath=/dev/null",
                "-c",
                "gc.auto=0",
            ])
            .args(args)
            .current_dir(&api.runtime.cwd)
            .env_clear()
            .envs(&api.runtime.env)
            .output()
            .unwrap();
        assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    }
    fn write(api: &SpecApi, selector: &ContextSelector, path: &str, content: &str) {
        let c = api.resolve_context(selector).unwrap();
        api.execute(crate::SpecRequest::WriteDocument(
            crate::WriteDocumentRequest {
                selector: selector.clone(),
                path: path.into(),
                content: content.into(),
                expected_revision: c.revision_token,
            },
        ))
        .unwrap();
    }
    fn snap(api: &SpecApi, selector: &ContextSelector) {
        let c = api.resolve_context(selector).unwrap();
        api.execute(crate::SpecRequest::Snapshot(crate::SnapshotRequest {
            selector: selector.clone(),
            expected_revision: c.revision_token,
        }))
        .unwrap();
    }
    fn prepared() -> (vibe_core::testscratch::Scratch, SpecApi, ContextSelector) {
        let (owner, api, c) = transaction::tests::fixture();
        let target = ContextSelector {
            project: c.project_id.clone(),
            branch: Some("main".into()),
            checkout: None,
        };
        git(&api, &["branch", "topic"]);
        let show = api
            .execute(crate::SpecRequest::ProjectShow {
                selector: target.clone(),
            })
            .unwrap();
        api.execute(crate::SpecRequest::BranchInit(crate::InitBranchRequest {
            selector: target.clone(),
            name: "topic".into(),
            from: "main".into(),
            expected_revision: show.data["revision"].as_str().unwrap().into(),
            expected_project_revision: show.data["project_revision"].as_str().unwrap().into(),
        }))
        .unwrap();
        api.refresh(&target).unwrap();
        write(
            &api,
            &target,
            "spec/lessons.md",
            "# Lessons\n\nTarget fact.\n",
        );
        snap(&api, &target);
        git(&api, &["checkout", "topic"]);
        git(&api, &["commit", "--allow-empty", "-m", "topic"]);
        let source = ContextSelector {
            branch: Some("topic".into()),
            ..target.clone()
        };
        write(&api, &source, "spec/extra.md", "# Extra\n\nSource fact.\n");
        snap(&api, &source);
        git(&api, &["checkout", "main"]);
        git(&api, &["merge", "--no-ff", "topic", "-m", "merge"]);
        (owner, api, target)
    }
    #[test]
    fn spec_projects_mcp_7_failure_keeps_observation_and_recovers_original_receipt() {
        for at in [
            transaction::Checkpoint::BeforePublish,
            transaction::Checkpoint::AfterPublish,
        ] {
            let (_owner, api, selector) = prepared();
            let c = crate::context::resolve_locked(&api.runtime, &selector).unwrap();
            let r = registry::load(&api.runtime).unwrap();
            let id = c.project_id.as_ref().unwrap();
            let before = registry::load_project(&r, id).unwrap().observations;
            let lock = transaction::tests::record_lock(&api, &c);
            let mut fired = false;
            let failed = locked_checked(&api, &c, &lock, |point| {
                if point == at && !fired {
                    fired = true;
                    return Err(SpecError::new("interrupted", "test interruption"));
                }
                Ok(())
            })
            .unwrap();
            assert!(fired);
            assert!(!failed.diagnostics.is_empty());
            assert_eq!(registry::load_project(&r, id).unwrap().observations, before);
            assert!(
                lock.anchor
                    .join(format!(
                        "refresh-{}.json",
                        registry::load_project(&r, id)
                            .unwrap()
                            .branches
                            .iter()
                            .find(|b| b.code_ref == "refs/heads/main")
                            .unwrap()
                            .id
                    ))
                    .exists()
            );
            drop(lock);
            let resumed = api.refresh(&selector).unwrap();
            assert_eq!(
                resumed.applied.len(),
                1,
                "{at:?}: {:?}",
                resumed.diagnostics
            );
            let c = resumed.context.unwrap();
            assert_eq!(
                fs::read(c.spec_root.join("extra.md")).unwrap(),
                b"# Extra\n\nSource fact.\n"
            );
            assert_eq!(api.refresh(&selector).unwrap().applied, resumed.applied);
        }
    }
    #[test]
    fn spec_projects_mcp_7_metadata_only_snapshot_requires_publication_marker() {
        for at in [
            transaction::Checkpoint::BeforePublish,
            transaction::Checkpoint::AfterPublish,
        ] {
            let (_owner, api, c) = transaction::tests::fixture();
            let result = transaction::apply_planned(
                &api,
                &ContextSelector::default(),
                &c.revision_token,
                true,
                |_| Ok(Vec::new()),
                |point| {
                    if point == at {
                        Err(SpecError::new("interrupted", "test interruption"))
                    } else {
                        Ok(())
                    }
                },
            );
            assert!(result.is_err());
            let r = registry::load(&api.runtime).unwrap();
            let id = c.project_id.as_ref().unwrap();
            assert!(registry::load_project(&r, id).unwrap().snapshots.is_empty());
            let (fresh, ()) =
                transaction::read_typed(&api, &ContextSelector::default(), |_| Ok(())).unwrap();
            assert_eq!(fresh.spec_revision, c.spec_revision);
            assert_eq!(
                registry::load_project(&r, id).unwrap().snapshots.is_empty(),
                at == transaction::Checkpoint::BeforePublish
            );
        }
    }
    #[test]
    fn spec_projects_mcp_7_snapshot_recovery_preserves_foreign_metadata_bytes() {
        let (_owner, api, c) = transaction::tests::fixture();
        let result = transaction::apply_planned(
            &api,
            &ContextSelector::default(),
            &c.revision_token,
            true,
            |_| Ok(Vec::new()),
            |at| {
                if at == transaction::Checkpoint::AfterPublish {
                    Err(SpecError::new("interrupted", "test interruption"))
                } else {
                    Ok(())
                }
            },
        );
        assert!(result.is_err());
        let r = registry::load(&api.runtime).unwrap();
        let path = registry::project_dir(&r, c.project_id.as_ref().unwrap()).join("project.json");
        let mut bytes = fs::read(&path).unwrap();
        bytes.push(b' ');
        fs::write(&path, &bytes).unwrap();
        let e = transaction::read_typed(&api, &ContextSelector::default(), |_| Ok(())).unwrap_err();
        assert_eq!(e.code, "recovery_conflict");
        assert_eq!(fs::read(path).unwrap(), bytes);
    }
    #[test]
    fn spec_projects_mcp_7_evidence_schema_rejects_one_field_mutations() {
        let good = json!({"v":1,"project":"p-one","source":"b-source","target":"b-target","code_head":"1".repeat(40),"observed_target_code_head":"2".repeat(40),"spec_revision":"3".repeat(40)});
        assert!(SnapshotBinding::parse(&good).is_ok());
        for (key, value) in [
            ("v", json!(2)),
            ("code_head", json!("short")),
            ("spec_revision", Value::Null),
            ("source", json!("../source")),
            ("extra", json!(1)),
        ] {
            let mut changed = good.clone();
            changed[key] = value;
            assert!(SnapshotBinding::parse(&changed).is_err(), "{key}");
        }
        let mut changed = good;
        changed.as_object_mut().unwrap().remove("target");
        assert!(SnapshotBinding::parse(&changed).is_err());
        let (_owner, api, c) = transaction::tests::fixture();
        let registry = registry::load(&api.runtime).unwrap();
        let id = c.project_id.as_ref().unwrap();
        let state = registry::load_project(&registry, id).unwrap();
        let binding = SnapshotBinding {
            project: "p-other".parse().unwrap(),
            source: state.branches[0].id.clone(),
            target: state.branches[0].id.clone(),
            code_head: "1".repeat(40),
            observed_target_code_head: "1".repeat(40),
            spec_revision: "2".repeat(40),
        };
        assert!(validate_identities(id, &state.branches, &[binding], &[]).is_err());
    }
    #[test]
    fn spec_projects_mcp_7_corrupt_planned_metadata_hash_cannot_publish() {
        let (_owner, api, c) = transaction::tests::fixture();
        transaction::apply_planned(
            &api,
            &ContextSelector::default(),
            &c.revision_token,
            true,
            |_| Ok(Vec::new()),
            |at| {
                if at == transaction::Checkpoint::AfterPublish {
                    Err(SpecError::new("interrupted", "test interruption"))
                } else {
                    Ok(())
                }
            },
        )
        .unwrap_err();
        let r = registry::load(&api.runtime).unwrap();
        let id = c.project_id.as_ref().unwrap();
        let state = registry::load_project(&r, id).unwrap();
        let dir = registry::project_dir(&r, id);
        let metadata = fs::read(dir.join("project.json")).unwrap();
        let journal = dir.join(format!("transaction-{}.json", state.branches[0].id));
        let mut value: Value = serde_json::from_slice(&fs::read(&journal).unwrap()).unwrap();
        value["snapshot_metadata"]["after_hash"] = json!("corrupt");
        fs::write(&journal, serde_json::to_vec_pretty(&value).unwrap()).unwrap();
        assert!(transaction::read_typed(&api, &ContextSelector::default(), |_| Ok(())).is_err());
        assert_eq!(fs::read(dir.join("project.json")).unwrap(), metadata);
        assert!(journal.exists());
    }

    #[test]
    fn spec_projects_mcp_7_review_consumed_envelope_retires_before_new_work() {
        for new_branch in [false, true] {
            let (_owner, api, selector) = prepared();
            let c = crate::context::resolve_locked(&api.runtime, &selector).unwrap();
            let lock = transaction::tests::record_lock(&api, &c);
            let mut publications = 0;
            let result = locked_checked(&api, &c, &lock, |at| {
                if at == transaction::Checkpoint::AfterPublish {
                    publications += 1;
                    if publications == 2 {
                        return Err(SpecError::new("interrupted", "after observation"));
                    }
                }
                Ok(())
            });
            assert!(result.is_err());
            let registry = registry::load(&api.runtime).unwrap();
            let id = selector.project.as_ref().unwrap();
            let state = registry::load_project(&registry, id).unwrap();
            let observation = state
                .observations
                .iter()
                .find(|o| {
                    o.target
                        == state
                            .branches
                            .iter()
                            .find(|b| b.code_ref == "refs/heads/main")
                            .unwrap()
                            .id
                })
                .unwrap();
            let original = observation.records.clone();
            let envelope = lock
                .anchor
                .join(format!("refresh-{}.json", observation.target));
            assert!(envelope.exists());
            drop(lock);
            let source = if new_branch {
                git(&api, &["branch", "new-source"]);
                let show = api
                    .execute(crate::SpecRequest::ProjectShow {
                        selector: selector.clone(),
                    })
                    .unwrap();
                api.execute(crate::SpecRequest::BranchInit(crate::InitBranchRequest {
                    selector: selector.clone(),
                    name: "new-source".into(),
                    from: "main".into(),
                    expected_revision: show.data["revision"].as_str().unwrap().into(),
                    expected_project_revision: show.data["project_revision"]
                        .as_str()
                        .unwrap()
                        .into(),
                }))
                .unwrap();
                "new-source"
            } else {
                "topic"
            };
            write(
                &api,
                &ContextSelector {
                    branch: Some(source.into()),
                    ..selector.clone()
                },
                "spec/later.md",
                "# Later\n\nLater fact.\n",
            );
            let resumed = api.refresh(&selector).unwrap();
            assert_eq!(resumed.applied, original);
            assert_eq!(resumed.needs_input.len(), 1);
            assert!(!envelope.exists());
            let repeated = api.refresh(&selector).unwrap();
            assert_eq!(repeated.applied, resumed.applied);
            assert_eq!(repeated.needs_input, resumed.needs_input);
        }
    }
    #[test]
    fn spec_projects_mcp_7_review_consumed_nonautomatic_pending_retains_material() {
        let (_owner, api, selector) = prepared();
        let registry = registry::load(&api.runtime).unwrap();
        let id = selector.project.as_ref().unwrap();
        let mut state = registry::load_project(&registry, id).unwrap();
        state.snapshots.clear();
        registry::write_project(&registry, id, &state).unwrap();
        let plan = reconcile::plan(
            &api,
            reconcile::ReconcileRequest {
                selector: selector.clone(),
                source: "topic".into(),
                target: "main".into(),
            },
        )
        .unwrap();
        reconcile::apply_checked(&api, &plan, &plan.target_revision, |at| {
            if at == transaction::Checkpoint::BeforeWrites {
                Err(SpecError::new("interrupted", "explicit before writes"))
            } else {
                Ok(())
            }
        })
        .unwrap_err();
        let c = crate::context::resolve_locked(&api.runtime, &selector).unwrap();
        let lock = transaction::tests::record_lock(&api, &c);
        let record_id = plan.id().unwrap();
        let record_path = lock
            .anchor
            .join("reconciliations")
            .join(record_id.as_str())
            .join("record.json");
        let before = fs::read(&record_path).unwrap();
        locked_checked(&api, &c, &lock, |at| {
            if at == transaction::Checkpoint::AfterPublish {
                Err(SpecError::new(
                    "interrupted",
                    "after nonautomatic observation",
                ))
            } else {
                Ok(())
            }
        })
        .unwrap_err();
        let state = registry::load_project(&registry, id).unwrap();
        let observation = state
            .observations
            .iter()
            .find(|o| o.target == plan.target)
            .unwrap();
        assert_eq!(observation.records, vec![record_id.clone()]);
        let envelope = lock.anchor.join(format!("refresh-{}.json", plan.target));
        assert!(envelope.exists());
        drop(lock);
        let resumed = api.refresh(&selector).unwrap();
        assert!(resumed.applied.is_empty());
        assert_eq!(resumed.needs_input, vec![record_id.clone()]);
        assert!(!envelope.exists());
        assert_eq!(fs::read(&record_path).unwrap(), before);
        assert_eq!(
            reconcile::status(&api, id, &record_id).unwrap().state,
            reconcile::ReconcileState::Pending
        );
        assert_eq!(
            api.refresh(&selector).unwrap().needs_input,
            resumed.needs_input
        );
    }
    #[test]
    fn spec_projects_mcp_8_refresh_cancellation_is_local_to_each_publication() {
        use std::sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
        };
        for (point, nth) in [
            (transaction::Checkpoint::BeforePublish, 1),
            (transaction::Checkpoint::AfterPublish, 1),
            (transaction::Checkpoint::AfterPublish, 2),
        ] {
            let (_owner, api, selector) = prepared();
            let context = crate::context::resolve_locked(&api.runtime, &selector).unwrap();
            let lock = transaction::tests::record_lock(&api, &context);
            let cancelled = Arc::new(AtomicBool::new(false));
            let token = cancelled.clone();
            let controlled = api.with_control(crate::InvocationControl::new(move || {
                token.load(Ordering::SeqCst)
            }));
            let mut seen = 0;
            let outcome = locked_checked(&controlled, &context, &lock, |at| {
                if at == point {
                    seen += 1;
                    if seen == nth {
                        cancelled.store(true, Ordering::SeqCst);
                    }
                }
                Ok(())
            });
            assert!(cancelled.load(Ordering::SeqCst));
            if nth == 2 {
                assert!(outcome.is_ok());
                assert!(
                    !lock
                        .anchor
                        .join(format!(
                            "refresh-{}.json",
                            registry::load_project(
                                &registry::load(&api.runtime).unwrap(),
                                context.project_id.as_ref().unwrap()
                            )
                            .unwrap()
                            .branches
                            .iter()
                            .find(|b| Some(&b.code_ref) == context.code_ref.as_ref())
                            .unwrap()
                            .id
                        ))
                        .exists()
                );
            } else {
                assert_eq!(outcome.unwrap_err().code, "cancelled");
            }
            drop(lock);
            let resumed = api.refresh(&selector).unwrap();
            assert_eq!(resumed.applied.len(), 1);
            let id = resumed.applied[0].clone();
            let record =
                reconcile::status(&api, context.project_id.as_ref().unwrap(), &id).unwrap();
            assert_eq!(record.state, reconcile::ReconcileState::Applied);
            assert_eq!(api.refresh(&selector).unwrap().applied, vec![id]);
        }
    }
}
