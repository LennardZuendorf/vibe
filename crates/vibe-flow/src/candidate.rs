//! Content identity for tracked and untracked candidate bytes. Records remain untrusted claims.
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

/// Git object digest of a length-framed snapshot, without creating an object or commit.
pub fn digest(root: &Path, bytes: &[u8]) -> Result<String, String> {
    let mut child = Command::new("git")
        .current_dir(root)
        .args(["hash-object", "--stdin"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| e.to_string())?;
    child
        .stdin
        .take()
        .ok_or("digest stdin unavailable")?
        .write_all(bytes)
        .map_err(|e| e.to_string())?;
    let output = child.wait_with_output().map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Err("git candidate digest failed".into());
    }
    let hash = String::from_utf8(output.stdout).map_err(|e| e.to_string())?;
    let hash = hash.trim();
    if !matches!(hash.len(), 40 | 64) || !hash.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("invalid candidate digest output".into());
    }
    Ok(format!("git-object:{hash}"))
}
fn frame(out: &mut Vec<u8>, bytes: &[u8]) {
    out.extend_from_slice(&(bytes.len() as u64).to_be_bytes());
    out.extend_from_slice(bytes);
}
#[cfg(unix)]
fn path(bytes: &[u8]) -> Result<PathBuf, String> {
    use std::os::unix::ffi::OsStrExt;
    Ok(PathBuf::from(std::ffi::OsStr::from_bytes(bytes)))
}
#[cfg(not(unix))]
fn path(bytes: &[u8]) -> Result<PathBuf, String> {
    String::from_utf8(bytes.to_vec())
        .map(PathBuf::from)
        .map_err(|e| e.to_string())
}
/// Include tracked deletions, file modes, symlink targets and every untracked path, including ignored paths.
/// Only the declared `.vibe/run/flow` and Cargo `target` output trees are excluded.
pub fn snapshot(root: &Path) -> Result<String, String> {
    let top = Command::new("git")
        .current_dir(root)
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .map_err(|e| e.to_string())?;
    if !top.status.success() {
        return Err("candidate requires a Git worktree".into());
    }
    let top = String::from_utf8(top.stdout).map_err(|e| e.to_string())?;
    if fs::canonicalize(top.trim()).map_err(|e| e.to_string())?
        != fs::canonicalize(root).map_err(|e| e.to_string())?
    {
        return Err("candidate root must be worktree root".into());
    }
    let output = Command::new("git")
        .current_dir(root)
        .args(["ls-files", "--cached", "--others", "-z"])
        .output()
        .map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Err("cannot enumerate candidate files".into());
    }
    let mut names: Vec<&[u8]> = output
        .stdout
        .split(|b| *b == 0)
        .filter(|p| !p.is_empty())
        .collect();
    names.sort_unstable();
    names.dedup();
    let mut bytes = b"flow-candidate-v1\0".to_vec();
    for name in names {
        let rel = path(name)?;
        if rel.starts_with(".vibe/run/flow") || rel.starts_with("target") {
            continue;
        }
        if rel.is_absolute()
            || rel
                .components()
                .any(|c| matches!(c, std::path::Component::ParentDir))
        {
            return Err("unsafe candidate path".into());
        }
        let full = root.join(&rel);
        frame(&mut bytes, name);
        // Do not traverse a symlinked directory or read outside the worktree.
        for parent in rel
            .ancestors()
            .skip(1)
            .filter(|p| !p.as_os_str().is_empty())
        {
            match fs::symlink_metadata(root.join(parent)) {
                Ok(m) if !m.is_dir() => return Err("candidate ancestor is not a directory".into()),
                Err(e) if e.kind() != std::io::ErrorKind::NotFound => return Err(e.to_string()),
                _ => {}
            }
        }
        let meta = match fs::symlink_metadata(&full) {
            Ok(m) => m,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                frame(&mut bytes, b"deleted");
                continue;
            }
            Err(e) => return Err(e.to_string()),
        };
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            frame(
                &mut bytes,
                &(meta.permissions().mode() & 0o777).to_be_bytes(),
            );
        }
        if meta.file_type().is_symlink() {
            frame(&mut bytes, b"symlink");
            let target = fs::read_link(&full).map_err(|e| e.to_string())?;
            #[cfg(unix)]
            {
                use std::os::unix::ffi::OsStrExt;
                frame(&mut bytes, target.as_os_str().as_bytes());
            }
            #[cfg(not(unix))]
            frame(&mut bytes, target.to_string_lossy().as_bytes());
        } else if meta.is_file() {
            frame(&mut bytes, b"file");
            frame(&mut bytes, &fs::read(&full).map_err(|e| e.to_string())?);
        } else {
            return Err(format!("unsupported candidate entry {}", rel.display()));
        }
    }
    digest(root, &bytes)
}
