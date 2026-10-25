#![allow(clippy::unwrap_used, clippy::expect_used)]
#![cfg(unix)]
//! Real-CLI acceptance: the real `codex`, `droid`, `claude` (and optionally `opencode`)
//! accept the files `vibe install` / `vibe sync` generate. Gated by `VIBE_E2E=1`;
//! `VIBE_E2E_REQUIRE=1` turns a missing CLI into a failure. Every spawned process runs
//! under `env_clear()` with every config location pointed into the scratch home.
mod common;
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use common::{Home, Out};

const DEADLINE: Duration = Duration::from_secs(30);

fn enabled() -> bool {
    if std::env::var_os("VIBE_E2E").is_none() {
        eprintln!("skipped: set VIBE_E2E=1");
        return false;
    }
    true
}

fn which(bin: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|d| d.join(bin))
        .find(|p| p.is_file())
}

/// True when the test should run; a missing CLI skips unless `VIBE_E2E_REQUIRE=1`.
fn ready(bins: &[&str]) -> bool {
    if !enabled() {
        return false;
    }
    for bin in bins {
        if which(bin).is_none() {
            assert!(
                std::env::var_os("VIBE_E2E_REQUIRE").is_none(),
                "VIBE_E2E_REQUIRE=1 but `{bin}` is not on PATH"
            );
            eprintln!("skipped: `{bin}` not on PATH");
            return false;
        }
    }
    true
}

/// The only environment any spawned process gets (plus per-test `VIBE_*` vars).
fn isolate(cmd: &mut Command, home: &Home, claude_dir: &str, extra: &[(&str, &str)]) {
    let d = &home.dir;
    fs::create_dir_all(d.join("tmp")).unwrap();
    cmd.current_dir(d)
        .env_clear()
        .env("PATH", std::env::var_os("PATH").unwrap_or_default())
        .env("HOME", d)
        .env("CODEX_HOME", d.join(".codex"))
        .env("CLAUDE_CONFIG_DIR", d.join(claude_dir))
        .env("XDG_CONFIG_HOME", d.join(".config"))
        .env("XDG_STATE_HOME", d.join(".local/state"))
        .env("XDG_DATA_HOME", d.join(".local/share"))
        .env("XDG_CACHE_HOME", d.join(".cache"))
        .env("TMPDIR", d.join("tmp"))
        .env("LC_ALL", "C");
    for (k, v) in extra {
        assert!(k.starts_with("VIBE_"), "only VIBE_* extras allowed: {k}");
        cmd.env(k, v);
    }
}

fn run_with_deadline(mut cmd: Command, home: &Home, label: &str) -> Out {
    let n = {
        static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    };
    let out_path = home.dir.join("tmp").join(format!("out-{n}"));
    let err_path = home.dir.join("tmp").join(format!("err-{n}"));
    cmd.stdin(Stdio::null())
        .stdout(Stdio::from(fs::File::create(&out_path).unwrap()))
        .stderr(Stdio::from(fs::File::create(&err_path).unwrap()));
    let mut child = cmd.spawn().unwrap_or_else(|e| panic!("{label}: {e}"));
    let start = Instant::now();
    let code = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status.code().unwrap_or(-1);
        }
        if start.elapsed() > DEADLINE {
            let _ = child.kill();
            let _ = child.wait();
            panic!("{label}: timed out after {DEADLINE:?}");
        }
        std::thread::sleep(Duration::from_millis(50));
    };
    Out {
        code,
        stdout: fs::read(&out_path).unwrap(),
        stderr: String::from_utf8_lossy(&fs::read(&err_path).unwrap()).into_owned(),
    }
}

/// Runs a real harness CLI under the isolated environment.
fn real_cli(home: &Home, bin: &str, args: &[&str]) -> Out {
    real_cli_in(home, bin, args, ".claude")
}

/// [`real_cli`] with `CLAUDE_CONFIG_DIR` set to `<home>/<claude_dir>`.
fn real_cli_in(home: &Home, bin: &str, args: &[&str], claude_dir: &str) -> Out {
    let mut cmd = Command::new(which(bin).unwrap_or_else(|| PathBuf::from(bin)));
    cmd.args(args);
    isolate(&mut cmd, home, claude_dir, &[]);
    run_with_deadline(cmd, home, &format!("{bin} {}", args.join(" ")))
}

/// Runs the `vibe` binary under test under the same isolated environment.
fn vibe(home: &Home, args: &[&str]) -> Out {
    vibe_in(home, args, ".claude")
}

