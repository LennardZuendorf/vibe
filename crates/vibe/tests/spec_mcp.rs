#![allow(clippy::unwrap_used, clippy::expect_used)]
//! spec-projects-mcp/8: the central binary owns streaming without a peer.
mod common;
use serde_json::{Value, json};
use std::{
    io::{BufRead, BufReader, Write},
    process::{Command, Stdio},
};
#[test]
fn central_only_binary_streams_mcp_without_node_or_peer() {
    let owner = vibe_core::testscratch::scratch("vibe", "mcp-central");
    let mut child = Command::new(env!("CARGO_BIN_EXE_vibe"))
        .args(["spec", "mcp", "--stdio"])
        .current_dir(&owner)
        .env_clear()
        .env("VIBE_STORE", owner.join("store"))
        .env("PATH", "")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut input = child.stdin.take().unwrap();
    let mut output = BufReader::new(child.stdout.take().unwrap());
    writeln!(input,"{}",json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"central","version":"1"}}})).unwrap();
    let mut line = String::new();
    output.read_line(&mut line).unwrap();
    let initialized: Value = serde_json::from_str(&line).unwrap();
    assert_eq!(initialized["result"]["protocolVersion"], "2025-11-25");
    writeln!(
        input,
        "{}",
        json!({"jsonrpc":"2.0","method":"notifications/initialized"})
    )
    .unwrap();
    writeln!(input,"{}",json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"spec_projects","arguments":{}}})).unwrap();
    line.clear();
    output.read_line(&mut line).unwrap();
    let result: Value = serde_json::from_str(&line).unwrap();
    assert_eq!(
        result["result"]["structuredContent"]["data"]["projects"],
        json!([])
    );
    drop(input);
    assert!(child.wait().unwrap().success());
}

fn isolated_smoke_command(
    program: &std::path::Path,
    cwd: &std::path::Path,
    home: &common::Home,
) -> Command {
    let mut command = Command::new(program);
    let path = home.path("bin");
    #[cfg(windows)]
    let path = std::env::join_paths([path, home.path("git-runtime").join("cmd")]).unwrap();
    command
        .current_dir(cwd)
        .env_clear()
        .env("PATH", path)
        .env("HOME", home.path("home"))
        .env("USERPROFILE", home.path("home"))
        .env("XDG_CONFIG_HOME", home.path("config"))
        .env("XDG_DATA_HOME", home.path("data"))
        .env("XDG_CACHE_HOME", home.path("cache"))
        .env("XDG_STATE_HOME", home.path("state"))
        .env("APPDATA", home.path("appdata"))
        .env("LOCALAPPDATA", home.path("localappdata"))
        .env("VIBE_STORE", home.path("store"))
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", home.path("gitconfig"))
        .env("LC_ALL", "C")
        .env("TZ", "UTC");
    #[cfg(windows)]
    for name in ["SystemRoot", "WINDIR"] {
        if let Some(value) = std::env::var_os(name) {
            command.env(name, value);
        }
    }
    command
}

fn git_installation_layout(
    real_git: &std::path::Path,
    exec_path: &std::path::Path,
) -> Option<(std::path::PathBuf, std::path::PathBuf)> {
    let prefix = exec_path.parent()?.parent()?;
    let installation = real_git.ancestors().find(|root| {
        root.join("cmd").join("git.exe").is_file()
            && exec_path.starts_with(root)
            && prefix.starts_with(root)
            && prefix.join("bin").join("git.exe").is_file()
            && prefix
                .join("share")
                .join("git-core")
                .join("templates")
                .is_dir()
    })?;
    Some((
        installation.to_owned(),
        prefix.strip_prefix(installation).ok()?.to_owned(),
    ))
}

