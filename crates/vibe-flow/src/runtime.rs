//! Explicit v2 adoption and revision-checked publication. Legacy APIs cannot write v2.
use crate::{cmd::Outcome, paths::FlowPaths};
use serde_json::{Value, json};
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};
use vibe_core::{
    fs::write_atomic,
    lock::{Acquired, Lock, STALE, verify_path},
};

/// Validate required versioned metadata without interpreting v1 as v2.
pub fn validate(v: &Value) -> Result<(), String> {
    if v["v"].as_u64() != Some(2) {
        return Err("v2 runtime requires explicit upgrade".into());
    }
    for field in [
        "run_id",
        "flow",
        "phase",
        "updated",
        "plan_digest",
        "scope_digest",
    ] {
        if v[field].as_str().is_none_or(|s| s.trim().is_empty()) {
            return Err(format!("invalid runtime {field}"));
        }
    }
    if !safe_id(v["run_id"].as_str().ok_or("invalid run")?) {
        return Err("invalid run id".into());
    }
    if !v["feature"].is_null() && v["feature"].as_str().is_none_or(|s| !safe_id(s)) {
        return Err("invalid runtime feature identity".into());
    }
    if v["flow"] == "feature" && v["feature"].as_str().is_none_or(|s| !safe_id(s)) {
        return Err("feature runtime requires feature identity".into());
    }
    if v["revision"].as_u64().is_none()
        || !v["task_state"].is_object()
        || !v["evidence_refs"].is_array()
        || v.get("feature").is_none()
    {
        return Err("invalid runtime metadata".into());
    }
    if !v["evidence_refs"]
        .as_array()
        .ok_or("invalid evidence refs")?
        .iter()
        .all(|x| x.as_str().is_some_and(safe_id))
    {
        return Err("invalid evidence reference".into());
    }
    for (id, task) in v["task_state"].as_object().ok_or("invalid task state")? {
        if !safe_id(id)
            || task["plan_digest"] != v["plan_digest"]
            || task["run_id"] != v["run_id"]
            || !matches!(
                task["status"].as_str(),
                Some("done" | "needs_context" | "blocked" | "failed")
            )
            || task["result_ref"].as_str().is_none_or(|s| !safe_id(s))
        {
            return Err("invalid or stale task result reference".into());
        }
    }
    Ok(())
}
fn safe_id(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 128
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
        && s != "."
        && s != ".."
}
pub(crate) fn validate_current(paths: &FlowPaths, value: &Value) -> Result<(), String> {
    validate(value)?;
    let machine = crate::machine::Machine::load(paths.data_dir())?;
    validate_state(&machine, value)
}
/// Validate current state membership against the selected machine, not only JSON shape.
pub(crate) fn validate_state(
    machine: &crate::machine::Machine,
    value: &Value,
) -> Result<(), String> {
    validate(value)?;
    let key = crate::cursor::state_key(
        value["flow"].as_str().ok_or("missing flow")?,
        value["phase"].as_str().ok_or("missing phase")?,
    );
    if machine.state_of(&key).is_none() {
        return Err("unknown current runtime state; explicit recovery required".into());
    }
    Ok(())
}
/// Read the authoritative cursor. Present corrupt current state never selects legacy.
pub fn read(paths: &FlowPaths) -> Result<Value, String> {
    let p = paths.cursor_path();
    let bytes = fs::read(&p).map_err(|e| format!("cannot read {}: {e}", p.display()))?;
    let v: Value = serde_json::from_slice(&bytes)
        .map_err(|e| format!("corrupt current cursor; explicit recovery required: {e}"))?;
    validate_current(paths, &v)?;
    Ok(v)
}
/// Persisted run records make adoption sticky even if cursor bytes are lost.
pub(crate) fn adoption_records_present(paths: &FlowPaths) -> Result<bool, String> {
    match fs::symlink_metadata(paths.cursor_path().with_file_name("runs")) {
        Ok(_) => Ok(true),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(e.to_string()),
    }
}
/// Detect upgraded metadata or corruption before a compatibility writer runs.
pub fn legacy_write_allowed(paths: &FlowPaths) -> Result<(), String> {
    if adoption_records_present(paths)? {
        return Err("adopted runtime records exist; legacy writer refused; use explicit recovery for damaged state".into());
    }
    if crate::cursor::regular_file_absent(&paths.cursor_path())? {
        return Ok(());
    }
    let bytes = fs::read(paths.cursor_path()).map_err(|e| e.to_string())?;
    // Legacy corrupt recovery remains v1 behavior only; unknown versions fail closed.
    if let Ok(v) = serde_json::from_slice::<Value>(&bytes)
        && let Some(version) = v.get("v")
    {
        return Err(format!(
            "legacy writer refuses versioned cursor {version}; use state transition"
        ));
    }
    Ok(())
}
fn confined(paths: &FlowPaths, path: &Path) -> Result<(), String> {
    crate::cursor::constrain_cursor_path(paths.root(), path)
}
fn locked(paths: &FlowPaths) -> Result<(Lock, PathBuf), String> {
    let path = paths.cursor_path().with_file_name("runtime.lock");
    confined(paths, &path)?;
    let acquired = Lock::acquire(&path, STALE).map_err(|e| e.to_string())?;
    match acquired {
        Acquired::Got(lock) => Ok((lock, path)),
        Acquired::Busy => Err("runtime busy; inspect active holder before recovery".into()),
    }
}
fn immutable(paths: &FlowPaths, path: &Path, bytes: &[u8]) -> Result<(), String> {
    confined(paths, path)?;
    fs::create_dir_all(path.parent().ok_or("missing record parent")?).map_err(|e| e.to_string())?;
    let mut f = match OpenOptions::new().write(true).create_new(true).open(path) {
        Ok(file) => file,
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
            let meta = fs::symlink_metadata(path).map_err(|e| e.to_string())?;
            if meta.is_file()
                && !meta.file_type().is_symlink()
                && fs::read(path).map_err(|e| e.to_string())? == bytes
            {
                return Ok(());
            }
            return Err(format!("conflicting immutable record {}", path.display()));
        }
        Err(e) => return Err(format!("immutable record {}: {e}", path.display())),
    };
    f.write_all(bytes)
        .and_then(|()| f.sync_all())
        .map_err(|e| e.to_string())
}
fn publish(paths: &FlowPaths, lock: &Path, current: &Value) -> Result<Value, String> {
    verify_path(lock)?;
    validate_current(paths, current)?;
    let run = current["run_id"].as_str().ok_or("run missing")?;
    let record = paths
        .cursor_path()
        .parent()
        .ok_or("runtime directory missing")?
        .join("runs")
        .join(run)
        .join(format!("revision-{}.json", current["revision"]));
    let (bytes, published) = match fs::symlink_metadata(&record) {
        Ok(meta) => {
            if !meta.is_file() || meta.file_type().is_symlink() {
                return Err("invalid pending revision record".into());
            }
            let bytes = fs::read(&record).map_err(|e| e.to_string())?;
            let pending: Value = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
            validate(&pending)?;
            let mut comparable = pending.clone();
            comparable["updated"] = current["updated"].clone();
            if comparable != *current {
                return Err("conflicting pending revision; explicit inspection required".into());
            }
            (bytes, pending)
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            let bytes = serde_json::to_vec_pretty(current).map_err(|e| e.to_string())?;
            immutable(paths, &record, &bytes)?;
            (bytes, current.clone())
        }
        Err(e) => return Err(e.to_string()),
    };
    verify_path(lock)?;
    confined(paths, &paths.cursor_path())?;
    write_atomic(&paths.cursor_path(), &bytes).map_err(|e| e.to_string())?;
    Ok(published)
}
/// Explicit adoption preserves the original cursor as an immutable record and invents no grant.
pub fn upgrade(paths: &FlowPaths, run: &str, plan: &str, scope: &str) -> Result<Value, String> {
    if !safe_id(run) || plan.trim().is_empty() || scope.trim().is_empty() {
        return Err("run, plan and scope must be nonempty".into());
    }
    let (_held, lock) = locked(paths)?;
    legacy_write_allowed(paths)?;
    let old = crate::cursor::read_cursor(paths)?;
    let mut flow = old.flow.clone();
    let mut phase = old.phase.clone();
    if flow == "feature" && !matches!(phase.as_str(), "design" | "plan") {
        phase = "plan".into();
    }
    if flow == "quick" && phase != "triage" {
        phase = "triage".into();
    }
    if flow == "setup" && phase == "apply" {
        flow = "feature".into();
        phase = "design".into();
    }
    let current = json!({"v":2,"run_id":run,"revision":0,"flow":flow,"phase":phase,"feature":old.feature,"updated":crate::cursor::now_utc_seconds(),"plan_digest":plan,"scope_digest":scope,"task_state":{},"evidence_refs":[]});
    let preserved = paths
        .cursor_path()
        .parent()
        .ok_or("runtime directory missing")?
        .join("runs")
        .join(run)
        .join("legacy.json");
    let legacy = if crate::cursor::regular_file_absent(&paths.cursor_path())? {
        json!({"flow":old.flow,"phase":old.phase,"feature":old.feature,"updated":old.updated})
    } else {
        serde_json::from_slice(&fs::read(paths.cursor_path()).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?
    };
    immutable(
        paths,
        &preserved,
        &serde_json::to_vec_pretty(&legacy).map_err(|e| e.to_string())?,
    )?;
    publish(paths, &lock, &current)
}
/// Persist one evaluated transition while holding the generic lock before reading state.
pub fn transition(paths: &FlowPaths, request: &Value) -> Result<Value, String> {
    let (_held, lock) = locked(paths)?;
    let current = read(paths)?;
    let machine = crate::machine::Machine::load(paths.data_dir())?;
    let next = crate::transition::evaluate(&machine, &current, request)?;
    let next = publish(paths, &lock, &next)?;
    Ok(json!({"v":1,"status":"applied","revision":next["revision"],"cursor":next,"reasons":[]}))
}
/// Map domain errors to a nonzero CLI result without changing state.
pub fn outcome(result: Result<Value, String>) -> Outcome {
    match result {
        Ok(v) => Outcome::new(0, format!("{v}\n"), String::new()),
        Err(e) => Outcome::new(1, String::new(), format!("vibe state: ERROR: {e}\n")),
    }
}
/// Explicit recovery starts a new unapproved idle run and retains the damaged bytes.
pub fn recover(
    paths: &FlowPaths,
    expected: &str,
    run: &str,
    plan: &str,
    scope: &str,
) -> Result<Value, String> {
    if !safe_id(run) || plan.trim().is_empty() || scope.trim().is_empty() {
        return Err("invalid recovery identity".into());
    }
    let (_held, lock) = locked(paths)?;
    let bytes = fs::read(paths.cursor_path()).map_err(|e| e.to_string())?;
    if crate::candidate::digest(paths.root(), &bytes)? != expected {
        return Err("recovery digest changed; inspect current cursor again".into());
    }
    if serde_json::from_slice::<Value>(&bytes).is_ok_and(|v| validate_current(paths, &v).is_ok()) {
        return Err("valid runtime cannot be reset by corruption recovery".into());
    }
    let original = paths
        .cursor_path()
        .parent()
        .ok_or("runtime directory missing")?
        .join("runs")
        .join(run)
        .join("recovered-cursor");
    immutable(paths, &original, &bytes)?;
    let current = json!({"v":2,"run_id":run,"revision":0,"flow":"idle","phase":"idle","feature":null,"updated":crate::cursor::now_utc_seconds(),"plan_digest":plan,"scope_digest":scope,"task_state":{},"evidence_refs":[]});
    publish(paths, &lock, &current)
}
/// Shared lock for compatibility writers; the version check happens after acquisition.
pub(crate) fn legacy_lock(paths: &FlowPaths) -> Result<(Lock, PathBuf), String> {
    let held = locked(paths)?;
    legacy_write_allowed(paths)?;
    Ok(held)
}
/// Distinguish absent/v1 from upgraded/invalid current state for hook dispatch.
pub(crate) fn hook_cursor(paths: &FlowPaths) -> Result<Option<Value>, String> {
    if crate::cursor::regular_file_absent(&paths.cursor_path())? {
        legacy_write_allowed(paths)?;
        return Ok(None);
    }
    let bytes = fs::read(paths.cursor_path()).map_err(|e| e.to_string())?;
    let value: Value =
        serde_json::from_slice(&bytes).map_err(|e| format!("corrupt current cursor: {e}"))?;
    if value.get("v").is_none() {
        legacy_write_allowed(paths)?;
        return Ok(None);
    }
    validate_current(paths, &value)?;
    Ok(Some(value))
}
/// Publish an explicitly untrusted worker result. This never grants product-write authority.
pub fn record_task(
    paths: &FlowPaths,
    expected: u64,
    brief: &Value,
    result: &Value,
) -> Result<Value, String> {
    let (_held, lock) = locked(paths)?;
    let mut current = read(paths)?;
    if current["revision"].as_u64() != Some(expected)
        || brief["run_id"] != current["run_id"]
        || brief["plan_digest"] != current["plan_digest"]
    {
        return Err("stale task brief or revision".into());
    }
    let task = brief["task_id"]
        .as_str()
        .filter(|s| safe_id(s))
        .ok_or("invalid task id")?;
    for field in [
        "requirements",
        "read_paths",
        "write_scope",
        "dependencies",
        "checks",
    ] {
        if brief[field].as_array().is_none_or(|items| {
            items
                .iter()
                .any(|v| v.as_str().is_none_or(|s| s.trim().is_empty()))
        }) {
            return Err(format!("invalid task brief {field}"));
        }
    }
    if brief["role"].as_str().is_none_or(|s| s.trim().is_empty())
        || brief["requirements"].as_array().is_none_or(Vec::is_empty)
        || brief["checks"].as_array().is_none_or(Vec::is_empty)
    {
        return Err("incomplete task brief".into());
    }
    for dependency in brief["dependencies"]
        .as_array()
        .ok_or("invalid dependencies")?
    {
        let id = dependency.as_str().ok_or("invalid dependency")?;
        if current["task_state"][id]["status"] != "done"
            || current["task_state"][id]["plan_digest"] != current["plan_digest"]
        {
            return Err("unfinished or stale task dependency".into());
        }
        let reference = current["task_state"][id]["result_ref"]
            .as_str()
            .filter(|s| safe_id(s))
            .ok_or("missing dependency result reference")?;
        let path = paths
            .cursor_path()
            .parent()
            .ok_or("runtime directory missing")?
            .join("runs")
            .join(current["run_id"].as_str().ok_or("run missing")?)
            .join(reference);
        confined(paths, &path)?;
        let meta = fs::symlink_metadata(&path).map_err(|e| e.to_string())?;
        if !meta.is_file() || meta.file_type().is_symlink() {
            return Err("invalid dependency result record".into());
        }
        let record: Value = serde_json::from_slice(&fs::read(&path).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
        if record["source"] != "agent"
            || record["brief"]["task_id"] != id
            || record["brief"]["run_id"] != current["run_id"]
            || record["brief"]["plan_digest"] != current["plan_digest"]
            || record["result"]["task_id"] != id
            || record["result"]["status"] != "done"
            || current["task_state"][id]["candidate_digest"]
                .as_str()
                .is_none_or(|s| s.trim().is_empty())
            || record["result"]["candidate_digest"] != current["task_state"][id]["candidate_digest"]
        {
            return Err("invalid or stale dependency result".into());
        }
        let required = record["brief"]["checks"]
            .as_array()
            .ok_or("missing dependency check requirements")?;
        let required: Vec<&str> = required
            .iter()
            .map(|c| {
                c.as_str()
                    .filter(|s| !s.trim().is_empty())
                    .ok_or("invalid dependency check requirement")
            })
            .collect::<Result<_, _>>()?;
        crate::evidence::validate_checks(&record["result"]["check_results"], &required)?;
    }
    if result["task_id"] != task
        || !matches!(
            result["status"].as_str(),
            Some("done" | "needs_context" | "blocked" | "failed")
        )
    {
        return Err("invalid task result identity or status".into());
    }
    for field in ["changed_paths", "check_results", "findings", "artifacts"] {
        if !result[field].is_array() {
            return Err(format!("missing result {field}"));
        }
    }
    if result["status"] == "done" {
        let required: Vec<&str> = brief["checks"]
            .as_array()
            .ok_or("invalid task checks")?
            .iter()
            .filter_map(Value::as_str)
            .collect();
        crate::evidence::validate_checks(&result["check_results"], &required)?;
    }
    for changed in result["changed_paths"]
        .as_array()
        .ok_or("missing changed paths")?
    {
        let changed = changed.as_str().ok_or("invalid changed path")?;
        let relative = Path::new(changed);
        if relative.is_absolute()
            || relative
                .components()
                .any(|c| matches!(c, std::path::Component::ParentDir))
            || !brief["write_scope"]
                .as_array()
                .ok_or("invalid scope")?
                .iter()
                .any(|s| s.as_str() == Some(changed))
        {
            return Err("task result exceeds exact write scope".into());
        }
    }
    let candidate = crate::candidate::snapshot(paths.root())?;
    if result["candidate_digest"] != candidate {
        return Err("stale task candidate".into());
    }
    let revision = expected.checked_add(1).ok_or("revision exhausted")?;
    let reference = format!("task-{task}-revision-{revision}.json");
    let path = paths
        .cursor_path()
        .parent()
        .ok_or("runtime dir missing")?
        .join("runs")
        .join(current["run_id"].as_str().ok_or("run missing")?)
        .join(&reference);
    immutable(
        paths,
        &path,
        &serde_json::to_vec_pretty(&json!({"source":"agent","brief":brief,"result":result}))
            .map_err(|e| e.to_string())?,
    )?;
    current["task_state"][task] = json!({"run_id":current["run_id"],"plan_digest":current["plan_digest"],"status":result["status"],"candidate_digest":candidate,"result_ref":reference,"source":"agent"});
    current["revision"] = json!(revision);
    current["updated"] = json!(crate::cursor::now_utc_seconds());
    publish(paths, &lock, &current)
}
/// Limited v2 hook behavior. Unknown effects deny until host confinement is demonstrated.
pub(crate) fn guard(paths: &FlowPaths, cursor: &Value, payload: &Value) -> Result<(), String> {
    let tool = payload["tool_name"].as_str().ok_or("missing tool name")?;
    if matches!(tool, "Read" | "Glob" | "Grep") {
        return Ok(());
    }
    if matches!(tool, "Write" | "Edit") {
        let raw = payload["tool_input"]["file_path"]
            .as_str()
            .ok_or("missing planning file path")?;
        let path = Path::new(raw);
        let relative = if path.is_absolute() {
            path.strip_prefix(paths.root())
                .map_err(|_| "planning path outside worktree")?
        } else {
            path
        };
        if relative
            .components()
            .any(|c| !matches!(c, std::path::Component::Normal(_)))
        {
            return Err("noncanonical planning path".into());
        }
        let feature = cursor["feature"]
            .as_str()
            .filter(|s| safe_id(s))
            .ok_or("planning requires feature identity")?;
        let name = relative
            .file_name()
            .and_then(|s| s.to_str())
            .ok_or("invalid planning path")?;
        let allowed = match cursor["phase"].as_str() {
            Some("design") => {
                relative.parent() == Some(Path::new("docs/superpowers/specs"))
                    && name.ends_with(&format!("-{feature}-design.md"))
            }
            Some("plan") => {
                relative.parent() == Some(Path::new("docs/superpowers/plans"))
                    && name.ends_with(&format!("-{feature}.md"))
            }
            _ => false,
        };
        if allowed {
            for component in relative.ancestors().filter(|p| !p.as_os_str().is_empty()) {
                match fs::symlink_metadata(paths.root().join(component)) {
                    Ok(meta) if meta.file_type().is_symlink() => {
                        return Err("planning path aliases another file through a symlink".into());
                    }
                    #[cfg(unix)]
                    Ok(meta) if component == relative && meta.is_file() => {
                        use std::os::unix::fs::MetadataExt;
                        if meta.nlink() != 1 {
                            return Err("planning file has multiple hard links".into());
                        }
                    }
                    Err(e) if e.kind() != std::io::ErrorKind::NotFound => return Err(e.to_string()),
                    _ => {}
                }
            }
            confined(paths, &paths.root().join(relative))?;
            return Ok(());
        }
    }
    Err("trusted approval and scoped action boundary unavailable; product and unknown effects denied".into())
}
#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;
    struct Empty;
    impl crate::paths::Env for Empty {
        fn get(&self, _: &str) -> Option<String> {
            None
        }
    }
    #[test] // Task 5: an abandoned owner cannot publish through its former lock.
    fn lost_lock_rejects_publication() {
        let root = std::env::temp_dir().join(format!("flow-lock-loss-{}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        let paths = FlowPaths::resolve(Some(&root), None, &root, &Empty);
        let (_held, path) = locked(&paths).unwrap();
        fs::remove_file(&path).unwrap();
        fs::write(&path, "successor").unwrap();
        let current = json!({"v":2,"run_id":"run-1","revision":0,"flow":"idle","phase":"idle","feature":null,"updated":"2026-11-11T10:00:00Z","plan_digest":"plan","scope_digest":"scope","task_state":{},"evidence_refs":[]});
        assert!(
            publish(&paths, &path, &current)
                .unwrap_err()
                .contains("ownership lost")
        );
        assert!(!paths.cursor_path().exists());
        drop(_held);
        fs::remove_dir_all(root).unwrap();
    }
}
