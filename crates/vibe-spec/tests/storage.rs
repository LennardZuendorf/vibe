#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Global spec history controls (spec-projects-mcp/3).
mod common;
use serde_json::Value;
#[test]
fn global_setup_preserves_code_branch_index_status_and_tree() {
    let owner = common::project("storage-main");
    let code = owner.join("code");
    let store = owner.join("store");
    std::fs::create_dir(&code).unwrap();
    common::git(&code, "2026-01-01T00:00:00Z", &["init", "-b", "main"]);
    common::write(&code, "tracked", "initial");
    common::git(&code, "2026-01-01T00:00:00Z", &["add", "tracked"]);
    common::git(&code, "2026-01-01T00:00:00Z", &["commit", "-m", "initial"]);
    common::write(&code, "tracked", "staged");
    common::git(&code, "2026-01-01T00:00:00Z", &["add", "tracked"]);
    common::write(&code, "tracked", "unstaged");
    common::write(&code, "untracked", "keep");
    let head = common::git_out(&code, &["rev-parse", "HEAD"]);
    let status = common::git_out(
        &code,
        &["status", "--porcelain=v1", "--untracked-files=all"],
    );
    let index = std::fs::read(code.join(".git/index")).unwrap();
    let out = common::spec(
        &code,
        &["project", "init", "--mode", "global-first"],
        &[("VIBE_STORE", store.to_str().unwrap())],
    );
    assert_eq!(out.code, 0, "{}", out.stderr);
    let data: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(common::git_out(&code, &["rev-parse", "HEAD"]), head);
    assert_eq!(
        common::git_out(&code, &["branch", "--show-current"]),
        "main"
    );
    assert_eq!(
        common::git_out(
            &code,
            &["status", "--porcelain=v1", "--untracked-files=all"]
        ),
        status
    );
    assert_eq!(std::fs::read(code.join(".git/index")).unwrap(), index);
    assert!(!code.join("docs").exists());
    assert!(!code.join(".vibe").exists());
    assert!(
        std::path::Path::new(data["context"]["spec_root"].as_str().unwrap())
            .join("product.md")
            .is_file()
    );
    assert!(data["context"]["spec_revision"].as_str().is_some());
}
#[test]
fn existing_repo_specs_and_corrupt_registry_are_untouched() {
    let owner = common::project("storage-existing");
    let code = owner.join("code");
    let store = owner.join("store");
    std::fs::create_dir(&code).unwrap();
    common::git(&code, "2026-01-01T00:00:00Z", &["init", "-b", "main"]);
    common::write(&code, "docs/spec/product.md", "keep me");
    let out = common::spec(
        &code,
        &["project", "init", "--mode", "global-first"],
        &[("VIBE_STORE", store.to_str().unwrap())],
    );
    assert_eq!(out.code, 1);
    assert!(
        out.stderr.contains("mode_migration_required"),
        "{}",
        out.stderr
    );
    assert!(!store.exists());
    std::fs::create_dir_all(store.join("spec")).unwrap();
    std::fs::write(store.join("spec/registry.json"), b"{broken").unwrap();
    let out = common::spec(
        &code,
        &["project", "init", "--mode", "repo-first"],
        &[("VIBE_STORE", store.to_str().unwrap())],
    );
    assert_eq!(out.code, 1);
    assert!(out.stderr.contains("invalid_registry"), "{}", out.stderr);
    assert_eq!(
        std::fs::read(store.join("spec/registry.json")).unwrap(),
        b"{broken"
    );
    assert_eq!(
        std::fs::read(code.join("docs/spec/product.md")).unwrap(),
        b"keep me"
    );
}
#[test]
fn branch_names_are_exact_refs_never_paths_and_require_explicit_base() {
    let owner = common::project("storage-branches");
    let code = owner.join("code");
    let store = owner.join("store");
    std::fs::create_dir(&code).unwrap();
    common::git(&code, "2026-01-01T00:00:00Z", &["init", "-b", "main"]);
    common::git(
        &code,
        "2026-01-01T00:00:00Z",
        &["commit", "--allow-empty", "-m", "initial"],
    );
    let env = [("VIBE_STORE", store.to_str().unwrap())];
    let out = common::spec(&code, &["project", "init", "--mode", "global-first"], &env);
    assert_eq!(out.code, 0, "{}", out.stderr);
    for branch in [
        "topic/slash",
        "Unicode/雪",
        "lookalike/spec",
        "spaces rejected",
        "../escape",
    ] {
        let out = common::spec(&code, &["branch", "init", branch, "--from", "main"], &env);
        if branch.contains(' ') || branch.contains("..") {
            assert_eq!(out.code, 1, "{branch}: {}", out.stderr);
        } else {
            assert_eq!(out.code, 0, "{branch}: {}", out.stderr);
            let value: Value = serde_json::from_slice(&out.stdout).unwrap();
            assert_eq!(value["context"]["code_ref"], format!("refs/heads/{branch}"));
            assert!(value["context"]["code_available"].as_bool() == Some(false));
            assert!(!value["branch"]["id"].as_str().unwrap().contains('/'));
        }
    }
    assert_eq!(
        common::git_out(&code, &["branch", "--show-current"]),
        "main"
    );
    assert_eq!(
        common::git_out(
            &code,
            &["for-each-ref", "--format=%(refname)", "refs/heads"]
        ),
        "refs/heads/main"
    );
    let out = common::spec(
        &code,
        &["branch", "init", "missing-base", "--from", "missing"],
        &env,
    );
    assert_eq!(out.code, 1);
    assert!(out.stderr.contains("branch_not_found"), "{}", out.stderr);
}
#[test]
fn repeated_global_setup_preserves_uncommitted_spec_edits_and_history() {
    let owner = common::project("storage-edits");
    let code = owner.join("code");
    let store = owner.join("store");
    std::fs::create_dir(&code).unwrap();
    common::git(&code, "2026-01-01T00:00:00Z", &["init", "-b", "main"]);
    let env = [("VIBE_STORE", store.to_str().unwrap())];
    let first = common::spec(&code, &["project", "init", "--mode", "global-first"], &env);
    assert_eq!(first.code, 0, "{}", first.stderr);
    let first: Value = serde_json::from_slice(&first.stdout).unwrap();
    let spec = std::path::Path::new(first["context"]["spec_root"].as_str().unwrap());
    std::fs::write(spec.join("product.md"), "uncommitted user edit").unwrap();
    let again = common::spec(&code, &["project", "init", "--mode", "global-first"], &env);
    assert_eq!(again.code, 0, "{}", again.stderr);
    let again: Value = serde_json::from_slice(&again.stdout).unwrap();
    assert_eq!(
        again["context"]["spec_revision"],
        first["context"]["spec_revision"]
    );
    assert_eq!(
        std::fs::read(spec.join("product.md")).unwrap(),
        b"uncommitted user edit"
    );
    // Crash after project metadata publication but before registry publication.
    std::fs::remove_file(store.join("spec/registry.json")).unwrap();
    let resumed = common::spec(&code, &["project", "init", "--mode", "global-first"], &env);
    assert_eq!(resumed.code, 0, "{}", resumed.stderr);
    let resumed: Value = serde_json::from_slice(&resumed.stdout).unwrap();
    assert_eq!(
        resumed["context"]["spec_revision"],
        first["context"]["spec_revision"]
    );
    assert_eq!(
        std::fs::read(spec.join("product.md")).unwrap(),
        b"uncommitted user edit"
    );
}
#[test]
fn managed_history_rejects_ambient_locators_and_local_config_redirection() {
    let owner = common::project("storage-git-injection");
    let code = owner.join("code");
    let store = owner.join("store");
    std::fs::create_dir(&code).unwrap();
    common::git(&code, "2026-01-01T00:00:00Z", &["init", "-b", "main"]);
    common::git(
        &code,
        "2026-01-01T00:00:00Z",
        &["commit", "--allow-empty", "-m", "code"],
    );
    let head = common::git_out(&code, &["rev-parse", "HEAD"]);
    let gitdir = code.join(".git");
    let index = code.join(".git/index");
    let injected = [
        ("VIBE_STORE", store.to_str().unwrap()),
        ("GIT_DIR", gitdir.to_str().unwrap()),
        ("GIT_WORK_TREE", code.to_str().unwrap()),
        ("GIT_INDEX_FILE", index.to_str().unwrap()),
    ];
    let first = common::spec(
        &code,
        &["project", "init", "--mode", "global-first"],
        &injected,
    );
    assert_eq!(first.code, 0, "{}", first.stderr);
    let first: Value = serde_json::from_slice(&first.stdout).unwrap();
    let id = first["project"]["id"].as_str().unwrap();
    let project = store.join("spec/projects").join(id);
    let config = project.join("history.git/config");
    let original = std::fs::read(&config).unwrap();
    let mut redirected = original.clone();
    redirected.extend_from_slice(format!("\n[core]\n worktree = {}\n", code.display()).as_bytes());
    std::fs::write(&config, redirected).unwrap();
    let out = common::spec(
        &code,
        &["branch", "init", "topic", "--from", "main"],
        &[("VIBE_STORE", store.to_str().unwrap())],
    );
    assert_eq!(out.code, 1);
    assert!(out.stderr.contains("invalid_storage"), "{}", out.stderr);
    assert_eq!(common::git_out(&code, &["rev-parse", "HEAD"]), head);
    assert!(!code.join("spec").exists());
}
#[cfg(unix)]
#[test]
fn symlinked_store_ancestor_is_refused_without_external_writes() {
    use std::os::unix::fs::symlink;
    let owner = common::project("storage-ancestor");
    let code = owner.join("code");
    let outside = owner.join("outside");
    std::fs::create_dir(&code).unwrap();
    std::fs::create_dir(&outside).unwrap();
    common::git(&code, "2026-01-01T00:00:00Z", &["init", "-b", "main"]);
    symlink(&outside, owner.join("link")).unwrap();
    let store = owner.join("link/nested/store");
    let out = common::spec(
        &code,
        &["project", "init", "--mode", "global-first"],
        &[("VIBE_STORE", store.to_str().unwrap())],
    );
    assert_eq!(out.code, 1, "{}", out.stderr);
    assert!(!outside.join("nested").exists());
}
#[test]
fn interrupted_setup_preserves_divergent_project_metadata() {
    let owner = common::project("storage-divergent");
    let code = owner.join("code");
    let store = owner.join("store");
    std::fs::create_dir(&code).unwrap();
    common::git(&code, "2026-01-01T00:00:00Z", &["init", "-b", "main"]);
    let env = [("VIBE_STORE", store.to_str().unwrap())];
    let first = common::spec(&code, &["project", "init", "--mode", "global-first"], &env);
    assert_eq!(first.code, 0, "{}", first.stderr);
    let first: Value = serde_json::from_slice(&first.stdout).unwrap();
    let id = first["project"]["id"].as_str().unwrap();
    let metadata = store.join("spec/projects").join(id).join("project.json");
    let mut state: Value = serde_json::from_slice(&std::fs::read(&metadata).unwrap()).unwrap();
    state["snapshots"] = serde_json::json!([{"sentinel":"later user metadata"}]);
    let changed = state.to_string();
    std::fs::write(&metadata, &changed).unwrap();
    std::fs::remove_file(store.join("spec/registry.json")).unwrap();
    let resumed = common::spec(&code, &["project", "init", "--mode", "global-first"], &env);
    assert_eq!(resumed.code, 1, "{}", resumed.stderr);
    assert!(
        resumed.stderr.contains("setup_conflict"),
        "{}",
        resumed.stderr
    );
    assert_eq!(std::fs::read_to_string(metadata).unwrap(), changed);
    assert!(!store.join("spec/registry.json").exists());
}
#[test]
fn typed_management_requires_exact_registry_and_project_revisions() {
    use std::collections::BTreeMap;
    use vibe_spec::{
        ContextSelector, InitBranchRequest, InitProjectRequest, Runtime, SpecApi, SpecRequest,
        StorageMode,
    };
    let owner = common::project("storage-typed-tokens");
    let code = owner.join("code");
    let store = owner.join("store");
    std::fs::create_dir(&code).unwrap();
    common::git(&code, "2026-01-01T00:00:00Z", &["init", "-b", "main"]);
    let api = SpecApi::new(Runtime {
        cwd: code,
        env: BTreeMap::from([
            ("PATH".into(), std::env::var_os("PATH").unwrap_or_default()),
            ("HOME".into(), owner.as_os_str().to_owned()),
            ("VIBE_STORE".into(), store.as_os_str().to_owned()),
        ]),
    })
    .unwrap();
    let first = api
        .execute(SpecRequest::ProjectInit(InitProjectRequest {
            selector: ContextSelector::default(),
            mode: StorageMode::GlobalFirst,
            name: None,
            expected_revision: "absent".into(),
        }))
        .unwrap();
    let request = InitBranchRequest {
        selector: ContextSelector::default(),
        name: "topic".into(),
        from: "main".into(),
        expected_revision: first.data["revision"].as_str().unwrap().into(),
        expected_project_revision: "wrong".into(),
    };
    assert_eq!(
        api.execute(SpecRequest::BranchInit(request.clone()))
            .unwrap_err()
            .code,
        "revision_changed"
    );
    let mut request = request;
    request.expected_project_revision = first.data["project_revision"].as_str().unwrap().into();
    let created = api
        .execute(SpecRequest::BranchInit(request.clone()))
        .unwrap();
    assert_eq!(created.data["branch"]["code_ref"], "refs/heads/topic");
    assert_eq!(
        api.execute(SpecRequest::BranchInit(request))
            .unwrap_err()
            .code,
        "revision_changed"
    );
}
#[test]
fn global_setup_refuses_invalid_code_configuration_before_storage_effects() {
    let owner = common::project("storage-invalid-code-config");
    let code = owner.join("code");
    let store = owner.join("store");
    std::fs::create_dir(&code).unwrap();
    common::git(&code, "2026-01-01T00:00:00Z", &["init", "-b", "main"]);
    common::write(&code, ".vibe/spec.json", "{broken");
    let out = common::spec(
        &code,
        &["project", "init", "--mode", "global-first"],
        &[("VIBE_STORE", store.to_str().unwrap())],
    );
    assert_eq!(out.code, 1);
    assert!(out.stderr.contains("invalid_config"), "{}", out.stderr);
    assert!(!store.exists(), "failed setup created managed storage");
    assert_eq!(
        std::fs::read(code.join(".vibe/spec.json")).unwrap(),
        b"{broken"
    );
}
#[test]
fn existing_branch_init_rejects_an_explicit_unrelated_checkout() {
    let owner = common::project("storage-existing-branch-selection");
    let code = owner.join("code");
    let other = owner.join("other");
    let store = owner.join("store");
    for root in [&code, &other] {
        std::fs::create_dir(root).unwrap();
        common::git(root, "2026-01-01T00:00:00Z", &["init", "-b", "main"]);
    }
    let env = [("VIBE_STORE", store.to_str().unwrap())];
    let first = common::spec(&code, &["project", "init", "--mode", "global-first"], &env);
    assert_eq!(first.code, 0, "{}", first.stderr);
    let first: Value = serde_json::from_slice(&first.stdout).unwrap();
    let id = first["project"]["id"].as_str().unwrap();
    let out = common::spec(
        &code,
        &[
            "branch",
            "init",
            "main",
            "--from",
            "main",
            "--project",
            id,
            "--checkout",
            other.to_str().unwrap(),
        ],
        &env,
    );
    assert_eq!(out.code, 1);
    assert!(out.stderr.contains("context_mismatch"), "{}", out.stderr);
}
#[test]
fn conflicting_root_override_cannot_publish_project_registration() {
    let owner = common::project("storage-root-override");
    let code = owner.join("code");
    let store = owner.join("store");
    let other = owner.join("other-spec");
    std::fs::create_dir(&code).unwrap();
    std::fs::create_dir(&other).unwrap();
    common::git(&code, "2026-01-01T00:00:00Z", &["init", "-b", "main"]);
    let out = common::spec(
        &code,
        &["project", "init", "--mode", "global-first"],
        &[
            ("VIBE_STORE", store.to_str().unwrap()),
            ("VIBE_SPEC_ROOT", other.to_str().unwrap()),
        ],
    );
    assert_eq!(out.code, 1);
    assert!(out.stderr.contains("context_mismatch"), "{}", out.stderr);
    assert!(
        !store.join("spec/registry.json").exists(),
        "conflicting override published project"
    );
    assert!(std::fs::read_dir(&other).unwrap().next().is_none());
}
#[test]
fn setup_resumes_after_partial_bare_history_initialization() {
    let owner = common::project("storage-partial-history");
    let code = owner.join("code");
    let store = owner.join("store");
    std::fs::create_dir(&code).unwrap();
    common::git(&code, "2026-01-01T00:00:00Z", &["init", "-b", "main"]);
    let env = [("VIBE_STORE", store.to_str().unwrap())];
    let first = common::spec(&code, &["project", "init", "--mode", "global-first"], &env);
    assert_eq!(first.code, 0, "{}", first.stderr);
    let first: Value = serde_json::from_slice(&first.stdout).unwrap();
    let project = store
        .join("spec/projects")
        .join(first["project"]["id"].as_str().unwrap());
    std::fs::remove_file(store.join("spec/registry.json")).unwrap();
    std::fs::remove_file(project.join("project.json")).unwrap();
    std::fs::remove_dir_all(project.join("history.git")).unwrap();
    std::fs::remove_dir_all(project.join("branches")).unwrap();
    for item in std::fs::read_dir(&project).unwrap() {
        let path = item.unwrap().path();
        if path
            .file_name()
            .unwrap()
            .to_string_lossy()
            .starts_with("setup-b-")
        {
            std::fs::remove_file(path).unwrap();
        }
    }
    std::fs::create_dir_all(project.join("history.git/objects")).unwrap();
    std::fs::create_dir_all(project.join("history.git/refs")).unwrap();
    let resumed = common::spec(&code, &["project", "init", "--mode", "global-first"], &env);
    assert_eq!(resumed.code, 0, "{}", resumed.stderr);
    let resumed: Value = serde_json::from_slice(&resumed.stdout).unwrap();
    assert_eq!(resumed["project"]["id"], first["project"]["id"]);
    assert!(resumed["context"]["spec_revision"].as_str().is_some());
    assert!(!code.join("docs").exists());
}
#[test]
fn store_inside_another_checkouts_git_directory_is_refused_before_effects() {
    let owner = common::project("storage-other-gitdir");
    let code = owner.join("code");
    let other = owner.join("other");
    for root in [&code, &other] {
        std::fs::create_dir(root).unwrap();
        common::git(root, "2026-01-01T00:00:00Z", &["init", "-b", "main"]);
    }
    let store = other.join(".git/spec-store");
    let out = common::spec(
        &code,
        &["project", "init", "--mode", "global-first"],
        &[("VIBE_STORE", store.to_str().unwrap())],
    );
    assert_eq!(out.code, 1);
    assert!(
        out.stderr.contains("store_inside_checkout"),
        "{}",
        out.stderr
    );
    assert!(!store.exists());
}
#[test]
fn branch_init_does_not_replace_missing_or_non_git_explicit_checkout_with_main() {
    let owner = common::project("storage-explicit-no-git");
    let code = owner.join("code");
    let nongit = owner.join("nongit");
    let store = owner.join("store");
    std::fs::create_dir(&code).unwrap();
    std::fs::create_dir(&nongit).unwrap();
    common::git(&code, "2026-01-01T00:00:00Z", &["init", "-b", "main"]);
    let env = [("VIBE_STORE", store.to_str().unwrap())];
    let first = common::spec(&code, &["project", "init", "--mode", "global-first"], &env);
    assert_eq!(first.code, 0, "{}", first.stderr);
    let first: Value = serde_json::from_slice(&first.stdout).unwrap();
    let id = first["project"]["id"].as_str().unwrap();
    let metadata = store.join("spec/projects").join(id).join("project.json");
    let before = std::fs::read(&metadata).unwrap();
    let registry = std::fs::read(store.join("spec/registry.json")).unwrap();
    fn history_bytes(
        root: &std::path::Path,
    ) -> std::collections::BTreeMap<std::path::PathBuf, Vec<u8>> {
        fn walk(
            root: &std::path::Path,
            dir: &std::path::Path,
            out: &mut std::collections::BTreeMap<std::path::PathBuf, Vec<u8>>,
        ) {
            for item in std::fs::read_dir(dir).unwrap() {
                let path = item.unwrap().path();
                if path.is_dir() {
                    walk(root, &path, out);
                } else {
                    out.insert(
                        path.strip_prefix(root).unwrap().to_owned(),
                        std::fs::read(path).unwrap(),
                    );
                }
            }
        }
        let mut out = std::collections::BTreeMap::new();
        walk(root, root, &mut out);
        out
    }
    let history = store.join("spec/projects").join(id).join("history.git");
    let original_history = history_bytes(&history);
    for (new_target, path) in [
        ("topic-missing", owner.join("missing")),
        ("topic-nongit", nongit),
    ] {
        for target in ["main", new_target] {
            let out = common::spec(
                &code,
                &[
                    "branch",
                    "init",
                    target,
                    "--from",
                    "main",
                    "--project",
                    id,
                    "--checkout",
                    path.to_str().unwrap(),
                ],
                &env,
            );
            assert_eq!(out.code, 1, "{}", out.stderr);
            assert!(
                out.stderr.contains("checkout_unavailable"),
                "{}",
                out.stderr
            );
            assert_eq!(std::fs::read(&metadata).unwrap(), before);
            assert_eq!(
                std::fs::read(store.join("spec/registry.json")).unwrap(),
                registry
            );
            assert_eq!(history_bytes(&history), original_history);
        }
    }
    let outside = common::spec(
        &owner,
        &[
            "branch",
            "init",
            "doc-only",
            "--from",
            "main",
            "--project",
            id,
        ],
        &env,
    );
    assert_eq!(outside.code, 0, "{}", outside.stderr);
    let outside: Value = serde_json::from_slice(&outside.stdout).unwrap();
    assert_eq!(outside["context"]["code_ref"], "refs/heads/doc-only");
    assert_eq!(outside["context"]["code_available"], false);
    assert_eq!(
        common::git_out(&code, &["branch", "--show-current"]),
        "main"
    );
}

