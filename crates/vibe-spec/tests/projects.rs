#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Project management controls (spec-projects-mcp/3).
mod common;
use serde_json::Value;
use std::path::Path;
fn run(cwd: &Path, store: &Path, args: &[&str]) -> common::Out {
    common::spec(
        cwd,
        args,
        &[
            ("VIBE_STORE", store.to_str().unwrap()),
            ("GIT_CONFIG_NOSYSTEM", "1"),
        ],
    )
}

// spec-ci-portability/2: default Windows storage stays in the fixture, and
// caller-provided APPDATA wins for both process helpers.
#[cfg(windows)]
#[test]
fn windows_default_store_and_explicit_appdata_use_fixture_registration() {
    for stdin in [false, true] {
        for override_appdata in [false, true] {
            let owner = common::project("projects-windows-default-store");
            let code = owner.join("code");
            std::fs::create_dir(&code).unwrap();
            let appdata = owner.join(if override_appdata {
                "custom-appdata"
            } else {
                ".appdata"
            });
            let store = appdata.join("vibe");
            let initial = run(&code, &store, &["project", "init", "--mode", "repo-first"]);
            assert_eq!(initial.code, 0, "{}", initial.stderr);
            let initial: Value = serde_json::from_slice(&initial.stdout).unwrap();
            let explicit = [("APPDATA", appdata.to_str().unwrap())];
            let env = if override_appdata {
                explicit.as_slice()
            } else {
                &[]
            };
            let listed = if stdin {
                common::spec_stdin(&owner, &["project", "list"], env, b"")
            } else {
                common::spec(&owner, &["project", "list"], env)
            };
            assert_eq!(listed.code, 0, "{}", listed.stderr);
            let listed: Value = serde_json::from_slice(&listed.stdout).unwrap();
            assert_eq!(listed["projects"].as_array().unwrap().len(), 1);
            assert_eq!(listed["projects"][0]["id"], initial["project"]["id"]);
        }
    }
}

