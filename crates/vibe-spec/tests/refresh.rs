#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Real-Git explicit reconciliation controls (spec-projects-mcp/6).
mod common;
use std::{collections::BTreeMap, fs, path::Path};
use vibe_spec::{ContextSelector, Runtime, SpecApi, SpecRequest};
struct Fixture {
    _owner: vibe_core::testscratch::Scratch,
    api: SpecApi,
    project: vibe_spec::ProjectId,
    code: std::path::PathBuf,
}
fn snapshot(root: &Path) -> BTreeMap<String, Vec<u8>> {
    fn walk(root: &Path, dir: &Path, out: &mut BTreeMap<String, Vec<u8>>) {
        for e in fs::read_dir(dir).unwrap() {
            let e = e.unwrap();
            let p = e.path();
            if e.file_type().unwrap().is_dir() {
                walk(root, &p, out)
            } else {
                out.insert(
                    p.strip_prefix(root).unwrap().to_string_lossy().into(),
                    fs::read(p).unwrap(),
                );
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(root, root, &mut out);
    out
}
impl Fixture {
    fn new() -> Self {
        let owner = common::project("reconcile");
        let code = owner.join("code");
        fs::create_dir(&code).unwrap();
        common::git(&code, "2026-01-01T00:00:00Z", &["init", "-b", "main"]);
        common::git(
            &code,
            "2026-01-01T00:00:00Z",
            &["config", "maintenance.auto", "false"],
        );
        common::git(
            &code,
            "2026-01-01T00:00:00Z",
            &["commit", "--allow-empty", "-m", "initial"],
        );
        common::git(&code, "2026-01-01T00:00:00Z", &["config", "gc.auto", "0"]);
        let api = SpecApi::new(Runtime {
            cwd: code.clone(),
            env: BTreeMap::from([
                ("PATH".into(), std::env::var_os("PATH").unwrap_or_default()),
                ("HOME".into(), owner.as_os_str().to_owned()),
                ("VIBE_STORE".into(), owner.join("store").into_os_string()),
            ]),
        })
        .unwrap();
        let r = api
            .execute(SpecRequest::ProjectInit(vibe_spec::InitProjectRequest {
                selector: ContextSelector::default(),
                mode: vibe_spec::StorageMode::GlobalFirst,
                name: None,
                expected_revision: "absent".into(),
            }))
            .unwrap();
        let project = r.context.unwrap().project_id.unwrap();
        Self {
            _owner: owner,
            api,
            project,
            code,
        }
    }
    fn selector(&self, branch: &str) -> ContextSelector {
        ContextSelector {
            project: Some(self.project.clone()),
            branch: Some(branch.into()),
            checkout: None,
        }
    }
    fn branch(&self, name: &str) {
        let r = self
            .api
            .execute(SpecRequest::ProjectShow {
                selector: self.selector("main"),
            })
            .unwrap();
        self.api
            .execute(SpecRequest::BranchInit(vibe_spec::InitBranchRequest {
                selector: self.selector("main"),
                name: name.into(),
                from: "main".into(),
                expected_revision: r.data["revision"].as_str().unwrap().into(),
                expected_project_revision: r.data["project_revision"].as_str().unwrap().into(),
            }))
            .unwrap();
    }
    fn write(&self, branch: &str, path: &str, content: &str) {
        let selector = self.selector(branch);
        let c = self.api.resolve_context(&selector).unwrap();
        self.api
            .execute(SpecRequest::WriteDocument(
                vibe_spec::WriteDocumentRequest {
                    selector,
                    path: path.into(),
                    content: content.into(),
                    expected_revision: c.revision_token,
                },
            ))
            .unwrap();
    }
    fn snap(&self, branch: &str) {
        let selector = self.selector(branch);
        let c = self.api.resolve_context(&selector).unwrap();
        self.api
            .execute(SpecRequest::Snapshot(vibe_spec::SnapshotRequest {
                selector,
                expected_revision: c.revision_token,
            }))
            .unwrap();
    }
    fn merge(&self) {
        common::git(
            &self.code,
            "2026-01-02T00:00:00Z",
            &["merge", "--no-ff", "topic", "-m", "merge"],
        );
    }
    fn prepared() -> Self {
        let f = Self::new();
        common::git(&f.code, "2026-01-02T00:00:00Z", &["branch", "topic"]);
        f.branch("topic");
        f.api.refresh(&f.selector("main")).unwrap();
        f.write("main", "spec/lessons.md", "# Lessons\n\nTarget fact.\n");
        f.snap("main");
        common::git(&f.code, "2026-01-02T00:00:00Z", &["checkout", "topic"]);
        common::git(
            &f.code,
            "2026-01-02T00:00:00Z",
            &["commit", "--allow-empty", "-m", "topic"],
        );
        f.write("topic", "spec/extra.md", "# Extra\n\nSource fact.\n");
        f.snap("topic");
        common::git(&f.code, "2026-01-02T00:00:00Z", &["checkout", "main"]);
        f
    }
}
#[test]
fn spec_projects_mcp_7_exact_merge_applies_once_and_preserves_code() {
    let f = Fixture::prepared();
    f.merge();
    let code = snapshot(&f.code);
    let result = f.api.refresh(&f.selector("main")).unwrap();
    assert_eq!(result.applied.len(), 1);
    assert!(result.conflict.is_empty());
    let c = result.context.unwrap();
    assert_eq!(
        fs::read(c.spec_root.join("lessons.md")).unwrap(),
        b"# Lessons\n\nTarget fact.\n"
    );
    assert_eq!(
        fs::read(c.spec_root.join("extra.md")).unwrap(),
        b"# Extra\n\nSource fact.\n"
    );
    let store = snapshot(&f._owner.join("store"));
    let repeat = f.api.refresh(&f.selector("main")).unwrap();
    assert_eq!(repeat.applied, result.applied);
    assert_eq!(repeat.context.unwrap().spec_revision, c.spec_revision);
    assert_eq!(snapshot(&f._owner.join("store")), store);
    assert_eq!(snapshot(&f.code), code);
}
#[test]
fn spec_projects_mcp_7_clean_snapshot_deduplicates_and_later_source_edit_refuses() {
    let f = Fixture::prepared();
    f.snap("main");
    let before = f.api.resolve_context(&f.selector("main")).unwrap();
    let state = snapshot(&f._owner.join("store"));
    f.snap("main");
    assert_eq!(snapshot(&f._owner.join("store")), state);
    assert_eq!(
        f.api
            .resolve_context(&f.selector("main"))
            .unwrap()
            .spec_revision,
        before.spec_revision
    );
    f.write(
        "topic",
        "spec/extra.md",
        "# Extra\n\nEdited after snapshot.\n",
    );
    f.merge();
    let code = snapshot(&f.code);
    let r = f.api.refresh(&f.selector("main")).unwrap();
    assert!(r.applied.is_empty());
    assert_eq!(r.needs_input.len(), 1);
    assert!(!r.context.unwrap().spec_root.join("extra.md").exists());
    assert_eq!(snapshot(&f.code), code);
}
#[test]
fn spec_projects_mcp_7_conflict_keeps_reads_available_and_preview_pure() {
    let f = Fixture::prepared();
    // Record a new source code tip and exactly one snapshot for that tip.
    common::git(&f.code, "2026-01-03T00:00:00Z", &["checkout", "topic"]);
    common::git(
        &f.code,
        "2026-01-03T00:00:00Z",
        &["commit", "--allow-empty", "-m", "more"],
    );
    f.write(
        "topic",
        "spec/lessons.md",
        "# Lessons\n\nConflicting source fact.\n",
    );
    f.snap("topic");
    common::git(&f.code, "2026-01-03T00:00:00Z", &["checkout", "main"]);
    f.merge();
    let before = snapshot(&f._owner.join("store"));
    f.api
        .plan_reconciliation(vibe_spec::ReconcileRequest {
            selector: f.selector("main"),
            source: "topic".into(),
            target: "main".into(),
        })
        .unwrap();
    assert_eq!(snapshot(&f._owner.join("store")), before);
    let read = f
        .api
        .read_document(&f.selector("main"), "spec/lessons.md")
        .unwrap();
    assert_eq!(read.context.refresh.conflict.len(), 1);
    assert_eq!(read.document.content, "# Lessons\n\nTarget fact.\n");
    let r = f.api.refresh(&f.selector("main")).unwrap();
    assert_eq!(r.conflict, read.context.refresh.conflict);
}
#[test]
fn spec_projects_mcp_7_uncertain_evidence_never_automatically_applies() {
    for case in [
        "missing-source",
        "dirty-source",
        "dirty-target",
        "ambiguous",
        "missing-snapshot",
        "intervening",
        "squash",
        "rebase",
        "octopus",
        "no-baseline",
    ] {
        let f = Fixture::prepared();
        let target = f.api.resolve_context(&f.selector("main")).unwrap();
        let source = f.api.resolve_context(&f.selector("topic")).unwrap();
        match case {
            "ambiguous" => {
                common::git(&f.code, "2026-01-03T00:00:00Z", &["checkout", "topic"]);
                f.write(
                    "topic",
                    "spec/extra.md",
                    "# Extra\n\nNew source revision.\n",
                );
                f.snap("topic");
                common::git(&f.code, "2026-01-03T00:00:00Z", &["checkout", "main"]);
                f.merge();
            }
            "intervening" => {
                f.merge();
                common::git(
                    &f.code,
                    "2026-01-03T00:00:00Z",
                    &["commit", "--allow-empty", "-m", "after merge"],
                );
            }
            "squash" => {
                common::git(
                    &f.code,
                    "2026-01-03T00:00:00Z",
                    &["merge", "--squash", "topic"],
                );
                common::git(
                    &f.code,
                    "2026-01-03T00:00:00Z",
                    &["commit", "--allow-empty", "-m", "squash"],
                );
            }
            "rebase" => {
                common::git(
                    &f.code,
                    "2026-01-03T00:00:00Z",
                    &["commit", "--allow-empty", "-m", "new main"],
                );
                common::git(&f.code, "2026-01-03T00:00:00Z", &["checkout", "topic"]);
                common::git(&f.code, "2026-01-03T00:00:00Z", &["rebase", "main"]);
                common::git(&f.code, "2026-01-03T00:00:00Z", &["checkout", "main"]);
                common::git(
                    &f.code,
                    "2026-01-03T00:00:00Z",
                    &["merge", "--ff-only", "topic"],
                );
            }
            "octopus" => {
                common::git(
                    &f.code,
                    "2026-01-03T00:00:00Z",
                    &["checkout", "-b", "other"],
                );
                common::git(
                    &f.code,
                    "2026-01-03T00:00:00Z",
                    &["commit", "--allow-empty", "-m", "other"],
                );
                common::git(&f.code, "2026-01-03T00:00:00Z", &["checkout", "main"]);
                common::git(
                    &f.code,
                    "2026-01-03T00:00:00Z",
                    &["merge", "--no-ff", "topic", "other", "-m", "octopus"],
                );
            }
            _ => f.merge(),
        }
        if case == "missing-source" {
            common::git(&f.code, "2026-01-03T00:00:00Z", &["branch", "-D", "topic"]);
        }
        if case == "dirty-source" {
            fs::write(source.spec_root.join("extra.md"), "dirty source").unwrap();
        }
        if case == "dirty-target" {
            fs::write(target.spec_root.join("lessons.md"), "dirty target").unwrap();
        }
        if case == "missing-snapshot" || case == "no-baseline" {
            let path = f
                ._owner
                .join("store/spec/projects")
                .join(f.project.as_str())
                .join("project.json");
            let mut v: serde_json::Value =
                serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
            v[if case == "no-baseline" {
                "observations"
            } else {
                "snapshots"
            }] = serde_json::json!([]);
            fs::write(path, serde_json::to_vec_pretty(&v).unwrap()).unwrap();
        }
        let before = fs::read(target.spec_root.join("lessons.md")).unwrap();
        let code = snapshot(&f.code);
        let result = f.api.refresh(&f.selector("main")).unwrap();
        assert!(result.applied.is_empty(), "{case}");
        assert!(result.conflict.is_empty(), "{case}");
        if case == "no-baseline" {
            assert!(result.needs_input.is_empty());
            let old_ancestor = f.api.refresh(&f.selector("main")).unwrap();
            assert!(old_ancestor.applied.is_empty());
            assert_eq!(old_ancestor.needs_input.len(), 1);
        } else {
            assert_eq!(result.needs_input.len(), 1, "{case}");
        }
        assert_eq!(
            fs::read(target.spec_root.join("lessons.md")).unwrap(),
            before,
            "{case}"
        );
        assert!(!target.spec_root.join("extra.md").exists(), "{case}");
        assert!(snapshot(&f.code) == code, "code changed: {case}");
        let store = snapshot(&f._owner.join("store"));
        if case != "no-baseline" {
            let repeat = f.api.refresh(&f.selector("main")).unwrap();
            assert_eq!(repeat.needs_input, result.needs_input, "{case}");
            assert!(
                snapshot(&f._owner.join("store")) == store,
                "repeat changed store: {case}"
            );
        }
    }
}
#[test]
fn spec_projects_mcp_7_stale_mutation_never_refreshes_or_retargets() {
    let f = Fixture::prepared();
    let old = f.api.resolve_context(&f.selector("main")).unwrap();
    f.merge();
    let store = snapshot(&f._owner.join("store"));
    let e = f
        .api
        .execute(SpecRequest::WriteDocument(
            vibe_spec::WriteDocumentRequest {
                selector: f.selector("main"),
                path: "spec/extra.md".into(),
                content: "must not write".into(),
                expected_revision: old.revision_token,
            },
        ))
        .unwrap_err();
    assert_eq!(e.code, "revision_changed");
    assert!(snapshot(&f._owner.join("store")) == store);
    assert_eq!(f.api.refresh(&f.selector("main")).unwrap().applied.len(), 1);
}
#[test]
fn spec_projects_mcp_7_legacy_query_keeps_json_shape_and_reports_refresh_separately() {
    let f = Fixture::prepared();
    f.merge();
    let store = f._owner.join("store");
    let out = common::spec(
        &f.code,
        &["validate", "--spec-only", "--json"],
        &[("VIBE_STORE", store.to_str().unwrap())],
    );
    let value: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(value["v"], 1);
    assert!(value.get("refresh").is_none());
    assert!(out.stderr.contains("refresh:"), "{}", out.stderr);
}
#[test]
fn spec_projects_mcp_7_review_ambiguous_unrelated_source_keeps_target_reads_available() {
    for integrated in [false, true] {
        let f = Fixture::prepared();
        if integrated {
            let p = f
                .api
                .plan_reconciliation(vibe_spec::ReconcileRequest {
                    selector: f.selector("main"),
                    source: "topic".into(),
                    target: "main".into(),
                })
                .unwrap();
            f.api.apply_reconciliation(&p, &p.target_revision).unwrap();
        }
        for n in ["clone-two", "clone-three"] {
            let clone = f._owner.join(n);
            common::git(
                &f._owner,
                "2026-01-03T00:00:00Z",
                &["clone", f.code.to_str().unwrap(), clone.to_str().unwrap()],
            );
            common::git(&clone, "2026-01-03T00:00:00Z", &["checkout", "topic"]);
            let registry = f.api.execute(SpecRequest::Projects).unwrap();
            f.api
                .execute(SpecRequest::ProjectAttach(
                    vibe_spec::AttachProjectRequest {
                        project: f.project.clone(),
                        checkout: Some(clone),
                        main: false,
                        expected_revision: registry.data["revision"].as_str().unwrap().into(),
                    },
                ))
                .unwrap();
        }
        let code = snapshot(&f.code);
        let c = f.api.resolve_context(&f.selector("main")).unwrap();
        assert!(c.refresh.applied.is_empty());
        assert!(
            c.refresh
                .diagnostics
                .iter()
                .any(|d| d["code"] == "ambiguous_checkout")
        );
        let read = f
            .api
            .read_document(&f.selector("main"), "spec/lessons.md")
            .unwrap();
        assert_eq!(read.document.content, "# Lessons\n\nTarget fact.\n");
        let query = f
            .api
            .execute(SpecRequest::ListSpecs(f.selector("main")))
            .unwrap();
        assert!(query.context.unwrap().refresh.applied.is_empty());
        assert!(snapshot(&f.code) == code);
    }
}
#[test]
fn spec_projects_mcp_7_review_automatic_descendant_leaves_no_false_pending() {
    for names in [["y", "x"], ["x", "y"]] {
        let f = Fixture::new();
        for name in names {
            common::git(&f.code, "2026-01-03T00:00:00Z", &["branch", name]);
            f.branch(name);
        }
        f.api.refresh(&f.selector("main")).unwrap();
        f.write("main", "spec/lessons.md", "# Lessons\n\nTarget fact.\n");
        f.snap("main");
        common::git(&f.code, "2026-01-03T00:00:00Z", &["checkout", "y"]);
        common::git(
            &f.code,
            "2026-01-03T00:00:00Z",
            &["commit", "--allow-empty", "-m", "y"],
        );
        f.write("y", "spec/y.md", "# Y\n\nY fact.\n");
        f.snap("y");
        common::git(&f.code, "2026-01-03T00:00:00Z", &["checkout", "x"]);
        common::git(
            &f.code,
            "2026-01-03T00:00:00Z",
            &["merge", "--ff-only", "y"],
        );
        common::git(
            &f.code,
            "2026-01-03T00:00:00Z",
            &["commit", "--allow-empty", "-m", "x"],
        );
        let p = f
            .api
            .plan_reconciliation(vibe_spec::ReconcileRequest {
                selector: f.selector("x"),
                source: "y".into(),
                target: "x".into(),
            })
            .unwrap();
        f.api.apply_reconciliation(&p, &p.target_revision).unwrap();
        f.write("x", "spec/x.md", "# X\n\nX fact.\n");
        f.snap("x");
        common::git(&f.code, "2026-01-03T00:00:00Z", &["checkout", "main"]);
        common::git(
            &f.code,
            "2026-01-03T00:00:00Z",
            &["merge", "--no-ff", "x", "-m", "merge x"],
        );
        let code = snapshot(&f.code);
        let report = f.api.refresh(&f.selector("main")).unwrap();
        assert_eq!(report.applied.len(), 1);
        assert!(
            report.needs_input.is_empty(),
            "{names:?}: {:?}",
            report.needs_input
        );
        assert!(report.conflict.is_empty());
        let c = report.context.unwrap();
        assert_eq!(
            fs::read(c.spec_root.join("y.md")).unwrap(),
            b"# Y\n\nY fact.\n"
        );
        assert_eq!(
            fs::read(c.spec_root.join("x.md")).unwrap(),
            b"# X\n\nX fact.\n"
        );
        let state = snapshot(&f._owner.join("store"));
        let repeat = f.api.refresh(&f.selector("main")).unwrap();
        assert_eq!(repeat.applied, report.applied);
        assert!(repeat.needs_input.is_empty());
        assert!(snapshot(&f._owner.join("store")) == state);
        assert!(snapshot(&f.code) == code);
    }
}
