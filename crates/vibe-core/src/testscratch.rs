//! Scratch space for test runs: created under the system temp dir and reaped, not leaked.
//!
//! Every `cargo test` process used to leave one directory per test case in `TMPDIR`
//! forever — a day of worker runs left 14 500 dead directories and 4.3 GiB on a disk with
//! 5.7 GiB free, because nothing outside Hermes's 24 h pruner ever removed them. A test
//! helper cannot delete its own directory at exit without an `atexit` hook (banned by
//! `unsafe_code = "forbid"`), so the first scratch use in a process sweeps the *stale*
//! siblings earlier processes left behind. Accumulation is then bounded by [`MAX_IDLE`]
//! instead of by how long the machine stays up.
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Once;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, SystemTime};

/// Scratch entries idle for longer than this are swept. No test binary runs this long, so
/// a live run's directories are never candidates; a hung one loses its scratch, which is
/// the price of bounded growth.
pub const MAX_IDLE: Duration = Duration::from_secs(6 * 60 * 60);

static SWEEP: Once = Once::new();
static NEXT: AtomicUsize = AtomicUsize::new(0);

/// A fresh, unique scratch directory named `<prefix>-<name>-<pid>-<n>`.
///
/// The name carries the pid so two processes never collide; the pid is also what keeps a
/// concurrent run's entries out of [`sweep_stale`].
pub fn scratch(prefix: &str, name: &str) -> PathBuf {
    SWEEP.call_once(|| {
        sweep_stale(&std::env::temp_dir(), SystemTime::now(), MAX_IDLE);
    });
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("{prefix}-{name}-{}-{n}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    if let Err(err) = fs::create_dir_all(&dir) {
        panic!("create test scratch {}: {err}", dir.display());
    }
    dir
}

/// Deletes the top-level `vibe-*` entries of `root` idle for at least `max_idle` as of
/// `now`, and returns how many went. This process's own entries are kept regardless of
/// age, and an entry whose mtime cannot be read (or is in the future) is kept too: an
/// incomplete scan cannot establish that it is dead.
pub fn sweep_stale(root: &Path, now: SystemTime, max_idle: Duration) -> usize {
    let own = format!("-{}-", std::process::id());
    let own_tail = format!("-{}", std::process::id());
    let entries = match fs::read_dir(root) {
        Ok(entries) => entries,
        Err(_) => return 0,
    };
    let mut removed = 0;
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if !name.starts_with("vibe-") || name.contains(&own) || name.ends_with(&own_tail) {
            continue;
        }
        let Ok(meta) = entry.metadata() else { continue };
        let Ok(mtime) = meta.modified() else { continue };
        let Ok(idle) = now.duration_since(mtime) else {
            continue;
        };
        if idle < max_idle {
            continue;
        }
        let path = entry.path();
        let gone = if meta.is_dir() {
            fs::remove_dir_all(&path).is_ok()
        } else {
            fs::remove_file(&path).is_ok()
        };
        if gone {
            removed += 1;
        }
    }
    removed
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::{MAX_IDLE, scratch, sweep_stale};
    use std::fs;
    use std::path::PathBuf;
    use std::time::{Duration, SystemTime};

    /// A root of its own, so a sweep here cannot touch a real run's scratch.
    fn root(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "vibe-testscratch-unit-{name}-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn names(dir: &PathBuf) -> Vec<String> {
        let mut names: Vec<String> = fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }

    #[test]
    fn sweeps_entries_past_the_idle_window_only() {
        let dir = root("window");
        fs::create_dir_all(dir.join("vibe-stale-a")).unwrap();
        fs::create_dir_all(dir.join("vibe-stale-b")).unwrap();
        // Nothing is 6 h old yet, so a real-window sweep leaves both.
        assert_eq!(sweep_stale(&dir, SystemTime::now(), MAX_IDLE), 0);
        // Far enough in the future, both are old.
        let later = SystemTime::now() + MAX_IDLE + Duration::from_secs(1);
        assert_eq!(sweep_stale(&dir, later, MAX_IDLE), 2);
        assert!(names(&dir).is_empty(), "left {:?}", names(&dir));
    }

    #[test]
    fn keeps_entries_this_process_owns() {
        let dir = root("own");
        let mine = format!("vibe-own-{}-0", std::process::id());
        fs::create_dir_all(dir.join(&mine)).unwrap();
        let later = SystemTime::now() + Duration::from_secs(24 * 3600);
        assert_eq!(sweep_stale(&dir, later, Duration::ZERO), 0);
        assert_eq!(names(&dir), vec![mine]);
    }

    #[test]
    fn keeps_entries_outside_the_vibe_prefix() {
        let dir = root("foreign");
        fs::create_dir_all(dir.join("tmp.not-ours")).unwrap();
        fs::create_dir_all(dir.join("vibe-old")).unwrap();
        let later = SystemTime::now() + Duration::from_secs(24 * 3600);
        assert_eq!(sweep_stale(&dir, later, Duration::ZERO), 1);
        assert_eq!(names(&dir), vec!["tmp.not-ours".to_string()]);
    }

    #[test]
    fn scratch_is_unique_and_inside_the_temp_dir() {
        let a = scratch("vibe-testscratch-unit-a", "case");
        let b = scratch("vibe-testscratch-unit-a", "case");
        assert_ne!(a, b);
        assert!(a.starts_with(std::env::temp_dir()));
        assert!(a.is_dir());
        let _ = fs::remove_dir_all(&a);
        let _ = fs::remove_dir_all(&b);
    }
}
