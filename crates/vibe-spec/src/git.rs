//! `git` as a subprocess (no `git2`). Every function degrades to "nothing"
//! when git is missing, the directory is outside a work tree, or the
//! repository has no commits. A shallow clone yields no dates either: git
//! reports every file as added in the boundary commit.
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::text::lossy;

/// `git -C dir --literal-pathspecs <args>` stdout when it exits 0.
pub(crate) fn git(dir: &Path, args: &[&str]) -> Option<Vec<u8>> {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .arg("--literal-pathspecs")
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

/// For each file under `under` (top-relative, `/`-joined, `.` for the
/// top), the committer date `YYYY-MM-DD` of the last commit touching it.
pub fn last_dates(top: &Path, under: &str) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    let shallow = git(top, &["rev-parse", "--is-shallow-repository"]);
    if shallow.is_some_and(|b| b.starts_with(b"true")) {
        return out;
    }
    // Fails without commits: nothing to report.
    let Some(log) = git(
        top,
        &["log", "-z", "--format=%x01%cs", "--name-only", "--", under],
    ) else {
        return out;
    };
    let mut date = String::new();
    for token in log.split(|&b| b == 0) {
        if let Some(d) = token.strip_prefix(b"\x01") {
            date = lossy(d);
            continue;
        }
        let name = token.strip_prefix(b"\n").unwrap_or(token);
        if !name.is_empty() && !date.is_empty() {
            out.entry(lossy(name)).or_insert_with(|| date.clone());
        }
    }
    out
}
