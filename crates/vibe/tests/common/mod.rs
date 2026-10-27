//! Shared fixtures for the vibe integration tests. Defined once; later tasks only read them.
#![allow(dead_code, clippy::unwrap_used, clippy::expect_used)]
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Output, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, MutexGuard};
use std::time::Duration;

/// What a process produced.
#[derive(Debug)]
pub struct Out {
    pub code: i32,
    pub stdout: Vec<u8>,
    pub stderr: String,
}

impl Out {
    pub fn text(&self) -> String {
        String::from_utf8_lossy(&self.stdout).into_owned()
    }
}

// --- exec race -----------------------------------------------------------------------------
//
// A fixture writes a script (or copies the binary) and then execs it. On Linux the exec can
// fail with `ETXTBSY` — "text file busy" — which does not mean the file is broken: `fork`
// copies every open descriptor, so a thread that starts a child while another thread is still
// writing that file leaves a write descriptor on it in the child until the child's own exec
// closes it. Any exec in that window fails. Two mechanisms, both shared here:
//
//   * [`fork_lock`] closes the window: writing an executable and starting a process never
//     overlap, so no child can inherit a write descriptor on a file about to be exec'd.
//   * [`retry_text_file_busy`] covers what the lock cannot see (a grandchild exec'ing the
//     file, another process outside this test binary) with a bounded backoff.

/// Attempts an exec gets before a still-busy file is reported as a failure.
pub const EXEC_ATTEMPTS: u32 = 50;

static FORK_LOCK: Mutex<()> = Mutex::new(());

/// Held while a process is started and while an executable is written — never across a wait.
fn fork_lock() -> MutexGuard<'static, ()> {
    FORK_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

/// True when `e` says "some process still holds this file open for writing": `ETXTBSY` on
/// Linux and macOS, a sharing violation on Windows.
pub fn is_text_file_busy(e: &io::Error) -> bool {
    if e.kind() == io::ErrorKind::ExecutableFileBusy {
        return true;
    }
    cfg!(windows) && e.raw_os_error() == Some(32)
}

/// Runs `op` while it keeps failing with [`is_text_file_busy`], up to [`EXEC_ATTEMPTS`] times.
/// Any other error is returned immediately and unchanged, so a real failure still fails the test.
pub fn retry_text_file_busy<T, F: FnMut() -> io::Result<T>>(mut op: F) -> io::Result<T> {
    retry_bounded(&mut op, EXEC_ATTEMPTS, Duration::from_millis(1))
}

/// [`retry_text_file_busy`] with an explicit budget: `attempts` tries in total, `first` the
/// wait before the first retry, doubled after each one up to 50 ms.
pub fn retry_bounded<T, F: FnMut() -> io::Result<T>>(
    mut op: F,
    attempts: u32,
    first: Duration,
) -> io::Result<T> {
    let mut delay = first;
    let mut attempt = 1;
    loop {
        match op() {
            Err(e) if is_text_file_busy(&e) && attempt < attempts => {
                std::thread::sleep(delay);
                delay = (delay * 2).min(Duration::from_millis(50));
                attempt += 1;
            }
            other => return other,
        }
    }
}

/// Starts `cmd` under [`fork_lock`].
fn spawn_guarded(cmd: &mut Command) -> io::Result<Child> {
    let _guard = fork_lock();
    cmd.spawn()
}

/// [`Command::output`], guarded and retried: the child is started only while no executable is
/// being written, and a `ETXTBSY` start is retried.
pub fn output(cmd: &mut Command) -> io::Result<Output> {
    cmd.stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    retry_text_file_busy(|| {
        let child = spawn_guarded(cmd)?;
        child.wait_with_output()
    })
}

/// [`Command::status`], guarded and retried, with stdio inherited.
pub fn status(cmd: &mut Command) -> io::Result<ExitStatus> {
    retry_text_file_busy(|| {
        let mut child = spawn_guarded(cmd)?;
        child.wait()
    })
}

/// Writes `text` to `path`, creates the parents, and makes it executable — all before any
/// process can be started, so no child inherits a write descriptor on it.
pub fn write_exec(path: &Path, text: &str) {
    let _guard = fork_lock();
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
    }
}

/// Copies the executable `src` to `path` and makes it executable, under the same lock as
/// [`write_exec`].
pub fn copy_exec(src: &Path, path: &Path) {
    let _guard = fork_lock();
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::copy(src, path).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
    }
}

/// A scratch `HOME` directory, removed and recreated on construction.
pub struct Home {
    pub dir: PathBuf,
}

