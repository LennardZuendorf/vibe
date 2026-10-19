//! `vibe-spec init` (P4-R11): seeds a layered spec root. The target is
//! `--to`, else the discovered root, else `<project>/docs/spec`. A file that
//! exists is never touched (`SKIP:`); a symlink anywhere on the target path is
//! refused; the writes are all-or-nothing.
//!
//! `init` is a redesign, not a port of the oracle-only `setup.sh`.
use std::fs;
use std::io::Write;
use std::path::{Component, Path, PathBuf};

use vibe_core::exit;

use crate::root::{Miss, Root};

/// The seeded files, in order. `{{today}}` is replaced by the UTC date.
const FILES: [(&str, &str); 7] = [
    ("README.md", include_str!("templates/init/README.md")),
    ("AGENTS.md", include_str!("templates/init/AGENTS.md")),
    ("product.md", include_str!("templates/init/product.md")),
    ("tech.md", include_str!("templates/init/tech.md")),
    ("design.md", include_str!("templates/init/design.md")),
    ("plan.md", include_str!("templates/init/plan.md")),
    ("lessons.md", include_str!("templates/init/lessons.md")),
];

/// One file to create.
struct Planned {
    path: PathBuf,
    label: String,
    bytes: Vec<u8>,
}

/// Runs the command.
pub fn run(found: &Result<Root, Miss>, cwd: &Path, to: Option<&Path>, json: bool) -> (u8, Vec<u8>) {
    let project = normalise(&match found {
        Ok(root) => root.project.clone(),
        Err(_) => crate::git::top(cwd).unwrap_or_else(|| cwd.to_path_buf()),
    });
    let dir = match (to, found) {
        (Some(to), _) => normalise(&cwd.join(to)),
        (None, Ok(root)) => normalise(&root.dir),
        (None, Err(Miss::NotFound)) => project.join("docs").join("spec"),
        (None, Err(Miss::Missing { path, .. })) => normalise(path),
        (None, Err(miss)) => {
            eprintln!("{}", miss.message());
            return (miss.code(), Vec::new());
        }
    };
    let Ok(rel_dir) = dir.strip_prefix(&project) else {
        eprintln!(
            "vibe-spec init: {} is outside the project {}; not writing there",
            dir.display(),
            project.display()
        );
        return (exit::FINDINGS, Vec::new());
    };
    if rel_dir.as_os_str().is_empty() {
        eprintln!("vibe-spec init: the spec root cannot be the project directory itself");
        return (exit::FINDINGS, Vec::new());
    }
    let root_label = slash(rel_dir);

    let today = crate::text::today_utc();
    let mut errors: Vec<String> = Vec::new();
    let mut plan: Vec<Planned> = Vec::new();
    let mut skipped: Vec<String> = Vec::new();
    for (name, template) in FILES {
        let path = dir.join(name);
        let label = format!("{root_label}/{name}");
        match inspect(&project, &path, &label) {
            Ok(true) => skipped.push(label),
            Ok(false) => plan.push(Planned {
                path,
                label,
                bytes: template.replace("{{today}}", &today).into_bytes(),
            }),
            Err(e) => errors.push(e),
        }
    }
    if !errors.is_empty() {
        for e in &errors {
            eprintln!("vibe-spec init: {e}");
        }
        return (exit::FINDINGS, Vec::new());
    }

    let created = match commit(&plan) {
        Ok(skipped_late) => {
            let mut created = Vec::new();
            for p in &plan {
                if skipped_late.contains(&p.label) {
                    skipped.push(p.label.clone());
                } else {
                    created.push(p.label.clone());
                }
            }
            created
        }
        Err(msg) => {
            eprintln!("vibe-spec init: {msg}");
            return (exit::FINDINGS, Vec::new());
        }
    };
    skipped.sort_by_key(|l| {
        FILES
            .iter()
            .position(|(n, _)| l.ends_with(&format!("/{n}")))
    });
    if json {
        let v = serde_json::json!({
            "v": 1,
            "root": root_label,
            "created": created,
            "skipped": skipped,
        });
        return (exit::OK, format!("{v}\n").into_bytes());
    }
    let mut out = String::new();
    for l in &created {
        out.push_str(&format!("OK: {l}\n"));
    }
    for l in &skipped {
        out.push_str(&format!("SKIP: {l}\n"));
    }
    (exit::OK, out.into_bytes())
}

/// `/`-joined path text.
pub(crate) fn slash(path: &Path) -> String {
    path.components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/")
}

