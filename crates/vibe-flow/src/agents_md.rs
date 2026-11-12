//! `vibe-flow render agents-md [--write|--check]`: flow's standalone AGENTS.md
//! block.
//!
//! The block is owned by `flow` under the marker grammar v1
//! (`<!-- vibe:flow:begin v=1 hash=… -->`), so a flow-only install renders the
//! same bytes instruct would compose, and flow never touches `vibe:rules`,
//! `vibe:active-rules` or `vibe:instructions`. The body is *policy-derived*:
//! [`crate::policy::render_invariants`] prints the same rules
//! `vibe-flow policy decide` enforces, which is what makes the shipped prose
//! and the enforcer unable to disagree.
//!
//! `--write` is idempotent (a second run changes nothing) and also writes the
//! ordered fragment instruct recomposes from; `--check` is the CI tooth and
//! exits 1 on a missing block or a hash mismatch.

use std::path::PathBuf;

use vibe_core::markers::{self, Style};

use crate::cmd::Outcome;
use crate::paths::FlowPaths;
use crate::policy::{load_policy, render_invariants};

/// The marker owner: `spec` (10) < `flow` (20) < `instruct` (30).
pub const OWNER: &str = "flow";

/// The fragment path order instruct recomposes from.
pub const FRAGMENT: &str = ".vibe/run/agents-md/20-flow.md";

/// What `render agents-md` should do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Print the block body (the default).
    Print,
    /// Upsert the block into AGENTS.md and write the fragment.
    Write,
    /// Exit 1 when the block is missing or no longer hashes to its marker.
    Check,
}

/// The composed block body, without a trailing newline.
pub fn body(paths: &FlowPaths) -> String {
    let loaded = load_policy(paths.data_dir());
    let invariants = render_invariants(&loaded.policy);
    let mut text = String::from(
        "## Write invariants\n\n\
         Every write this harness restricts is one rule in `content/policy.json`. The\n\
         list below is RENDERED from that file — the same data\n\
         `vibe-flow policy decide` enforces — so the prose and the enforcer cannot\n\
         disagree. Edit the rules, never this text.\n\n",
    );
    text.push_str(&invariants);
    text.push_str(
        "\n\nCheck a path before writing it:\n\n\
         ```bash\n\
         vibe-flow policy decide <path>\n\
         ```",
    );
    text
}

/// Runs the requested mode against `<root>/AGENTS.md`.
pub fn run(paths: &FlowPaths, mode: Mode) -> Outcome {
    let target = paths.root().join("AGENTS.md");
    match mode {
        Mode::Print => Outcome::new(0, format!("{}\n", body(paths)), String::new()),
        Mode::Check => check(&target, &body(paths)),
        Mode::Write => write(paths, &target, &body(paths)),
    }
}

fn check(target: &PathBuf, body: &str) -> Outcome {
    let existing = match std::fs::read_to_string(target) {
        Ok(text) => text,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return Outcome::new(
                1,
                String::new(),
                format!(
                    "render: drift — {} has no vibe:{OWNER} block; run vibe-flow render agents-md --write\n",
                    target.display()
                ),
            );
        }
        Err(err) => {
            return Outcome::new(
                1,
                String::new(),
                format!("render: ERROR — cannot read {}: {err}\n", target.display()),
            );
        }
    };
    match markers::find(&existing, Style::Html, OWNER) {
        Ok(Some(block)) => {
            if markers::intact(&block) && block.body.trim_end() == body.trim_end() {
                Outcome::new(
                    0,
                    format!(
                        "render: ok — the vibe:{OWNER} block in {} matches its recorded hash\n",
                        target.display()
                    ),
                    String::new(),
                )
            } else {
                Outcome::new(
                    1,
                    String::new(),
                    format!(
                        "render: drift — the vibe:{OWNER} block in {} does not match its recorded hash; run vibe-flow render agents-md --write\n",
                        target.display()
                    ),
                )
            }
        }
        Ok(None) => Outcome::new(
            1,
            String::new(),
            format!(
                "render: drift — {} has no vibe:{OWNER} block; run vibe-flow render agents-md --write\n",
                target.display()
            ),
        ),
        Err(err) => Outcome::new(
            1,
            String::new(),
            format!(
                "render: ERROR — the vibe:{OWNER} markers in {} are malformed: {err}\n",
                target.display()
            ),
        ),
    }
}