/// [`vibe`] with `CLAUDE_CONFIG_DIR` set to `<home>/<claude_dir>`.
fn vibe_in(home: &Home, args: &[&str], claude_dir: &str) -> Out {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_vibe"));
    cmd.args(args);
    isolate(
        &mut cmd,
        home,
        claude_dir,
        &[
            ("VIBE_HTTP_TOKEN", "fake-token"),
            ("VIBE_ECHO_KEY", "fake-key"),
        ],
    );
    run_with_deadline(cmd, home, &format!("vibe {}", args.join(" ")))
}

fn show(out: &Out) -> String {
    format!(
        "code={}\nstdout:\n{}\nstderr:\n{}",
        out.code,
        out.text(),
        out.stderr
    )
}

fn assert_ok(out: &Out) {
    assert_eq!(out.code, 0, "{}", show(out));
}

/// A real-shaped home plus the three-server store, installed.
fn installed(name: &str) -> Home {
    let home = Home::real_shaped(name);
    home.write_json(
        ".config/vibe/store.json",
        r#"{"v":1,"skills":{"sources":[]}}"#,
    );
    home.write_json(
        ".config/vibe/mcp.json",
        r#"{"mcpServers":{
          "echo-srv":{"type":"stdio","command":"node","args":["-e","0"]},
          "env-srv":{"type":"stdio","command":"node","args":["-e","0"],"env":{"ECHO_KEY":"${VIBE_ECHO_KEY}"}},
          "http-srv":{"type":"http","url":"https://mcp.example.com/mcp","headers":{"Authorization":"Bearer ${VIBE_HTTP_TOKEN}"}}
        }}"#,
    );
    home.write(
        ".config/vibe/AGENTS.md",
        "# Global rules\n\nUse the shared tools.\n",
    );
    let out = vibe(&home, &["install"]);
    assert_ok(&out);
    eprintln!("[vibe install]\n{}", out.text());
    home
}

fn json(out: &Out) -> serde_json::Value {
    serde_json::from_slice(&out.stdout).unwrap_or_else(|e| panic!("{e}\n{}", show(out)))
}

#[test]
fn e2e_codex_accepts_generated_config() {
    if !ready(&["codex", "node"]) {
        return;
    }
    let home = installed("e2e-codex");
    eprintln!("[config.toml]\n{}", home.read(".codex/config.toml"));
    let list = real_cli(&home, "codex", &["mcp", "list", "--json"]);
    assert_ok(&list);
    let text = list.text();
    assert!(text.contains("echo-srv"), "{}", show(&list));
    assert!(text.contains("github"), "{}", show(&list));

    let get = real_cli(&home, "codex", &["mcp", "get", "echo-srv", "--json"]);
    assert_ok(&get);
    let v = json(&get);
    assert_eq!(
        v.pointer("/transport/command").or_else(|| v.get("command")),
        Some(&serde_json::json!("node")),
        "{}",
        show(&get)
    );

    let http = real_cli(&home, "codex", &["mcp", "get", "http-srv", "--json"]);
    assert_ok(&http);
    let v = json(&http);
    assert_eq!(
        v.pointer("/transport/bearer_token_env_var")
            .or_else(|| v.get("bearer_token_env_var")),
        Some(&serde_json::json!("VIBE_HTTP_TOKEN")),
        "{}",
        show(&http)
    );
}

#[test]
fn e2e_codex_rejects_duplicate_table_control() {
    if !ready(&["codex", "node"]) {
        return;
    }
    let home = installed("e2e-codex-dup");
    let ok = real_cli(&home, "codex", &["mcp", "list"]);
    assert_ok(&ok);
    let mut toml = home.read(".codex/config.toml");
    toml.push_str("\n[mcp_servers.echo-srv]\ncommand = \"other\"\n");
    home.write(".codex/config.toml", &toml);
    let bad = real_cli(&home, "codex", &["mcp", "list"]);
    assert_ne!(bad.code, 0, "{}", show(&bad));
}

#[test]
fn e2e_droid_lists_generated_mcp() {
    if !ready(&["droid", "node"]) {
        return;
    }
    let home = installed("e2e-droid");
    let out = real_cli(&home, "droid", &["mcp", "list"]);
    assert_ok(&out);
    let text = format!("{}{}", out.text(), out.stderr);
    assert!(text.contains("echo-srv"), "{}", show(&out));
    assert!(text.contains("mesh"), "{}", show(&out));
}

