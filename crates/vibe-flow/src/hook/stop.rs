//! The stop gate, ported from `oracles/js/commands/hook.mjs` (lines
//! 602-889): the re-entry guard, the TDD nudge (warn-only) and the evidence
//! receipt tooth (the one block). The oracle's known-stricter choices —
//! `stop_hook_active` matching only `true`/`"true"`, the `-uall` porcelain,
//! and the three-way tree answer — are kept. Two v0.4 divergences are
//! pinned in the parity suite: receipts resolve at the runtime path first
//! (FV-R6), and the staleness scan excludes the candidate's own
//! `.vibe/run/**` writes.

use std::path::PathBuf;
use std::time::SystemTime;

use serde_json::Value;

use super::{Outcome, append_warn_log, porcelain_path};
use crate::cursor;
use crate::paths::FlowPaths;

/// The gate's escape hatch, byte-identical to the oracle.
const ABORT_HINT: &str = "  not verifying? abort with: vibe-flow state set idle";

/// The states whose working tree the predicates consult.
const TREE_STATES: [&str; 4] = [
    "feature.impl",
    "quick.fix",
    "feature.verify",
    "quick.verify",
];

/// Runs the stop gate over the stdin payload.
pub fn run(paths: &FlowPaths, stdin: &str) -> Outcome {
    if stop_hook_active(stdin) {
        return Outcome::new(0, String::new(), String::new());
    }
    if !paths.detect_script().exists() {
        return Outcome::new(0, String::new(), String::new());
    }

    let (state, feature) = match cursor::read_cursor(paths) {
        Ok(cur) => (
            cursor::state_key(&cur.flow, &cur.phase),
            string_feature(&cur.feature),
        ),
        Err(_) => ("idle".to_string(), None),
    };

    let tree = if TREE_STATES.contains(&state.as_str()) {
        git_porcelain(paths.root())
    } else {
        Tree::default_undetermined()
    };

    let mut stderr = String::new();
    let mut relay = |msg: &str| {
        stderr.push_str(&format!("vibe-gate: {msg}\n"));
        append_warn_log(paths, &format!("gate: {msg}"));
    };

    if let Some(msg) = tdd_predicate(&state, &tree.text) {
        relay(&msg);
    }

    if let Some(block) = receipt_check(paths, &state, feature.as_deref(), &tree, &mut relay) {
        return Outcome::new(2, String::new(), stderr + &block);
    }

    Outcome::new(0, String::new(), stderr)
}

/// `stop_hook_active` is truthy only for the boolean `true` or the string
/// `"true"` — the oracle's jq leg, chosen fail-safe so the gate can never
/// block its own re-invocation into a loop.
fn stop_hook_active(stdin: &str) -> bool {
    let Ok(parsed) = serde_json::from_str::<Value>(stdin) else {
        return false;
    };
    match parsed.get("stop_hook_active") {
        Some(Value::Bool(true)) => true,
        Some(Value::String(text)) if text == "true" => true,
        _ => false,
    }
}

/// The feature as a string, when the cursor carries one.
fn string_feature(feature: &Value) -> Option<String> {
    feature.as_str().map(str::to_string)
}

/// The working tree: determined (git answered), not-a-repo, or
/// undetermined-with-a-reason. "Could not look" is never "nothing changed".
struct Tree {
    determined: bool,
    no_git: bool,
    text: String,
    why: String,
}

impl Tree {
    fn default_undetermined() -> Tree {
        Tree {
            determined: false,
            no_git: false,
            text: String::new(),
            why: String::new(),
        }
    }
}

