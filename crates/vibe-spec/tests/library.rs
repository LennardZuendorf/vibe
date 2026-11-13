#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Library contract and frozen standalone differential coverage (spec-projects-mcp/1).
mod common;
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::path::Path;
use vibe_spec::{Runtime, run_cli};

fn runtime(cwd: &Path) -> Runtime {
    Runtime {
        cwd: cwd.to_path_buf(),
        env: BTreeMap::from([
            ("HOME".into(), cwd.as_os_str().to_owned()),
            ("PATH".into(), std::env::var_os("PATH").unwrap_or_default()),
            ("LC_ALL".into(), "C".into()),
            ("TZ".into(), "UTC".into()),
            ("VIBE_STORE".into(), cwd.join("store").into_os_string()),
            ("GIT_CONFIG_NOSYSTEM".into(), "1".into()),
            (
                "GIT_CONFIG_GLOBAL".into(),
                cwd.join("gitconfig").into_os_string(),
            ),
            (
                "GIT_CONFIG_SYSTEM".into(),
                cwd.join("gitconfig-system").into_os_string(),
            ),
        ]),
    }
}
fn invoke(runtime: &Runtime, args: &[OsString]) -> vibe_spec::CommandOutput {
    let mut argv = vec![OsString::from("vibe-spec")];
    argv.extend_from_slice(args);
    run_cli(&argv, runtime, &mut &b""[..])
}

#[test]
fn explicit_runtime_isolation() {
    let a = common::project("library-isolation-a");
    let b = common::project("library-isolation-b");
    common::write(&a, "first/lessons.md", "# Lessons\n");
    common::write(&b, "second/lessons.md", "# Lessons\n");
    let before_cwd = std::env::current_dir().unwrap();
    let before_env: BTreeMap<_, _> = std::env::vars_os().collect();
    let mut ra = runtime(&a);
    let mut rb = runtime(&b);
    ra.env.insert("VIBE_SPEC_ROOT".into(), "first".into());
    rb.env.insert("VIBE_SPEC_ROOT".into(), "second".into());
    std::thread::scope(|s| {
        for (r, expected) in [(&ra, "first"), (&rb, "second")] {
            s.spawn(move || {
                for _ in 0..8 {
                    let out = invoke(r, &["root".into(), "--json".into()]);
                    assert_eq!(out.exit, 0);
                    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
                    assert!(v.to_string().contains(expected));
                    assert!(out.stderr.is_empty());
                }
            });
        }
    });
    let error = invoke(&ra, &["feature".into(), "INVALID".into()]);
    assert_eq!(error.exit, 2);
    assert!(String::from_utf8_lossy(&error.stderr).contains("invalid feature name"));
    assert_eq!(std::env::current_dir().unwrap(), before_cwd);
    assert_eq!(std::env::vars_os().collect::<BTreeMap<_, _>>(), before_env);
}

#[test]
fn legacy_argv_tail_parity() {
    parity_with(Path::new(env!("CARGO_BIN_EXE_vibe-spec")), false);
}

#[test]
#[ignore = "requires the controller's frozen pre-edit standalone binary"]
fn frozen_legacy_binary_parity() {
    let oracle =
        common::repo().join(".superpowers/sdd/2026-11-11-spec-projects-mcp/oracle-vibe-spec");
    assert!(
        oracle.is_file(),
        "capture the pre-edit binary before running this acceptance test"
    );
    parity_with(&oracle, true);
}