#[test]
fn project_init_is_idempotent_and_preserves_existing_content() {
    let owner = common::project("projects-idempotent");
    let store = owner.join("store");
    let code = owner.join("code");
    std::fs::create_dir(&code).unwrap();
    common::write(&code, "docs/spec/product.md", "user owned\n");
    let first = run(
        &code,
        &store,
        &["project", "init", "--mode", "repo-first", "--name", "One"],
    );
    assert_eq!(first.code, 0, "{}", first.stderr);
    let initial: Value = serde_json::from_slice(&first.stdout).unwrap();
    let second = run(&code, &store, &["project", "init", "--mode", "repo-first"]);
    assert_eq!(second.code, 0, "{}", second.stderr);
    let again: Value = serde_json::from_slice(&second.stdout).unwrap();
    assert_eq!(initial["project"]["id"], again["project"]["id"]);
    assert_eq!(
        std::fs::read(code.join("docs/spec/product.md")).unwrap(),
        b"user owned\n"
    );
    let listed = run(&code, &store, &["project", "list"]);
    let listed: Value = serde_json::from_slice(&listed.stdout).unwrap();
    assert_eq!(listed["projects"].as_array().unwrap().len(), 1);
    assert!(listed["projects"][0]["checkouts"][0]["common_dir"].is_null());
}
#[test]
fn explicit_stale_registry_revision_is_refused_before_seeding() {
    let owner = common::project("projects-stale");
    let code = owner.join("code");
    std::fs::create_dir(&code).unwrap();
    let store = owner.join("store");
    let out = run(
        &code,
        &store,
        &[
            "project",
            "init",
            "--mode",
            "repo-first",
            "--expect",
            "wrong",
        ],
    );
    assert_eq!(out.code, 1, "{}", out.stderr);
    assert!(out.stderr.contains("revision_changed"), "{}", out.stderr);
    assert!(!code.join("docs").exists());
    assert!(!store.join("spec/registry.json").exists());
}
#[test]
fn global_store_inside_checkout_is_refused_before_any_effect() {
    let owner = common::project("projects-inner-store");
    common::git(&owner, "2026-01-01T00:00:00Z", &["init", "-b", "main"]);
    let store = owner.join("nested/store");
    let out = run(
        &owner,
        &store,
        &["project", "init", "--mode", "global-first"],
    );
    assert_eq!(out.code, 1, "{}", out.stderr);
    assert!(
        out.stderr.contains("store_inside_checkout"),
        "{}",
        out.stderr
    );
    assert!(!owner.join("nested").exists());
}
fn setup_code(root: &Path) {
    std::fs::create_dir_all(root).unwrap();
    common::git(root, "2026-01-01T00:00:00Z", &["init", "-b", "main"]);
    common::git(
        root,
        "2026-01-01T00:00:00Z",
        &["commit", "--allow-empty", "-m", "initial"],
    );
}
#[test]
fn ordinary_remote_clone_auto_matches_but_forks_ports_and_host_aliases_do_not() {
    let owner = common::project("projects-clones");
    let store = owner.join("store");
    let one = owner.join("one");
    setup_code(&one);
    common::git(
        &one,
        "2026-01-01T00:00:00Z",
        &["remote", "add", "origin", "git@github.com:Org/Repo.git"],
    );
    let first = run(&one, &store, &["project", "init", "--mode", "global-first"]);
    assert_eq!(first.code, 0, "{}", first.stderr);
    let id: Value = serde_json::from_slice(&first.stdout).unwrap();
    for (n, url, matching) in [
        (
            "equivalent",
            "https://user:secret@github.com:443/Org/Repo.git/",
            true,
        ),
        ("fork", "https://github.com/Fork/Repo", false),
        ("port", "ssh://git@github.com:2222/Org/Repo.git", false),
        ("alias", "git@work-github:Org/Repo.git", false),
        ("case", "https://github.com/org/Repo", false),
        ("local", "file:///Org/Repo", false),
    ] {
        let code = owner.join(n);
        setup_code(&code);
        common::git(
            &code,
            "2026-01-01T00:00:00Z",
            &["remote", "add", "origin", url],
        );
        let out = run(&code, &store, &["context"]);
        if matching {
            assert_eq!(out.code, 0, "{}", out.stderr);
            let value: Value = serde_json::from_slice(&out.stdout).unwrap();
            assert_eq!(value["project_id"], id["project"]["id"]);
        } else {
            assert_ne!(out.code, 0, "{n}: incorrectly matched {}", out.text());
        }
    }
    let registry = std::fs::read_to_string(store.join("spec/registry.json")).unwrap();
    assert!(registry.contains("github.com/Org/Repo"));
    assert!(!registry.contains("secret"));
    assert!(!registry.contains("git@"));
}
#[test]
fn common_directory_worktree_matches_without_attachment() {
    let owner = common::project("projects-common");
    let store = owner.join("store");
    let code = owner.join("code");
    setup_code(&code);
    let first = run(
        &code,
        &store,
        &["project", "init", "--mode", "global-first"],
    );
    assert_eq!(first.code, 0, "{}", first.stderr);
    let other = owner.join("other");
    common::git(
        &code,
        "2026-01-01T00:00:00Z",
        &["worktree", "add", "--detach", other.to_str().unwrap()],
    );
    common::git(
        &other,
        "2026-01-01T00:00:00Z",
        &["switch", "-c", "topic/other"],
    );
    let created = run(
        &other,
        &store,
        &["branch", "init", "topic/other", "--from", "main"],
    );
    assert_eq!(created.code, 0, "{}", created.stderr);
    assert_eq!(
        common::git_out(&code, &["branch", "--show-current"]),
        "main"
    );
    assert_eq!(
        common::git_out(&other, &["branch", "--show-current"]),
        "topic/other"
    );
}
#[test]
fn mode_migration_and_ambiguous_remote_aliases_fail_closed() {
    let owner = common::project("projects-ambiguous");
    let store = owner.join("store");
    let code = owner.join("code");
    setup_code(&code);
    common::git(
        &code,
        "2026-01-01T00:00:00Z",
        &["remote", "add", "origin", "https://github.com/Org/Repo.git"],
    );
    let first = run(
        &code,
        &store,
        &["project", "init", "--mode", "global-first"],
    );
    assert_eq!(first.code, 0, "{}", first.stderr);
    let stale = std::fs::read(store.join("spec/registry.json")).unwrap();
    let refused = run(&code, &store, &["project", "init", "--mode", "repo-first"]);
    assert_eq!(refused.code, 1);
    assert!(refused.stderr.contains("mode_migration_required"));
    assert_eq!(
        std::fs::read(store.join("spec/registry.json")).unwrap(),
        stale
    );
    let mut reg: Value = serde_json::from_slice(&stale).unwrap();
    let mut duplicate = reg["projects"][0].clone();
    duplicate["id"] = "p-duplicate".into();
    duplicate["main_checkout"] = owner.join("missing").to_str().unwrap().into();
    duplicate["checkouts"][0]["path"] = duplicate["main_checkout"].clone();
    duplicate["checkouts"][0]["common_dir"] = owner.join("missing/.git").to_str().unwrap().into();
    reg["projects"].as_array_mut().unwrap().push(duplicate);
    std::fs::write(store.join("spec/registry.json"), reg.to_string()).unwrap();
    let clone = owner.join("clone");
    setup_code(&clone);
    common::git(
        &clone,
        "2026-01-01T00:00:00Z",
        &["remote", "add", "origin", "git@github.com:Org/Repo.git"],
    );
    let out = run(&clone, &store, &["context"]);
    assert_eq!(out.code, 1);
    assert!(out.stderr.contains("ambiguous_project"), "{}", out.stderr);
}
#[test]
fn two_simultaneous_setup_processes_create_one_project() {
    let owner = common::project("projects-concurrent");
    let store = owner.join("store");
    let a = owner.join("a");
    let b = owner.join("b");
    for code in [&a, &b] {
        setup_code(code);
        common::git(
            code,
            "2026-01-01T00:00:00Z",
            &[
                "remote",
                "add",
                "origin",
                "https://github.com/Org/Concurrent.git",
            ],
        );
    }
    let outputs = std::thread::scope(|s| {
        let first = s.spawn(|| run(&a, &store, &["project", "init", "--mode", "global-first"]));
        let second = s.spawn(|| run(&b, &store, &["project", "init", "--mode", "global-first"]));
        [first.join().unwrap(), second.join().unwrap()]
    });
    assert!(outputs.iter().any(|o| o.code == 0), "{outputs:?}");
    for out in &outputs {
        if out.code != 0 {
            let changed = out
                .stderr
                .strip_prefix("revision_changed: ")
                .is_some_and(|message| !message.trim().is_empty() && message.lines().count() == 1);
            let busy = out.stderr
                == "locked: storage is locked; verify the holder before manual recovery\n";
            assert!(
                out.code == 1 && out.stdout.is_empty() && (changed || busy),
                "{out:?}"
            );
        }
    }
    // spec-ci-portability/2: bounded contention must still converge after exit.
    assert!(
        fs_metadata_absent(&store.join("spec/registry.lock")),
        "setup child retained the registry lock"
    );
    let listed = run(&a, &store, &["project", "list"]);
    assert_eq!(listed.code, 0, "{}", listed.stderr);
    let value: Value = serde_json::from_slice(&listed.stdout).unwrap();
    assert_eq!(value["projects"].as_array().unwrap().len(), 1);
    let id = value["projects"][0]["id"].clone();
    for out in outputs.iter().filter(|out| out.code == 0) {
        let published: Value = serde_json::from_slice(&out.stdout).unwrap();
        assert_eq!(published["project"]["id"], id);
    }
    for code in [&a, &b] {
        let retry = run(code, &store, &["project", "init", "--mode", "global-first"]);
        assert_eq!(retry.code, 0, "{}", retry.stderr);
        let registered: Value = serde_json::from_slice(&retry.stdout).unwrap();
        assert_eq!(registered["project"]["id"], id);
        let context = run(code, &store, &["context"]);
        assert_eq!(context.code, 0, "{}", context.stderr);
        let context: Value = serde_json::from_slice(&context.stdout).unwrap();
        assert_eq!(context["project_id"], id);
        assert_eq!(context["code_available"], true);
        assert_eq!(
            std::fs::canonicalize(context["checkout"].as_str().unwrap()).unwrap(),
            std::fs::canonicalize(code).unwrap()
        );
    }
    let listed = run(&a, &store, &["project", "list"]);
    assert_eq!(listed.code, 0, "{}", listed.stderr);
    let value: Value = serde_json::from_slice(&listed.stdout).unwrap();
    assert_eq!(value["projects"].as_array().unwrap().len(), 1);
    assert_eq!(value["projects"][0]["id"], id);
    assert_eq!(
        value["projects"][0]["checkouts"].as_array().unwrap().len(),
        2
    );
    assert!(fs_metadata_absent(&store.join("spec/registry.lock")));
    assert!(fs_metadata_absent(
        &store
            .join("spec/projects")
            .join(id.as_str().unwrap())
            .join("project.lock")
    ));
}

