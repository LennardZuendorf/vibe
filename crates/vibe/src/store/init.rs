//! `vibe store init`: the store skeleton.
use std::fs;
use std::path::{Path, PathBuf};

use vibe_core::json::{Object, Value};

use super::read_optional;
use crate::paths::Paths;

/// One entry the skeleton still lacks.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InitFile {
    /// Where the entry goes.
    pub path: PathBuf,
    /// File content; `None` marks a directory.
    pub bytes: Option<Vec<u8>>,
}

/// What `init` did to one entry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InitEntry {
    /// The entry's path.
    pub path: PathBuf,
    /// `true` when this run created it, `false` when it already existed.
    pub created: bool,
}

/// The outcome of [`init`], one entry per skeleton path in creation order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InitReport {
    /// Every skeleton entry with its status.
    pub entries: Vec<InitEntry>,
}

const PLACEHOLDER: &str = "<!-- vibe store: edit this file; every harness reads it -->\n";

fn store_json_bytes() -> Vec<u8> {
    let mut sources = Object::new();
    sources.insert("sources", Value::Array(Vec::new()));
    let mut root = Object::new();
    root.insert("v", Value::Number(1i64.into()));
    root.insert("harnesses", Value::Object(Object::new()));
    root.insert("skills", Value::Object(sources));
    vibe_core::json::to_pretty(&Value::Object(root)).into_bytes()
}

fn mcp_json_bytes() -> Vec<u8> {
    let mut root = Object::new();
    root.insert("mcpServers", Value::Object(Object::new()));
    vibe_core::json::to_pretty(&Value::Object(root)).into_bytes()
}

fn exists(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok()
}

/// Refuses a dangling symlink: creating the file would write through it. A symlink to an
/// existing file is a valid read-only input.
fn refuse_dangling_symlink(path: &Path) -> Result<(), String> {
    match fs::symlink_metadata(path) {
        Ok(m) if m.file_type().is_symlink() && fs::metadata(path).is_err() => Err(format!(
            "{}: is a dangling symlink; refusing to write through it",
            path.display()
        )),
        _ => Ok(()),
    }
}

fn skeleton(paths: &Paths) -> [PathBuf; 4] {
    [
        paths.store_json(),
        paths.store_agents(),
        paths.store_mcp(),
        paths.store_skills(),
    ]
}

/// The skeleton entries that do not exist yet, in creation order. Pure: writes nothing.
/// `instructions` seeds `AGENTS.md`. A dangling symlink at `store.json`, `AGENTS.md` or `mcp.json` is refused; a symlink to an existing file is read as it is.
pub fn init_files(paths: &Paths, instructions: Option<&Path>) -> Result<Vec<InitFile>, String> {
    for file in [paths.store_json(), paths.store_agents(), paths.store_mcp()] {
        refuse_dangling_symlink(&file)?;
    }
    let mut out = Vec::new();
    if !exists(&paths.store_json()) {
        out.push(InitFile {
            path: paths.store_json(),
            bytes: Some(store_json_bytes()),
        });
    }
    if !exists(&paths.store_agents()) {
        let bytes = match instructions {
            Some(src) => {
                read_optional(src)?.ok_or_else(|| format!("{}: no such file", src.display()))?
            }
            None => PLACEHOLDER.as_bytes().to_vec(),
        };
        out.push(InitFile {
            path: paths.store_agents(),
            bytes: Some(bytes),
        });
    }
    if !exists(&paths.store_mcp()) {
        out.push(InitFile {
            path: paths.store_mcp(),
            bytes: Some(mcp_json_bytes()),
        });
    }
    if !exists(&paths.store_skills()) {
        out.push(InitFile {
            path: paths.store_skills(),
            bytes: None,
        });
    }
    Ok(out)
}

/// Creates the missing skeleton entries; existing entries are never touched.
pub fn init(paths: &Paths, instructions: Option<&Path>) -> Result<InitReport, String> {
    let missing = init_files(paths, instructions)?;
    fs::create_dir_all(&paths.store).map_err(|e| format!("{}: {e}", paths.store.display()))?;
    for item in &missing {
        match &item.bytes {
            None => fs::create_dir_all(&item.path)
                .map_err(|e| format!("{}: {e}", item.path.display()))?,
            Some(bytes) => {
                vibe_core::fs::write_atomic(&item.path, bytes).map_err(|e| e.to_string())?
            }
        }
    }
    let entries = skeleton(paths)
        .into_iter()
        .map(|path| InitEntry {
            created: missing.iter().any(|m| m.path == path),
            path,
        })
        .collect();
    Ok(InitReport { entries })
}