fn check_additive_top_help(old: &[u8], new: &[u8]) -> Result<(), String> {
    fn sections(bytes: &[u8]) -> Result<(&str, &str, &str, &str), String> {
        let text = std::str::from_utf8(bytes).map_err(|e| e.to_string())?;
        let (header, text) = text.split_once("\nCommands:\n").ok_or("missing Commands")?;
        let (commands, text) = text.split_once("\nOptions:\n").ok_or("missing Options")?;
        let (options, footer) = text
            .split_once("\nEnvironment:\n")
            .ok_or("missing Environment")?;
        Ok((header, commands, options, footer))
    }
    fn rows(text: &str, commands: bool) -> Result<Vec<(&str, &str)>, String> {
        let mut rows = Vec::new();
        let mut keys = BTreeSet::new();
        for original in text.lines() {
            let text = if commands {
                original.strip_prefix("  ").ok_or("command indentation")?
            } else {
                original.trim_start_matches(' ')
            };
            let (key, description) = text.split_once("  ").unwrap_or((text, ""));
            if key.is_empty()
                || !keys.insert(key)
                || (commands && (key.contains(' ') || description.is_empty()))
                || (!commands && !key.starts_with('-'))
            {
                return Err(format!("invalid or duplicate help row: {original}"));
            }
            rows.push((
                key,
                if commands {
                    original
                } else {
                    description.trim_start_matches(' ')
                },
            ));
        }
        Ok(rows)
    }
    fn retained(
        old: &[(&str, &str)],
        new: &[(&str, &str)],
        approved: &[&str],
    ) -> Result<(), String> {
        let approved: BTreeSet<_> = approved.iter().copied().collect();
        let mut additions = BTreeSet::new();
        let mut next = 0;
        for row in new {
            if old.get(next).is_some_and(|legacy| legacy.0 == row.0) {
                if old[next] != *row {
                    return Err(format!("changed legacy help row: {}", row.0));
                }
                next += 1;
            } else if approved.contains(row.0) {
                additions.insert(row.0);
            } else {
                return Err(format!("unexpected or reordered help row: {}", row.0));
            }
        }
        if next != old.len() || additions != approved {
            return Err("missing legacy or approved help row".into());
        }
        Ok(())
    }
    let (old_header, old_commands, old_options, old_footer) = sections(old)?;
    let (new_header, new_commands, new_options, new_footer) = sections(new)?;
    if old_header != new_header || old_footer != new_footer {
        return Err("changed help synopsis, environment or footer".into());
    }
    retained(
        &rows(old_commands, true)?,
        &rows(new_commands, true)?,
        &[
            "mcp",
            "reconcile",
            "document",
            "snapshot",
            "project",
            "context",
            "branch",
        ],
    )?;
    retained(
        &rows(old_options, false)?,
        &rows(new_options, false)?,
        &[
            "--project <PROJECT>",
            "--checkout <CHECKOUT>",
            "--branch <BRANCH>",
        ],
    )?;
    Ok(())
}

