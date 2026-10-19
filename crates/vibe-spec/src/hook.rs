//! The Claude Code hook payload: the JSON a hook reads on stdin.
use std::path::PathBuf;

use serde_json::Value;

/// The fields every spec hook needs from the payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Payload {
    /// `hook_event_name`.
    pub event: String,
    /// `cwd`.
    pub cwd: PathBuf,
    /// `transcript_path`: for a subagent this is the main session's transcript.
    pub transcript: Option<PathBuf>,
    /// `agent_id`, present only for a subagent's tool calls.
    pub agent_id: Option<String>,
    /// `tool_name`.
    pub tool: Option<String>,
    /// `tool_input.file_path`, else `tool_input.notebook_path`.
    pub file: Option<String>,
    /// `tool_input.command`.
    pub command: Option<String>,
}

fn string(v: &Value, key: &str) -> Option<String> {
    v.get(key).and_then(Value::as_str).map(str::to_string)
}

/// Parses a hook payload; an error names the first problem.
pub fn parse(stdin: &[u8]) -> Result<Payload, String> {
    let v: Value = serde_json::from_slice(stdin).map_err(|e| format!("malformed payload: {e}"))?;
    if !v.is_object() {
        return Err("malformed payload: not a JSON object".to_string());
    }
    let event = string(&v, "hook_event_name").ok_or("payload has no hook_event_name")?;
    let cwd = string(&v, "cwd").ok_or("payload has no cwd")?;
    let input = v.get("tool_input").unwrap_or(&Value::Null);
    Ok(Payload {
        event,
        cwd: PathBuf::from(cwd),
        transcript: string(&v, "transcript_path").map(PathBuf::from),
        agent_id: string(&v, "agent_id"),
        tool: string(&v, "tool_name"),
        file: string(input, "file_path").or_else(|| string(input, "notebook_path")),
        command: string(input, "command"),
    })
}
