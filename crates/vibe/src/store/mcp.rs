//! `mcp.json`: the canonical MCP server list.
use vibe_core::json::{Object, Value};

use crate::paths::Harness;

/// How a server is reached.
#[derive(Clone, Debug, PartialEq)]
pub enum Transport {
    /// A local process.
    Stdio,
    /// A remote HTTP endpoint.
    Http,
}

/// One server of the store. Unknown JSON fields are not kept.
/// `Debug` shows `env` and `headers` keys but never their values, and hides `args` and the
/// userinfo, query values and fragment of `url`.
#[derive(Clone, PartialEq)]
pub struct McpServer {
    /// The key in `mcpServers`.
    pub name: String,
    /// `stdio` (default) or `http`.
    pub transport: Transport,
    /// The program to run (stdio).
    pub command: Option<String>,
    /// The program arguments.
    pub args: Vec<String>,
    /// Environment variables in file order.
    pub env: Vec<(String, String)>,
    /// The endpoint (http).
    pub url: Option<String>,
    /// Request headers in file order.
    pub headers: Vec<(String, String)>,
    /// `false` marks the server disabled; default true.
    pub enabled: bool,
    /// Allow-list of harnesses; `None` means every harness.
    pub harnesses: Option<Vec<Harness>>,
}

/// `url` with its userinfo, query values and fragment hidden, for `Debug` output.
pub(crate) fn redact_url(url: &str) -> String {
    let (main, fragment) = match url.split_once('#') {
        Some((m, _)) => (m, true),
        None => (url, false),
    };
    let (base, query) = match main.split_once('?') {
        Some((b, q)) => (b, Some(q)),
        None => (main, None),
    };
    let mut out = match base.split_once("://") {
        Some((scheme, rest)) => {
            let (authority, path) = rest.split_at(rest.find('/').unwrap_or(rest.len()));
            let host = authority.rsplit_once('@').map_or(authority, |(_, h)| h);
            let userinfo = if host.len() == authority.len() {
                ""
            } else {
                "<redacted>@"
            };
            format!("{scheme}://{userinfo}{host}{path}")
        }
        None => base.to_string(),
    };
    if let Some(q) = query {
        let keys: Vec<String> = q
            .split('&')
            .map(|p| match p.split_once('=') {
                Some((k, _)) => format!("{k}=<redacted>"),
                None => p.to_string(),
            })
            .collect();
        out.push('?');
        out.push_str(&keys.join("&"));
    }
    if fragment {
        out.push_str("#<redacted>");
    }
    out
}

impl std::fmt::Debug for McpServer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let redact = |pairs: &[(String, String)]| -> Vec<(String, &'static str)> {
            pairs
                .iter()
                .map(|(k, _)| (k.clone(), "<redacted>"))
                .collect()
        };
        f.debug_struct("McpServer")
            .field("name", &self.name)
            .field("transport", &self.transport)
            .field("command", &self.command)
            .field("args", &vec!["<redacted>"; self.args.len()])
            .field("env", &redact(&self.env))
            .field("url", &self.url.as_deref().map(redact_url))
            .field("headers", &redact(&self.headers))
            .field("enabled", &self.enabled)
            .field("harnesses", &self.harnesses)
            .finish()
    }
}

impl McpServer {
    /// Whether the server's `harnesses` allow-list admits `h`. Ignores `enabled`.
    pub fn applies_to(&self, h: Harness) -> bool {
        self.harnesses.as_ref().is_none_or(|list| list.contains(&h))
    }
}

/// Names of every `${NAME}` in `s`, NAME matching `[A-Z_][A-Z0-9_]*`.
pub fn placeholders(s: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut rest = s;
    while let Some(at) = rest.find("${") {
        let after = &rest[at + 2..];
        match after.find('}') {
            Some(end) if is_name(&after[..end]) => {
                out.push(&after[..end]);
                rest = &after[end + 1..];
            }
            _ => rest = after,
        }
    }
    out
}

/// Whether `s` is a valid placeholder name.
pub(crate) fn is_name(s: &str) -> bool {
    let mut chars = s.chars();
    matches!(chars.next(), Some(c) if c == '_' || c.is_ascii_uppercase())
        && chars.all(|c| c == '_' || c.is_ascii_uppercase() || c.is_ascii_digit())
}

/// Whether `s` consists of placeholders only (at least one, nothing between).
pub(crate) fn is_pure_placeholder(s: &str) -> bool {
    let mut rest = s;
    if rest.is_empty() {
        return false;
    }
    while let Some(after) = rest.strip_prefix("${") {
        match after.find('}') {
            Some(end) if is_name(&after[..end]) => rest = &after[end + 1..],
            _ => return false,
        }
    }
    rest.is_empty()
}