fn git_porcelain(root: &std::path::Path) -> Tree {
    use std::process::{Command, Stdio};
    let run = |args: &[&str]| -> Option<(bool, String)> {
        let out = Command::new("git")
            .args(args)
            .current_dir(root)
            .stdin(Stdio::null())
            .output()
            .ok()?;
        Some((
            out.status.success(),
            String::from_utf8_lossy(&out.stdout).into_owned(),
        ))
    };
    let Some((true, _)) = run(&["rev-parse", "--is-inside-work-tree"]) else {
        return Tree {
            determined: false,
            no_git: true,
            text: String::new(),
            why: String::new(),
        };
    };
    match run(&["status", "--porcelain", "-uall"]) {
        Some((true, text)) => Tree {
            determined: true,
            no_git: false,
            why: String::new(),
            text: text.trim_end_matches('\n').to_string(),
        },
        Some((false, _)) => Tree {
            determined: false,
            no_git: false,
            text: String::new(),
            why: "git status failed".to_string(),
        },
        None => Tree {
            determined: false,
            no_git: false,
            why: "spawn threw".to_string(),
            ..Tree::default_undetermined()
        },
    }
}

/// Predicate 1 — impl touched `src/` with no test changes (warn-only).
/// Reproduces the oracle's row quirk faithfully: a top-level ` M src/a.rs`
/// row does NOT match, because the porcelain prefix precedes `src/`.
fn tdd_predicate(state: &str, porcelain: &str) -> Option<String> {
    if state != "feature.impl" && state != "quick.fix" {
        return None;
    }
    if porcelain.is_empty() {
        return None;
    }
    let matches = |needle: &str, tail: &str| {
        porcelain.lines().any(|row| {
            let path = porcelain_path(row);
            path == needle || path.ends_with(tail)
        })
    };
    let touches_src = matches("src", "/src/");
    let touches_tests = matches("tests", "/tests/") || matches("test", "/test/");
    if touches_src && !touches_tests {
        return Some(format!(
            "in {state}, src changed with no test changes — TDD expects a reproducing/covering test. (warn-only)"
        ));
    }
    None
}

/// Predicate 2 — the evidence receipt tooth. Returns the block lines on a
/// refusal; `None` passes the gate.
fn receipt_check(
    paths: &FlowPaths,
    state: &str,
    feature: Option<&str>,
    tree: &Tree,
    warn: &mut dyn FnMut(&str),
) -> Option<String> {
    if state != "feature.verify" && state != "quick.verify" {
        return None;
    }
    let receipt: Option<PathBuf> = if state == "feature.verify" {
        let Some(feature) = feature else {
            warn(
                "in feature.verify but the cursor names no feature — cannot resolve the evidence receipt; skipping the gate. (warn-only)",
            );
            return None;
        };
        let name = format!("feature-{feature}.md");
        Some(paths.evidence_dir().join(name))
    } else {
        Some(paths.evidence_dir().join("quick.md"))
    };
    // The v0.4 runtime path answers first; the oracle's legacy path stays a
    // fallback so receipts written before the migration keep gating.
    let receipt = receipt?;
    let name = receipt.file_name().unwrap_or_default().to_os_string();
    let legacy = paths.legacy_evidence_dir().join(name);
    let (receipt_path, expected) = if receipt.is_file() {
        (receipt.clone(), receipt)
    } else if legacy.is_file() {
        (legacy, receipt)
    } else {
        return Some(block_missing(state, &receipt));
    };

    let receipt_mtime = std::fs::metadata(&receipt_path)
        .and_then(|m| m.modified())
        .unwrap_or(SystemTime::UNIX_EPOCH);

    if !tree.determined {
        if tree.no_git {
            return None;
        }
        return Some(block_undetermined(&expected, &tree.why));
    }

    for row in tree.text.lines() {
        if row.is_empty() {
            continue;
        }
        let xy = row.get(..2).unwrap_or("");
        if xy.contains('D') {
            continue;
        }
        let path = porcelain_path(row);
        if is_own_runtime(paths, path) {
            continue;
        }
        let file = paths.root().join(path);
        let Ok(meta) = std::fs::metadata(&file) else {
            continue;
        };
        let Ok(modified) = meta.modified() else {
            continue;
        };
        if modified > receipt_mtime {
            return Some(block_stale(&expected, path));
        }
    }
    None
}

