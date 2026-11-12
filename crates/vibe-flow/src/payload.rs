//! Payload v1: the file instruct composes from (D22).
//!
//! `.vibe/run/inject/flow.json` carries
//! `{v, provider, seq, level, edge, session, events}`. `state set` rewrites
//! `level`/`edge`/`session` and bumps `seq` (`FV-R7`); the guard and Stop hooks
//! append `events`. Field order is fixed by the contract, so the encoder writes
//! it by hand instead of leaning on map ordering.
//!
//! Writes are atomic and pre-flighted exactly like the ledger's:
//! [`prepare`] creates the directory, refuses a symlinked leaf, and proves the
//! file can be read before the cursor moves. A refused write leaves the old
//! payload byte-identical, so `seq` is never bumped without the content that
//! explains it. A payload that cannot be written is advisory (`FV-R7`): the
//! caller warns and continues.

use std::path::Path;

use serde_json::{Value, json};
use vibe_core::fs::write_atomic;

use crate::cursor::{now_utc_seconds, read_cursor};
use crate::machine::Machine;
use crate::paths::FlowPaths;

/// Contract version this writer emits.
pub const VERSION: u64 = 1;
/// The provider id this writer owns.
pub const PROVIDER: &str = "flow";
/// `session` carries at most this many doctrine lines.
pub const SESSION_LINES: usize = 10;

/// The decoded payload. Missing or unusable fields degrade to their empty
/// shape: a corrupt payload is advisory data, never a hard failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Payload {
    /// Transition counter; instruct injects the edge when it advances.
    pub seq: u64,
    /// The every-turn line.
    pub level: String,
    /// The full orders markdown, injected on the turn after a transition.
    pub edge: String,
    /// Timeless doctrine lines for the session-start channel.
    pub session: Vec<String>,
    /// One-shot events, each `{id, ts, text}`.
    pub events: Vec<Value>,
}

impl Default for Payload {
    fn default() -> Payload {
        Payload {
            seq: 0,
            level: String::new(),
            edge: String::new(),
            session: Vec::new(),
            events: Vec::new(),
        }
    }
}

/// The payload body as the contract spells it: pretty JSON, fixed key order,
/// trailing newline.
pub fn encode(payload: &Payload) -> String {
    let session = serde_json::to_string(&payload.session).unwrap_or_else(|_| "[]".to_string());
    let events = encode_events(&payload.events);
    format!(
        "{{\n  \"v\": {VERSION},\n  \"provider\": {},\n  \"seq\": {},\n  \"level\": {},\n  \"edge\": {},\n  \"session\": {session},\n  \"events\": {events}\n}}\n",
        json_string(PROVIDER),
        payload.seq,
        json_string(&payload.level),
        json_string(&payload.edge)
    )
}

/// Events are written in contract order (`id`, `ts`, `text`). Anything that is
/// not that exact shape is encoded as-is, so a foreign event is never dropped.
fn encode_events(events: &[Value]) -> String {
    let parts: Vec<String> = events
        .iter()
        .map(|event| {
            let (Some(id), Some(ts), Some(text)) = (
                event.get("id").and_then(Value::as_str),
                event.get("ts").and_then(Value::as_str),
                event.get("text").and_then(Value::as_str),
            ) else {
                return serde_json::to_string(event).unwrap_or_else(|_| "null".to_string());
            };
            format!(
                "{{\"id\":{},\"ts\":{},\"text\":{}}}",
                json_string(id),
                json_string(ts),
                json_string(text)
            )
        })
        .collect();
    format!("[{}]", parts.join(","))
}

fn json_string(text: &str) -> String {
    serde_json::to_string(text).unwrap_or_else(|_| "\"\"".to_string())
}

