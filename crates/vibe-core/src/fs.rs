//! Atomic file writes.
use std::ffi::{OsStr, OsString};
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::{Error, Result};

static COUNTER: AtomicU64 = AtomicU64::new(0);

/// Writes `bytes` to `path` atomically.
///
/// A new temporary file is created exclusively in the same directory
/// (mode 0600 on unix), written, synced, then renamed over `path`. A
/// symlink at `path` is replaced, not followed. On any failure the
/// temporary file is removed and `path` is left as it was.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> Result<()> {
    let name = path
        .file_name()
        .ok_or_else(|| Error::Invalid(format!("not a file path: {}", path.display())))?;
    let dir = match path.parent() {
        Some(d) if !d.as_os_str().is_empty() => d,
        _ => Path::new("."),
    };
    let tmp = tmp_path(dir, name);
    let result = write_then_rename(&tmp, path, bytes);
    if result.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    result
}

fn write_then_rename(tmp: &Path, path: &Path, bytes: &[u8]) -> Result<()> {
    let io = |source| Error::Io {
        path: tmp.to_path_buf(),
        source,
    };
    let mut file = create_exclusive(tmp).map_err(io)?;
    file.write_all(bytes).map_err(io)?;
    file.sync_all().map_err(io)?;
    drop(file);
    fs::rename(tmp, path).map_err(|source| Error::Io {
        path: path.to_path_buf(),
        source,
    })
}

fn create_exclusive(path: &Path) -> std::io::Result<File> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options.open(path)
}

fn tmp_path(dir: &Path, name: &OsStr) -> PathBuf {
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(0);
    let mut file = OsString::from(".");
    file.push(name);
    file.push(format!(".tmp.{}.{n}.{nanos}", std::process::id()));
    dir.join(file)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::write_atomic;
    use crate::Error;
    use std::fs;
    use std::path::{Path, PathBuf};

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("vibe-core-fs-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn entries(dir: &Path) -> Vec<String> {
        let mut names: Vec<String> = fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }

    #[test]
    fn creates_a_new_file() {
        let dir = scratch("new");
        let target = dir.join("out.json");
        write_atomic(&target, b"{}\n").unwrap();
        assert_eq!(fs::read(&target).unwrap(), b"{}\n");
        assert_eq!(entries(&dir), ["out.json"]);
    }

    #[test]
    fn replaces_existing_content() {
        let dir = scratch("replace");
        let target = dir.join("out.txt");
        fs::write(&target, b"old content that is longer").unwrap();
        write_atomic(&target, b"new").unwrap();
        assert_eq!(fs::read(&target).unwrap(), b"new");
        assert_eq!(entries(&dir), ["out.txt"]);
    }

    #[cfg(unix)]
    #[test]
    fn result_is_mode_0600() {
        use std::os::unix::fs::PermissionsExt;
        let dir = scratch("mode");
        let target = dir.join("out.txt");
        fs::write(&target, b"x").unwrap();
        fs::set_permissions(&target, fs::Permissions::from_mode(0o644)).unwrap();
        write_atomic(&target, b"y").unwrap();
        let mode = fs::metadata(&target).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
    }

    #[test]
    fn a_directory_target_fails_and_leaves_no_temp_file() {
        let dir = scratch("isdir");
        let target = dir.join("sub");
        fs::create_dir(&target).unwrap();
        fs::write(target.join("keep"), b"k").unwrap();
        let err = write_atomic(&target, b"x").unwrap_err();
        assert!(matches!(err, Error::Io { .. }), "{err}");
        assert_eq!(entries(&dir), ["sub"]);
        assert_eq!(fs::read(target.join("keep")).unwrap(), b"k");
    }

    #[test]
    fn a_missing_parent_fails() {
        let dir = scratch("noparent");
        let err = write_atomic(&dir.join("absent/out.txt"), b"x").unwrap_err();
        assert!(matches!(err, Error::Io { .. }), "{err}");
        assert!(entries(&dir).is_empty());
    }

    #[test]
    fn a_path_without_a_file_name_is_invalid() {
        for p in ["", "/", ".."] {
            let err = write_atomic(Path::new(p), b"x").unwrap_err();
            assert!(matches!(err, Error::Invalid(_)), "{p:?}: {err}");
        }
    }
}
