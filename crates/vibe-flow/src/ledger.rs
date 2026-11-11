//! The transition ledger (D28's append side).
//!
//! One JSONL line per accepted `state set`, appended to
//! `.vibe/run/flow/ledger.jsonl`. Append-only: the writer never reads the file
//! back, never rewrites it and never truncates it, so a ledger written by an
//! older build stays readable. When the live file reaches [`ROTATE_AT`] it is
//! renamed to `ledger.jsonl.1` — replacing any older generation, one is kept —
//! and a fresh file starts. The rotated bytes are moved, never dropped: a
//! rotation that cannot complete returns an error instead of appending.

use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::cursor::now_utc_seconds;
use crate::paths::FlowPaths;

/// The ledger event kind this writer emits (D28: `transition | hook | signal`).
pub const TRANSITION: &str = "transition";

/// Rotation threshold: the live ledger is rotated once it reaches 5 MB
/// (5 × 1024 × 1024 bytes). Read once per append, so tests inject their own.
pub const ROTATE_AT: u64 = 5 * 1024 * 1024;

/// The one retained generation beside the live file.
const ROTATED_SUFFIX: &str = ".1";

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
    append_transition_at(paths, from, to, feature, confirm, ROTATE_AT)
}

/// [`append_transition`] with the rotation threshold injected.
pub fn append_transition_at(
    paths: &FlowPaths,
    from: &str,
    to: &str,
    feature: &Value,
    confirm: Option<bool>,
    rotate_at: u64,
) -> Result<(), String> {
    let path = paths.ledger_path();
    prepare_at(&path, rotate_at)?;
    let body = event_line(from, to, feature, confirm)?;
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|err| format!("cannot open {}: {err}", path.display()))?;
    file.write_all(body.as_bytes())
        .map_err(|err| format!("cannot append to {}: {err}", path.display()))
}

/// Everything the next append needs, checked and done **before** the caller
/// writes the cursor.
///
/// The cursor write commits the transition; an append refused afterwards
/// would leave the cursor advanced with no event recording how it got there.
/// So the run directory is created, both leaves are refused if they are links,
/// the rotation is performed while nothing has been committed yet, and the
/// live file is opened for append once to prove it can be. Only a disk that
/// fails between this call and the append itself can still lose the event, and
/// that failure is reported rather than swallowed.
///
/// Rotating here means a rotation can precede a cursor write that then fails.
/// Nothing is lost when that happens: the rotated bytes stay in place and the
/// live file simply restarts empty.
pub fn prepare(paths: &FlowPaths) -> Result<(), String> {
    prepare_at(&paths.ledger_path(), ROTATE_AT)
}

/// [`prepare`] with the rotation threshold injected.
pub fn prepare_at(path: &Path, rotate_at: u64) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)
            .map_err(|err| format!("cannot create {}: {err}", dir.display()))?;
    }
    constrain(path, "append through the link")?;
    if live_len(path)? >= rotate_at {
        let rotated = rotated_path(path);
        constrain(&rotated, "rotate over the link")?;
        rotate(path, &rotated)?;
    }
    probe(path)
}

/// The rotated generation beside `path`: `ledger.jsonl` → `ledger.jsonl.1`.
pub fn rotated_path(path: &Path) -> PathBuf {
    let mut name = path
        .file_name()
        .map(|name| name.to_os_string())
        .unwrap_or_else(|| "ledger.jsonl".into());
    name.push(ROTATED_SUFFIX);
    path.with_file_name(name)
}

