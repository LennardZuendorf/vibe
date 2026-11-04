//! Claude Code: `mcpServers` entries for `claude mcp add-json`.
use vibe_core::json::Value;

use super::{Translated, passthrough, require_target};
use crate::store::mcp::McpServer;

/// Translates one server; `${N}` is kept because Claude expands it at runtime.
pub fn claude(s: &McpServer) -> Translated<Value> {
    if !s.enabled {
        return Translated::Skip("disabled".to_string());
    }
    let valid = !s.name.is_empty()
        && s.name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-');
    if !valid {
        return Translated::Skip("name not accepted by claude".to_string());
    }
    if let Some(skip) = require_target(s) {
        return skip;
    }
    Translated::Ok(Value::Object(passthrough(s)))
}
