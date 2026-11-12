//! Private child-process inputs and per-invocation diagnostics.
use crate::{CommandOutput, Runtime};
use std::cell::RefCell;
#[cfg(windows)]
use std::collections::BTreeMap;
use std::ffi::{OsStr, OsString};
#[cfg(not(unix))]
use std::path::PathBuf;
use std::process::Command;

pub(crate) struct Execution<'a> {
    pub runtime: &'a Runtime,
    stderr: RefCell<Vec<u8>>,
    notices: RefCell<Vec<crate::Diagnostic>>,
}
impl<'a> Execution<'a> {
    pub fn new(runtime: &'a Runtime) -> Self {
        Self {
            runtime,
            stderr: RefCell::new(Vec::new()),
            notices: RefCell::new(Vec::new()),
        }
    }
    pub fn var_os(&self, key: impl AsRef<OsStr>) -> Option<OsString> {
        let key = key.as_ref();
        #[cfg(not(windows))]
        {
            self.runtime.env.get(key).cloned()
        }
        #[cfg(windows)]
        {
            // Command owns Windows' native Unicode, case-insensitive key comparison.
            // Build only its environment map; never spawn or touch the process env.
            // Iteration matches command().envs(), including duplicate-name precedence.
            let mut native = Command::new(OsStr::new(""));
            native.envs(&self.runtime.env);
            let before: BTreeMap<OsString, OsString> = native
                .get_envs()
                .filter_map(|(name, value)| value.map(|value| (name.to_owned(), value.to_owned())))
                .collect();
            native.env_remove(key);
            native.get_envs().find_map(|(name, value)| {
                if value.is_none() {
                    before.get(name).cloned()
                } else {
                    None
                }
            })
        }
    }
    pub fn var(&self, key: impl AsRef<OsStr>) -> Result<String, std::env::VarError> {
        self.var_os(key)
            .ok_or(std::env::VarError::NotPresent)?
            .into_string()
            .map_err(std::env::VarError::NotUnicode)
    }
    pub(crate) fn notice(&self, code: &str, args: std::fmt::Arguments<'_>) {
        let message = args.to_string();
        self.diagnostic(format_args!("{message}"));
        self.notices.borrow_mut().push(crate::Diagnostic {
            code: code.into(),
            message,
        });
    }
    pub(crate) fn notices(&self) -> Vec<crate::Diagnostic> {
        self.notices.borrow().clone()
    }
    pub fn diagnostic(&self, args: std::fmt::Arguments<'_>) {
        use std::io::Write;
        let _ = writeln!(&mut *self.stderr.borrow_mut(), "{args}");
    }
    pub fn diagnostic_bytes(&self, bytes: &[u8]) {
        self.stderr.borrow_mut().extend_from_slice(bytes);
    }
    pub fn command(&self, program: impl AsRef<OsStr>) -> Command {
        let program = program.as_ref();
        // Unix Command searches the explicit child PATH after changing directory;
        // retain its native EACCES/ENOEXEC behavior as well as successful lookup.
        #[cfg(unix)]
        let mut cmd = Command::new(program);
        // Windows executable lookup can consult the parent PATH. Resolve it first.
        #[cfg(not(unix))]
        let mut cmd = {
            let path = if std::path::Path::new(program).components().count() == 1 {
                self.find_executable(program).unwrap_or_default()
            } else {
                self.runtime.cwd.join(program)
            };
            Command::new(path)
        };
        cmd.current_dir(&self.runtime.cwd)
            .env_clear()
            .envs(&self.runtime.env);
        cmd
    }
    #[cfg(not(unix))]
    fn find_executable(&self, name: impl AsRef<OsStr>) -> Option<PathBuf> {
        let name = name.as_ref();
        let paths = self.var_os("PATH").unwrap_or_default();
        std::env::split_paths(&paths).find_map(|dir| {
            let path = self.runtime.cwd.join(dir).join(name);
            if executable(&path) {
                return Some(path);
            }
            #[cfg(windows)]
            if path.extension().is_none() {
                let exe = path.with_extension("exe");
                if executable(&exe) {
                    return Some(exe);
                }
            }
            None
        })
    }
    pub fn take_diagnostics(&self) -> Vec<u8> {
        std::mem::take(&mut *self.stderr.borrow_mut())
    }
    pub fn finish(self, exit: u8, stdout: Vec<u8>) -> CommandOutput {
        CommandOutput {
            exit,
            stdout,
            stderr: self.stderr.into_inner(),
        }
    }
}

#[cfg(unix)]
pub(crate) fn executable(path: &std::path::Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    path.metadata()
        .is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
}
#[cfg(not(unix))]
pub(crate) fn executable(path: &std::path::Path) -> bool {
    path.is_file()
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    #[test]
    fn windows_native_lookup_preserves_os_values() {
        // spec-projects-mcp/1: native key comparison must not decode OS-string values.
        use std::os::windows::ffi::OsStringExt;
        let value = OsString::from_wide(&[0xd800, b'x' as u16]);
        let runtime = Runtime {
            cwd: PathBuf::from("C:\\fixture"),
            env: BTreeMap::from([(OsString::from("vibe_spec_root"), value.clone())]),
        };
        let cx = Execution::new(&runtime);
        assert_eq!(cx.var_os("VIBE_SPEC_ROOT"), Some(value));
    }
}
