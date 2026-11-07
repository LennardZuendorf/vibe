//! The transition ledger (D28's append side).
//!
//! One JSONL line per accepted `state set`, appended to
//! `.vibe/run/flow/ledger.jsonl`. Append-only: the writer never reads the file
//! back, never rewrites it and never truncates it, so a ledger written by an
//! older build stays readable. Rotation at 5 MB retaining one generation is
//! F7's pending half and is not implemented here.

use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;

use serde_json::Value;

use crate::cursor::now_utc_seconds;
use crate::paths::FlowPaths;

/// The ledger event kind this writer emits (D28: `transition | hook | signal`).
pub const TRANSITION: &str = "transition";

/// Appends one `transition` event for an accepted write.
///
/// `confirm` is `Some` only when the edge is gated, so an ordinary hop carries
/// no key it has no value for. The caller appends **after** the cursor write:
/// a refusal must leave both the cursor and the ledger untouched, and a write
/// that never landed is not an event.
pub fn append_transition(
    paths: &FlowPaths,
    from: &str,
    to: &str,
    feature: &Value,
    confirm: Option<bool>,
) -> Result<(), String> {
    let path = paths.ledger_path();
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)
            .map_err(|err| format!("cannot create {}: {err}", dir.display()))?;
    }
    constrain_ledger(&path)?;
    let body = event_line(from, to, feature, confirm)?;
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|err| format!("cannot open {}: {err}", path.display()))?;
    file.write_all(body.as_bytes())
        .map_err(|err| format!("cannot append to {}: {err}", path.display()))
}

/// One compact JSONL line, keys in a fixed order. `feature` keeps its JSON
/// shape (the cursor carries whatever it was given), and `confirm` sits after
/// it only on a gated edge.
fn event_line(
    from: &str,
    to: &str,
    feature: &Value,
    confirm: Option<bool>,
) -> Result<String, String> {
    let feature = serde_json::to_string(feature)
        .map_err(|err| format!("cannot encode the ledger feature: {err}"))?;
    let mut line = format!(
        "{{\"ts\":{},\"kind\":{},\"from\":{},\"to\":{},\"feature\":{feature}",
        json_string(&now_utc_seconds()),
        json_string(TRANSITION),
        json_string(from),
        json_string(to)
    );
    if let Some(confirm) = confirm {
        line.push_str(if confirm {
            ",\"confirm\":true"
        } else {
            ",\"confirm\":false"
        });
    }
    line.push_str("}\n");
    Ok(line)
}

fn json_string(text: &str) -> String {
    serde_json::to_string(text).unwrap_or_else(|_| "\"\"".to_string())
}

/// Append follows a symlink, so a linked leaf would write outside the project.
/// A ledger is never a link: refuse rather than follow.
fn constrain_ledger(path: &Path) -> Result<(), String> {
    match std::fs::symlink_metadata(path) {
        Ok(meta) if meta.file_type().is_symlink() => Err(format!(
            "refusing to append through the link {}",
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
    use super::{append_transition, event_line};
    use crate::paths::{Env, FlowPaths};
    use serde_json::{Value, json};
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
            "vibe-flow-ledger-{name}-{}-{n}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("create scratch");
        dir
    }

    fn paths_for(root: &Path) -> FlowPaths {
        FlowPaths::resolve(Some(root), None, root, &NoEnv)
    }

    #[test]
    fn an_ungated_event_carries_no_confirm_key() {
        let line = event_line("idle", "quick.triage", &json!(null), None).expect("line");
        assert_eq!(
            line.len() - 1,
            line.trim_end().len(),
            "one trailing newline"
        );
        assert!(line.ends_with(",\"feature\":null}\n"), "{line}");
        assert!(!line.contains("confirm"), "{line}");
        let parsed: Value = serde_json::from_str(line.trim_end()).expect("json");
        assert_eq!(parsed["kind"], json!("transition"));
        assert_eq!(parsed["from"], json!("idle"));
        assert_eq!(parsed["to"], json!("quick.triage"));
        assert_eq!(parsed["feature"], Value::Null);
        assert_eq!(parsed["ts"].as_str().map(str::len), Some(20));
    }

    #[test]
    fn a_gated_event_names_its_confirmation() {
        let line =
            event_line("feature.plan", "feature.impl", &json!("auth"), Some(true)).expect("line");
        let parsed: Value = serde_json::from_str(line.trim_end()).expect("json");
        assert_eq!(parsed["confirm"], json!(true));
        assert_eq!(parsed["feature"], json!("auth"));
        let line =
            event_line("feature.plan", "feature.impl", &json!(null), Some(false)).expect("line");
        assert!(line.contains(",\"confirm\":false}\n"), "{line}");
    }

    #[test]
    fn every_append_adds_exactly_one_line_and_never_rewrites() {
        let root = scratch("append");
        let paths = paths_for(&root);
        append_transition(&paths, "idle", "quick.triage", &Value::Null, None).expect("first");
        let first = fs::read_to_string(paths.ledger_path()).expect("ledger");
        append_transition(&paths, "quick.triage", "quick.fix", &json!("bug"), None)
            .expect("second");
        let both = fs::read_to_string(paths.ledger_path()).expect("ledger");
        assert!(both.starts_with(&first), "the first line was rewritten");
        assert_eq!(both.lines().count(), 2, "{both}");
    }

    #[test]
    fn the_ledger_lives_in_the_run_directory() {
        let root = scratch("path");
        let paths = paths_for(&root);
        append_transition(&paths, "idle", "quick.triage", &Value::Null, None).expect("append");
        assert_eq!(
            paths.ledger_path(),
            root.join(".vibe/run/flow/ledger.jsonl")
        );
        assert!(paths.ledger_path().is_file());
    }

    #[cfg(unix)]
    #[test]
    fn a_linked_ledger_is_refused_rather_than_followed() {
        let root = scratch("symlink");
        let outside = scratch("symlink-outside");
        let keeper = outside.join("keeper.jsonl");
        fs::write(&keeper, "keep\n").expect("keeper");
        let paths = paths_for(&root);
        fs::create_dir_all(paths.ledger_path().parent().expect("dir")).expect("dir");
        std::os::unix::fs::symlink(&keeper, paths.ledger_path()).expect("symlink");
        let err = append_transition(&paths, "idle", "quick.triage", &Value::Null, None)
            .expect_err("linked ledger");
        assert!(
            err.starts_with("refusing to append through the link "),
            "{err}"
        );
        assert_eq!(fs::read_to_string(&keeper).expect("keeper"), "keep\n");
    }

    #[cfg(unix)]
    #[test]
    fn a_dangling_ledger_link_is_refused_too() {
        let root = scratch("dangling");
        let paths = paths_for(&root);
        fs::create_dir_all(paths.ledger_path().parent().expect("dir")).expect("dir");
        std::os::unix::fs::symlink("missing.jsonl", paths.ledger_path()).expect("symlink");
        assert!(append_transition(&paths, "idle", "quick.triage", &Value::Null, None).is_err());
    }
}
