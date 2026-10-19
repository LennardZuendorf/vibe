//! Spec root discovery (`.spec/tech-spec.md`, "Root and discovery"). One // literal
//! function implements the order; every command goes through it.
//!
//! Order: `--root`, `$VIBE_SPEC_ROOT`, `.vibe/spec.json` `root`, `docs/spec/`,
//! `.spec/`; steps 3–5 repeat per parent, up to the directory holding `.git`; // literal
//! nothing found → exit 3.
use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};
use vibe_core::exit;

/// The discovery step that found the root (`root.source` in Spec JSON v1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    /// `--root`.
    Flag,
    /// `$VIBE_SPEC_ROOT`.
    Env,
    /// `.vibe/spec.json` key `root`.
    Config,
    /// `docs/spec/`.
    DocsSpec,
    /// `.spec/` (legacy). // literal
    DotSpec,
}

impl Source {
    /// The contract spelling.
    pub fn as_str(self) -> &'static str {
        match self {
            Source::Flag => "flag",
            Source::Env => "env",
            Source::Config => "config",
            Source::DocsSpec => "docs-spec",
            Source::DotSpec => "dot-spec",
        }
    }
}

/// A found spec root.
#[derive(Debug)]
pub struct Root {
    /// Absolute path of the spec root.
    pub dir: PathBuf,
    /// The project directory: where a walk step hit, or for `--root` and
    /// `$VIBE_SPEC_ROOT` the nearest ancestor of the cwd holding `.git`.
    pub project: PathBuf,
    /// The step that hit.
    pub source: Source,
    /// Parent directories walked before the hit; 0 for the cwd, flag and env.
    pub depth: usize,
}

impl Root {
    /// `dir` relative to `project`, `/`-joined (`.spec`, `docs/spec`); the // literal
    /// absolute path when `dir` lies outside `project`.
    pub fn display(&self) -> String {
        match self.dir.strip_prefix(&self.project) {
            Ok(rel) if !rel.as_os_str().is_empty() => rel
                .components()
                .map(|c| c.as_os_str().to_string_lossy().into_owned())
                .collect::<Vec<_>>()
                .join("/"),
            _ => self.dir.to_string_lossy().into_owned(),
        }
    }
}

/// Why discovery found no root.
#[derive(Debug)]
pub enum Miss {
    /// No step hit.
    NotFound,
    /// `--root`, `$VIBE_SPEC_ROOT` or a config `root` names no directory.
    Missing {
        /// The step that named it.
        source: Source,
        /// The path it named.
        path: PathBuf,
    },
    /// `.vibe/spec.json` is unreadable, not a JSON object, or `root` is not a non-empty string.
    BadConfig {
        /// The config file.
        path: PathBuf,
        /// What is wrong with it.
        why: String,
    },
}

impl Miss {
    /// Exit code: 2 for a bad config, else 3.
    pub fn code(&self) -> u8 {
        match self {
            Miss::BadConfig { .. } => exit::USAGE,
            _ => exit::NO_ROOT,
        }
    }

    /// One stderr line.
    pub fn message(&self) -> String {
        match self {
            Miss::NotFound => "vibe-spec: no spec root (searched docs/spec/ and .spec/ up to the directory holding .git)".to_string(), // literal
            Miss::Missing { source, path } => format!(
                "vibe-spec: spec root {} (from {}) is not a directory",
                path.display(),
                source.as_str()
            ),
            Miss::BadConfig { path, why } => format!("vibe-spec: {}: {why}", path.display()),
        }
    }
}

/// Runs the discovery order from `cwd`.
pub fn discover(cwd: &Path, flag: Option<&Path>, env: Option<&OsStr>) -> Result<Root, Miss> {
    if let Some(path) = flag {
        return explicit(cwd, path, Source::Flag);
    }
    if let Some(path) = env.filter(|p| !p.is_empty()) {
        return explicit(cwd, Path::new(path), Source::Env);
    }
    let mut dir = cwd.to_path_buf();
    let mut depth = 0;
    loop {
        if let Some(found) = at(&dir, depth)? {
            return Ok(found);
        }
        if dir.join(".git").exists() {
            return Err(Miss::NotFound);
        }
        match dir.parent() {
            Some(up) => {
                dir = up.to_path_buf();
                depth += 1;
            }
            None => return Err(Miss::NotFound),
        }
    }
}

