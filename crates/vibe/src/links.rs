//! Symlink helpers.
use std::fs;
use std::path::Path;

/// Creates the symlink `path` pointing at `target`, creating missing parent directories.
pub fn create_link(path: &Path, target: &Path) -> Result<(), String> {
    #[cfg(unix)]
    {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
        }
        std::os::unix::fs::symlink(target, path).map_err(|e| format!("{}: {e}", path.display()))
    }
    #[cfg(not(unix))]
    {
        let _ = (path, target);
        Err("links unsupported on this OS".to_string())
    }
}

/// True when `path` is a symlink whose target is exactly `target`.
pub fn is_link_to(path: &Path, target: &Path) -> bool {
    fs::symlink_metadata(path).is_ok_and(|m| m.file_type().is_symlink())
        && fs::read_link(path).is_ok_and(|t| t == target)
}
