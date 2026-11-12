#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Explicit project and checkout context (spec-projects-mcp/2).
mod common;

use serde_json::{Value, json};
use std::collections::BTreeMap;
#[cfg(all(unix, not(target_os = "macos")))]
use std::ffi::OsString;
use std::fs;
use std::path::Path;
use std::process::Command;
use vibe_spec::{
    BranchId, ContextSelector, ProjectId, RecordId, Runtime, SpecApi, StorageMode, resolve,
};

fn runtime(cwd: &Path, store: &Path) -> Runtime {
    Runtime {
        cwd: cwd.to_owned(),
        env: BTreeMap::from([
            ("HOME".into(), cwd.as_os_str().to_owned()),
            ("PATH".into(), std::env::var_os("PATH").unwrap_or_default()),
            ("VIBE_STORE".into(), store.as_os_str().to_owned()),
            ("LC_ALL".into(), "C".into()),
            ("GIT_CONFIG_NOSYSTEM".into(), "1".into()),
            (
                "GIT_CONFIG_GLOBAL".into(),
                cwd.join("no-gitconfig").into_os_string(),
            ),
            (
                "GIT_CONFIG_SYSTEM".into(),
                cwd.join("no-system-config").into_os_string(),
            ),
        ]),
    }
}
fn git(r: &Runtime, args: &[&std::ffi::OsStr]) -> String {
    let out = Command::new("git")
        .current_dir(&r.cwd)
        .env_clear()
        .envs(&r.env)
        .args([
            "-c",
            "commit.gpgsign=false",
            "-c",
            "core.hooksPath=/dev/null",
            "-c",
            "user.name=test",
            "-c",
            "user.email=t@t",
        ])
        .args(args)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).trim().to_owned()
}
fn g(r: &Runtime, args: &[&str]) -> String {
    git(r, &args.iter().map(|a| a.as_ref()).collect::<Vec<_>>())
}
fn init(r: &Runtime) {
    fs::create_dir_all(&r.cwd).unwrap();
    g(r, &["init", "-b", "main"]);
    g(r, &["commit", "--allow-empty", "-m", "initial"]);
}
fn fixture(name: &str) -> (vibe_core::testscratch::Scratch, Runtime) {
    let owner = vibe_core::testscratch::scratch("context", name);
    let r = runtime(&owner.join("code"), &owner.join("store"));
    init(&r);
    (owner, r)
}
fn project_record(r: &Runtime, mode: &str) -> Value {
    json!({"id":"p-one", "name":"One", "mode":mode,
        "main_checkout":r.cwd, "checkouts":[{"path":r.cwd,
        "common_dir":g(r, &["rev-parse", "--path-format=absolute", "--git-common-dir"])}],
        "remote_aliases":[]})
}
fn register(owner: &Path, r: &Runtime, mode: &str) {
    common::write(
        owner,
        "store/spec/registry.json",
        &json!({"v":1,"projects":[project_record(r,mode)]}).to_string(),
    );
    common::write(owner, "store/spec/projects/p-one/project.json", &json!({"v":1,
        "branches":[
            {"id":"b-main","code_ref":"refs/heads/main","spec_ref":"refs/heads/spec/b-main","parent_spec_revision":null,"last_code_head":null,"last_code_ref":null},
            {"id":"b-other","code_ref":"refs/heads/topic/other","spec_ref":"refs/heads/spec/b-other","parent_spec_revision":null,"last_code_head":null,"last_code_ref":null}
        ],"snapshots":[]}).to_string());
    for branch in ["b-main", "b-other"] {
        common::write(
            owner,
            &format!("store/spec/projects/p-one/branches/{branch}/spec/product.md"),
            branch,
        );
        fs::create_dir_all(owner.join(format!(
            "store/spec/projects/p-one/branches/{branch}/superpowers/plans"
        )))
        .unwrap();
    }
}
fn selected() -> ContextSelector {
    ContextSelector {
        project: Some(ProjectId::new("p-one").unwrap()),
        ..Default::default()
    }
}

