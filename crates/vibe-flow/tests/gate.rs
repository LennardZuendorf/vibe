#![allow(clippy::unwrap_used, clippy::expect_used)]
//! `state set` as the gate (D27): the negative matrix, one case per illegal
//! `(state, target)` pair derivable from the machine, plus the gated edges, the
//! `idle` escape hatch and the ledger's append side (D28).
//!
//! The matrix is generated, not hand-picked, and its size is asserted against
//! the machine's own numbers: a check that examines nothing must fail loudly.

mod support;

use std::fs;
use std::path::Path;

use support::{root, run, write_cursor};
use vibe_flow::machine::{IDLE, Machine, split_target};

fn shipped_machine() -> Machine {
    Machine::load(&support::manifest().join("../../flow")).expect("load the shipped machine")
}

fn cursor_for(state: &str) -> String {
    let (flow, phase) = split_target(state);
    format!("{{\"flow\":\"{flow}\",\"phase\":\"{phase}\",\"feature\":null}}\n")
}

fn ledger(root: &Path) -> Option<String> {
    fs::read_to_string(root.join(".vibe/run/flow/ledger.jsonl")).ok()
}

/// Every `(current, target)` pair the machine does **not** allow, plus the
/// counts the assertion below pins them to.
fn illegal_pairs(machine: &Machine) -> (Vec<(String, String)>, usize, usize) {
    let states = machine.state_keys();
    let edges: usize = states.iter().map(|state| machine.next(state).len()).sum();
    let mut illegal = Vec::new();
    for current in &states {
        for target in &states {
            let legal = target == IDLE || machine.next(current).iter().any(|key| key == target);
            if !legal {
                illegal.push((current.clone(), target.clone()));
            }
        }
    }
    (illegal, edges, states.len())
}

#[test]
fn every_illegal_pair_is_refused_and_writes_nothing() {
    let machine = shipped_machine();
    let (illegal, edges, states) = illegal_pairs(&machine);

    // The two data preconditions the closed form relies on: `idle` is not in
    // its own `next`, and every other state lists it. Without them the count
    // below would be measuring a shape the machine no longer has.
    assert!(!machine.next(IDLE).iter().any(|key| key == IDLE));
    for state in machine.state_keys().iter().filter(|key| *key != IDLE) {
        assert!(
            machine.next(state).iter().any(|key| key == IDLE),
            "{state} does not list idle"
        );
    }
    // legal pairs = the machine's 30 edges + idle -> idle; the rest must all be
    // here, so a shrinking matrix cannot pass silently.
    assert_eq!(illegal.len(), states * states - edges - 1);
    assert_eq!(
        illegal.len(),
        138,
        "the shipped machine has 138 illegal pairs"
    );
    println!(
        "illegal-edge cases: {} ({} states x {} states, {} legal edges + idle->idle)",
        illegal.len(),
        states,
        states,
        edges
    );

    let root = root("gate-illegal");
    let cursor = root.join(".vibe/run/flow/state.json");
    for (current, target) in &illegal {
        write_cursor(&root, &cursor_for(current));
        let before = fs::read(&cursor).expect("cursor");
        let ledger_before = ledger(&root);
        let (code, stdout, stderr) = run(&root, &["state", "set", target]);
        assert_eq!(code, 1, "{current} -> {target} must be refused");
        assert!(stdout.is_empty(), "{current} -> {target}: {stdout}");
        assert!(
            stderr.contains(&format!("'{target}' is not in next of '{current}'.")),
            "{current} -> {target}: {stderr}"
        );
        assert_eq!(
            fs::read(&cursor).expect("cursor"),
            before,
            "{current} -> {target} rewrote the cursor"
        );
        assert_eq!(
            ledger(&root),
            ledger_before,
            "{current} -> {target} appended a ledger line"
        );
    }
}

