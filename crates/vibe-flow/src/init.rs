//! `vibe-flow init`: the provider manifest and the payload's first shape.
//!
//! Writes `.vibe/providers/flow.json` (Provider manifest v1) pointing at
//! `["vibe-flow", "emit"]` with the payload file as the fallback, so per-turn
//! drift nudges keep working even when instruct cannot resolve the command.
//! Idempotent: identical content is not rewritten, and a re-run prints the same
//! line.

use std::path::Path;

use crate::cmd::Outcome;
use crate::paths::FlowPaths;
use crate::payload;

/// The manifest body, byte-stable across runs.
pub const MANIFEST: &str = concat!(
    "{\n",
    "  \"id\": \"flow\",\n",
    "  \"command\": [\"vibe-flow\", \"emit\"],\n",
    "  \"file\": \".vibe/run/inject/flow.json\",\n",
    "  \"channel\": \"user-prompt\",\n",
    "  \"trigger\": \"edge\",\n",
    "  \"budget_lines\": 15,\n",
    "  \"timeout_ms\": 500\n",
    "}\n",
);

/// `init`: write the manifest, seed the payload, exit 0.
pub fn run(paths: &FlowPaths) -> Outcome {
    let path = paths.providers_path();
    if let Some(dir) = path.parent() {
        if let Err(err) = std::fs::create_dir_all(dir) {
            return Outcome::new(
                1,
                String::new(),
                format!(
                    "vibe-flow init: ERROR: cannot create {}: {err}\n",
                    dir.display()
                ),
            );
        }
    }
    if let Err(err) = refuse_link(&path) {
        return Outcome::new(1, String::new(), format!("vibe-flow init: ERROR: {err}\n"));
    }
    let existing = std::fs::read_to_string(&path).unwrap_or_default();
    let mut stdout = String::new();
    if existing == MANIFEST {
        stdout.push_str(&format!(
            "init: provider manifest unchanged — {}\n",
            path.display()
        ));
    } else {
        if let Err(err) = vibe_core::fs::write_atomic(&path, MANIFEST.as_bytes()) {
            return Outcome::new(
                1,
                String::new(),
                format!(
                    "vibe-flow init: ERROR: cannot write {}: {err}\n",
                    path.display()
                ),
            );
        }
        stdout.push_str(&format!(
            "init: wrote {} (command [\"vibe-flow\", \"emit\"], payload file as fallback)\n",
            path.display()
        ));
    }
    // Seed the payload so instruct's file fallback has something to read before
    // the first transition. A refused payload write is advisory (FV-R7).
    let payload = payload::refresh(paths);
    let mut stderr = String::new();
    if let Err(err) = payload::prepare(paths).and_then(|()| payload::write(paths, &payload)) {
        stderr.push_str(&format!(
            "vibe-flow init: WARN — the payload was not written: {err}\n"
        ));
    }
    Outcome::new(0, stdout, stderr)
}

fn refuse_link(path: &Path) -> Result<(), String> {
    match std::fs::symlink_metadata(path) {
        Ok(meta) if meta.file_type().is_symlink() => Err(format!(
            "refusing to write the manifest through the link {}",
            path.display()
        )),
        Ok(_) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(format!("cannot inspect {}: {err}", path.display())),
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::{MANIFEST, run};
    use crate::paths::{Env, FlowPaths};
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};

    static COUNTER: AtomicU64 = AtomicU64::new(0);

    struct NoEnv;

    impl Env for NoEnv {
        fn get(&self, _key: &str) -> Option<String> {
            None
        }
    }

    fn scratch(name: &str) -> PathBuf {
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir =
            std::env::temp_dir().join(format!("vibe-flow-init-{name}-{}-{n}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("create scratch");
        dir
    }

    fn install(name: &str) -> (PathBuf, FlowPaths) {
        let root = scratch(name);
        let data = root.join(".agents/skills/vibe");
        fs::create_dir_all(&data).expect("data");
        fs::copy(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../flow/state-machine.json"),
            data.join("state-machine.json"),
        )
        .expect("machine");
        let paths = FlowPaths::resolve(Some(&root), None, &root, &NoEnv);
        (root, paths)
    }

    #[test]
    fn init_writes_the_manifest_and_seeds_the_payload() {
        let (root, paths) = install("writes");
        let out = run(&paths);
        assert_eq!(out.code, 0);
        assert!(out.stderr.is_empty(), "{}", out.stderr);
        assert!(out.stdout.contains("init: wrote "));
        let manifest =
            fs::read_to_string(root.join(".vibe/providers/flow.json")).expect("manifest");
        assert_eq!(manifest, MANIFEST);
        let parsed: serde_json::Value = serde_json::from_str(&manifest).expect("json");
        assert_eq!(parsed["command"], serde_json::json!(["vibe-flow", "emit"]));
        assert_eq!(parsed["file"], ".vibe/run/inject/flow.json");
        assert!(root.join(".vibe/run/inject/flow.json").is_file());
    }

    #[test]
    fn a_second_init_is_byte_identical() {
        let (root, paths) = install("twice");
        assert_eq!(run(&paths).code, 0);
        let first = fs::read(root.join(".vibe/providers/flow.json")).expect("manifest");
        let second_out = run(&paths);
        assert_eq!(second_out.code, 0);
        assert!(second_out.stdout.contains("provider manifest unchanged"));
        assert_eq!(
            fs::read(root.join(".vibe/providers/flow.json")).expect("manifest"),
            first
        );
    }

    #[cfg(unix)]
    #[test]
    fn a_linked_manifest_is_refused() {
        let (root, paths) = install("link");
        let outside = scratch("link-outside");
        let keeper = outside.join("keeper.json");
        fs::write(&keeper, "keep\n").expect("keeper");
        fs::create_dir_all(root.join(".vibe/providers")).expect("dir");
        std::os::unix::fs::symlink(&keeper, root.join(".vibe/providers/flow.json")).expect("link");
        let out = run(&paths);
        assert_eq!(out.code, 1);
        assert!(
            out.stderr
                .contains("refusing to write the manifest through the link")
        );
        assert_eq!(fs::read_to_string(&keeper).expect("keeper"), "keep\n");
    }
}