fn explicit(cwd: &Path, path: &Path, source: Source) -> Result<Root, Miss> {
    let dir = cwd.join(path);
    if !dir.is_dir() {
        return Err(Miss::Missing { source, path: dir });
    }
    Ok(Root {
        dir,
        project: project_of(cwd),
        source,
        depth: 0,
    })
}

/// The stderr warning for a project that holds both `docs/spec/` and the legacy
/// root while discovery fell through to `docs/spec/` (no config `root`).
pub fn both_roots_warning(root: &Root) -> Option<String> {
    if root.source != Source::DocsSpec {
        return None;
    }
    let legacy = root.project.join(crate::literal::LEGACY);
    std::fs::symlink_metadata(legacy)
        .ok()
        .filter(|m| m.is_dir())?;
    Some(format!(
        "vibe-spec: warning: both docs/spec/ and {}/ exist; using docs/spec/ (set \"root\" in .vibe/spec.json)",
        crate::literal::LEGACY
    ))
}

/// Steps 3–5 at one directory.
fn at(dir: &Path, depth: usize) -> Result<Option<Root>, Miss> {
    let config = dir.join(".vibe").join("spec.json");
    if config.is_file()
        && let Some(rel) = config_root(&config)?
    {
        let spec = dir.join(rel);
        if !spec.is_dir() {
            return Err(Miss::Missing {
                source: Source::Config,
                path: spec,
            });
        }
        return Ok(Some(Root {
            dir: spec,
            project: dir.to_path_buf(),
            source: Source::Config,
            depth,
        }));
    }
    for (rel, source) in [
        ("docs/spec", Source::DocsSpec),
        (crate::literal::LEGACY, Source::DotSpec),
    ] {
        let spec = dir.join(rel);
        if spec.is_dir() {
            return Ok(Some(Root {
                dir: spec,
                project: dir.to_path_buf(),
                source,
                depth,
            }));
        }
    }
    Ok(None)
}

/// The `root` key of a config file; `None` when it has none.
fn config_root(path: &Path) -> Result<Option<PathBuf>, Miss> {
    let bad = |why: String| Miss::BadConfig {
        path: path.to_path_buf(),
        why,
    };
    let text = std::fs::read_to_string(path).map_err(|e| bad(e.to_string()))?;
    let value: Value = serde_json::from_str(&text).map_err(|e| bad(format!("not JSON: {e}")))?;
    let Value::Object(map) = value else {
        return Err(bad("not a JSON object".to_string()));
    };
    match map.get("root") {
        None => Ok(None),
        Some(Value::String(s)) if !s.is_empty() => Ok(Some(PathBuf::from(s))),
        Some(_) => Err(bad("`root` must be a non-empty string".to_string())),
    }
}

/// How the spec tooth treats a would-be block (`tooth` in `.vibe/spec.json`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tooth {
    /// Refuse the edit (exit 2). The default.
    Block,
    /// Print the refusal and allow the edit.
    Warn,
}

/// The `tooth` key of `<project>/.vibe/spec.json`; `Block` when the file or key is absent.
pub fn tooth(project: &Path) -> Result<Tooth, String> {
    let path = project.join(".vibe").join("spec.json");
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Tooth::Block),
        Err(e) => return Err(format!("{}: {e}", path.display())),
    };
    let value: Value =
        serde_json::from_str(&text).map_err(|e| format!("{}: not JSON: {e}", path.display()))?;
    match value.get("tooth") {
        None => Ok(Tooth::Block),
        Some(Value::String(s)) if s == "block" => Ok(Tooth::Block),
        Some(Value::String(s)) if s == "warn" => Ok(Tooth::Warn),
        Some(_) => Err(format!(
            "{}: `tooth` must be \"block\" or \"warn\"",
            path.display()
        )),
    }
}

