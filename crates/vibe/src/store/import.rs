//! `vibe store import-mcp`: copy servers into the store with secrets turned into placeholders.
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use vibe_core::json::{Object, Value};
use vibe_core::lock::{Acquired, Lock, STALE};

use super::mcp::{is_name, is_pure_placeholder, parse_one, placeholders};
use super::read_json;
use crate::paths::Paths;

/// What an import did. Holds variable names, never values.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ImportReport {
    /// Servers written to the store.
    pub added: Vec<String>,
    /// Servers skipped because the store already has the name.
    pub kept_existing: Vec<String>,
    /// Placeholder variable names the user must export, in creation order.
    pub vars: Vec<String>,
    /// Warning lines.
    pub warnings: Vec<String>,
}

const SECRET_WORDS: [&str; 5] = ["key", "token", "secret", "password", "auth"];

/// Whether a field name (case-insensitive substring) marks its value as a likely secret.
pub(crate) fn is_secret_key(key: &str) -> bool {
    let lower = key.to_ascii_lowercase();
    SECRET_WORDS.iter().any(|w| lower.contains(w))
}

fn sanitize(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_uppercase()
            } else {
                '_'
            }
        })
        .collect()
}

/// The placeholder variable for `field` of `server`: `VIBE_<SERVER>_<FIELD>`.
fn var_name(server: &str, field: &str) -> String {
    format!("VIBE_{}_{}", sanitize(server), sanitize(field))
}

struct Rewriter<'a> {
    server: &'a str,
    used: &'a mut BTreeSet<String>,
    foreign: &'a mut BTreeSet<String>,
    vars: &'a mut Vec<String>,
}

impl Rewriter<'_> {
    /// The placeholder for an env value of key `field`: `${field}` itself (Codex forwards the
    /// variable by that name) unless `field` is no valid name or another field already uses
    /// the name for a different purpose.
    fn env_placeholder(&mut self, field: &str) -> String {
        if is_name(field) && !self.foreign.contains(field) {
            self.used.insert(field.to_string());
            if !self.vars.iter().any(|v| v == field) {
                self.vars.push(field.to_string());
            }
            return format!("${{{field}}}");
        }
        self.placeholder(field)
    }

    fn placeholder(&mut self, field: &str) -> String {
        let base = var_name(self.server, field);
        let mut name = base.clone();
        let mut n = 2;
        while self.used.contains(&name) {
            name = format!("{base}_{n}");
            n += 1;
        }
        self.used.insert(name.clone());
        self.foreign.insert(name.clone());
        self.vars.push(name.clone());
        format!("${{{name}}}")
    }

    fn map_values(&mut self, obj: &mut Object, all: bool) {
        let keys: Vec<String> = obj.keys().map(str::to_string).collect();
        for key in keys {
            let Some(Value::String(value)) = obj.get(&key) else {
                continue;
            };
            if is_pure_placeholder(value) || !(all || is_secret_key(&key)) {
                continue;
            }
            let replacement = if all {
                self.placeholder(&key)
            } else {
                self.env_placeholder(&key)
            };
            obj.insert(key, Value::String(replacement));
        }
    }

    fn url(&mut self, url: &str) -> String {
        let (main, fragment) = match url.split_once('#') {
            Some((m, f)) => (m, Some(f)),
            None => (url, None),
        };
        let Some((base, query)) = main.split_once('?') else {
            return url.to_string();
        };
        let pairs: Vec<String> = query
            .split('&')
            .map(|pair| match pair.split_once('=') {
                Some((k, v)) if is_secret_key(k) && !v.is_empty() && !is_pure_placeholder(v) => {
                    format!("{k}={}", self.placeholder(k))
                }
                _ => pair.to_string(),
            })
            .collect();
        let mut out = format!("{base}?{}", pairs.join("&"));
        if let Some(f) = fragment {
            out.push('#');
            out.push_str(f);
        }
        out
    }
}

fn refuse_symlink(path: &Path) -> Result<(), String> {
    match fs::symlink_metadata(path) {
        Ok(m) if m.file_type().is_symlink() => Err(format!(
            "{}: is a symlink; refusing to write through it",
            path.display()
        )),
        _ => Ok(()),
    }
}

fn collect_placeholders(v: &Value, out: &mut BTreeSet<String>) {
    match v {
        Value::String(s) => out.extend(placeholders(s).into_iter().map(str::to_string)),
        Value::Array(items) => items.iter().for_each(|i| collect_placeholders(i, out)),
        Value::Object(o) => o.iter().for_each(|(_, v)| collect_placeholders(v, out)),
        _ => {}
    }
}

