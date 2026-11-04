//! Advisory lock files with explicit ownership and conservative recovery.
use crate::{Error, Result};
use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::io::{ErrorKind, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{
    Mutex,
    atomic::{AtomicU64, Ordering},
    mpsc,
};
use std::thread::{self, JoinHandle};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Heartbeat freshness window. Age alone never establishes that a holder died.
pub const STALE: Duration = Duration::from_secs(60);
static NEXT: AtomicU64 = AtomicU64::new(0);
static HELD: Mutex<BTreeMap<PathBuf, Vec<String>>> = Mutex::new(BTreeMap::new());

/// Outcome of [`Lock::acquire`].
#[derive(Debug)]
pub enum Acquired {
    /// The lock is held until the value drops.
    Got(Lock),
    /// Another holder exists; its death has not been established.
    Busy,
}

/// A held lock. Only its exact ownership token may refresh or remove the file.
#[derive(Debug)]
pub struct Lock {
    path: PathBuf,
    token: String,
    stop: mpsc::Sender<()>,
    heartbeat: Option<JoinHandle<()>>,
}

fn ours(path: &Path, token: &str) -> bool {
    fs::symlink_metadata(path).is_ok_and(|m| m.is_file())
        && fs::read_to_string(path).is_ok_and(|text| text == token)
}

fn touch(path: &Path, token: &str) {
    if !fs::symlink_metadata(path).is_ok_and(|m| m.is_file()) {
        return;
    }
    if let Ok(mut file) = OpenOptions::new().read(true).write(true).open(path) {
        let mut text = String::new();
        if file.read_to_string(&mut text).is_ok() && text == token {
            let _ = file.set_modified(SystemTime::now());
        }
    }
}

/// Verifies the active in-process holder before a protected state write.
/// Unlocked callers are left unchanged; command entry points acquire before planning.
pub fn verify_path(path: &Path) -> std::result::Result<(), String> {
    let held = HELD
        .lock()
        .map_err(|_| "lock ownership registry poisoned".to_string())?;
    if let Some(tokens) = held.get(path)
        && (tokens.len() != 1 || !ours(path, &tokens[0]))
    {
        return Err(format!(
            "{}: lock ownership lost; state not written; inspect the holder before recovery",
            path.display()
        ));
    }
    Ok(())
}

impl Lock {
    /// Refreshes this holder only, leaving replacements untouched.
    pub fn refresh(&self) {
        touch(&self.path, &self.token);
    }
}

/// Refreshes only the exact registered owner, never another lock with the same pid.
pub fn refresh_path(path: &Path) {
    if let Ok(held) = HELD.lock()
        && let Some(tokens) = held.get(path)
        && tokens.len() == 1
    {
        touch(path, &tokens[0]);
    }
}

impl Drop for Lock {
    fn drop(&mut self) {
        let _ = self.stop.send(());
        if let Some(heartbeat) = self.heartbeat.take() {
            let _ = heartbeat.join();
        }
        // Retire ownership before exposing the file; a successor registers under this mutex.
        if let Ok(mut held) = HELD.lock() {
            if let Some(tokens) = held.get_mut(&self.path) {
                tokens.retain(|t| t != &self.token);
                if tokens.is_empty() {
                    held.remove(&self.path);
                }
            }
            if ours(&self.path, &self.token) {
                let _ = fs::remove_file(&self.path);
            }
        }
        #[cfg(test)]
        tests::after_release(&self.path);
    }
}

fn io(path: &Path, source: std::io::Error) -> Error {
    Error::Io {
        path: path.to_path_buf(),
        source,
    }
}

impl Lock {
    /// Acquires exclusively. An existing lock is retained regardless of age: std on the
    /// supported MSRV cannot prove process death portably without a subprocess or unsafe
    /// code. After independently verifying its holder stopped, remove the abandoned file
    /// and retry. A heartbeat keeps old clients from treating blocking work as abandoned.
    pub fn acquire(path: &Path, _stale: Duration) -> Result<Acquired> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| io(parent, e))?;
        }
        let mut file = match OpenOptions::new().write(true).create_new(true).open(path) {
            Ok(f) => f,
            Err(e)
                if matches!(
                    e.kind(),
                    ErrorKind::AlreadyExists | ErrorKind::PermissionDenied
                ) =>
            {
                return Ok(Acquired::Busy);
            }
            Err(e) => return Err(io(path, e)),
        };
        let token = format!(
            "{}:{}:{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        );
        if let Err(e) = file.write_all(token.as_bytes()) {
            let _ = fs::remove_file(path);
            return Err(io(path, e));
        }
        let (stop, receive) = mpsc::channel();
        let (heartbeat_path, heartbeat_token) = (path.to_path_buf(), token.clone());
        let heartbeat = thread::spawn(move || {
            while receive.recv_timeout(STALE / 4) == Err(mpsc::RecvTimeoutError::Timeout) {
                touch(&heartbeat_path, &heartbeat_token);
            }
        });
        if let Ok(mut held) = HELD.lock() {
            held.entry(path.to_path_buf())
                .or_default()
                .push(token.clone());
        }
        Ok(Acquired::Got(Lock {
            path: path.to_path_buf(),
            token,
            stop,
            heartbeat: Some(heartbeat),
        }))
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;
    use std::fs::{self, File};
    use std::time::SystemTime;

    struct ReleaseHook {
        path: PathBuf,
        released: mpsc::SyncSender<()>,
        resume: mpsc::Receiver<()>,
    }

    static RELEASE_HOOK: Mutex<Option<ReleaseHook>> = Mutex::new(None);

    // Only the selected fixture pauses; unrelated parallel lock tests keep running.
    pub(super) fn after_release(path: &Path) {
        let hook = {
            let mut hook = RELEASE_HOOK.lock().unwrap();
            if hook.as_ref().is_some_and(|hook| hook.path == path) {
                hook.take()
            } else {
                None
            }
        };
        if let Some(hook) = hook {
            hook.released.send(()).unwrap();
            hook.resume.recv().unwrap();
        }
    }

    // global-store Task 2: a real successor verifies while its predecessor's Drop is paused.
    #[test]
    fn concurrent_successor_verifies_before_predecessor_drop_returns() {
        let d = dir("successor-handoff");
        let p = d.join("x.lock");
        let Acquired::Got(first) = Lock::acquire(&p, STALE).unwrap() else {
            panic!("first acquisition")
        };
        let (released, released_rx) = mpsc::sync_channel(0);
        let (resume, resume_rx) = mpsc::sync_channel(0);
        *RELEASE_HOOK.lock().unwrap() = Some(ReleaseHook {
            path: p.clone(),
            released,
            resume: resume_rx,
        });
        let predecessor = thread::spawn(move || drop(first));
        released_rx.recv_timeout(Duration::from_secs(10)).unwrap();
        assert!(!p.exists(), "predecessor must expose the filesystem lock");
        let successor = Lock::acquire(&p, STALE).unwrap();
        let verified = verify_path(&p);
        // Resume and join before asserting so a failing control leaves no blocked worker.
        resume.send(()).unwrap();
        predecessor.join().unwrap();
        assert!(
            matches!(successor, Acquired::Got(_)),
            "successor must acquire the real lock"
        );
        drop(successor);
        assert!(!p.exists(), "successor must release its own lock");
        let _ = fs::remove_dir_all(&d);
        assert!(
            verified.is_ok(),
            "legitimate successor rejected: {verified:?}"
        );
    }

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

    // global-store Task 2: age alone never authorizes stealing a live holder.
    #[test]
    fn old_live_holder_stays_exclusive_without_refresh() {
        let d = dir("old-live");
        let p = d.join("x.lock");
        let Acquired::Got(held) = Lock::acquire(&p, STALE).unwrap() else {
            panic!("first")
        };
        age(&p, 120);
        assert!(matches!(Lock::acquire(&p, STALE).unwrap(), Acquired::Busy));
        drop(held);
        assert!(!p.exists());
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn drop_does_not_remove_replacement_with_same_pid() {
        let d = dir("same-pid");
        let p = d.join("x.lock");
        let Acquired::Got(held) = Lock::acquire(&p, STALE).unwrap() else {
            panic!("first")
        };
        fs::remove_file(&p).unwrap();
        fs::write(&p, std::process::id().to_string()).unwrap();
        age(&p, 120);
        held.refresh();
        refresh_path(&p);
        assert!(
            SystemTime::now()
                .duration_since(fs::metadata(&p).unwrap().modified().unwrap())
                .unwrap()
                > Duration::from_secs(100)
        );
        assert!(verify_path(&p).is_err());
        drop(held);
        assert!(p.exists());
        let _ = fs::remove_dir_all(&d);
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
    fn stale_unknown_holder_requires_verified_manual_recovery() {
        let d = dir("stale");
        fs::create_dir_all(&d).unwrap();
        let p = d.join("x.lock");
        fs::write(&p, "999999999").unwrap();
        age(&p, 120);
        assert!(matches!(Lock::acquire(&p, STALE).unwrap(), Acquired::Busy));
        assert_eq!(fs::read_to_string(&p).unwrap(), "999999999");
        // Operator independently confirmed the fixture owner does not exist.
        fs::remove_file(&p).unwrap();
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
    fn old_foreign_lock_is_not_changed_by_competing_acquirers() {
        let d = dir("race");
        fs::create_dir_all(&d).unwrap();
        let p = d.join("x.lock");
        fs::write(&p, "999999999").unwrap();
        age(&p, 120);
        let handles: Vec<_> = (0..2)
            .map(|_| {
                let p = p.clone();
                std::thread::spawn(move || Lock::acquire(&p, STALE).unwrap())
            })
            .collect();
        for handle in handles {
            assert!(matches!(handle.join().unwrap(), Acquired::Busy));
        }
        assert_eq!(fs::read_to_string(&p).unwrap(), "999999999");
        let _ = fs::remove_dir_all(&d);
    }
}
