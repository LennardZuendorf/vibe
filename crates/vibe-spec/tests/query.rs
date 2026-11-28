#![allow(clippy::unwrap_used, clippy::expect_used)]
// vibe-spec: literal-file
//! `plan` and `feature` answer Spec JSON v1 from superpowers artifacts, or
//! from the fallback feature folder.
mod common;

use common::{project, spec, write};
use serde_json::{Value, json};

const SP_PLAN: &str = "# X Plan\n\n### Task 1: Freeze the oracles\n\n- [x] **Step 1**\n- [x] **Step 2**\n\n### Task 2: Port\n\n- [x] **Step 1**\n- [ ] **Step 2**\n\n### Task 3\n\n- [ ] **Step 1**\n\n## Notes\n\n- [x] not a task step\n";
const FALLBACK_PLAN: &str = "---\ntype: feature-plan\n---\n\n## Status\n\n| Unit | Status |\n|---|---|\n| auth/1 | DONE |\n| auth/2 | IN PROGRESS |\n|  |  |\n\nAfter the table.\n";

fn v(out: &common::Out) -> Value {
    assert_eq!(out.code, 0, "{}", out.stderr);
    serde_json::from_slice(&out.stdout).unwrap()
}

#[test]
fn plan_reads_the_newest_superpowers_plan() {
    let p = project("q-plan-sp");
    std::fs::create_dir_all(p.join(".spec")).unwrap();
    write(
        &p,
        "docs/superpowers/plans/2026-01-01-csv-export.md",
        "### Task 1: Old\n",
    );
    write(
        &p,
        "docs/superpowers/plans/2026-02-01-csv-export.md",
        SP_PLAN,
    );
    write(
        &p,
        "docs/superpowers/plans/2026-03-01-csv-export-v2.md",
        "### Task 9: Other feature\n",
    );
    let out = spec(&p, &["plan", "--feature", "csv-export", "--json"], &[]);
    assert_eq!(
        v(&out),
        json!({
            "v": 1,
            "feature": "csv-export",
            "source": { "kind": "plan", "path": "docs/superpowers/plans/2026-02-01-csv-export.md" },
            "rows": [
                { "id": "Task 1", "status": "done", "title": "Freeze the oracles" },
                { "id": "Task 2", "status": "in-progress", "title": "Port" },
                { "id": "Task 3", "status": "todo" }
            ]
        })
    );
    #[cfg(unix)]
    common::assert_spec_json("plan", out.text().trim_end());
    let text = spec(&p, &["plan", "--feature", "csv-export"], &[]);
    assert_eq!(
        text.text(),
        "Task 1\tdone\tFreeze the oracles\nTask 2\tin-progress\tPort\nTask 3\ttodo\n"
    );
}

#[test]
fn plan_falls_back_to_the_unit_status_table() {
    let p = project("q-plan-fb");
    write(&p, ".spec/features/auth/plan.md", FALLBACK_PLAN);
    let out = spec(&p, &["plan", "--feature", "auth", "--json"], &[]);
    assert_eq!(
        v(&out),
        json!({
            "v": 1,
            "feature": "auth",
            "source": { "kind": "plan", "path": ".spec/features/auth/plan.md" },
            "rows": [
                { "id": "auth/1", "status": "DONE" },
                { "id": "auth/2", "status": "IN PROGRESS" }
            ]
        })
    );
    #[cfg(unix)]
    common::assert_spec_json("plan", out.text().trim_end());
}

#[test]
fn a_fallback_plan_without_a_table_reads_its_unit_headings() {
    let p = project("q-plan-head");
    let plan = "---\ntype: feature-plan\nfeature: auth\n---\n\n# Plan\n\n## Units\n\n### auth/1 — Record the choice\n\n- [x] write it\n- [x] read it\n\n### auth/2 – Publish it\n\n- [x] first\n- [ ] second\n\n### auth/3: Wire it\n\n- [ ] only\n\n### auth/4\n\nNothing to tick yet.\n\n## Notes\n\n- [x] not a unit step\n\n```md\n### auth/9 — fenced\n- [ ] ignored\n```\n\n### other/1 — another feature\n\n- [ ] ignored\n";
    write(&p, "docs/spec/features/auth/plan.md", plan);
    let out = spec(&p, &["plan", "--feature", "auth", "--json"], &[]);
    assert_eq!(
        v(&out),
        json!({
            "v": 1,
            "feature": "auth",
            "source": { "kind": "plan", "path": "docs/spec/features/auth/plan.md" },
            "rows": [
                { "id": "auth/1", "status": "done", "title": "Record the choice" },
                { "id": "auth/2", "status": "in-progress", "title": "Publish it" },
                { "id": "auth/3", "status": "todo", "title": "Wire it" },
                { "id": "auth/4", "status": "todo" }
            ]
        })
    );
    #[cfg(unix)]
    common::assert_spec_json("plan", out.text().trim_end());
    let text = spec(&p, &["plan", "--feature", "auth"], &[]);
    assert_eq!(
        text.text(),
        "auth/1\tdone\tRecord the choice\nauth/2\tin-progress\tPublish it\nauth/3\ttodo\tWire it\nauth/4\ttodo\n"
    );
}

