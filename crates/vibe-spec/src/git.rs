//! `git` as a subprocess (no `git2`). Every function degrades to "nothing"
//! when git is missing, the directory is outside a work tree, or the
//! repository has no commits. A shallow clone yields no dates either: git
//! reports every file as added in the boundary commit.
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;

use crate::text::lossy;

/// Git for Windows does not accept Win32 verbatim prefixes in path operands.
pub(crate) fn argument_path(path: &Path) -> PathBuf {
    #[cfg(windows)]
    {
        use std::path::{Component, Prefix};
        if let Some(Component::Prefix(prefix)) = path.components().next() {
            let mut plain = match prefix.kind() {
                Prefix::VerbatimDisk(drive) => PathBuf::from(format!("{}:", char::from(drive))),
                Prefix::VerbatimUNC(server, share) => {
                    let mut name = std::ffi::OsString::from(r"\\");
                    name.push(server);
                    name.push(r"\");
                    name.push(share);
                    PathBuf::from(name)
                }
                _ => return path.to_path_buf(),
            };
            for component in path.components().skip(1) {
                plain.push(component.as_os_str());
            }
            return plain;
        }
    }
    path.to_path_buf()
}

/// `git -C dir --literal-pathspecs <args>` stdout when it exits 0.
pub(crate) fn git(cx: &crate::api::Execution<'_>, dir: &Path, args: &[&str]) -> Option<Vec<u8>> {
    let out = cx
        .command("git")
        .arg("-C")
        .arg(argument_path(dir))
        .arg("--literal-pathspecs")
        .args(args)
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    out.status.success().then_some(out.stdout)
}

/// The top directory of the work tree holding `dir`.
pub fn top(cx: &crate::api::Execution<'_>, dir: &Path) -> Option<PathBuf> {
    let out = git(cx, dir, &["rev-parse", "--show-toplevel"])?;
    output_path(&out)
}

pub(crate) fn output_path(out: &[u8]) -> Option<PathBuf> {
    let bytes = out.strip_suffix(b"\n").unwrap_or(out);
    if bytes.is_empty() {
        return None;
    }
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        Some(PathBuf::from(std::ffi::OsStr::from_bytes(bytes)))
    }
    #[cfg(not(unix))]
    {
        std::str::from_utf8(bytes).ok().map(PathBuf::from)
    }
}

/// Git locators from the caller must never retarget an explicitly selected tree.
pub(crate) fn context_runtime(runtime: &crate::Runtime) -> crate::Runtime {
    let mut clean = runtime.clone();
    clean.env.retain(|key, _| {
        let key = key.to_string_lossy();
        let key = if cfg!(windows) {
            key.to_ascii_uppercase()
        } else {
            key.into_owned()
        };
        !matches!(
            key.as_str(),
            "GIT_DIR"
                | "GIT_WORK_TREE"
                | "GIT_COMMON_DIR"
                | "GIT_INDEX_FILE"
                | "GIT_OBJECT_DIRECTORY"
                | "GIT_ALTERNATE_OBJECT_DIRECTORIES"
                | "GIT_CEILING_DIRECTORIES"
                | "GIT_DISCOVERY_ACROSS_FILESYSTEM"
                | "GIT_NAMESPACE"
                | "GIT_PREFIX"
                | "GIT_CONFIG"
                | "GIT_CONFIG_COUNT"
                | "GIT_CONFIG_PARAMETERS"
                | "GIT_CONFIG_GLOBAL"
                | "GIT_CONFIG_SYSTEM"
        ) && !key.starts_with("GIT_CONFIG_KEY_")
            && !key.starts_with("GIT_CONFIG_VALUE_")
    });
    clean.env.insert("GIT_OPTIONAL_LOCKS".into(), "0".into());
    clean
}

/// True when the repository holding `dir` is a shallow clone.
pub fn shallow(cx: &crate::api::Execution<'_>, dir: &Path) -> bool {
    git(cx, dir, &["rev-parse", "--is-shallow-repository"]).is_some_and(|b| b.starts_with(b"true"))
}