// spec-ci-windows-followup/2: installed Git selects its native runtime prefix.
#[test]
fn git_installation_layout_follows_actual_runtime_prefix() {
    let home = common::Home::new("git-runtime-layout");
    for prefix in ["ucrt64", "mingw64"] {
        let installation = home.path(prefix);
        let native = installation.join(prefix);
        let git = installation.join("cmd").join("git.exe");
        let native_git = native.join("bin").join("git.exe");
        let helpers = native.join("libexec").join("git-core");
        let templates = native.join("share").join("git-core").join("templates");
        for dir in [
            git.parent().unwrap(),
            native_git.parent().unwrap(),
            &helpers,
            &templates,
        ] {
            std::fs::create_dir_all(dir).unwrap();
        }
        std::fs::write(&git, b"fixture launcher").unwrap();
        std::fs::write(&native_git, b"fixture native Git").unwrap();
        let git = std::fs::canonicalize(git).unwrap();
        let helpers = std::fs::canonicalize(helpers).unwrap();
        let expected = (std::fs::canonicalize(&installation).unwrap(), prefix.into());
        assert_eq!(git_installation_layout(&git, &helpers), Some(expected));
        assert!(
            git_installation_layout(&git, &std::fs::canonicalize(&home.dir).unwrap()).is_none()
        );
        std::fs::remove_file(&native_git).unwrap();
        assert!(git_installation_layout(&git, &helpers).is_none());
        std::fs::write(&native_git, b"fixture native Git").unwrap();
        std::fs::remove_dir(&templates).unwrap();
        assert!(git_installation_layout(&git, &helpers).is_none());
    }
}

#[cfg(windows)]
fn copy_git_installation(real_git: &std::path::Path, home: &common::Home) -> std::path::PathBuf {
    fn copy_tree(source: &std::path::Path, target: &std::path::Path) {
        std::fs::create_dir(target).unwrap();
        for entry in std::fs::read_dir(source).unwrap() {
            let entry = entry.unwrap();
            let kind = entry.file_type().unwrap();
            let target = target.join(entry.file_name());
            if kind.is_dir() {
                copy_tree(&entry.path(), &target);
            } else {
                assert!(
                    kind.is_file(),
                    "unexpected Git installation link: {:?}",
                    entry.path()
                );
                std::fs::copy(entry.path(), target).unwrap();
            }
        }
    }
    let mut command = isolated_smoke_command(real_git, &home.path("code"), home);
    command.arg("--exec-path");
    let output = smoke_output(command);
    let exec_path =
        std::fs::canonicalize(String::from_utf8(output.stdout).unwrap().trim()).unwrap();
    let (installation, prefix) = git_installation_layout(real_git, &exec_path)
        .expect("complete Git for Windows installation required for copied smoke");
    let runtime = home.path("git-runtime");
    copy_tree(&installation, &runtime);
    assert!(
        runtime
            .join(prefix)
            .join("share")
            .join("git-core")
            .join("templates")
            .is_dir()
    );
    runtime.join("cmd").join("git.exe")
}

fn smoke_output(mut command: Command) -> std::process::Output {
    let output = common::output(&mut command).unwrap();
    assert!(
        output.status.success(),
        "clean central smoke: {}\n{}",
        String::from_utf8_lossy(&output.stderr),
        String::from_utf8_lossy(&output.stdout),
    );
    output
}

fn smoke_json(
    home: &common::Home,
    vibe: &std::path::Path,
    cwd: &std::path::Path,
    args: &[&str],
) -> Value {
    let mut command = isolated_smoke_command(vibe, cwd, home);
    command.args(args);
    serde_json::from_slice(&smoke_output(command).stdout).unwrap()
}