#[test]
fn a_unit_table_still_wins_over_headings() {
    let p = project("q-plan-both");
    let plan = "| Unit | Status |\n|---|---|\n| auth/1 | DONE |\n\n### auth/1 — Record the choice\n\n- [ ] write it\n";
    write(&p, "docs/spec/features/auth/plan.md", plan);
    let got = v(&spec(&p, &["plan", "--feature", "auth", "--json"], &[]));
    assert_eq!(got["rows"], json!([{ "id": "auth/1", "status": "DONE" }]));
}

#[test]
fn an_empty_fallback_plan_names_only_the_forms_the_fallback_reads() {
    let p = project("q-plan-empty");
    write(
        &p,
        "docs/spec/features/auth/plan.md",
        "### Task 1: a superpowers heading this file never answers to\n\n- [ ] write it\n",
    );
    let out = spec(&p, &["plan", "--feature", "auth", "--json"], &[]);
    assert_eq!(out.code, 0, "{}", out.stderr);
    assert_eq!(v(&out)["rows"], json!([]));
    assert!(
        out.stderr.contains("docs/spec/features/auth/plan.md")
            && out.stderr.contains("no unit rows in a fallback plan"),
        "stderr was {:?}",
        out.stderr
    );
    assert!(
        out.stderr.contains("`| Unit | Status |`") && out.stderr.contains("`### auth/<n>`"),
        "the fallback's own forms are missing from {:?}",
        out.stderr
    );
    assert!(
        !out.stderr.contains("### Task <n>:"),
        "the fallback names a form it never reads: {:?}",
        out.stderr
    );
}

#[test]
fn an_empty_superpowers_plan_names_only_the_task_headings() {
    let p = project("q-plan-empty-sp");
    std::fs::create_dir_all(p.join("docs/spec")).unwrap();
    write(
        &p,
        "docs/superpowers/plans/2026-01-01-tab.md",
        "# Tab\n\n| Unit | Status |\n|---|---|\n| tab/1 | done |\n",
    );
    let out = spec(&p, &["plan", "--feature", "tab", "--json"], &[]);
    assert_eq!(out.code, 0, "{}", out.stderr);
    assert_eq!(v(&out)["rows"], json!([]));
    assert!(
        out.stderr
            .contains("docs/superpowers/plans/2026-01-01-tab.md")
            && out.stderr.contains("no unit rows in a superpowers plan"),
        "stderr was {:?}",
        out.stderr
    );
    assert!(
        out.stderr.contains("`### Task <n>:`"),
        "the superpowers form is missing from {:?}",
        out.stderr
    );
    assert!(
        !out.stderr.contains("| Unit | Status |") && !out.stderr.contains("tab/<n>"),
        "the superpowers plan names forms it never reads: {:?}",
        out.stderr
    );
}

#[test]
fn feature_reports_its_layer_and_docs() {
    let p = project("q-feature");
    write(&p, ".spec/features/auth/product.md", "x\n");
    write(&p, ".spec/features/auth/plan.md", "x\n");
    write(&p, ".spec/features/auth/notes.md", "x\n");
    write(
        &p,
        "docs/superpowers/specs/2026-02-01-csv-export-design.md",
        "x\n",
    );
    let sp = spec(&p, &["feature", "csv-export", "--json"], &[]);
    assert_eq!(
        v(&sp),
        json!({
            "v": 1, "feature": "csv-export", "layer": "superpowers",
            "docs": [{ "kind": "design", "path": "docs/superpowers/specs/2026-02-01-csv-export-design.md" }]
        })
    );
    let fb = spec(&p, &["feature", "auth", "--json"], &[]);
    assert_eq!(
        v(&fb),
        json!({
            "v": 1, "feature": "auth", "layer": "fallback",
            "docs": [
                { "kind": "product", "path": ".spec/features/auth/product.md" },
                { "kind": "plan", "path": ".spec/features/auth/plan.md" }
            ]
        })
    );
    #[cfg(unix)]
    {
        common::assert_spec_json("feature", sp.text().trim_end());
        common::assert_spec_json("feature", fb.text().trim_end());
    }
    let text = spec(&p, &["feature", "auth"], &[]);
    assert_eq!(
        text.text(),
        "layer: fallback\nproduct\t.spec/features/auth/product.md\nplan\t.spec/features/auth/plan.md\n"
    );
}