/// Collects the placeholder names that stand for something other than "the env value of the
/// key of the same name": every placeholder outside `env`, and env values whose name differs
/// from their key.
fn collect_foreign(doc: &Value, out: &mut BTreeSet<String>) {
    let Some(servers) = doc.path(&["mcpServers"]).and_then(Value::as_object) else {
        return;
    };
    for (_, server) in servers.iter() {
        let Some(obj) = server.as_object() else {
            continue;
        };
        for (field, value) in obj.iter() {
            match (field, value.as_object()) {
                ("env", Some(env)) => {
                    for (key, v) in env.iter() {
                        if let Some(text) = v.as_str() {
                            let same = text.strip_prefix("${").and_then(|t| t.strip_suffix('}'));
                            if same != Some(key) {
                                out.extend(placeholders(text).into_iter().map(str::to_string));
                            }
                        }
                    }
                }
                _ => collect_placeholders(value, out),
            }
        }
    }
}

fn is_secret_flag(arg: &str) -> Option<bool> {
    if !arg.starts_with('-') {
        return None;
    }
    let lower = arg.trim_start_matches('-').to_ascii_lowercase();
    let flag = lower.split('=').next().unwrap_or_default();
    let named = [
        "apikey", "api-key", "api_key", "token", "secret", "password",
    ]
    .iter()
    .any(|w| flag.starts_with(w));
    named.then_some(arg.contains('='))
}

/// Names of the places where a secret-looking value survived the rewrite.
fn leftover_secrets(server: &Object) -> Vec<&'static str> {
    let mut out = Vec::new();
    if let Some(args) = server.get("args").and_then(Value::as_array) {
        let strs: Vec<&str> = args.iter().filter_map(Value::as_str).collect();
        let leaked = strs
            .iter()
            .enumerate()
            .any(|(i, a)| match is_secret_flag(a) {
                Some(true) => a
                    .split_once('=')
                    .is_some_and(|(_, v)| !v.is_empty() && !is_pure_placeholder(v)),
                Some(false) => strs.get(i + 1).is_some_and(|n| !is_pure_placeholder(n)),
                None => false,
            });
        if leaked {
            out.push("args");
        }
    }
    if let Some(url) = server.get("url").and_then(Value::as_str) {
        let (main, fragment) = match url.split_once('#') {
            Some((m, f)) => (m, Some(f)),
            None => (url, None),
        };
        let (base, query) = main.split_once('?').unwrap_or((main, ""));
        if let Some((_, rest)) = base.split_once("://") {
            let authority = rest.split('/').next().unwrap_or_default();
            if let Some((userinfo, _)) = authority.rsplit_once('@')
                && !userinfo.contains("${")
            {
                out.push("url userinfo");
            }
        }
        let leaks = |text: &str| {
            text.split(['&', ';']).any(|pair| {
                matches!(pair.split_once('='), Some((k, v))
                    if is_secret_key(k) && !v.is_empty() && !is_pure_placeholder(v))
            })
        };
        let semi = query
            .split('&')
            .any(|piece| piece.split(';').skip(1).any(leaks));
        if semi {
            out.push("url query");
        }
        if fragment.is_some_and(leaks) {
            out.push("url fragment");
        }
    }
    out
}