#[test]
fn main_checkout_uses_actual_branch() {
    let (owner, r) = fixture("actual-main");
    register(&owner, &r, "global-first");
    g(&r, &["switch", "-c", "topic/other"]);
    let mut outside = r.clone();
    outside.cwd = owner.to_path_buf();
    let cx = resolve(&outside, &selected()).unwrap();
    assert_eq!(cx.mode, StorageMode::GlobalFirst);
    assert_eq!(cx.code_ref.as_deref(), Some("refs/heads/topic/other"));
    assert_eq!(cx.code_head, Some(g(&r, &["rev-parse", "HEAD"])));
    assert_eq!(cx.checkout, Some(fs::canonicalize(&r.cwd).unwrap()));
    assert!(cx.code_available);
    assert_eq!(
        cx.spec_root,
        owner.join("store/spec/projects/p-one/branches/b-other/spec")
    );
    assert_eq!(
        cx.artifact_root,
        owner.join("store/spec/projects/p-one/branches/b-other/superpowers")
    );
    assert_eq!(
        cx.require_code().unwrap(),
        fs::canonicalize(&r.cwd).unwrap()
    );
}

#[test]
fn worktree_selects_itself() {
    let (owner, r) = fixture("worktree");
    register(&owner, &r, "global-first");
    let wt = owner.join("worktree");
    git(
        &r,
        &[
            "worktree".as_ref(),
            "add".as_ref(),
            "-b".as_ref(),
            "topic/other".as_ref(),
            wt.as_os_str(),
        ],
    );
    let mut wr = r.clone();
    wr.cwd = wt.clone();
    let cx = resolve(&wr, &ContextSelector::default()).unwrap();
    assert_eq!(cx.project_id, Some(ProjectId::new("p-one").unwrap()));
    assert_eq!(cx.checkout, Some(fs::canonicalize(wt).unwrap()));
    assert_eq!(cx.code_ref.as_deref(), Some("refs/heads/topic/other"));
    assert!(cx.spec_root.ends_with("branches/b-other/spec"));
}