#[test]
fn exit_codes_follow_the_contract() {
    let p = project("q-codes");
    std::fs::create_dir_all(p.join(".spec")).unwrap();
    assert_eq!(
        spec(&p, &["plan", "--feature", "nope", "--json"], &[]).code,
        1
    );
    assert_eq!(spec(&p, &["feature", "nope"], &[]).code, 1);
    assert_eq!(spec(&p, &["feature", "Bad_Name"], &[]).code, 2);
    assert_eq!(spec(&p, &["plan", "--feature", "../x"], &[]).code, 2);
    assert_eq!(spec(&p, &["plan"], &[]).code, 2);
    let none = project("q-none");
    assert_eq!(
        spec(&none, &["plan", "--feature", "auth", "--json"], &[]).code,
        3
    );
    assert_eq!(spec(&none, &["feature", "auth"], &[]).code, 3);
}

#[test]
fn a_root_outside_the_project_reports_absolute_paths() {
    let p = project("q-out-proj");
    let outside = project("q-out-root");
    write(&outside, "features/auth/plan.md", FALLBACK_PLAN);
    let root = outside.to_string_lossy().into_owned();
    let abs = outside
        .join("features")
        .join("auth")
        .join("plan.md")
        .to_string_lossy()
        .into_owned();
    let out = spec(
        &p,
        &["--root", &root, "plan", "--feature", "auth", "--json"],
        &[],
    );
    let got = v(&out);
    assert_eq!(got["source"]["path"], json!(abs));
    let out = spec(&p, &["--root", &root, "feature", "auth", "--json"], &[]);
    let got = v(&out);
    let path = got["docs"][0]["path"].as_str().unwrap();
    assert!(!path.starts_with("//"), "{path}");
    assert_eq!(path, abs);
}

#[test]
fn fenced_blocks_do_not_count_as_plan_structure() {
    let p = project("q-fence");
    std::fs::create_dir_all(p.join(".spec")).unwrap();
    let plan = "### Task 1: A\n\n- [x] one\n- [x] two\n\n```md\n- [ ] x\n## Example\n```\n\n~~~~\n### Task 7: fake\n~~~\n- [ ] still fenced\n~~~~\n\n### Task 2: B\n\n- [ ] b\n";
    write(&p, "docs/superpowers/plans/2026-02-01-fz.md", plan);
    let got = v(&spec(&p, &["plan", "--feature", "fz", "--json"], &[]));
    assert_eq!(
        got["rows"],
        json!([
            { "id": "Task 1", "status": "done", "title": "A" },
            { "id": "Task 2", "status": "todo", "title": "B" }
        ])
    );
}

#[test]
fn a_directory_named_like_a_plan_is_skipped() {
    let p = project("q-dir");
    std::fs::create_dir_all(p.join(".spec")).unwrap();
    std::fs::create_dir_all(p.join("docs/superpowers/plans/2026-02-01-dd.md")).unwrap();
    assert_eq!(spec(&p, &["plan", "--feature", "dd"], &[]).code, 1);
    write(&p, ".spec/features/dd/product.md", "x\n");
    let got = v(&spec(&p, &["feature", "dd", "--json"], &[]));
    assert_eq!(got["layer"], "fallback");
}

// ── `.vibe/spec.json` `featureLayer` (D45, #125) ────────────────────────────
// The key selects a layer and never switches silently: the selected layer
// answers, or the read is refused naming the artifact the other layer holds.

