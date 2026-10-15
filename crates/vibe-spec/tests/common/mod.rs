//! Shared helpers for the vibe-spec integration tests.
#![allow(dead_code, clippy::unwrap_used, clippy::expect_used)]
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

/// The repository root.
pub fn repo() -> PathBuf {
    fs::canonicalize(Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")).unwrap()
}

/// A fresh, canonical scratch project holding an empty `.git/` (bounds discovery).
pub fn project(name: &str) -> PathBuf {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let dir =
        std::env::temp_dir().join(format!("vibe-spec-test-{name}-{}-{n}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join(".git")).unwrap();
    fs::canonicalize(dir).unwrap()
}

/// Writes `text` to `root/rel`, creating parents.
pub fn write(root: &Path, rel: &str, text: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

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

fn run(mut cmd: Command, cwd: &Path, env: &[(&str, &str)]) -> Out {
    cmd.current_dir(cwd)
        .env_clear()
        .env("PATH", std::env::var_os("PATH").unwrap_or_default())
        .env("HOME", cwd)
        .env("LC_ALL", "C")
        .env("TZ", "UTC");
    for (k, v) in env {
        cmd.env(k, v);
    }
    let o = cmd.output().unwrap();
    Out {
        code: o.status.code().unwrap_or(-1),
        stdout: o.stdout,
        stderr: String::from_utf8_lossy(&o.stderr).into_owned(),
    }
}

/// Runs the candidate in `cwd` with a clean environment plus `env`.
pub fn spec(cwd: &Path, args: &[&str], env: &[(&str, &str)]) -> Out {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_vibe-spec"));
    cmd.args(args);
    run(cmd, cwd, env)
}

/// Runs the frozen oracle `oracles/bash/spec/scripts/<script>` the same way.
pub fn oracle(cwd: &Path, script: &str, args: &[&str]) -> Out {
    let mut cmd = Command::new("bash");
    cmd.arg(repo().join("oracles/bash/spec/scripts").join(script))
        .args(args);
    run(cmd, cwd, &[])
}

/// Like [`oracle`], with extra environment variables.
pub fn oracle_env(cwd: &Path, script: &str, args: &[&str], env: &[(&str, &str)]) -> Out {
    let mut cmd = Command::new("bash");
    cmd.arg(repo().join("oracles/bash/spec/scripts").join(script))
        .args(args);
    run(cmd, cwd, env)
}

/// Asserts the candidate's `command args` and the oracle's `script args`
/// agree on exit code and stdout bytes when run in `cwd`.
pub fn assert_parity(cwd: &Path, command: &str, script: &str, args: &[&str]) {
    let want = oracle(cwd, script, args);
    let mut full = vec![command];
    full.extend_from_slice(args);
    let got = spec(cwd, &full, &[]);
    assert_eq!(
        (got.code, got.text()),
        (want.code, want.text()),
        "`vibe-spec {command} {args:?}` vs `{script}` in {}\ncandidate stderr: {}",
        cwd.display(),
        got.stderr
    );
}

/// Validates `json` against `$defs/<def>` of `contracts/spec-json.schema.json`
/// with the contracts' own validator (`contracts/lib/schema.mjs`).
#[cfg(unix)]
pub fn assert_spec_json(def: &str, json: &str) {
    let lib = repo().join("contracts/lib/schema.mjs");
    let schema = repo().join("contracts/spec-json.schema.json");
    let script = format!(
        "import {{ validate }} from 'file://{}';\n\
         import {{ readFileSync }} from 'node:fs';\n\
         const root = JSON.parse(readFileSync({:?}, 'utf8'));\n\
         const errs = validate(root, {{ $ref: '#/$defs/{def}' }}, JSON.parse(process.argv[1]));\n\
         if (errs.length) {{ console.error(errs.join('\\n')); process.exit(1); }}",
        lib.display(),
        schema.display().to_string()
    );
    let out = Command::new("node")
        .args(["--input-type=module", "-e", &script, json])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{def} does not validate: {}\n{json}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// `vibe-spec validate --format json <extra>` in `cwd`, parsed.
pub fn validate_json(cwd: &Path, extra: &[&str], env: &[(&str, &str)]) -> (i32, serde_json::Value) {
    let mut args = vec!["validate", "--format", "json"];
    args.extend_from_slice(extra);
    let out = spec(cwd, &args, env);
    let v = serde_json::from_slice(&out.stdout)
        .unwrap_or_else(|e| panic!("not JSON ({e}): {}\nstderr: {}", out.text(), out.stderr));
    (out.code, v)
}
