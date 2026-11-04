//! Exclusively reserved scratch space, reclaimed by the caller's scope.
//!
//! Keep the returned [`Scratch`] alive while any derived path is in use. Its private
//! container and ownership marker stay outside the visible workspace. Drop removes
//! only that container after checking its identity, workspace and marker. It never
//! scans siblings: age, a PID or a `vibe-*` name cannot prove ownership or inactivity.
//! Aborted processes can leave data behind; reclaiming it requires separate proof.
//! Cleanup assumes a stable filesystem during its final path checks and removal.
//! Windows handles deny replacement while held, then close immediately before
//! removal; this is not a defense against a hostile actor racing that final step.
use std::fs;
use std::io::{self, Write};
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::SystemTime;

static NEXT: AtomicUsize = AtomicUsize::new(0);
const MARKER: &str = ".scratch-owner";

/// Owns one scratch workspace. Derived `PathBuf`s do not retain this owner.
#[derive(Debug)]
#[must_use = "keep the scratch owner alive while using its paths"]
pub struct Scratch {
    path: PathBuf,
    container: PathBuf,
    container_meta: fs::Metadata,
    workspace_meta: fs::Metadata,
    marker: String,
    // Keep the original inode allocated even if its name is removed or replaced.
    #[cfg(unix)]
    _container_handle: fs::File,
    #[cfg(unix)]
    _workspace_handle: fs::File,
    #[cfg(windows)]
    directory_handles: Option<(fs::File, fs::File)>,
}

impl Deref for Scratch {
    type Target = Path;
    fn deref(&self) -> &Path {
        &self.path
    }
}

impl AsRef<Path> for Scratch {
    fn as_ref(&self) -> &Path {
        &self.path
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        #[cfg(windows)]
        if self.directory_handles.is_none() {
            return;
        }
        let unchanged = |path: &Path, original: &fs::Metadata| {
            fs::symlink_metadata(path)
                .is_ok_and(|meta| meta.is_dir() && same_directory(&meta, original))
        };
        if !self
            .container
            .parent()
            .is_some_and(|parent| fs::canonicalize(parent).is_ok_and(|real| plain(real) == parent))
        {
            return;
        }
        let workspace_owned_or_gone = match fs::symlink_metadata(&self.path) {
            Ok(meta) => meta.is_dir() && same_directory(&meta, &self.workspace_meta),
            Err(err) => err.kind() == io::ErrorKind::NotFound,
        };
        if !unchanged(&self.container, &self.container_meta) || !workspace_owned_or_gone {
            return;
        }
        let marker = self.container.join(MARKER);
        let Ok(meta) = fs::symlink_metadata(&marker) else {
            return;
        };
        if !meta.is_file() || meta.len() != self.marker.len() as u64 {
            return;
        }
        if fs::read_to_string(marker).is_ok_and(|text| text == self.marker) {
            // Windows cannot delete the directories while these handles deny delete.
            #[cfg(windows)]
            drop(self.directory_handles.take());
            let _ = fs::remove_dir_all(&self.container);
        }
    }
}

fn same_directory(a: &fs::Metadata, b: &fs::Metadata) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        a.dev() == b.dev() && a.ino() == b.ino()
    }
    #[cfg(windows)]
    {
        // The original directories cannot be replaced while our handles deny delete.
        let _ = (a, b);
        true
    }
    #[cfg(not(any(unix, windows)))]
    {
        // No supported directory identity primitive: never authorize deletion.
        let _ = (a, b);
        false
    }
}

#[cfg(windows)]
fn hold_directory(path: &Path) -> io::Result<fs::File> {
    use std::os::windows::fs::OpenOptionsExt;
    // Request data-read access so Windows tracks the sharing hold. Metadata-only
    // access_mode(0) allowed the empty workspace to be renamed in Windows CI.
    // OpenOptionsExt (stable since 1.10) passes these through to CreateFileW.
    // https://doc.rust-lang.org/std/os/windows/fs/trait.OpenOptionsExt.html
    // https://learn.microsoft.com/windows/win32/api/fileapi/nf-fileapi-createfilew
    const FILE_SHARE_READ: u32 = 0x0000_0001;
    const FILE_SHARE_WRITE: u32 = 0x0000_0002;
    const FILE_FLAG_BACKUP_SEMANTICS: u32 = 0x0200_0000;
    fs::OpenOptions::new()
        .read(true)
        .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE)
        .custom_flags(FILE_FLAG_BACKUP_SEMANTICS)
        .open(path)
}