#[test]
fn feature_layer_vibe_wins_over_a_superpowers_plan() {
    let p = project("q-layer-vibe");
    write(&p, ".vibe/spec.json", "{\"featureLayer\": \"vibe\"}\n");
    write(&p, "docs/spec/features/auth/plan.md", FALLBACK_PLAN);
    write(&p, "docs/superpowers/plans/2026-02-01-auth.md", SP_PLAN);
    let plan = v(&spec(&p, &["plan", "--feature", "auth", "--json"], &[]));
    assert_eq!(
        plan["source"]["path"],
        json!("docs/spec/features/auth/plan.md"),
        "the fallback layer wins when featureLayer says vibe"
    );
    assert_eq!(
        plan["rows"],
        json!([
            { "id": "auth/1", "status": "DONE" },
            { "id": "auth/2", "status": "IN PROGRESS" }
        ])
    );
    let feature = v(&spec(&p, &["feature", "auth", "--json"], &[]));
    assert_eq!(feature["layer"], json!("fallback"));
    assert_eq!(
        feature["docs"][0]["path"],
        json!("docs/spec/features/auth/plan.md")
    );
}

#[test]
fn feature_layer_superpowers_refuses_instead_of_reading_the_fallback_layer() {
    let p = project("q-layer-sp");
    write(
        &p,
        ".vibe/spec.json",
        "{\"featureLayer\": \"superpowers\"}\n",
    );
    write(&p, "docs/spec/features/auth/plan.md", FALLBACK_PLAN);
    let out = spec(&p, &["plan", "--feature", "auth", "--json"], &[]);
    assert_eq!(out.code, 1, "{}", out.stderr);
    assert!(out.stdout.is_empty(), "{}", out.text());
    assert!(
        out.stderr.contains("featureLayer")
            && out.stderr.contains("docs/spec/features/auth/plan.md"),
        "stderr was {:?}",
        out.stderr
    );
    let out = spec(&p, &["feature", "auth"], &[]);
    assert_eq!(out.code, 1, "{}", out.stderr);
    assert!(
        out.stderr.contains("featureLayer"),
        "stderr was {:?}",
        out.stderr
    );
}

#[test]
fn feature_layer_vibe_refuses_when_the_fallback_layer_holds_no_plan() {
    let p = project("q-layer-empty");
    write(&p, ".vibe/spec.json", "{\"featureLayer\": \"vibe\"}\n");
    write(&p, "docs/spec/README.md", "x\n");
    write(&p, "docs/superpowers/plans/2026-02-01-auth.md", SP_PLAN);
    let out = spec(&p, &["plan", "--feature", "auth", "--json"], &[]);
    assert_eq!(out.code, 1, "{}", out.stderr);
    assert!(out.stdout.is_empty(), "{}", out.text());
    assert!(
        out.stderr.contains("featureLayer")
            && out.stderr.contains("docs/spec/features/auth/plan.md")
            && out
                .stderr
                .contains("docs/superpowers/plans/2026-02-01-auth.md"),
        "the refusal names both candidates; stderr was {:?}",
        out.stderr
    );
    let out = spec(&p, &["feature", "auth"], &[]);
    assert_eq!(out.code, 1, "{}", out.stderr);
    assert!(
        out.stderr.contains("featureLayer") && out.stderr.contains("docs/superpowers"),
        "stderr was {:?}",
        out.stderr
    );
}

#[test]
fn feature_layer_vibe_refuses_an_empty_fallback_folder() {
    let p = project("q-layer-emptyfolder");
    write(&p, ".vibe/spec.json", "{\"featureLayer\": \"vibe\"}\n");
    std::fs::create_dir_all(p.join("docs/spec/features/auth")).unwrap();
    let out = spec(&p, &["feature", "auth", "--json"], &[]);
    assert_eq!(out.code, 1, "{}", out.stderr);
    assert!(
        out.stderr.contains("featureLayer") && out.stderr.contains("docs/spec/features/auth"),
        "stderr was {:?}",
        out.stderr
    );
    assert!(out.stdout.is_empty(), "{}", out.text());
}

#[test]
fn an_unknown_feature_layer_is_a_config_error() {
    let p = project("q-layer-bad");
    write(&p, ".vibe/spec.json", "{\"featureLayer\": \"openspec\"}\n");
    write(&p, "docs/spec/features/auth/plan.md", FALLBACK_PLAN);
    for args in [
        vec!["plan", "--feature", "auth", "--json"],
        vec!["feature", "auth", "--json"],
    ] {
        let out = spec(&p, &args, &[]);
        assert_eq!(out.code, 2, "{args:?}: {}", out.stderr);
        assert!(
            out.stderr.contains("`featureLayer`")
                && out.stderr.contains("\"vibe\"")
                && out.stderr.contains("\"superpowers\""),
            "the error names the allowed values; stderr was {:?}",
            out.stderr
        );
    }
}

