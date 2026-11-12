//! `vibe-flow emit`: the payload as a provider command (`D22`).
//!
//! instruct's provider manifest names `["vibe-flow", "emit"]` as the command
//! source, with the payload file as the fallback. Emit therefore prints the
//! payload for the *live* cursor — a stale file still names the current state —
//! but never bumps `seq`: the edge is injected when a transition advanced the
//! cursor, and a per-turn emit that bumped it would inject orders every turn.

use crate::cmd::Outcome;
use crate::paths::FlowPaths;
use crate::payload;

/// `emit`: refresh the payload content, print it, always exit 0.
pub fn run(paths: &FlowPaths) -> Outcome {
    let payload = payload::refresh(paths);
    let mut stderr = String::new();
    if let Err(err) = payload::prepare(paths) {
        stderr.push_str(&format!(
            "vibe-flow emit: WARN — the payload cannot be written, printing it only: {err}\n"
        ));
    } else if let Err(err) = payload::write(paths, &payload) {
        stderr.push_str(&format!(
            "vibe-flow emit: WARN — the payload was not written: {err}\n"
        ));
    }
    Outcome::new(0, payload::encode(&payload), stderr)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::run;
    use crate::paths::{Env, FlowPaths};
    use serde_json::Value;
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
            std::env::temp_dir().join(format!("vibe-flow-emit-{name}-{}-{n}", std::process::id()));
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
        fs::create_dir_all(root.join(".vibe/run/flow")).expect("run");
        fs::write(
            root.join(".vibe/run/flow/state.json"),
            "{\"flow\":\"feature\",\"phase\":\"impl\",\"feature\":\"auth\",\"updated\":\"2026-01-01T00:00:00Z\"}\n",
        )
        .expect("cursor");
        let paths = FlowPaths::resolve(Some(&root), None, &root, &NoEnv);
        (root, paths)
    }

    #[test]
    fn emit_prints_the_payload_for_the_live_cursor_and_writes_it() {
        let (root, paths) = install("live");
        let out = run(&paths);
        assert_eq!(out.code, 0);
        assert!(out.stderr.is_empty(), "{}", out.stderr);
        let parsed: Value = serde_json::from_str(&out.stdout).expect("json");
        assert_eq!(parsed["seq"], 0, "emit must not bump seq");
        assert!(parsed["level"].as_str().unwrap_or("").contains("feature.impl"));
        assert!(root.join(".vibe/run/inject/flow.json").is_file());

        // A second emit reprints the same bytes and bumps nothing.
        let again = run(&paths);
        assert_eq!(again.stdout, out.stdout);
        let parsed: Value = serde_json::from_str(&again.stdout).expect("json");
        assert_eq!(parsed["seq"], 0);
    }

    #[test]
    fn emit_preserves_a_previous_seq_and_its_events() {
        let (root, paths) = install("preserve");
        fs::create_dir_all(root.join(".vibe/run/inject")).expect("inject");
        fs::write(
            root.join(".vibe/run/inject/flow.json"),
            "{\"v\":1,\"provider\":\"flow\",\"seq\":4,\"level\":\"old\",\"edge\":\"old\",\"session\":[],\"events\":[{\"id\":\"e4-1\",\"ts\":\"t\",\"text\":\"x\"}]}\n",
        )
        .expect("payload");
        let out = run(&paths);
        let parsed: Value = serde_json::from_str(&out.stdout).expect("json");
        assert_eq!(parsed["seq"], 4);
        assert_eq!(parsed["events"].as_array().map(Vec::len), Some(1));
        assert!(parsed["level"].as_str().unwrap_or("").contains("feature.impl"));
    }

    #[cfg(unix)]
    #[test]
    fn emit_still_prints_when_the_payload_is_a_link() {
        let (root, paths) = install("link");
        let outside = scratch("link-outside");
        let keeper = outside.join("keeper.json");
        fs::write(&keeper, "keep\n").expect("keeper");
        fs::create_dir_all(root.join(".vibe/run/inject")).expect("inject");
        std::os::unix::fs::symlink(&keeper, root.join(".vibe/run/inject/flow.json")).expect("link");
        let out = run(&paths);
        assert_eq!(out.code, 0);
        assert!(out.stderr.contains("refusing to append through the link"), "{}", out.stderr);
        assert_eq!(fs::read_to_string(&keeper).expect("keeper"), "keep\n");
        let parsed: Value = serde_json::from_str(&out.stdout).expect("json");
        assert_eq!(parsed["provider"], "flow");
    }
}
