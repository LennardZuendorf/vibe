//! Codex: `[mcp_servers.<name>]` tables.
use super::{Translated, reject_placeholders, require_target};
use crate::store::mcp::{McpServer, Transport, placeholders};

/// One Codex server table.
#[derive(Clone, Debug, PartialEq)]
pub struct CodexServer {
    /// The table name.
    pub name: String,
    /// The transport-specific fields.
    pub body: CodexBody,
    /// `false` emits `enabled = false`.
    pub enabled: bool,
}

/// The transport-specific part of a Codex table. `Debug` hides every value that can hold a
/// secret: arguments, literal environment values, the URL's userinfo and query, and literal
/// headers.
#[derive(Clone, PartialEq)]
pub enum CodexBody {
    /// A local process.
    Stdio {
        /// The program.
        command: String,
        /// The arguments.
        args: Vec<String>,
        /// Literal environment values.
        env: Vec<(String, String)>,
        /// Variables forwarded from Codex's own environment.
        env_vars: Vec<String>,
    },
    /// A streamable HTTP endpoint.
    Http {
        /// The endpoint.
        url: String,
        /// The variable holding a bearer token.
        bearer_token_env_var: Option<String>,
        /// Literal headers.
        http_headers: Vec<(String, String)>,
        /// Headers whose value is read from a variable.
        env_http_headers: Vec<(String, String)>,
    },
}

fn keys(pairs: &[(String, String)]) -> Vec<(&str, &'static str)> {
    pairs
        .iter()
        .map(|(k, _)| (k.as_str(), "<redacted>"))
        .collect()
}

impl std::fmt::Debug for CodexBody {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CodexBody::Stdio {
                command,
                args,
                env,
                env_vars,
            } => f
                .debug_struct("Stdio")
                .field("command", command)
                .field("args", &vec!["<redacted>"; args.len()])
                .field("env", &keys(env))
                .field("env_vars", env_vars)
                .finish(),
            CodexBody::Http {
                url,
                bearer_token_env_var,
                http_headers,
                env_http_headers,
            } => f
                .debug_struct("Http")
                .field("url", &crate::store::mcp::redact_url(url))
                .field("bearer_token_env_var", bearer_token_env_var)
                .field("http_headers", &keys(http_headers))
                .field("env_http_headers", env_http_headers)
                .finish(),
        }
    }
}

fn exact_placeholder(v: &str) -> Option<&str> {
    match placeholders(v).as_slice() {
        [n] if v.len() == n.len() + 3 => Some(n),
        _ => None,
    }
}

/// Translates one server; placeholders map to variable references or skip the server.
pub fn codex(s: &McpServer) -> Translated<CodexServer> {
    if let Some(skip) = require_target(s) {
        return skip;
    }
    if let Some(skip) = reject_placeholders(s) {
        return skip;
    }
    let body = match s.transport {
        Transport::Stdio => {
            let mut env = Vec::new();
            let mut env_vars = Vec::new();
            for (k, v) in &s.env {
                if exact_placeholder(v) == Some(k.as_str()) {
                    env_vars.push(k.clone());
                } else if placeholders(v).is_empty() {
                    env.push((k.clone(), v.clone()));
                } else {
                    return Translated::Skip(format!("env {k} uses another variable"));
                }
            }
            CodexBody::Stdio {
                command: s.command.clone().unwrap_or_default(),
                args: s.args.clone(),
                env,
                env_vars,
            }
        }
        Transport::Http => {
            let mut bearer = None;
            let mut http_headers = Vec::new();
            let mut env_http_headers = Vec::new();
            for (h, v) in &s.headers {
                let bearer_var = h
                    .eq_ignore_ascii_case("authorization")
                    .then(|| v.strip_prefix("Bearer ").and_then(exact_placeholder))
                    .flatten();
                if let Some(n) = bearer_var {
                    if bearer.replace(n.to_string()).is_some() {
                        return Translated::Skip(format!("header {h} repeated as bearer token"));
                    }
                } else if let Some(n) = exact_placeholder(v) {
                    env_http_headers.push((h.clone(), n.to_string()));
                } else if placeholders(v).is_empty() {
                    http_headers.push((h.clone(), v.clone()));
                } else {
                    return Translated::Skip(format!("header {h} mixes text and a variable"));
                }
            }
            CodexBody::Http {
                url: s.url.clone().unwrap_or_default(),
                bearer_token_env_var: bearer,
                http_headers,
                env_http_headers,
            }
        }
    };
    Translated::Ok(CodexServer {
        name: s.name.clone(),
        body,
        enabled: s.enabled,
    })
}
