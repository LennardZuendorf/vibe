//! Factory droid: `mcp.json` entries.
use vibe_core::json::Value;

use super::{Translated, passthrough, reject_placeholders, require_target};
use crate::store::mcp::McpServer;

/// Translates one server; `${N}` stays in `env` and `headers`, where droid expands it.
pub fn factory(s: &McpServer) -> Translated<Value> {
    if let Some(skip) = require_target(s) {
        return skip;
    }
    if let Some(skip) = reject_placeholders(s) {
        return skip;
    }
    let mut o = passthrough(s);
    if !s.enabled {
        o.insert("disabled", Value::Bool(true));
    }
    Translated::Ok(Value::Object(o))
}