#[test]
fn spec_projects_mcp_7_interrupted_setup_preserves_observation_bytes() {
    let owner = common::project("storage-observations");
    let code = owner.join("code");
    let store = owner.join("store");
    std::fs::create_dir(&code).unwrap();
    common::git(&code, "2026-01-01T00:00:00Z", &["init", "-b", "main"]);
    common::git(
        &code,
        "2026-01-01T00:00:00Z",
        &["commit", "--allow-empty", "-m", "initial"],
    );
    let env = [("VIBE_STORE", store.to_str().unwrap())];
    let init = common::spec(&code, &["project", "init", "--mode", "global-first"], &env);
    assert_eq!(init.code, 0, "{}", init.stderr);
    let value: Value = serde_json::from_slice(&init.stdout).unwrap();
    let id = value["project"]["id"].as_str().unwrap();
    let observed = common::spec(&code, &["context"], &env);
    assert_eq!(observed.code, 0, "{}", observed.stderr);
    let path = store.join("spec/projects").join(id).join("project.json");
    let bytes = std::fs::read(&path).unwrap();
    let state: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(state["observations"].as_array().unwrap().len(), 1);
    assert!(state["snapshots"].as_array().unwrap().is_empty());
    std::fs::remove_file(store.join("spec/registry.json")).unwrap();
    let resumed = common::spec(&code, &["project", "init", "--mode", "global-first"], &env);
    assert_eq!(resumed.code, 1, "{}", resumed.stderr);
    assert!(resumed.stderr.contains("setup_conflict"));
    assert_eq!(std::fs::read(path).unwrap(), bytes);
}
