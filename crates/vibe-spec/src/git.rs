//! `git` as a subprocess (no `git2`). Every function degrades to "nothing"
//! when git is missing, the directory is outside a work tree, or the
//! repository has no commits.
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::text::lossy;

/// `git -C dir -c core.quotepath=off <args>` stdout when it exits 0.
pub(crate) fn git(dir: &Path, args: &[&str]) -> Option<Vec<u8>> {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(["-c", "core.quotepath=off"])
        .args(args)
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    out.status.success().then_some(out.stdout)
}

/// The top directory of the work tree holding `dir`.
pub fn top(dir: &Path) -> Option<PathBuf> {
    let out = git(dir, &["rev-parse", "--show-toplevel"])?;
    let s = String::from_utf8(out).ok()?;
    let t = s.trim_end_matches('\n');
    (!t.is_empty()).then(|| PathBuf::from(t))
}

/// `HEAD` resolves.
pub fn has_commits(top: &Path) -> bool {
    git(top, &["rev-parse", "--verify", "-q", "HEAD"]).is_some()
}

/// For each file under `under` (top-relative, `/`-joined, `.` for the
/// top), the committer date `YYYY-MM-DD` of the last commit touching it.
pub fn last_dates(top: &Path, under: &str) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    if !has_commits(top) {
        return out;
    }
    let Some(log) = git(
        top,
        &["log", "--format=%x00%cs", "--name-only", "--", under],
    ) else {
        return out;
    };
    for record in log.split(|&b| b == 0).skip(1) {
        let mut rows = record.split(|&b| b == b'\n');
        let date = lossy(rows.next().unwrap_or_default());
        for name in rows.filter(|l| !l.is_empty()) {
            out.entry(lossy(name)).or_insert_with(|| date.clone());
        }
    }
    out
}
