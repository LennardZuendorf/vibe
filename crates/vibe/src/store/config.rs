//! `store.json`: per-harness switches and extra skill sources.
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use vibe_core::json::Value;

use crate::paths::{Harness, Paths};

/// The parsed `store.json`. Unknown keys are ignored. A missing `v` is read as version 1;
/// any other `v` is refused.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StoreConfig {
    /// Explicit per-harness switches; a harness not listed keeps its default.
    pub enabled: BTreeMap<Harness, bool>,
    /// Extra skill source directories in file order: `~` expanded, relative entries resolved
    /// against the store directory.
    pub skill_sources: Vec<PathBuf>,
}

/// Expands a leading `~` against `home`.
fn expand(raw: &str, home: &Path) -> PathBuf {
    if raw == "~" {
        return home.to_path_buf();
    }
    match raw.strip_prefix("~/").or_else(|| raw.strip_prefix("~\\")) {
        Some(rest) => home.join(rest),
        None => PathBuf::from(raw),
    }
}

/// Reads the configuration out of a parsed `store.json`; the `Err` names the offending key.
pub fn parse(root: &Value, paths: &Paths) -> Result<StoreConfig, String> {
    let obj = root.as_object().ok_or("expected a JSON object")?;
    let mut config = StoreConfig::default();
    if let Some(v) = obj.get("v")
        && !matches!(v, Value::Number(n) if n.as_i64() == Some(1))
    {
        return Err("unsupported \"v\" (this vibe reads store version 1)".to_string());
    }
    if let Some(harnesses) = obj.get("harnesses") {
        let harnesses = harnesses
            .as_object()
            .ok_or("\"harnesses\" must be an object")?;
        for (id, entry) in harnesses.iter() {
            let Some(h) = Harness::parse(id) else {
                continue;
            };
            let entry = entry
                .as_object()
                .ok_or_else(|| format!("\"harnesses.{id}\" must be an object"))?;
            if let Some(flag) = entry.get("enabled") {
                let flag = flag
                    .as_bool()
                    .ok_or_else(|| format!("\"harnesses.{id}.enabled\" must be true or false"))?;
                config.enabled.insert(h, flag);
            }
        }
    }
    if let Some(skills) = obj.get("skills") {
        let skills = skills.as_object().ok_or("\"skills\" must be an object")?;
        if let Some(sources) = skills.get("sources") {
            let sources = sources
                .as_array()
                .ok_or("\"skills.sources\" must be an array of strings")?;
            for s in sources {
                let s = s
                    .as_str()
                    .ok_or("\"skills.sources\" must be an array of strings")?;
                let path = expand(s, &paths.home);
                config.skill_sources.push(if path.is_absolute() {
                    path
                } else {
                    paths.store.join(path)
                });
            }
        }
    }
    Ok(config)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    #[test]
    fn expand_handles_tilde_forms() {
        let home = Path::new("/h");
        assert_eq!(expand("~", home), Path::new("/h"));
        assert_eq!(expand("~/a/b", home), Path::new("/h").join("a/b"));
        assert_eq!(expand("~user/x", home), Path::new("~user/x"));
        assert_eq!(expand("/abs", home), Path::new("/abs"));
    }
}