#[test]
fn idle_is_legal_from_every_state_and_records_the_hop() {
    let machine = shipped_machine();
    let root = root("gate-idle");
    let ledger_path = root.join(".vibe/run/flow/ledger.jsonl");
    let mut lines = 0;
    for state in machine.state_keys() {
        write_cursor(&root, &cursor_for(&state));
        let (code, stdout, stderr) = run(&root, &["state", "set", IDLE]);
        assert_eq!(code, 0, "{state} -> idle: {stderr}");
        assert_eq!(
            stdout,
            "-> idle\n   next: setup.detect, strategy.brainstorm, feature.design, quick.triage\n"
        );
        lines += 1;
        let text = fs::read_to_string(&ledger_path).expect("ledger");
        assert_eq!(text.lines().count(), lines, "{state}");
        let last = text.lines().last().expect("a line");
        assert!(
            last.contains(&format!("\"from\":\"{state}\",\"to\":\"{IDLE}\"")),
            "{state}: {last}"
        );
        assert!(!last.contains("confirm"), "{state}: {last}");
    }
}

#[test]
fn every_gated_edge_needs_confirm_and_names_its_requirement() {
    let machine = shipped_machine();
    let gates = machine.gates();
    assert_eq!(
        gates.len(),
        2,
        "the shipped machine has two gates: {gates:?}"
    );

    let root = root("gate-gated");
    for key in &gates {
        let (current, target) = key.split_once('>').expect("gate key shape");
        // A gate on a non-edge would be dead teeth: pin that none is.
        assert!(
            machine.next(current).iter().any(|next| next == target),
            "{key} is not a legal edge"
        );
        let requirement = machine.gate_text(current, target).expect("gate text");

        write_cursor(&root, &cursor_for(current));
        let before = fs::read(root.join(".vibe/run/flow/state.json")).expect("cursor");
        let ledger_before = ledger(&root);

        let (code, stdout, stderr) = run(&root, &["state", "set", target]);
        assert_eq!(code, 1, "{key} without --confirm");
        assert!(stdout.is_empty());
        assert!(
            stderr.starts_with(&format!(
                "vibe state: ERROR: '{current}' -> '{target}' is a human gate: {requirement}.\n"
            )),
            "{key}: {stderr}"
        );
        assert_eq!(
            fs::read(root.join(".vibe/run/flow/state.json")).expect("cursor"),
            before,
            "{key} rewrote the cursor"
        );
        assert_eq!(ledger(&root), ledger_before, "{key} appended a ledger line");

        let (code, stdout, stderr) = run(&root, &["state", "set", target, "--confirm"]);
        assert_eq!(code, 0, "{key} with --confirm: {stderr}");
        assert!(stdout.starts_with(&format!("-> {target}\n")), "{stdout}");
        let text = fs::read_to_string(root.join(".vibe/run/flow/ledger.jsonl")).expect("ledger");
        let last = text.lines().last().expect("a line");
        assert!(
            last.contains(&format!(
                "\"from\":\"{current}\",\"to\":\"{target}\",\"feature\":null,\"confirm\":true"
            )),
            "{key}: {last}"
        );
    }
}

#[test]
fn a_corrupt_cursor_still_degrades_to_idle_and_still_accepts_it() {
    let root = root("gate-corrupt");
    let dir = root.join(".vibe/run/flow");
    fs::create_dir_all(&dir).expect("cursor dir");
    for body in ["not json at all", "[1,2]", "{\"flow\":7}"] {
        fs::write(dir.join("state.json"), body).expect("write");
        let (code, stdout, stderr) = run(&root, &["state", "set", IDLE]);
        assert_eq!(code, 0, "{body}: {stderr}");
        assert_eq!(
            stdout,
            "-> idle\n   next: setup.detect, strategy.brainstorm, feature.design, quick.triage\n"
        );
        assert!(stderr.is_empty(), "{body}: {stderr}");
    }
    // And a cursor that names a state the machine does not know is not a
    // legal starting point for anything but the abort.
    fs::write(
        dir.join("state.json"),
        "{\"flow\":\"bogus\",\"phase\":\"state\",\"feature\":null}",
    )
    .expect("write");
    let (code, _, stderr) = run(&root, &["state", "set", "feature.design"]);
    assert_eq!(code, 1, "{stderr}");
    assert!(
        stderr.contains("is not in next of 'bogus.state'"),
        "{stderr}"
    );
    assert!(
        stderr.contains("run `vibe state set idle` to recover"),
        "{stderr}"
    );
    assert_eq!(run(&root, &["state", "set", IDLE]).0, 0);
}
