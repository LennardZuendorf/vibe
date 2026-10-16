#![cfg(unix)]
#![allow(clippy::unwrap_used, clippy::expect_used)]
//! `vibe-spec check-drift` (F18): the oracle's three streams, then the
//! superpowers artifact gate and JSON (Task 4).
mod common;

use std::path::Path;

use common::{assert_spec_json, project, spec, staged_oracle, write};

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
            "dr-prefix",
            vec![
                (
                    ".spec/plan.md",
                    "| spec | IN PROGRESS |\n| spec-delta | IN PROGRESS |\n",
                ),
                (".spec/features/spec/plan.md", "NOT STARTED\n"),
                (".spec/features/spec-delta/plan.md", "NOT STARTED\n"),
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

#[test]
fn a_symlinked_root_is_scanned() {
    let p = project("dr-link");
    write(&p, "real/c.md", "8 passed\n");
    write(&p, "real/plan.md", "x\n");
    std::os::unix::fs::symlink(p.join("real"), p.join(".spec")).unwrap();
    let got = spec(&p, &["check-drift"], &[]);
    assert_eq!(got.code, 1, "{}", got.stderr);
    assert!(
        got.stderr
            .contains("check-drift: ERROR: hand-written test count in .spec/c.md:1:8 passed"),
        "{}",
        got.stderr
    );
}

const NEXT: &str = " — promote its facts, then run vibe-spec promote <feature> --done";

#[test]
fn superpowers_artifacts_warn_and_fail_pre_merge() {
    let p = project("dr-sp");
    write(&p, ".spec/plan.md", "x\n");
    write(&p, "docs/superpowers/plans/2026-01-01-auth.md", "x\n");
    write(
        &p,
        "docs/superpowers/specs/deep/2026-01-01-auth-design.md",
        "x\n",
    );
    write(&p, "docs/superpowers/.DS_Store", "x");
    let plain = spec(&p, &["check-drift"], &[]);
    assert_eq!(plain.code, 0, "{}", plain.stderr);
    assert_eq!(plain.text(), "check-drift: OK — 0 errors, 2 warning(s).\n");
    assert_eq!(
        plain.stderr,
        format!(
            "check-drift: WARN: superpowers artifact remains: docs/superpowers/plans/2026-01-01-auth.md{NEXT}\n\
             check-drift: WARN: superpowers artifact remains: docs/superpowers/specs/deep/2026-01-01-auth-design.md{NEXT}\n"
        )
    );
    let gate = spec(&p, &["check-drift", "--pre-merge"], &[]);
    assert_eq!(gate.code, 1);
    assert!(gate.text().is_empty());
    assert!(
        gate.stderr
            .ends_with("check-drift: FAIL — 2 error(s), 0 warning(s).\n"),
        "{}",
        gate.stderr
    );
}

#[test]
fn json_validates_and_exits_like_text() {
    let p = project("dr-json");
    write(&p, ".spec/plan.md", "| auth | DONE |\n");
    write(&p, ".spec/features/auth/plan.md", "NOT STARTED\n");
    let out = spec(&p, &["check-drift", "--json"], &[]);
    assert_eq!(out.code, 1);
    assert!(out.stderr.is_empty(), "{}", out.stderr);
    assert_spec_json("check-drift", &out.text());
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(
        (v["errors"].as_u64(), v["warnings"].as_u64()),
        (Some(1), Some(1))
    );
    assert_eq!(v["pre_merge"], false);
    assert_eq!(
        v["features"],
        serde_json::json!([{ "name": "auth", "status": "DONE" }])
    );
    assert_eq!(v["findings"][1]["file"], ".spec/features/auth/plan.md");
    let none = project("dr-json-none");
    let miss = spec(&none, &["check-drift", "--json"], &[]);
    assert_eq!((miss.code, miss.stdout.is_empty()), (3, true));
}
