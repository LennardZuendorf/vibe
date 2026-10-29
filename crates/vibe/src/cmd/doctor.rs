//! `vibe doctor`: harness CLIs, store, state, lock and recorded hook binaries.
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use vibe_core::dirs::Env;
use vibe_core::lock::STALE;

use super::common;
use crate::paths::Paths;
use crate::{state, store};

/// Looks `name` up in the directories of `path_var`.
pub fn find_on_path(name: &str, path_var: Option<&str>) -> Option<PathBuf> {
    let dirs = std::env::split_paths(path_var?).collect::<Vec<_>>();
    let exts: &[&str] = if cfg!(windows) {
        &["", ".exe", ".cmd", ".bat"]
    } else {
        &[""]
    };
    dirs.iter()
        .filter(|d| !d.as_os_str().is_empty())
        .flat_map(|d| exts.iter().map(move |e| d.join(format!("{name}{e}"))))
        .find(|p| is_executable(p))
}

#[cfg(unix)]
fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    fs::metadata(path).is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
}

#[cfg(not(unix))]
fn is_executable(path: &Path) -> bool {
    path.is_file()
}

/// The program path of a recorded hook command: the quoted first token.
fn hook_binary(command: &str) -> Option<&str> {
    let rest = command.strip_prefix('"')?;
    rest.split_once('"').map(|(bin, _)| bin)
}

fn lock_line(path: &Path) -> String {
    let Ok(meta) = fs::metadata(path) else {
        return "lock free".to_string();
    };
    let age = meta
        .modified()
        .ok()
        .and_then(|m| SystemTime::now().duration_since(m).ok());
    if age.is_some_and(|a| a > STALE) {
        return "lock stale".to_string();
    }
    let pid = fs::read_to_string(path).unwrap_or_default();
    format!("lock held by {}", pid.trim())
}

/// Runs `vibe doctor` against the process environment. Exit 1 when a `problem` line is printed.
pub fn doctor(paths: &Paths, out: &mut dyn Write, _err: &mut dyn Write) -> u8 {
    let env = Env::from_process();
    let path_var = env.var("PATH");
    for name in ["claude", "codex", "opencode", "droid", "vibe-spec"] {
        match find_on_path(name, path_var) {
            Some(p) => {
                let _ = writeln!(out, "found {name} {}", p.display());
            }
            None => {
                let _ = writeln!(out, "missing {name}");
            }
        }
    }
    let mut problems = 0;
    match store::load(paths) {
        Ok(_) => {
            let _ = writeln!(out, "store ok");
        }
        Err(e) => {
            problems += 1;
            let _ = writeln!(out, "problem store: {}", common::one_line(&e));
        }
    }
    match state::load(paths) {
        Ok(s) => {
            let _ = writeln!(out, "state ok");
            for (id, cmd) in &s.owned.hooks {
                if hook_binary(cmd).is_some_and(|b| !Path::new(b).exists()) {
                    problems += 1;
                    let _ = writeln!(
                        out,
                        "problem {id}: hook binary missing \u{2014} run `vibe install`"
                    );
                }
            }
        }
        Err(e) => {
            problems += 1;
            let _ = writeln!(out, "problem state: {}", common::one_line(&e));
        }
    }
    let _ = writeln!(out, "{}", lock_line(&paths.lock_file()));
    u8::from(problems > 0)
}
