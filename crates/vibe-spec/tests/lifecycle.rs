#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Complete context lifecycle controls (spec-projects-mcp/5).
mod common;
use std::{collections::BTreeMap, path::Path};
use vibe_spec::{ContextSelector, Runtime, SpecApi, SpecRequest, ValidationScope};
fn api(code: &Path, store: &Path) -> SpecApi {
    SpecApi::new(Runtime {
        cwd: code.into(),
        env: BTreeMap::from([
            ("PATH".into(), std::env::var_os("PATH").unwrap_or_default()),
            ("HOME".into(), code.as_os_str().to_owned()),
            ("VIBE_STORE".into(), store.as_os_str().to_owned()),
        ]),
    })
    .unwrap()
}
#[test]
fn spec_projects_mcp_5_typed_lifecycle_in_both_modes() {
    for global in [false, true] {
        let owner = common::project("lifecycle");
        let code = owner.join("code");
        std::fs::create_dir(&code).unwrap();
        common::git(&code, "2026-01-01T00:00:00Z", &["init", "-b", "main"]);
        common::git(
            &code,
            "2026-01-01T00:00:00Z",
            &["commit", "--allow-empty", "-m", "initial"],
        );
        let store = owner.join("store");
        let api = api(&code, &store);
        api.execute(SpecRequest::ProjectInit(vibe_spec::InitProjectRequest {
            selector: ContextSelector::default(),
            mode: if global {
                vibe_spec::StorageMode::GlobalFirst
            } else {
                vibe_spec::StorageMode::RepoFirst
            },
            name: None,
            expected_revision: "absent".into(),
        }))
        .unwrap();
        let result = api
            .execute(SpecRequest::Validate(vibe_spec::ValidateRequest {
                selector: ContextSelector::default(),
                scope: ValidationScope::SpecOnly,
                strict: false,
            }))
            .unwrap();
        assert_eq!(result.data["errors"], 0, "{}", result.data);
        assert!(
            result.data["diagnostics"]
                .as_array()
                .unwrap()
                .iter()
                .any(|d| d["code"] == "code_checks_skipped")
        );
    }
}