/// Drops the Windows verbatim prefix that `fs::canonicalize` adds.
fn plain(path: PathBuf) -> PathBuf {
    match path.to_str().and_then(|s| s.strip_prefix(r"\\?\")) {
        Some(rest) if !rest.starts_with(r"UNC\") => PathBuf::from(rest),
        _ => path,
    }
}

/// Re-renders `text` as canonical 2-space JSON ending in a newline.
fn canonical(text: &str) -> String {
    let value = vibe_core::json::parse(text).unwrap();
    let mut out = vibe_core::json::to_pretty(&value);
    if !out.ends_with('\n') {
        out.push('\n');
    }
    out
}

#[cfg(unix)]
fn link(target: &Path, at: &Path) {
    fs::create_dir_all(at.parent().unwrap()).unwrap();
    std::os::unix::fs::symlink(target, at).unwrap();
}

#[cfg(not(unix))]
fn link(target: &Path, at: &Path) {
    fs::create_dir_all(at.parent().unwrap()).unwrap();
    if target.is_dir() {
        fs::create_dir_all(at).unwrap();
    } else {
        fs::copy(target, at).unwrap();
    }
}

impl Home {
    /// A fresh, empty home named after the test.
    pub fn new(name: &str) -> Home {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let n = NEXT.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("vibe-test-{name}-{}-{n}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        Home {
            dir: plain(fs::canonicalize(dir).unwrap()),
        }
    }

    /// The path `rel` under the home.
    pub fn path(&self, rel: &str) -> PathBuf {
        self.dir.join(rel)
    }

    /// `rel` rendered the way the product prints a path on this host: `rel` may be written
    /// with `/`, and `PathBuf` keeps those bytes — but the product builds its paths by joining
    /// components, so on Windows it prints `\`. Compare shown strings, not path literals.
    pub fn shown(&self, rel: &str) -> String {
        self.path(rel)
            .to_string_lossy()
            .replace('/', std::path::MAIN_SEPARATOR_STR)
    }

    /// The store directory as a child `vibe` process resolves it here: `%APPDATA%\vibe` on
    /// Windows (`run` points `APPDATA` at `<home>/AppData/Roaming`), `~/.config/vibe` elsewhere.
    pub fn store_rel(&self) -> &'static str {
        if cfg!(windows) {
            "AppData/Roaming/vibe"
        } else {
            ".config/vibe"
        }
    }

    /// Writes `text` to `rel` under the home, creating parents.
    pub fn write(&self, rel: &str, text: &str) {
        let path = self.path(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }

    /// Writes the executable `text` to `rel` under the home and returns its path. Use this —
    /// never [`Home::write`] plus a later `set_permissions` — for anything that will be exec'd:
    /// it closes the write-descriptor window that makes exec fail with `ETXTBSY`.
    pub fn write_exec(&self, rel: &str, text: &str) -> PathBuf {
        let path = self.path(rel);
        write_exec(&path, text);
        path
    }

    /// Writes `json` to `rel` in canonical 2-space form ending in a newline.
    pub fn write_json(&self, rel: &str, json: &str) {
        self.write(rel, &canonical(json));
    }

    /// Reads `rel` under the home as text.
    pub fn read(&self, rel: &str) -> String {
        fs::read_to_string(self.path(rel)).unwrap()
    }

    /// The directory first on the default `PATH`; `fake_claude` installs its script here.
    pub fn bin_dir(&self) -> PathBuf {
        let dir = self.path("bin");
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn default_path(&self) -> std::ffi::OsString {
        let mut dirs = vec![self.bin_dir()];
        #[cfg(unix)]
        dirs.extend([PathBuf::from("/usr/bin"), PathBuf::from("/bin")]);
        #[cfg(windows)]
        if let Some(real) = std::env::var_os("PATH") {
            dirs.extend(
                std::env::split_paths(&real)
                    .filter(|d| d.to_string_lossy().to_lowercase().contains("system32")),
            );
        }
        std::env::join_paths(dirs).unwrap()
    }

    fn run(&self, args: &[&str], env: &[(&str, &str)], path: std::ffi::OsString) -> Out {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_vibe"));
        cmd.args(args)
            .current_dir(&self.dir)
            .env_clear()
            .env("PATH", path)
            .env("HOME", &self.dir)
            .env("LC_ALL", "C")
            .env("TZ", "UTC");
        #[cfg(windows)]
        {
            cmd.env("USERPROFILE", &self.dir)
                .env("APPDATA", self.dir.join("AppData").join("Roaming"))
                .env("LOCALAPPDATA", self.dir.join("AppData").join("Local"));
            for key in ["SystemRoot", "PATHEXT"] {
                if let Some(v) = std::env::var_os(key) {
                    cmd.env(key, v);
                }
            }
        }
        for (k, v) in env {
            cmd.env(k, v);
        }
        let o = output(&mut cmd).unwrap();
        Out {
            code: o.status.code().unwrap_or(-1),
            stdout: o.stdout,
            stderr: String::from_utf8_lossy(&o.stderr).into_owned(),
        }
    }

    /// Runs `vibe` with a clean environment plus `HOME` and `env`, in the home directory.
    /// `PATH` is `<home>/bin` plus the system directories only, so a real harness CLI is never reached.
    pub fn vibe(&self, args: &[&str], env: &[(&str, &str)]) -> Out {
        self.run(args, env, self.default_path())
    }

    /// Like [`Home::vibe`], with `prefix` prepended to the default `PATH`.
    pub fn vibe_with_path(&self, args: &[&str], env: &[(&str, &str)], prefix: &Path) -> Out {
        let mut dirs = vec![prefix.to_path_buf()];
        dirs.extend(std::env::split_paths(&self.default_path()));
        self.run(args, env, std::env::join_paths(dirs).unwrap())
    }

    /// Like [`Home::vibe`], but inherits the real `PATH`. End-to-end tests only
    /// (they need the real `claude`, `codex`, `droid`, `node`); any other test must use [`Home::vibe`].
    pub fn vibe_real_path(&self, args: &[&str], env: &[(&str, &str)]) -> Out {
        self.run(args, env, std::env::var_os("PATH").unwrap_or_default())
    }

    /// The calls the fake `claude` logged: one argv vector per call (without the program name).
    pub fn claude_calls(&self) -> Vec<Vec<String>> {
        let Ok(log) = fs::read_to_string(self.path("claude-argv.log")) else {
            return Vec::new();
        };
        log.lines()
            .map(|line| {
                line.split_terminator('\u{1f}')
                    .map(str::to_string)
                    .collect()
            })
            .collect()
    }

    /// A home holding a valid store: stdio and http servers, a disabled one, a
    /// codex-only one, root instructions and one skill.
    pub fn with_store(name: &str) -> Home {
        let home = Home::new(name);
        home.write_json(
            ".config/vibe/store.json",
            r#"{"v":1,"harnesses":{"codex":{"enabled":true}},"skills":{"sources":[]}}"#,
        );
        home.write_json(
            ".config/vibe/mcp.json",
            r#"{"mcpServers":{
              "local":{"type":"stdio","command":"npx","args":["-y","some-server"],"env":{"LOG_LEVEL":"debug","API_TOKEN":"${VIBE_LOCAL_API_TOKEN}"}},
              "remote":{"type":"http","url":"https://mcp.example.com/mcp","headers":{"Authorization":"${VIBE_REMOTE_AUTHORIZATION}"}},
              "off":{"command":"off-server","enabled":false},
              "codex-only":{"command":"codex-server","harnesses":["codex"]}
            }}"#,
        );
        home.write(
            ".config/vibe/AGENTS.md",
            "# Global rules\n\nUse the shared tools.\n",
        );
        home.write(
            ".config/vibe/skills/a/SKILL.md",
            "---\nname: a\ndescription: Skill a.\n---\n\nBody.\n",
        );
        home
    }

    /// A home shaped like the owner's real machine (design section 1, "Observed"):
    /// third-party hooks in every harness, user MCP entries everywhere, a symlinked
    /// instruction file, mixed real and foreign-symlink skills, literal keys.
    pub fn real_shaped(name: &str) -> Home {
        let home = Home::new(name);
        let opencode_rules = "# Global rules\n\nPrefer serena, context7, exa, indexed.\n";
        home.write(".config/opencode/AGENTS.md", opencode_rules);
        home.write_json(
            ".config/opencode/opencode.json",
            r#"{"$schema":"https://opencode.ai/config.json","mcp":{"user-oc":{"type":"local","command":["oc-server"]}}}"#,
        );
        link(
            &home.path(".config/opencode/AGENTS.md"),
            &home.path(".claude/AGENTS.md"),
        );
        home.write(
            ".codex/AGENTS.md",
            "# Global rules\n\nA drifted copy of the rules.\n",
        );

        home.write(
            ".agents/skills/hub-skill/SKILL.md",
            "---\nname: hub-skill\ndescription: From the hub.\n---\n",
        );
        home.write(
            ".claude/skills/own-skill/SKILL.md",
            "---\nname: own-skill\ndescription: A real directory.\n---\n",
        );
        home.write(
            ".claude/skills/other-real/SKILL.md",
            "---\nname: other-real\ndescription: Another real directory.\n---\n",
        );
        link(
            &home.path(".agents/skills/hub-skill"),
            &home.path(".claude/skills/hub-skill"),
        );
        link(
            &home.path(".agents/skills/hub-skill"),
            &home.path(".claude/skills/foreign-link"),
        );

        home.write_json(
            ".claude/settings.json",
            r#"{"model":"opus","hooks":{"SessionStart":[{"matcher":"startup","hooks":[{"type":"command","command":"/opt/thirdparty/bin/hook session-start","timeout":5}]}],"Stop":[{"hooks":[{"type":"command","command":"/opt/thirdparty/bin/hook stop"}]}]}}"#,
        );
        home.write_json(
            ".claude.json",
            r#"{"numStartups":7,"mcpServers":{"grafana":{"type":"http","url":"https://grafana.example.com/mcp"},"yab-browser":{"type":"stdio","command":"yab-mcp","args":[]}}}"#,
        );

        home.write(
            ".codex/config.toml",
            "model = \"gpt-5\"\n\n[projects.\"/x\"]\ntrust_level = \"trusted\"\n\n[mcp_servers.github]\ncommand = \"github-mcp\"\nargs = [\"stdio\"]\n",
        );
        home.write_json(
            ".codex/hooks.json",
            r#"{"hooks":{"SessionStart":[{"matcher":"startup|resume","hooks":[{"type":"command","command":"orca-hook session-start","timeout":10}]}]}}"#,
        );

        home.write_json(
            ".factory/mcp.json",
            r#"{"mcpServers":{"mesh":{"type":"stdio","command":"mesh-mcp","args":["serve"]}}}"#,
        );
        home.write_json(
            ".factory/hooks.json",
            r#"{"hooks":{"SessionStart":[{"matcher":"startup","hooks":[{"type":"command","command":"superset-hook notify","timeout":10}]}]}}"#,
        );

        home.write_json(
            ".config/agent-stack/mcp.json",
            r#"{"mcpServers":{
              "exa":{"type":"stdio","command":"exa-mcp","args":[],"env":{"EXA_API_KEY":"sk-test-FAKE-exa"}},
              "context7":{"type":"http","url":"https://context7.example.com/mcp","headers":{"CONTEXT7_API_KEY":"sk-test-FAKE-c7"}},
              "indexed":{"type":"http","url":"https://indexed.example.com/mcp?api_key=sk-test-FAKE-url&mode=fast"},
              "serena":{"type":"stdio","command":"serena","args":["start-mcp-server"]}
            }}"#,
        );
        home
    }
}