/// For each file under `under` (top-relative, `/`-joined, `.` for the
/// top), the committer date `YYYY-MM-DD` of the last commit touching it.
pub fn last_dates(
    cx: &crate::api::Execution<'_>,
    top: &Path,
    under: &str,
) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    if shallow(cx, top) {
        return out;
    }
    // Fails without commits: nothing to report.
    let Some(log) = git(
        cx,
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
pub fn commits(
    cx: &crate::api::Execution<'_>,
    dir: &Path,
    since: Option<&str>,
) -> Result<Option<Vec<Commit>>, String> {
    if let Some(s) = since
        && (s.is_empty() || s.starts_with('-'))
    {
        return Err(format!("--since '{s}' is not a revision"));
    }
    let Some(top) =
        top(cx, dir).filter(|t| git(cx, t, &["rev-parse", "--verify", "-q", "HEAD"]).is_some())
    else {
        return match since {
            Some(_) => Err("--since needs git history".to_string()),
            None => Ok(None),
        };
    };
    if since.is_none() && shallow(cx, &top) {
        return Ok(None);
    }
    let range = match since {
        Some(s) => {
            if git(
                cx,
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
    // Only this project's history: the repository may hold more.
    let scope = std::fs::canonicalize(dir)
        .ok()
        .and_then(|d| d.strip_prefix(&top).ok().map(Path::to_path_buf))
        .map(|rel| {
            rel.components()
                .map(|c| c.as_os_str().to_string_lossy().into_owned())
                .collect::<Vec<_>>()
                .join("/")
        })
        .filter(|rel| !rel.is_empty());
    // A pathspec prunes merge and empty commits, so the top takes none.
    let mut args = vec!["log", "-z", "--format=%H%n%B", &range];
    if let Some(scope) = &scope {
        args.extend(["--", scope]);
    }
    let log = git(cx, &top, &args).ok_or_else(|| format!("git log {range} failed"))?;
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

/// The commit this branch forked from (P3c-R13): `merge-base HEAD <base>`,
/// where `<base>` is `explicit` or the first of `origin/HEAD`,
/// `origin/main`, `main`, `master` that names a commit. None without git,
/// history or a base, and in a shallow clone.
pub fn branch_base(
    cx: &crate::api::Execution<'_>,
    dir: &Path,
    explicit: Option<&str>,
) -> Option<String> {
    // A shallow history ends at an arbitrary boundary: no trustworthy base.
    if shallow(cx, dir) {
        return None;
    }
    let candidates: Vec<&str> = match explicit {
        Some(b) => vec![b],
        None => vec!["origin/HEAD", "origin/main", "main", "master"],
    };
    let base = candidates.into_iter().find(|c| {
        !c.starts_with('-')
            && git(
                cx,
                dir,
                &["rev-parse", "--verify", "-q", &format!("{c}^{{commit}}")],
            )
            .is_some()
    })?;
    let out = git(cx, dir, &["merge-base", "HEAD", base])?;
    let hash = String::from_utf8_lossy(&out).trim().to_string();
    (!hash.is_empty()).then_some(hash)
}

/// Whether the tree under `dir` differs from `base`: tracked changes,
/// committed or not, or untracked files. None when git fails.
pub fn changed_since(cx: &crate::api::Execution<'_>, dir: &Path, base: &str) -> Option<bool> {
    let diff = git(cx, dir, &["diff", "--name-only", base, "--", "."])?;
    if !diff.is_empty() {
        return Some(true);
    }
    let untracked = git(
        cx,
        dir,
        &["ls-files", "--others", "--exclude-standard", "--", "."],
    )?;
    Some(!untracked.is_empty())
}

/// Staged, unstaged and untracked files in the work tree at `top`
/// (`git status --porcelain=v1 -z --untracked-files=all`), top-relative and
/// `/`-joined. A rename lists its new path. None when git fails.
pub fn changed_files(cx: &crate::api::Execution<'_>, top: &Path) -> Option<Vec<String>> {
    let out = git(
        cx,
        top,
        &["status", "--porcelain=v1", "-z", "--untracked-files=all"],
    )?;
    let mut files = Vec::new();
    let mut tokens = out.split(|&b| b == 0).filter(|t| !t.is_empty());
    while let Some(entry) = tokens.next() {
        let (Some(status), Some(name)) = (entry.get(..2), entry.get(3..)) else {
            continue;
        };
        if status.iter().any(|&c| c == b'R' || c == b'C') {
            // The next token is the rename's source.
            tokens.next();
        }
        files.push(lossy(name));
    }
    Some(files)
}

#[cfg(test)]
mod context_tests {
    use super::*;
    #[cfg(windows)]
    #[test]
    fn spec_ci_portability_git_paths_preserve_native_units() {
        use std::os::windows::ffi::{OsStrExt, OsStringExt};
        for (input, expected) in [
            (r"\\?\C:\managed\history.git", r"C:\managed\history.git"),
            (
                r"\\?\UNC\server\share\history.git",
                r"\\server\share\history.git",
            ),
            (r"C:\managed\history.git", r"C:\managed\history.git"),
            (r"relative\history.git", r"relative\history.git"),
        ] {
            assert_eq!(argument_path(Path::new(input)), PathBuf::from(expected));
        }
        let mut native: Vec<u16> = r"\\?\C:\managed\".encode_utf16().collect();
        native.push(0xd800);
        let input = PathBuf::from(std::ffi::OsString::from_wide(&native));
        let actual: Vec<u16> = argument_path(&input).as_os_str().encode_wide().collect();
        assert_eq!(actual, native[4..]);
    }
    #[test]
    fn new_domain_git_configuration_omits_ambient_core_worktree_injection() {
        // spec-projects-mcp/2: no ambient config override reaches domain Git children.
        let runtime = crate::Runtime {
            cwd: PathBuf::from("fixture"),
            env: BTreeMap::from([
                ("GIT_CONFIG_COUNT".into(), "1".into()),
                ("GIT_CONFIG_KEY_0".into(), "core.worktree".into()),
                ("GIT_CONFIG_VALUE_0".into(), "outside-checkout".into()),
                (
                    "GIT_CONFIG_PARAMETERS".into(),
                    "'core.worktree=outside-checkout'".into(),
                ),
                ("GIT_CONFIG_GLOBAL".into(), "outside-config".into()),
                ("GIT_CONFIG_SYSTEM".into(), "outside-config".into()),
                ("GIT_CONFIG".into(), "outside-config".into()),
                ("HOME".into(), "fixture-home".into()),
                ("GIT_CONFIG_NOSYSTEM".into(), "1".into()),
            ]),
        };
        let clean = context_runtime(&runtime);
        assert_eq!(
            clean.env,
            BTreeMap::from([
                ("HOME".into(), "fixture-home".into()),
                ("GIT_CONFIG_NOSYSTEM".into(), "1".into()),
                ("GIT_OPTIONAL_LOCKS".into(), "0".into())
            ])
        );
        assert_eq!(runtime.env.len(), 9);
    }
}
