//! A minimal TOML emitter and table scanner for Codex's `config.toml`.
use std::collections::BTreeSet;

use super::codex::{CodexBody, CodexServer};

/// A TOML basic string: quoted, with escapes.
pub fn basic_string(s: &str) -> String {
    let mut o = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            '\n' => o.push_str("\\n"),
            '\r' => o.push_str("\\r"),
            '\t' => o.push_str("\\t"),
            '\u{8}' => o.push_str("\\b"),
            '\u{c}' => o.push_str("\\f"),
            c if (c as u32) < 0x20 || c == '\u{7f}' => {
                o.push_str(&format!("\\u{:04X}", c as u32));
            }
            c => o.push(c),
        }
    }
    o.push('"');
    o
}

fn is_bare(s: &str) -> bool {
    !s.is_empty()
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

/// A TOML key: bare when it only holds `[A-Za-z0-9_-]`, else a basic string.
pub fn key(s: &str) -> String {
    if is_bare(s) {
        s.to_string()
    } else {
        basic_string(s)
    }
}

fn inline_table(pairs: &[(String, String)]) -> String {
    let items: Vec<String> = pairs
        .iter()
        .map(|(k, v)| format!("{} = {}", key(k), basic_string(v)))
        .collect();
    format!("{{ {} }}", items.join(", "))
}

fn string_array(items: &[String]) -> String {
    let items: Vec<String> = items.iter().map(|i| basic_string(i)).collect();
    format!("[{}]", items.join(", "))
}

fn table(s: &CodexServer) -> String {
    let mut out = format!("[mcp_servers.{}]\n", key(&s.name));
    match &s.body {
        CodexBody::Stdio {
            command,
            args,
            env,
            env_vars,
        } => {
            out.push_str(&format!("command = {}\n", basic_string(command)));
            if !args.is_empty() {
                out.push_str(&format!("args = {}\n", string_array(args)));
            }
            if !env.is_empty() {
                out.push_str(&format!("env = {}\n", inline_table(env)));
            }
            if !env_vars.is_empty() {
                out.push_str(&format!("env_vars = {}\n", string_array(env_vars)));
            }
        }
        CodexBody::Http {
            url,
            bearer_token_env_var,
            http_headers,
            env_http_headers,
        } => {
            out.push_str(&format!("url = {}\n", basic_string(url)));
            if let Some(v) = bearer_token_env_var {
                out.push_str(&format!("bearer_token_env_var = {}\n", basic_string(v)));
            }
            if !http_headers.is_empty() {
                out.push_str(&format!("http_headers = {}\n", inline_table(http_headers)));
            }
            if !env_http_headers.is_empty() {
                out.push_str(&format!(
                    "env_http_headers = {}\n",
                    inline_table(env_http_headers)
                ));
            }
        }
    }
    if !s.enabled {
        out.push_str("enabled = false\n");
    }
    out
}

/// The text between the markers: one table per server, blank-line separated.
pub fn codex_tables(servers: &[CodexServer]) -> String {
    let tables: Vec<String> = servers.iter().map(table).collect();
    tables.join("\n")
}

/// Parses one key part (bare, basic or literal string) at `i`.
fn key_part(c: &[char], i: &mut usize) -> Option<String> {
    match c.get(*i)? {
        '"' => {
            *i += 1;
            let mut out = String::new();
            loop {
                let ch = *c.get(*i)?;
                *i += 1;
                match ch {
                    '"' => return Some(out),
                    '\\' => {
                        let e = *c.get(*i)?;
                        *i += 1;
                        match e {
                            'n' => out.push('\n'),
                            't' => out.push('\t'),
                            'r' => out.push('\r'),
                            'b' => out.push('\u{8}'),
                            'f' => out.push('\u{c}'),
                            'u' | 'U' => {
                                let n = if e == 'u' { 4 } else { 8 };
                                let hex: String = c.get(*i..*i + n)?.iter().collect();
                                *i += n;
                                out.push(char::from_u32(u32::from_str_radix(&hex, 16).ok()?)?);
                            }
                            other => out.push(other),
                        }
                    }
                    ch => out.push(ch),
                }
            }
        }
        '\'' => {
            *i += 1;
            let start = *i;
            while *c.get(*i)? != '\'' {
                *i += 1;
            }
            let out: String = c[start..*i].iter().collect();
            *i += 1;
            Some(out)
        }
        _ => {
            let start = *i;
            while c
                .get(*i)
                .is_some_and(|ch| ch.is_ascii_alphanumeric() || *ch == '_' || *ch == '-')
            {
                *i += 1;
            }
            (*i > start).then(|| c[start..*i].iter().collect())
        }
    }
}

fn skip_ws(c: &[char], i: &mut usize) {
    while c.get(*i).is_some_and(|ch| *ch == ' ' || *ch == '\t') {
        *i += 1;
    }
}

/// Parses a dotted key at `i`; returns the parts parsed so far when a part fails.
fn dotted_key(c: &[char], i: &mut usize) -> Vec<String> {
    let mut parts = Vec::new();
    loop {
        skip_ws(c, i);
        let Some(p) = key_part(c, i) else {
            return parts;
        };
        parts.push(p);
        skip_ws(c, i);
        if c.get(*i) == Some(&'.') {
            *i += 1;
        } else {
            return parts;
        }
    }
}

/// What `scan_mcp_tables` found.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct McpScan {
    /// Server names that already own a table, array of tables or dotted key.
    pub names: BTreeSet<String>,
    /// `mcp_servers` itself is assigned (`mcp_servers = { ... }`). TOML forbids extending
    /// an inline table, so the caller must not emit any `[mcp_servers.*]` table.
    pub root_inline: bool,
}

