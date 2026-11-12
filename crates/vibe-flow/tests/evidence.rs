#![allow(clippy::unwrap_used, clippy::expect_used)]
use serde_json::{Value, json};
use std::{
    fs,
    path::PathBuf,
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
};
static NEXT: AtomicU64 = AtomicU64::new(0);
fn root() -> PathBuf {
    let p = std::env::temp_dir().join(format!(
        "flow-candidate-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(&p).unwrap();
    assert!(
        Command::new("git")
            .args(["init", "-q"])
            .arg(&p)
            .status()
            .unwrap()
            .success()
    );
    p
}
#[test] // Task 6: full candidate includes untracked, mode, deletions and prepared docs.
fn candidate_changes_for_untracked_bytes_but_ignores_declared_runtime() {
    let p = root();
    fs::write(p.join("tracked"), "before").unwrap();
    assert!(
        Command::new("git")
            .current_dir(&p)
            .args(["add", "tracked"])
            .status()
            .unwrap()
            .success()
    );
    let a = vibe_flow::candidate::snapshot(&p).unwrap();
    fs::write(p.join("new-file"), "new").unwrap();
    let b = vibe_flow::candidate::snapshot(&p).unwrap();
    assert_ne!(a, b);
    fs::create_dir_all(p.join(".vibe/run/flow")).unwrap();
    fs::write(p.join(".vibe/run/flow/receipt"), "claim").unwrap();
    assert_eq!(b, vibe_flow::candidate::snapshot(&p).unwrap());
    fs::write(p.join("new-file"), "changed").unwrap();
    assert_ne!(b, vibe_flow::candidate::snapshot(&p).unwrap());
    fs::remove_file(p.join("tracked")).unwrap();
    assert_ne!(b, vibe_flow::candidate::snapshot(&p).unwrap());
}
fn record() -> Value {
    json!({"v":1,"evidence_id":"ev-1","run_id":"run-1","task_ids":["task-6"],"candidate_digest":"candidate-a","source":"agent","checks":[{"id":"tests","argv":["cargo","test"],"cwd":"/repo","exit_code":0,"output_digest":"output-a","output_path":"output.txt","started_at":"2026-11-11T10:00:00Z","finished_at":"2026-11-11T10:01:00Z"}],"review":{"independent":true,"verdict":"passed","candidate_digest":"candidate-a"},"verdict":"passed"})
}
#[test] // Task 6: fresh timestamps cannot hide failed/missing/stale observations.
fn evidence_checks_run_candidate_commands_exit_status_and_review() {
    let good = record();
    assert!(
        vibe_flow::evidence::validate_record(&good, "run-1", "candidate-a", &["tests"], true)
            .is_ok()
    );
    for mutation in [
        json!({"verdict":"gaps_found"}),
        json!({"candidate_digest":"old"}),
        json!({"checks":[]}),
        json!({"run_id":"other"}),
        json!({"review":{"independent":false,"verdict":"passed","candidate_digest":"candidate-a"}}),
    ] {
        let mut bad = good.clone();
        for (k, v) in mutation.as_object().unwrap() {
            bad[k] = v.clone();
        }
        assert!(
            vibe_flow::evidence::validate_record(&bad, "run-1", "candidate-a", &["tests"], true)
                .is_err(),
            "{bad}"
        );
    }
    for (field, value) in [
        ("exit_code", json!(1)),
        ("argv", json!([])),
        ("output_digest", json!("")),
        ("finished_at", json!("2025-01-01T00:00:00Z")),
    ] {
        let mut bad = good.clone();
        bad["checks"][0][field] = value;
        assert!(
            vibe_flow::evidence::validate_record(&bad, "run-1", "candidate-a", &["tests"], true)
                .is_err(),
            "{field}"
        );
    }
    assert!(
        vibe_flow::evidence::production_readiness(
            &json!({"evidence_refs":["ev-1"]}),
            Some("candidate-a")
        )
        .is_err()
    );
}
#[test] // Task 6: ignored product bytes are candidate bytes, not declared outputs.
fn candidate_includes_ignored_product_files() {
    let p = root();
    fs::write(p.join(".gitignore"), "ignored-product\n").unwrap();
    let before = vibe_flow::candidate::snapshot(&p).unwrap();
    fs::write(p.join("ignored-product"), "product").unwrap();
    assert_ne!(before, vibe_flow::candidate::snapshot(&p).unwrap());
}