struct SmokeClient {
    child: std::process::Child,
    input: Option<std::process::ChildStdin>,
    lines: std::sync::mpsc::Receiver<String>,
    next: u64,
}
impl SmokeClient {
    fn new(mut command: Command) -> Self {
        command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut child =
            common::retry_text_file_busy(|| common::spawn_guarded(&mut command)).unwrap();
        let input = child.stdin.take();
        let stdout = child.stdout.take().unwrap();
        let (send, lines) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines() {
                if send.send(line.unwrap()).is_err() {
                    break;
                }
            }
        });
        Self {
            child,
            input,
            lines,
            next: 1,
        }
    }
    fn rpc(&mut self, method: &str, params: Value) -> Value {
        let id = self.next;
        self.next += 1;
        writeln!(
            self.input.as_mut().unwrap(),
            "{}",
            json!({
                "jsonrpc":"2.0", "id":id, "method":method, "params":params
            })
        )
        .unwrap();
        let line = self
            .lines
            .recv_timeout(std::time::Duration::from_secs(30))
            .expect("copied central MCP response before EOF");
        let response: Value = serde_json::from_str(&line).expect("JSON-only central stdout");
        assert_eq!(response["id"], id, "{response}");
        assert!(response.get("error").is_none(), "{response}");
        response["result"].clone()
    }
    fn call(&mut self, tool: &str, arguments: Value) -> Value {
        let response = self.rpc("tools/call", json!({"name":tool,"arguments":arguments}));
        assert_ne!(response["isError"], true, "{response}");
        response["structuredContent"]["data"].clone()
    }
    fn close(mut self) {
        self.input.take();
        assert!(
            self.child.wait().unwrap().success(),
            "copied central EOF failed"
        );
        for line in self.lines.try_iter() {
            let _: Value = serde_json::from_str(&line).expect("JSON-only trailing stdout");
        }
    }
}
impl Drop for SmokeClient {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

// spec-projects-mcp/9. Break caught: the copied front door still locates a peer,
// depends on Node/global configuration, or ignores explicit project selection.
#[test]
fn copied_central_vibe_project_mcp_with_git_only() {
    let home = common::Home::new("copied-central-project-mcp");
    let code = home.path("code");
    let neutral = home.path("neutral");
    for directory in [&code, &neutral, &home.path("home")] {
        std::fs::create_dir_all(directory).unwrap();
    }
    home.write("gitconfig", "");
    let vibe_name = format!("vibe{}", std::env::consts::EXE_SUFFIX);
    let git_name = format!("git{}", std::env::consts::EXE_SUFFIX);
    let vibe = home.path("bin").join(&vibe_name);
    let real_git = std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default())
        .map(|directory| directory.join(&git_name))
        .find(|candidate| candidate.is_file())
        .map(|candidate| std::fs::canonicalize(candidate).unwrap())
        .expect("real Git required for clean-install smoke");
    common::copy_exec(std::path::Path::new(env!("CARGO_BIN_EXE_vibe")), &vibe);
    #[cfg(not(windows))]
    let git = home.path("bin").join(&git_name);
    #[cfg(not(windows))]
    common::copy_exec(&real_git, &git);
    #[cfg(windows)]
    let git = copy_git_installation(&real_git, &home);
    let inventory = || {
        std::fs::read_dir(home.path("bin"))
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect::<std::collections::BTreeSet<_>>()
    };
    #[cfg(not(windows))]
    let expected_bin = std::collections::BTreeSet::from([vibe_name, git_name]);
    #[cfg(windows)]
    let expected_bin = std::collections::BTreeSet::from([vibe_name]);
    assert_eq!(inventory(), expected_bin);
    #[cfg(windows)]
    {
        let mut command = isolated_smoke_command(&git, &code, &home);
        let output = smoke_output({
            command.arg("--exec-path");
            command
        });
        let helpers =
            std::fs::canonicalize(String::from_utf8(output.stdout).unwrap().trim()).unwrap();
        assert!(helpers.starts_with(std::fs::canonicalize(home.path("git-runtime")).unwrap()));
        for directory in [home.path("bin"), home.path("git-runtime").join("cmd")] {
            for forbidden in ["node.exe", "vibe-spec.exe"] {
                assert!(
                    !directory.join(forbidden).exists(),
                    "unexpected runtime: {forbidden}"
                );
            }
        }
    }
    for args in [
        vec!["init", "-b", "release-live"],
        vec!["config", "maintenance.auto", "false"],
        vec!["config", "gc.auto", "0"],
        vec![
            "-c",
            "user.name=fixture",
            "-c",
            "user.email=fixture@example.invalid",
            "-c",
            "commit.gpgsign=false",
            "-c",
            "core.hooksPath=/dev/null",
            "commit",
            "--allow-empty",
            "-m",
            "fixture",
        ],
    ] {
        let mut command = isolated_smoke_command(&git, &code, &home);
        command
            .args(args)
            .env("GIT_AUTHOR_DATE", "2026-01-15T00:00:00Z")
            .env("GIT_COMMITTER_DATE", "2026-01-15T00:00:00Z");
        smoke_output(command);
    }
    let initialized = smoke_json(
        &home,
        &vibe,
        &code,
        &["spec", "project", "init", "--mode", "global-first"],
    );
    let project = initialized["project"]["id"].as_str().unwrap();
    let context = smoke_json(
        &home,
        &vibe,
        &neutral,
        &["spec", "context", "--project", project],
    );
    let published_args = ["spec", "--project", project, "mcp", "--stdio"];
    eprintln!("spec-projects-mcp/9 copied central launch: {published_args:?}");
    eprintln!(
        "spec-projects-mcp/9 real Git source: {}",
        real_git.display()
    );
    let mut command = isolated_smoke_command(&vibe, &neutral, &home);
    command.args(published_args);
    let mut client = SmokeClient::new(command);
    assert_eq!(
        client.rpc(
            "initialize",
            json!({
                "protocolVersion":"2025-11-25", "capabilities":{},
                "clientInfo":{"name":"copied-central-smoke","version":"1"}
            })
        )["protocolVersion"],
        "2025-11-25"
    );
    writeln!(
        client.input.as_mut().unwrap(),
        "{}",
        json!({
            "jsonrpc":"2.0", "method":"notifications/initialized"
        })
    )
    .unwrap();
    let selected = client.call("spec_context", json!({}));
    assert_eq!(selected["project_id"], project);
    assert_eq!(selected["mode"], "global-first");
    assert_eq!(selected["code_ref"], "refs/heads/release-live");
    assert_eq!(
        std::fs::canonicalize(selected["checkout"].as_str().unwrap()).unwrap(),
        std::fs::canonicalize(&code).unwrap()
    );
    assert_eq!(selected["code_available"], true);
    for field in [
        "code_head",
        "spec_root",
        "artifact_root",
        "spec_revision",
        "revision_token",
    ] {
        assert_eq!(selected[field], context[field], "{field}");
    }
    let root = std::path::Path::new(selected["spec_root"].as_str().unwrap());
    assert!(
        std::fs::canonicalize(root)
            .unwrap()
            .starts_with(std::fs::canonicalize(home.path("store/spec/projects")).unwrap())
    );
    assert_eq!(
        root.with_file_name("superpowers"),
        std::path::Path::new(selected["artifact_root"].as_str().unwrap())
    );
    let content = "# Lessons\n\nCopied central executable records this fact.\n";
    client.call(
        "spec_write",
        json!({
            "path":"spec/lessons.md", "content":content,
            "expected_revision":selected["revision_token"],
        }),
    );
    assert_eq!(
        client.call("spec_read", json!({"path":"spec/lessons.md"}))["content"],
        content
    );
    assert_eq!(
        client.call("spec_validate", json!({"scope":"spec-only"}))["errors"],
        0
    );
    let after = client.call("spec_context", json!({}));
    assert_ne!(after["spec_revision"], selected["spec_revision"]);
    assert_ne!(after["revision_token"], selected["revision_token"]);
    assert_eq!(after["code_head"], selected["code_head"]);
    assert_eq!(after["code_ref"], selected["code_ref"]);
    client.close();
    assert_eq!(
        smoke_json(
            &home,
            &vibe,
            &neutral,
            &[
                "spec",
                "document",
                "read",
                "spec/lessons.md",
                "--project",
                project,
            ]
        )["content"],
        content
    );
    assert!(!code.join("docs").exists());
    assert_eq!(inventory(), expected_bin);
}