/// Multi-line string state: the closing delimiter and whether it honours escapes.
type OpenString = Option<(char, bool)>;

/// Advances over `c` from `i`, tracking multi-line strings, until a comment or the end.
fn advance(c: &[char], mut i: usize, state: &mut OpenString) {
    while i < c.len() {
        if let Some((q, escapes)) = *state {
            if escapes && c[i] == '\\' {
                i += 2;
            } else if c[i] == q && c.get(i + 1) == Some(&q) && c.get(i + 2) == Some(&q) {
                *state = None;
                i += 3;
            } else {
                i += 1;
            }
            continue;
        }
        match c[i] {
            '#' => return,
            q @ ('"' | '\'') => {
                if c.get(i + 1) == Some(&q) && c.get(i + 2) == Some(&q) {
                    *state = Some((q, q == '"'));
                    i += 3;
                    continue;
                }
                i += 1;
                while i < c.len() && c[i] != q {
                    i += if q == '"' && c[i] == '\\' { 2 } else { 1 };
                }
                i += 1;
            }
            _ => i += 1,
        }
    }
}

/// Scans `text` (a `config.toml` without the vibe block) for existing `mcp_servers` tables.
/// Line based, aware of comments and multi-line strings, and tolerant of a BOM and CRLF.
pub fn scan_mcp_tables(text: &str) -> McpScan {
    let mut scan = McpScan::default();
    let mut header: Vec<String> = Vec::new();
    let mut open: OpenString = None;
    for line in text.strip_prefix('\u{feff}').unwrap_or(text).lines() {
        let chars: Vec<char> = line.trim_start().chars().collect();
        if open.is_some() {
            advance(&chars, 0, &mut open);
            continue;
        }
        if chars.is_empty() || chars[0] == '#' {
            continue;
        }
        let mut i = 0;
        let full: Vec<String> = if chars[0] == '[' {
            i += if chars.get(1) == Some(&'[') { 2 } else { 1 };
            header = dotted_key(&chars, &mut i);
            header.clone()
        } else {
            let parts = dotted_key(&chars, &mut i);
            skip_ws(&chars, &mut i);
            if chars.get(i) != Some(&'=') {
                advance(&chars, 0, &mut open);
                continue;
            }
            header.iter().cloned().chain(parts).collect()
        };
        match full.as_slice() {
            [root, name, ..] if root == "mcp_servers" => {
                scan.names.insert(name.clone());
            }
            [root] if root == "mcp_servers" && chars[0] != '[' => scan.root_inline = true,
            _ => {}
        }
        advance(&chars, 0, &mut open);
    }
    scan
}
