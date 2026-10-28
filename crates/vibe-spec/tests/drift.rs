#![cfg(unix)]
// vibe-spec: literal-file
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
    assert_eq!(
        gate.stderr,
        format!(
            "check-drift: ERROR: superpowers artifact remains: docs/superpowers/plans/2026-01-01-auth.md{NEXT}\n\
             check-drift: ERROR: superpowers artifact remains: docs/superpowers/specs/deep/2026-01-01-auth-design.md{NEXT}\n\
             check-drift: FAIL — 2 error(s), 0 warning(s).\n"
        )
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
    let f = v["findings"].as_array().unwrap();
    assert_eq!(f.len(), 2);
    assert_eq!(f[0]["severity"], "error");
    assert!(f[0].get("file").is_none());
    assert_eq!(f[1]["severity"], "warning");
    assert_eq!(f[1]["file"], ".spec/features/auth/plan.md");
    let none = project("dr-json-none");
    let miss = spec(&none, &["check-drift", "--json"], &[]);
    assert_eq!((miss.code, miss.stdout.is_empty()), (3, true));
}

#[test]
fn json_pre_merge_reports_artifacts_with_file() {
    let p = project("dr-json-sp");
    write(&p, ".spec/plan.md", "x\n");
    write(&p, "docs/superpowers/plans/2026-01-01-auth.md", "x\n");
    let out = spec(&p, &["check-drift", "--json", "--pre-merge"], &[]);
    assert_eq!(out.code, 1);
    assert_spec_json("check-drift", &out.text());
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["pre_merge"], true);
    let f = v["findings"].as_array().unwrap();
    assert_eq!(f.len(), 1);
    assert_eq!(f[0]["severity"], "error");
    assert_eq!(f[0]["file"], "docs/superpowers/plans/2026-01-01-auth.md");
}

fn is_root() -> bool {
    std::process::Command::new("id")
        .arg("-u")
        .output()
        .is_ok_and(|o| o.stdout == b"0\n")
}

#[test]
fn unreadable_artifact_dir_fails_closed() {
    use std::os::unix::fs::PermissionsExt;
    if is_root() {
        return; // root reads everything: the test would pass vacuously.
    }
    let p = project("dr-locked");
    write(&p, ".spec/plan.md", "x\n");
    write(&p, "docs/superpowers/locked/secret.md", "x\n");
    let locked = p.join("docs").join("superpowers").join("locked");
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o000)).unwrap();
    let plain = spec(&p, &["check-drift"], &[]);
    let gate = spec(&p, &["check-drift", "--pre-merge"], &[]);
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o755)).unwrap();
    assert_eq!(plain.code, 0, "{}", plain.stderr);
    assert!(
        plain
            .stderr
            .contains("check-drift: WARN: cannot read docs/superpowers: "),
        "{}",
        plain.stderr
    );
    assert_eq!(gate.code, 1, "{}", gate.stderr);
    assert!(
        gate.stderr
            .contains("check-drift: ERROR: cannot read docs/superpowers: "),
        "{}",
        gate.stderr
    );
}

fn locked_run(rel: &str, dir: bool, name: &str) {
    use std::os::unix::fs::PermissionsExt;
    if is_root() {
        return; // root reads everything: the test would pass vacuously.
    }
    let p = project(name);
    write(&p, ".spec/plan.md", "x\n");
    if dir {
        write(&p, &format!("{rel}/inner.md"), "x\n");
    } else {
        write(&p, rel, "x\n");
    }
    let locked = p.join(rel);
    let set = |mode: u32| std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(mode));
    set(0o000).unwrap();
    let text = spec(&p, &["check-drift"], &[]);
    let json = spec(&p, &["check-drift", "--json"], &[]);
    set(if dir { 0o755 } else { 0o644 }).unwrap();
    assert_eq!(text.code, 1, "{}", text.stderr);
    assert!(
        text.stderr.contains("check-drift: ERROR: cannot read "),
        "{}",
        text.stderr
    );
    assert_eq!(json.code, 1, "{}", json.stderr);
    let v: serde_json::Value = serde_json::from_slice(&json.stdout).unwrap();
    assert!(
        v["findings"].as_array().unwrap().iter().any(|f| {
            f["severity"] == "error" && f["message"].as_str().unwrap().starts_with("cannot read ")
        }),
        "{v}"
    );
}

#[test]
fn unreadable_root_plan_fails_closed() {
    locked_run(".spec/plan.md", false, "dr-locked-plan");
}

#[test]
fn unreadable_dir_in_the_count_walk_fails_closed() {
    locked_run(".spec/locked", true, "dr-locked-walk");
}

#[test]
fn unreadable_feature_plan_fails_closed() {
    use std::os::unix::fs::PermissionsExt;
    if is_root() {
        return;
    }
    let p = project("dr-locked-fplan");
    write(&p, ".spec/plan.md", "| auth | IN PROGRESS |\n");
    write(&p, ".spec/features/auth/plan.md", "x\n");
    let f = p.join(".spec/features/auth/plan.md");
    std::fs::set_permissions(&f, std::fs::Permissions::from_mode(0o000)).unwrap();
    let out = spec(&p, &["check-drift"], &[]);
    std::fs::set_permissions(&f, std::fs::Permissions::from_mode(0o644)).unwrap();
    assert_eq!(out.code, 1, "{}", out.stderr);
    assert!(out.stderr.contains("cannot read "), "{}", out.stderr);
}

