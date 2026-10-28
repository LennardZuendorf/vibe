//! Parity suite files and corpus cases.
use std::collections::BTreeMap;
use std::fs;
use std::path::{Component, Path, PathBuf};

use serde::Deserialize;

use crate::sandbox::refuse_symlink;

/// One parity suite: which commands run over which corpus classes, and the floors.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Suite {
    /// Suite file format major version; must be 1.
    pub v: u32,
    /// Name printed in the summary line.
    pub name: String,
    /// Corpus directory (`<class>/<case>/case.json`), relative to the suite file.
    pub corpus: PathBuf,
    /// Directory staged into every sandbox as `.oracle/`, relative to the suite file.
    pub oracle_root: PathBuf,
    /// Commands to compare.
    pub commands: Vec<CommandSpec>,
    /// Minimum case count per corpus class. Every class a command uses needs one.
    pub floors: BTreeMap<String, usize>,
    /// Before file comparison, rename a snapshot key on each side to its mapped
    /// value. Use: two engines that write the same logical file to different
    /// paths (the JS oracle's legacy cursor vs the candidate's `.vibe/run`
    /// cursor) compare under one key.
    #[serde(default)]
    pub file_aliases: BTreeMap<String, String>,
    /// Named byte normalizers, applied to every compared file's bytes on both
    /// sides before comparison. See [`NORMALIZERS`].
    #[serde(default)]
    pub normalize: Vec<String>,
    /// Known, deliberate differences between oracle and candidate.
    #[serde(default)]
    pub divergences: Vec<Divergence>,
    /// Wall-clock limit per run, in milliseconds. A run over it fails.
    #[serde(default = "default_timeout_ms")]
    pub timeout_ms: u64,
}

fn default_timeout_ms() -> u64 {
    30_000
}

/// Every normalizer a suite's `normalize` list may name.
pub const NORMALIZERS: [&str; 1] = ["cursor-updated"];

/// Applies one named normalizer to a file's bytes.
///
/// `cursor-updated` replaces every JSON `"updated"` value with a fixed
/// placeholder: the byte sequence `"updated":` (optional following space)
/// followed by a quoted string has the quoted value replaced with `"T"`, so
/// two cursors written seconds apart compare equal. Hand-written; no regex.
pub fn normalize_bytes(name: &str, bytes: &[u8]) -> Result<Vec<u8>, String> {
    match name {
        "cursor-updated" => Ok(cursor_updated(bytes)),
        other => Err(format!(
            "unknown normalizer {other:?}; known: {}",
            NORMALIZERS.join(", ")
        )),
    }
}

const UPDATED_KEY: &[u8] = b"\"updated\":";