#[test]
fn without_the_key_superpowers_still_wins() {
    let p = project("q-layer-default");
    write(&p, "docs/spec/features/auth/plan.md", FALLBACK_PLAN);
    write(&p, "docs/superpowers/plans/2026-02-01-auth.md", SP_PLAN);
    let plan = v(&spec(&p, &["plan", "--feature", "auth", "--json"], &[]));
    assert_eq!(
        plan["source"]["path"],
        json!("docs/superpowers/plans/2026-02-01-auth.md")
    );
    let feature = v(&spec(&p, &["feature", "auth", "--json"], &[]));
    assert_eq!(feature["layer"], json!("superpowers"));
}

// ── Two backends holding one name (#126) ───────────────────────────────────
// The contract: "Two backends holding one name is an error, not a precedence
// question"; each refusal names both candidates, and a total miss names the
// feature and both candidate roots. Neither selector may diverge.

#[test]
fn plan_refuses_a_name_both_backends_hold() {
    let p = project("q-amb-plan");
    write(&p, "docs/spec/README.md", "x\n");
    write(&p, "docs/superpowers/plans/2026-01-01-auth.md", SP_PLAN);
    write(&p, "openspec/changes/auth/proposal.md", "## Why\n\nx\n");
    let out = spec(&p, &["plan", "--feature", "auth"], &[]);
    assert_eq!(out.code, 1, "{}", out.stderr);
    assert!(out.stdout.is_empty(), "{}", out.text());
    assert!(
        out.stderr
            .contains("docs/superpowers/plans/2026-01-01-auth.md")
            && out.stderr.contains("openspec/changes/auth"),
        "the refusal names both candidates; stderr was {:?}",
        out.stderr
    );
}

#[test]
fn feature_refuses_a_name_both_backends_hold() {
    let p = project("q-amb-feature");
    write(&p, "docs/spec/README.md", "x\n");
    write(&p, "docs/superpowers/plans/2026-01-01-auth.md", SP_PLAN);
    write(&p, "openspec/changes/auth/proposal.md", "## Why\n\nx\n");
    let out = spec(&p, &["feature", "auth"], &[]);
    assert_eq!(out.code, 1, "{}", out.stderr);
    assert!(out.stdout.is_empty(), "{}", out.text());
    assert!(
        out.stderr
            .contains("docs/superpowers/plans/2026-01-01-auth.md")
            && out.stderr.contains("openspec/changes/auth"),
        "the refusal names both candidates, as `plan` does; stderr was {:?}",
        out.stderr
    );
}

#[test]
fn plan_miss_names_the_feature_and_both_candidate_roots() {
    let p = project("q-miss-plan");
    write(&p, "docs/spec/README.md", "x\n");
    let out = spec(&p, &["plan", "--feature", "ghost"], &[]);
    assert_eq!(out.code, 1, "{}", out.stderr);
    assert!(
        out.stderr.contains("ghost")
            && out.stderr.contains("docs/superpowers/plans")
            && out.stderr.contains("openspec/changes")
            && out.stderr.contains("docs/spec/features/ghost"),
        "the miss states the feature and both candidate roots; stderr was {:?}",
        out.stderr
    );
}

#[test]
fn feature_miss_names_the_feature_and_both_candidate_roots() {
    let p = project("q-miss-feature");
    write(&p, "docs/spec/README.md", "x\n");
    let out = spec(&p, &["feature", "ghost"], &[]);
    assert_eq!(out.code, 1, "{}", out.stderr);
    assert!(
        out.stderr.contains("ghost")
            && out.stderr.contains("docs/superpowers/specs")
            && out.stderr.contains("openspec/changes")
            && out.stderr.contains("docs/spec/features/ghost"),
        "the miss states the feature and both candidate roots; stderr was {:?}",
        out.stderr
    );
}

#[test]
fn an_openspec_only_name_is_not_reported_as_superpowers_absence_alone() {
    let p = project("q-openspec-only");
    write(&p, "docs/spec/README.md", "x\n");
    write(&p, "openspec/changes/auth/proposal.md", "## Why\n\nx\n");
    for args in [vec!["plan", "--feature", "auth"], vec!["feature", "auth"]] {
        let out = spec(&p, &args, &[]);
        assert_eq!(out.code, 1, "{args:?}: {}", out.stderr);
        assert!(
            out.stderr.contains("openspec/changes/auth") && !out.stderr.contains("both backends"),
            "a sole openspec holder is named as such, never as ambiguity; stderr was {:?}",
            out.stderr
        );
    }
}
