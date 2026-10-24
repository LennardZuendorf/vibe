//! Cursor I/O: the only reader and writer of `state.json` (mirrors
//! `cursor.mjs` + `set-state.sh`).
//!
//! Read: the new path first, then the legacy one, then idle. Only an *absent*
//! cursor may mean idle — a present but unparseable or non-object document is
//! an error, never a silent idle.
//!
//! Write: always the new path, atomic (0600, temp + rename), the flat four-key
//! body `{flow, phase, feature, updated}` with a trailing newline, and
//! `updated` at second precision in UTC.

use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;
use vibe_core::fs::write_atomic;

use crate::paths::FlowPaths;

/// The resolved cursor, plus its `state` key (never written back).
#[derive(Debug, Clone, PartialEq)]
pub struct Cursor {
    /// The flow half of the state key.
    pub flow: String,
    /// The phase half of the state key.
    pub phase: String,
    /// The feature pointer, carried verbatim whatever its JSON type.
    pub feature: Value,
    /// The last write timestamp, carried verbatim.
    pub updated: Value,
    /// `flow` when flow == phase, else `flow.phase`.
    pub state: String,
}

/// The state key for a flow/phase pair.
pub fn state_key(flow: &str, phase: &str) -> String {
    if flow == phase {
        flow.to_string()
    } else {
        format!("{flow}.{phase}")
    }
}

/// The idle default an absent cursor reads as.
fn idle() -> Cursor {
    Cursor {
        flow: "idle".to_string(),
        phase: "idle".to_string(),
        feature: Value::Null,
        updated: Value::Null,
        state: "idle".to_string(),
    }
}

/// Reads the cursor. `Err` names the malformed file and the reason.
pub fn read_cursor(paths: &FlowPaths) -> Result<Cursor, String> {
    let new_path = paths.cursor_path();
    match std::fs::read(&new_path) {
        Ok(bytes) => return parse(&new_path, &bytes),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
        Err(err) => return Err(malformed(&new_path, &err.to_string())),
    }
    let legacy = paths.legacy_cursor_path();
    match std::fs::read(&legacy) {
        Ok(bytes) => parse(&legacy, &bytes),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(idle()),
        Err(err) => Err(malformed(&legacy, &err.to_string())),
    }
}

fn malformed(path: &Path, reason: &str) -> String {
    format!("cursor at {} is malformed: {reason}", path.display())
}

fn parse(path: &Path, bytes: &[u8]) -> Result<Cursor, String> {
    let raw: Value =
        serde_json::from_slice(bytes).map_err(|err| malformed(path, &err.to_string()))?;
    let Some(object) = raw.as_object() else {
        return Err(malformed(path, "top-level JSON value is not an object"));
    };
    let flow = non_empty_string(object.get("flow")).unwrap_or_else(|| "idle".to_string());
    let phase = non_empty_string(object.get("phase")).unwrap_or_else(|| "idle".to_string());
    let feature = object.get("feature").cloned().unwrap_or(Value::Null);
    let updated = object.get("updated").cloned().unwrap_or(Value::Null);
    let state = state_key(&flow, &phase);
    Ok(Cursor {
        flow,
        phase,
        feature,
        updated,
        state,
    })
}

fn non_empty_string(value: Option<&Value>) -> Option<String> {
    value
        .and_then(Value::as_str)
        .filter(|text| !text.is_empty())
        .map(str::to_string)
}

/// Writes the cursor to the new path, creating its directory when needed.
pub fn write_cursor(
    paths: &FlowPaths,
    flow: &str,
    phase: &str,
    feature: &Value,
) -> Result<(), String> {
    let path = paths.cursor_path();
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)
            .map_err(|err| format!("cannot create {}: {err}", dir.display()))?;
    }
    let feature_json =
        serde_json::to_string(feature).map_err(|err| format!("cannot encode feature: {err}"))?;
    let body = format!(
        "{{\n  \"flow\": {},\n  \"phase\": {},\n  \"feature\": {},\n  \"updated\": \"{}\"\n}}\n",
        json_string(flow),
        json_string(phase),
        feature_json,
        now_utc_seconds()
    );
    write_atomic(&path, body.as_bytes()).map_err(|err| err.to_string())
}

fn json_string(text: &str) -> String {
    serde_json::to_string(text).unwrap_or_else(|_| "\"\"".to_string())
}

/// `YYYY-MM-DDTHH:MM:SSZ` from the system clock.
pub fn now_utc_seconds() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |delta| delta.as_secs());
    let days = i64::try_from(secs / 86_400).unwrap_or(0);
    let rest = secs % 86_400;
    format!(
        "{}T{:02}:{:02}:{:02}Z",
        civil(days),
        rest / 3600,
        (rest % 3600) / 60,
        rest % 60
    )
}

