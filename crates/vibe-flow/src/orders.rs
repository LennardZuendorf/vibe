//! `vibe-flow orders` (mirrors `commands/orders.mjs`, which mirrors
//! `orders.sh` — the oracle).
//!
//! D12 resolution chain: cursor → the state's `skill` link in the machine →
//! the `<!-- vibe:orders:<state> -->` block in that skill's `SKILL.md` → the
//! machine's inline `inject` string (idle's only carrier) → a hardcoded generic
//! one-liner. `<feature>` is the only interpolation. Always exits 0: orders are
//! read-only and a missing machine, skill or block is a degrade, never a
//! session-ending failure.
//!
//! [`enrichment`] is the spec half of the same answer: a soft `vibe-spec` probe
//! that appends the spec root and its state-tagged lessons to the *payload's*
//! edge, or a bare path pointer when the tool or the tree is absent. It is not
//! on the `orders` stdout: that output is byte-parity with the frozen oracle,
//! which knows nothing about a spec peer.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use serde_json::Value;

use crate::blocks::{extract, strip_trailing_newlines};
use crate::cmd::Outcome;
use crate::cursor::read_cursor;
use crate::machine::Machine;
use crate::paths::FlowPaths;

/// The last-resort line: still valid guidance, still byte-stable.
pub const GENERIC_FALLBACK: &str = "state=unknown · read .agents/skills/vibe/state-machine.json and pick the matching vibe phase · transition via set-state.sh";

/// The degrade line when no spec peer answers: a path pointer and nothing else.
pub const PATH_POINTER: &str = "spec: docs/spec/ (or .spec/) — read lessons.md and plan.md there; vibe-spec is unavailable, so no tagged lessons.";

fn line(text: &str) -> String {
    format!("{text}\n")
}

/// `orders [<state>]`: the block for the cursor's state (or the argument).
pub fn run(paths: &FlowPaths, arg: Option<&str>) -> Outcome {
    let (cursor_state, feature) = current_state_and_feature(paths);
    let state = arg.filter(|text| !text.is_empty()).unwrap_or(&cursor_state);
    Outcome::new(0, line(&orders_text(paths, state, &feature)), String::new())
}

/// The resolved orders text for `state`, without a trailing newline.
pub fn orders_text(paths: &FlowPaths, state: &str, feature: &Value) -> String {
    let machine = Machine::load(paths.data_dir()).ok();
    if let Some(machine) = &machine {
        let skill = machine
            .skill(state)
            .and_then(Value::as_str)
            .filter(|text| !text.is_empty());
        if let Some(skill) = skill {
            let skill_file = skills_dir(paths).join(skill).join("SKILL.md");
            if let Ok(text) = std::fs::read_to_string(&skill_file) {
                if let Some(block) = extract(&text, &format!("vibe:orders:{state}")) {
                    let block = strip_trailing_newlines(&block);
                    if !block.is_empty() {
                        return interpolate(block, feature);
                    }
                }
            }
        }
        let inline = machine
            .inject(state)
            .and_then(Value::as_str)
            .map(strip_trailing_newlines)
            .unwrap_or("");
        if !inline.is_empty() {
            return interpolate(inline, feature);
        }
    }
    GENERIC_FALLBACK.to_string()
}

/// The cursor's state and feature. An absent cursor is idle; a *corrupt* one
/// also degrades to idle here — orders paper over it exactly like the oracle,
/// unlike `state get`, whose job is to surface the damage.
pub fn current_state_and_feature(paths: &FlowPaths) -> (String, Value) {
    match read_cursor(paths) {
        Ok(cursor) => (cursor.state, cursor.feature),
        Err(_) => ("idle".to_string(), Value::Null),
    }
}

/// Where sibling skills live: the data home's parent when it already holds
/// `vibe/SKILL.md` (the installed layout), else `<root>/.agents/skills`.
pub fn skills_dir(paths: &FlowPaths) -> PathBuf {
    let data = paths.data_dir();
    if let Some(parent) = data.parent() {
        if parent.join("vibe").join("SKILL.md").is_file() {
            return parent.to_path_buf();
        }
    }
    paths.root().join(".agents").join("skills")
}

/// `<feature>` is the only interpolation. A feature-less cursor leaves the
/// literal placeholder in place: still valid guidance, still byte-stable.
fn interpolate(text: &str, feature: &Value) -> String {
    let feature = jq_alt_raw(feature);
    if feature.is_empty() {
        text.to_string()
    } else {
        text.replace("<feature>", &feature)
    }
}

