//! Pure v2 progression. No caller-controlled flag or reference authenticates approval.
use crate::machine::Machine;
use serde_json::{Value, json};

/// Evaluate one revision-bound request. The production authority is unavailable.
pub fn evaluate(machine: &Machine, current: &Value, request: &Value) -> Result<Value, String> {
    crate::runtime::validate_state(machine, current)?;
    if request.get("v").and_then(Value::as_u64) != Some(1) {
        return Err("unsupported transition request version".into());
    }
    for field in ["feature", "approval_ref", "candidate_digest", "action_ref"] {
        if request.get(field).is_none() {
            return Err(format!("missing request field {field}"));
        }
    }
    if request["run_id"] != current["run_id"] {
        return Err("wrong run identity".into());
    }
    let revision = current["revision"].as_u64().ok_or("invalid revision")?;
    if request["expected_revision"].as_u64() != Some(revision) {
        return Err("stale expected revision".into());
    }
    let target = request["target"].as_str().ok_or("missing target")?;
    let from = crate::cursor::state_key(
        current["flow"].as_str().ok_or("missing flow")?,
        current["phase"].as_str().ok_or("missing phase")?,
    );
    if machine.state_of(target).is_none() || !machine.next(&from).iter().any(|s| s == target) {
        return Err(format!("illegal transition {from}>{target}"));
    }
    if target == "setup.apply" {
        return Err("setup mutation requires feature planning".into());
    }
    if target.starts_with("feature.")
        && request["feature"]
            .as_str()
            .is_none_or(|s| s.trim().is_empty())
    {
        return Err("feature state requires a feature".into());
    }
    if from.starts_with("feature.")
        && target.starts_with("feature.")
        && request["feature"] != current["feature"]
    {
        return Err("feature identity cannot change during a run".into());
    }
    if machine.gate(&from, target) || target == "feature.impl" || target == "feature.ship" {
        return Err("trusted approval verifier unavailable; references and confirm flags cannot authorize this gate".into());
    }
    if matches!(target, "feature.ship" | "idle")
        && matches!(from.as_str(), "feature.verify" | "quick.verify")
        && !(target == "idle" && request["action_ref"].as_str() == Some("abort"))
    {
        crate::evidence::production_readiness(current, request["candidate_digest"].as_str())?;
    }
    let (flow, phase) = crate::machine::split_target(target);
    let mut next = current.clone();
    next["flow"] = json!(flow);
    next["phase"] = json!(phase);
    next["feature"] = if target == "idle" {
        Value::Null
    } else {
        request["feature"].clone()
    };
    next["revision"] = json!(revision.checked_add(1).ok_or("revision exhausted")?);
    next["updated"] = json!(crate::cursor::now_utc_seconds());
    if matches!(target, "feature.plan" | "feature.design" | "idle") {
        next["task_state"] = json!({});
        next["evidence_refs"] = json!([]);
    }
    Ok(next)
}
