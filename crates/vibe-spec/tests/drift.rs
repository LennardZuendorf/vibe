#![cfg(unix)]
#![allow(clippy::unwrap_used, clippy::expect_used)]
//! `vibe-spec check-drift` (F18): the oracle's three streams, then the
//! superpowers artifact gate and JSON (Task 4).
mod common;

use std::path::Path;

use common::{project, spec, staged_oracle, write};

/// Exit code, stdout and stderr all equal the staged oracle's.
fn same(p: &Path) {
    let want = staged_oracle(p, "check-drift.sh", &[]);
    let got = spec(p, &["check-drift"], &[]);
    assert_eq!(
        (got.code, &got.stdout, &got.stderr),
        (want.code, &want.stdout, &want.stderr),
        "{}",
        p.display()
    );
}

#[test]
fn check_drift_matches_the_oracle_on_every_stream() {
    let cases: Vec<(&str, Vec<(&str, &str)>)> = vec![
        (
            "dr-clean",
            vec![
                (".spec/plan.md", "| auth | IN PROGRESS |\n"),
                (".spec/features/auth/plan.md", "### auth/1 — a\n"),
            ],
        ),
        (
            "dr-norow",
            vec![
                (".spec/plan.md", "# Plan\n"),
                (".spec/features/billing/tech.md", "x\n"),
            ],
        ),
        (
            "dr-done",
            vec![
                (
                    ".spec/plan.md",
                    "| auth | **DONE** | js-core DONE |\n| oauth | DONE ✅ |\n",
                ),
                (".spec/features/auth/plan.md", "x\n"),
                (".spec/features/oauth/plan.md", "x\n"),
            ],
        ),
        (
            "dr-bold",
            vec![
                (".spec/plan.md", "- **auth** — IN PROGRESS | x\n"),
                (
                    ".spec/features/auth/plan.md",
                    "- M1: NOT STARTED\n- M2: NOT STARTED",
                ),
            ],
        ),
        (
            "dr-substr",
            vec![
                (".spec/plan.md", "| oauth | DONE |\n"),
                (".spec/features/auth/tech.md", "x\n"),
            ],
        ),
        (
            "dr-counts",
            vec![
                (".spec/plan.md", "x\n"),
                ("README.md", "All 12 passing.\n3 green\n"),
                (".spec/sub/deep.md", "7 assertions\nv2 passed\n"),
                (".spec/archive/old.md", "9 passing\n"),
                (".spec/.hidden.md", "4 passed\n"),
            ],
        ),
        (
            "dr-noplan",
            vec![(".spec/features/auth/plan.md", "NOT STARTED\n")],
        ),
    ];
    for (name, files) in cases {
        let p = project(name);
        for (rel, text) in &files {
            write(&p, rel, text);
        }
        same(&p);
    }
    same(&project("dr-bare"));
}
