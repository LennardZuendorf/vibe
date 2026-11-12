#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Real-Git explicit reconciliation controls (spec-projects-mcp/6).
mod common;
use std::{collections::BTreeMap, fs, path::Path};
use vibe_spec::{
    ContextSelector, ReconcileRequest, ReconcileState, ResolutionFile, ResolveRequest, Runtime,
    SpecApi, SpecRequest,
};
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
    fn plan(&self) -> vibe_spec::ReconcilePlan {
        self.api
            .plan_reconciliation(ReconcileRequest {
                selector: self.selector("main"),
                source: "topic".into(),
                target: "main".into(),
            })
            .unwrap()
    }
    fn apply(&self, p: &vibe_spec::ReconcilePlan) -> vibe_spec::ReconcileRecord {
        self.api
            .apply_reconciliation(p, &p.target_revision)
            .unwrap()
    }
}
#[test]
fn spec_projects_mcp_6_clean_merge_two_parents_idempotent_and_code_unchanged() {
    let f = Fixture::new();
    f.branch("topic");
    f.write("topic", "spec/lessons.md", "# Lessons\n\nSource fact.\n");
    f.write("main", "spec/lessons.md", "# Lessons\n\nSource fact.\n");
    let code = snapshot(&f.code);
    let p = f.plan();
    let before = f.api.resolve_context(&f.selector("main")).unwrap();
    let r = f.apply(&p);
    assert_eq!(r.state, ReconcileState::Applied);
    assert_eq!(r.diagnostics["errors"], 0);
    assert_ne!(r.result_spec_oid.as_ref(), before.spec_revision.as_ref());
    assert_eq!(f.apply(&p).result_spec_oid, r.result_spec_oid);
    assert_eq!(
        f.api
            .reconciliation_status(&f.project, &r.id)
            .unwrap()
            .state,
        ReconcileState::Applied
    );
    assert_eq!(snapshot(&f.code), code);
}
#[test]
fn spec_projects_mcp_6_conflict_read_resolve_dual_tokens_and_exact_manifest() {
    let f = Fixture::new();
    f.write("main", "spec/lessons.md", "# Lessons\n\nBase.\n");
    f.branch("topic");
    f.write("main", "spec/lessons.md", "# Lessons\n\nTarget.\n");
    f.write("topic", "spec/lessons.md", "# Lessons\n\nSource.\n");
    let code = snapshot(&f.code);
    let p = f.plan();
    let r = f.apply(&p);
    assert_eq!(r.state, ReconcileState::Conflict);
    assert_eq!(
        f.api
            .resolve_context(&f.selector("main"))
            .unwrap()
            .spec_revision
            .as_deref(),
        Some(p.target_spec_oid.as_str())
    );
    let c = f
        .api
        .read_reconciliation_conflict(&f.project, &r.id, "spec/lessons.md")
        .unwrap();
    assert_eq!(c.base.as_deref(), Some("# Lessons\n\nBase.\n"));
    assert_eq!(c.source.as_deref(), Some("# Lessons\n\nSource.\n"));
    assert_eq!(c.target.as_deref(), Some("# Lessons\n\nTarget.\n"));
    let request = ResolveRequest {
        project: f.project.clone(),
        record_id: r.id.clone(),
        expected_target_revision: c.target_revision.clone(),
        expected_resolution_revision: c.resolution_revision.clone(),
        files: vec![ResolutionFile {
            path: "spec/lessons.md".into(),
            content: Some("# Lessons\n\nResolved.\n".into()),
        }],
    };
    let mut bad = request.clone();
    bad.files.push(ResolutionFile {
        path: "spec/other.md".into(),
        content: None,
    });
    assert_eq!(
        f.api.resolve_reconciliation(bad).unwrap_err().code,
        "invalid_resolution"
    );
    let done = f.api.resolve_reconciliation(request).unwrap();
    assert_eq!(done.state, ReconcileState::Applied);
    assert_eq!(
        f.api
            .read_document(&f.selector("main"), "spec/lessons.md")
            .unwrap()
            .document
            .content,
        "# Lessons\n\nResolved.\n"
    );
    assert_eq!(snapshot(&f.code), code);
}
#[test]
fn spec_projects_mcp_6_stale_source_and_target_refuse() {
    for source in [false, true] {
        let f = Fixture::new();
        f.branch("topic");
        f.write("topic", "spec/lessons.md", "# Lessons\n\nsource\n");
        let p = f.plan();
        f.write(
            if source { "topic" } else { "main" },
            "spec/lessons.md",
            "# Lessons\n\nLater\n",
        );
        assert_eq!(
            f.api
                .apply_reconciliation(&p, &p.target_revision)
                .unwrap_err()
                .code,
            "revision_changed"
        );
    }
}
#[test]
fn spec_projects_mcp_6_cli_preview_is_pure_and_requires_expected_on_apply() {
    let f = Fixture::new();
    f.branch("topic");
    f.write("topic", "spec/lessons.md", "# Lessons\n\nsource\n");
    let runtime = Runtime {
        cwd: f.code.clone(),
        env: BTreeMap::from([
            ("PATH".into(), std::env::var_os("PATH").unwrap_or_default()),
            ("HOME".into(), f._owner.as_os_str().to_owned()),
            ("VIBE_STORE".into(), f._owner.join("store").into_os_string()),
        ]),
    };
    let before = snapshot(&f._owner.join("store"));
    let run = |tail: &[&str]| {
        let argv: Vec<_> = std::iter::once("vibe-spec")
            .chain(tail.iter().copied())
            .map(std::ffi::OsString::from)
            .collect();
        vibe_spec::run_cli(&argv, &runtime, &mut std::io::empty())
    };
    let out = run(&[
        "reconcile",
        "--from",
        "topic",
        "--into",
        "main",
        "--dry-run",
    ]);
    assert_eq!(out.exit, 0, "{}", String::from_utf8_lossy(&out.stderr));
    assert_eq!(snapshot(&f._owner.join("store")), before);
    let out = run(&["reconcile", "--from", "topic", "--into", "main"]);
    assert_eq!(out.exit, 1);
    assert!(String::from_utf8_lossy(&out.stderr).contains("expected_revision_required"));
}
fn request(
    f: &Fixture,
    r: &vibe_spec::ReconcileRecord,
    files: Vec<ResolutionFile>,
) -> ResolveRequest {
    let c = f
        .api
        .read_reconciliation_conflict(&f.project, &r.id, &r.conflicts[0])
        .unwrap();
    ResolveRequest {
        project: f.project.clone(),
        record_id: r.id.clone(),
        expected_target_revision: c.target_revision,
        expected_resolution_revision: c.resolution_revision,
        files,
    }
}
#[test]
fn spec_projects_mcp_6_conflicts_reject_missing_duplicate_escape_and_stale_workspace() {
    let f = Fixture::new();
    f.write("main", "spec/lessons.md", "# Lessons\n\nbase\n");
    f.branch("topic");
    f.write("main", "spec/lessons.md", "# Lessons\n\ntarget\n");
    f.write("topic", "spec/lessons.md", "# Lessons\n\nsource\n");
    let r = f.apply(&f.plan());
    let file = ResolutionFile {
        path: "spec/lessons.md".into(),
        content: Some("# Lessons\n".into()),
    };
    for files in [
        vec![],
        vec![file.clone(), file.clone()],
        vec![ResolutionFile {
            path: "spec/../escape.md".into(),
            content: None,
        }],
    ] {
        assert_eq!(
            f.api
                .resolve_reconciliation(request(&f, &r, files))
                .unwrap_err()
                .code,
            "invalid_resolution"
        )
    }
    let stale = request(&f, &r, vec![file]);
    let workspace = f
        ._owner
        .join("store/spec/projects")
        .join(f.project.as_str())
        .join("reconciliations")
        .join(r.id.as_str())
        .join("workspace");
    fs::write(
        workspace.join("spec/lessons.md"),
        b"external resolution bytes\n",
    )
    .unwrap();
    assert_eq!(
        f.api.resolve_reconciliation(stale).unwrap_err().code,
        "revision_changed"
    );
    let fresh = request(
        &f,
        &r,
        vec![ResolutionFile {
            path: "spec/lessons.md".into(),
            content: Some("# Lessons\n\nresolved\n".into()),
        }],
    );
    f.write("main", "spec/design.md", "# Design\n\nadvanced\n");
    assert_eq!(
        f.api.resolve_reconciliation(fresh).unwrap_err().code,
        "revision_changed"
    );
    let refreshed = request(
        &f,
        &r,
        vec![ResolutionFile {
            path: "spec/lessons.md".into(),
            content: Some("# Lessons\n\nresolved\n".into()),
        }],
    );
    assert_eq!(
        f.api.resolve_reconciliation(refreshed).unwrap_err().code,
        "revision_changed"
    );
}
#[test]
fn spec_projects_mcp_6_dirty_source_target_and_missing_merge_base() {
    for branch in ["main", "topic"] {
        let f = Fixture::new();
        f.branch("topic");
        let c = f.api.resolve_context(&f.selector(branch)).unwrap();
        fs::write(c.spec_root.join("lessons.md"), b"raw edit\n").unwrap();
        assert_eq!(
            f.api
                .plan_reconciliation(ReconcileRequest {
                    selector: f.selector("main"),
                    source: "topic".into(),
                    target: "main".into()
                })
                .unwrap_err()
                .code,
            "dirty_tree"
        );
    }
    let f = Fixture::new();
    f.branch("topic");
    let project = f
        ._owner
        .join("store/spec/projects")
        .join(f.project.as_str());
    let history = project.join("history.git");
    let main = f
        .api
        .resolve_context(&f.selector("main"))
        .unwrap()
        .spec_revision
        .unwrap();
    let tree = common::git_out(&history, &["rev-parse", &format!("{main}^{{tree}}")]);
    let orphan = common::git_out(&history, &["commit-tree", &tree, "-m", "unrelated root"]);
    let show = f
        .api
        .execute(SpecRequest::ProjectShow {
            selector: f.selector("main"),
        })
        .unwrap();
    let source_ref = show.data["branches"]
        .as_array()
        .unwrap()
        .iter()
        .find(|b| b["code_ref"] == "refs/heads/topic")
        .unwrap()["spec_ref"]
        .as_str()
        .unwrap();
    common::git(
        &history,
        "2026-01-01T00:00:00Z",
        &["update-ref", source_ref, &orphan],
    );
    let before = f.api.resolve_context(&f.selector("main")).unwrap();
    let r = f.apply(&f.plan());
    assert_eq!(r.state, ReconcileState::NeedsInput);
    assert_eq!(
        f.api
            .resolve_context(&f.selector("main"))
            .unwrap()
            .spec_revision,
        before.spec_revision
    );
}
#[test]
fn spec_projects_mcp_6_delete_change_and_rename_rename_manifests_resolve_by_bundle() {
    for rename in [false, true] {
        let f = Fixture::new();
        f.write("main", "spec/lessons.md", "# Lessons\n\nbase\n");
        f.branch("topic");
        let remove = |branch: &str| {
            let selector = f.selector(branch);
            let c = f.api.resolve_context(&selector).unwrap();
            f.api
                .execute(SpecRequest::RemoveDocument(
                    vibe_spec::RemoveDocumentRequest {
                        selector,
                        path: "spec/lessons.md".into(),
                        expected_revision: c.revision_token,
                    },
                ))
                .unwrap();
        };
        remove("topic");
        if rename {
            f.write("topic", "spec/source-name.md", "# Lessons\n\nbase\n");
            remove("main");
            f.write("main", "spec/target-name.md", "# Lessons\n\nbase\n");
        } else {
            f.write("main", "spec/lessons.md", "# Lessons\n\nchanged\n");
        }
        let p = f.plan();
        let r = f.apply(&p);
        assert_eq!(r.state, ReconcileState::Conflict);
        assert_eq!(r.conflicts.len(), if rename { 3 } else { 1 });
        for path in &r.conflicts {
            f.api
                .read_reconciliation_conflict(&f.project, &r.id, path)
                .unwrap();
        }
        let files = r
            .conflicts
            .iter()
            .map(|p| ResolutionFile {
                path: p.clone(),
                content: None,
            })
            .collect();
        let done = f
            .api
            .resolve_reconciliation(request(&f, &r, files))
            .unwrap();
        assert_eq!(done.state, ReconcileState::Applied);
        assert!(
            !f.api
                .resolve_context(&f.selector("main"))
                .unwrap()
                .spec_root
                .join("lessons.md")
                .exists()
        );
    }
}
#[test]
fn spec_projects_mcp_6_completed_validation_refuses_clean_and_conflicting_invalid_trees() {
    let f = Fixture::new();
    f.branch("topic");
    f.write(
        "topic",
        "spec/product.md",
        "---\ntype: invalid\n---\n# Product\n",
    );
    let p = f.plan();
    let r = f.apply(&p);
    assert_eq!(r.state, ReconcileState::NeedsInput);
    assert_eq!(
        f.api
            .resolve_context(&f.selector("main"))
            .unwrap()
            .spec_revision
            .as_deref(),
        Some(p.target_spec_oid.as_str())
    );
    let f = Fixture::new();
    let valid_product = f
        .api
        .read_document(&f.selector("main"), "spec/product.md")
        .unwrap()
        .document
        .content;
    f.branch("topic");
    f.write("topic", "spec/product.md", "# Product\n\nsource\n");
    f.write("main", "spec/product.md", "# Product\n\ntarget\n");
    let p = f.plan();
    let r = f.apply(&p);
    assert_eq!(r.state, ReconcileState::Conflict);
    let invalid = request(
        &f,
        &r,
        vec![ResolutionFile {
            path: "spec/product.md".into(),
            content: Some("---\ntype: invalid\n---\n# Product\n".into()),
        }],
    );
    let r = f.api.resolve_reconciliation(invalid).unwrap();
    assert_eq!(r.state, ReconcileState::NeedsInput);
    assert_eq!(
        f.api
            .resolve_context(&f.selector("main"))
            .unwrap()
            .spec_revision
            .as_deref(),
        Some(p.target_spec_oid.as_str())
    );
    let valid = request(
        &f,
        &r,
        vec![ResolutionFile {
            path: "spec/product.md".into(),
            content: Some(valid_product),
        }],
    );
    assert_eq!(
        f.api.resolve_reconciliation(valid).unwrap().state,
        ReconcileState::Applied
    );
}
fn done(f: &Fixture, feature: &str) -> Result<vibe_spec::SpecResponse, vibe_spec::SpecError> {
    let selector = f.selector("main");
    let c = f.api.resolve_context(&selector).unwrap();
    f.api
        .execute(SpecRequest::Promote(vibe_spec::PromoteRequest {
            selector,
            feature: feature.into(),
            expected_revision: c.revision_token,
            dry_run: false,
            done: true,
            target: None,
            base: None,
        }))
}
#[test]
fn spec_projects_mcp_6_source_lifecycle_is_inherited_and_same_name_reuse_needs_new_durable_proof() {
    let f = Fixture::new();
    f.branch("topic");
    let baseline = f
        .api
        .resolve_context(&f.selector("topic"))
        .unwrap()
        .spec_revision
        .unwrap();
    let artifact = "superpowers/specs/2026-01-15-inherited-design.md";
    f.write("topic", artifact, "# Inherited\n");
    f.write("topic", "spec/lessons.md", "# Lessons\n\nDurable source.\n");
    let p = f.plan();
    let r = f.apply(&p);
    assert_eq!(r.state, ReconcileState::Applied);
    f.write(
        "main",
        "superpowers/plans/2026-01-15-inherited.md",
        "# Plan\n\n**Spec:** `../specs/2026-01-15-inherited-design.md`\n",
    );
    let completed = done(&f, "inherited").unwrap();
    assert_eq!(completed.data["baseline"], baseline);
    f.write("main", artifact, "# Inherited again\n");
    assert_eq!(
        done(&f, "inherited").unwrap_err().code,
        "promotion_required"
    );
    f.write(
        "main",
        "spec/lessons.md",
        "# Lessons\n\nA new durable fact.\n",
    );
    done(&f, "inherited").unwrap();
    let history = f
        ._owner
        .join("store/spec/projects")
        .join(f.project.as_str())
        .join("history.git");
    let parents = common::git_out(
        &history,
        &[
            "rev-list",
            "--parents",
            "--max-count=1",
            r.result_spec_oid.as_deref().unwrap(),
        ],
    );
    assert_eq!(
        parents,
        format!(
            "{} {} {}",
            r.result_spec_oid.unwrap(),
            p.target_spec_oid,
            p.source_spec_oid
        )
    );
}
#[test]
fn spec_projects_mcp_6_ambiguous_lifecycle_keeps_authoring_but_refuses_completion() {
    let f = Fixture::new();
    f.branch("topic");
    let artifact = "superpowers/specs/2026-01-15-shared-design.md";
    f.write("topic", artifact, "# Shared\n");
    f.write(
        "main",
        "spec/lessons.md",
        "# Lessons\n\nA durable target fact.\n",
    );
    f.write("main", artifact, "# Shared\n");
    let r = f.apply(&f.plan());
    assert_eq!(r.state, ReconcileState::Applied);
    assert_eq!(done(&f, "shared").unwrap_err().code, "ambiguous_evidence");
    let selector = f.selector("main");
    let c = f.api.resolve_context(&selector).unwrap();
    let response = f
        .api
        .execute(SpecRequest::WriteDocument(
            vibe_spec::WriteDocumentRequest {
                selector,
                path: "spec/lessons.md".into(),
                content: "# Lessons\n\nAllowed authoring.\n".into(),
                expected_revision: c.revision_token,
            },
        ))
        .unwrap();
    assert_eq!(
        response.data["lifecycle_diagnostics"][0]["code"],
        "ambiguous_evidence"
    );
    assert_eq!(done(&f, "shared").unwrap_err().code, "ambiguous_evidence");
}
#[test]
fn spec_projects_mcp_6_saved_cli_plan_refuses_source_advance_and_selection_overrides() {
    let f = Fixture::new();
    f.branch("topic");
    f.write("topic", "spec/lessons.md", "# Lessons\n\nSource.\n");
    let runtime = Runtime {
        cwd: f.code.clone(),
        env: BTreeMap::from([
            ("PATH".into(), std::env::var_os("PATH").unwrap_or_default()),
            ("HOME".into(), f._owner.as_os_str().to_owned()),
            ("VIBE_STORE".into(), f._owner.join("store").into_os_string()),
        ]),
    };
    let run = |tail: &[&str], bytes: &[u8]| {
        let argv: Vec<_> = std::iter::once("vibe-spec")
            .chain(tail.iter().copied())
            .map(std::ffi::OsString::from)
            .collect();
        vibe_spec::run_cli(&argv, &runtime, &mut std::io::Cursor::new(bytes))
    };
    let preview = run(
        &[
            "reconcile",
            "--from",
            "topic",
            "--into",
            "main",
            "--dry-run",
        ],
        b"",
    );
    assert_eq!(preview.exit, 0);
    let value: serde_json::Value = serde_json::from_slice(&preview.stdout).unwrap();
    let token = value["target_revision"].as_str().unwrap();
    let out = run(
        &[
            "reconcile",
            "--input",
            "-",
            "--expect",
            token,
            "--project",
            f.project.as_str(),
        ],
        &preview.stdout,
    );
    assert_eq!(out.exit, 1);
    assert!(String::from_utf8_lossy(&out.stderr).contains("invalid_request"));
    f.write("topic", "spec/lessons.md", "# Lessons\n\nNew source.\n");
    let out = run(
        &["reconcile", "--input", "-", "--expect", token],
        &preview.stdout,
    );
    assert_eq!(out.exit, 1);
    assert!(String::from_utf8_lossy(&out.stderr).contains("revision_changed"));
}
#[test]
fn spec_projects_mcp_6_nonoverlapping_add_delete_retains_artifacts_and_deleted_code_branch() {
    let f = Fixture::new();
    f.write("main", "spec/lessons.md", "# Lessons\n\nBase.\n");
    common::git(&f.code, "2026-01-01T00:00:00Z", &["branch", "topic"]);
    f.branch("topic");
    common::git(&f.code, "2026-01-01T00:00:00Z", &["checkout", "topic"]);
    f.write(
        "topic",
        "superpowers/specs/2026-01-15-retained-design.md",
        "# Retained\n",
    );
    common::git(&f.code, "2026-01-01T00:00:00Z", &["checkout", "main"]);
    common::git(&f.code, "2026-01-01T00:00:00Z", &["branch", "-D", "topic"]);
    let selector = f.selector("main");
    let context = f.api.resolve_context(&selector).unwrap();
    f.api
        .execute(SpecRequest::RemoveDocument(
            vibe_spec::RemoveDocumentRequest {
                selector,
                path: "spec/lessons.md".into(),
                expected_revision: context.revision_token,
            },
        ))
        .unwrap();
    let code = snapshot(&f.code);
    assert!(
        !f.api
            .resolve_context(&f.selector("topic"))
            .unwrap()
            .code_available
    );
    let r = f.apply(&f.plan());
    assert_eq!(r.state, ReconcileState::Applied);
    let c = f.api.resolve_context(&f.selector("main")).unwrap();
    assert!(!c.spec_root.join("lessons.md").exists());
    assert_eq!(
        f.api
            .read_document(
                &f.selector("main"),
                "superpowers/specs/2026-01-15-retained-design.md"
            )
            .unwrap()
            .document
            .content,
        "# Retained\n"
    );
    assert_eq!(snapshot(&f.code), code);
}
#[test]
fn spec_projects_mcp_6_explicit_checkout_preview_and_exact_apply_work_outside_checkout() {
    let f = Fixture::new();
    f.branch("topic");
    f.write("topic", "spec/lessons.md", "# Lessons\n\nSource.\n");
    let api = SpecApi::new(Runtime {
        cwd: f._owner.to_path_buf(),
        env: BTreeMap::from([
            ("PATH".into(), std::env::var_os("PATH").unwrap_or_default()),
            ("HOME".into(), f._owner.as_os_str().to_owned()),
            ("VIBE_STORE".into(), f._owner.join("store").into_os_string()),
        ]),
    })
    .unwrap();
    let p = api
        .plan_reconciliation(ReconcileRequest {
            selector: ContextSelector {
                checkout: Some(f.code.clone()),
                ..Default::default()
            },
            source: "topic".into(),
            target: "main".into(),
        })
        .unwrap();
    assert_eq!(
        api.apply_reconciliation(&p, &p.target_revision)
            .unwrap()
            .state,
        ReconcileState::Applied
    );
}
#[test]
fn spec_projects_mcp_6_applied_receipt_remains_idempotent_after_code_checkout_moves() {
    let f = Fixture::new();
    f.branch("topic");
    f.write("topic", "spec/lessons.md", "# Lessons\n\nSource.\n");
    let p = f.plan();
    let r = f.apply(&p);
    common::git(
        &f.code,
        "2026-01-01T00:00:00Z",
        &["checkout", "-b", "later-code"],
    );
    let code = snapshot(&f.code);
    assert_eq!(f.apply(&p).result_spec_oid, r.result_spec_oid);
    assert_eq!(snapshot(&f.code), code);
}
#[test]
fn spec_projects_mcp_6_fresh_workspace_token_does_not_admit_outside_manifest_edits() {
    for staged in [false, true] {
        let f = Fixture::new();
        f.write("main", "spec/lessons.md", "# Lessons\n\nbase\n");
        f.branch("topic");
        f.write("main", "spec/lessons.md", "# Lessons\n\ntarget\n");
        f.write("topic", "spec/lessons.md", "# Lessons\n\nsource\n");
        let p = f.plan();
        let r = f.apply(&p);
        let root = f
            ._owner
            .join("store/spec/projects")
            .join(f.project.as_str());
        let workspace = root
            .join("reconciliations")
            .join(r.id.as_str())
            .join("workspace");
        let original = fs::read(workspace.join("spec/design.md")).unwrap();
        let mut changed = original.clone();
        changed.extend_from_slice(b"\nUnrelated raw edit.\n");
        fs::write(workspace.join("spec/design.md"), &changed).unwrap();
        if staged {
            common::git(
                &workspace,
                "2026-01-01T00:00:00Z",
                &["add", "spec/design.md"],
            );
        }
        let before = f.api.resolve_context(&f.selector("main")).unwrap();
        let fresh = request(
            &f,
            &r,
            vec![ResolutionFile {
                path: "spec/lessons.md".into(),
                content: Some("# Lessons\n\nresolved\n".into()),
            }],
        );
        assert_eq!(
            f.api.resolve_reconciliation(fresh).unwrap_err().code,
            "invalid_resolution"
        );
        assert_eq!(
            f.api
                .resolve_context(&f.selector("main"))
                .unwrap()
                .spec_revision,
            before.spec_revision
        );
        assert_eq!(fs::read(workspace.join("spec/design.md")).unwrap(), changed);
    }
}
fn outside_api(f: &Fixture) -> SpecApi {
    SpecApi::new(Runtime {
        cwd: f._owner.to_path_buf(),
        env: BTreeMap::from([
            ("PATH".into(), std::env::var_os("PATH").unwrap_or_default()),
            ("HOME".into(), f._owner.as_os_str().to_owned()),
            ("VIBE_STORE".into(), f._owner.join("store").into_os_string()),
        ]),
    })
    .unwrap()
}
fn second_clone(f: &Fixture) -> std::path::PathBuf {
    let clone = f._owner.join("clone");
    common::git(
        &f._owner,
        "2026-01-01T00:00:00Z",
        &["clone", f.code.to_str().unwrap(), clone.to_str().unwrap()],
    );
    let revision = f.api.execute(SpecRequest::Projects).unwrap().data["revision"]
        .as_str()
        .unwrap()
        .to_owned();
    f.api
        .execute(SpecRequest::ProjectAttach(
            vibe_spec::AttachProjectRequest {
                project: f.project.clone(),
                checkout: Some(clone.clone()),
                main: false,
                expected_revision: revision,
            },
        ))
        .unwrap();
    clone
}
#[test]
fn spec_projects_mcp_6_review_captured_checkout_survives_two_clones_and_outside_client() {
    let f = Fixture::new();
    f.branch("topic");
    second_clone(&f);
    f.write("main", "spec/lessons.md", "# Lessons\n\nBase.\n");
    f.write("topic", "spec/lessons.md", "# Lessons\n\nSource.\n");
    let p = f.plan();
    let outside = outside_api(&f);
    let r = outside
        .apply_reconciliation(&p, &p.target_revision)
        .unwrap();
    assert_eq!(r.state, ReconcileState::Conflict);
    assert_eq!(
        outside
            .reconciliation_status(&f.project, &r.id)
            .unwrap()
            .state,
        ReconcileState::Conflict
    );
    let c = outside
        .read_reconciliation_conflict(&f.project, &r.id, "spec/lessons.md")
        .unwrap();
    let r = outside
        .resolve_reconciliation(ResolveRequest {
            project: f.project.clone(),
            record_id: r.id,
            expected_target_revision: c.target_revision,
            expected_resolution_revision: c.resolution_revision,
            files: vec![ResolutionFile {
                path: "spec/lessons.md".into(),
                content: Some("# Lessons\n\nResolved.\n".into()),
            }],
        })
        .unwrap();
    assert_eq!(r.state, ReconcileState::Applied);
    common::git(
        &f.code,
        "2026-01-01T00:00:00Z",
        &["checkout", "-b", "later-code"],
    );
    assert_eq!(
        outside
            .reconciliation_status(&f.project, &r.id)
            .unwrap()
            .state,
        ReconcileState::Applied
    );
    assert_eq!(
        outside
            .apply_reconciliation(&p, &p.target_revision)
            .unwrap()
            .result_spec_oid,
        r.result_spec_oid
    );
}
#[test]
fn spec_projects_mcp_6_review_conflict_cannot_move_to_another_or_missing_checkout() {
    for clone in [false, true] {
        let f = Fixture::new();
        f.branch("topic");
        if clone {
            second_clone(&f);
        }
        f.write("main", "spec/lessons.md", "# Lessons\n\nTarget.\n");
        f.write("topic", "spec/lessons.md", "# Lessons\n\nSource.\n");
        let p = f.plan();
        let r = f.apply(&p);
        assert_eq!(r.state, ReconcileState::Conflict);
        common::git(
            &f.code,
            "2026-01-01T00:00:00Z",
            &["checkout", "-b", "later-code"],
        );
        let outside = outside_api(&f);
        assert_eq!(
            outside
                .read_reconciliation_conflict(&f.project, &r.id, "spec/lessons.md")
                .unwrap_err()
                .code,
            "context_mismatch"
        );
        let history = f
            ._owner
            .join("store/spec/projects")
            .join(f.project.as_str())
            .join("history.git");
        let show = f
            .api
            .execute(SpecRequest::ProjectShow {
                selector: ContextSelector {
                    project: Some(f.project.clone()),
                    ..Default::default()
                },
            })
            .unwrap();
        let target_ref = show.data["branches"]
            .as_array()
            .unwrap()
            .iter()
            .find(|b| b["id"] == p.target.as_str())
            .unwrap()["spec_ref"]
            .as_str()
            .unwrap();
        assert_eq!(
            common::git_out(&history, &["rev-parse", target_ref]),
            p.target_spec_oid
        );
    }
}
#[test]
fn spec_projects_mcp_6_review_same_checkout_branch_head_can_advance_with_fresh_resolution_tokens() {
    let f = Fixture::new();
    f.branch("topic");
    f.write("main", "spec/lessons.md", "# Lessons\n\nTarget.\n");
    f.write("topic", "spec/lessons.md", "# Lessons\n\nSource.\n");
    let r = f.apply(&f.plan());
    common::git(
        &f.code,
        "2026-01-01T00:00:00Z",
        &[
            "commit",
            "--allow-empty",
            "-m",
            "new code on same selected branch",
        ],
    );
    let code = snapshot(&f.code);
    let fresh = request(
        &f,
        &r,
        vec![ResolutionFile {
            path: "spec/lessons.md".into(),
            content: Some("# Lessons\n\nResolved.\n".into()),
        }],
    );
    assert_eq!(
        f.api.resolve_reconciliation(fresh).unwrap().state,
        ReconcileState::Applied
    );
    assert_eq!(snapshot(&f.code), code);
}
#[test]
fn spec_projects_mcp_6_review_cli_record_selectors_cannot_override_captured_checkout_or_branch() {
    let f = Fixture::new();
    f.branch("topic");
    let clone = second_clone(&f);
    f.write("main", "spec/lessons.md", "# Lessons\n\nTarget.\n");
    f.write("topic", "spec/lessons.md", "# Lessons\n\nSource.\n");
    let r = f.apply(&f.plan());
    for selector in [
        ["--checkout", clone.to_str().unwrap()],
        ["--branch", "topic"],
    ] {
        let out = common::spec(
            &f.code,
            &[
                "reconcile",
                "read",
                r.id.as_str(),
                "spec/lessons.md",
                "--project",
                f.project.as_str(),
                selector[0],
                selector[1],
            ],
            &[("VIBE_STORE", f._owner.join("store").to_str().unwrap())],
        );
        assert_eq!(out.code, 1, "{}", out.stderr);
        assert!(out.stderr.contains("context_mismatch"), "{}", out.stderr);
    }
}