#[test]
fn explicit_branch_never_checks_out_code() {
    let (owner, r) = fixture("explicit");
    register(&owner, &r, "global-first");
    g(&r, &["branch", "topic/other"]);
    common::write(&r.cwd, "dirty.txt", "keep");
    let before = g(&r, &["status", "--porcelain=v1"]);
    let cx = resolve(
        &r,
        &ContextSelector {
            branch: Some("topic/other".into()),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(cx.code_ref.as_deref(), Some("refs/heads/topic/other"));
    assert!(!cx.code_available);
    assert_eq!(cx.checkout, None);
    assert_eq!(cx.code_head, None);
    assert_eq!(g(&r, &["symbolic-ref", "HEAD"]), "refs/heads/main");
    assert_eq!(g(&r, &["status", "--porcelain=v1"]), before);
    assert_eq!(fs::read(r.cwd.join("dirty.txt")).unwrap(), b"keep");
}

#[test]
fn global_branch_without_checkout() {
    let (owner, r) = fixture("off-checkout");
    register(&owner, &r, "global-first");
    let mut outside = r.clone();
    outside.cwd = owner.to_path_buf();
    let cx = resolve(
        &outside,
        &ContextSelector {
            branch: Some("topic/other".into()),
            ..selected()
        },
    )
    .unwrap();
    assert_eq!(
        fs::read(cx.spec_root.join("product.md")).unwrap(),
        b"b-other"
    );
    assert!(!cx.code_available);
    assert_eq!(
        cx.require_code().unwrap_err().code,
        "code_context_unavailable"
    );
    assert_eq!(cx.code_head, None);
}

#[test]
fn legacy_detached_read() {
    let (_owner, r) = fixture("detached");
    common::write(&r.cwd, "docs/spec/product.md", "legacy");
    g(&r, &["switch", "--detach"]);
    let cx = resolve(&r, &ContextSelector::default()).unwrap();
    assert_eq!(cx.mode, StorageMode::RepoFirst);
    assert_eq!(cx.code_ref, None);
    assert_eq!(cx.code_head, Some(g(&r, &["rev-parse", "HEAD"])));
    assert!(cx.code_available);
    assert_eq!(cx.spec_root, r.cwd.join("docs/spec"));
    assert_eq!(cx.artifact_root, r.cwd.join("docs/superpowers"));
}

#[test]
fn unborn_head_is_null() {
    let owner = vibe_core::testscratch::scratch("context", "unborn");
    let r = runtime(&owner.join("code"), &owner.join("store"));
    fs::create_dir_all(&r.cwd).unwrap();
    g(&r, &["init", "-b", "main"]);
    common::write(&r.cwd, "docs/spec/product.md", "unborn");
    let cx = resolve(&r, &ContextSelector::default()).unwrap();
    assert_eq!(cx.code_ref.as_deref(), Some("refs/heads/main"));
    assert_eq!(cx.code_head, None);
    assert!(cx.code_available);
}

#[test]
fn legacy_nested_discovery_and_relative_override_keep_cwd_semantics() {
    let (_owner, mut r) = fixture("nested-root");
    common::write(&r.cwd, "docs/spec/product.md", "top");
    common::write(&r.cwd, "packages/nested/docs/spec/product.md", "nearest");
    common::write(&r.cwd, "packages/nested/custom/product.md", "override");
    r.cwd = r.cwd.join("packages/nested");
    let cx = resolve(&r, &ContextSelector::default()).unwrap();
    assert_eq!(cx.spec_root, r.cwd.join("docs/spec"));
    r.env.insert("VIBE_SPEC_ROOT".into(), "custom".into());
    assert_eq!(
        resolve(&r, &ContextSelector::default()).unwrap().spec_root,
        r.cwd.join("custom")
    );
}

#[test]
fn non_git_repo_first_retains_available_local_code_anchor() {
    let owner = vibe_core::testscratch::scratch("context", "non-git");
    let r = runtime(&owner, &owner.join("store"));
    common::write(&owner, "docs/spec/product.md", "local files");
    common::write(&owner, "src/main.rs", "fn main() {}");
    let cx = resolve(&r, &ContextSelector::default()).unwrap();
    assert_eq!(cx.mode, StorageMode::RepoFirst);
    assert_eq!(cx.checkout, Some(owner.to_path_buf()));
    assert!(cx.code_available);
    assert_eq!(cx.require_code().unwrap(), owner.as_ref());
    assert_eq!(cx.code_ref, None);
    assert_eq!(cx.code_head, None);
    assert_eq!(cx.spec_root, owner.join("docs/spec"));
}

#[test]
fn branch_switch_to_uninitialized_storage_reports_context_changed() {
    let (owner, r) = fixture("stale-unknown");
    register(&owner, &r, "global-first");
    let selector = ContextSelector::default();
    let old = resolve(&r, &selector).unwrap();
    g(&r, &["switch", "-c", "new-code-branch"]);
    assert_eq!(
        old.ensure_current(&r, &selector).unwrap_err().code,
        "context_changed"
    );
}

#[test]
fn explicit_checkout_branch_conflict_is_not_redirected() {
    let (owner, r) = fixture("checkout-conflict");
    register(&owner, &r, "global-first");
    let wt = owner.join("other-checkout");
    git(
        &r,
        &[
            "worktree".as_ref(),
            "add".as_ref(),
            "-b".as_ref(),
            "topic/other".as_ref(),
            wt.as_os_str(),
        ],
    );
    let error = resolve(
        &r,
        &ContextSelector {
            checkout: Some(r.cwd.clone()),
            branch: Some("topic/other".into()),
            ..selected()
        },
    )
    .unwrap_err();
    assert_eq!(error.code, "context_mismatch");
}

#[test]
fn branch_switch_stales_token() {
    let (owner, r) = fixture("stale-branch");
    register(&owner, &r, "global-first");
    let selector = ContextSelector::default();
    let old = resolve(&r, &selector).unwrap();
    g(&r, &["switch", "-c", "topic/other"]);
    let new = resolve(&r, &selector).unwrap();
    assert_ne!(old.revision_token, new.revision_token);
    assert_eq!(
        old.ensure_current(&r, &selector).unwrap_err().code,
        "context_changed"
    );
}

#[test]
fn document_and_artifact_changes_stale_token() {
    let (owner, r) = fixture("stale-content");
    register(&owner, &r, "global-first");
    let selector = ContextSelector::default();
    let first = resolve(&r, &selector).unwrap();
    fs::write(first.spec_root.join("product.md"), "edited").unwrap();
    assert_eq!(
        first.ensure_current(&r, &selector).unwrap_err().code,
        "revision_changed"
    );
    let second = resolve(&r, &selector).unwrap();
    common::write(&second.artifact_root, "plans/2026-11-11-one.md", "a plan");
    assert_ne!(
        second.revision_token,
        resolve(&r, &selector).unwrap().revision_token
    );
}

#[test]
fn global_override_mismatch_and_repo_precedence() {
    let (owner, mut r) = fixture("overrides");
    register(&owner, &r, "global-first");
    common::write(&r.cwd, "override/product.md", "decoy");
    r.env.insert("VIBE_SPEC_ROOT".into(), "override".into());
    assert_eq!(
        resolve(&r, &ContextSelector::default()).unwrap_err().code,
        "context_mismatch"
    );
    let want = owner.join("store/spec/projects/p-one/branches/b-main/spec");
    r.env
        .insert("VIBE_SPEC_ROOT".into(), want.as_os_str().to_owned());
    assert_eq!(
        resolve(&r, &ContextSelector::default()).unwrap().spec_root,
        want
    );
    register(&owner, &r, "repo-first");
    r.env.insert("VIBE_SPEC_ROOT".into(), "override".into());
    assert_eq!(
        resolve(&r, &ContextSelector::default()).unwrap().spec_root,
        r.cwd.join("override")
    );
}

#[test]
fn global_config_override_cannot_redirect_registered_storage() {
    let (owner, r) = fixture("config-override");
    register(&owner, &r, "global-first");
    common::write(&r.cwd, "elsewhere/product.md", "decoy");
    common::write(&r.cwd, ".vibe/spec.json", r#"{"root":"elsewhere"}"#);
    assert_eq!(
        resolve(&r, &ContextSelector::default()).unwrap_err().code,
        "context_mismatch"
    );
}

#[cfg(unix)]
#[test]
fn symlinked_global_artifact_ancestor_is_rejected() {
    use std::os::unix::fs::symlink;
    let (owner, r) = fixture("symlink-artifacts");
    register(&owner, &r, "global-first");
    let anchor = owner.join("store/spec/projects/p-one/branches/b-main/superpowers");
    fs::remove_dir_all(&anchor).unwrap();
    fs::create_dir_all(owner.join("outside")).unwrap();
    symlink(owner.join("outside"), &anchor).unwrap();
    assert_eq!(
        resolve(&r, &ContextSelector::default()).unwrap_err().code,
        "invalid_registry"
    );
    assert_eq!(fs::read_dir(owner.join("outside")).unwrap().count(), 0);
}

#[test]
fn malformed_branch_records_and_historical_metadata_are_rejected() {
    let (owner, r) = fixture("bad-branches");
    register(&owner, &r, "global-first");
    let target = owner.join("store/spec/projects/p-one/project.json");
    let valid: Value = serde_json::from_slice(&fs::read(&target).unwrap()).unwrap();
    for (key, bad) in [
        ("id", json!("../escape")),
        ("code_ref", json!("refs/heads/a b")),
        ("spec_ref", json!("refs/heads/../escape")),
        ("parent_spec_revision", json!("not-an-oid")),
        ("last_code_head", json!("not-an-oid")),
        ("last_code_ref", json!("main")),
    ] {
        let mut v = valid.clone();
        v["branches"][0][key] = bad;
        fs::write(&target, v.to_string()).unwrap();
        assert_eq!(
            resolve(&r, &ContextSelector::default()).unwrap_err().code,
            "invalid_registry",
            "{key}"
        );
    }
}

#[test]
fn repo_branch_requires_matching_checkout() {
    let (owner, r) = fixture("repo-other");
    register(&owner, &r, "repo-first");
    common::write(&r.cwd, "docs/spec/product.md", "main");
    let e = resolve(
        &r,
        &ContextSelector {
            branch: Some("topic/other".into()),
            ..Default::default()
        },
    )
    .unwrap_err();
    assert_eq!(e.code, "code_context_unavailable");
    assert_eq!(g(&r, &["symbolic-ref", "HEAD"]), "refs/heads/main");
}

#[test]
fn branch_only_repo_first_selection_uses_selected_checkout_roots() {
    let (owner, r) = fixture("branch-only-roots");
    register(&owner, &r, "repo-first");
    common::write(&r.cwd, "docs/spec/product.md", "main specs");
    let wt = owner.join("topic-checkout");
    git(
        &r,
        &[
            "worktree".as_ref(),
            "add".as_ref(),
            "-b".as_ref(),
            "topic/other".as_ref(),
            wt.as_os_str(),
        ],
    );
    let mut topic = r.clone();
    topic.cwd = wt.clone();
    common::write(&wt, "docs/spec/product.md", "topic specs");
    let target = owner.join("store/spec/registry.json");
    let mut registry: Value = serde_json::from_slice(&fs::read(&target).unwrap()).unwrap();
    registry["projects"][0]["checkouts"].as_array_mut().unwrap().push(json!({
        "path":wt,"common_dir":g(&topic,&["rev-parse","--path-format=absolute","--git-common-dir"])
    }));
    fs::write(target, registry.to_string()).unwrap();
    let selector = ContextSelector {
        branch: Some("topic/other".into()),
        ..Default::default()
    };
    let cx = resolve(&r, &selector).unwrap();
    let code = fs::canonicalize(&wt).unwrap();
    assert_eq!(cx.checkout, Some(code.clone()));
    assert_eq!(cx.code_ref.as_deref(), Some("refs/heads/topic/other"));
    assert_eq!(cx.spec_root, code.join("docs/spec"));
    assert_eq!(cx.artifact_root, code.join("docs/superpowers"));
    assert_eq!(
        fs::read(cx.spec_root.join("product.md")).unwrap(),
        b"topic specs"
    );
    assert_eq!(g(&r, &["symbolic-ref", "HEAD"]), "refs/heads/main");
}

#[test]
fn moving_sibling_document_into_empty_directory_changes_revision() {
    let (_owner, r) = fixture("token-directory-boundaries");
    common::write(&r.cwd, "docs/spec/b.md", "unchanged bytes");
    fs::create_dir_all(r.cwd.join("docs/spec/a")).unwrap();
    let selector = ContextSelector::default();
    let before = resolve(&r, &selector).unwrap();
    fs::rename(r.cwd.join("docs/spec/b.md"), r.cwd.join("docs/spec/a/b.md")).unwrap();
    let after = resolve(&r, &selector).unwrap();
    assert_ne!(before.revision_token, after.revision_token);
    assert_eq!(
        before.ensure_current(&r, &selector).unwrap_err().code,
        "revision_changed"
    );
}

#[test]
fn moving_document_between_spec_and_artifact_roots_changes_revision() {
    let (_owner, r) = fixture("token-root-boundaries");
    common::write(&r.cwd, "docs/spec/b.md", "unchanged bytes");
    fs::create_dir_all(r.cwd.join("docs/superpowers")).unwrap();
    let selector = ContextSelector::default();
    let before = resolve(&r, &selector).unwrap();
    fs::rename(
        r.cwd.join("docs/spec/b.md"),
        r.cwd.join("docs/superpowers/b.md"),
    )
    .unwrap();
    let after = resolve(&r, &selector).unwrap();
    assert_ne!(before.revision_token, after.revision_token);
    assert_eq!(
        before.ensure_current(&r, &selector).unwrap_err().code,
        "revision_changed"
    );
}

#[test]
fn ambient_git_configuration_cannot_retarget_selected_checkout() {
    let (owner, r) = fixture("git-config-injection");
    register(&owner, &r, "global-first");
    let (_other, other) = fixture("git-config-decoy");
    for kind in [
        "count",
        "parameters",
        "global-file",
        "config-file",
        "system-file",
    ] {
        let mut injected = r.clone();
        match kind {
            "count" => {
                injected.env.insert("GIT_CONFIG_COUNT".into(), "1".into());
                injected
                    .env
                    .insert("GIT_CONFIG_KEY_0".into(), "core.worktree".into());
                injected.env.insert(
                    "GIT_CONFIG_VALUE_0".into(),
                    other.cwd.as_os_str().to_owned(),
                );
            }
            "parameters" => {
                injected.env.insert(
                    "GIT_CONFIG_PARAMETERS".into(),
                    format!("'core.worktree={}'", other.cwd.display()).into(),
                );
            }
            name => {
                let file = owner.join("injected-gitconfig");
                fs::write(
                    &file,
                    format!("[core]\nworktree = {}\n", other.cwd.display()),
                )
                .unwrap();
                let key = match name {
                    "global-file" => "GIT_CONFIG_GLOBAL",
                    "system-file" => "GIT_CONFIG_SYSTEM",
                    _ => "GIT_CONFIG",
                };
                injected.env.insert(key.into(), file.into_os_string());
                if name == "system-file" {
                    injected
                        .env
                        .remove(std::ffi::OsStr::new("GIT_CONFIG_NOSYSTEM"));
                }
            }
        }
        let cx = resolve(
            &injected,
            &ContextSelector {
                checkout: Some(r.cwd.clone()),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(
            cx.checkout,
            Some(fs::canonicalize(&r.cwd).unwrap()),
            "{kind}"
        );
        assert_eq!(cx.code_ref.as_deref(), Some("refs/heads/main"), "{kind}");
        assert_eq!(
            cx.project_id,
            Some(ProjectId::new("p-one").unwrap()),
            "{kind}"
        );
    }
}

#[test]
fn repository_configuration_cannot_redirect_git_top_outside_selection() {
    let (_owner, r) = fixture("git-top-containment");
    let (_other, other) = fixture("git-top-decoy");
    common::write(&other.cwd, "docs/spec/product.md", "foreign specs");
    git(
        &r,
        &[
            "config".as_ref(),
            "core.worktree".as_ref(),
            other.cwd.as_os_str(),
        ],
    );
    let error = resolve(
        &r,
        &ContextSelector {
            checkout: Some(r.cwd.clone()),
            ..Default::default()
        },
    )
    .unwrap_err();
    assert_eq!(error.code, "context_mismatch");
    // A valid local setting must still allow discovery inside its actual worktree.
    git(
        &r,
        &[
            "config".as_ref(),
            "core.worktree".as_ref(),
            r.cwd.as_os_str(),
        ],
    );
    common::write(&r.cwd, "docs/spec/product.md", "local specs");
    let mut inside = r.clone();
    fs::create_dir_all(r.cwd.join("src/deep")).unwrap();
    inside.cwd = r.cwd.join("src/deep");
    assert_eq!(
        resolve(&inside, &ContextSelector::default())
            .unwrap()
            .checkout,
        Some(fs::canonicalize(&r.cwd).unwrap())
    );
}

#[test]
fn inherited_git_locators_cannot_redirect_selected_checkout() {
    let (owner, mut r) = fixture("git-locators");
    register(&owner, &r, "global-first");
    let (_other, other) = fixture("git-locator-decoy");
    g(&other, &["switch", "-c", "decoy"]);
    r.env
        .insert("GIT_DIR".into(), other.cwd.join(".git").into_os_string());
    r.env.insert(
        "GIT_COMMON_DIR".into(),
        other.cwd.join(".git").into_os_string(),
    );
    r.env
        .insert("GIT_WORK_TREE".into(), other.cwd.as_os_str().to_owned());
    let cx = resolve(&r, &ContextSelector::default()).unwrap();
    assert_eq!(cx.code_ref.as_deref(), Some("refs/heads/main"));
    assert_eq!(cx.checkout, Some(fs::canonicalize(&r.cwd).unwrap()));
    assert!(cx.spec_root.ends_with("branches/b-main/spec"));
}

#[test]
fn independent_api_contexts_do_not_mutate_process() {
    let (a, ra) = fixture("api-a");
    let (b, rb) = fixture("api-b");
    register(&a, &ra, "global-first");
    register(&b, &rb, "global-first");
    let before_cwd = std::env::current_dir().unwrap();
    let before_env: BTreeMap<_, _> = std::env::vars_os().collect();
    let aa = SpecApi::new(ra).unwrap();
    let bb = SpecApi::new(rb).unwrap();
    std::thread::scope(|s| {
        for (api, owner) in [(&aa, &a), (&bb, &b)] {
            s.spawn(move || {
                let cx = api.resolve_context(&ContextSelector::default()).unwrap();
                assert_eq!(
                    cx.spec_root,
                    owner.join("store/spec/projects/p-one/branches/b-main/spec")
                );
            });
        }
    });
    assert_eq!(std::env::current_dir().unwrap(), before_cwd);
    assert_eq!(std::env::vars_os().collect::<BTreeMap<_, _>>(), before_env);
}

#[test]
fn invalid_ids_and_corrupt_registration_fail_closed() {
    for id in ["", ".", "..", "a/b", "a\\b", "-option", "a b", "雪"] {
        assert!(ProjectId::new(id).is_err(), "{id}");
        assert!(BranchId::new(id).is_err());
        assert!(RecordId::new(id).is_err());
    }
    let (owner, r) = fixture("corrupt");
    register(&owner, &r, "global-first");
    let path = owner.join("store/spec/registry.json");
    let valid: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    for bad in [
        json!({"v":99,"projects":[]}),
        json!({"v":1,"projects":{}}),
        {
            let mut v = valid.clone();
            v["projects"][0]["id"] = json!("../outside");
            v
        },
        {
            let mut v = valid.clone();
            v["projects"][0]["checkouts"] = json!(null);
            v
        },
    ] {
        fs::write(&path, bad.to_string()).unwrap();
        assert_eq!(
            resolve(&r, &ContextSelector::default()).unwrap_err().code,
            "invalid_registry"
        );
    }
}

#[test]
fn missing_main_checkout_and_conflicting_project_are_errors() {
    let (owner, r) = fixture("selectors");
    register(&owner, &r, "global-first");
    let e = resolve(
        &r,
        &ContextSelector {
            project: Some(ProjectId::new("p-missing").unwrap()),
            ..Default::default()
        },
    )
    .unwrap_err();
    assert_eq!(e.code, "project_not_found");
    let mut outside = r.clone();
    outside.cwd = owner.to_path_buf();
    fs::rename(&r.cwd, owner.join("moved")).unwrap();
    assert_eq!(
        resolve(&outside, &selected()).unwrap_err().code,
        "checkout_unavailable"
    );
    // Explicit global branch reads remain available after the main checkout disappears.
    assert!(
        !resolve(
            &outside,
            &ContextSelector {
                branch: Some("main".into()),
                ..selected()
            }
        )
        .unwrap()
        .code_available
    );
}

#[cfg(all(unix, not(target_os = "macos")))]
#[test]
fn os_string_checkout_and_store_are_not_lossy() {
    use std::os::unix::ffi::OsStringExt;
    let owner = vibe_core::testscratch::scratch("context", "raw-path");
    let code = owner.join(OsString::from_vec(b"code-\xff".to_vec()));
    let store = owner.join(OsString::from_vec(b"store-\xfe".to_vec()));
    let initial = runtime(&owner.join("initial"), &store);
    init(&initial);
    fs::rename(&initial.cwd, &code).unwrap();
    // The macOS runner cannot launch any child with a non-UTF-8 process cwd.
    // Select the raw checkout explicitly from its ordinary parent instead.
    let r = runtime(&owner, &store);
    common::write(&code, "docs/spec/product.md", "raw");
    let selector = ContextSelector {
        checkout: Some(code.clone()),
        ..Default::default()
    };
    let cx = resolve(&r, &selector).unwrap();
    assert_eq!(cx.checkout, Some(fs::canonicalize(&code).unwrap()));
    assert_eq!(cx.spec_root, code.join("docs/spec"));
    let api = SpecApi::new(r).unwrap();
    assert_eq!(
        api.resolve_context(&selector).unwrap().spec_root,
        code.join("docs/spec")
    );
}

#[test]
fn spec_projects_mcp_8_explicit_project_outranks_other_ambient_checkout() {
    use vibe_spec::{InitProjectRequest, SpecApi, SpecRequest, StorageMode};
    let (owner, a) = fixture("explicit-project-over-ambient");
    register(&owner, &a, "global-first");
    let mut b = a.clone();
    b.cwd = owner.join("other-code");
    init(&b);
    let api_a = SpecApi::new(a.clone()).unwrap();
    let registry = api_a.execute(SpecRequest::Projects).unwrap();
    let api_b = SpecApi::new(b.clone()).unwrap();
    let initialized = api_b
        .execute(SpecRequest::ProjectInit(InitProjectRequest {
            selector: ContextSelector::default(),
            mode: StorageMode::GlobalFirst,
            name: Some("Other".into()),
            expected_revision: registry.data["revision"].as_str().unwrap().into(),
        }))
        .unwrap();
    let id = initialized
        .context
        .as_ref()
        .unwrap()
        .project_id
        .clone()
        .unwrap();
    let selector = ContextSelector {
        project: Some(id.clone()),
        ..Default::default()
    };
    // An invalid spec config and invalid document in A must not participate in B.
    common::write(&a.cwd, ".vibe/spec.json", "{invalid config");
    fs::create_dir_all(a.cwd.join("docs/spec")).unwrap();
    fs::write(a.cwd.join("docs/spec/poison.md"), [0xff]).unwrap();
    let before = fs::read(a.cwd.join(".vibe/spec.json")).unwrap();
    let old_cwd = a.cwd.clone();
    let old_env = a.env.clone();
    let selected = api_a.resolve_context(&selector).unwrap();
    assert_eq!(selected.project_id, Some(id.clone()));
    assert_eq!(selected.checkout.as_deref(), Some(b.cwd.as_path()));
    let branch = api_a
        .resolve_context(&ContextSelector {
            branch: Some("main".into()),
            ..selector.clone()
        })
        .unwrap();
    assert_eq!(branch.project_id, Some(id.clone()));
    let output = vibe_spec::run_cli(
        &[
            "vibe-spec".into(),
            "context".into(),
            "--project".into(),
            id.as_str().into(),
        ],
        &a,
        &mut std::io::empty(),
    );
    assert_eq!(
        output.exit,
        0,
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["project_id"], id.as_str());
    let legacy = vibe_spec::run_cli(
        &[
            "vibe-spec".into(),
            "--project".into(),
            id.as_str().into(),
            "validate".into(),
            "--spec-only".into(),
            "--json".into(),
        ],
        &a,
        &mut std::io::empty(),
    );
    assert_eq!(
        legacy.exit,
        0,
        "{}",
        String::from_utf8_lossy(&legacy.stderr)
    );
    let initialized = vibe_spec::run_cli(
        &[
            "vibe-spec".into(),
            "--project".into(),
            id.as_str().into(),
            "init".into(),
            "--json".into(),
        ],
        &a,
        &mut std::io::empty(),
    );
    assert_eq!(
        initialized.exit,
        0,
        "{}",
        String::from_utf8_lossy(&initialized.stderr)
    );
    let mut overridden = a.clone();
    overridden.env.insert(
        "VIBE_SPEC_ROOT".into(),
        a.cwd.join("docs/spec").into_os_string(),
    );
    assert_eq!(
        SpecApi::new(overridden)
            .unwrap()
            .resolve_context(&selector)
            .unwrap_err()
            .code,
        "context_mismatch"
    );
    assert_eq!(
        api_a
            .resolve_context(&ContextSelector {
                checkout: Some(a.cwd.clone()),
                ..selector
            })
            .unwrap_err()
            .code,
        "context_mismatch"
    );
    assert_eq!(a.cwd, old_cwd);
    assert_eq!(a.env, old_env);
    assert_eq!(fs::read(a.cwd.join(".vibe/spec.json")).unwrap(), before);
    assert_eq!(fs::read(a.cwd.join("docs/spec/poison.md")).unwrap(), [0xff]);
}
