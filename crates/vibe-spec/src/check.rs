//! The validate check registry (P3b-R4). Every finding names one check;
//! its level comes from `.vibe/spec.json` `strictness`, else the finding's
//! own default (P3b-R5).
use std::collections::BTreeMap;
use std::path::Path;

use serde_json::Value;

/// Who introduced a check.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Origin {
    /// In the frozen oracle: numbered there, or a family it left unnumbered.
    Legacy,
    /// Added by F17; `off` in the parity corpus (P3b-R2).
    New,
}

/// One registered check.
#[derive(Debug)]
pub struct Check {
    /// `SF12`, `I1`.
    pub id: &'static str,
    /// Who introduced it.
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "read by the registry tests only")
    )]
    pub origin: Origin,
    /// What it checks.
    #[expect(
        dead_code,
        reason = "documentation of the registry; no listing prints it"
    )]
    pub title: &'static str,
}

const fn legacy(id: &'static str, title: &'static str) -> Check {
    Check {
        id,
        origin: Origin::Legacy,
        title,
    }
}

const fn new(id: &'static str, title: &'static str) -> Check {
    Check {
        id,
        origin: Origin::New,
        title,
    }
}

/// Every check, legacy first, in id order within each origin.
pub const CHECKS: &[Check] = &[
    legacy("SF3", "empty design token group"),
    legacy("SF4", "design.md lint"),
    legacy("SF8", "feature Scope section"),
    legacy("SF9", "feature frontmatter fields"),
    legacy("SF10", "requirement keyword and scenario"),
    legacy("SF11", "plan structure"),
    legacy("SF12", "unit ids and R-ID trace"),
    legacy("SF13", "stale features/archive link"),
    legacy("SF14", "scope conflict"),
    legacy("SF15", "root spec length"),
    legacy("SF16", "lesson tags"),
    legacy("I1", "cross-feature unit dependency"),
    legacy("SF20", "frontmatter presence"),
    legacy("SF21", "file naming"),
    legacy("SF22", "link resolution"),
    legacy("SF23", "spec tree and required files"),
    new("SF24", "requirement population"),
    new("SF25", "scenario WHEN/THEN"),
    new("SF26", "frontmatter values"),
    new("SF27", "children and backlinks"),
    new("SF28", "feature folder has a plan row"),
    new("SF29", "updated: not older than the last commit"),
    new("SF30", "layout allowlist"),
    new("SF31", "line budget"),
    new("SF32", "kebab and unique names"),
    new("SF33", "typed notes"),
    new("SF34", "reachable from a hub"),
    new("SF35", "superpowers artifact name"),
    new("SF36", "superpowers plan Spec link"),
];

/// A check's level after configuration.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Level {
    /// Not reported, not counted.
    Off,
    /// A warning.
    Warn,
    /// An error (exit 1).
    Error,
}

impl Level {
    /// `off`, `warn`, `error`.
    pub fn parse(s: &str) -> Option<Level> {
        match s {
            "off" => Some(Level::Off),
            "warn" => Some(Level::Warn),
            "error" => Some(Level::Error),
            _ => None,
        }
    }

    /// The config spelling.
    pub fn as_str(self) -> &'static str {
        match self {
            Level::Off => "off",
            Level::Warn => "warn",
            Level::Error => "error",
        }
    }
}

/// `SF-12`, `sf12`, `SF12` → `SF12`; `i1` → `I1`; unregistered → `None`.
pub fn normalize(key: &str) -> Option<&'static str> {
    let up: String = key
        .chars()
        .filter(|&c| c != '-')
        .collect::<String>()
        .to_ascii_uppercase();
    CHECKS.iter().find(|c| c.id == up).map(|c| c.id)
}

/// `<project>/.vibe/spec.json` `strictness` (P3b-R6). No file or no key is
/// an empty set; an unknown check id warns on stderr and is ignored.
pub fn load(project: &Path, strict: bool) -> Result<Levels, String> {
    let path = project.join(".vibe").join("spec.json");
    if !path.is_file() {
        return Ok(Levels::new(BTreeMap::new(), strict));
    }
    let shown = path.display();
    let text = std::fs::read_to_string(&path).map_err(|e| format!("{shown}: {e}"))?;
    let value: Value =
        serde_json::from_str(&text).map_err(|e| format!("{shown}: not JSON: {e}"))?;
    let Value::Object(map) = value else {
        return Err(format!("{shown}: not a JSON object"));
    };
    let mut set = BTreeMap::new();
    let mut named: BTreeMap<&'static str, &str> = BTreeMap::new();
    match map.get("strictness") {
        None => {}
        Some(Value::Object(entries)) => {
            for (key, v) in entries {
                let level = v.as_str().and_then(Level::parse).ok_or_else(|| {
                    format!("{shown}: strictness.{key} must be \"warn\", \"error\" or \"off\"")
                })?;
                match normalize(key) {
                    Some(id) => {
                        if let Some(first) = named.insert(id, key) {
                            eprintln!(
                                "vibe-spec: {shown}: strictness keys '{first}' and '{key}' both name {id}; '{key}' wins (keys are read in sorted order)"
                            );
                        }
                        set.insert(id, level);
                    }
                    None => {
                        eprintln!(
                            "vibe-spec: {shown}: unknown check '{key}' in strictness (ignored)"
                        );
                    }
                }
            }
        }
        Some(_) => return Err(format!("{shown}: `strictness` must be an object")),
    }
    Ok(Levels::new(set, strict))
}

