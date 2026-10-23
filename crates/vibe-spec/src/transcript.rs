//! Skill-load detection in a Claude Code session transcript (P4-R2).
//!
//! A skill `N` is loaded when an `assistant` record holds a `tool_use` item
//! named `Skill` whose `input.skill` is `N` or `<ns>:N`, or when a `user`
//! record's string content holds `<command-name>/N</command-name>` (or
//! `/<ns>:N`). Text anywhere else never counts.
use std::fs::File;
use std::io::{BufRead, BufReader, ErrorKind};
use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::text;

const OPEN: &str = "<command-name>";
const CLOSE: &str = "</command-name>";

/// True when any of `names` is loaded in `main`, or, when `agent_id` is set,
/// in that subagent's own transcript. A missing subagent file is "not loaded".
pub fn loaded(main: &Path, agent_id: Option<&str>, names: &[&str]) -> std::io::Result<bool> {
    if scan(main, names)? {
        return Ok(true);
    }
    let Some(path) = agent_id.and_then(|id| subagent_path(main, id)) else {
        return Ok(false);
    };
    match scan(&path, names) {
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(false),
        other => other,
    }
}

/// `<main without .jsonl>/subagents/agent-<id>.jsonl`; `None` for an id that
/// could escape that directory.
fn subagent_path(main: &Path, id: &str) -> Option<PathBuf> {
    let safe = !id.is_empty()
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_');
    safe.then(|| {
        main.with_extension("")
            .join("subagents")
            .join(format!("agent-{id}.jsonl"))
    })
}

/// Streams `path` line by line; only lines that can hold a load are parsed.
fn scan(path: &Path, names: &[&str]) -> std::io::Result<bool> {
    let mut reader = BufReader::with_capacity(1 << 16, File::open(path)?);
    let mut line = Vec::new();
    loop {
        line.clear();
        if reader.read_until(b'\n', &mut line)? == 0 {
            return Ok(false);
        }
        if text::find(&line, b"\"Skill\"").is_none() && text::find(&line, OPEN.as_bytes()).is_none()
        {
            continue;
        }
        if let Ok(record) = serde_json::from_slice::<Value>(&line)
            && record_loads(&record, names)
        {
            return Ok(true);
        }
    }
}

fn matches(skill: &str, names: &[&str]) -> bool {
    names.iter().any(|n| {
        skill == *n
            || (!n.contains(':')
                && skill
                    .strip_suffix(n)
                    .is_some_and(|head| head.ends_with(':') && head.len() > 1))
    })
}

fn record_loads(record: &Value, names: &[&str]) -> bool {
    let content = &record["message"]["content"];
    match record["type"].as_str() {
        Some("assistant") => content.as_array().is_some_and(|items| {
            items.iter().any(|item| {
                item["type"] == "tool_use"
                    && item["name"] == "Skill"
                    && item["input"]["skill"]
                        .as_str()
                        .is_some_and(|s| matches(s, names))
            })
        }),
        Some("user") => content
            .as_str()
            .is_some_and(|s| command_names(s).any(|c| matches(c, names))),
        _ => false,
    }
}

/// Each `<command-name>/x</command-name>` in `s`, yielding `x`.
fn command_names(s: &str) -> impl Iterator<Item = &str> {
    s.split(OPEN).skip(1).filter_map(|rest| {
        let (name, _) = rest.split_once(CLOSE)?;
        name.strip_prefix('/')
    })
}
