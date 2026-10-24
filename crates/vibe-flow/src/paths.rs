//! The one owner of the flow layout (FV-R1, design §Layout v2).
//!
//! Root order: `--root` → `CLAUDE_PROJECT_DIR` → `VIBE_ROOT` → the nearest
//! ancestor of the working directory holding `.vibe/` or `.git` → the working
//! directory. Data home: `--vibe-dir` → `<root>/.agents/flow` when it exists →
//! the legacy `<root>/.agents/skills/vibe` (read fallback during migration).
//!
//! Runtime state lives under `<root>/.vibe/run/`; nothing here reads or writes
//! a per-user directory.

use std::path::{Path, PathBuf};

/// Environment lookups, injected so resolution is testable without touching
/// the process environment.
pub trait Env {
    /// The value of `key`, or `None` when unset or empty.
    fn get(&self, key: &str) -> Option<String>;
}

/// The real process environment.
pub struct Process;

impl Env for Process {
    fn get(&self, key: &str) -> Option<String> {
        std::env::var(key).ok().filter(|value| !value.is_empty())
    }
}

/// Every path the flow half reads or writes, resolved once.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlowPaths {
    root: PathBuf,
    data: PathBuf,
    legacy_data: PathBuf,
}

impl FlowPaths {
    /// Resolves the layout. `cwd` must be absolute; the marker search walks up
    /// from it.
    pub fn resolve(
        root_override: Option<&Path>,
        vibe_dir_override: Option<&Path>,
        cwd: &Path,
        env: &dyn Env,
    ) -> FlowPaths {
        let root = match root_override {
            Some(path) => absolute(cwd, path),
            None => resolve_root(cwd, env),
        };
        let legacy_data = root.join(".agents").join("skills").join("vibe");
        let data = match vibe_dir_override {
            Some(path) => absolute(cwd, path),
            None => {
                let current = root.join(".agents").join("flow");
                if current.is_dir() {
                    current
                } else {
                    legacy_data.clone()
                }
            }
        };
        FlowPaths {
            root,
            data,
            legacy_data,
        }
    }

    /// The project root.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// The shipped flow data home.
    pub fn data_dir(&self) -> &Path {
        &self.data
    }

    /// The legacy skill directory, read-only during migration.
    pub fn legacy_data_dir(&self) -> &Path {
        &self.legacy_data
    }

    /// `<data>/state-machine.json`.
    pub fn machine_path(&self) -> PathBuf {
        self.data.join("state-machine.json")
    }

    /// `<data>/content/policy.json`.
    pub fn policy_path(&self) -> PathBuf {
        self.data.join("content").join("policy.json")
    }

    /// `<data>/SKILL.md`.
    pub fn skill_path(&self) -> PathBuf {
        self.data.join("SKILL.md")
    }

    /// The cursor write path, always the new one.
    pub fn cursor_path(&self) -> PathBuf {
        self.run_dir().join("state.json")
    }

    /// The legacy cursor, a read fallback only.
    pub fn legacy_cursor_path(&self) -> PathBuf {
        self.legacy_data.join("state.json")
    }

    /// `.vibe/run/flow/ledger.jsonl`.
    pub fn ledger_path(&self) -> PathBuf {
        self.run_dir().join("ledger.jsonl")
    }

    /// `.vibe/run/inject/flow.json`.
    pub fn inject_path(&self) -> PathBuf {
        self.root
            .join(".vibe")
            .join("run")
            .join("inject")
            .join("flow.json")
    }

    /// `.vibe/providers/flow.json`.
    pub fn providers_path(&self) -> PathBuf {
        self.root.join(".vibe").join("providers").join("flow.json")
    }

    /// The evidence receipts directory.
    pub fn evidence_dir(&self) -> PathBuf {
        self.run_dir().join("evidence")
    }

    fn run_dir(&self) -> PathBuf {
        self.root.join(".vibe").join("run").join("flow")
    }
}

/// `--root`/env values resolve against the working directory when relative.
fn absolute(cwd: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        cwd.join(path)
    }
}