/// Configured levels plus `--strict`.
#[derive(Clone, Debug, Default)]
pub struct Levels {
    set: BTreeMap<&'static str, Level>,
    strict: bool,
}

impl Levels {
    /// Levels from a config map and the `--strict` flag.
    pub fn new(set: BTreeMap<&'static str, Level>, strict: bool) -> Levels {
        Levels { set, strict }
    }

    /// `--strict` was given.
    pub fn strict(&self) -> bool {
        self.strict
    }

    /// The level of one finding of `check` whose own default is `default`.
    pub fn resolve(&self, check: &str, default: Level) -> Level {
        let level = self.set.get(check).copied().unwrap_or(default);
        if self.strict && level == Level::Warn {
            Level::Error
        } else {
            level
        }
    }

    /// What `--format json` reports for `check`: `off`, `warn`, `error`, or
    /// `default` when unconfigured; under `--strict` every non-off is `error`.
    pub fn describe(&self, check: &str) -> &'static str {
        match (self.set.get(check), self.strict) {
            (Some(Level::Off), _) => "off",
            (_, true) => "error",
            (Some(l), false) => l.as_str(),
            (None, false) => "default",
        }
    }

    /// Every registered check is configured `off`.
    pub fn all_off(&self) -> bool {
        CHECKS
            .iter()
            .all(|c| self.set.get(c.id) == Some(&Level::Off))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    #[allow(clippy::unwrap_used)]
    fn the_corpus_config_turns_off_exactly_the_new_checks() {
        let text = include_str!("../../../tests/parity/legacy-strictness.json");
        let v: serde_json::Value = serde_json::from_str(text).unwrap();
        let mut got: Vec<&str> = v["strictness"]
            .as_object()
            .unwrap()
            .iter()
            .map(|(k, level)| {
                assert_eq!(level, "off", "{k}");
                normalize(k).unwrap()
            })
            .collect();
        got.sort_unstable();
        let mut want: Vec<&str> = CHECKS
            .iter()
            .filter(|c| c.origin == Origin::New)
            .map(|c| c.id)
            .collect();
        want.sort_unstable();
        assert_eq!(got, want);
    }

    #[test]
    fn ids_are_unique_and_partitioned() {
        let mut seen = std::collections::BTreeSet::new();
        for c in CHECKS {
            assert!(seen.insert(c.id), "duplicate {}", c.id);
        }
        let legacy: Vec<&str> = CHECKS
            .iter()
            .filter(|c| c.origin == Origin::Legacy)
            .map(|c| c.id)
            .collect();
        assert_eq!(
            legacy,
            [
                "SF3", "SF4", "SF8", "SF9", "SF10", "SF11", "SF12", "SF13", "SF14", "SF15", "SF16",
                "I1", "SF20", "SF21", "SF22", "SF23"
            ]
        );
        let new: Vec<&str> = CHECKS
            .iter()
            .filter(|c| c.origin == Origin::New)
            .map(|c| c.id)
            .collect();
        assert_eq!(
            new,
            [
                "SF24", "SF25", "SF26", "SF27", "SF28", "SF29", "SF30", "SF31", "SF32", "SF33",
                "SF34", "SF35", "SF36"
            ]
        );
    }

    #[test]
    fn keys_normalize_to_registered_ids() {
        assert_eq!(normalize("SF-12"), Some("SF12"));
        assert_eq!(normalize("sf12"), Some("SF12"));
        assert_eq!(normalize("SF12"), Some("SF12"));
        assert_eq!(normalize("i1"), Some("I1"));
        assert_eq!(normalize("SF-1"), None);
        assert_eq!(normalize("SF99"), None);
        assert_eq!(normalize(""), None);
    }

    #[test]
    fn levels_resolve_config_then_default_then_strict() {
        let mut set = BTreeMap::new();
        set.insert("SF12", Level::Off);
        set.insert("SF13", Level::Error);
        set.insert("SF20", Level::Warn);
        let plain = Levels::new(set.clone(), false);
        assert_eq!(plain.resolve("SF12", Level::Warn), Level::Off);
        assert_eq!(plain.resolve("SF13", Level::Warn), Level::Error);
        assert_eq!(plain.resolve("SF20", Level::Error), Level::Warn);
        assert_eq!(plain.resolve("SF14", Level::Warn), Level::Warn);
        assert_eq!(plain.resolve("SF14", Level::Error), Level::Error);
        let strict = Levels::new(set, true);
        assert_eq!(strict.resolve("SF12", Level::Warn), Level::Off);
        assert_eq!(strict.resolve("SF14", Level::Warn), Level::Error);
        assert_eq!(strict.resolve("SF20", Level::Error), Level::Error);
        assert_eq!(plain.describe("SF12"), "off");
        assert_eq!(plain.describe("SF14"), "default");
        assert_eq!(strict.describe("SF14"), "error");
        assert_eq!(strict.describe("SF12"), "off");
        assert!(!plain.all_off());
        let all: BTreeMap<&'static str, Level> =
            CHECKS.iter().map(|c| (c.id, Level::Off)).collect();
        assert!(Levels::new(all, false).all_off());
    }

    #[test]
    fn level_names_round_trip() {
        for l in [Level::Off, Level::Warn, Level::Error] {
            assert_eq!(Level::parse(l.as_str()), Some(l));
        }
        assert_eq!(Level::parse("warning"), None);
    }
}