/// Drops `.` and resolves `..` lexically.
pub(crate) fn normalise(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for c in path.components() {
        match c {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            other => out.push(other),
        }
    }
    out
}

/// Walks `project` → `path` without following links. `Ok(true)`: a regular
/// file is already there. `Ok(false)`: the file (and maybe some directories)
/// are absent. An error: a symlink or a non-directory on the way, or a
/// non-regular file at the end.
fn inspect(project: &Path, path: &Path, label: &str) -> Result<bool, String> {
    let rel = path
        .strip_prefix(project)
        .map_err(|_| format!("{label}: outside the project"))?;
    let parts: Vec<_> = rel.components().collect();
    let mut cur = project.to_path_buf();
    for (i, part) in parts.iter().enumerate() {
        cur.push(part);
        let shown = slash(cur.strip_prefix(project).unwrap_or(&cur));
        match fs::symlink_metadata(&cur) {
            Ok(m) if m.file_type().is_symlink() => {
                return Err(format!(
                    "{label}: {shown} is a symlink; not writing through it"
                ));
            }
            Ok(m) if i + 1 < parts.len() && !m.is_dir() => {
                return Err(format!("{label}: {shown} is not a directory"));
            }
            Ok(m) if i + 1 == parts.len() => {
                return if m.is_file() {
                    Ok(true)
                } else {
                    Err(format!("{label}: exists and is not a regular file"))
                };
            }
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(false),
            Err(e) => return Err(format!("{label}: {e}")),
        }
    }
    Ok(false)
}

/// Creates every planned file; a file that appeared since planning is left
/// alone and returned by label. If one write fails, everything this run
/// created is removed and the error is returned.
fn commit(plan: &[Planned]) -> Result<Vec<String>, String> {
    let mut files: Vec<&Path> = Vec::new();
    let mut dirs: Vec<PathBuf> = Vec::new();
    let mut late: Vec<String> = Vec::new();
    for p in plan {
        match create(p, &mut dirs) {
            Ok(true) => files.push(&p.path),
            Ok(false) => late.push(p.label.clone()),
            Err(e) => {
                let mut msg = format!("ERROR: {}: {e}", p.label);
                for f in files.iter().rev() {
                    if let Err(err) = fs::remove_file(f) {
                        msg.push_str(&format!(" — cleanup failed for {}: {err}", f.display()));
                    }
                }
                for d in dirs.iter().rev() {
                    let _ = fs::remove_dir(d);
                }
                msg.push_str(" — removed what this run created");
                return Err(msg);
            }
        }
    }
    Ok(late)
}

/// `Ok(true)`: created; `Ok(false)`: the file already exists.
fn create(p: &Planned, made: &mut Vec<PathBuf>) -> Result<bool, String> {
    if let Some(parent) = p.path.parent() {
        let mut missing = Vec::new();
        let mut cur = parent;
        while !cur.exists() {
            missing.push(cur.to_path_buf());
            match cur.parent() {
                Some(up) => cur = up,
                None => break,
            }
        }
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        missing.reverse();
        made.extend(missing);
    }
    let mut file = match fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&p.path)
    {
        Ok(f) => f,
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
            return if fs::symlink_metadata(&p.path).is_ok_and(|m| m.is_file()) {
                Ok(false)
            } else {
                Err("exists and is not a regular file".to_string())
            };
        }
        Err(e) => return Err(e.to_string()),
    };
    if let Err(e) = file.write_all(&p.bytes).and_then(|()| file.flush()) {
        // The file is ours (create_new): drop the half-written one.
        let _ = fs::remove_file(&p.path);
        return Err(e.to_string());
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    #[test]
    fn a_failing_write_removes_everything_created() {
        let base = std::env::temp_dir().join(format!("vibe-spec-init-unit-{}", std::process::id()));
        let _ = fs::remove_dir_all(&base);
        fs::create_dir_all(&base).unwrap();
        fs::write(base.join("blocker"), "x").unwrap();
        let item = |rel: &str| Planned {
            path: base.join(rel),
            label: rel.to_string(),
            bytes: b"x".to_vec(),
        };
        // The third file needs `blocker/` as a directory: it fails.
        let plan = [item("d/a.md"), item("d/e/b.md"), item("blocker/c.md")];
        let err = commit(&plan).unwrap_err();
        assert!(err.contains("blocker/c.md"), "{err}");
        assert!(!base.join("d").exists(), "created directories remain");
        assert_eq!(fs::read(base.join("blocker")).unwrap(), b"x");
        fs::remove_dir_all(&base).unwrap();
    }
}
