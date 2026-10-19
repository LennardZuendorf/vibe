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
