//! The projection state: what vibe last wrote and what it owns.
use std::collections::BTreeMap;

use vibe_core::json::{self, Object, Value};

use crate::paths::{Harness, Paths};
use crate::plan::{OwnedEdit, PlanItem};
use crate::store::read_optional;

/// What vibe owns in the harness configs.
#[derive(Clone, Default, PartialEq)]
pub struct Owned {
    /// Claude MCP servers with the canonical JSON vibe added, sorted by name.
    pub mcp_claude: Vec<(String, Value)>,
    /// Owned MCP names per harness id (`codex`, `droid`).
    pub mcp_names: BTreeMap<String, Vec<String>>,
    /// The hook command per harness id.
    pub hooks: BTreeMap<String, String>,
    /// The plugin path per harness id.
    pub plugins: BTreeMap<String, String>,
    /// `(path, target)` of each link vibe made, sorted.
    pub links: Vec<(String, String)>,
    /// Paths vibe created, sorted.
    pub created: Vec<String>,
}

impl std::fmt::Debug for Owned {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let claude: Vec<&str> = self.mcp_claude.iter().map(|(n, _)| n.as_str()).collect();
        f.debug_struct("Owned")
            .field("mcp_claude", &claude)
            .field("mcp_names", &self.mcp_names)
            .field("hooks", &self.hooks)
            .field("plugins", &self.plugins)
            .field("links", &self.links)
            .field("created", &self.created)
            .finish()
    }
}

/// The contents of `state.json`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct State {
    /// Store hash of the last fully successful run.
    pub store_hash: String,
    /// Hash of `mcp.json` at the last fully successful run.
    pub mcp_hash: String,
    /// Target key to the hash vibe last wrote there.
    pub targets: BTreeMap<String, String>,
    /// Ownership records.
    pub owned: Owned,
}

fn corrupt(paths: &Paths, why: &str) -> String {
    format!("{}: {why}", paths.state_file().display())
}

fn strings(v: &Value, what: &str) -> Result<Vec<String>, String> {
    let list = v
        .as_array()
        .ok_or_else(|| format!("\"{what}\" must be an array"))?;
    list.iter()
        .map(|s| {
            s.as_str()
                .map(str::to_string)
                .ok_or_else(|| format!("\"{what}\" must hold strings"))
        })
        .collect()
}

fn string_map(v: &Value, what: &str) -> Result<BTreeMap<String, String>, String> {
    let obj = v
        .as_object()
        .ok_or_else(|| format!("\"{what}\" must be an object"))?;
    obj.iter()
        .map(|(k, v)| {
            v.as_str()
                .map(|s| (k.to_string(), s.to_string()))
                .ok_or_else(|| format!("\"{what}\" must hold strings"))
        })
        .collect()
}

fn text(obj: &Object, key: &str) -> Result<String, String> {
    match obj.get(key) {
        None => Ok(String::new()),
        Some(v) => v
            .as_str()
            .map(str::to_string)
            .ok_or_else(|| format!("\"{key}\" must be a string")),
    }
}

fn parse(root: &Value) -> Result<State, String> {
    let obj = root.as_object().ok_or("expected a JSON object")?;
    match obj.get("v") {
        Some(Value::Number(n)) if n.as_str() == "1" => {}
        _ => return Err("unsupported state version".to_string()),
    }
    let mut state = State {
        store_hash: text(obj, "store_hash")?,
        mcp_hash: text(obj, "mcp_hash")?,
        ..State::default()
    };
    if let Some(t) = obj.get("targets") {
        let t = t.as_object().ok_or("\"targets\" must be an object")?;
        for (k, v) in t.iter() {
            let hash = v
                .path(&["hash"])
                .and_then(Value::as_str)
                .ok_or("\"targets\" entries need a \"hash\" string")?;
            state.targets.insert(k.to_string(), hash.to_string());
        }
    }
    let Some(owned) = obj.get("owned") else {
        return Ok(state);
    };
    let owned = owned.as_object().ok_or("\"owned\" must be an object")?;
    if let Some(mcp) = owned.get("mcp") {
        let mcp = mcp.as_object().ok_or("\"owned.mcp\" must be an object")?;
        for (k, v) in mcp.iter() {
            if k == "claude" {
                let servers = v
                    .as_object()
                    .ok_or("\"owned.mcp.claude\" must be an object")?;
                state.owned.mcp_claude = servers
                    .iter()
                    .map(|(n, c)| (n.to_string(), c.clone()))
                    .collect();
            } else {
                state
                    .owned
                    .mcp_names
                    .insert(k.to_string(), strings(v, "owned.mcp")?);
            }
        }
    }
    if let Some(h) = owned.get("hooks") {
        state.owned.hooks = string_map(h, "owned.hooks")?;
    }
    if let Some(p) = owned.get("plugins") {
        state.owned.plugins = string_map(p, "owned.plugins")?;
    }
    if let Some(l) = owned.get("links") {
        let list = l.as_array().ok_or("\"owned.links\" must be an array")?;
        for e in list {
            let get = |k| e.path(&[k]).and_then(Value::as_str);
            let (Some(path), Some(target)) = (get("path"), get("target")) else {
                return Err("\"owned.links\" entries need \"path\" and \"target\" strings".into());
            };
            state
                .owned
                .links
                .push((path.to_string(), target.to_string()));
        }
    }
    if let Some(c) = owned.get("created") {
        state.owned.created = strings(c, "owned.created")?;
    }
    Ok(state)
}

