//! Parity suite files and corpus cases.
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

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
    /// Known, deliberate differences between oracle and candidate.
    #[serde(default)]
    pub divergences: Vec<Divergence>,
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
    pub candidate: Vec<String>,
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
    /// The field that differs.
    pub field: Field,
    /// Why the difference is correct.
    pub reason: String,
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

impl Suite {
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
        let base = path.parent().unwrap_or_else(|| Path::new("."));
        suite.corpus = base.join(&suite.corpus);
        suite.oracle_root = base.join(&suite.oracle_root);
        Ok(suite)
    }

    /// The cases of `class`, sorted by name. A missing class directory has none.
    pub fn cases(&self, class: &str) -> Result<Vec<Case>, String> {
        let dir = self.corpus.join(class);
        if !dir.is_dir() {
            return Ok(Vec::new());
        }
        let mut cases = Vec::new();
        let entries = fs::read_dir(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        for entry in entries {
            let case_dir = entry.map_err(|e| format!("{}: {e}", dir.display()))?.path();
            let file = case_dir.join("case.json");
            if !file.is_file() {
                continue;
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
