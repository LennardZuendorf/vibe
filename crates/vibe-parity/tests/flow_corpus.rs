#![cfg(unix)]
#![allow(clippy::unwrap_used, clippy::expect_used)]
//! The generated flow corpus is deterministic and its derived floors hold.
//! Pure filesystem checks — no oracle or candidate is
//! spawned here; the differential legs live in `tests/parity/run.sh`.
use std::fs;
use std::path::{Path, PathBuf};

use vibe_parity::corpus_flow::{MARKER, generate};
use vibe_parity::sandbox::snapshot;
use vibe_parity::suite::Suite;

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("vibe-parity-flow-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn corpus(name: &str) -> PathBuf {
    let out = scratch(name).join("flow");
    let count = generate(&repo().join("flow"), &out).unwrap();
    assert_eq!(count, 129, "generated only {count} cases");
    out
}

fn cases_of(corpus: &Path, class: &str) -> usize {
    fs::read_dir(corpus.join(class))
        .unwrap_or_else(|e| panic!("{}: {e}", corpus.join(class).display()))
        .filter(|entry| {
            entry
                .as_ref()
                .is_ok_and(|e| e.path().join("case.json").is_file())
        })
        .count()
}

#[test]
fn generated_corpus_is_deterministic() {
    let first = corpus("determinism-first");
    let second = corpus("determinism-second");
    assert_eq!(snapshot(&first).unwrap(), snapshot(&second).unwrap());
}

#[test]
fn the_generator_refuses_a_directory_it_did_not_write() {
    let out = scratch("refuse");
    fs::write(out.join("keep.txt"), b"mine").unwrap();
    let err = generate(&repo().join("flow"), &out).unwrap_err();
    assert!(err.contains(MARKER), "{err}");
    assert_eq!(fs::read(out.join("keep.txt")).unwrap(), b"mine");
}

#[test]
fn edges_case_count_equals_the_sum_of_next_arrays() {
    let corpus = corpus("edges");
    let machine: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(repo().join("flow/state-machine.json")).unwrap())
            .unwrap();
    let sum: usize = machine["states"]
        .as_object()
        .unwrap()
        .values()
        .map(|state| state["next"].as_array().map(|list| list.len()).unwrap_or(0))
        .sum();
    assert_eq!(sum, 39, "the shipped machine's next arrays sum to {sum}");
    assert_eq!(
        cases_of(&corpus, "edges"),
        sum,
        "state-set/edges must carry one case per legal edge, derived from the machine"
    );
}

#[test]
fn every_class_meets_its_floor_and_every_floor_names_a_class() {
    let corpus = corpus("floors");
    let suite = Suite::load(&repo().join("tests/parity/flow.json")).unwrap();
    let used: Vec<&str> = suite
        .commands
        .iter()
        .flat_map(|cmd| cmd.classes.iter().map(String::as_str))
        .collect();
    for (class, floor) in &suite.floors {
        assert!(
            used.contains(&class.as_str()),
            "floor for class {class}, which no command uses"
        );
        let have = cases_of(&corpus, class);
        assert!(
            have >= *floor,
            "class {class}: {have} case(s), floor {floor}"
        );
    }
    for class in used {
        assert!(
            suite.floors.contains_key(class),
            "class {class} has no floor"
        );
    }
}