/// Imports the servers of `file` (a `{"mcpServers": {...}}` document) into the store's
/// `mcp.json`, under the store lock. Servers already in the store are kept as they are;
/// the file is rewritten as 2-space JSON (vibe owns it), keeping key order and number
/// text. Unless `keep_literals`, header values, secret-looking env values and
/// secret-looking URL query values become `${VIBE_<SERVER>_<FIELD>}`, except a secret env value of key `K` (a valid name) that becomes `${K}` unless another field already uses `${K}` for a different purpose; new names never
/// reuse a `${NAME}` already in the store or the input. A server of invalid shape is
/// skipped with a warning. Secret-looking values the rewrite cannot replace (URL
/// userinfo or fragment, `;` query pairs, secret flags in `args`) produce a warning.
/// The `Err` is one refusal line and never contains a value.
pub fn import(file: &Path, paths: &Paths, keep_literals: bool) -> Result<ImportReport, String> {
    let doc = read_json(file)?.ok_or_else(|| format!("{}: no such file", file.display()))?;
    let servers = doc
        .path(&["mcpServers"])
        .and_then(Value::as_object)
        .ok_or_else(|| format!("{}: expected an object \"mcpServers\"", file.display()))?;

    let _lock = match Lock::acquire(&paths.lock_file(), STALE).map_err(|e| e.to_string())? {
        Acquired::Got(lock) => lock,
        Acquired::Busy => return Err("store busy; retry".to_string()),
    };
    let target = paths.store_mcp();
    refuse_symlink(&target)?;
    let mut store_doc = read_json(&target)?.unwrap_or_else(|| Value::Object(Object::new()));
    let mut used = BTreeSet::new();
    collect_placeholders(&store_doc, &mut used);
    collect_placeholders(&doc, &mut used);
    let mut foreign = BTreeSet::new();
    collect_foreign(&store_doc, &mut foreign);
    collect_foreign(&doc, &mut foreign);
    let store_obj = store_doc
        .as_object_mut()
        .ok_or_else(|| format!("{}: expected a JSON object", target.display()))?;
    if store_obj.get("mcpServers").is_none() {
        store_obj.insert("mcpServers", Value::Object(Object::new()));
    }
    let existing = store_obj
        .get_mut("mcpServers")
        .and_then(Value::as_object_mut)
        .ok_or_else(|| format!("{}: \"mcpServers\" must be an object", target.display()))?;

    let mut report = ImportReport::default();
    if keep_literals {
        report
            .warnings
            .push("keeping literal values (--keep-literals)".to_string());
    }
    for (name, value) in servers.iter() {
        if existing.get(name).is_some() {
            report.kept_existing.push(name.to_string());
            continue;
        }
        if let Err(cause) = parse_one(name, value) {
            report.warnings.push(format!("skipped {cause}"));
            continue;
        }
        let mut server = value.clone();
        if let Some(obj) = server.as_object_mut()
            && !keep_literals
        {
            let mut rw = Rewriter {
                server: name,
                used: &mut used,
                foreign: &mut foreign,
                vars: &mut report.vars,
            };
            if let Some(Value::Object(headers)) = obj.get_mut("headers") {
                rw.map_values(headers, true);
            }
            if let Some(Value::Object(env)) = obj.get_mut("env") {
                rw.map_values(env, false);
            }
            if let Some(Value::String(url)) = obj.get("url") {
                let rewritten = rw.url(url);
                obj.insert("url", Value::String(rewritten));
            }
            for place in leftover_secrets(obj) {
                report.warnings.push(format!(
                        "{name}: possible secret in {place} left unchanged \u{2014} edit it to a ${{VAR}}"
                    ));
            }
        }
        existing.insert(name, server);
        report.added.push(name.to_string());
    }

    if !report.added.is_empty() {
        fs::create_dir_all(&paths.store).map_err(|e| format!("{}: {e}", paths.store.display()))?;
        vibe_core::fs::write_atomic(&target, vibe_core::json::to_pretty(&store_doc).as_bytes())
            .map_err(|e| e.to_string())?;
    }
    Ok(report)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    #[test]
    fn secret_keys_match_case_insensitively() {
        for k in [
            "API_KEY",
            "token",
            "MySecret",
            "PASSWORD",
            "Authorization",
            "exaApiKey",
        ] {
            assert!(is_secret_key(k), "{k}");
        }
        for k in ["PATH_X", "LOG_LEVEL", "tools", "mode"] {
            assert!(!is_secret_key(k), "{k}");
        }
    }

    #[test]
    fn var_name_sanitizes_both_parts() {
        assert_eq!(var_name("exa", "API_KEY"), "VIBE_EXA_API_KEY");
        assert_eq!(var_name("my.srv-1", "X-Api key"), "VIBE_MY_SRV_1_X_API_KEY");
        assert_eq!(var_name("é", "k"), "VIBE___K");
    }

    #[test]
    fn url_keeps_fragment_and_non_secret_pairs() {
        let mut used = BTreeSet::new();
        let mut foreign = BTreeSet::new();
        let mut vars = Vec::new();
        let mut rw = Rewriter {
            server: "s",
            used: &mut used,
            foreign: &mut foreign,
            vars: &mut vars,
        };
        assert_eq!(
            rw.url("https://h/p?a=1&token=zz&flag&auth=${X}&key=#f"),
            "https://h/p?a=1&token=${VIBE_S_TOKEN}&flag&auth=${X}&key=#f"
        );
        assert_eq!(rw.url("https://h/p"), "https://h/p");
    }
}
