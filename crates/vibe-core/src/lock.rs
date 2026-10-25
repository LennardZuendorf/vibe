//! Advisory lock files with stale takeover.
use crate::{Error, Result};
use std::fs::{self, OpenOptions};
use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

/// Age after which a lock file counts as abandoned.
pub const STALE: Duration = Duration::from_secs(60);

/// Outcome of [`Lock::acquire`].
#[derive(Debug)]
pub enum Acquired {
    /// The lock is held until the value drops.
    Got(Lock),
    /// Another holder has a live lock.
    Busy,
}

/// A held lock file. Dropping it removes the file, unless another process took the lock over.
#[derive(Debug)]
pub struct Lock {
    path: PathBuf,
}

fn holds_our_pid(path: &Path) -> bool {
    fs::read_to_string(path).is_ok_and(|text| text.trim() == std::process::id().to_string())
}

impl Lock {
    /// Marks the lock alive by touching its modification time, so a long run is not taken
    /// for abandoned. Does nothing when the file no longer holds this process's pid.
    pub fn refresh(&self) {
        refresh_path(&self.path);
    }
}

/// Touches the lock file at `path` when it holds this process's pid; any other file is left alone.
pub fn refresh_path(path: &Path) {
    if holds_our_pid(path)
        && let Ok(file) = OpenOptions::new().write(true).open(path)
    {
        let _ = file.set_modified(SystemTime::now());
    }
}

impl Drop for Lock {
    fn drop(&mut self) {
        if holds_our_pid(&self.path) {
            let _ = fs::remove_file(&self.path);
        }
    }
}

struct Guard(PathBuf);

