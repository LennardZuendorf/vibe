#![allow(dead_code)]
//! Shared test scaffolding: scratch roots, the shipped machine/policy, and an
//! in-process `vibe-flow` invocation.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use vibe_flow::paths::{Env, FlowPaths};

static COUNTER: AtomicU64 = AtomicU64::new(0);

/// An environment with nothing set.
pub struct NoEnv;

impl Env for NoEnv {
    fn get(&self, _key: &str) -> Option<String> {
        None
    }
}

/// A fixed environment.
pub struct MapEnv(pub BTreeMap<String, String>);

impl MapEnv {
    /// Builds an environment from pairs.
    pub fn new(pairs: &[(&str, &str)]) -> MapEnv {
        MapEnv(
            pairs
                .iter()
                .map(|(key, value)| ((*key).to_string(), (*value).to_string()))
                .collect(),
        )
    }
}

impl Env for MapEnv {
    fn get(&self, key: &str) -> Option<String> {
        self.0.get(key).cloned().filter(|value| !value.is_empty())
    }
}

/// This crate's manifest directory.
pub fn manifest() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// A fresh, unique scratch directory.
pub fn scratch(name: &str) -> PathBuf {
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("vibe-flow-it-{name}-{}-{n}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("create scratch");
    dir
}

/// A root whose data home carries the shipped state machine.
pub fn root(name: &str) -> PathBuf {
    let root = scratch(name);
    let data = root.join(".agents").join("flow");
    fs::create_dir_all(&data).expect("data home");
    fs::copy(
        manifest().join("../../flow/state-machine.json"),
        data.join("state-machine.json"),
    )
    .expect("copy machine");
    root
}

/// The shipped policy's bytes.
pub fn shipped_policy() -> String {
    fs::read_to_string(manifest().join("../../flow/content/policy.json")).expect("shipped policy")
}

/// Writes a policy into `root`'s data home.
pub fn install_policy(root: &Path, text: &str) {
    let content = root.join(".agents").join("flow").join("content");
    fs::create_dir_all(&content).expect("content dir");
    fs::write(content.join("policy.json"), text).expect("write policy");
}

/// Writes the cursor into `root`'s runtime directory.
pub fn write_cursor(root: &Path, body: &str) {
    let dir = root.join(".vibe").join("run").join("flow");
    fs::create_dir_all(&dir).expect("cursor dir");
    fs::write(dir.join("state.json"), body).expect("write cursor");
}

/// Runs `vibe-flow --root <root> <args>` in process.
pub fn run(root: &Path, args: &[&str]) -> (u8, String, String) {
    run_with(root, args, &NoEnv)
}

/// Runs `vibe-flow` with an explicit environment.
pub fn run_with(root: &Path, args: &[&str], env: &dyn Env) -> (u8, String, String) {
    let mut argv: Vec<std::ffi::OsString> = vec![
        "vibe-flow".into(),
        "--root".into(),
        root.as_os_str().to_os_string(),
    ];
    argv.extend(args.iter().map(std::ffi::OsString::from));
    run_argv(root, &argv, env)
}

/// Runs `vibe-flow` verbatim (no injected flags) with `cwd`.
pub fn run_argv(cwd: &Path, argv: &[std::ffi::OsString], env: &dyn Env) -> (u8, String, String) {
    let (code, stdout, stderr) = vibe_flow::cli::run(cwd, argv, env);
    (
        code,
        String::from_utf8(stdout).expect("stdout utf8"),
        String::from_utf8(stderr).expect("stderr utf8"),
    )
}

/// Resolves paths for a root, with nothing in the environment.
pub fn paths_for(root: &Path) -> FlowPaths {
    FlowPaths::resolve(Some(root), None, root, &NoEnv)
}