/// Reads the payload; an absent, unreadable or malformed file reads as the
/// empty payload (seq 0, no events).
pub fn read(paths: &FlowPaths) -> Payload {
    let Ok(bytes) = std::fs::read(paths.inject_path()) else {
        return Payload::default();
    };
    let Ok(raw) = serde_json::from_slice::<Value>(&bytes) else {
        return Payload::default();
    };
    let seq = raw.get("seq").and_then(Value::as_u64).unwrap_or(0);
    let level = raw
        .get("level")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let edge = raw
        .get("edge")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let session = raw
        .get("session")
        .and_then(Value::as_array)
        .map(|lines| {
            lines
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();
    let events = raw
        .get("events")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    Payload {
        seq,
        level,
        edge,
        session,
        events,
    }
}

/// Pre-flight for the next payload write: create the directory, refuse to
/// append through a link, and prove the existing leaf is readable.
///
/// Called before the cursor moves, for the same reason [`crate::ledger`] is:
/// an append refused after the commit would leave `seq` (or the cursor) ahead
/// of the record that explains it.
pub fn prepare(paths: &FlowPaths) -> Result<(), String> {
    let path = paths.inject_path();
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)
            .map_err(|err| format!("cannot create {}: {err}", dir.display()))?;
    }
    constrain(&path, "append through the link")?;
    match std::fs::read(&path) {
        Ok(_) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(format!("cannot read {}: {err}", path.display())),
    }
}

/// Rewrites `level`/`edge`/`session` from the cursor and bumps `seq`.
///
/// `events` is carried forward untouched: a transition does not consume what
/// the hooks queued. The caller writes the cursor first, so this reads the new
/// state back and describes it.
pub fn write_transition(paths: &FlowPaths) -> Result<Payload, String> {
    prepare(paths)?;
    let previous = read(paths);
    let payload = build(paths, previous.seq.saturating_add(1), previous.events);
    write(paths, &payload)?;
    Ok(payload)
}

/// Rebuilds the content for the cursor without bumping `seq`.
///
/// `emit` uses this so a stale payload still names the live state; `seq` stays
/// tied to transitions, or the edge would be injected on every turn.
pub fn refresh(paths: &FlowPaths) -> Payload {
    let previous = read(paths);
    build(paths, previous.seq, previous.events)
}

/// Appends one event and rewrites the payload atomically.
///
/// The pre-flight is [`prepare`], so a symlinked payload is refused with the
/// ledger's own wording and the file keeps its previous bytes.
pub fn append_event(paths: &FlowPaths, text: &str) -> Result<(), String> {
    prepare(paths)?;
    let mut payload = read(paths);
    if payload.level.is_empty() && payload.edge.is_empty() {
        payload = build(paths, payload.seq, payload.events);
    }
    let id = format!("e{}-{}", payload.seq, payload.events.len() + 1);
    payload.events.push(json!({
        "id": id,
        "ts": now_utc_seconds(),
        "text": text,
    }));
    write(paths, &payload)
}

/// The content half of a payload, for `seq` and the carried `events`.
fn build(paths: &FlowPaths, seq: u64, events: Vec<Value>) -> Payload {
    let (state, feature) = match read_cursor(paths) {
        Ok(cursor) => (cursor.state, cursor.feature),
        Err(_) => ("idle".to_string(), Value::Null),
    };
    let machine = Machine::load(paths.data_dir()).ok();
    let level = level_line(&state, &feature, machine.as_ref());
    let mut edge = crate::orders::orders_text(paths, &state, &feature);
    let enrichment = crate::orders::enrichment(paths.root(), &state);
    if !enrichment.is_empty() {
        edge.push('\n');
        edge.push('\n');
        edge.push_str(&enrichment.join("\n"));
    }
    let session = crate::doctrine::doctrine_text(paths)
        .lines()
        .filter(|line| !line.trim().is_empty())
        .take(SESSION_LINES)
        .map(str::to_string)
        .collect();
    Payload {
        seq,
        level,
        edge,
        session,
        events,
    }
}

/// `flow: <state>[ · <feature>][ · next: <a, b (gate)>]`.
fn level_line(state: &str, feature: &Value, machine: Option<&Machine>) -> String {
    let mut line = format!("flow: {state}");
    let feature = match feature {
        Value::Null | Value::Bool(false) => String::new(),
        Value::String(text) => text.clone(),
        other => serde_json::to_string(other).unwrap_or_default(),
    };
    if !feature.is_empty() {
        line.push_str(&format!(" · {feature}"));
    }
    let next = machine.map(|machine| machine.next(state)).unwrap_or_default();
    if !next.is_empty() {
        let marked: Vec<String> = next
            .iter()
            .map(|target| {
                if machine.is_some_and(|machine| machine.gate(state, target)) {
                    format!("{target} (gate)")
                } else {
                    target.clone()
                }
            })
            .collect();
        line.push_str(&format!(" · next: {}", marked.join(", ")));
    }
    line
}

