//! `vibe-flow state get|set` (mirrors `commands/state.mjs`).
//!
//! `set` is the writer only: it validates that the target state NAME exists in
//! the machine, exactly like `set-state.sh` — no `next` membership, no gate
//! token. Gate enforcement is the machine's job (F7).

use serde_json::Value;

use crate::cmd::Outcome;
use crate::cursor::{read_cursor, write_cursor};
use crate::machine::{Machine, split_target};
use crate::paths::FlowPaths;

fn line(text: &str) -> String {
    format!("{text}\n")
}

/// `state get`: the resolved cursor as pretty JSON, `state` last.
pub fn run_get(paths: &FlowPaths) -> Outcome {
    match read_cursor(paths) {
        Ok(cursor) => Outcome::new(0, format!("{}\n", cursor_json(&cursor)), String::new()),
        Err(err) => Outcome::new(1, String::new(), line(&format!("vibe state: ERROR: {err}"))),
    }
}

fn cursor_json(cursor: &crate::cursor::Cursor) -> String {
    let feature = serde_json::to_string(&cursor.feature).unwrap_or_else(|_| "null".to_string());
    let updated = serde_json::to_string(&cursor.updated).unwrap_or_else(|_| "null".to_string());
    format!(
        "{{\n  \"flow\": {},\n  \"phase\": {},\n  \"feature\": {},\n  \"updated\": {},\n  \"state\": {}\n}}",
        json_string(&cursor.flow),
        json_string(&cursor.phase),
        feature,
        updated,
        json_string(&cursor.state)
    )
}

fn json_string(text: &str) -> String {
    serde_json::to_string(text).unwrap_or_else(|_| "\"\"".to_string())
}

/// `state set <target> [feature]`: validates, carries the feature forward,
/// writes the cursor and echoes the new state's legal `next`.
pub fn run_set(paths: &FlowPaths, target: Option<&str>, feature_arg: Option<&str>) -> Outcome {
    let target = target.filter(|text| !text.is_empty()).unwrap_or("");
    if target.is_empty() {
        return Outcome::new(
            1,
            String::new(),
            line(
                "vibe state: ERROR: no target state given. Usage: vibe state set <flow.phase> [feature]",
            ),
        );
    }
    let machine = match Machine::load(paths.data_dir()) {
        Ok(machine) => machine,
        Err(err) => {
            return Outcome::new(
                1,
                String::new(),
                line(&format!(
                    "vibe state: ERROR: cannot read state machine: {err}"
                )),
            );
        }
    };
    if machine.state_of(target).is_none() {
        return Outcome::new(
            1,
            String::new(),
            line(&format!(
                "vibe state: ERROR: '{target}' is not a known state."
            )) + &line(&format!(
                "vibe state: Known states: {}",
                machine.state_keys().join(", ")
            )),
        );
    }
    let (flow, phase) = split_target(target);
    let feature = resolve_feature(paths, &flow, feature_arg);
    let mut stderr = String::new();
    if flow == "feature" && feature.is_null() {
        stderr.push_str(&line(&format!(
            "vibe state: WARN: entering '{target}' with no feature set. Pass one: vibe state set {target} <feature>"
        )));
    }
    if let Err(err) = write_cursor(paths, &flow, &phase, &feature) {
        return Outcome::new(
            1,
            String::new(),
            line(&format!("vibe state: ERROR: failed to write cursor: {err}")),
        );
    }
    let mut stdout = line(&format!("-> {target}"));
    let next = machine.next(target);
    if !next.is_empty() {
        stdout.push_str(&line(&format!("   next: {}", next.join(", "))));
    }
    Outcome::new(0, stdout, stderr)
}