struct Fixture {
    _owner: vibe_core::testscratch::Scratch,
    code: std::path::PathBuf,
    store: std::path::PathBuf,
    api: SpecApi,
    global: bool,
    before: BTreeMap<String, Vec<u8>>,
}
fn snapshot(root: &Path) -> BTreeMap<String, Vec<u8>> {
    fn walk(root: &Path, dir: &Path, out: &mut BTreeMap<String, Vec<u8>>) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let entry = entry.unwrap();
            let p = entry.path();
            if entry.file_type().unwrap().is_dir() {
                walk(root, &p, out);
            } else {
                out.insert(
                    p.strip_prefix(root)
                        .unwrap()
                        .components()
                        .map(|part| part.as_os_str().to_str().unwrap())
                        .collect::<Vec<_>>()
                        .join("/"),
                    std::fs::read(p).unwrap(),
                );
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(root, root, &mut out);
    out
}
impl Fixture {
    fn new(global: bool) -> Self {
        let owner = common::project("complete-lifecycle");
        let code = owner.join("code");
        let store = owner.join("store");
        std::fs::create_dir(&code).unwrap();
        common::write(&code, "src/billing.rs", "// billing\n");
        common::write(&code, "tests/citations.rs", "// checkout/1\n");
        common::git(&code, "2026-01-01T00:00:00Z", &["init", "-b", "main"]);
        common::git(&code, "2026-01-01T00:00:00Z", &["add", "."]);
        common::git(&code, "2026-01-01T00:00:00Z", &["commit", "-m", "initial"]);
        let api = api(&code, &store);
        api.execute(SpecRequest::ProjectInit(vibe_spec::InitProjectRequest {
            selector: ContextSelector::default(),
            mode: if global {
                vibe_spec::StorageMode::GlobalFirst
            } else {
                vibe_spec::StorageMode::RepoFirst
            },
            name: None,
            expected_revision: "absent".into(),
        }))
        .unwrap();
        let before = snapshot(&code);
        Self {
            _owner: owner,
            code,
            store,
            api,
            global,
            before,
        }
    }
    fn selector(&self) -> ContextSelector {
        ContextSelector::default()
    }
    fn write(&self, path: &str, text: &str) {
        let context = self.api.resolve_context(&self.selector()).unwrap();
        self.api
            .execute(SpecRequest::WriteDocument(
                vibe_spec::WriteDocumentRequest {
                    selector: self.selector(),
                    path: path.into(),
                    content: text.into(),
                    expected_revision: context.revision_token,
                },
            ))
            .unwrap();
    }
    fn root_fixture(&self) {
        let source = common::layered_tree("lifecycle-source");
        for (path, bytes) in snapshot(&source.join("docs/spec")) {
            self.write(
                &format!("spec/{path}"),
                std::str::from_utf8(&bytes).unwrap(),
            );
        }
        self.write("spec/lessons.md","# Lessons\n\n### Preserve evidence\n**Rule:** Record the source.\n**Tags:** lifecycle\n**Date:** 2026-01-15\n");
        if !self.global {
            common::git(&self.code, "2026-01-01T00:00:00Z", &["add", "docs"]);
            common::git(
                &self.code,
                "2026-01-01T00:00:00Z",
                &["commit", "-m", "base specs"],
            );
            common::git(
                &self.code,
                "2026-01-01T00:00:00Z",
                &["checkout", "-b", "feature"],
            );
        }
    }
    fn feature_fixture(&self) {
        self.write("spec/features/checkout/product.md","---\ntype: feature-product\nfeature: checkout\nsibling: tech.md\nparent: ../../product.md\nupdated: 2026-01-15\n---\n\n# Checkout\n\n## Scope\n\n| Owns | Does not own |\n|---|---|\n| Checkout | Billing |\n\n## ADDED Requirements\n\n### Requirement: Confirmation R1\nThe system MUST confirm checkout.\n\n#### Scenario: Confirmed\n- GIVEN a cart\n- WHEN the user checks out\n- THEN the system confirms\n");
        self.write("spec/features/checkout/tech.md","---\ntype: feature-tech\nfeature: checkout\nsibling: product.md\nparent: ../../tech.md\nupdated: 2026-01-15\n---\n\n# Checkout tech\n\n<!-- merge -->\nCheckout confirms an accepted cart.\n<!-- /merge -->\n");
        self.write("spec/features/checkout/plan.md","---\ntype: feature-plan\nfeature: checkout\nparent: ../../plan.md\nupdated: 2026-01-15\n---\n\n# Plan\n\n| Unit | Status |\n|---|---|\n| checkout/1 | DONE |\n\n### checkout/1 — Confirm checkout\n\n**Files:** tests/citations.rs\n**What to do:** Confirm checkout.\n**Verify:** Run tests.\n\n## Requirements Trace\n\n| Requirement | Unit |\n|---|---|\n| R1 | checkout/1 |\n");
        self.write(
            "superpowers/specs/2026-01-15-checkout-design.md",
            "# Checkout design\n\nConfirm accepted carts.\n",
        );
        self.write("superpowers/plans/2026-01-15-checkout.md","# Checkout plan\n\n**Spec:** `docs/superpowers/specs/2026-01-15-checkout-design.md`\n\n### Task 1: Confirm checkout\n\n- [x] Implement checkout/1.\n");
    }
    fn assert_code_unchanged(&self) {
        if self.global {
            assert_eq!(
                snapshot(&self.code),
                self.before,
                "global lifecycle modified code repo bytes"
            );
        }
    }
}
#[test]
fn complete_feature_journey_uses_selected_anchors_in_both_modes() {
    for global in [false, true] {
        let f = Fixture::new(global);
        f.root_fixture();
        let context = f.api.resolve_context(&f.selector()).unwrap();
        f.api
            .execute(SpecRequest::Init(vibe_spec::InitRequest {
                selector: f.selector(),
                expected_revision: context.revision_token,
            }))
            .unwrap();
        f.feature_fixture();
        let feature = f
            .api
            .execute(SpecRequest::Feature(vibe_spec::FeatureRequest {
                selector: f.selector(),
                feature: "checkout".into(),
            }))
            .unwrap();
        assert_eq!(feature.data["layer"], "superpowers");
        let plan = f
            .api
            .execute(SpecRequest::Plan(vibe_spec::PlanRequest {
                selector: f.selector(),
                feature: "checkout".into(),
            }))
            .unwrap();
        assert_eq!(plan.data["rows"][0]["status"], "done");
        let lessons = f
            .api
            .execute(SpecRequest::Lessons(vibe_spec::LessonsRequest {
                selector: f.selector(),
                tags: vec!["lifecycle".into()],
            }))
            .unwrap();
        assert_eq!(lessons.data["lessons"][0]["title"], "Preserve evidence");
        let validation = f
            .api
            .execute(SpecRequest::Validate(vibe_spec::ValidateRequest {
                selector: f.selector(),
                scope: ValidationScope::WithCode,
                strict: false,
            }))
            .unwrap();
        assert_eq!(validation.data["errors"], 0, "{}", validation.data);
        assert!(
            validation.data["findings"]
                .as_array()
                .unwrap()
                .iter()
                .all(|v| v["check"] != "SF36"),
            "{}",
            validation.data
        );
        let trace = f
            .api
            .execute(SpecRequest::Trace(vibe_spec::TraceRequest {
                selector: f.selector(),
                since: None,
            }))
            .unwrap();
        assert_eq!(trace.data["cited"], 1);
        let drift = f
            .api
            .execute(SpecRequest::Drift(vibe_spec::DriftRequest {
                selector: f.selector(),
                scope: ValidationScope::WithCode,
                pre_merge: false,
            }))
            .unwrap();
        assert_eq!(drift.data["errors"], 0, "{}", drift.data);
        let exported = f
            .api
            .execute(SpecRequest::Export(vibe_spec::ExportRequest {
                selector: f.selector(),
            }))
            .unwrap();
        let bundle = vibe_spec::OpenSpecBundle {
            files: exported.data["files"]
                .as_array()
                .unwrap()
                .iter()
                .map(|file| vibe_spec::BundleFile {
                    path: file["path"].as_str().unwrap().into(),
                    content: file["content"].as_str().unwrap().into(),
                })
                .collect(),
        };
        let target = Fixture::new(global);
        target.root_fixture();
        let context = target.api.resolve_context(&target.selector()).unwrap();
        target
            .api
            .execute(SpecRequest::Import(vibe_spec::ImportRequest {
                selector: target.selector(),
                expected_revision: context.revision_token,
                bundle,
                feature: None,
            }))
            .unwrap();
        assert_eq!(
            f.api
                .read_document(&f.selector(), "spec/features/checkout/product.md")
                .unwrap()
                .document
                .content,
            target
                .api
                .read_document(&target.selector(), "spec/features/checkout/product.md")
                .unwrap()
                .document
                .content
        );
        let mut request = vibe_spec::PromoteRequest {
            selector: f.selector(),
            feature: "checkout".into(),
            expected_revision: String::new(),
            dry_run: true,
            done: false,
            target: None,
            base: if global { None } else { Some("main".into()) },
        };
        let before = f.api.resolve_context(&f.selector()).unwrap();
        f.api
            .execute(SpecRequest::Promote(request.clone()))
            .unwrap();
        assert_eq!(before, f.api.resolve_context(&f.selector()).unwrap());
        request.dry_run = false;
        request.expected_revision = before.revision_token;
        f.api
            .execute(SpecRequest::Promote(request.clone()))
            .unwrap();
        request.done = true;
        request.expected_revision = f.api.resolve_context(&f.selector()).unwrap().revision_token;
        f.api.execute(SpecRequest::Promote(request)).unwrap();
        assert!(
            f.api
                .list_documents(&f.selector())
                .unwrap()
                .documents
                .iter()
                .all(|d| !d.path.contains("checkout") || d.path == "spec/flows/checkout.md")
        );
        f.assert_code_unchanged();
        target.assert_code_unchanged();
    }
}

#[test]
fn global_off_checkout_never_reads_main_code_or_config() {
    let f = Fixture::new(true);
    let first = f
        .api
        .execute(SpecRequest::ProjectShow {
            selector: f.selector(),
        })
        .unwrap();
    f.api
        .execute(SpecRequest::BranchInit(vibe_spec::InitBranchRequest {
            selector: f.selector(),
            name: "stored-only".into(),
            from: "main".into(),
            expected_revision: first.data["revision"].as_str().unwrap().into(),
            expected_project_revision: first.data["project_revision"].as_str().unwrap().into(),
        }))
        .unwrap();
    let project = f.api.resolve_context(&f.selector()).unwrap().project_id;
    // Main's malformed config must not affect the other stored branch.
    common::write(&f.code, ".vibe/spec.json", "this is deliberately not JSON");
    let before = snapshot(&f.code);
    let selector = ContextSelector {
        project,
        checkout: None,
        branch: Some("stored-only".into()),
    };
    let context = f.api.resolve_context(&selector).unwrap();
    assert!(!context.code_available);
    assert!(context.checkout.is_none());
    let valid = f
        .api
        .execute(SpecRequest::Validate(vibe_spec::ValidateRequest {
            selector: selector.clone(),
            scope: ValidationScope::SpecOnly,
            strict: false,
        }))
        .unwrap();
    assert_eq!(valid.data["errors"], 0, "{}", valid.data);
    for request in [
        SpecRequest::Validate(vibe_spec::ValidateRequest {
            selector: selector.clone(),
            scope: ValidationScope::WithCode,
            strict: false,
        }),
        SpecRequest::Trace(vibe_spec::TraceRequest {
            selector: selector.clone(),
            since: None,
        }),
        SpecRequest::Drift(vibe_spec::DriftRequest {
            selector: selector.clone(),
            scope: ValidationScope::WithCode,
            pre_merge: false,
        }),
        SpecRequest::Promote(vibe_spec::PromoteRequest {
            selector: selector.clone(),
            feature: "checkout".into(),
            expected_revision: context.revision_token.clone(),
            dry_run: false,
            done: true,
            target: None,
            base: Some("main".into()),
        }),
    ] {
        assert_eq!(
            f.api.execute(request).unwrap_err().code,
            "code_context_unavailable"
        );
    }
    assert_eq!(snapshot(&f.code), before);
}
#[test]
fn global_completion_uses_durable_spec_evidence_and_old_cli_cannot_bypass() {
    let f = Fixture::new(true);
    f.write(
        "superpowers/specs/2026-01-15-evidence-design.md",
        "# Evidence\n",
    );
    f.write(
        "superpowers/plans/2026-01-15-evidence.md",
        "# Plan\n\n**Spec:** `../specs/2026-01-15-evidence-design.md`\n",
    );
    let context = f.api.resolve_context(&f.selector()).unwrap();
    let request = vibe_spec::PromoteRequest {
        selector: f.selector(),
        feature: "evidence".into(),
        expected_revision: context.revision_token.clone(),
        dry_run: false,
        done: true,
        target: None,
        base: Some("HEAD~0".into()),
    };
    assert_eq!(
        f.api
            .execute(SpecRequest::Promote(request.clone()))
            .unwrap_err()
            .code,
        "promotion_required"
    );
    let cli = common::spec(
        &f.code,
        &["promote", "evidence", "--done", "--base", "HEAD"],
        &[("VIBE_STORE", f.store.to_str().unwrap())],
    );
    assert_eq!(cli.code, 1);
    assert!(
        cli.stderr.contains("expected_revision_required"),
        "{}",
        cli.stderr
    );
    let tech = f
        .api
        .read_document(&f.selector(), "spec/tech.md")
        .unwrap()
        .document
        .content;
    f.write(
        "spec/tech.md",
        &(tech + "\nDurable evidence is recorded atomically.\n"),
    );
    assert_eq!(
        f.api
            .execute(SpecRequest::Promote(request.clone()))
            .unwrap_err()
            .code,
        "revision_changed"
    );
    let context = f.api.resolve_context(&f.selector()).unwrap();
    let cli = common::spec(
        &f.code,
        &[
            "promote",
            "evidence",
            "--done",
            "--base",
            "not-a-code-ref",
            "--expect",
            &context.revision_token,
            "--json",
        ],
        &[("VIBE_STORE", f.store.to_str().unwrap())],
    );
    assert_eq!(cli.code, 0, "{}", cli.stderr);
    assert!(
        f.api
            .list_documents(&f.selector())
            .unwrap()
            .documents
            .iter()
            .all(|d| !d.path.contains("evidence"))
    );
    f.assert_code_unchanged();
}
#[cfg(unix)]
#[test]
fn managed_lint_is_denied_with_structured_diagnostics_and_legacy_opt_in_runs() {
    use std::os::unix::fs::PermissionsExt;
    let f = Fixture::new(true);
    let bin = f._owner.join("bin");
    std::fs::create_dir(&bin).unwrap();
    let marker = f._owner.join("lint-ran");
    for name in ["node", "curl", "npx"] {
        let file = bin.join(name);
        std::fs::write(
            &file,
            format!("#!/bin/sh\nprintf ran >> '{}'\nexit 0\n", marker.display()),
        )
        .unwrap();
        std::fs::set_permissions(file, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    let path = format!("{}:{}", bin.display(), std::env::var("PATH").unwrap());
    let mut runtime = Runtime {
        cwd: f.code.clone(),
        env: BTreeMap::from([
            ("PATH".into(), path.clone().into()),
            ("HOME".into(), f.code.as_os_str().to_owned()),
            ("VIBE_STORE".into(), f.store.as_os_str().to_owned()),
            ("VIBE_DESIGN_LINT".into(), "1".into()),
        ]),
    };
    let api = SpecApi::new(runtime.clone()).unwrap();
    let result = api
        .execute(SpecRequest::Validate(vibe_spec::ValidateRequest {
            selector: f.selector(),
            scope: ValidationScope::WithCode,
            strict: false,
        }))
        .unwrap();
    assert!(
        result.data["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["code"] == "external_lint_disabled")
    );
    assert!(!marker.exists());
    let context = api.resolve_context(&f.selector()).unwrap();
    let result = api
        .execute(SpecRequest::WriteDocument(
            vibe_spec::WriteDocumentRequest {
                selector: f.selector(),
                path: "spec/quick/draft.md".into(),
                content: "draft\n".into(),
                expected_revision: context.revision_token,
            },
        ))
        .unwrap();
    assert!(
        result.data["diagnostics"][0]["result"]["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["code"] == "external_lint_disabled")
    );
    assert!(!marker.exists());
    let old = common::layered_tree("legacy-lint");
    runtime.cwd = old.to_path_buf();
    runtime.env.remove(std::ffi::OsStr::new("VIBE_STORE"));
    let argv = ["vibe-spec", "validate"].map(std::ffi::OsString::from);
    let out = vibe_spec::run_cli(&argv, &runtime, &mut std::io::empty());
    assert_eq!(out.exit, 0, "{}", String::from_utf8_lossy(&out.stderr));
    assert!(marker.exists());
    f.assert_code_unchanged();
}
#[cfg(unix)]
#[test]
fn managed_queries_refuse_artifact_symlinks_and_bundle_paths_before_writes() {
    let f = Fixture::new(true);
    let context = f.api.resolve_context(&f.selector()).unwrap();
    let before = f.api.list_documents(&f.selector()).unwrap();
    for paths in [
        [
            "openspec/changes/../escape/spec.md",
            "openspec/changes/../escape/spec.md",
        ],
        [
            "openspec/changes/a/design.md",
            "openspec/changes/a/design.md",
        ],
        [
            "openspec/changes/a/design.md",
            "openspec/changes/a/design.md/nested.md",
        ],
    ] {
        let bundle = vibe_spec::OpenSpecBundle {
            files: paths
                .iter()
                .map(|p| vibe_spec::BundleFile {
                    path: (*p).into(),
                    content: "x".into(),
                })
                .collect(),
        };
        assert_eq!(
            f.api
                .execute(SpecRequest::Import(vibe_spec::ImportRequest {
                    selector: f.selector(),
                    expected_revision: context.revision_token.clone(),
                    bundle,
                    feature: None
                }))
                .unwrap_err()
                .code,
            "invalid_bundle"
        );
    }
    assert_eq!(
        f.api.list_documents(&f.selector()).unwrap().documents,
        before.documents
    );
    let outside = f._owner.join("outside");
    common::write(&outside, "2026-01-15-checkout.md", "### Task 9\n");
    std::fs::remove_dir(context.artifact_root.join("plans")).unwrap();
    std::os::unix::fs::symlink(&outside, context.artifact_root.join("plans")).unwrap();
    assert_eq!(
        f.api
            .execute(SpecRequest::Plan(vibe_spec::PlanRequest {
                selector: f.selector(),
                feature: "checkout".into()
            }))
            .unwrap_err()
            .code,
        "context_unreadable"
    );
    assert_eq!(
        std::fs::read(outside.join("2026-01-15-checkout.md")).unwrap(),
        b"### Task 9\n"
    );
    f.assert_code_unchanged();
}

#[test]
fn registering_repo_first_preserves_legacy_query_bytes_and_non_git_trace() {
    let source = common::layered_tree("registered-legacy");
    let store = source.parent().unwrap().join(format!(
        "{}-store",
        source.file_name().unwrap().to_string_lossy()
    ));
    common::write(
        &source,
        "docs/spec/lessons.md",
        "# Lessons\n\n### Bound context\n**Rule:** Check the source.\n**Tags:** lifecycle\n",
    );
    common::write(
        &source,
        "docs/superpowers/specs/2026-01-15-checkout-design.md",
        "# Design\n",
    );
    common::write(
        &source,
        "docs/superpowers/plans/2026-01-15-checkout.md",
        "**Spec:** `../specs/2026-01-15-checkout-design.md`\n\n### Task 1: Check\n- [ ] Verify\n",
    );
    let commands: Vec<Vec<&str>> = vec![
        vec!["root", "--json"],
        vec!["validate", "--json"],
        vec!["plan", "--feature", "checkout", "--json"],
        vec!["feature", "checkout", "--json"],
        vec!["lessons-for", "lifecycle", "--json"],
        vec!["list-specs"],
        vec!["scan-merges", "--format", "json"],
        vec!["trace", "--json"],
        vec!["check-drift", "--json"],
    ];
    let before: Vec<_> = commands
        .iter()
        .map(|args| common::spec(&source, args, &[("VIBE_STORE", store.to_str().unwrap())]))
        .collect();
    let registered = common::spec(
        &source,
        &["project", "init", "--mode", "repo-first"],
        &[("VIBE_STORE", store.to_str().unwrap())],
    );
    assert_eq!(registered.code, 0, "{}", registered.stderr);
    for (args, before) in commands.iter().zip(before) {
        let after = common::spec(&source, args, &[("VIBE_STORE", store.to_str().unwrap())]);
        assert_eq!(
            (after.code, after.stdout, after.stderr),
            (before.code, before.stdout, before.stderr),
            "{args:?}"
        );
    }
}
#[test]
fn ordinary_draft_authoring_reports_errors_without_claiming_tree_validity() {
    let f = Fixture::new(true);
    let product = f
        .api
        .read_document(&f.selector(), "spec/product.md")
        .unwrap()
        .document
        .content;
    let context = f.api.resolve_context(&f.selector()).unwrap();
    let response = f
        .api
        .execute(SpecRequest::WriteDocument(
            vibe_spec::WriteDocumentRequest {
                selector: f.selector(),
                path: "spec/product.md".into(),
                content: "Draft without YAML\n".into(),
                expected_revision: context.revision_token,
            },
        ))
        .unwrap();
    assert_eq!(response.data["recorded"], true);
    assert_eq!(response.data["validated"], false);
    assert!(
        response.data["diagnostics"][0]["result"]["errors"]
            .as_u64()
            .unwrap()
            > 0
    );
    let result = f
        .api
        .execute(SpecRequest::Validate(vibe_spec::ValidateRequest {
            selector: f.selector(),
            scope: ValidationScope::SpecOnly,
            strict: false,
        }))
        .unwrap();
    assert!(result.data["errors"].as_u64().unwrap() > 0);
    f.write("spec/product.md", &product);
    f.write(
        "spec/concepts/new-note.md",
        "---\ntype: concept\nupdated: 2026-01-15\n---\n\nA new fact.\n",
    );
    f.write(
        "spec/product.md",
        &(product + "\n[Fact](concepts/new-note.md)\n"),
    );
    let result = f
        .api
        .execute(SpecRequest::Validate(vibe_spec::ValidateRequest {
            selector: f.selector(),
            scope: ValidationScope::SpecOnly,
            strict: false,
        }))
        .unwrap();
    assert_eq!(result.data["errors"], 0, "{}", result.data);
    f.assert_code_unchanged();
}
#[test]
fn global_cli_and_hooks_share_roots_without_code_runtime_writes() {
    let f = Fixture::new(true);
    let env = [("VIBE_STORE", f.store.to_str().unwrap())];
    f.write(
        "superpowers/specs/2026-01-15-hooked-design.md",
        "# Hooked\n",
    );
    f.write(
        "superpowers/plans/2026-01-15-hooked.md",
        "**Spec:** `../specs/2026-01-15-hooked-design.md`\n\n### Task 1: Work\n- [ ] Do work\n",
    );
    for args in [
        vec!["root", "--json"],
        vec!["validate", "--json"],
        vec!["plan", "--feature", "hooked", "--json"],
        vec!["feature", "hooked", "--json"],
        vec!["list-specs"],
        vec!["check-drift", "--json"],
        vec!["init", "--json"],
    ] {
        let out = common::spec(&f.code, &args, &env);
        assert_eq!(out.code, 0, "{args:?}: {}", out.stderr);
    }
    let session = serde_json::json!({"hook_event_name":"SessionStart","cwd":f.code});
    let out = common::spec_stdin(
        &f.code,
        &["hook", "session-start"],
        &env,
        session.to_string().as_bytes(),
    );
    assert_eq!(out.code, 0);
    assert!(!out.stdout.is_empty());
    let stop = serde_json::json!({"hook_event_name":"Stop","cwd":f.code});
    let out = common::spec_stdin(
        &f.code,
        &["hook", "validate"],
        &env,
        stop.to_string().as_bytes(),
    );
    assert_eq!(out.code, 0, "{}", out.stderr);
    let context = f.api.resolve_context(&f.selector()).unwrap();
    let transcript = f._owner.join("empty-transcript");
    std::fs::write(&transcript, "").unwrap();
    let guard = serde_json::json!({"hook_event_name":"PreToolUse","cwd":f.code,"tool_name":"Write","tool_input":{"file_path":context.artifact_root.join("plans/2026-01-15-hooked.md")},"transcript_path":transcript});
    let out = common::spec_stdin(
        &f.code,
        &["hook", "guard"],
        &env,
        guard.to_string().as_bytes(),
    );
    assert_eq!(out.code, 2, "{}", out.stderr);
    f.assert_code_unchanged();
}

#[test]
fn completion_refuses_missing_ambiguous_or_unreachable_history_evidence() {
    for kind in ["missing", "ambiguous", "unreachable"] {
        let f = Fixture::new(true);
        f.write("superpowers/specs/2026-01-15-bound-design.md", "# Bound\n");
        let tech = f
            .api
            .read_document(&f.selector(), "spec/tech.md")
            .unwrap()
            .document
            .content;
        f.write("spec/tech.md", &(tech + "\nDurable fact.\n"));
        let context = f.api.resolve_context(&f.selector()).unwrap();
        let head = context.spec_revision.as_ref().unwrap();
        let history = context
            .spec_root
            .ancestors()
            .nth(3)
            .unwrap()
            .join("history.git");
        let worktree = context.spec_root.parent().unwrap();
        let reference = common::git_out(worktree, &["symbolic-ref", "HEAD"]);
        let tree = common::git_out(&history, &["rev-parse", &format!("{head}^{{tree}}")]);
        let metadata = serde_json::json!({"v":1,"project":context.project_id.as_ref().unwrap().as_str(),"branch":reference,"feature_introductions":if kind=="ambiguous"{serde_json::json!([{"feature":"bound","baseline":head},{"feature":"bound","baseline":head}])}else{serde_json::json!([{"feature":"bound","baseline":"0000000000000000000000000000000000000001"}])},"feature_completions":[]});
        let message = if kind == "missing" {
            "unmanaged root".into()
        } else {
            format!("controlled malformed evidence\n\nVibe-Spec-Metadata: {metadata}")
        };
        let mut args = vec!["commit-tree", tree.as_str(), "-m", message.as_str()];
        if kind != "missing" {
            args.extend(["-p", head.as_str()]);
        }
        let replacement = common::git_out(&history, &args);
        common::git(
            &history,
            "2026-01-15T00:00:00Z",
            &["update-ref", &reference, &replacement, head],
        );
        let current = f.api.resolve_context(&f.selector()).unwrap();
        let error = f
            .api
            .execute(SpecRequest::Promote(vibe_spec::PromoteRequest {
                selector: f.selector(),
                feature: "bound".into(),
                expected_revision: current.revision_token,
                dry_run: false,
                done: true,
                target: None,
                base: None,
            }))
            .unwrap_err();
        assert!(
            matches!(
                error.code.as_str(),
                "missing_evidence" | "ambiguous_evidence" | "invalid_history"
            ),
            "{kind}: {error}"
        );
        assert_eq!(
            common::git_out(&history, &["rev-parse", &reference]),
            replacement
        );
        assert!(
            context
                .artifact_root
                .join("specs/2026-01-15-bound-design.md")
                .exists()
        );
        f.assert_code_unchanged();
    }
}

#[test]
fn disabling_external_lint_retains_internal_design_checks() {
    let f = Fixture::new(true);
    f.write(
        "spec/design.md",
        "---\ntype: hub\nupdated: 2026-01-15\ncolors:\n---\n\n# Design\n",
    );
    let result = f
        .api
        .execute(SpecRequest::Validate(vibe_spec::ValidateRequest {
            selector: f.selector(),
            scope: ValidationScope::SpecOnly,
            strict: false,
        }))
        .unwrap();
    assert!(
        result.data["findings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|f| f["check"] == "SF3"),
        "{}",
        result.data
    );
}

#[test]
fn review_fix_unregistered_managed_mutations_obey_the_document_lock() {
    let owner = common::project("unregistered-lock");
    let code = owner.join("code");
    std::fs::create_dir(&code).unwrap();
    common::write(&code, "docs/spec/tech.md", "# Tech\n");
    common::write(
        &code,
        "docs/spec/features/auth/tech.md",
        "<!-- merge -->\nA promoted fact.\n<!-- /merge -->\n",
    );
    let lock_path = code.join(".vibe/run/spec-documents.lock");
    std::fs::create_dir_all(lock_path.parent().unwrap()).unwrap();
    let held = match vibe_core::lock::Lock::acquire(&lock_path, vibe_core::lock::STALE).unwrap() {
        vibe_core::lock::Acquired::Got(lock) => lock,
        _ => panic!("unexpected fixture lock"),
    };
    let before = std::fs::read(code.join("docs/spec/tech.md")).unwrap();
    let store = owner.join("store");
    let result = common::spec(
        &code,
        &["promote", "auth", "--json"],
        &[("VIBE_STORE", store.to_str().unwrap())],
    );
    assert_eq!(result.code, 1, "{} {}", result.stderr, result.text());
    assert!(result.stderr.contains("locked"), "{}", result.stderr);
    assert_eq!(
        std::fs::read(code.join("docs/spec/tech.md")).unwrap(),
        before
    );
    drop(held);
}
#[test]
fn review_fix_fallback_artifacts_never_substitute_for_durable_facts() {
    for fallback_target in [true, false] {
        let f = Fixture::new(true);
        let feature = if fallback_target {
            "unpromoted"
        } else {
            "selected"
        };
        if !fallback_target {
            f.write(
                "superpowers/specs/2026-01-15-selected-design.md",
                "# Selected\n",
            );
        }
        let fallback = if fallback_target {
            "unpromoted"
        } else {
            "unrelated"
        };
        for kind in ["product", "tech"] {
            let sibling = if kind == "product" { "tech" } else { "product" };
            f.write(&format!("spec/features/{fallback}/{kind}.md"),&format!("---\ntype: feature-{kind}\nfeature: {fallback}\nsibling: {sibling}.md\nparent: ../../{kind}.md\nupdated: 2026-01-15\n---\n\n# Feature\n"));
        }
        let before = f.api.list_documents(&f.selector()).unwrap();
        let mut request = vibe_spec::PromoteRequest {
            selector: f.selector(),
            feature: feature.into(),
            expected_revision: before.context.revision_token.clone(),
            dry_run: false,
            done: true,
            target: None,
            base: None,
        };
        let error = f
            .api
            .execute(SpecRequest::Promote(request.clone()))
            .unwrap_err();
        assert_eq!(error.code, "promotion_required");
        assert_eq!(
            before.documents,
            f.api.list_documents(&f.selector()).unwrap().documents
        );
        let tech = f
            .api
            .read_document(&f.selector(), "spec/tech.md")
            .unwrap()
            .document
            .content;
        f.write("spec/tech.md", &(tech + "\nA durable root fact.\n"));
        request.expected_revision = f.api.resolve_context(&f.selector()).unwrap().revision_token;
        f.api.execute(SpecRequest::Promote(request)).unwrap();
        let valid = f
            .api
            .execute(SpecRequest::Validate(vibe_spec::ValidateRequest {
                selector: f.selector(),
                scope: ValidationScope::SpecOnly,
                strict: false,
            }))
            .unwrap();
        assert_eq!(valid.data["errors"], 0, "{}", valid.data);
        f.assert_code_unchanged();
    }
}

#[test]
fn review_fix_global_selection_failures_never_fall_back_to_code_initialization() {
    for failure in ["missing-branch", "detached", "missing-root"] {
        let f = Fixture::new(true);
        match failure {
            "missing-branch" => common::git(
                &f.code,
                "2026-01-15T00:00:00Z",
                &["checkout", "-b", "not-stored"],
            ),
            "detached" => common::git(
                &f.code,
                "2026-01-15T00:00:00Z",
                &["checkout", "--detach", "HEAD"],
            ),
            _ => {
                let context = f.api.resolve_context(&f.selector()).unwrap();
                std::fs::remove_dir_all(context.spec_root).unwrap();
            }
        }
        let before = snapshot(&f.code);
        let result = common::spec(
            &f.code,
            &["init", "--json"],
            &[("VIBE_STORE", f.store.to_str().unwrap())],
        );
        assert_eq!(
            result.code,
            1,
            "{failure}: {} {}",
            result.stderr,
            result.text()
        );
        assert_eq!(
            snapshot(&f.code),
            before,
            "{failure}: code checkout changed"
        );
        assert!(!f.code.join("docs/spec").exists());
    }
}

#[cfg(unix)]
#[test]
fn unregistered_mutation_preserves_symlinked_root_and_initialization() {
    let owner = common::project("legacy-root-alias");
    let code = owner.join("code");
    let real = owner.join("real-spec");
    std::fs::create_dir(&code).unwrap();
    common::write(&real, "tech.md", "# Tech\n");
    common::write(
        &real,
        "features/auth/tech.md",
        "<!-- merge -->\nA fact.\n<!-- /merge -->\n",
    );
    std::os::unix::fs::symlink(&real, code.join(".spec")).unwrap(); // literal
    let store = owner.join("store");
    let env = [("VIBE_STORE", store.to_str().unwrap())];
    let out = common::spec(&code, &["promote", "auth", "--json"], &env);
    assert_eq!(out.code, 0, "{}", out.stderr);
    assert!(
        std::fs::read_to_string(real.join("tech.md"))
            .unwrap()
            .contains("A fact.")
    );
    assert!(
        std::fs::symlink_metadata(code.join(".spec")) // literal
            .unwrap()
            .file_type()
            .is_symlink()
    );
    let fresh = owner.join("fresh");
    std::fs::create_dir(&fresh).unwrap();
    let out = common::spec(&fresh, &["init", "--json"], &env);
    assert_eq!(out.code, 0, "{}", out.stderr);
    assert!(fresh.join("docs/spec/product.md").is_file());
}
#[test]
fn managed_validation_still_rejects_partial_nonempty_fallback_features() {
    let f = Fixture::new(true);
    f.write(
        "spec/features/partial/product.md",
        "---\ntype: feature-product\nupdated: 2026-01-15\n---\n\n# Partial\n",
    );
    let result = f
        .api
        .execute(SpecRequest::Validate(vibe_spec::ValidateRequest {
            selector: f.selector(),
            scope: ValidationScope::SpecOnly,
            strict: false,
        }))
        .unwrap();
    assert!(
        result.data["findings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|f| f["check"] == "SF23" && f["file"] == "spec/features/partial/tech.md")
    );
}

#[test]
fn review_fix_relative_spec_named_folder_links_keep_document_meaning() {
    let f = Fixture::new(true);
    f.write(
        "spec/tech.md",
        "---\ntype: hub\nupdated: 2026-01-15\n---\n\n[Spec system](tech/spec.md)\n",
    );
    f.write(
        "spec/tech/spec.md",
        "---\ntype: tech\napplies: src/**\nupdated: 2026-01-15\n---\n\n[Details](spec/detail.md)\n",
    );
    f.write(
        "spec/tech/spec/detail.md",
        "---\ntype: tech\napplies: src/**\nupdated: 2026-01-15\n---\n\nDetails.\n",
    );
    let result = f
        .api
        .execute(SpecRequest::Validate(vibe_spec::ValidateRequest {
            selector: f.selector(),
            scope: ValidationScope::SpecOnly,
            strict: false,
        }))
        .unwrap();
    assert!(
        result.data["findings"]
            .as_array()
            .unwrap()
            .iter()
            .all(|f| f["check"] != "SF22" && f["check"] != "SF34"),
        "{}",
        result.data
    );
}

#[test]
fn registered_ordinary_consumers_ignore_completed_artifact_free_directories() {
    let f = Fixture::new(false);
    let context = f.api.resolve_context(&f.selector()).unwrap();
    std::fs::create_dir_all(context.spec_root.join("features/completed")).unwrap();
    f.write("spec/plan.md","---\ntype: hub\nupdated: 2026-01-15\n---\n\n| Feature | Status |\n|---|---|\n| active | IN PROGRESS |\n");
    for kind in ["product", "tech"] {
        f.write(&format!("spec/features/active/{kind}.md"),&format!("---\ntype: feature-{kind}\nfeature: active\nsibling: {}.md\nparent: ../../{kind}.md\nupdated: 2026-01-15\n---\n\n# Active\n",if kind=="product"{"tech"}else{"product"}));
    }
    f.write("spec/features/active/plan.md","---\ntype: feature-plan\nfeature: active\nparent: ../../plan.md\nupdated: 2026-01-15\n---\n\n### active/1 — Work\n");
    common::write(&f.code, "tests/citations.rs", "// active/1 completed/99\n");
    for command in ["validate", "check-drift", "trace"] {
        let out = common::spec(
            &f.code,
            &[command, "--json"],
            &[("VIBE_STORE", f.store.to_str().unwrap())],
        );
        assert_eq!(out.code, 0, "{command}: {} {}", out.stderr, out.text());
    }
}
