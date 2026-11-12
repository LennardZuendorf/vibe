//! `vibe-flow doctrine` (mirrors `commands/doctrine.mjs`, which mirrors
//! `doctrine.sh` — the oracle).
//!
//! Emits the vibe skill's `<!-- vibe:doctrine -->` block, markers excluded, and
//! nothing else. Read-only and always exit 0: a missing block, skill or
//! `SKILL.md` is silence.
//!
//! NO LIVE STATE RIDES THIS OUTPUT. Claude Code replays a SessionStart hook's
//! saved output on `--resume`, so anything cursor-shaped here goes stale; the
//! per-turn level channel names the state instead. The cursor summary this
//! command once appended was deleted in the oracle and the engine together.

use crate::blocks::{extract, strip_trailing_newlines};
use crate::cmd::Outcome;
use crate::paths::FlowPaths;

/// The doctrine block for the installed skill, without its trailing newline.
pub fn doctrine_text(paths: &FlowPaths) -> String {
    let Ok(text) = std::fs::read_to_string(paths.skill_path()) else {
        return String::new();
    };
    match extract(&text, "vibe:doctrine") {
        Some(block) => strip_trailing_newlines(&block).to_string(),
        None => String::new(),
    }
}

/// `doctrine`: the block plus exactly one trailing newline, or nothing.
pub fn run(paths: &FlowPaths) -> Outcome {
    let doctrine = doctrine_text(paths);
    if doctrine.is_empty() {
        return Outcome::new(0, String::new(), String::new());
    }
    Outcome::new(0, format!("{doctrine}\n"), String::new())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::run;
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
        let dir = std::env::temp_dir().join(format!(
            "vibe-flow-doctrine-{name}-{}-{n}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("create scratch");
        dir
    }

    fn with_skill(name: &str, body: &str) -> (PathBuf, FlowPaths) {
        let root = scratch(name);
        let skill = root.join(".agents/skills/vibe");
        fs::create_dir_all(&skill).expect("skill dir");
        fs::write(skill.join("SKILL.md"), body).expect("skill");
        let paths = FlowPaths::resolve(Some(&root), None, &root, &NoEnv);
        (root, paths)
    }

    #[test]
    fn prints_the_block_and_nothing_else() {
        let (_, paths) = with_skill(
            "ok",
            "head\n<!-- vibe:doctrine -->\nline 1\nline 2\n<!-- /vibe:doctrine -->\ntail\n",
        );
        let out = run(&paths);
        assert_eq!(out.code, 0);
        assert_eq!(out.stdout, "line 1\nline 2\n");
        assert!(out.stderr.is_empty());
    }

    #[test]
    fn a_missing_block_is_silent_success() {
        let (_, paths) = with_skill("absent", "no block here\n");
        let out = run(&paths);
        assert_eq!(out.code, 0);
        assert!(out.stdout.is_empty());
        assert!(out.stderr.is_empty());
    }

    #[test]
    fn an_empty_block_is_silent_success() {
        let (_, paths) = with_skill("empty", "<!-- vibe:doctrine -->\n<!-- /vibe:doctrine -->\n");
        let out = run(&paths);
        assert_eq!(out.code, 0);
        assert!(out.stdout.is_empty());
    }

    #[test]
    fn an_unclosed_block_is_silent() {
        let (_, paths) = with_skill("unclosed", "<!-- vibe:doctrine -->\nleak?\n");
        let out = run(&paths);
        assert!(out.stdout.is_empty());
    }

    #[test]
    fn a_missing_skill_file_is_silent() {
        let root = scratch("missing");
        let paths = FlowPaths::resolve(Some(&root), None, &root, &NoEnv);
        let out = run(&paths);
        assert_eq!(out.code, 0);
        assert!(out.stdout.is_empty());
        assert!(Path::new(&root).exists());
    }
}