fn resolve_root(cwd: &Path, env: &dyn Env) -> PathBuf {
    for key in ["CLAUDE_PROJECT_DIR", "VIBE_ROOT"] {
        if let Some(value) = env.get(key) {
            return absolute(cwd, Path::new(&value));
        }
    }
    let mut dir: &Path = cwd;
    loop {
        if dir.join(".vibe").exists() || dir.join(".git").exists() {
            return dir.to_path_buf();
        }
        match dir.parent() {
            Some(parent) => dir = parent,
            None => break,
        }
    }
    cwd.to_path_buf()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::{Env, FlowPaths};
    use std::collections::BTreeMap;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};

    static COUNTER: AtomicU64 = AtomicU64::new(0);

    fn scratch(name: &str) -> PathBuf {
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir =
            std::env::temp_dir().join(format!("vibe-flow-paths-{name}-{}-{n}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("create scratch");
        dir
    }

    struct Map(BTreeMap<String, String>);

    impl Map {
        fn new(pairs: &[(&str, &str)]) -> Map {
            Map(pairs
                .iter()
                .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
                .collect())
        }
    }

    impl Env for Map {
        fn get(&self, key: &str) -> Option<String> {
            self.0.get(key).cloned().filter(|v| !v.is_empty())
        }
    }

    #[test]
    fn root_override_wins_over_everything() {
        let cwd = scratch("root-override");
        let env = Map::new(&[("CLAUDE_PROJECT_DIR", "/env"), ("VIBE_ROOT", "/vibe")]);
        let paths = FlowPaths::resolve(Some(Path::new("/explicit")), None, &cwd, &env);
        assert_eq!(paths.root(), Path::new("/explicit"));
    }

    #[test]
    fn claude_project_dir_beats_vibe_root() {
        let cwd = scratch("env-order");
        let env = Map::new(&[("CLAUDE_PROJECT_DIR", "/env"), ("VIBE_ROOT", "/vibe")]);
        assert_eq!(
            FlowPaths::resolve(None, None, &cwd, &env).root(),
            Path::new("/env")
        );
    }

    #[test]
    fn vibe_root_is_used_when_claude_project_dir_is_absent() {
        let cwd = scratch("vibe-root");
        let env = Map::new(&[("VIBE_ROOT", "/vibe")]);
        assert_eq!(
            FlowPaths::resolve(None, None, &cwd, &env).root(),
            Path::new("/vibe")
        );
    }

    #[test]
    fn empty_env_values_are_ignored() {
        let cwd = scratch("empty-env");
        fs::create_dir_all(cwd.join(".git")).expect("git marker");
        let env = Map::new(&[("CLAUDE_PROJECT_DIR", ""), ("VIBE_ROOT", "")]);
        assert_eq!(FlowPaths::resolve(None, None, &cwd, &env).root(), cwd);
    }

    #[test]
    fn marker_search_finds_the_nearest_ancestor_with_git() {
        let top = scratch("markers");
        fs::create_dir_all(top.join(".git")).expect("git marker");
        let nested = top.join("a").join("b");
        fs::create_dir_all(&nested).expect("nested");
        let env = Map::new(&[]);
        assert_eq!(FlowPaths::resolve(None, None, &nested, &env).root(), top);
    }

    #[test]
    fn marker_search_prefers_a_dot_vibe_marker() {
        let top = scratch("dot-vibe");
        fs::create_dir_all(top.join(".git")).expect("git marker");
        let nested = top.join("a");
        fs::create_dir_all(nested.join(".vibe")).expect("vibe marker");
        let env = Map::new(&[]);
        assert_eq!(FlowPaths::resolve(None, None, &nested, &env).root(), nested);
    }

    #[test]
    fn with_no_marker_the_working_directory_is_the_root() {
        let cwd = scratch("no-marker");
        let env = Map::new(&[]);
        assert_eq!(FlowPaths::resolve(None, None, &cwd, &env).root(), cwd);
    }

    #[test]
    fn data_home_prefers_agents_flow_then_falls_back_to_the_legacy_skill() {
        let root = scratch("data-home");
        let env = Map::new(&[]);
        let legacy = root.join(".agents").join("skills").join("vibe");
        let paths = FlowPaths::resolve(Some(&root), None, &root, &env);
        assert_eq!(paths.data_dir(), legacy);
        assert_eq!(paths.legacy_data_dir(), legacy);

        fs::create_dir_all(root.join(".agents").join("flow")).expect("flow data home");
        let paths = FlowPaths::resolve(Some(&root), None, &root, &env);
        assert_eq!(paths.data_dir(), root.join(".agents").join("flow"));
        assert_eq!(paths.legacy_data_dir(), legacy);
    }

    #[test]
    fn vibe_dir_override_wins_over_both_data_homes() {
        let root = scratch("vibe-dir");
        fs::create_dir_all(root.join(".agents").join("flow")).expect("flow data home");
        let env = Map::new(&[]);
        let paths = FlowPaths::resolve(Some(&root), Some(Path::new("/pinned")), &root, &env);
        assert_eq!(paths.data_dir(), Path::new("/pinned"));
    }

    #[test]
    fn every_accessor_lands_under_the_root() {
        let root = scratch("accessors");
        let env = Map::new(&[]);
        let paths = FlowPaths::resolve(Some(&root), None, &root, &env);
        let run = root.join(".vibe").join("run").join("flow");
        assert_eq!(paths.cursor_path(), run.join("state.json"));
        assert_eq!(paths.ledger_path(), run.join("ledger.jsonl"));
        assert_eq!(paths.evidence_dir(), run.join("evidence"));
        assert_eq!(
            paths.inject_path(),
            root.join(".vibe")
                .join("run")
                .join("inject")
                .join("flow.json")
        );
        assert_eq!(
            paths.providers_path(),
            root.join(".vibe").join("providers").join("flow.json")
        );
        assert_eq!(
            paths.legacy_cursor_path(),
            root.join(".agents")
                .join("skills")
                .join("vibe")
                .join("state.json")
        );
    }
}