/// Reserves a fresh `<prefix>-<name>-<pid>-<n>/work` directory under system temp.
/// Existing paths (including files and symlinks) are never removed or reused.
/// Names must be nonempty ASCII filename fragments without separators or colons.
pub fn scratch(prefix: &str, name: &str) -> Scratch {
    try_scratch(prefix, name).unwrap_or_else(|err| panic!("create test scratch: {err}"))
}

/// Fallible scratch reservation for command-line callers that report I/O errors.
pub fn try_scratch(prefix: &str, name: &str) -> io::Result<Scratch> {
    reserve(&std::env::temp_dir(), prefix, name, &NEXT)
}

fn reserve(root: &Path, prefix: &str, name: &str, next: &AtomicUsize) -> io::Result<Scratch> {
    for fragment in [prefix, name] {
        if fragment.is_empty()
            || !fragment
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "invalid scratch name",
            ));
        }
    }
    let root = plain(fs::canonicalize(root)?);
    loop {
        let n = next.fetch_add(1, Ordering::Relaxed);
        let container = root.join(format!("{prefix}-{name}-{}-{n}", std::process::id()));
        #[cfg(unix)]
        let builder = {
            use std::os::unix::fs::DirBuilderExt;
            let mut builder = fs::DirBuilder::new();
            builder.mode(0o700);
            builder
        };
        #[cfg(not(unix))]
        let builder = fs::DirBuilder::new();
        match builder.create(&container) {
            Err(err) if err.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(err) => return Err(err),
            Ok(()) => {}
        }
        let path = container.join("work");
        // Setup failures leak only this new reservation, never remove an uncertain path.
        fs::create_dir(&path)?;
        let marker = format!(
            "scratch-v1:{}:{n}:{:?}\n",
            std::process::id(),
            SystemTime::now()
        );
        fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(container.join(MARKER))?
            .write_all(marker.as_bytes())?;
        return Ok(Scratch {
            container_meta: fs::symlink_metadata(&container)?,
            workspace_meta: fs::symlink_metadata(&path)?,
            #[cfg(unix)]
            _container_handle: fs::File::open(&container)?,
            #[cfg(unix)]
            _workspace_handle: fs::File::open(&path)?,
            #[cfg(windows)]
            directory_handles: Some((hold_directory(&container)?, hold_directory(&path)?)),
            container,
            path,
            marker,
        });
    }
}