/// A new feature argument wins; moving to idle clears it; otherwise the
/// current cursor's feature is carried. A corrupt cursor degrades to null and
/// still writes — `state set idle` is the one recovery path.
fn resolve_feature(paths: &FlowPaths, flow: &str, feature_arg: Option<&str>) -> Value {
    if let Some(new_feature) = feature_arg.filter(|text| !text.is_empty()) {
        return Value::String(new_feature.to_string());
    }
    if flow == "idle" {
        return Value::Null;
    }
    match read_cursor(paths) {
        Ok(cursor) if !cursor.feature.is_null() && cursor.feature.as_str() != Some("") => {
            cursor.feature
        }
        _ => Value::Null,
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::{run_get, run_set};
    use crate::paths::{Env, FlowPaths};
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
        let dir =
            std::env::temp_dir().join(format!("vibe-flow-state-{name}-{}-{n}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("create scratch");
        dir
    }

    /// A root carrying the shipped machine as its data home.
    fn root_with_machine(name: &str) -> (PathBuf, FlowPaths) {
        let root = scratch(name);
        let data = root.join(".agents").join("flow");
        fs::create_dir_all(&data).expect("data home");
        fs::copy(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../flow/state-machine.json"),
            data.join("state-machine.json"),
        )
        .expect("copy machine");
        let paths = FlowPaths::resolve(Some(&root), None, &root, &NoEnv);
        (root, paths)
    }

    #[test]
    fn get_prints_the_idle_cursor_when_none_exists() {
        let (_, paths) = root_with_machine("get-idle");
        let out = run_get(&paths);
        assert_eq!(out.code, 0);
        assert_eq!(
            out.stdout,
            "{\n  \"flow\": \"idle\",\n  \"phase\": \"idle\",\n  \"feature\": null,\n  \"updated\": null,\n  \"state\": \"idle\"\n}\n"
        );
        assert!(out.stderr.is_empty());
    }

    #[test]
    fn get_reports_a_corrupt_cursor() {
        let (root, paths) = root_with_machine("get-corrupt");
        let dir = root.join(".vibe").join("run").join("flow");
        fs::create_dir_all(&dir).expect("cursor dir");
        fs::write(dir.join("state.json"), "{ nope").expect("write");
        let out = run_get(&paths);
        assert_eq!(out.code, 1);
        assert!(out.stdout.is_empty());
        assert!(
            out.stderr.starts_with("vibe state: ERROR: cursor at "),
            "{}",
            out.stderr
        );
    }

    #[test]
    fn set_requires_a_target() {
        let (_, paths) = root_with_machine("set-usage");
        let out = run_set(&paths, None, None);
        assert_eq!(out.code, 1);
        assert_eq!(
            out.stderr,
            "vibe state: ERROR: no target state given. Usage: vibe state set <flow.phase> [feature]\n"
        );
    }

    #[test]
    fn set_refuses_an_unknown_target_listing_the_sorted_states() {
        let (_, paths) = root_with_machine("set-unknown");
        let out = run_set(&paths, Some("nope"), None);
        assert_eq!(out.code, 1);
        assert!(out.stdout.is_empty());
        assert!(
            out.stderr
                .starts_with("vibe state: ERROR: 'nope' is not a known state.\n"),
            "{}",
            out.stderr
        );
        assert!(
            out.stderr
                .contains("vibe state: Known states: feature.compound, feature.design"),
            "{}",
            out.stderr
        );
        assert!(out.stderr.ends_with("strategy.spec\n"), "{}", out.stderr);
    }

    #[test]
    fn set_writes_and_echoes_the_legal_next_states() {
        let (root, paths) = root_with_machine("set-ok");
        let out = run_set(&paths, Some("feature.plan"), Some("auth"));
        assert_eq!(out.code, 0);
        assert_eq!(
            out.stdout,
            "-> feature.plan\n   next: feature.impl, feature.design, idle\n"
        );
        assert!(out.stderr.is_empty());
        let cursor = fs::read_to_string(
            root.join(".vibe")
                .join("run")
                .join("flow")
                .join("state.json"),
        )
        .expect("cursor");
        assert!(cursor.contains("\"flow\": \"feature\""), "{cursor}");
        assert!(cursor.contains("\"feature\": \"auth\""), "{cursor}");
    }

    #[test]
    fn set_warns_when_a_feature_state_has_no_feature() {
        let (_, paths) = root_with_machine("set-warn");
        let out = run_set(&paths, Some("feature.impl"), None);
        assert_eq!(out.code, 0);
        assert_eq!(
            out.stderr,
            "vibe state: WARN: entering 'feature.impl' with no feature set. Pass one: vibe state set feature.impl <feature>\n"
        );
    }

    #[test]
    fn set_carries_the_feature_forward_and_idle_clears_it() {
        let (_, paths) = root_with_machine("set-carry");
        assert_eq!(run_set(&paths, Some("feature.impl"), Some("auth")).code, 0);
        let out = run_set(&paths, Some("feature.verify"), None);
        assert_eq!(out.code, 0);
        assert!(out.stderr.is_empty(), "{}", out.stderr);
        assert!(run_get(&paths).stdout.contains("\"feature\": \"auth\""));

        assert_eq!(run_set(&paths, Some("idle"), None).code, 0);
        assert!(run_get(&paths).stdout.contains("\"feature\": null"));
    }

    #[test]
    fn set_recovers_a_corrupt_cursor_by_writing_a_fresh_one() {
        let (root, paths) = root_with_machine("set-recover");
        let dir = root.join(".vibe").join("run").join("flow");
        fs::create_dir_all(&dir).expect("cursor dir");
        fs::write(dir.join("state.json"), "not json at all").expect("write");
        let out = run_set(&paths, Some("feature.impl"), None);
        assert_eq!(out.code, 0);
        assert!(out.stderr.contains("with no feature set"), "{}", out.stderr);
        assert_eq!(run_get(&paths).code, 0);
    }

    #[test]
    fn set_reports_a_missing_machine() {
        let root = scratch("set-no-machine");
        let paths = FlowPaths::resolve(Some(&root), None, &root, &NoEnv);
        let out = run_set(&paths, Some("idle"), None);
        assert_eq!(out.code, 1);
        assert!(
            out.stderr
                .starts_with("vibe state: ERROR: cannot read state machine: "),
            "{}",
            out.stderr
        );
    }
}
