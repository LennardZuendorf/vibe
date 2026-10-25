//! Skill discovery over the store's sources.
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

/// One skill: a directory holding `SKILL.md`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Skill {
    /// The directory name.
    pub name: String,
    /// The skill directory.
    pub dir: PathBuf,
    /// The source directory it was found in.
    pub source: PathBuf,
}

/// A skill dropped because an earlier source already holds that name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Shadowed {
    /// The skill name.
    pub name: String,
    /// The directory that won.
    pub kept: PathBuf,
    /// The directory that lost.
    pub dropped: PathBuf,
}

/// Finds skills in `sources`, in order; the first source holding a name wins.
/// A child directory (a symlink to one counts) is a skill when it has `SKILL.md`.
/// Names are sorted within a source; an unreadable source is skipped.
pub fn discover(sources: &[PathBuf]) -> (Vec<Skill>, Vec<Shadowed>) {
    let mut found: Vec<Skill> = Vec::new();
    let mut shadowed = Vec::new();
    let mut seen_sources: Vec<&PathBuf> = Vec::new();
    let mut by_name: BTreeMap<String, PathBuf> = BTreeMap::new();
    for source in sources {
        if seen_sources.contains(&source) {
            continue;
        }
        seen_sources.push(source);
        let Ok(entries) = fs::read_dir(source) else {
            continue;
        };
        let mut names: Vec<String> = entries
            .filter_map(Result::ok)
            .filter(|e| e.path().join("SKILL.md").is_file())
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        for name in names {
            let dir = source.join(&name);
            match by_name.get(&name) {
                Some(kept) => shadowed.push(Shadowed {
                    name,
                    kept: kept.clone(),
                    dropped: dir,
                }),
                None => {
                    by_name.insert(name.clone(), dir.clone());
                    found.push(Skill {
                        name,
                        dir,
                        source: source.clone(),
                    });
                }
            }
        }
    }
    (found, shadowed)
}