/// Loads the state. A missing file is the default; invalid JSON, another version or a
/// wrong shape is a refusal line. Unknown fields are ignored.
pub fn load(paths: &Paths) -> Result<State, String> {
    let Some(bytes) = read_optional(&paths.state_file())? else {
        return Ok(State::default());
    };
    let text = String::from_utf8(bytes).map_err(|_| corrupt(paths, "not valid UTF-8"))?;
    let root = json::parse(&text).map_err(|e| corrupt(paths, &e.to_string()))?;
    parse(&root).map_err(|e| corrupt(paths, &e))
}

fn obj(pairs: Vec<(&str, Value)>) -> Value {
    let mut o = Object::new();
    for (k, v) in pairs {
        o.insert(k, v);
    }
    Value::Object(o)
}

fn string_array(list: &[String]) -> Value {
    Value::Array(list.iter().map(|s| Value::str(s)).collect())
}

fn string_object(map: &BTreeMap<String, String>) -> Value {
    obj(map
        .iter()
        .map(|(k, v)| (k.as_str(), Value::str(v)))
        .collect())
}

/// The on-disk JSON form of `state`.
pub fn to_value(state: &State) -> Value {
    let mut mcp = Object::new();
    let mut claude = Object::new();
    for (name, canonical) in &state.owned.mcp_claude {
        claude.insert(name.as_str(), canonical.clone());
    }
    mcp.insert("claude", Value::Object(claude));
    for (id, names) in &state.owned.mcp_names {
        mcp.insert(id.as_str(), string_array(names));
    }
    let links = state
        .owned
        .links
        .iter()
        .map(|(p, t)| obj(vec![("path", Value::str(p)), ("target", Value::str(t))]))
        .collect();
    let targets = obj(state
        .targets
        .iter()
        .map(|(k, h)| (k.as_str(), obj(vec![("hash", Value::str(h))])))
        .collect());
    let owned = obj(vec![
        ("mcp", Value::Object(mcp)),
        ("hooks", string_object(&state.owned.hooks)),
        ("plugins", string_object(&state.owned.plugins)),
        ("links", Value::Array(links)),
        ("created", string_array(&state.owned.created)),
    ]);
    obj(vec![
        ("v", json::parse("1").unwrap_or(Value::Null)),
        ("store_hash", Value::str(&state.store_hash)),
        ("mcp_hash", Value::str(&state.mcp_hash)),
        ("targets", targets),
        ("owned", owned),
    ])
}

/// The plan item that writes `state` to `state.json`. The harness is a placeholder: the
/// state file belongs to no harness.
pub fn save_plan(paths: &Paths, state: &State) -> PlanItem {
    PlanItem::FileWrite {
        harness: Harness::Claude,
        show: "state.json".to_string(),
        path: paths.state_file(),
        bytes: json::to_pretty(&to_value(state)).into_bytes(),
    }
}

fn insert_sorted<T: Ord>(list: &mut Vec<T>, item: T) {
    if let Err(at) = list.binary_search(&item) {
        list.insert(at, item);
    }
}

impl State {
    /// Applies one edit.
    pub fn apply(&mut self, edit: OwnedEdit) {
        match edit {
            OwnedEdit::McpClaude { name, canonical } => {
                self.owned.mcp_claude.retain(|(n, _)| *n != name);
                if let Some(c) = canonical {
                    let at = self.owned.mcp_claude.partition_point(|(n, _)| *n < name);
                    self.owned.mcp_claude.insert(at, (name, c));
                }
            }
            OwnedEdit::McpNames { harness, names } => {
                if names.is_empty() {
                    self.owned.mcp_names.remove(harness.id());
                } else {
                    self.owned.mcp_names.insert(harness.id().to_string(), names);
                }
            }
            OwnedEdit::Hook { harness, command } => {
                set_or_clear(&mut self.owned.hooks, harness, command)
            }
            OwnedEdit::Plugin { harness, path } => {
                set_or_clear(&mut self.owned.plugins, harness, path)
            }
            OwnedEdit::Link {
                path,
                target,
                present,
            } => {
                self.owned.links.retain(|(p, _)| *p != path);
                if present {
                    insert_sorted(&mut self.owned.links, (path, target));
                }
            }
            OwnedEdit::Created { path } => insert_sorted(&mut self.owned.created, path),
            OwnedEdit::Target { key, hash } => match hash {
                Some(h) => {
                    self.targets.insert(key, h);
                }
                None => {
                    self.targets.remove(&key);
                }
            },
        }
    }
}

fn set_or_clear(map: &mut BTreeMap<String, String>, h: Harness, value: Option<String>) {
    match value {
        Some(v) => {
            map.insert(h.id().to_string(), v);
        }
        None => {
            map.remove(h.id());
        }
    }
}
