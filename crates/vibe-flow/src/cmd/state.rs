//! `vibe-flow state get|set` (mirrors `commands/state.mjs`).
//!
//! `set` is the gate D27 names. It asks [`Machine::check_transition`] before
//! anything is written — unknown target, a target outside the current state's
//! `next`, and an unconfirmed gated edge are all refusals that leave the cursor
//! and the ledger untouched — then writes the cursor and appends exactly one
//! ledger event. `idle` is legal from every state, so the abort can never be
//! gated shut; a corrupt cursor still degrades to `idle` and still accepts it.

use serde_json::Value;

use crate::cmd::Outcome;
use crate::cursor::{read_cursor, write_cursor};
use crate::ledger;
use crate::machine::{IDLE, Machine, split_target};
use crate::paths::FlowPaths;
use crate::payload;

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

/// `state set <target> [feature] [--confirm]`: checks the move, carries the
/// feature forward, writes the cursor, appends one ledger event and echoes the
/// new state's legal `next`.
pub fn run_set(
    paths: &FlowPaths,
    target: Option<&str>,
    feature_arg: Option<&str>,
    confirm: bool,
) -> Outcome {
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
    // The gate is asked before anything is written: a refusal must leave the
    // cursor and the ledger exactly as they were.
    let current = current_state(paths);
    if let Err(refusal) = machine.check_transition(&current, target, confirm) {
        return Outcome::new(1, String::new(), refusal.message());
    }
    let (flow, phase) = split_target(target);
    let feature = resolve_feature(paths, &flow, feature_arg);
    let mut stderr = String::new();
    if flow == "feature" && feature.is_null() {
        stderr.push_str(&line(&format!(
            "vibe state: WARN: entering '{target}' with no feature set. Pass one: vibe state set {target} <feature>"
        )));
    }
    // D28's append side: exactly one event per accepted write, and none for a
    // refusal. A gated edge records the approval it crossed on.
    let gated = machine.gate(&current, target).then_some(confirm);
    // Pre-flight the ledger before the cursor moves. The cursor write is the
    // commit; an append refused afterwards would leave it advanced with no
    // event saying how it got there. Rotation is the refusal most likely to
    // happen, so it happens here, before anything is committed.
    if let Err(err) = ledger::prepare(paths) {
        return Outcome::new(
            1,
            String::new(),
            line(&format!(
                "vibe state: ERROR: the ledger cannot record this write, so the cursor did not move: {err}"
            )),
        );
    }
    // The payload is the second record of the same write (FV-R7). It is
    // pre-flighted here for the same reason: a payload append refused after the
    // cursor moved would bump nothing but leave the report stale, and a linked
    // leaf must be refused before anything is committed.
    if let Err(err) = payload::prepare(paths) {
        return Outcome::new(
            1,
            String::new(),
            line(&format!(
                "vibe state: ERROR: the payload cannot record this write, so the cursor did not move: {err}"
            )),
        );
    }
    if let Err(err) = write_cursor(paths, &flow, &phase, &feature) {
        return Outcome::new(
            1,
            String::new(),
            line(&format!("vibe state: ERROR: failed to write cursor: {err}")),
        );
    }
    if let Err(err) = ledger::append_transition(paths, &current, target, &feature, gated) {
        return Outcome::new(
            1,
            String::new(),
            line(&format!(
                "vibe state: ERROR: the cursor moved to '{target}' but the ledger did not record it: {err}"
            )),
        );
    }
    // Advisory (FV-R7): the cursor and the ledger already carry the write, so a
    // payload that cannot be written warns and the transition still stands.
    let mut stderr_tail = String::new();
    if let Err(err) = payload::write_transition(paths) {
        stderr_tail.push_str(&line(&format!(
            "vibe state: WARN — the cursor moved but the payload did not: {err}"
        )));
    }
    let mut stdout = line(&format!("-> {target}"));
    let next = machine.next(target);
    if !next.is_empty() {
        stdout.push_str(&line(&format!("   next: {}", next.join(", "))));
    }
    Outcome::new(0, stdout, stderr + &stderr_tail)
}