#[test]
fn symlinked_artifacts_are_findings() {
    let p = project("dr-link");
    write(&p, ".spec/plan.md", "x\n");
    write(&p, "keep/real.md", "x\n");
    std::fs::create_dir_all(p.join("docs/superpowers/plans")).unwrap();
    std::os::unix::fs::symlink(
        p.join("keep/real.md"),
        p.join("docs/superpowers/plans/2026-01-01-auth.md"),
    )
    .unwrap();
    std::os::unix::fs::symlink(p.join("keep"), p.join("docs/superpowers/specs")).unwrap();
    let plain = spec(&p, &["check-drift"], &[]);
    assert_eq!(plain.code, 0, "{}", plain.stderr);
    assert!(
        plain.stderr.contains(
            "check-drift: WARN: superpowers artifact is a symlink: docs/superpowers/plans/2026-01-01-auth.md\n"
        ),
        "{}",
        plain.stderr
    );
    assert!(
        plain.stderr.contains(
            "check-drift: WARN: superpowers artifact is a symlink: docs/superpowers/specs\n"
        ),
        "{}",
        plain.stderr
    );
    let gate = spec(&p, &["check-drift", "--pre-merge", "--json"], &[]);
    assert_eq!(gate.code, 1, "{}", gate.stderr);
    let v: serde_json::Value = serde_json::from_slice(&gate.stdout).unwrap();
    let f = v["findings"].as_array().unwrap();
    assert_eq!(f.len(), 2, "{v}");
    assert!(f.iter().all(|f| f["severity"] == "error"));
    assert_eq!(f[0]["file"], "docs/superpowers/plans/2026-01-01-auth.md");
}

#[test]
fn an_entry_that_cannot_be_inspected_in_the_count_walk_fails_closed() {
    use std::os::unix::fs::PermissionsExt;
    if is_root() {
        return;
    }
    let p = project("dr-noexec");
    write(&p, ".spec/plan.md", "x\n");
    write(&p, ".spec/locked/inner.md", "x\n");
    let locked = p.join(".spec/locked");
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o444)).unwrap();
    let out = spec(&p, &["check-drift"], &[]);
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o755)).unwrap();
    assert_eq!(out.code, 1, "{}", out.stderr);
    assert!(
        out.stderr
            .contains("check-drift: ERROR: cannot read .spec/locked/inner.md: "),
        "{}",
        out.stderr
    );
}

const ROW: &str = "| demo | scope | IN PROGRESS |\n";

fn hub(children: &str, body: &str) -> String {
    format!("---\ntype: hub\nchildren: [{children}]\nupdated: 2026-01-15\n---\n\n# Plan\n\n{body}")
}

fn layered(name: &str, plan: &str) -> std::path::PathBuf {
    let p = project(name);
    write(&p, ".vibe/spec.json", common::LEGACY_STRICTNESS);
    write(&p, "docs/spec/plan.md", plan);
    write(&p, "docs/spec/features/demo/tech.md", "x\n");
    p
}

#[test]
fn rows_in_a_listed_note_count() {
    // skill-revision/16
    let p = layered(
        "dr-layered",
        &hub("concepts/feature-sequence.md", "See notes.\n"),
    );
    write(&p, "docs/spec/concepts/feature-sequence.md", ROW);
    let out = spec(&p, &["check-drift"], &[]);
    assert_eq!(out.code, 0, "{}", out.stderr);
    assert!(!out.stderr.contains("has no row"), "{}", out.stderr);
    assert!(
        out.stderr
            .contains("live feature 'demo' \u{2014} root plan says: IN PROGRESS"),
        "{}",
        out.stderr
    );
}

#[test]
fn rows_in_a_linked_note_count() {
    // skill-revision/16: a relative .md link counts like a child.
    let p = layered("dr-linked", &hub("", "- [Sequence](concepts/seq.md)\n"));
    write(&p, "docs/spec/concepts/seq.md", ROW);
    let out = spec(&p, &["check-drift"], &[]);
    assert_eq!(out.code, 0, "{}", out.stderr);
    assert!(
        out.stderr.contains("root plan says: IN PROGRESS"),
        "{}",
        out.stderr
    );
}

#[test]
fn a_missing_row_still_fails_and_a_flat_row_still_passes() {
    // skill-revision/16 controls
    let none = layered("dr-norow2", &hub("concepts/feature-sequence.md", ""));
    write(
        &none,
        "docs/spec/concepts/feature-sequence.md",
        "# Sequence\n",
    );
    let out = spec(&none, &["check-drift"], &[]);
    assert_eq!(out.code, 1, "{}", out.stderr);
    assert!(
        out.stderr
            .contains("feature 'demo' has no row in docs/spec/plan.md"),
        "{}",
        out.stderr
    );
    let flat = layered("dr-flat", &format!("# Plan\n\n{ROW}"));
    let out = spec(&flat, &["check-drift"], &[]);
    assert_eq!(out.code, 0, "{}", out.stderr);
}

#[test]
fn hostile_children_are_never_read() {
    // skill-revision/16 (Review Focus 3)
    let p = layered(
        "dr-hostile",
        &hub(
            "../outside.md, concepts/missing.md, concepts/link.md",
            "- [Out](../outside.md)\n",
        ),
    );
    write(&p, "outside.md", ROW);
    write(&p, "docs/spec/concepts/placeholder.md", "x\n");
    std::os::unix::fs::symlink("../../../outside.md", p.join("docs/spec/concepts/link.md"))
        .unwrap();
    let out = spec(&p, &["check-drift"], &[]);
    assert_eq!(out.code, 1, "{}", out.stderr);
    assert!(
        out.stderr
            .contains("feature 'demo' has no row in docs/spec/plan.md"),
        "{}",
        out.stderr
    );
    assert!(!out.stderr.contains("panicked"), "{}", out.stderr);
}
