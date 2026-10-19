//! validate's findings and their text rendering. Text output is the F16
//! byte format; each finding also keeps its check, severity and file.
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use serde_json::json;

use crate::check::{CHECKS, Level, Levels, normalize};
use crate::text::{echo_e, find, lossy, trim_space};

/// A finding's severity after strictness.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Severity {
    /// Counted in `Warnings:`.
    Warning,
    /// Counted in `Errors:`; exit 1.
    Error,
}

impl Severity {
    /// The Spec JSON spelling.
    pub fn as_str(self) -> &'static str {
        match self {
            Severity::Warning => "warning",
            Severity::Error => "error",
        }
    }
}

/// One reported finding.
#[derive(Debug)]
pub struct Finding {
    /// The registered check id.
    pub check: &'static str,
    /// After strictness.
    pub severity: Severity,
    /// Project-relative, `/`-joined; absolute outside the project; `""` for the tree.
    pub file: String,
    /// The text-mode message without colour or tag.
    pub message: Vec<u8>,
}

/// validate's stdout, counters and findings.
pub struct Report {
    /// Everything printed so far (text mode).
    pub out: Vec<u8>,
    /// `Errors:`.
    pub errors: u32,
    /// `Warnings:`.
    pub warnings: u32,
    /// Every finding not dropped by `off`.
    pub findings: Vec<Finding>,
    levels: Levels,
    project: PathBuf,
    file: String,
    examined: BTreeSet<String>,
    markers: BTreeMap<String, Vec<&'static str>>,
}

impl Report {
    /// An empty report for a project.
    pub fn new(project: &Path, levels: Levels) -> Report {
        Report {
            out: Vec::new(),
            errors: 0,
            warnings: 0,
            findings: Vec::new(),
            levels,
            project: project.to_path_buf(),
            file: String::new(),
            examined: BTreeSet::new(),
            markers: BTreeMap::new(),
        }
    }

    /// The levels in force.
    pub fn levels(&self) -> &Levels {
        &self.levels
    }

    /// Later findings belong to `file` (see [`Finding::file`]).
    pub fn at(&mut self, file: &str) {
        self.file = file.to_string();
    }

    /// Like [`Report::at`], and counts `file` as examined when non-empty.
    pub fn examine(&mut self, file: &str) {
        self.at(file);
        if !file.is_empty() {
            self.examined.insert(file.to_string());
        }
    }

    /// Distinct files examined.
    pub fn examined(&self) -> usize {
        self.examined.len()
    }

    /// A finding whose default is error.
    pub fn error(&mut self, check: &'static str, msg: &[u8]) {
        self.push(check, Level::Error, true, msg);
    }

    /// A finding whose default is warning.
    pub fn warn(&mut self, check: &'static str, msg: &[u8]) {
        self.push(check, Level::Warn, true, msg);
    }

    /// A warning the oracle prints from a pipeline subshell and so never
    /// counts; it counts once it resolves to error.
    pub fn warn_uncounted(&mut self, check: &'static str, msg: &[u8]) {
        self.push(check, Level::Warn, false, msg);
    }

    /// An error no configuration can silence (`--strict` refusal).
    pub fn refuse(&mut self, msg: &[u8]) {
        self.colour("31", "ERROR: ", msg);
        self.errors += 1;
        self.record("SF23", Severity::Error, msg);
    }

    fn push(&mut self, check: &'static str, default: Level, counted: bool, msg: &[u8]) {
        if self.allowed(check) {
            return;
        }
        match self.levels.resolve(check, default) {
            Level::Off => {}
            Level::Warn => {
                self.colour("33", "WARN:  ", msg);
                if counted {
                    self.warnings += 1;
                }
                self.record(check, Severity::Warning, msg);
            }
            Level::Error => {
                self.colour("31", "ERROR: ", msg);
                self.errors += 1;
                self.record(check, Severity::Error, msg);
            }
        }
    }

    /// The current file holds `<!-- vibe-spec: allow <check> -->` (P3b-R8).
    fn allowed(&mut self, check: &str) -> bool {
        if self.file.is_empty() {
            return false;
        }
        if !self.markers.contains_key(&self.file) {
            let path = if self.file.starts_with('/') {
                PathBuf::from(&self.file)
            } else {
                self.project.join(&self.file)
            };
            let ids = std::fs::read(path)
                .map(|t| allow_markers(&t))
                .unwrap_or_default();
            self.markers.insert(self.file.clone(), ids);
        }
        self.markers
            .get(&self.file)
            .is_some_and(|ids| ids.contains(&check))
    }

    fn record(&mut self, check: &'static str, severity: Severity, msg: &[u8]) {
        self.findings.push(Finding {
            check,
            severity,
            file: self.file.clone(),
            message: msg.to_vec(),
        });
    }

    fn colour(&mut self, code: &str, tag: &str, msg: &[u8]) {
        self.out
            .extend(echo_e(&cat!("\\033[", code, "m  ", tag, msg, "\\033[0m")));
    }