fn cursor_updated(bytes: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(bytes.len());
    let mut rest = bytes;
    while let Some(at) = find(rest, UPDATED_KEY) {
        out.extend_from_slice(&rest[..at + UPDATED_KEY.len()]);
        rest = &rest[at + UPDATED_KEY.len()..];
        if rest.first() == Some(&b' ') {
            out.push(b' ');
            rest = &rest[1..];
        }
        match quoted_value(rest) {
            Some(len) => {
                out.extend_from_slice(b"\"T\"");
                rest = &rest[len..];
            }
            None => continue,
        }
    }
    out.extend_from_slice(rest);
    out
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

/// The length of the quoted string at the start of `bytes`, quotes included,
/// or `None` when `bytes` does not start with a closed quoted string.
fn quoted_value(bytes: &[u8]) -> Option<usize> {
    if bytes.first() != Some(&b'"') {
        return None;
    }
    let mut index = 1;
    while index < bytes.len() {
        match bytes[index] {
            b'\\' => index += 2,
            b'"' => return Some(index + 1),
            _ => index += 1,
        }
    }
    None
}

/// Renames each aliased snapshot key to its mapped value. The file's parent
/// directories become present too, because the harness snapshots directories
/// as keys ending in `/`: the side that never created them would otherwise
/// differ from the side that did. When a side holds both spellings, the
/// source key stays put so the comparison reports the extra file.
fn apply_file_aliases(files: &mut BTreeMap<String, Vec<u8>>, aliases: &BTreeMap<String, String>) {
    for (from, to) in aliases {
        let Some(bytes) = files.remove(from) else {
            continue;
        };
        if files.contains_key(to) {
            files.insert(from.clone(), bytes);
            continue;
        }
        let parts: Vec<&str> = to.split('/').collect();
        let mut prefix = String::new();
        for part in &parts[..parts.len() - 1] {
            if !prefix.is_empty() {
                prefix.push('/');
            }
            prefix.push_str(part);
            files.entry(format!("{prefix}/")).or_default();
        }
        files.insert(to.clone(), bytes);
    }
}

/// One command compared between oracle and candidate.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CommandSpec {
    /// Command name; also the key into a case's `runs`.
    pub name: String,
    /// Oracle argv. `{oracle}` expands to the staged oracle directory.
    pub oracle: Vec<String>,
    /// Candidate argv. `{candidate}` expands to the candidate binary.
    /// `null` (or absent) means not ported yet: candidate mode skips the
    /// command; the self-check still runs it.
    #[serde(default)]
    pub candidate: Option<Vec<String>>,
    /// Corpus classes this command runs over.
    pub classes: Vec<String>,
}

/// What a run compares.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Field {
    /// The exit code.
    Exit,
    /// The stdout bytes.
    Stdout,
    /// The bytes of every file left in the sandbox.
    Files,
}

/// A pinned `expected-divergence`: the candidate differs from the oracle on
/// purpose (an oracle bug fixed only in the port, or a deliberate change).
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Divergence {
    /// Command name.
    pub command: String,
    /// Case id, `<class>/<case>`.
    pub case: String,
    /// Only the run with exactly these extra arguments. Absent: every run of the case.
    #[serde(default)]
    pub args: Option<Vec<String>>,
    /// The field that differs.
    pub field: Field,
    /// Only on this `std::env::consts::OS`. Elsewhere the pin is inactive.
    /// An os-scoped pin is never reported stale: local tools vary.
    #[serde(default)]
    pub os: Option<String>,
    /// Why the difference is correct.
    pub reason: String,
}

impl Divergence {
    /// False when the pin is scoped to another operating system.
    pub fn active(&self) -> bool {
        self.os
            .as_deref()
            .is_none_or(|os| os == std::env::consts::OS)
    }
}

/// A corpus case's `case.json`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CaseSpec {
    /// Case file format major version; must be 1.
    pub v: u32,
    /// Working directory inside the sandbox, relative; empty means the sandbox root.
    #[serde(default)]
    pub cwd: String,
    /// Extra argument lists per command. A command without an entry runs once with none.
    #[serde(default)]
    pub runs: BTreeMap<String, Vec<Vec<String>>>,
}

/// A corpus case on disk.
#[derive(Debug)]
pub struct Case {
    /// Class directory name.
    pub class: String,
    /// Case directory name.
    pub name: String,
    /// The case directory.
    pub dir: PathBuf,
    /// The parsed `case.json`.
    pub spec: CaseSpec,
}

impl Case {
    /// `<class>/<case>`.
    pub fn id(&self) -> String {
        format!("{}/{}", self.class, self.name)
    }

    /// The files staged into the sandbox.
    pub fn tree(&self) -> PathBuf {
        self.dir.join("tree")
    }

    /// Argument lists for `command`; one empty list when the case names none.
    pub fn arg_sets(&self, command: &str) -> Vec<Vec<String>> {
        self.spec
            .runs
            .get(command)
            .cloned()
            .unwrap_or_else(|| vec![Vec::new()])
    }
}

