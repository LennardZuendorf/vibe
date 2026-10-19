//! Atomic file writes.
use std::ffi::{OsStr, OsString};
use std::fs::{self, File, OpenOptions};
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::{Error, Result};

type StdResult<T, E> = std::result::Result<T, E>;

static COUNTER: AtomicU64 = AtomicU64::new(0);

/// Writes `bytes` to `path` atomically.
///
/// A new temporary file is created exclusively in the same directory
/// (mode 0600 on unix), written, synced, then renamed over `path`. A
/// symlink at `path` is replaced, not followed. If a later step fails, the
/// temporary file this call created is removed and `path` is left as it
/// was; a file this call did not create is never removed.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> Result<()> {
    let name = path
        .file_name()
        .ok_or_else(|| Error::Invalid(format!("not a file path: {}", path.display())))?;
    let dir = match path.parent() {
        Some(d) if !d.as_os_str().is_empty() => d,
        _ => Path::new("."),
    };
    write_via(&tmp_path(dir, name), path, bytes)
}

/// Creates `tmp` exclusively, then writes and renames it over `path`.
/// Cleanup runs only after the create succeeded, so an existing `tmp`
/// (the create fails with `AlreadyExists`) is left untouched.
fn write_via(tmp: &Path, path: &Path, bytes: &[u8]) -> Result<()> {
    let file = create_exclusive(tmp).map_err(|source| Error::Io {
        path: tmp.to_path_buf(),
        source,
    })?;
    let result = write_then_rename(file, tmp, path, bytes);
    if result.is_err() {
        let _ = fs::remove_file(tmp);
    }
    result
}

fn write_then_rename(mut file: File, tmp: &Path, path: &Path, bytes: &[u8]) -> Result<()> {
    let io = |source| Error::Io {
        path: tmp.to_path_buf(),
        source,
    };
    file.write_all(bytes).map_err(io)?;
    file.sync_all().map_err(io)?;
    drop(file);
    fs::rename(tmp, path).map_err(|source| Error::Io {
        path: path.to_path_buf(),
        source,
    })
}

/// One planned write with the bytes to put back if a later write fails.
pub struct Write {
    /// The file to write.
    pub path: PathBuf,
    /// How errors name the file.
    pub show: String,
    /// The new content.
    pub bytes: Vec<u8>,
    /// The content to put back; `None` when the file did not exist.
    pub original: Option<Vec<u8>>,
}

/// Writes `bytes` atomically, then puts `mode` (the mode the file had) back.
/// An `Err` carries whether the file was already replaced.
fn put_staged(
    path: &Path,
    bytes: &[u8],
    mode: Option<&std::fs::Permissions>,
) -> StdResult<(), (String, bool)> {
    write_atomic(path, bytes).map_err(|e| (e.to_string(), false))?;
    match mode {
        Some(m) => fs::set_permissions(path, m.clone()).map_err(|e| (e.to_string(), true)),
        None => Ok(()),
    }
}

fn put(path: &Path, bytes: &[u8], mode: Option<&std::fs::Permissions>) -> StdResult<(), String> {
    put_staged(path, bytes, mode).map_err(|(e, _)| e)
}

/// Puts the originals back (or removes files that did not exist), in the
/// order given. Returns the names restored and one line per failure.
pub fn restore<'a>(writes: impl IntoIterator<Item = &'a Write>) -> (Vec<&'a str>, Vec<String>) {
    let mut restored = Vec::new();
    let mut failed = Vec::new();
    for w in writes {
        let mode = fs::metadata(&w.path).ok().map(|m| m.permissions());
        let back = match &w.original {
            Some(b) => put(&w.path, b, mode.as_ref()),
            None => fs::remove_file(&w.path).map_err(|e| e.to_string()),
        };
        match back {
            Ok(()) => restored.push(w.show.as_str()),
            Err(err) => failed.push(format!("{}: {err}", w.show)),
        }
    }
    (restored, failed)
}

/// Performs the writes in order. If write k fails, writes 0..k are put back
/// (or removed when the file did not exist) and the refusal is returned as
/// `<show>: <cause>`, followed by ` \u{2014} restored <names>` and one
/// ` \u{2014} restore failed for <name>: <cause>` per file that stayed changed.
pub fn commit(writes: &[Write]) -> StdResult<(), String> {
    // `write_atomic` makes a fresh file: put the original mode back on it.
    let modes: Vec<Option<std::fs::Permissions>> = writes
        .iter()
        .map(|w| fs::metadata(&w.path).ok().map(|m| m.permissions()))
        .collect();
    for (k, (w, mode)) in writes.iter().zip(&modes).enumerate() {
        let Err((e, replaced)) = put_staged(&w.path, &w.bytes, mode.as_ref()) else {
            continue;
        };
        let mut msg = format!("{}: {e}", w.show);
        // A file that was replaced but could not get its mode back is put back too.
        let (restored, failed) = restore(writes.iter().take(k + usize::from(replaced)));
        if !restored.is_empty() {
            msg.push_str(&format!(" \u{2014} restored {}", restored.join(", ")));
        }
        for f in failed {
            msg.push_str(&format!(" \u{2014} restore failed for {f}"));
        }
        return Err(msg);
    }
    Ok(())
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
    use super::{write_atomic, write_via};
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

    #[cfg(unix)]
    #[test]
    fn a_symlink_target_is_replaced_not_followed() {
        let dir = scratch("symlink");
        let keep = dir.join("keep.txt");
        let link = dir.join("link");
        fs::write(&keep, b"keep").unwrap();
        std::os::unix::fs::symlink("keep.txt", &link).unwrap();
        write_atomic(&link, b"new").unwrap();
        let meta = fs::symlink_metadata(&link).unwrap();
        assert!(meta.file_type().is_file(), "{:?}", meta.file_type());
        assert_eq!(fs::read(&link).unwrap(), b"new");
        assert_eq!(fs::read(&keep).unwrap(), b"keep");
        assert_eq!(entries(&dir), ["keep.txt", "link"]);
    }

    #[test]
    fn an_existing_temp_path_is_left_alone() {
        let dir = scratch("tmpexists");
        let tmp = dir.join(".out.txt.tmp");
        let target = dir.join("out.txt");
        fs::write(&tmp, b"someone else").unwrap();
        let err = write_via(&tmp, &target, b"x").unwrap_err();
        assert!(matches!(err, Error::Io { .. }), "{err}");
        assert_eq!(fs::read(&tmp).unwrap(), b"someone else");
        assert!(!target.exists());
    }
}