fn string_pairs(v: &Value, what: &str) -> Result<Vec<(String, String)>, String> {
    let obj = v
        .as_object()
        .ok_or_else(|| format!("\"{what}\" must be an object of strings"))?;
    obj.iter()
        .map(|(k, v)| {
            v.as_str()
                .map(|s| (k.to_string(), s.to_string()))
                .ok_or_else(|| format!("\"{what}.{k}\" must be a string"))
        })
        .collect()
}

fn optional_string(obj: &Object, key: &str) -> Result<Option<String>, String> {
    match obj.get(key) {
        None => Ok(None),
        Some(v) => v
            .as_str()
            .map(|s| Some(s.to_string()))
            .ok_or_else(|| format!("\"{key}\" must be a string")),
    }
}

fn parse_server(name: &str, v: &Value) -> Result<McpServer, String> {
    let obj = v.as_object().ok_or("must be an object")?;
    let transport = match obj.get("type") {
        None => Transport::Stdio,
        Some(v) => match v.as_str() {
            Some("stdio") => Transport::Stdio,
            Some("http") => Transport::Http,
            Some(_) => return Err("unsupported type (allowed: stdio, http)".to_string()),
            None => return Err("\"type\" must be a string".to_string()),
        },
    };
    let args = match obj.get("args") {
        None => Vec::new(),
        Some(v) => v
            .as_array()
            .ok_or("\"args\" must be an array of strings")?
            .iter()
            .map(|a| a.as_str().map(str::to_string))
            .collect::<Option<Vec<_>>>()
            .ok_or("\"args\" must be an array of strings")?,
    };
    let env = obj
        .get("env")
        .map(|v| string_pairs(v, "env"))
        .transpose()?
        .unwrap_or_default();
    let headers = obj
        .get("headers")
        .map(|v| string_pairs(v, "headers"))
        .transpose()?
        .unwrap_or_default();
    let enabled = match obj.get("enabled") {
        None => true,
        Some(v) => v.as_bool().ok_or("\"enabled\" must be true or false")?,
    };
    let harnesses = match obj.get("harnesses") {
        None => None,
        Some(v) => {
            let list = v.as_array().ok_or("\"harnesses\" must be an array")?;
            let mut out = Vec::new();
            for item in list {
                let id = item.as_str().ok_or("\"harnesses\" must hold strings")?;
                out.push(Harness::parse(id).ok_or("unknown harness id in \"harnesses\"")?);
            }
            Some(out)
        }
    };
    Ok(McpServer {
        name: name.to_string(),
        transport,
        command: optional_string(obj, "command")?,
        args,
        env,
        url: optional_string(obj, "url")?,
        headers,
        enabled,
        harnesses,
    })
}

/// Parses one server; the `Err` names the server and the field.
pub fn parse_one(name: &str, v: &Value) -> Result<McpServer, String> {
    parse_server(name, v).map_err(|e| format!("server \"{name}\": {e}"))
}

/// Parses the `mcpServers` object of a document; absent means no servers.
pub fn parse_servers(root: &Value) -> Result<Vec<McpServer>, String> {
    let obj = root.as_object().ok_or("expected a JSON object")?;
    let Some(servers) = obj.get("mcpServers") else {
        return Ok(Vec::new());
    };
    let servers = servers
        .as_object()
        .ok_or("\"mcpServers\" must be an object")?;
    servers.iter().map(|(n, v)| parse_one(n, v)).collect()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    #[test]
    fn pure_placeholder_means_placeholders_only() {
        assert!(is_pure_placeholder("${A}"));
        assert!(is_pure_placeholder("${A}${B_1}"));
        assert!(!is_pure_placeholder("Bearer ${A}"));
        assert!(!is_pure_placeholder("${a}"));
        assert!(!is_pure_placeholder("${A}x"));
        assert!(!is_pure_placeholder(""));
    }

    #[test]
    fn defaults_apply() {
        let v = vibe_core::json::parse(r#"{"command":"x"}"#).unwrap();
        let s = parse_one("n", &v).unwrap();
        assert_eq!(s.transport, Transport::Stdio);
        assert!(s.enabled && s.harnesses.is_none() && s.args.is_empty());
    }
}

#[cfg(test)]
mod redact_tests {
    use super::redact_url;

    #[test]
    fn redact_url_hides_userinfo_query_values_and_fragment() {
        assert_eq!(
            redact_url("https://u:p@h:8/p?a=1&token=S&flag#frag"),
            "https://<redacted>@h:8/p?a=<redacted>&token=<redacted>&flag#<redacted>"
        );
        assert_eq!(redact_url("https://h/p"), "https://h/p");
    }
}
