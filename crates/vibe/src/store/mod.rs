//! The global store: `store.json`, `AGENTS.md`, `mcp.json` and `skills/`.
pub mod config;
pub mod import;
pub mod init;
pub mod mcp;
pub mod skills;

use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use vibe_core::json::{self, Value};
use vibe_core::markers::hash_bytes;

use crate::paths::Paths;
pub use config::StoreConfig;
pub use mcp::{McpServer, Transport};
pub use skills::{Shadowed, Skill};

/// File bytes that `Debug` shows only as lengths, so a debug dump never leaks secrets.
#[derive(Clone)]
struct RawBytes([Vec<u8>; 3]);

impl std::fmt::Debug for RawBytes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let lens: Vec<usize> = self.0.iter().map(Vec::len).collect();
        write!(f, "<{lens:?} bytes redacted>")
    }
}

/// A loaded, validated store.
#[derive(Clone, Debug)]
pub struct Store {
    /// The parsed `store.json`.
    pub config: StoreConfig,
    /// The root instructions; `None` when missing, empty or not UTF-8.
    pub instructions: Option<String>,
    /// The servers of `mcp.json`, in file order.
    pub servers: Vec<McpServer>,
    /// The skills found in every source.
    pub skills: Vec<Skill>,
    /// Skills dropped because an earlier source holds the same name.
    pub shadowed: Vec<Shadowed>,
    /// Non-fatal observations, one line each.
    pub notes: Vec<String>,
    /// The bytes read for `store.json`, `AGENTS.md` and `mcp.json` (absent is empty);
    /// [`store_hash`] hashes these, not a second read.
    raw: RawBytes,
}

impl Store {
    /// A store with no configuration, instructions, servers or skills; for commands that work
    /// from the recorded state alone.
    pub fn empty() -> Store {
        Store {
            config: StoreConfig::default(),
            instructions: None,
            servers: Vec::new(),
            skills: Vec::new(),
            shadowed: Vec::new(),
            notes: Vec::new(),
            raw: RawBytes([Vec::new(), Vec::new(), Vec::new()]),
        }
    }
}

/// Reads `path`; `None` when it does not exist.
pub(crate) fn read_optional(path: &Path) -> Result<Option<Vec<u8>>, String> {
    match fs::read(path) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(None),
        Err(e) => Err(format!("{}: {e}", path.display())),
    }
}

/// Parses the bytes of the JSON document at `path`.
fn parse_bytes(path: &Path, bytes: Vec<u8>) -> Result<Value, String> {
    let text =
        String::from_utf8(bytes).map_err(|_| format!("{}: not valid UTF-8", path.display()))?;
    json::parse(&text).map_err(|e| format!("{}: {e}", path.display()))
}

/// Reads and parses a JSON document of the store; `None` when the file is missing.
pub(crate) fn read_json(path: &Path) -> Result<Option<Value>, String> {
    read_optional(path)?
        .map(|bytes| parse_bytes(path, bytes))
        .transpose()
}

/// Loads the store. `Err` is one refusal line: `store.json` or `mcp.json` is not valid
/// JSON of the expected shape. Missing files are fine.
pub fn load(paths: &Paths) -> Result<Store, String> {
    let store_json = paths.store_json();
    let store_bytes = read_optional(&store_json)?;
    let config = match store_bytes
        .clone()
        .map(|b| parse_bytes(&store_json, b))
        .transpose()?
    {
        Some(v) => {
            config::parse(&v, paths).map_err(|e| format!("{}: {e}", store_json.display()))?
        }
        None => StoreConfig::default(),
    };
    let mcp_json = paths.store_mcp();
    let mcp_bytes = read_optional(&mcp_json)?;
    let servers = match mcp_bytes
        .clone()
        .map(|b| parse_bytes(&mcp_json, b))
        .transpose()?
    {
        Some(v) => mcp::parse_servers(&v).map_err(|e| format!("{}: {e}", mcp_json.display()))?,
        None => Vec::new(),
    };
    let mut notes = Vec::new();
    for server in servers.iter().filter(|s| s.enabled) {
        let literal = |v: &str| !v.is_empty() && mcp::placeholders(v).is_empty();
        let leaks = server.headers.iter().any(|(_, v)| literal(v))
            || server
                .env
                .iter()
                .any(|(k, v)| import::is_secret_key(k) && literal(v));
        if leaks {
            notes.push(format!(
                "{}: literal secret-looking value projected \u{2014} prefer ${{VAR}}",
                server.name
            ));
        }
    }
    let agents = paths.store_agents();
    let agents_bytes = read_optional(&agents)?;
    let instructions = match agents_bytes.clone() {
        None => None,
        Some(bytes) => match String::from_utf8(bytes) {
            Err(_) => {
                notes.push(format!(
                    "{} is not valid UTF-8; instructions skipped",
                    agents.display()
                ));
                None
            }
            Ok(text) if text.trim().is_empty() => {
                notes.push(format!(
                    "{} is empty; instructions skipped",
                    agents.display()
                ));
                None
            }
            Ok(text) => Some(text),
        },
    };
    let mut sources: Vec<PathBuf> = vec![paths.store_skills()];
    sources.extend(config.skill_sources.iter().cloned());
    let (skills, shadowed) = skills::discover(&sources);
    Ok(Store {
        config,
        instructions,
        servers,
        skills,
        shadowed,
        notes,
        raw: RawBytes([
            store_bytes.unwrap_or_default(),
            agents_bytes.unwrap_or_default(),
            mcp_bytes.unwrap_or_default(),
        ]),
    })
}

/// The cache key of everything the store feeds into a projection: the bytes of
/// `store.json`, `AGENTS.md` and `mcp.json` (absent counts as empty), the sorted
/// `(name, source dir)` skill list and the binary version. The file bytes are the ones
/// `load` read, so the hash always describes the loaded store. `paths` is kept for callers.
pub fn store_hash(_paths: &Paths, store: &Store) -> String {
    let mut buf: Vec<u8> = Vec::new();
    for bytes in &store.raw.0 {
        buf.extend_from_slice(format!("{}:", bytes.len()).as_bytes());
        buf.extend_from_slice(bytes);
        buf.push(b'\n');
    }
    let mut list: Vec<(&str, &Path)> = store
        .skills
        .iter()
        .map(|s| (s.name.as_str(), s.source.as_path()))
        .collect();
    list.sort();
    for (name, source) in list {
        buf.extend_from_slice(name.as_bytes());
        buf.push(0);
        buf.extend_from_slice(source.to_string_lossy().as_bytes());
        buf.push(b'\n');
    }
    buf.extend_from_slice(env!("CARGO_PKG_VERSION").as_bytes());
    hash_bytes(&buf)
}