impl Drop for Guard {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

fn io(path: &Path, source: std::io::Error) -> Error {
    Error::Io {
        path: path.to_path_buf(),
        source,
    }
}

fn is_stale(path: &Path, stale: Duration) -> std::io::Result<bool> {
    let modified = fs::metadata(path)?.modified()?;
    Ok(SystemTime::now()
        .duration_since(modified)
        .is_ok_and(|age| age > stale))
}

/// Creates `path` exclusively. `None` means it exists or is pending deletion.
fn create(path: &Path) -> std::io::Result<Option<std::fs::File>> {
    match OpenOptions::new().write(true).create_new(true).open(path) {
        Ok(file) => Ok(Some(file)),
        Err(e)
            if matches!(
                e.kind(),
                ErrorKind::AlreadyExists | ErrorKind::PermissionDenied
            ) =>
        {
            Ok(None)
        }
        Err(e) => Err(e),
    }
}

fn create_lock(path: &Path) -> Result<Option<Lock>> {
    Ok(create(path).map_err(|e| io(path, e))?.map(|mut file| {
        let _ = write!(file, "{}", std::process::id());
        Lock {
            path: path.to_path_buf(),
        }
    }))
}

fn take_guard(guard: &Path, stale: Duration) -> Result<Option<Guard>> {
    if create(guard).map_err(|e| io(guard, e))?.is_some() {
        return Ok(Some(Guard(guard.to_path_buf())));
    }
    if !is_stale(guard, stale).unwrap_or(false) {
        return Ok(None);
    }
    let _ = fs::remove_file(guard);
    Ok(create(guard)
        .map_err(|e| io(guard, e))?
        .map(|_| Guard(guard.to_path_buf())))
}

impl Lock {
    /// Takes the lock at `path`, or reports [`Acquired::Busy`] when a live holder exists.
    /// A lock file older than `stale` is replaced.
    pub fn acquire(path: &Path, stale: Duration) -> Result<Acquired> {
        acquire_with(path, stale, || {})
    }
}

fn acquire_with(path: &Path, stale: Duration, before_guard: impl Fn()) -> Result<Acquired> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| io(parent, e))?;
    }
    let mut guard_name = path.as_os_str().to_owned();
    guard_name.push(".takeover");
    let guard_path = PathBuf::from(guard_name);
    for _ in 0..2 {
        if let Some(lock) = create_lock(path)? {
            return Ok(Acquired::Got(lock));
        }
        match is_stale(path, stale) {
            Ok(false) => return Ok(Acquired::Busy),
            Err(e) if e.kind() == ErrorKind::NotFound => continue,
            Err(e) => return Err(io(path, e)),
            Ok(true) => {}
        }
        before_guard();
        let Some(_guard) = take_guard(&guard_path, stale)? else {
            return Ok(Acquired::Busy);
        };
        match is_stale(path, stale) {
            Ok(true) => {
                fs::remove_file(path).map_err(|e| io(path, e))?;
            }
            Ok(false) => return Ok(Acquired::Busy),
            Err(e) if e.kind() == ErrorKind::NotFound => {}
            Err(e) => return Err(io(path, e)),
        }
        return Ok(match create_lock(path)? {
            Some(lock) => Acquired::Got(lock),
            None => Acquired::Busy,
        });
    }
    Ok(Acquired::Busy)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;
    use std::fs::{self, File};
    use std::time::SystemTime;

    fn dir(name: &str) -> PathBuf {
        let d =
            std::env::temp_dir().join(format!("vibe-core-test-lock-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        d
    }

    fn age(path: &Path, secs: u64) {
        let f = File::options().write(true).open(path).unwrap();
        f.set_modified(SystemTime::now() - Duration::from_secs(secs))
            .unwrap();
    }

    #[test]
    fn second_acquire_is_busy_until_drop() {
        let d = dir("busy");
        let p = d.join("x.lock");
        let Acquired::Got(first) = Lock::acquire(&p, STALE).unwrap() else {
            panic!("first")
        };
        assert!(matches!(Lock::acquire(&p, STALE).unwrap(), Acquired::Busy));
        drop(first);
        assert!(!p.exists());
        assert!(matches!(
            Lock::acquire(&p, STALE).unwrap(),
            Acquired::Got(_)
        ));
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn refresh_keeps_a_long_run_from_going_stale() {
        let d = dir("refresh");
        let p = d.join("x.lock");
        let Acquired::Got(held) = Lock::acquire(&p, STALE).unwrap() else {
            panic!("first")
        };
        age(&p, 120);
        held.refresh();
        assert!(matches!(Lock::acquire(&p, STALE).unwrap(), Acquired::Busy));
        age(&p, 120);
        refresh_path(&p);
        assert!(matches!(Lock::acquire(&p, STALE).unwrap(), Acquired::Busy));
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn refresh_and_drop_leave_a_foreign_lock_alone() {
        let d = dir("foreign");
        let p = d.join("x.lock");
        let Acquired::Got(held) = Lock::acquire(&p, STALE).unwrap() else {
            panic!("first")
        };
        fs::write(&p, "999999999").unwrap();
        age(&p, 120);
        held.refresh();
        let before = fs::metadata(&p).unwrap().modified().unwrap();
        assert!(SystemTime::now().duration_since(before).unwrap() > Duration::from_secs(100));
        drop(held);
        assert_eq!(fs::read_to_string(&p).unwrap(), "999999999");
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn creates_missing_parent_dir() {
        let d = dir("parent");
        let p = d.join("a/b/x.lock");
        assert!(matches!(
            Lock::acquire(&p, STALE).unwrap(),
            Acquired::Got(_)
        ));
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn stale_lock_is_taken_over() {
        let d = dir("stale");
        fs::create_dir_all(&d).unwrap();
        let p = d.join("x.lock");
        fs::write(&p, "999").unwrap();
        age(&p, 120);
        assert!(matches!(
            Lock::acquire(&p, STALE).unwrap(),
            Acquired::Got(_)
        ));
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn fresh_foreign_lock_is_busy() {
        let d = dir("fresh");
        fs::create_dir_all(&d).unwrap();
        let p = d.join("x.lock");
        fs::write(&p, "999").unwrap();
        assert!(matches!(Lock::acquire(&p, STALE).unwrap(), Acquired::Busy));
        assert!(p.exists());
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn two_stale_takers_one_wins() {
        let d = dir("race");
        fs::create_dir_all(&d).unwrap();
        let p = d.join("x.lock");
        fs::write(&p, "999").unwrap();
        age(&p, 120);
        let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
        let handles: Vec<_> = (0..2)
            .map(|_| {
                let (p, b) = (p.clone(), barrier.clone());
                std::thread::spawn(move || {
                    b.wait();
                    Lock::acquire(&p, STALE).unwrap()
                })
            })
            .collect();
        let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();
        let got = results
            .iter()
            .filter(|r| matches!(r, Acquired::Got(_)))
            .count();
        assert_eq!(got, 1);
        assert!(p.exists());
        drop(results);
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn stale_lock_with_fresh_guard_is_busy() {
        let d = dir("guard-fresh");
        fs::create_dir_all(&d).unwrap();
        let p = d.join("x.lock");
        fs::write(&p, "999").unwrap();
        age(&p, 120);
        fs::write(d.join("x.lock.takeover"), "").unwrap();
        assert!(matches!(Lock::acquire(&p, STALE).unwrap(), Acquired::Busy));
        assert!(p.exists());
        assert!(d.join("x.lock.takeover").exists());
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn stale_guard_is_recovered() {
        let d = dir("guard-stale");
        fs::create_dir_all(&d).unwrap();
        let p = d.join("x.lock");
        fs::write(&p, "999").unwrap();
        age(&p, 120);
        let g = d.join("x.lock.takeover");
        fs::write(&g, "").unwrap();
        age(&g, 120);
        assert!(matches!(
            Lock::acquire(&p, STALE).unwrap(),
            Acquired::Got(_)
        ));
        assert!(!g.exists());
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn lock_that_turns_fresh_before_the_guard_is_not_stolen() {
        let d = dir("turns-fresh");
        fs::create_dir_all(&d).unwrap();
        let p = d.join("x.lock");
        fs::write(&p, "999").unwrap();
        age(&p, 120);
        let r = acquire_with(&p, STALE, || age(&p, 0)).unwrap();
        assert!(matches!(r, Acquired::Busy));
        assert_eq!(fs::read_to_string(&p).unwrap(), "999");
        assert!(!d.join("x.lock.takeover").exists());
        let _ = fs::remove_dir_all(&d);
    }
}
