//! The SessionStart hook entry shared by Claude, Codex and Factory.
use std::path::Path;

use vibe_core::json::{self, Object, Value};

use crate::paths::Harness;

const EVENT: &str = "SessionStart";
const MATCHER: &str = "startup|resume|clear";

/// The characters a double-quoted shell word cannot hold literally. A backslash is an
/// ordinary path separator on Windows, so it is rejected elsewhere only.
fn unquotable(c: char) -> bool {
    matches!(c, '"' | '$' | '`' | '\n' | '\r') || (c == '\\' && !cfg!(windows))
}

/// The hook command for `vibe_bin`. A path with a character that a double-quoted shell word
/// cannot hold (`"`, `$`, a backtick, a newline, and `\` off Windows) is refused.
pub fn hook_command(vibe_bin: &Path, h: Harness) -> Result<String, String> {
    let bin = vibe_bin.to_string_lossy();
    if let Some(c) = bin.chars().find(|c| unquotable(*c)) {
        return Err(format!(
            "{bin}: path contains {} and cannot be quoted safely",
            c.escape_debug()
        ));
    }
    Ok(format!("\"{bin}\" hook session-start --harness {}", h.id()))
}

fn commands(group: &Value) -> Vec<&str> {
    group
        .path(&["hooks"])
        .and_then(Value::as_array)
        .map(|hs| {
            hs.iter()
                .filter_map(|h| h.path(&["command"]).and_then(Value::as_str))
                .collect()
        })
        .unwrap_or_default()
}

/// True when some `SessionStart` hook has exactly this command.
pub fn has_hook(doc: &Value, command: &str) -> bool {
    doc.path(&["hooks", EVENT])
        .and_then(Value::as_array)
        .is_some_and(|groups| groups.iter().any(|g| commands(g).contains(&command)))
}

fn entry(command: &str) -> Value {
    let mut hook = Object::new();
    hook.insert("type", Value::str("command"));
    hook.insert("command", Value::str(command));
    hook.insert("timeout", json::parse("10").unwrap_or(Value::Null));
    let mut group = Object::new();
    group.insert("matcher", Value::str(MATCHER));
    group.insert("hooks", Value::Array(vec![Value::Object(hook)]));
    Value::Object(group)
}

/// Appends the hook group unless a hook with this command exists. `Ok(true)` when the
/// document changed; `Err` when the document, `hooks` or `SessionStart` has the wrong type
/// (nothing is changed then).
pub fn add_hook(doc: &mut Value, command: &str) -> Result<bool, String> {
    check_shape(doc)?;
    if has_hook(doc, command) {
        return Ok(false);
    }
    let Some(root) = doc.as_object_mut() else {
        return Err(WRONG_ROOT.to_string());
    };
    if root.get("hooks").is_none() {
        root.insert("hooks", Value::Object(Object::new()));
    }
    let Some(hooks) = root.get_mut("hooks").and_then(Value::as_object_mut) else {
        return Err(WRONG_HOOKS.to_string());
    };
    if hooks.get(EVENT).is_none() {
        hooks.insert(EVENT, Value::Array(Vec::new()));
    }
    let Some(groups) = hooks.get_mut(EVENT).and_then(Value::as_array_mut) else {
        return Err(WRONG_EVENT.to_string());
    };
    groups.push(entry(command));
    Ok(true)
}

const WRONG_ROOT: &str = "top level is not an object";
const WRONG_HOOKS: &str = "\"hooks\" is not an object";
const WRONG_EVENT: &str = "\"hooks.SessionStart\" is not an array";

fn check_shape(doc: &Value) -> Result<(), String> {
    let root = doc.as_object().ok_or(WRONG_ROOT)?;
    match root.get("hooks") {
        None => Ok(()),
        Some(h) => match h.as_object().ok_or(WRONG_HOOKS)?.get(EVENT) {
            None => Ok(()),
            Some(e) => e.as_array().map(|_| ()).ok_or(WRONG_EVENT.to_string()),
        },
    }
}

/// Removes the hook with this command. A group left without hooks is dropped, then
/// `SessionStart` and `hooks` when they become empty. Nothing else is touched. `Ok(true)`
/// when the document changed; `Err` on a wrong type.
pub fn remove_hook(doc: &mut Value, command: &str) -> Result<bool, String> {
    check_shape(doc)?;
    if !has_hook(doc, command) {
        return Ok(false);
    }
    let Some(root) = doc.as_object_mut() else {
        return Err(WRONG_ROOT.to_string());
    };
    let Some(hooks) = root.get_mut("hooks").and_then(Value::as_object_mut) else {
        return Err(WRONG_HOOKS.to_string());
    };
    if let Some(groups) = hooks.get_mut(EVENT).and_then(Value::as_array_mut) {
        let mut emptied = Vec::with_capacity(groups.len());
        for g in groups.iter_mut() {
            let list = g
                .as_object_mut()
                .and_then(|o| o.get_mut("hooks"))
                .and_then(Value::as_array_mut);
            emptied.push(list.is_some_and(|l| {
                let before = l.len();
                l.retain(|h| h.path(&["command"]).and_then(Value::as_str) != Some(command));
                l.len() < before && l.is_empty()
            }));
        }
        let mut flags = emptied.into_iter();
        groups.retain(|_| !flags.next().unwrap_or(false));
    }
    if hooks
        .get(EVENT)
        .and_then(Value::as_array)
        .is_some_and(Vec::is_empty)
    {
        hooks.remove(EVENT);
    }
    if hooks.is_empty() {
        root.remove("hooks");
    }
    Ok(true)
}