#[test]
fn strict_additive_help_rejects_legacy_and_unapproved_changes() {
    let old = concat!(
        "The spec tool.\n\nUsage: vibe-spec [OPTIONS] <COMMAND>\n\nCommands:\n",
        "  root     Show root\n",
        "  help     Print command help\n",
        "\nOptions:\n",
        "      --root <DIR>  Root directory\n",
        "  -h, --help        Print help\n",
        "  -V, --version     Print version\n",
        "\nEnvironment:\n  VIBE_SPEC_ROOT root\nExit: 0 ok\n",
    );
    let new = concat!(
        "The spec tool.\n\nUsage: vibe-spec [OPTIONS] <COMMAND>\n\nCommands:\n",
        "  mcp      Serve tools\n",
        "  reconcile  Reconcile specs\n",
        "  document  Edit documents\n",
        "  snapshot  Record edits\n",
        "  project  Register projects\n",
        "  context  Resolve selection\n",
        "  branch   Initialize branches\n",
        "  root     Show root\n",
        "  help     Print command help\n",
        "\nOptions:\n",
        "      --root <DIR>             Root directory\n",
        "      --project <PROJECT>      Select project\n",
        "      --checkout <CHECKOUT>\n",
        "      --branch <BRANCH>\n",
        "  -h, --help                  Print help\n",
        "  -V, --version               Print version\n",
        "\nEnvironment:\n  VIBE_SPEC_ROOT root\nExit: 0 ok\n",
    );
    assert_eq!(
        check_additive_top_help(old.as_bytes(), new.as_bytes()),
        Ok(())
    );
    let mutations = [
        ("command description", new.replace("Show root", "Choose root")),
        ("option description", new.replace("Root directory", "Another directory")),
        ("legacy removal", new.replace("  root     Show root\n", "")),
        ("legacy order", new.replace(
            "  root     Show root\n  help     Print command help\n",
            "  help     Print command help\n  root     Show root\n",
        )),
        ("unexpected command", new.replace("\nOptions:\n", "  extra    Not approved\n\nOptions:\n")),
        ("duplicate command", new.replace("  mcp      Serve tools\n", "  mcp      Serve tools\n  mcp      Serve tools\n")),
        ("missing addition", new.replace("  branch   Initialize branches\n", "")),
        ("unexpected selector", new.replace("\nEnvironment:\n", "      --extra <EXTRA>\n\nEnvironment:\n")),
        ("duplicate selector", new.replace("      --branch <BRANCH>\n", "      --branch <BRANCH>\n      --branch <BRANCH>\n")),
        ("legacy option removal", new.replace("  -V, --version               Print version\n", "")),
        ("legacy option order", new.replace(
            "  -h, --help                  Print help\n  -V, --version               Print version\n",
            "  -V, --version               Print version\n  -h, --help                  Print help\n",
        )),
        ("synopsis", new.replace("[OPTIONS]", "[FLAGS]")),
        ("environment", new.replace("VIBE_SPEC_ROOT root", "VIBE_SPEC_ROOT elsewhere")),
        ("footer", new.replace("Exit: 0 ok", "Exit: 0 done")),
        ("command padding", new.replace("  root     Show root", "  root      Show root")),
    ];
    for (name, changed) in mutations {
        assert_ne!(changed, new, "negative control did not mutate {name}");
        assert!(
            check_additive_top_help(old.as_bytes(), changed.as_bytes()).is_err(),
            "{name}"
        );
    }
}

fn parity_with(oracle: &Path, additive_top_help: bool) {
    let p = common::layered_tree("library-frozen");
    let r = runtime(&p);
    let cases: Vec<Vec<OsString>> = vec![
        vec!["root".into(), "--json".into()],
        vec!["--help".into()],
        vec!["wrong".into()],
        vec!["--root".into(), "docs/spec".into(), "root".into()],
        vec!["--root=docs/spec".into(), "lessons-for".into(), "--".into()],
        vec!["scan-merges".into(), "--".into(), "auth".into()],
        vec!["list-specs".into()],
        vec![
            "lessons-for".into(),
            "rust".into(),
            "--format".into(),
            "json".into(),
        ],
        vec!["feature".into(), "INVALID".into()],
        vec!["validate".into(), "--json".into()],
        vec!["promote".into(), "--help".into()],
        vec!["check-drift".into()],
    ];
    for args in &cases {
        if additive_top_help && args.len() == 1 && args[0] == "--help" {
            let want = standalone(oracle, &r, args);
            let got = invoke_standalone_argv(oracle, &r, args);
            assert_eq!(want.status.code(), Some(0));
            assert_eq!(got.exit, 0);
            assert!(want.stderr.is_empty() && got.stderr.is_empty());
            assert_eq!(check_additive_top_help(&want.stdout, &got.stdout), Ok(()));
        } else {
            assert_frozen(oracle, &r, args);
        }
    }
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStringExt;
        let mut nonunicode = r.clone();
        nonunicode.env.insert(
            "VIBE_SPEC_ROOT".into(),
            OsString::from_vec(vec![b'n', 0xff]),
        );
        nonunicode
            .env
            .insert("PATH".into(), OsString::from_vec(vec![b'/', 0xff]));
        assert_frozen(oracle, &nonunicode, &["root".into(), "--json".into()]);
        assert_frozen(
            oracle,
            &nonunicode,
            &["scan-merges".into(), OsString::from_vec(vec![0xff])],
        );
        assert_frozen(
            oracle,
            &r,
            &[
                "--root".into(),
                OsString::from_vec(vec![0xff]),
                "root".into(),
            ],
        );
    }
    for args in [
        vec!["init"],
        vec!["add", "."],
        vec!["commit", "-m", "fixture"],
        vec!["checkout", "--detach"],
    ] {
        let out = std::process::Command::new("git")
            .args([
                "-c",
                "commit.gpgsign=false",
                "-c",
                "core.hooksPath=/dev/null",
                "-c",
                "user.name=fixture",
                "-c",
                "user.email=fixture@example.invalid",
                "-c",
                "init.defaultBranch=main",
            ])
            .args(args)
            .current_dir(&r.cwd)
            .env_clear()
            .envs(&r.env)
            .env("GIT_AUTHOR_DATE", "2026-01-15T00:00:00Z")
            .env("GIT_COMMITTER_DATE", "2026-01-15T00:00:00Z")
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
    assert_frozen(oracle, &r, &["root".into(), "--json".into()]);
    assert_frozen(oracle, &r, &["trace".into(), "--json".into()]);
}
fn standalone(oracle: &Path, r: &Runtime, args: &[OsString]) -> std::process::Output {
    let mut command = std::process::Command::new(oracle);
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.arg0("vibe-spec");
    }
    command
        .args(args)
        .current_dir(&r.cwd)
        .env_clear()
        .envs(&r.env)
        .output()
        .unwrap()
}
fn assert_frozen(oracle: &Path, r: &Runtime, args: &[OsString]) {
    let want = standalone(oracle, r, args);
    let got = invoke_standalone_argv(oracle, r, args);
    assert_eq!(
        (i32::from(got.exit), &got.stdout, &got.stderr),
        (want.status.code().unwrap(), &want.stdout, &want.stderr),
        "{args:?}"
    );
}