/// Size of the live file, `0` when it does not exist yet.
fn live_len(path: &Path) -> Result<u64, String> {
    match std::fs::metadata(path) {
        Ok(meta) => Ok(meta.len()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(0),
        Err(err) => Err(format!("cannot inspect {}: {err}", path.display())),
    }
}

/// Moves the live file onto the one retained generation.
///
/// Unix `rename` replaces the target atomically, so the older generation only
/// stops existing as a consequence of the new one appearing. Windows refuses
/// an existing target, so it is dropped first there — and that drop loses
/// nothing, because the live file still holds every line written since the
/// previous rotation until the rename lands.
fn rotate(live: &Path, rotated: &Path) -> Result<(), String> {
    #[cfg(windows)]
    match std::fs::symlink_metadata(rotated) {
        Ok(_) => {
            std::fs::remove_file(rotated).map_err(|err| {
                format!("cannot replace the generation {}: {err}", rotated.display())
            })?;
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
        Err(err) => return Err(format!("cannot inspect {}: {err}", rotated.display())),
    }
    std::fs::rename(live, rotated).map_err(|err| {
        format!(
            "cannot rotate {} to {}: {err}",
            live.display(),
            rotated.display()
        )
    })
}

/// Opening for append is the capability being promised, so prove it now.
fn probe(path: &Path) -> Result<(), String> {
    OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map(|_| ())
        .map_err(|err| format!("cannot open {}: {err}", path.display()))
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
/// A ledger is never a link, and neither is the generation it rotates onto:
/// refuse rather than follow.
fn constrain(path: &Path, action: &str) -> Result<(), String> {
    match std::fs::symlink_metadata(path) {
        Ok(meta) if meta.file_type().is_symlink() => {
            Err(format!("refusing to {action} {}", path.display()))
        }
        Ok(_) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(format!("cannot inspect {}: {err}", path.display())),
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::{append_transition, append_transition_at, event_line, rotated_path};
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

    fn live_len(paths: &FlowPaths) -> u64 {
        fs::metadata(paths.ledger_path()).expect("live").len()
    }

    fn names(dir: &Path) -> Vec<String> {
        let mut names: Vec<String> = fs::read_dir(dir)
            .expect("run dir")
            .filter_map(Result::ok)
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }

    #[test]
    fn the_ledger_rotates_once_at_the_threshold_and_keeps_every_line() {
        let root = scratch("rotate");
        let paths = paths_for(&root);
        let rotated = rotated_path(&paths.ledger_path());
        let hops = [
            ("idle", "quick.triage"),
            ("quick.triage", "quick.fix"),
            ("quick.fix", "idle"),
        ];
        // Three events first, so the threshold can be set from the file the
        // writer actually produced rather than from a guessed line length.
        for (from, to) in hops {
            append_transition_at(&paths, from, to, &Value::Null, None, u64::MAX).expect("append");
        }
        let rotate_at = live_len(&paths);
        let before = fs::read_to_string(paths.ledger_path()).expect("live");
        assert_eq!(before.lines().count(), 3);
        assert!(!rotated.exists(), "rotated below the threshold");

        // The boundary: the live file sits at the threshold, so this append
        // rotates first and starts a fresh file.
        append_transition_at(&paths, hops[0].0, hops[0].1, &Value::Null, None, rotate_at)
            .expect("rotating append");
        assert_eq!(
            fs::read_to_string(&rotated).expect("generation"),
            before,
            "the rotated generation did not keep the live file's bytes"
        );
        assert_eq!(
            fs::read_to_string(paths.ledger_path())
                .expect("live")
                .lines()
                .count(),
            1,
            "the live file did not start fresh"
        );
        // One generation, not a numbered series.
        assert_eq!(
            names(paths.ledger_path().parent().expect("dir")),
            vec!["ledger.jsonl".to_string(), "ledger.jsonl.1".to_string()]
        );

        // Two events after the rotation, neither reaching the threshold again:
        // exactly one rotation happened, so 3 events before plus the rotating
        // event and 2 after are 3 + 3 lines.
        for (from, to) in [hops[1], hops[2]] {
            append_transition_at(&paths, from, to, &Value::Null, None, rotate_at).expect("append");
        }
        let live = fs::read_to_string(paths.ledger_path()).expect("live");
        let kept = fs::read_to_string(&rotated).expect("generation");
        assert_eq!(
            live.lines().count() + kept.lines().count(),
            6,
            "3 events before + 3 after, across the two files"
        );
        assert_eq!(
            (kept.lines().count(), live.lines().count()),
            (3, 3),
            "the rotation split the lines in the wrong place"
        );
        assert_eq!(
            kept, before,
            "the generation was overwritten by a second rotation"
        );
        assert!(live.starts_with("{\"ts\":"), "{live}");
        assert_eq!(
            names(paths.ledger_path().parent().expect("dir")),
            vec!["ledger.jsonl".to_string(), "ledger.jsonl.1".to_string()]
        );
    }

    #[test]
    fn an_absent_ledger_appends_without_creating_a_generation() {
        let root = scratch("rotate-absent");
        let paths = paths_for(&root);
        append_transition_at(&paths, "idle", "quick.triage", &Value::Null, None, 1)
            .expect("append");
        assert!(
            !rotated_path(&paths.ledger_path()).exists(),
            "rotated an absent ledger"
        );
        assert_eq!(
            fs::read_to_string(paths.ledger_path())
                .expect("live")
                .lines()
                .count(),
            1
        );
    }

    #[cfg(unix)]
    #[test]
    fn a_linked_generation_is_refused_rather_than_replaced() {
        let root = scratch("rotate-symlink");
        let outside = scratch("rotate-symlink-outside");
        let keeper = outside.join("keeper.jsonl");
        fs::write(&keeper, "keep\n").expect("keeper");
        let paths = paths_for(&root);
        append_transition(&paths, "idle", "quick.triage", &Value::Null, None).expect("first");
        let live = fs::read_to_string(paths.ledger_path()).expect("live");
        std::os::unix::fs::symlink(&keeper, rotated_path(&paths.ledger_path())).expect("symlink");

        let err = append_transition_at(&paths, "idle", "quick.triage", &Value::Null, None, 1)
            .expect_err("linked generation");
        assert!(
            err.starts_with("refusing to rotate over the link "),
            "{err}"
        );
        assert!(err.ends_with("ledger.jsonl.1"), "{err}");
        assert_eq!(fs::read_to_string(&keeper).expect("keeper"), "keep\n");
        assert_eq!(
            fs::read_to_string(paths.ledger_path()).expect("live"),
            live,
            "a refused rotation swallowed or duplicated the event"
        );
    }
}