/// The state the cursor names. A corrupt cursor degrades to `idle`: the gate
/// must not brick the one path that repairs it.
fn current_state(paths: &FlowPaths) -> String {
    read_cursor(paths)
        .map(|cursor| cursor.state)
        .unwrap_or_else(|_| IDLE.to_string())
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
        let out = run_set(&paths, None, None, false);
        assert_eq!(out.code, 1);
        assert_eq!(
            out.stderr,
            "vibe state: ERROR: no target state given. Usage: vibe state set <flow.phase> [feature]\n"
        );
    }

    #[test]
    fn set_refuses_an_unknown_target_listing_the_sorted_states() {
        let (_, paths) = root_with_machine("set-unknown");
        let out = run_set(&paths, Some("nope"), None, false);
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
        assert_eq!(
            run_set(&paths, Some("feature.design"), Some("auth"), false).code,
            0
        );
        let out = run_set(&paths, Some("feature.plan"), Some("auth"), false);
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

    // Rewritten for F7: the old test drove `idle -> feature.impl`, which is
    // neither a legal next (idle's next is setup/strategy/feature.design/
    // quick.triage) nor an ungated edge. It now walks the legal path
    // design -> plan and still asserts the no-feature warning.
    #[test]
    fn set_warns_when_a_feature_state_has_no_feature() {
        let (_, paths) = root_with_machine("set-warn");
        assert_eq!(run_set(&paths, Some("feature.design"), None, false).code, 0);
        let out = run_set(&paths, Some("feature.plan"), None, false);
        assert_eq!(out.code, 0);
        assert_eq!(
            out.stderr,
            "vibe state: WARN: entering 'feature.plan' with no feature set. Pass one: vibe state set feature.plan <feature>\n"
        );
    }

    #[test]
    fn set_carries_the_feature_forward_and_idle_clears_it() {
        let (_, paths) = root_with_machine("set-carry");
        assert_eq!(
            run_set(&paths, Some("feature.design"), Some("auth"), false).code,
            0
        );
        assert_eq!(run_set(&paths, Some("feature.plan"), None, false).code, 0);
        assert!(run_get(&paths).stdout.contains("\"feature\": \"auth\""));
        // A gated hop still carries it when the human approved.
        let out = run_set(&paths, Some("feature.impl"), None, true);
        assert_eq!(out.code, 0);
        assert!(out.stderr.is_empty(), "{}", out.stderr);
        assert!(run_get(&paths).stdout.contains("\"feature\": \"auth\""));

        assert_eq!(run_set(&paths, Some("idle"), None, false).code, 0);
        assert!(run_get(&paths).stdout.contains("\"feature\": null"));
    }

    // Rewritten for F7: the old test recovered a corrupt cursor by writing
    // `feature.impl` directly, which the gate now refuses. Recovery is
    // `state set idle` — covered here — and the no-feature warning is still
    // reached by walking forward from idle on a legal edge.
    #[test]
    fn set_recovers_a_corrupt_cursor_by_writing_a_fresh_one() {
        let (root, paths) = root_with_machine("set-recover");
        let dir = root.join(".vibe").join("run").join("flow");
        fs::create_dir_all(&dir).expect("cursor dir");
        fs::write(dir.join("state.json"), "not json at all").expect("write");
        let out = run_set(&paths, Some("idle"), None, false);
        assert_eq!(out.code, 0);
        assert!(out.stderr.is_empty(), "{}", out.stderr);
        assert_eq!(run_get(&paths).code, 0);

        let out = run_set(&paths, Some("feature.design"), None, false);
        assert_eq!(out.code, 0);
        assert!(out.stderr.contains("with no feature set"), "{}", out.stderr);
    }

    #[test]
    fn set_refuses_a_target_outside_next_and_writes_nothing() {
        let (root, paths) = root_with_machine("set-illegal");
        let out = run_set(&paths, Some("feature.impl"), Some("auth"), false);
        assert_eq!(out.code, 1);
        assert!(out.stdout.is_empty());
        assert_eq!(
            out.stderr,
            "vibe state: ERROR: 'feature.impl' is not in next of 'idle'.\n\
             vibe state: Legal next: setup.detect, strategy.brainstorm, feature.design, quick.triage\n"
        );
        assert!(
            !root.join(".vibe/run/flow/state.json").exists(),
            "wrote a cursor"
        );
        assert!(!paths.ledger_path().exists(), "wrote a ledger line");
    }

    #[cfg(unix)]
    #[test]
    fn a_refused_ledger_leaves_the_cursor_where_it_was() {
        let (_, paths) = root_with_machine("set-ledger-refused");
        assert_eq!(
            run_set(&paths, Some("feature.design"), Some("auth"), false).code,
            0
        );
        let before = fs::read_to_string(paths.cursor_path()).expect("cursor");
        let outside = scratch("set-ledger-refused-outside");
        let keeper = outside.join("keeper.jsonl");
        fs::write(&keeper, "keep\n").expect("keeper");
        fs::remove_file(paths.ledger_path()).expect("live ledger");
        std::os::unix::fs::symlink(&keeper, paths.ledger_path()).expect("symlink");

        let out = run_set(&paths, Some("feature.plan"), None, false);
        assert_eq!(out.code, 1);
        assert!(out.stdout.is_empty());
        assert!(
            out.stderr.starts_with(
                "vibe state: ERROR: the ledger cannot record this write, so the cursor did \
                 not move: refusing to append through the link "
            ),
            "{}",
            out.stderr
        );
        assert_eq!(
            fs::read_to_string(paths.cursor_path()).expect("cursor"),
            before,
            "the cursor moved although the event could not be recorded"
        );
        assert_eq!(fs::read_to_string(&keeper).expect("keeper"), "keep\n");
    }

    #[test]
    fn set_refuses_an_unconfirmed_gated_edge_naming_the_requirement() {
        let (_, paths) = root_with_machine("set-gated");
        assert_eq!(
            run_set(&paths, Some("feature.design"), Some("auth"), false).code,
            0
        );
        assert_eq!(run_set(&paths, Some("feature.plan"), None, false).code, 0);
        let before = fs::read_to_string(paths.cursor_path()).expect("cursor");
        let out = run_set(&paths, Some("feature.impl"), None, false);
        assert_eq!(out.code, 1);
        assert!(out.stdout.is_empty());
        assert!(
            out.stderr.starts_with(
                "vibe state: ERROR: 'feature.plan' -> 'feature.impl' is a human gate: \
                 human approves plan units + picks impl mode.\n"
            ),
            "{}",
            out.stderr
        );
        assert!(out.stderr.contains("--confirm"), "{}", out.stderr);
        assert_eq!(
            fs::read_to_string(paths.cursor_path()).expect("cursor"),
            before
        );
        assert_eq!(
            fs::read_to_string(paths.ledger_path())
                .expect("ledger")
                .lines()
                .count(),
            2
        );
    }

    #[test]
    fn set_allows_a_gated_edge_with_confirm_and_records_the_approval() {
        let (_, paths) = root_with_machine("set-confirmed");
        assert_eq!(
            run_set(&paths, Some("feature.design"), Some("auth"), false).code,
            0
        );
        assert_eq!(run_set(&paths, Some("feature.plan"), None, false).code, 0);
        let out = run_set(&paths, Some("feature.impl"), None, true);
        assert_eq!(out.code, 0);
        assert!(out.stderr.is_empty(), "{}", out.stderr);
        assert!(run_get(&paths).stdout.contains("\"phase\": \"impl\""));
        let ledger = fs::read_to_string(paths.ledger_path()).expect("ledger");
        let last = ledger.lines().last().expect("a ledger line");
        assert!(
            last.contains("\"from\":\"feature.plan\",\"to\":\"feature.impl\""),
            "{last}"
        );
        assert!(last.ends_with(",\"confirm\":true}"), "{last}");
    }

    #[test]
    fn every_accepted_write_appends_exactly_one_ledger_line_and_a_refusal_appends_none() {
        let (_, paths) = root_with_machine("set-ledger");
        assert_eq!(
            run_set(&paths, Some("feature.design"), Some("auth"), false).code,
            0
        );
        assert_eq!(
            fs::read_to_string(paths.ledger_path())
                .expect("ledger")
                .lines()
                .count(),
            1
        );

        // A refusal of each kind: illegal edge, unknown target, gated edge.
        for (target, confirm) in [
            ("quick.fix", false),
            ("nope", false),
            ("feature.impl", false),
        ] {
            assert_eq!(
                run_set(&paths, Some(target), None, confirm).code,
                1,
                "{target}"
            );
        }
        assert_eq!(
            fs::read_to_string(paths.ledger_path())
                .expect("ledger")
                .lines()
                .count(),
            1
        );

        assert_eq!(run_set(&paths, Some("feature.plan"), None, false).code, 0);
        assert_eq!(
            fs::read_to_string(paths.ledger_path())
                .expect("ledger")
                .lines()
                .count(),
            2
        );
    }

    #[test]
    fn set_recovers_a_cursor_that_names_no_known_state() {
        let (root, paths) = root_with_machine("set-unknown-cursor");
        let dir = root.join(".vibe").join("run").join("flow");
        fs::create_dir_all(&dir).expect("cursor dir");
        fs::write(
            dir.join("state.json"),
            "{\"flow\":\"bogus\",\"phase\":\"state\",\"feature\":null}",
        )
        .expect("write");
        let out = run_set(&paths, Some("feature.design"), None, false);
        assert_eq!(out.code, 1);
        assert!(
            out.stderr.contains(
                "Legal next: none — the cursor names no known state; run `vibe state set idle` to recover"
            ),
            "{}",
            out.stderr
        );
        assert_eq!(run_set(&paths, Some("idle"), None, false).code, 0);
        assert!(run_get(&paths).stdout.contains("\"state\": \"idle\""));
    }

    #[test]
    fn set_reports_a_missing_machine() {
        let root = scratch("set-no-machine");
        let paths = FlowPaths::resolve(Some(&root), None, &root, &NoEnv);
        let out = run_set(&paths, Some("idle"), None, false);
        assert_eq!(out.code, 1);
        assert!(
            out.stderr
                .starts_with("vibe state: ERROR: cannot read state machine: "),
            "{}",
            out.stderr
        );
    }
}