/// Match CLI paths on Windows, where canonicalize adds a verbatim prefix.
fn plain(path: PathBuf) -> PathBuf {
    match path.to_str().and_then(|s| s.strip_prefix(r"\\?\")) {
        Some(rest) if !rest.starts_with(r"UNC\") => PathBuf::from(rest),
        _ => path,
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;
    use std::time::Duration;

    // Every destructive test is nested in a separately owned dedicated root.
    fn root(name: &str) -> Scratch {
        scratch("vibe-testscratch-unit", name)
    }

    fn local(root: &Path, next: &AtomicUsize) -> Scratch {
        reserve(root, "vibe-case", "owned", next).unwrap()
    }

    // PR66: exclusive allocation must leave colliding data unchanged.
    #[test]
    fn regression_allocation_preserves_a_collision() {
        let root = root("collision");
        let collision = root.join(format!("vibe-case-owned-{}-0", std::process::id()));
        fs::create_dir(&collision).unwrap();
        fs::write(collision.join("keep"), "foreign").unwrap();
        let file_collision = root.join(format!("vibe-case-owned-{}-1", std::process::id()));
        fs::write(&file_collision, "foreign file").unwrap();
        let allocated = local(&root, &AtomicUsize::new(0));
        assert_ne!(
            allocated.container, collision,
            "must retry an existing directory"
        );
        drop(allocated);
        assert_eq!(fs::read_to_string(file_collision).unwrap(), "foreign file");
        assert_eq!(
            fs::read_to_string(collision.join("keep")).unwrap(),
            "foreign"
        );
    }

    // PR66: a completed scope reclaims its own data without a later process.
    #[test]
    fn regression_completed_scope_reclaims_owned_scratch() {
        let root = root("scope");
        let allocated = local(&root, &AtomicUsize::new(0));
        let path = allocated.container.clone();
        fs::write(allocated.join("data"), "owned").unwrap();
        drop(allocated);
        assert!(
            !path.exists(),
            "owned scratch must be removed at scope exit"
        );
    }

    // PR66: no global scan means foreign entries are untouched at every age.
    #[test]
    fn regression_age_never_authorizes_foreign_deletion() {
        let root = root("foreign");
        let names = [
            "vibe-release-backup",
            "vibe-parity-999999",
            "vibe-test-live-42-0",
        ];
        for name in names {
            fs::create_dir(root.join(name)).unwrap();
        }
        let old = SystemTime::UNIX_EPOCH + Duration::from_secs(1);
        let future = SystemTime::now() + Duration::from_secs(24 * 3600);
        for (name, modified) in [("vibe-release-file", old), ("vibe-future-file", future)] {
            let file = fs::File::create(root.join(name)).unwrap();
            file.set_modified(modified).unwrap();
        }
        fs::write(
            root.join("vibe-parity-999999").join(MARKER),
            "scratch-v1:999999:0\n",
        )
        .unwrap();
        #[cfg(unix)]
        for name in names {
            let path = root.join(name);
            fs::File::open(&path).unwrap().set_modified(old).unwrap();
            assert_eq!(fs::metadata(path).unwrap().modified().unwrap(), old);
        }
        drop(local(&root, &AtomicUsize::new(0)));
        for name in names {
            assert!(root.join(name).is_dir());
        }
        for name in ["vibe-release-file", "vibe-future-file"] {
            assert!(root.join(name).is_file());
        }
    }

    #[test]
    fn current_and_foreign_live_owners_survive_other_scope_cleanup() {
        let root = root("live");
        let current = local(&root, &AtomicUsize::new(0));
        // Spawn this test binary in a separate process, without invoking a host CLI.
        let mut child = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "testscratch::tests::foreign_owner_process",
                "--ignored",
            ])
            .env("VIBE_SCRATCH_TEST_ROOT", &*root)
            .spawn()
            .unwrap();
        let ready = root.join("ready");
        let deadline = std::time::Instant::now() + Duration::from_secs(10);
        while !ready.exists() {
            assert!(
                std::time::Instant::now() < deadline,
                "child failed to start"
            );
            std::thread::sleep(Duration::from_millis(10));
        }
        let foreign = PathBuf::from(fs::read_to_string(&ready).unwrap());
        #[cfg(unix)]
        for container in [&current.container, foreign.parent().unwrap()] {
            fs::File::open(container)
                .unwrap()
                .set_modified(SystemTime::UNIX_EPOCH + Duration::from_secs(1))
                .unwrap();
        }
        drop(local(&root, &AtomicUsize::new(10)));
        assert!(current.is_dir());
        assert!(foreign.is_dir());
        fs::write(root.join("finish"), "").unwrap();
        assert!(child.wait().unwrap().success());
        assert!(!foreign.exists(), "foreign owner cleans its own scope");
        assert!(current.is_dir());
    }

    #[test]
    #[ignore = "subprocess fixture for the live owner test"]
    fn foreign_owner_process() {
        let root = PathBuf::from(std::env::var_os("VIBE_SCRATCH_TEST_ROOT").unwrap());
        let owned = local(&root, &AtomicUsize::new(0));
        fs::write(root.join("ready"), owned.to_string_lossy().as_bytes()).unwrap();
        if std::env::var_os("VIBE_SCRATCH_TEST_ABANDON").is_some() {
            std::mem::forget(owned);
            return;
        }
        let deadline = std::time::Instant::now() + Duration::from_secs(10);
        while !root.join("finish").exists() {
            assert!(
                std::time::Instant::now() < deadline,
                "parent failed to finish"
            );
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    #[test]
    fn unwinding_reclaims_owned_data() {
        let root = root("unwind");
        let mut container = PathBuf::new();
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let owned = local(&root, &AtomicUsize::new(0));
            container = owned.container.clone();
            panic!("exercise scope unwind");
        }));
        assert!(!container.exists());
    }

    #[cfg(unix)]
    #[test]
    fn replaced_directory_with_copied_marker_is_preserved() {
        let root = root("replacement");
        let owned = local(&root, &AtomicUsize::new(0));
        let container = owned.container.clone();
        let moved = root.join("moved");
        fs::rename(&container, &moved).unwrap();
        fs::create_dir(&container).unwrap();
        fs::create_dir(container.join("work")).unwrap();
        fs::write(container.join(MARKER), &owned.marker).unwrap();
        drop(owned);
        assert!(container.is_dir());
        assert!(moved.is_dir());
    }

    #[cfg(unix)]
    #[test]
    fn changed_workspace_is_preserved() {
        let root = root("workspace-replacement");
        let owned = local(&root, &AtomicUsize::new(0));
        let container = owned.container.clone();
        fs::rename(&*owned, root.join("moved-work")).unwrap();
        fs::create_dir(&*owned).unwrap();
        drop(owned);
        assert!(container.is_dir());
    }

    #[test]
    fn missing_malformed_and_unreadable_markers_are_preserved() {
        let root = root("marker");
        for variant in ["missing", "foreign", "directory", "invalid-utf8"] {
            let owned = reserve(&root, "vibe-marker", variant, &AtomicUsize::new(0)).unwrap();
            let container = owned.container.clone();
            let marker = container.join(MARKER);
            fs::remove_file(&marker).unwrap();
            match variant {
                "missing" => {}
                "directory" => fs::create_dir(marker).unwrap(),
                "foreign" => fs::write(marker, "foreign owner").unwrap(),
                _ => fs::write(marker, vec![0xff; owned.marker.len()]).unwrap(),
            }
            drop(owned);
            assert!(container.exists(), "{variant}");
        }
    }

    #[cfg(unix)]
    #[test]
    fn symlink_collisions_and_replaced_leaves_are_preserved() {
        use std::os::unix::fs::symlink;
        let root = root("symlink");
        let foreign = root.join("foreign");
        fs::create_dir(&foreign).unwrap();
        fs::write(foreign.join("keep"), "foreign").unwrap();
        let collision = root.join(format!("vibe-case-owned-{}-0", std::process::id()));
        symlink(&foreign, &collision).unwrap();
        let owned = local(&root, &AtomicUsize::new(0));
        let container = owned.container.clone();
        fs::rename(&container, root.join("moved")).unwrap();
        symlink(&foreign, &container).unwrap();
        drop(owned);
        assert!(
            fs::symlink_metadata(&container)
                .unwrap()
                .file_type()
                .is_symlink()
        );
        assert!(
            fs::symlink_metadata(&collision)
                .unwrap()
                .file_type()
                .is_symlink()
        );
        assert_eq!(fs::read_to_string(foreign.join("keep")).unwrap(), "foreign");
    }

    #[test]
    fn rejects_names_that_can_leave_the_root() {
        let root = root("names");
        for invalid in [
            "",
            "../outside",
            "/absolute",
            r"..\outside",
            "C:outside",
            "a/b",
        ] {
            for (prefix, name) in [(invalid, "safe"), ("safe", invalid)] {
                assert_eq!(
                    reserve(&root, prefix, name, &AtomicUsize::new(0))
                        .unwrap_err()
                        .kind(),
                    io::ErrorKind::InvalidInput
                );
            }
        }
        assert_eq!(fs::read_dir(&*root).unwrap().count(), 0);
    }

    #[test]
    fn abandoned_foreign_ownership_is_not_a_cleanup_grant() {
        let root = root("abandoned");
        let status = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "testscratch::tests::foreign_owner_process",
                "--ignored",
            ])
            .env("VIBE_SCRATCH_TEST_ROOT", &*root)
            .env("VIBE_SCRATCH_TEST_ABANDON", "1")
            .status()
            .unwrap();
        assert!(status.success());
        let abandoned = PathBuf::from(fs::read_to_string(root.join("ready")).unwrap());
        #[cfg(unix)]
        fs::File::open(&abandoned)
            .unwrap()
            .set_modified(SystemTime::UNIX_EPOCH + Duration::from_secs(1))
            .unwrap();
        drop(local(&root, &AtomicUsize::new(20)));
        assert!(
            abandoned.is_dir(),
            "dead owner and age still do not grant this scope ownership"
        );
    }

    #[test]
    fn owned_scratch_is_reclaimed_even_with_a_future_mtime() {
        let root = root("future-owned");
        let owned = local(&root, &AtomicUsize::new(0));
        let container = owned.container.clone();
        #[cfg(unix)]
        fs::File::open(&container)
            .unwrap()
            .set_modified(SystemTime::now() + Duration::from_secs(24 * 3600))
            .unwrap();
        drop(owned);
        assert!(!container.exists());
    }

    #[cfg(unix)]
    #[test]
    fn unreadable_marker_is_preserved() {
        use std::os::unix::fs::PermissionsExt;
        let root = root("unreadable");
        let owned = local(&root, &AtomicUsize::new(0));
        let container = owned.container.clone();
        let marker = container.join(MARKER);
        fs::set_permissions(&marker, fs::Permissions::from_mode(0o0)).unwrap();
        assert!(
            fs::read(&marker).is_err(),
            "fixture requires unprivileged execution"
        );
        drop(owned);
        assert!(container.exists());
    }

    #[cfg(unix)]
    #[test]
    fn symlinked_ancestor_and_marker_are_preserved() {
        use std::os::unix::fs::symlink;
        let root = root("symlink-ancestor");
        let anchor = root.join("anchor");
        fs::create_dir(&anchor).unwrap();
        let owned = local(&anchor, &AtomicUsize::new(0));
        let moved = root.join("moved-anchor");
        fs::rename(&anchor, &moved).unwrap();
        symlink(&moved, &anchor).unwrap();
        let container_name = owned.container.file_name().unwrap().to_os_string();
        drop(owned);
        assert!(moved.join(container_name).is_dir());
        let owned = local(&root, &AtomicUsize::new(10));
        let container = owned.container.clone();
        let foreign_marker = root.join("foreign-marker");
        fs::write(&foreign_marker, &owned.marker).unwrap();
        fs::remove_file(container.join(MARKER)).unwrap();
        symlink(&foreign_marker, container.join(MARKER)).unwrap();
        drop(owned);
        assert!(container.is_dir());
    }

    #[cfg(unix)]
    #[test]
    fn deleting_owned_tree_does_not_follow_nested_symlinks() {
        use std::os::unix::fs::symlink;
        let root = root("nested-link");
        let foreign = root.join("foreign");
        fs::create_dir(&foreign).unwrap();
        fs::write(foreign.join("keep"), "foreign").unwrap();
        let owned = local(&root, &AtomicUsize::new(0));
        symlink(&foreign, owned.join("link")).unwrap();
        drop(owned);
        assert_eq!(fs::read_to_string(foreign.join("keep")).unwrap(), "foreign");
    }

    #[cfg(unix)]
    #[test]
    fn already_removed_workspace_does_not_leak_its_container() {
        let root = root("removed-work");
        let owned = local(&root, &AtomicUsize::new(0));
        let container = owned.container.clone();
        fs::remove_dir_all(&*owned).unwrap();
        drop(owned);
        assert!(!container.exists());
    }

    #[cfg(unix)]
    #[test]
    fn unreadable_workspace_metadata_is_preserved() {
        use std::os::unix::fs::PermissionsExt;
        let root = root("unreadable-meta");
        let owned = local(&root, &AtomicUsize::new(0));
        let container = owned.container.clone();
        fs::set_permissions(&container, fs::Permissions::from_mode(0o0)).unwrap();
        assert!(
            fs::symlink_metadata(&*owned).is_err(),
            "fixture requires unprivileged execution"
        );
        drop(owned);
        assert!(container.exists());
        fs::set_permissions(&container, fs::Permissions::from_mode(0o700)).unwrap();
    }

    #[cfg(windows)]
    #[test]
    fn windows_handles_deny_replacement_until_owned_cleanup() {
        let root = root("windows-handles");
        let owned = local(&root, &AtomicUsize::new(0));
        let container = owned.container.clone();
        assert!(fs::rename(&container, root.join("moved-container")).is_err());
        assert!(fs::rename(&*owned, root.join("moved-workspace")).is_err());
        assert!(fs::remove_dir(&*owned).is_err());
        fs::write(owned.join("data"), "owned").unwrap();
        drop(owned);
        assert!(
            !container.exists(),
            "drop releases handles before removing owned data"
        );
    }

    #[cfg(windows)]
    #[test]
    fn windows_directory_hold_pins_an_empty_directory_until_closed() {
        let root = root("windows-empty-hold");
        let original = root.join("original");
        let moved = root.join("moved");
        fs::create_dir(&original).unwrap();
        let hold = hold_directory(&original).unwrap();
        assert!(fs::rename(&original, &moved).is_err());
        assert!(original.is_dir());
        assert!(!moved.exists());
        drop(hold);
        fs::rename(&original, &moved).unwrap();
        assert!(moved.is_dir());
    }

    #[cfg(windows)]
    #[test]
    fn windows_missing_directory_holds_preserve_replaced_workspace() {
        let root = root("windows-missing-hold");
        let mut owned = local(&root, &AtomicUsize::new(0));
        let container = owned.container.clone();
        drop(owned.directory_handles.take());
        fs::rename(&*owned, root.join("original-workspace")).unwrap();
        fs::create_dir(&*owned).unwrap();
        fs::write(owned.join("keep"), "foreign").unwrap();
        drop(owned);
        assert_eq!(
            fs::read_to_string(container.join("work/keep")).unwrap(),
            "foreign"
        );
    }
}