/// Days since 1970-01-01 as `YYYY-MM-DD` (Howard Hinnant's civil_from_days).
fn civil(days: i64) -> String {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!("{year:04}-{month:02}-{day:02}")
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::{civil, now_utc_seconds, read_cursor, state_key, write_cursor};
    use crate::paths::{Env, FlowPaths};
    use serde_json::{Value, json};
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};

    static COUNTER: AtomicU64 = AtomicU64::new(0);

    struct NoEnv;

    impl Env for NoEnv {
        fn get(&self, _key: &str) -> Option<String> {
            None
        }
    }

    fn scratch(name: &str) -> PathBuf {
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "vibe-flow-cursor-{name}-{}-{n}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("create scratch");
        dir
    }

    fn paths_for(root: &Path) -> FlowPaths {
        FlowPaths::resolve(Some(root), None, root, &NoEnv)
    }

    fn write_raw(root: &Path, body: &str) {
        let dir = root.join(".vibe").join("run").join("flow");
        fs::create_dir_all(&dir).expect("cursor dir");
        fs::write(dir.join("state.json"), body).expect("write cursor");
    }

    #[test]
    fn civil_dates_match_the_reference_table() {
        assert_eq!(civil(0), "1970-01-01");
        assert_eq!(civil(10_957), "2000-01-01");
        assert_eq!(civil(19_723), "2024-01-01");
        assert_eq!(civil(19_782), "2024-02-29");
        assert_eq!(civil(20_000), "2024-10-04");
    }

    #[test]
    fn the_timestamp_has_second_precision_and_utc_shape() {
        let stamp = now_utc_seconds();
        assert_eq!(stamp.len(), 20, "{stamp}");
        assert!(stamp.ends_with('Z'), "{stamp}");
        assert_eq!(stamp.as_bytes()[10], b'T', "{stamp}");
        assert_eq!(stamp.as_bytes()[4], b'-', "{stamp}");
    }

    #[test]
    fn state_key_collapses_a_single_token() {
        assert_eq!(state_key("idle", "idle"), "idle");
        assert_eq!(state_key("feature", "impl"), "feature.impl");
    }

    #[test]
    fn an_absent_cursor_reads_as_idle() {
        let root = scratch("absent");
        let cursor = read_cursor(&paths_for(&root)).expect("absent cursor");
        assert_eq!(cursor.flow, "idle");
        assert_eq!(cursor.phase, "idle");
        assert_eq!(cursor.feature, Value::Null);
        assert_eq!(cursor.updated, Value::Null);
        assert_eq!(cursor.state, "idle");
    }

    #[test]
    fn the_legacy_cursor_is_a_read_fallback() {
        let root = scratch("legacy");
        let legacy = root.join(".agents").join("skills").join("vibe");
        fs::create_dir_all(&legacy).expect("legacy dir");
        fs::write(
            legacy.join("state.json"),
            "{\n  \"flow\": \"quick\",\n  \"phase\": \"fix\",\n  \"feature\": \"auth\",\n  \"updated\": \"2026-01-01T00:00:00Z\"\n}\n",
        )
        .expect("legacy cursor");
        let cursor = read_cursor(&paths_for(&root)).expect("legacy cursor");
        assert_eq!(cursor.state, "quick.fix");
        assert_eq!(cursor.feature, json!("auth"));
    }

    #[test]
    fn the_new_cursor_wins_over_the_legacy_one() {
        let root = scratch("new-wins");
        let legacy = root.join(".agents").join("skills").join("vibe");
        fs::create_dir_all(&legacy).expect("legacy dir");
        fs::write(
            legacy.join("state.json"),
            "{\"flow\":\"quick\",\"phase\":\"fix\"}",
        )
        .expect("legacy cursor");
        write_raw(&root, "{\"flow\":\"feature\",\"phase\":\"impl\"}");
        let cursor = read_cursor(&paths_for(&root)).expect("new cursor");
        assert_eq!(cursor.state, "feature.impl");
    }

    #[test]
    fn every_machine_state_round_trips() {
        let machine = crate::machine::Machine::load(
            &Path::new(env!("CARGO_MANIFEST_DIR")).join("../../flow"),
        )
        .expect("shipped machine");
        let root = scratch("round-trip");
        let paths = paths_for(&root);
        for key in machine.state_keys() {
            let (flow, phase) = crate::machine::split_target(&key);
            write_cursor(&paths, &flow, &phase, &Value::Null).expect("write cursor");
            let cursor = read_cursor(&paths).expect("read cursor");
            assert_eq!(cursor.flow, flow, "{key}");
            assert_eq!(cursor.phase, phase, "{key}");
            assert_eq!(cursor.state, key, "{key}");
        }
    }

    #[test]
    fn a_null_feature_stays_null_and_a_non_string_feature_is_carried_verbatim() {
        let root = scratch("feature-shapes");
        let paths = paths_for(&root);
        write_raw(
            &root,
            "{\"flow\":\"feature\",\"phase\":\"impl\",\"feature\":null}",
        );
        assert_eq!(
            read_cursor(&paths).expect("null feature").feature,
            Value::Null
        );
        write_raw(
            &root,
            "{\"flow\":\"feature\",\"phase\":\"impl\",\"feature\":123}",
        );
        assert_eq!(
            read_cursor(&paths).expect("number feature").feature,
            json!(123)
        );
        write_raw(
            &root,
            "{\"flow\":\"feature\",\"phase\":\"impl\",\"feature\":[\"a\"]}",
        );
        assert_eq!(
            read_cursor(&paths).expect("array feature").feature,
            json!(["a"])
        );
    }

    #[test]
    fn empty_or_non_string_flow_and_phase_degrade_to_idle() {
        let root = scratch("degrade");
        let paths = paths_for(&root);
        write_raw(&root, "{\"flow\":\"\",\"phase\":5,\"feature\":\"x\"}");
        let cursor = read_cursor(&paths).expect("degraded cursor");
        assert_eq!(cursor.state, "idle");
        assert_eq!(cursor.feature, json!("x"));
    }

    #[test]
    fn a_corrupt_cursor_is_an_error_never_a_silent_idle() {
        let root = scratch("corrupt");
        let paths = paths_for(&root);
        for (name, body) in [
            ("not-json", "{ not json"),
            ("empty", ""),
            ("array", "[]"),
            ("string", "\"idle\""),
            ("number", "7"),
            ("null", "null"),
            ("bool", "false"),
        ] {
            write_raw(&root, body);
            let err = read_cursor(&paths).expect_err(name);
            assert!(err.starts_with("cursor at "), "{name}: {err}");
            assert!(err.contains("is malformed: "), "{name}: {err}");
        }
    }

    #[test]
    fn an_object_without_cursor_keys_reads_as_idle() {
        // The oracle defaults every field of a well-formed object; only a
        // malformed or non-object document is an error.
        let root = scratch("empty-object");
        write_raw(&root, "{}");
        let cursor = read_cursor(&paths_for(&root)).expect("empty object");
        assert_eq!(cursor.state, "idle");
        assert_eq!(cursor.feature, Value::Null);
    }

    #[test]
    fn the_written_body_is_the_flat_four_key_shape() {
        let root = scratch("body");
        let paths = paths_for(&root);
        write_cursor(&paths, "feature", "impl", &json!("auth")).expect("write cursor");
        let bytes = fs::read(paths.cursor_path()).expect("read back");
        let text = String::from_utf8(bytes.clone()).expect("utf8");
        assert!(text.starts_with("{\n  \"flow\": \"feature\",\n  \"phase\": \"impl\",\n  \"feature\": \"auth\",\n  \"updated\": \""), "{text}");
        assert!(text.ends_with("\"\n}\n"), "{text}");
        let entries: Vec<String> = fs::read_dir(paths.cursor_path().parent().expect("dir"))
            .expect("read dir")
            .map(|e| e.expect("entry").file_name().to_string_lossy().into_owned())
            .collect();
        assert_eq!(entries, ["state.json"]);
    }

    #[cfg(unix)]
    #[test]
    fn the_written_cursor_is_mode_0600() {
        use std::os::unix::fs::PermissionsExt;
        let root = scratch("mode");
        let paths = paths_for(&root);
        write_cursor(&paths, "idle", "idle", &Value::Null).expect("write cursor");
        let mode = fs::metadata(paths.cursor_path())
            .expect("metadata")
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(mode, 0o600);
    }

    #[cfg(unix)]
    #[test]
    fn writing_replaces_a_symlink_instead_of_following_it() {
        let root = scratch("symlink");
        let paths = paths_for(&root);
        let dir = paths.cursor_path().parent().expect("dir").to_path_buf();
        fs::create_dir_all(&dir).expect("cursor dir");
        let keeper = dir.join("keeper.json");
        fs::write(&keeper, b"keep").expect("keeper");
        std::os::unix::fs::symlink("keeper.json", paths.cursor_path()).expect("symlink");
        write_cursor(&paths, "idle", "idle", &Value::Null).expect("write cursor");
        assert!(
            !fs::symlink_metadata(paths.cursor_path())
                .expect("metadata")
                .file_type()
                .is_symlink()
        );
        assert_eq!(fs::read(&keeper).expect("keeper bytes"), b"keep");
    }

    #[test]
    fn two_writes_in_a_row_leave_a_valid_cursor() {
        let root = scratch("twice");
        let paths = paths_for(&root);
        write_cursor(&paths, "feature", "impl", &json!("a")).expect("first write");
        write_cursor(&paths, "feature", "verify", &json!("a")).expect("second write");
        let cursor = read_cursor(&paths).expect("valid cursor");
        assert_eq!(cursor.state, "feature.verify");
        let count = fs::read_dir(paths.cursor_path().parent().expect("dir"))
            .expect("read dir")
            .count();
        assert_eq!(count, 1);
    }
}