fn validate_spec(spec: &CaseSpec) -> Result<(), String> {
    let confined = Path::new(&spec.cwd)
        .components()
        .all(|c| matches!(c, Component::Normal(_)));
    if !confined {
        return Err(format!(
            "cwd {:?} must be empty or a relative path without `..`",
            spec.cwd
        ));
    }
    for (command, sets) in &spec.runs {
        if sets.is_empty() {
            return Err(format!("runs for command {command} has no argument lists"));
        }
    }
    Ok(())
}

impl Suite {
    /// The suite's view of one side's snapshot: aliased keys renamed, then
    /// every file's bytes passed through the named normalizers. Applied to
    /// both sides alike, so a self-check (oracle vs oracle) stays green too.
    pub fn canonical_files(
        &self,
        mut files: BTreeMap<String, Vec<u8>>,
    ) -> Result<BTreeMap<String, Vec<u8>>, String> {
        apply_file_aliases(&mut files, &self.file_aliases);
        for name in &self.normalize {
            for bytes in files.values_mut() {
                *bytes = normalize_bytes(name, bytes)?;
            }
        }
        Ok(files)
    }

    /// Loads a suite file; `corpus` and `oracle_root` resolve against its directory.
    pub fn load(path: &Path) -> Result<Suite, String> {
        let text = fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let mut suite: Suite =
            serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
        if suite.v != 1 {
            return Err(format!(
                "{}: unsupported suite version {}",
                path.display(),
                suite.v
            ));
        }
        for name in &suite.normalize {
            normalize_bytes(name, &[]).map_err(|e| format!("{}: {e}", path.display()))?;
        }
        for (from, to) in &suite.file_aliases {
            if from.is_empty() || to.is_empty() {
                return Err(format!(
                    "{}: file alias {from:?} -> {to:?} must map two non-empty paths",
                    path.display()
                ));
            }
        }
        let base = path.parent().unwrap_or_else(|| Path::new("."));
        suite.corpus = base.join(&suite.corpus);
        suite.oracle_root = base.join(&suite.oracle_root);
        Ok(suite)
    }

    /// The cases of `class`, sorted by name. A missing class directory has
    /// none. Every entry of a class directory must be a case with a
    /// `case.json`, and every `runs` key must name a command of the suite;
    /// either gap is an error, never a silently skipped run.
    pub fn cases(&self, class: &str) -> Result<Vec<Case>, String> {
        refuse_symlink(&self.corpus)?;
        let dir = self.corpus.join(class);
        refuse_symlink(&dir)?;
        if !dir.is_dir() {
            return Ok(Vec::new());
        }
        let mut cases = Vec::new();
        let entries = fs::read_dir(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        for entry in entries {
            let case_dir = entry.map_err(|e| format!("{}: {e}", dir.display()))?.path();
            refuse_symlink(&case_dir)?;
            let file = case_dir.join("case.json");
            refuse_symlink(&file)?;
            if !file.is_file() {
                return Err(format!(
                    "{}: no case.json; every entry of a class directory must be a case",
                    case_dir.display()
                ));
            }
            let text = fs::read_to_string(&file).map_err(|e| format!("{}: {e}", file.display()))?;
            let spec: CaseSpec =
                serde_json::from_str(&text).map_err(|e| format!("{}: {e}", file.display()))?;
            if spec.v != 1 {
                return Err(format!(
                    "{}: unsupported case version {}",
                    file.display(),
                    spec.v
                ));
            }
            validate_spec(&spec).map_err(|e| format!("{}: {e}", file.display()))?;
            if let Some(unknown) = spec
                .runs
                .keys()
                .find(|k| !self.commands.iter().any(|c| &c.name == *k))
            {
                return Err(format!(
                    "{}: runs names command {unknown}, which is not in the suite",
                    file.display()
                ));
            }
            let name = case_dir
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            cases.push(Case {
                class: class.to_string(),
                name,
                dir: case_dir,
                spec,
            });
        }
        cases.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(cases)
    }
}
