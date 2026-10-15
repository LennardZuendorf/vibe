//! Staging a corpus case into a sandbox and snapshotting the result.
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

/// Where the oracle is staged inside a sandbox.
pub const ORACLE_DIR: &str = ".oracle";
/// An empty directory at the sandbox root. It bounds marker-based upward
/// searches (a script that stops where a `.git` entry exists). It does not
/// bound git's own repository discovery: an empty `.git/` is not a
/// repository, so `git` keeps walking up past the sandbox.
pub const GIT_DIR: &str = ".git";

fn err(path: &Path, e: std::io::Error) -> String {
    format!("{}: {e}", path.display())
}

/// Errors when `path` is itself a symlink. A missing path is fine.
pub fn refuse_symlink(path: &Path) -> Result<(), String> {
    match fs::symlink_metadata(path) {
        Ok(meta) if meta.file_type().is_symlink() => Err(format!(
            "{}: symlinks are not allowed in a corpus or oracle",
            path.display()
        )),
        _ => Ok(()),
    }
}

/// Recreates `dest` as a copy of `tree` (if it exists), plus the oracle at
/// `dest/.oracle/` and an empty `dest/.git/`.
pub fn stage(tree: &Path, oracle_root: &Path, dest: &Path) -> Result<(), String> {
    if dest.exists() {
        fs::remove_dir_all(dest).map_err(|e| err(dest, e))?;
    }
    fs::create_dir_all(dest).map_err(|e| err(dest, e))?;
    refuse_symlink(tree)?;
    if tree.is_dir() {
        copy_dir(tree, dest)?;
    }
    refuse_symlink(oracle_root)?;
    copy_dir(oracle_root, &dest.join(ORACLE_DIR))?;
    let git = dest.join(GIT_DIR);
    fs::create_dir_all(&git).map_err(|e| err(&git, e))
}

/// Copies the directory `from` into `to` (created if missing). Symlinks are refused.
pub fn copy_dir(from: &Path, to: &Path) -> Result<(), String> {
    fs::create_dir_all(to).map_err(|e| err(to, e))?;
    for entry in fs::read_dir(from).map_err(|e| err(from, e))? {
        let src = entry.map_err(|e| err(from, e))?.path();
        let Some(name) = src.file_name() else {
            continue;
        };
        let dst = to.join(name);
        let kind = fs::symlink_metadata(&src)
            .map_err(|e| err(&src, e))?
            .file_type();
        if kind.is_symlink() {
            return Err(format!(
                "{}: symlinks are not allowed in a corpus or oracle",
                src.display()
            ));
        } else if kind.is_dir() {
            copy_dir(&src, &dst)?;
        } else {
            fs::copy(&src, &dst).map_err(|e| err(&src, e))?;
        }
    }
    Ok(())
}

/// Maps every file and directory under `root`, except `.oracle/` and `.git/`
/// at the top, to its bytes. Keys are relative paths joined with `/`; a
/// directory's key ends in `/` and maps to no bytes, so an empty directory
/// left behind is a difference too.
pub fn snapshot(root: &Path) -> Result<BTreeMap<String, Vec<u8>>, String> {
    let mut files = BTreeMap::new();
    walk(root, "", &mut files)?;
    Ok(files)
}

fn walk(dir: &Path, prefix: &str, files: &mut BTreeMap<String, Vec<u8>>) -> Result<(), String> {
    for entry in fs::read_dir(dir).map_err(|e| err(dir, e))? {
        let path = entry.map_err(|e| err(dir, e))?.path();
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        if prefix.is_empty() && (name == ORACLE_DIR || name == GIT_DIR) {
            continue;
        }
        let rel = if prefix.is_empty() {
            name
        } else {
            format!("{prefix}/{name}")
        };
        let kind = fs::symlink_metadata(&path)
            .map_err(|e| err(&path, e))?
            .file_type();
        if kind.is_dir() {
            files.insert(format!("{rel}/"), Vec::new());
            walk(&path, &rel, files)?;
        } else {
            let bytes = if kind.is_symlink() {
                fs::read_link(&path)
                    .map_err(|e| err(&path, e))?
                    .to_string_lossy()
                    .into_owned()
                    .into_bytes()
            } else {
                fs::read(&path).map_err(|e| err(&path, e))?
            };
            files.insert(rel, bytes);
        }
    }
    Ok(())
}