fn write(paths: &FlowPaths, target: &PathBuf, body: &str) -> Outcome {
    let existing = match std::fs::read_to_string(target) {
        Ok(text) => text,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(err) => {
            return Outcome::new(
                1,
                String::new(),
                format!("render: ERROR — cannot read {}: {err}\n", target.display()),
            );
        }
    };
    let next = match markers::upsert(&existing, Style::Html, OWNER, body) {
        Ok(text) => text,
        Err(err) => {
            return Outcome::new(
                1,
                String::new(),
                format!("render: ERROR — cannot render the vibe:{OWNER} block: {err}\n"),
            );
        }
    };
    if next == existing {
        let fragment = write_fragment(paths, body);
        let mut stdout = format!(
            "render: no change — {} already carries the current vibe:{OWNER} block\n",
            target.display()
        );
        stdout.push_str(&fragment);
        return Outcome::new(0, stdout, String::new());
    }
    if let Err(err) = vibe_core::fs::write_atomic(target, next.as_bytes()) {
        return Outcome::new(
            1,
            String::new(),
            format!("render: ERROR — cannot write {}: {err}\n", target.display()),
        );
    }
    let fragment = write_fragment(paths, body);
    let mut stdout = format!(
        "render: wrote the vibe:{OWNER} block in {}\n",
        target.display()
    );
    stdout.push_str(&fragment);
    Outcome::new(0, stdout, String::new())
}

fn write_fragment(paths: &FlowPaths, body: &str) -> String {
    let path = paths.root().join(FRAGMENT);
    if let Some(dir) = path.parent() {
        if std::fs::create_dir_all(dir).is_err() {
            return String::new();
        }
    }
    match vibe_core::fs::write_atomic(&path, format!("{body}\n").as_bytes()) {
        Ok(()) => String::new(),
        Err(err) => format!("render: WARN — cannot write {}: {err}\n", path.display()),
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::{FRAGMENT, Mode, body, run};
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
            "vibe-flow-agents-{name}-{}-{n}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("create scratch");
        dir
    }

    fn install(name: &str) -> (PathBuf, FlowPaths) {
        let root = scratch(name);
        let content = root.join(".agents").join("flow").join("content");
        fs::create_dir_all(&content).expect("content");
        fs::copy(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../flow/content/policy.json"),
            content.join("policy.json"),
        )
        .expect("policy");
        let paths = FlowPaths::resolve(Some(&root), None, &root, &NoEnv);
        (root, paths)
    }

    #[test]
    fn the_body_renders_the_policy_invariants() {
        let (_, paths) = install("body");
        let text = body(&paths);
        assert!(text.starts_with("## Write invariants\n\n"), "{text}");
        assert!(
            text.contains("- `.planning/*` — `idle`: allow; otherwise: block"),
            "{text}"
        );
        assert!(text.contains("vibe-flow policy decide <path>"), "{text}");
        assert!(!text.ends_with('\n'), "{text}");
    }

    #[test]
    fn write_is_idempotent_and_leaves_other_blocks_alone() {
        let (root, paths) = install("idempotent");
        let original = concat!(
            "# Title\n\n",
            "<!-- vibe:rules -->\nhand-owned rules\n<!-- /vibe:rules -->\n\n",
            "<!-- vibe:instructions:start -->\ninstructions\n<!-- vibe:instructions:end -->\n",
        );
        fs::write(root.join("AGENTS.md"), original).expect("agents");

        let first = run(&paths, Mode::Write);
        assert_eq!(first.code, 0);
        let once = fs::read(root.join("AGENTS.md")).expect("agents");
        assert!(root.join(FRAGMENT).is_file());

        let second = run(&paths, Mode::Write);
        assert_eq!(second.code, 0);
        assert!(second.stdout.contains("no change"), "{}", second.stdout);
        assert_eq!(fs::read(root.join("AGENTS.md")).expect("agents"), once);

        let text = String::from_utf8(once).expect("utf8");
        assert!(text.contains("<!-- vibe:rules -->\nhand-owned rules\n<!-- /vibe:rules -->"));
        assert!(text.contains("<!-- vibe:instructions:start -->\ninstructions\n"));
        assert!(text.contains("<!-- vibe:flow:begin v=1 hash="), "{text}");
        assert_eq!(run(&paths, Mode::Check).code, 0);
    }

    #[test]
    fn a_hand_edit_of_the_block_is_drift() {
        let (root, paths) = install("drift");
        fs::write(root.join("AGENTS.md"), "hello\n").expect("agents");
        assert_eq!(run(&paths, Mode::Write).code, 0);
        let text = fs::read_to_string(root.join("AGENTS.md")).expect("agents");
        let edited = text.replace("## Write invariants", "## Write invariant");
        fs::write(root.join("AGENTS.md"), edited).expect("agents");
        let out = run(&paths, Mode::Check);
        assert_eq!(out.code, 1);
        assert!(out.stderr.contains("render: drift"), "{}", out.stderr);
    }

    #[test]
    fn check_reports_a_missing_block_as_drift() {
        let (root, paths) = install("missing");
        let out = run(&paths, Mode::Check);
        assert_eq!(out.code, 1);
        assert!(
            out.stderr.contains("has no vibe:flow block"),
            "{}",
            out.stderr
        );
        assert!(!root.join("AGENTS.md").exists());
    }

    #[test]
    fn print_mode_emits_the_body() {
        let (_, paths) = install("print");
        let out = run(&paths, Mode::Print);
        assert_eq!(out.code, 0);
        assert!(out.stdout.starts_with("## Write invariants"));
        assert!(out.stdout.ends_with("```\n"));
    }
}