/// `jq -r '.feature // empty'`: `false`/`null` are empty; a string is raw;
/// any other value prints as pretty JSON (jq's default for non-scalars).
fn jq_alt_raw(value: &Value) -> String {
    match value {
        Value::Null | Value::Bool(false) => String::new(),
        Value::String(text) => text.clone(),
        other => serde_json::to_string_pretty(other).unwrap_or_default(),
    }
}

// ---------------------------------------------------------------------------
// Spec enrichment (the payload's edge only — see the module note)
// ---------------------------------------------------------------------------

/// The spec lines appended to the payload's edge.
///
/// A reachable `vibe-spec` gives the root and the lessons tagged for this state.
/// Anything else — absent binary, non-zero exit, unreadable JSON — degrades to
/// [`PATH_POINTER`] alone, so the edge still tells the agent where to look.
pub fn enrichment(root: &Path, state: &str) -> Vec<String> {
    let Some(raw_root) = probe(root, &["root", "--json"]) else {
        return vec![PATH_POINTER.to_string()];
    };
    let Ok(answer) = serde_json::from_str::<Value>(&raw_root) else {
        return vec![PATH_POINTER.to_string()];
    };
    let Some(spec_root) = answer
        .get("root")
        .and_then(Value::as_str)
        .filter(|text| !text.is_empty())
    else {
        return vec![PATH_POINTER.to_string()];
    };
    let mut lines = vec![format!("spec: {spec_root}")];
    let Some(raw_lessons) = probe(root, &["lessons-for", "--tag", state, "--json"]) else {
        return lines;
    };
    let Ok(answer) = serde_json::from_str::<Value>(&raw_lessons) else {
        return lines;
    };
    let lessons = answer
        .get("lessons")
        .and_then(Value::as_array)
        .map(|list| {
            list.iter()
                .filter_map(|lesson| {
                    let title = lesson.get("title").and_then(Value::as_str)?;
                    let rule = lesson.get("rule").and_then(Value::as_str)?;
                    Some(format!("- {title}: {rule}"))
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    if !lessons.is_empty() {
        lines.push(format!("lessons ({state}):"));
        lines.extend(lessons);
    }
    lines
}

/// A soft peer call: `vibe-spec <args>` run at the project root, stdout only,
/// bounded at 500 ms. `None` for an absent binary, a refusal, a timeout or
/// non-UTF-8 output — every one of them is the degrade branch.
fn probe(root: &Path, args: &[&str]) -> Option<String> {
    let mut child = Command::new("vibe-spec")
        .args(args)
        .current_dir(root)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let deadline = Instant::now() + Duration::from_millis(500);
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                if !status.success() {
                    return None;
                }
                let output = child.wait_with_output().ok()?;
                return String::from_utf8(output.stdout).ok();
            }
            Ok(None) if Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(10));
            }
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::{GENERIC_FALLBACK, PATH_POINTER, enrichment, run};
    use crate::paths::{Env, FlowPaths};
    use serde_json::json;
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
            std::env::temp_dir().join(format!("vibe-flow-orders-{name}-{}-{n}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("create scratch");
        dir
    }

    fn copy(src: &str, dest: &Path) {
        if let Some(dir) = dest.parent() {
            fs::create_dir_all(dir).expect("dir");
        }
        fs::copy(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../").join(src),
            dest,
        )
        .expect("copy");
    }

    /// A project whose data home is the legacy skill dir (the oracle layout).
    fn installed(name: &str) -> (PathBuf, FlowPaths) {
        let root = scratch(name);
        copy("flow/state-machine.json", &root.join(".agents/skills/vibe/state-machine.json"));
        copy("flow/SKILL.md", &root.join(".agents/skills/vibe/SKILL.md"));
        let content = root.join(".agents/skills/vibe/content");
        fs::create_dir_all(&content).expect("content dir");
        fs::write(content.join("policy.json"), "{\"version\":1,\"rules\":[]}\n").expect("policy");
        let paths = FlowPaths::resolve(Some(&root), None, &root, &NoEnv);
        (root, paths)
    }

    fn write_cursor(root: &Path, state: &str, feature: &str) {
        let dir = root.join(".vibe/run/flow");
        fs::create_dir_all(&dir).expect("cursor dir");
        let (flow, phase) = state.split_once('.').unwrap_or((state, state));
        let feature = if feature.is_empty() {
            "null".to_string()
        } else {
            format!("\"{feature}\"")
        };
        fs::write(
            dir.join("state.json"),
            format!(
                "{{\"flow\":\"{flow}\",\"phase\":\"{phase}\",\"feature\":{feature},\"updated\":\"2026-01-01T00:00:00Z\"}}\n"
            ),
        )
        .expect("cursor");
    }

    #[test]
    fn idle_prints_the_machines_inline_inject() {
        let (root, paths) = installed("idle");
        write_cursor(&root, "idle", "");
        let out = run(&paths, None);
        assert_eq!(out.code, 0);
        assert_eq!(
            out.stdout,
            "state=idle · no active flow · pick a flow → vibe-flow state set {setup.detect | strategy.brainstorm | feature.design | quick.triage}\n"
        );
        assert!(out.stderr.is_empty());
    }

    #[test]
    fn a_skill_state_prints_its_orders_block_and_interpolates_the_feature() {
        let (root, paths) = installed("feature");
        write_cursor(&root, "feature.verify", "auth");
        let out = run(&paths, None);
        assert_eq!(out.code, 0);
        assert!(out.stdout.starts_with("skill=vibe"), "{}", out.stdout);
        assert!(out.stdout.contains("feature-auth.md"), "{}", out.stdout);
        assert!(!out.stdout.contains("<feature>"), "{}", out.stdout);
    }

    #[test]
    fn an_explicit_state_argument_wins_over_the_cursor() {
        let (root, paths) = installed("arg");
        write_cursor(&root, "idle", "");
        let out = run(&paths, Some("quick.fix"));
        assert!(out.stdout.starts_with("skill=vibe"), "{}", out.stdout);
    }

    #[test]
    fn a_corrupt_cursor_degrades_to_idle_instead_of_failing() {
        let (root, paths) = installed("corrupt");
        let dir = root.join(".vibe/run/flow");
        fs::create_dir_all(&dir).expect("cursor dir");
        fs::write(dir.join("state.json"), "{ nope").expect("cursor");
        let out = run(&paths, None);
        assert_eq!(out.code, 0);
        assert!(out.stdout.starts_with("state=idle"), "{}", out.stdout);
    }

    #[test]
    fn a_missing_machine_falls_back_to_the_generic_line() {
        let root = scratch("no-machine");
        let paths = FlowPaths::resolve(Some(&root), None, &root, &NoEnv);
        let out = run(&paths, None);
        assert_eq!(out.code, 0);
        assert_eq!(out.stdout, format!("{GENERIC_FALLBACK}\n"));
    }

    #[test]
    fn a_missing_closer_never_leaks_the_file_tail() {
        let (root, paths) = installed("unclosed");
        fs::write(
            root.join(".agents/skills/vibe/SKILL.md"),
            "<!-- vibe:orders:quick.verify -->\nsecret tail\n",
        )
        .expect("skill");
        write_cursor(&root, "quick.verify", "");
        let out = run(&paths, None);
        assert!(!out.stdout.contains("secret tail"), "{}", out.stdout);
        assert_eq!(out.stdout, format!("{GENERIC_FALLBACK}\n"));
    }

    #[test]
    fn a_non_string_feature_prints_as_pretty_json() {
        let (root, paths) = installed("numeric");
        let dir = root.join(".vibe/run/flow");
        fs::create_dir_all(&dir).expect("cursor dir");
        fs::write(
            dir.join("state.json"),
            "{\"flow\":\"feature\",\"phase\":\"verify\",\"feature\":{\"b\":2,\"a\":1}}\n",
        )
        .expect("cursor");
        let out = run(&paths, None);
        // serde_json has no `preserve_order` in this workspace, so a non-string
        // feature renders with sorted keys; JS keeps insertion order. Cursors
        // carry `feature` as a string, so this edge is pinned here only.
        assert!(
            out.stdout.contains("{\n  \"a\": 1,\n  \"b\": 2\n}"),
            "{}",
            out.stdout
        );
    }

    #[test]
    fn without_a_spec_peer_the_enrichment_is_the_path_pointer_only() {
        let (root, _) = installed("probe-absent");
        // The ambient PATH may or may not carry vibe-spec; the scratch root has
        // no spec tree, so a present binary exits 3 and the pointer is still
        // the answer. Pin the contract, not the machine's install state.
        let lines = enrichment(&root, "feature.impl");
        assert!(
            lines == vec![PATH_POINTER.to_string()] || lines[0].starts_with("spec: "),
            "{lines:?}"
        );
        if lines == vec![PATH_POINTER.to_string()] {
            assert!(!lines.iter().any(|line| line.contains("lessons (")), "{lines:?}");
        }
    }

    #[test]
    fn the_feature_value_is_the_only_interpolation() {
        let text = super::interpolate("go <feature> now", &json!("auth"));
        assert_eq!(text, "go auth now");
        let text = super::interpolate("go <feature> now", &json!(null));
        assert_eq!(text, "go <feature> now");
        let text = super::interpolate("go <feature> now", &json!(false));
        assert_eq!(text, "go <feature> now");
    }
}