fn invoke_standalone_argv(
    oracle: &Path,
    runtime: &Runtime,
    args: &[OsString],
) -> vibe_spec::CommandOutput {
    // Unix standalone() sets arg0 explicitly; Windows keeps the real basename.
    #[cfg(unix)]
    let program = {
        let _ = oracle;
        OsString::from("vibe-spec")
    };
    #[cfg(not(unix))]
    let program = oracle.file_name().unwrap().to_owned();
    let mut argv = vec![program];
    argv.extend_from_slice(args);
    run_cli(&argv, runtime, &mut &b""[..])
}

#[cfg(unix)]
#[test]
fn runtime_relative_path_drives_external_lint_without_host_env() {
    // spec-projects-mcp/1: relative PATH resolves against the supplied cwd.
    let p = common::project("library-relative-path");
    common::write(
        &p,
        "docs/spec/design.md",
        "---\ntype: entrypoint\nupdated: 2026-01-15\nchildren: []\n---\n# Design\n",
    );
    use std::os::unix::fs::PermissionsExt;
    for name in ["node", "npx", "curl", "sh"] {
        let body = if name == "sh" {
            "#!/bin/sh\nprintf '%s' \"$RUNTIME_MARKER\" > \"$HOME/lint-marker\"\nprintf '%s' \"$PATH\" > \"$HOME/path-marker\"\nexit 0\n"
        } else {
            "#!/bin/sh\nexit 0\n"
        };
        common::write(&p, &format!("bin/{name}"), body);
        std::fs::set_permissions(
            p.join("bin").join(name),
            std::fs::Permissions::from_mode(0o755),
        )
        .unwrap();
    }
    let mut r = runtime(&p);
    use std::os::unix::ffi::OsStringExt;
    let path = OsString::from_vec(vec![b'b', b'i', b'n', b':', b'/', 0xff]);
    r.env.insert("PATH".into(), path.clone());
    r.env.insert("VIBE_DESIGN_LINT".into(), "1".into());
    r.env
        .insert("RUNTIME_MARKER".into(), "explicit-child-env".into());
    let out = invoke(&r, &["validate".into()]);
    assert_eq!(
        std::fs::read(p.join("lint-marker")).unwrap_or_default(),
        b"explicit-child-env",
        "{}",
        String::from_utf8_lossy(&out.stdout)
    );
    assert_eq!(
        std::fs::read(p.join("path-marker")).unwrap(),
        path.into_vec()
    );
}

