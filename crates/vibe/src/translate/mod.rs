//! Pure per-harness translation of canonical MCP servers.
pub mod claude;
pub mod codex;
pub mod factory;
pub mod toml;

pub use claude::claude;
pub use codex::{CodexBody, CodexServer, codex};
pub use factory::factory;

use vibe_core::json::{Object, Value};

use crate::store::mcp::{McpServer, Transport, placeholders};

/// The result of translating one server: a projection or a one-line skip reason.
#[derive(Clone, Debug, PartialEq)]
pub enum Translated<T> {
    /// The server translates.
    Ok(T),
    /// The server is not projected; the reason has no trailing period and names fields only.
    Skip(String),
}

/// Skips when `command`, `args` or `url` hold a placeholder.
pub(crate) fn reject_placeholders<T>(s: &McpServer) -> Option<Translated<T>> {
    let hit = |what: &str| Some(Translated::Skip(format!("placeholder in {what}")));
    match s.transport {
        Transport::Stdio => {
            if s.command
                .as_deref()
                .is_some_and(|c| !placeholders(c).is_empty())
            {
                return hit("command");
            }
            if s.args.iter().any(|a| !placeholders(a).is_empty()) {
                return hit("args");
            }
        }
        Transport::Http => {
            if s.url
                .as_deref()
                .is_some_and(|u| !placeholders(u).is_empty())
            {
                return hit("url");
            }
        }
    }
    None
}

/// Skips when the field the transport needs is absent.
pub(crate) fn require_target<T>(s: &McpServer) -> Option<Translated<T>> {
    match (&s.transport, &s.command, &s.url) {
        (Transport::Stdio, None, _) => Some(Translated::Skip("command missing".to_string())),
        (Transport::Http, _, None) => Some(Translated::Skip("url missing".to_string())),
        _ => None,
    }
}

/// The shared `{type, command, args, env}` / `{type, url, headers}` object.
pub(crate) fn passthrough(s: &McpServer) -> Object {
    let strings = |pairs: &[(String, String)]| {
        let mut o = Object::new();
        for (k, v) in pairs {
            o.insert(k.clone(), Value::String(v.clone()));
        }
        Value::Object(o)
    };
    let mut o = Object::new();
    match s.transport {
        Transport::Stdio => {
            o.insert("type", Value::String("stdio".to_string()));
            o.insert(
                "command",
                Value::String(s.command.clone().unwrap_or_default()),
            );
            if !s.args.is_empty() {
                let args = s.args.iter().map(|a| Value::String(a.clone())).collect();
                o.insert("args", Value::Array(args));
            }
            if !s.env.is_empty() {
                o.insert("env", strings(&s.env));
            }
        }
        Transport::Http => {
            o.insert("type", Value::String("http".to_string()));
            o.insert("url", Value::String(s.url.clone().unwrap_or_default()));
            if !s.headers.is_empty() {
                o.insert("headers", strings(&s.headers));
            }
        }
    }
    o
}
