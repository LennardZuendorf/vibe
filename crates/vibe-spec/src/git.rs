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

/// True when the repository holding `dir` is a shallow clone.
pub fn shallow(dir: &Path) -> bool {
    git(dir, &["rev-parse", "--is-shallow-repository"]).is_some_and(|b| b.starts_with(b"true"))
}

/// For each file under `under` (top-relative, `/`-joined, `.` for the
/// top), the committer date `YYYY-MM-DD` of the last commit touching it.
pub fn last_dates(top: &Path, under: &str) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    if shallow(top) {
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

/// A commit hash and its message.
pub type Commit = (String, Vec<u8>);

/// Commit hashes and messages, newest first: all of `HEAD`, or
/// `<since>..HEAD`. `Ok(None)` without git, a work tree or commits (an
/// error when `since` was given); `Err` for a `since` git cannot resolve.
/// A shallow clone without `since` also gives `Ok(None)`: its history ends
/// at an arbitrary boundary.
pub fn commits(dir: &Path, since: Option<&str>) -> Result<Option<Vec<Commit>>, String> {
    if let Some(s) = since
        && (s.is_empty() || s.starts_with('-'))
    {
        return Err(format!("--since '{s}' is not a revision"));
    }
    let Some(top) = top(dir).filter(|t| git(t, &["rev-parse", "--verify", "-q", "HEAD"]).is_some())
    else {
        return match since {
            Some(_) => Err("--since needs git history".to_string()),
            None => Ok(None),
        };
    };
    if since.is_none() && shallow(&top) {
        return Ok(None);
    }
    let range = match since {
        Some(s) => {
            if git(
                &top,
                &["rev-parse", "--verify", "-q", &format!("{s}^{{commit}}")],
            )
            .is_none()
            {
                return Err(format!("unknown revision '{s}'"));
            }
            format!("{s}..HEAD")
        }
        None => "HEAD".to_string(),
    };
    let mut args = vec!["log", "-z", "--format=%H%n%B"];
    args.push(&range);
    let log = git(&top, &args).ok_or_else(|| format!("git log {range} failed"))?;
    let mut out = Vec::new();
    for record in log.split(|&b| b == 0) {
        let record = record.strip_prefix(b"\n").unwrap_or(record);
        let (sha, body) = match record.iter().position(|&b| b == b'\n') {
            Some(n) => (
                record.get(..n).unwrap_or_default(),
                record.get(n + 1..).unwrap_or_default(),
            ),
            None => (record, &[][..]),
        };
        if !matches!(sha.len(), 40 | 64) || !sha.iter().all(u8::is_ascii_hexdigit) {
            continue;
        }
        out.push((lossy(sha), body.to_vec()));
    }
    Ok(Some(out))
}