#[test]
fn e2e_claude_user_scope_has_servers() {
    if !ready(&["claude", "node"]) {
        return;
    }
    const CFG: &str = ".claude";
    let home = Home::real_shaped("e2e-claude");
    let seed = real_cli_in(
        &home,
        "claude",
        &[
            "mcp",
            "add-json",
            "--scope",
            "user",
            "user-own",
            r#"{"type":"stdio","command":"user-own-cmd","args":[]}"#,
        ],
        CFG,
    );
    assert_ok(&seed);
    let before = real_cli_in(&home, "claude", &["mcp", "get", "user-own"], CFG);
    assert_ok(&before);

    home.write_json(
        ".config/vibe/store.json",
        r#"{"v":1,"skills":{"sources":[]}}"#,
    );
    home.write_json(
        ".config/vibe/mcp.json",
        r#"{"mcpServers":{
          "echo-srv":{"type":"stdio","command":"node","args":["-e","0"]},
          "env-srv":{"type":"stdio","command":"node","args":["-e","0"],"env":{"ECHO_KEY":"${VIBE_ECHO_KEY}"}},
          "http-srv":{"type":"http","url":"https://mcp.example.com/mcp","headers":{"Authorization":"Bearer ${VIBE_HTTP_TOKEN}"}}
        }}"#,
    );
    let sync = vibe_in(&home, &["sync"], CFG);
    eprintln!("[vibe sync]\n{}", show(&sync));
    assert_ok(&sync);

    let get = real_cli_in(&home, "claude", &["mcp", "get", "echo-srv"], CFG);
    assert_ok(&get);
    assert!(get.text().contains("node"), "{}", show(&get));

    let again = vibe_in(&home, &["sync"], CFG);
    eprintln!("[vibe sync again]\n{}", show(&again));
    assert_ok(&again);
    assert!(
        again.text().lines().any(|l| l == "nothing to do"),
        "{}",
        show(&again)
    );
    assert!(
        !again.text().lines().any(|l| l.starts_with("claude mcp ")),
        "{}",
        show(&again)
    );

    let cfg = home.read(".claude/.claude.json");
    assert!(cfg.contains("${VIBE_ECHO_KEY}"), "{cfg}");
    assert!(cfg.contains("Bearer ${VIBE_HTTP_TOKEN}"), "{cfg}");
    assert!(
        !cfg.contains("fake-token") && !cfg.contains("fake-key"),
        "{cfg}"
    );

    let after = real_cli_in(&home, "claude", &["mcp", "get", "user-own"], CFG);
    assert_ok(&after);
    assert_eq!(before.text(), after.text());

    let un = vibe_in(&home, &["uninstall"], CFG);
    eprintln!("[vibe uninstall]\n{}", show(&un));
    assert_ok(&un);
    for name in ["echo-srv", "env-srv", "http-srv"] {
        let gone = real_cli_in(&home, "claude", &["mcp", "get", name], CFG);
        assert_ne!(gone.code, 0, "{name}: {}", show(&gone));
    }
    let own = real_cli_in(&home, "claude", &["mcp", "get", "user-own"], CFG);
    assert_ok(&own);
}

#[test]
fn e2e_claude_mcp_list_works_with_block() {
    if !ready(&["claude", "node"]) {
        return;
    }
    let home = installed("e2e-claude-block");
    assert!(home.read(".claude/CLAUDE.md").contains("vibe:store"));
    let out = real_cli(&home, "claude", &["mcp", "list"]);
    assert_ok(&out);
}

#[test]
fn e2e_opencode_loads_plugin() {
    if !enabled() {
        return;
    }
    if std::env::var_os("VIBE_E2E_OPENCODE").is_none() {
        eprintln!("skipped: opencode check is fragile (needs a service); set VIBE_E2E_OPENCODE=1");
        return;
    }
    if !ready(&["opencode", "node"]) {
        return;
    }
    let home = installed("e2e-opencode");
    struct StopService<'a>(&'a Home);
    impl Drop for StopService<'_> {
        fn drop(&mut self) {
            let _ = real_cli(self.0, "opencode", &["service", "stop"]);
        }
    }
    let _stop = StopService(&home);
    let port = real_cli(&home, "opencode", &["service", "set", "port", "49871"]);
    eprintln!("[opencode service set port]\n{}", show(&port));
    let out = real_cli(&home, "opencode", &["debug", "config"]);
    assert_ok(&out);
    // `debug config` prints file config only; plugin-injected servers are covered by the node test.
    eprintln!("[opencode debug config]\n{}", out.text());
    assert!(out.text().contains("user-oc"), "{}", show(&out));
}
