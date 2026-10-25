//! Per-user config and state directory resolution.
use crate::{Error, Result};
use std::collections::BTreeMap;
use std::path::PathBuf;

/// A snapshot of environment variables plus the platform flavour.
#[derive(Debug, Clone)]
pub struct Env {
    vars: BTreeMap<String, String>,
    windows: bool,
}

/// Which per-user root an application directory lives under.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// Configuration root.
    Config,
    /// State root.
    State,
}

impl Env {
    /// Builds an environment from explicit variables.
    pub fn new(vars: impl IntoIterator<Item = (String, String)>, windows: bool) -> Env {
        Env {
            vars: vars.into_iter().collect(),
            windows,
        }
    }

    /// Captures the process environment, dropping non-unicode pairs.
    pub fn from_process() -> Env {
        let vars = std::env::vars_os()
            .filter_map(|(k, v)| Some((k.into_string().ok()?, v.into_string().ok()?)));
        Env::new(vars, cfg!(windows))
    }

    /// Returns a variable's value; an empty value counts as unset.
    pub fn var(&self, key: &str) -> Option<&str> {
        self.vars
            .get(key)
            .map(String::as_str)
            .filter(|v| !v.is_empty())
    }

    /// Reports whether Windows rules apply.
    pub fn is_windows(&self) -> bool {
        self.windows
    }

    /// The user's home directory.
    pub fn home(&self) -> Result<PathBuf> {
        let key = if self.windows { "USERPROFILE" } else { "HOME" };
        self.var(key)
            .map(PathBuf::from)
            .ok_or_else(|| Error::Invalid(format!("no home directory ({key} is unset)")))
    }

    /// The per-user configuration root.
    pub fn config_home(&self) -> Result<PathBuf> {
        self.root("APPDATA", "XDG_CONFIG_HOME", ".config")
    }

    /// The per-user state root.
    pub fn state_home(&self) -> Result<PathBuf> {
        self.root("LOCALAPPDATA", "XDG_STATE_HOME", ".local/state")
    }

    /// An application directory: the override variable when set, else `<root>/<app>`.
    pub fn app_dir(&self, override_var: &str, kind: Kind, app: &str) -> Result<PathBuf> {
        if let Some(path) = self.var(override_var) {
            return Ok(PathBuf::from(path));
        }
        let root = match kind {
            Kind::Config => self.config_home()?,
            Kind::State => self.state_home()?,
        };
        Ok(root.join(app))
    }

    fn root(&self, windows_var: &str, xdg_var: &str, fallback: &str) -> Result<PathBuf> {
        if self.windows {
            return self.var(windows_var).map(PathBuf::from).ok_or_else(|| {
                Error::Invalid(format!(
                    "no {windows_var} directory ({windows_var} is unset)"
                ))
            });
        }
        if let Some(path) = self
            .var(xdg_var)
            .map(PathBuf::from)
            .filter(|p| p.is_absolute())
        {
            return Ok(path);
        }
        Ok(self.home()?.join(fallback))
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;
    use std::path::PathBuf;

    fn env(vars: &[(&str, &str)], windows: bool) -> Env {
        Env::new(
            vars.iter().map(|(k, v)| (k.to_string(), v.to_string())),
            windows,
        )
    }

    #[test]
    fn home_unix_uses_home() {
        assert_eq!(
            env(&[("HOME", "/h")], false).home().unwrap(),
            PathBuf::from("/h")
        );
    }

    #[test]
    fn home_windows_uses_userprofile() {
        let e = env(&[("USERPROFILE", "C:\\U"), ("HOME", "/h")], true);
        assert_eq!(e.home().unwrap(), PathBuf::from("C:\\U"));
    }

    #[test]
    fn home_missing_or_empty_is_error() {
        let msg = env(&[], false).home().unwrap_err().to_string();
        assert_eq!(msg, "no home directory (HOME is unset)");
        assert!(env(&[("HOME", "")], false).home().is_err());
        let msg = env(&[], true).home().unwrap_err().to_string();
        assert!(msg.contains("USERPROFILE"), "{msg}");
    }

    #[test]
    fn config_home_prefers_absolute_xdg() {
        let e = env(&[("HOME", "/h"), ("XDG_CONFIG_HOME", "/x")], false);
        assert_eq!(e.config_home().unwrap(), PathBuf::from("/x"));
    }

    #[test]
    fn config_home_ignores_relative_xdg() {
        let e = env(&[("HOME", "/h"), ("XDG_CONFIG_HOME", "rel")], false);
        assert_eq!(e.config_home().unwrap(), PathBuf::from("/h/.config"));
    }

    #[test]
    fn config_home_macos_is_dot_config() {
        let e = env(&[("HOME", "/h")], false);
        assert_eq!(e.config_home().unwrap(), PathBuf::from("/h/.config"));
    }

    #[test]
    fn config_home_windows_uses_appdata() {
        let e = env(&[("APPDATA", "C:\\A"), ("XDG_CONFIG_HOME", "/x")], true);
        assert_eq!(e.config_home().unwrap(), PathBuf::from("C:\\A"));
    }

    #[test]
    fn state_home_defaults_and_xdg() {
        let e = env(&[("HOME", "/h")], false);
        assert_eq!(e.state_home().unwrap(), PathBuf::from("/h/.local/state"));
        let e = env(&[("HOME", "/h"), ("XDG_STATE_HOME", "/x")], false);
        assert_eq!(e.state_home().unwrap(), PathBuf::from("/x"));
        let e = env(&[("HOME", "/h"), ("XDG_STATE_HOME", "rel")], false);
        assert_eq!(e.state_home().unwrap(), PathBuf::from("/h/.local/state"));
    }

    #[test]
    fn state_home_windows_uses_localappdata() {
        let e = env(&[("LOCALAPPDATA", "C:\\L"), ("XDG_STATE_HOME", "/x")], true);
        assert_eq!(e.state_home().unwrap(), PathBuf::from("C:\\L"));
    }

    #[test]
    fn app_dir_override_wins() {
        let e = env(&[("HOME", "/h"), ("VIBE_STORE", "/s")], false);
        assert_eq!(
            e.app_dir("VIBE_STORE", Kind::Config, "vibe").unwrap(),
            PathBuf::from("/s")
        );
    }

    #[test]
    fn app_dir_default() {
        let e = env(&[("HOME", "/h")], false);
        assert_eq!(
            e.app_dir("VIBE_STORE", Kind::Config, "vibe").unwrap(),
            PathBuf::from("/h/.config/vibe")
        );
    }

    #[test]
    fn app_dir_state_default() {
        let e = env(&[("HOME", "/h")], false);
        assert_eq!(
            e.app_dir("VIBE_STATE", Kind::State, "vibe").unwrap(),
            PathBuf::from("/h/.local/state/vibe")
        );
    }
}
