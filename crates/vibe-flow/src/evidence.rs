//! Structural/semantic evidence checks do not authenticate the observations they inspect.
use serde_json::Value;
use std::collections::BTreeSet;
fn text<'a>(v: &'a Value, key: &str) -> Result<&'a str, String> {
    v[key]
        .as_str()
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| format!("missing evidence {key}"))
}
fn timestamp(s: &str) -> bool {
    s.len() == 20
        && s.bytes().enumerate().all(|(i, b)| match i {
            4 | 7 => b == b'-',
            10 => b == b'T',
            13 | 16 => b == b':',
            19 => b == b'Z',
            _ => b.is_ascii_digit(),
        })
}
/// Check claimed observations against requirements. This never upgrades source trust.
pub fn validate_record(
    record: &Value,
    run: &str,
    candidate: &str,
    required: &[&str],
    independent: bool,
) -> Result<(), String> {
    if record["v"] != 1
        || text(record, "run_id")? != run
        || text(record, "candidate_digest")? != candidate
        || text(record, "verdict")? != "passed"
    {
        return Err("evidence version, run, candidate or verdict does not match".into());
    }
    text(record, "evidence_id")?;
    text(record, "source")?;
    let tasks = record["task_ids"].as_array().ok_or("missing task ids")?;
    if tasks.is_empty()
        || tasks
            .iter()
            .any(|t| t.as_str().is_none_or(|s| s.trim().is_empty()))
    {
        return Err("invalid evidence task ids".into());
    }
    validate_checks(&record["checks"], required)?;
    if independent
        && (record["review"]["independent"] != true
            || record["review"]["verdict"] != "passed"
            || record["review"]["candidate_digest"] != candidate)
    {
        return Err("independent candidate review unresolved".into());
    }
    Ok(())
}
/// Validate declared check outcomes and required-ID coverage without trusting their source.
pub fn validate_checks(checks: &Value, required: &[&str]) -> Result<(), String> {
    let checks = checks
        .as_array()
        .filter(|c| !c.is_empty())
        .ok_or("missing evidence checks")?;
    let mut ids = BTreeSet::new();
    for check in checks {
        let id = text(check, "id")?;
        if !ids.insert(id) {
            return Err("duplicate check id".into());
        }
        let argv = check["argv"]
            .as_array()
            .filter(|a| !a.is_empty())
            .ok_or("blank check argv")?;
        if argv.iter().any(|a| a.as_str().is_none_or(|s| s.is_empty()))
            || check["exit_code"].as_i64() != Some(0)
        {
            return Err("failed or invalid check".into());
        }
        for field in ["cwd", "output_digest", "output_path"] {
            text(check, field)?;
        }
        let start = text(check, "started_at")?;
        let end = text(check, "finished_at")?;
        if !timestamp(start) || !timestamp(end) || end < start {
            return Err("invalid check interval".into());
        }
    }
    if required.is_empty() || required.iter().any(|r| !ids.contains(r)) {
        return Err("required checks were not satisfied".into());
    }
    Ok(())
}
/// Writer and Stop share this fail-closed predicate. No trusted production collector exists.
pub fn production_readiness(_cursor: &Value, _candidate: Option<&str>) -> Result<(), String> {
    Err(
        "trusted evidence collector unavailable; agent-authored records do not prove readiness"
            .into(),
    )
}