    /// Keeps only the findings `keep` accepts; the counts and the text
    /// (after `header`) are rebuilt from them.
    pub fn retain(&mut self, header: &[u8], keep: impl Fn(&Finding) -> bool) {
        let mut kept = std::mem::take(&mut self.findings);
        kept.retain(|f| keep(f));
        self.out = header.to_vec();
        self.errors = 0;
        self.warnings = 0;
        for f in &kept {
            let (code, tag) = match f.severity {
                Severity::Error => {
                    self.errors += 1;
                    ("31", "ERROR: ")
                }
                Severity::Warning => {
                    self.warnings += 1;
                    ("33", "WARN:  ")
                }
            };
            self.colour(code, tag, &f.message);
        }
        self.findings = kept;
    }

    /// Spec JSON v1 `validate`.
    pub fn json(&self, root: &str, layout: &str) -> Vec<u8> {
        let checks: Vec<serde_json::Value> = CHECKS
            .iter()
            .map(|c| json!({ "id": c.id, "level": self.levels.describe(c.id) }))
            .collect();
        let findings: Vec<serde_json::Value> = self
            .findings
            .iter()
            .map(|f| {
                let mut v = json!({
                    "check": f.check,
                    "severity": f.severity.as_str(),
                    "message": lossy(&f.message),
                });
                if !f.file.is_empty() {
                    v["file"] = json!(f.file);
                }
                v
            })
            .collect();
        let v = json!({
            "v": 1,
            "root": root,
            "layout": layout,
            "strict": self.levels.strict(),
            "examined": self.examined(),
            "errors": self.errors,
            "warnings": self.warnings,
            "checks": checks,
            "findings": findings,
        });
        format!("{v}\n").into_bytes()
    }

    /// `green`.
    pub fn green(&mut self, msg: &[u8]) {
        self.colour("32", "OK:    ", msg);
    }

    /// `info`.
    pub fn info(&mut self, msg: &[u8]) {
        self.colour("36", "INFO:  ", msg);
    }

    /// Plain `echo "$msg"`.
    pub fn line(&mut self, msg: &[u8]) {
        self.out.extend_from_slice(msg);
        self.out.push(b'\n');
    }
}

/// The checks named by `<!-- vibe-spec: allow SF-nn -->` markers in `text`.
pub fn allow_markers(text: &[u8]) -> Vec<&'static str> {
    const OPEN: &[u8] = b"<!-- vibe-spec: allow ";
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(i) = find(rest, OPEN) {
        let after = rest.get(i + OPEN.len()..).unwrap_or_default();
        let Some(end) = find(after, b"-->") else {
            break;
        };
        let key = trim_space(after.get(..end).unwrap_or_default());
        if let Some(id) = std::str::from_utf8(key).ok().and_then(normalize) {
            out.push(id);
        }
        rest = after.get(end + 3..).unwrap_or_default();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::check::{Level, Levels};
    use std::collections::BTreeMap;
    use std::path::Path;

    fn report(set: &[(&'static str, Level)], strict: bool) -> Report {
        Report::new(
            Path::new("/nonexistent"),
            Levels::new(set.iter().copied().collect::<BTreeMap<_, _>>(), strict),
        )
    }

    #[test]
    fn default_levels_render_like_f16() {
        let mut r = report(&[], false);
        r.warn("SF13", b"w");
        r.error("SF22", b"e");
        r.warn_uncounted("SF11", b"u");
        r.green(b"g");
        r.info(b"i");
        assert_eq!(
            r.out,
            b"\x1b[33m  WARN:  w\x1b[0m\n\x1b[31m  ERROR: e\x1b[0m\n\x1b[33m  WARN:  u\x1b[0m\n\x1b[32m  OK:    g\x1b[0m\n\x1b[36m  INFO:  i\x1b[0m\n"
        );
        assert_eq!((r.errors, r.warnings), (1, 1));
        assert_eq!(r.findings.len(), 3);
        assert_eq!(r.findings[1].severity, Severity::Error);
        assert_eq!(r.findings[1].check, "SF22");
    }

    #[test]
    fn off_drops_error_promotes_strict_counts_uncounted() {
        let mut r = report(&[("SF13", Level::Off), ("SF14", Level::Error)], false);
        r.warn("SF13", b"gone");
        r.warn("SF14", b"up");
        assert_eq!(r.out, b"\x1b[31m  ERROR: up\x1b[0m\n");
        assert_eq!((r.errors, r.warnings, r.findings.len()), (1, 0, 1));
        let mut s = report(&[], true);
        s.warn_uncounted("SF11", b"u");
        assert_eq!((s.errors, s.warnings), (1, 0));
    }

    #[test]
    fn examined_counts_distinct_files() {
        let mut r = report(&[], false);
        r.examine(".spec/a.md");
        r.examine(".spec/a.md");
        r.at(".spec/b.md");
        r.examine("");
        r.examine(".spec/c.md");
        assert_eq!(r.examined(), 2);
    }

    #[test]
    fn markers_name_checks() {
        assert_eq!(
            allow_markers(
                b"a <!-- vibe-spec: allow SF-13 --> b <!-- vibe-spec: allow i1--> <!-- vibe-spec: allow SF99 --> <!-- vibe-spec: allow SF12"
            ),
            vec!["SF13", "I1"]
        );
    }

    #[test]
    fn refuse_ignores_levels() {
        let all: Vec<(&'static str, Level)> = crate::check::CHECKS
            .iter()
            .map(|c| (c.id, Level::Off))
            .collect();
        let mut r = report(&all, true);
        r.refuse(b"no");
        assert_eq!((r.errors, r.findings.len()), (1, 1));
        assert_eq!(r.findings[0].check, "SF23");
    }
}