#[cfg(unix)]
#[test]
fn standalone_help_still_works_after_cwd_is_removed() {
    // spec-projects-mcp/1: argument parsing precedes cwd discovery.
    let p = common::project("library-removed-cwd");
    let gone = p.join("gone");
    std::fs::create_dir(&gone).unwrap();
    let r = runtime(&p);
    let out = std::process::Command::new("/bin/sh")
        .args([
            "-c",
            "cd \"$1\" && /bin/rmdir \"$1\" && exec \"$2\" --help",
            "sh",
        ])
        .arg(&gone)
        .arg(env!("CARGO_BIN_EXE_vibe-spec"))
        .env_clear()
        .envs(&r.env)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(String::from_utf8_lossy(&out.stdout).contains("Usage: vibe-spec"));
}

#[cfg(unix)]
#[test]
fn runtime_lookup_preserves_permission_denied() {
    // spec-projects-mcp/1: do not collapse an inaccessible PATH candidate into ENOENT.
    let p = common::project("library-path-permission");
    common::write(&p, "bin/git", "#!/bin/sh\nexit 0\n");
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(p.join("bin/git"), std::fs::Permissions::from_mode(0o644)).unwrap();
    let mut r = runtime(&p);
    r.env.insert("PATH".into(), p.join("bin").into_os_string());
    let out = invoke(&r, &["setup".into(), "--git-hooks".into()]);
    assert_eq!(out.exit, 1);
    assert_eq!(
        out.stderr,
        b"vibe-spec setup: cannot run git: Permission denied (os error 13)\n"
    );
}

#[cfg(windows)]
#[test]
fn windows_environment_names_match_native_child_precedence() {
    // spec-projects-mcp/1: Windows compares names without case; last native insertion wins.
    let p = common::project("library-windows-env");
    common::write(&p, "selected/lessons.md", "# Lessons\n");
    let mut r = runtime(&p);
    r.env.insert("VIBE_SPEC_ROOT".into(), "missing".into());
    r.env.insert("vIbE_sPeC_rOoT".into(), "selected".into());
    let out = invoke(&r, &["root".into(), "--json".into()]);
    assert_eq!(out.exit, 0, "{}", String::from_utf8_lossy(&out.stderr));
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["source"], "env");
    assert_eq!(v["root"], p.join("selected").to_string_lossy().as_ref());
}

#[cfg(windows)]
#[test]
fn windows_path_spelling_and_duplicates_reach_the_selected_executable() {
    // spec-projects-mcp/1: ordinary `Path` and duplicate variants must agree with Command.envs.
    let p = common::project("library-windows-path");
    std::fs::create_dir_all(p.join("bin")).unwrap();
    std::fs::copy(env!("CARGO_BIN_EXE_vibe-spec"), p.join("bin/git.exe")).unwrap();
    for keys in [vec!["Path"], vec!["PATH", "Path", "path"]] {
        let mut r = runtime(&p);
        r.env.remove(std::ffi::OsStr::new("PATH"));
        for (i, key) in keys.iter().enumerate() {
            let dir = if i + 1 == keys.len() {
                "bin"
            } else {
                "missing"
            };
            r.env.insert((*key).into(), p.join(dir).into_os_string());
        }
        let out = invoke(&r, &["setup".into(), "--git-hooks".into()]);
        assert_eq!(out.exit, 1);
        // The selected executable is the spec binary, so it rejects git's argv.
        // A lookup failure instead says `cannot run git`.
        assert_eq!(out.stderr, b"vibe-spec setup: not a git repository\n");
    }
}

#[cfg(not(windows))]
#[test]
fn unix_environment_names_remain_case_sensitive() {
    // spec-projects-mcp/1: Windows normalization must not affect other platforms.
    let p = common::project("library-case-sensitive-env");
    common::write(&p, "docs/spec/lessons.md", "# Lessons\n");
    common::write(&p, "alternate/lessons.md", "# Lessons\n");
    let mut r = runtime(&p);
    r.env.insert("vibe_spec_root".into(), "alternate".into());
    let out = invoke(&r, &["root".into(), "--json".into()]);
    assert_eq!(out.exit, 0);
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["source"], "docs-spec");
}