fn fs_metadata_absent(path: &Path) -> bool {
    std::fs::symlink_metadata(path).is_err_and(|error| error.kind() == std::io::ErrorKind::NotFound)
}
#[test]
fn explicit_attachment_admits_a_fork_and_can_replace_main() {
    let owner = common::project("projects-attach");
    let store = owner.join("store");
    let one = owner.join("one");
    let fork = owner.join("fork");
    setup_code(&one);
    setup_code(&fork);
    common::git(
        &one,
        "2026-01-01T00:00:00Z",
        &["remote", "add", "origin", "https://github.com/Org/Original"],
    );
    common::git(
        &fork,
        "2026-01-01T00:00:00Z",
        &[
            "remote",
            "add",
            "origin",
            "https://github.com/Fork/Original",
        ],
    );
    let first = run(&one, &store, &["project", "init", "--mode", "global-first"]);
    assert_eq!(first.code, 0, "{}", first.stderr);
    let first: Value = serde_json::from_slice(&first.stdout).unwrap();
    let id = first["project"]["id"].as_str().unwrap();
    let out = run(
        &fork,
        &store,
        &["project", "attach", "--project", id, "--main"],
    );
    assert_eq!(out.code, 0, "{}", out.stderr);
    let value: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(
        std::fs::canonicalize(value["project"]["main_checkout"].as_str().unwrap()).unwrap(),
        std::fs::canonicalize(&fork).unwrap()
    );
    assert_eq!(value["project"]["checkouts"].as_array().unwrap().len(), 2);
    let out = run(&fork, &store, &["context"]);
    assert_eq!(out.code, 0, "{}", out.stderr);
    let value: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(value["project_id"], id);
}
#[test]
fn non_git_checkout_selection_resolves_registered_project() {
    let owner = common::project("projects-nongit-context");
    let store = owner.join("store");
    let code = owner.join("code");
    std::fs::create_dir(&code).unwrap();
    let first = run(&code, &store, &["project", "init", "--mode", "repo-first"]);
    assert_eq!(first.code, 0, "{}", first.stderr);
    let first: Value = serde_json::from_slice(&first.stdout).unwrap();
    let out = run(
        &owner,
        &store,
        &["context", "--checkout", code.to_str().unwrap()],
    );
    assert_eq!(out.code, 0, "{}", out.stderr);
    let value: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(value["project_id"], first["project"]["id"]);
    assert_eq!(
        std::fs::canonicalize(value["checkout"].as_str().unwrap()).unwrap(),
        std::fs::canonicalize(&code).unwrap()
    );
    assert_eq!(value["code_available"], true);
    assert!(value["code_ref"].is_null());
    let out = run(&code, &store, &["context"]);
    assert_eq!(out.code, 0, "{}", out.stderr);
    let value: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(value["project_id"], first["project"]["id"]);
}