/// The `migrate` section of `.vibe/spec.json` (P4-R12).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MigrateConfig {
    /// Extra globs added to the default file set.
    pub include: Vec<String>,
    /// Globs subtracted from the file set.
    pub exclude: Vec<String>,
    /// The fewest files a run must scan.
    pub floor: usize,
}

impl Default for MigrateConfig {
    fn default() -> Self {
        MigrateConfig {
            include: Vec::new(),
            exclude: Vec::new(),
            floor: 1,
        }
    }
}

/// The `migrate` section of `<project>/.vibe/spec.json`: `include` and
/// `exclude` (arrays of non-empty strings) and `floor` (a non-negative
/// integer, default 1). No file or no section gives the defaults; a wrong
/// type or an unknown key fails closed, and so does a symlinked config.
pub fn migrate_config(project: &Path) -> Result<MigrateConfig, String> {
    let path = project.join(".vibe").join("spec.json");
    let shown = path.display().to_string();
    match std::fs::symlink_metadata(&path) {
        Ok(m) if m.file_type().is_symlink() => {
            return Err(format!(
                "{shown}: is a symlink; not reading migrate settings through it"
            ));
        }
        Ok(_) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(MigrateConfig::default()),
        Err(e) => return Err(format!("{shown}: {e}")),
    }
    let text = std::fs::read_to_string(&path).map_err(|e| format!("{shown}: {e}"))?;
    let value: Value =
        serde_json::from_str(&text).map_err(|e| format!("{shown}: not JSON: {e}"))?;
    let Value::Object(map) = value else {
        return Err(format!("{shown}: not a JSON object"));
    };
    let Some(section) = map.get("migrate") else {
        return Ok(MigrateConfig::default());
    };
    let Value::Object(section) = section else {
        return Err(format!("{shown}: `migrate` must be an object"));
    };
    let mut cfg = MigrateConfig::default();
    for (key, v) in section {
        match key.as_str() {
            "include" => cfg.include = globs(v, &shown, "include")?,
            "exclude" => cfg.exclude = globs(v, &shown, "exclude")?,
            "floor" => {
                cfg.floor = v
                    .as_u64()
                    .and_then(|n| usize::try_from(n).ok())
                    .ok_or_else(|| {
                        format!("{shown}: `migrate.floor` must be a non-negative integer")
                    })?;
            }
            other => {
                return Err(format!(
                    "{shown}: unknown key `migrate.{other}` (allowed: include, exclude, floor)"
                ));
            }
        }
    }
    Ok(cfg)
}

fn globs(v: &Value, shown: &str, key: &str) -> Result<Vec<String>, String> {
    let bad = || format!("{shown}: `migrate.{key}` must be an array of non-empty strings");
    let Value::Array(items) = v else {
        return Err(bad());
    };
    items
        .iter()
        .map(|i| match i {
            Value::String(s) if !s.is_empty() => Ok(s.clone()),
            _ => Err(bad()),
        })
        .collect()
}

/// The nearest ancestor of `cwd` (inclusive) holding `.git`; `cwd` when none does.
fn project_of(cwd: &Path) -> PathBuf {
    cwd.ancestors()
        .find(|d| d.join(".git").exists())
        .unwrap_or(cwd)
        .to_path_buf()
}

/// `vibe-spec root [--json]`.
pub fn command(found: &Result<Root, Miss>, json: bool) -> (u8, Vec<u8>) {
    let root = match found {
        Ok(root) => root,
        Err(miss) => {
            eprintln!("{}", miss.message());
            return (miss.code(), Vec::new());
        }
    };
    if !json {
        return (exit::OK, format!("{}\n", root.dir.display()).into_bytes());
    }
    let mut v = json!({
        "v": 1,
        "root": root.dir.to_string_lossy(),
        "source": root.source.as_str(),
    });
    if !matches!(root.source, Source::Flag | Source::Env) {
        v["depth"] = json!(root.depth);
    }
    v["layout"] = json!(crate::layered::detect(&root.dir).as_str());
    (exit::OK, format!("{v}\n").into_bytes())
}