/// Installs a fake `claude` executable in `<home>/bin` (first on the default `PATH`) and
/// returns that directory. Each call appends one line to `<home>/claude-argv.log`: its
/// arguments, each followed by the unit separator (read it with [`Home::claude_calls`]).
/// A call whose `mcp add-json`/`mcp remove` name equals `$FAKE_FAIL_NAME` exits with `$FAKE_EXIT`.
#[cfg(unix)]
pub fn fake_claude(home: &Home) -> PathBuf {
    let dir = home.bin_dir();
    let log = home.path("claude-argv.log");
    let script = format!(
        "#!/bin/sh\n\
         printf '%s\\037' \"$@\" >> '{log}'\n\
         printf '\\n' >> '{log}'\n\
         if [ \"$1\" = mcp ]; then\n\
         \x20 case \"$2\" in\n\
         \x20   add-json|remove)\n\
         \x20     shift 2\n\
         \x20     while [ \"$#\" -gt 0 ]; do\n\
         \x20       case \"$1\" in --scope) shift 2;; -*) shift;; *) break;; esac\n\
         \x20     done\n\
         \x20     if [ -n \"$FAKE_FAIL_NAME\" ] && [ \"$1\" = \"$FAKE_FAIL_NAME\" ]; then exit \"${{FAKE_EXIT:-1}}\"; fi;;\n\
         \x20 esac\n\
         fi\n\
         exit 0\n",
        log = log.display()
    );
    let bin = dir.join("claude");
    write_exec(&bin, &script);
    dir
}