/// Writes `payload` atomically, after refusing a linked leaf.
pub fn write(paths: &FlowPaths, payload: &Payload) -> Result<(), String> {
    let path = paths.inject_path();
    constrain(&path, "append through the link")?;
    write_atomic(&path, encode(payload).as_bytes()).map_err(|err| err.to_string())
}

/// A payload is never a link: append and rename would both write outside the
/// project through one. Refuse rather than follow, with the ledger's wording.
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
    use super::{Payload, append_event, encode, prepare, read, refresh, write_transition};
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
            std::env::temp_dir().join(format!("vibe-flow-payload-{name}-{}-{n}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("create scratch");
        dir
    }

    fn copy(src: &str, dest: &Path) {
        if let Some(dir) = dest.parent() {
            fs::create_dir_all(dir).expect("dir");
        }
        fs::copy(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../")
                .join(src),
            dest,
        )
        .expect("copy");
    }

    fn install(name: &str) -> (PathBuf, FlowPaths) {
        let root = scratch(name);
        copy(
            "flow/state-machine.json",
            &root.join(".agents/skills/vibe/state-machine.json"),
        );
        copy("flow/SKILL.md", &root.join(".agents/skills/vibe/SKILL.md"));
        let paths = FlowPaths::resolve(Some(&root), None, &root, &NoEnv);
        (root, paths)
    }

    fn cursor(root: &Path, state: &str, feature: &str) {
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
    fn encode_writes_the_contract_shape_in_contract_order() {
        let payload = Payload {
            seq: 7,
            level: "flow: idle".to_string(),
            edge: "orders".to_string(),
            session: vec!["one".to_string()],
            events: vec![serde_json::json!({"id":"e7-1","ts":"t","text":"x"})],
        };
        let text = encode(&payload);
        assert!(text.starts_with("{\n  \"v\": 1,\n  \"provider\": \"flow\",\n  \"seq\": 7,\n"), "{text}");
        assert!(
            text.contains("\"level\": \"flow: idle\",\n  \"edge\": \"orders\",\n  \"session\": [\"one\"],\n  \"events\": [{\"id\":\"e7-1\",\"ts\":\"t\",\"text\":\"x\"}]"),
            "{text}"
        );
        assert!(text.ends_with("}\n"));
        let parsed: Value = serde_json::from_str(&text).expect("json");
        assert_eq!(parsed["v"], 1);
        assert_eq!(parsed["provider"], "flow");
    }

    #[test]
    fn a_transition_bumps_seq_and_rewrites_level_edge_and_session() {
        let (root, paths) = install("transition");
        cursor(&root, "idle", "");
        let first = write_transition(&paths).expect("first");
        assert_eq!(first.seq, 1);
        assert!(first.level.starts_with("flow: idle"), "{}", first.level);
        assert!(!first.edge.is_empty());
        assert!(!first.session.is_empty());

        cursor(&root, "feature.impl", "auth");
        let second = write_transition(&paths).expect("second");
        assert_eq!(second.seq, 2);
        assert!(second.level.contains("feature.impl"), "{}", second.level);
        assert!(second.level.contains("auth"), "{}", second.level);
        assert!(
            second.edge.starts_with("skill=vibe"),
            "{}",
            second.edge
        );
        assert_eq!(read(&paths), second);
    }

    #[test]
    fn a_transition_carries_queued_events_forward() {
        let (root, paths) = install("carry");
        cursor(&root, "idle", "");
        write_transition(&paths).expect("seed");
        append_event(&paths, "guard: blocked docs/spec/plan.md").expect("event");
        let after = write_transition(&paths).expect("transition");
        assert_eq!(after.seq, 2);
        assert_eq!(after.events.len(), 1);
        assert_eq!(after.events[0]["text"], "guard: blocked docs/spec/plan.md");
        assert_eq!(after.events[0]["id"], "e1-1");
    }

    #[test]
    fn an_event_append_keeps_seq_but_adds_one_event() {
        let (root, paths) = install("event");
        cursor(&root, "idle", "");
        write_transition(&paths).expect("seed");
        append_event(&paths, "gate: warn").expect("event");
        let payload = read(&paths);
        assert_eq!(payload.seq, 1);
        assert_eq!(payload.events.len(), 1);
        assert_eq!(payload.events[0]["id"], "e1-1");
        append_event(&paths, "gate: second").expect("event");
        let payload = read(&paths);
        assert_eq!(payload.events.len(), 2);
        assert_eq!(payload.events[1]["id"], "e1-2");
    }

    #[test]
    fn refresh_keeps_seq_and_follows_the_cursor() {
        let (root, paths) = install("refresh");
        cursor(&root, "idle", "");
        write_transition(&paths).expect("seed");
        cursor(&root, "quick.fix", "bug");
        let refreshed = refresh(&paths);
        assert_eq!(refreshed.seq, 1);
        assert!(refreshed.level.contains("quick.fix"), "{}", refreshed.level);
        assert!(
            refreshed.edge.starts_with("skill=vibe"),
            "{}",
            refreshed.edge
        );
    }

    #[test]
    fn the_payload_lives_where_the_contract_says() {
        let (root, paths) = install("path");
        cursor(&root, "idle", "");
        write_transition(&paths).expect("write");
        assert!(root.join(".vibe/run/inject/flow.json").is_file());
        // No temp file survives a write.
        let names: Vec<String> = fs::read_dir(root.join(".vibe/run/inject"))
            .expect("dir")
            .map(|entry| entry.expect("entry").file_name().to_string_lossy().into_owned())
            .collect();
        assert_eq!(names, ["flow.json"]);
    }

    #[cfg(unix)]
    #[test]
    fn a_linked_payload_is_refused_and_keeps_its_bytes() {
        let (root, paths) = install("symlink");
        let outside = scratch("symlink-outside");
        let keeper = outside.join("keeper.json");
        fs::write(&keeper, "keep\n").expect("keeper");
        fs::create_dir_all(root.join(".vibe/run/inject")).expect("dir");
        std::os::unix::fs::symlink(&keeper, root.join(".vibe/run/inject/flow.json")).expect("link");

        let err = prepare(&paths).expect_err("linked payload");
        assert!(err.starts_with("refusing to append through the link "), "{err}");
        let err = write_transition(&paths).expect_err("linked payload");
        assert!(err.starts_with("refusing to append through the link "), "{err}");
        assert_eq!(fs::read_to_string(&keeper).expect("keeper"), "keep\n");
    }

    #[cfg(unix)]
    #[test]
    fn an_event_append_is_refused_on_a_linked_payload_too() {
        let (root, paths) = install("event-symlink");
        let outside = scratch("event-outside");
        let keeper = outside.join("keeper.json");
        fs::write(&keeper, "keep\n").expect("keeper");
        fs::create_dir_all(root.join(".vibe/run/inject")).expect("dir");
        std::os::unix::fs::symlink(&keeper, root.join(".vibe/run/inject/flow.json")).expect("link");
        let err = append_event(&paths, "guard: x").expect_err("linked payload");
        assert!(err.starts_with("refusing to append through the link "), "{err}");
        assert_eq!(fs::read_to_string(&keeper).expect("keeper"), "keep\n");
    }

    #[test]
    fn a_corrupt_payload_reads_as_empty_and_is_replaced() {
        let (root, paths) = install("corrupt");
        cursor(&root, "idle", "");
        fs::create_dir_all(root.join(".vibe/run/inject")).expect("dir");
        fs::write(root.join(".vibe/run/inject/flow.json"), "{ nope").expect("payload");
        assert_eq!(read(&paths), Payload::default());
        let written = write_transition(&paths).expect("write");
        assert_eq!(written.seq, 1);
        assert_eq!(read(&paths), written);
    }

    #[test]
    fn the_level_names_the_gate_on_a_gated_next() {
        let (root, paths) = install("gate");
        cursor(&root, "feature.verify", "auth");
        let payload = write_transition(&paths).expect("write");
        assert!(
            payload.level.contains("feature.compound (gate)"),
            "{}",
            payload.level
        );
    }
}