/// The harness's own runtime writes, excluded from the staleness scan.
/// The oracle excludes three paths; the candidate extends the set with the
/// whole `.vibe/run/` tree, because the v0.4 runtime (cursor, ledger,
/// payload, receipts) lives there and a write to it must never stale a
/// receipt the human did not touch.
fn is_own_runtime(paths: &FlowPaths, path: &str) -> bool {
    let evidence = paths.posix_relative(&paths.legacy_evidence_dir());
    if path == evidence || path.starts_with(&format!("{evidence}/")) {
        return true;
    }
    if path == paths.posix_relative(&paths.warnings_log()) {
        return true;
    }
    if path == ".vibe/last-inject" {
        return true;
    }
    path == ".vibe/run" || path.starts_with(".vibe/run/")
}

fn block_missing(state: &str, receipt: &std::path::Path) -> String {
    format!(
        "vibe-gate: BLOCKED — {state} needs an evidence receipt before 'done'.\n  expected: {}\n  it must record the commands you ran and their observed output (per unit ID for a feature).\n{}\n",
        receipt.display(),
        ABORT_HINT
    )
}

fn block_undetermined(receipt: &std::path::Path, why: &str) -> String {
    format!(
        "vibe-gate: BLOCKED — the working tree state could not be determined, so the receipt cannot be trusted.\n  receipt: {}\n  git status --porcelain -uall did not answer ({why}).\n  a huge UN-IGNORED untracked tree is the usual cause — gitignore it (or commit it) and re-run the Stop.\n  \"I could not look\" is not \"nothing changed\", so this blocks rather than passing you through.\n{}\n",
        receipt.display(),
        ABORT_HINT
    )
}

fn block_stale(receipt: &std::path::Path, path: &str) -> String {
    format!(
        "vibe-gate: BLOCKED — the evidence receipt is stale.\n  receipt: {}\n  changed after it was written: {path}\n  re-run verification and rewrite the receipt with fresh commands + output.\n{}\n",
        receipt.display(),
        ABORT_HINT
    )
}

#[cfg(test)]
mod tests {
    use super::{ABORT_HINT, is_own_runtime, stop_hook_active, tdd_predicate};
    use crate::paths::{Env, FlowPaths};
    use std::path::Path;

    struct NoEnv;

    impl Env for NoEnv {
        fn get(&self, _key: &str) -> Option<String> {
            None
        }
    }

    fn fixture_paths() -> FlowPaths {
        let root = Path::new("/vibe-flow-stop-test");
        FlowPaths::resolve(Some(root), None, Path::new("/"), &NoEnv)
    }

    #[test]
    fn abort_hint_names_the_state_writer() {
        assert_eq!(
            ABORT_HINT,
            "  not verifying? abort with: vibe-flow state set idle"
        );
    }

    #[test]
    fn stop_hook_active_only_accepts_true() {
        assert!(stop_hook_active(r#"{"stop_hook_active":true}"#));
        assert!(stop_hook_active(r#"{"stop_hook_active":"true"}"#));
        assert!(!stop_hook_active(r#"{"stop_hook_active":false}"#));
        assert!(!stop_hook_active(r#"{"stop_hook_active":"false"}"#));
        assert!(!stop_hook_active("{}"));
        assert!(!stop_hook_active("not json"));
    }

    #[test]
    fn tdd_predicate_warns_on_src_without_tests() {
        assert!(tdd_predicate("feature.impl", "?? src").is_some());
        assert!(tdd_predicate("quick.fix", "?? pkg/src/").is_some());
        assert!(tdd_predicate("feature.impl", "?? src\n?? tests").is_none());
        assert!(tdd_predicate("feature.impl", "?? pkg/src/\n?? pkg/tests/").is_none());
        assert!(tdd_predicate("idle", "?? src").is_none());
    }

    #[test]
    fn own_runtime_covers_the_vibe_run_tree_and_the_legacy_three() {
        let paths = fixture_paths();
        for path in [
            ".vibe/run",
            ".vibe/run/flow/state.json",
            ".vibe/run/flow/evidence/quick.md",
            ".vibe/last-inject",
            ".agents/skills/vibe/warnings.log",
            ".agents/skills/vibe/evidence/quick.md",
        ] {
            assert!(is_own_runtime(&paths, path), "{path}");
        }
        assert!(!is_own_runtime(&paths, "docs/spec/tech.md"));
        assert!(!is_own_runtime(&paths, ".vibe/blocks/style.md"));
    }
}
