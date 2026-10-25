//! Every path the program touches derives from [`Paths`].
use std::path::PathBuf;
use vibe_core::dirs::{Env, Kind};

/// An agent harness vibe projects into.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum Harness {
    /// Claude Code.
    Claude,
    /// Codex CLI.
    Codex,
    /// OpenCode.
    Opencode,
    /// Factory Droid.
    Droid,
}

impl Harness {
    /// Every harness, in canonical order.
    pub const ALL: [Harness; 4] = [
        Harness::Claude,
        Harness::Codex,
        Harness::Opencode,
        Harness::Droid,
    ];

    /// The harness id used in files and on the command line.
    pub fn id(self) -> &'static str {
        match self {
            Harness::Claude => "claude",
            Harness::Codex => "codex",
            Harness::Opencode => "opencode",
            Harness::Droid => "droid",
        }
    }

    /// Parses a harness id; `None` for anything else.
    pub fn parse(s: &str) -> Option<Harness> {
        Harness::ALL.into_iter().find(|h| h.id() == s)
    }
}

/// The resolved location of the store, the state and each harness.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Paths {
    /// The user's home directory.
    pub home: PathBuf,
    /// The store root.
    pub store: PathBuf,
    /// The state directory.
    pub state_dir: PathBuf,
    /// The Claude config directory.
    pub claude_dir: PathBuf,
    /// The value of `CLAUDE_CONFIG_DIR` when the environment set it; `None` when it was unset.
    pub claude_config_env: Option<PathBuf>,
    /// The Claude user-level JSON file holding MCP servers.
    pub claude_json: PathBuf,
    /// The Codex home directory.
    pub codex_dir: PathBuf,
    /// The Factory config directory.
    pub factory_dir: PathBuf,
    /// The OpenCode config directory.
    pub opencode_dir: PathBuf,
    /// The shared skills hub.
    pub agents_skills: PathBuf,
}

impl Paths {
    /// Resolves every path from `env`; fails only when no home directory is known.
    pub fn from_env(env: &Env) -> vibe_core::Result<Paths> {
        let home = env.home()?;
        let store = env.app_dir("VIBE_STORE", Kind::Config, "vibe")?;
        let state_dir = env.app_dir("VIBE_STATE", Kind::State, "vibe")?;
        let claude_override = env.var("CLAUDE_CONFIG_DIR").map(PathBuf::from);
        let claude_json = match &claude_override {
            Some(dir) => dir.join(".claude.json"),
            None => home.join(".claude.json"),
        };
        let claude_dir = claude_override
            .clone()
            .unwrap_or_else(|| home.join(".claude"));
        let codex_dir = env
            .var("CODEX_HOME")
            .map_or_else(|| home.join(".codex"), PathBuf::from);
        let xdg = env
            .var("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .filter(|p| p.is_absolute())
            .unwrap_or_else(|| home.join(".config"));
        Ok(Paths {
            factory_dir: home.join(".factory"),
            opencode_dir: xdg.join("opencode"),
            agents_skills: home.join(".agents").join("skills"),
            home,
            store,
            state_dir,
            claude_dir,
            claude_config_env: claude_override,
            claude_json,
            codex_dir,
        })
    }

    /// The store configuration file.
    pub fn store_json(&self) -> PathBuf {
        self.store.join("store.json")
    }

    /// The store's root instructions.
    pub fn store_agents(&self) -> PathBuf {
        self.store.join("AGENTS.md")
    }

    /// The store's canonical MCP list.
    pub fn store_mcp(&self) -> PathBuf {
        self.store.join("mcp.json")
    }

    /// The store's shared skills directory.
    pub fn store_skills(&self) -> PathBuf {
        self.store.join("skills")
    }

    /// The projection state file.
    pub fn state_file(&self) -> PathBuf {
        self.state_dir.join("state.json")
    }

    /// The projection lock file.
    pub fn lock_file(&self) -> PathBuf {
        self.state_dir.join("lock")
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    fn env(vars: &[(&str, &str)]) -> Env {
        Env::new(
            vars.iter().map(|(k, v)| (k.to_string(), v.to_string())),
            false,
        )
    }

    #[test]
    fn paths_default_layout() {
        let p = Paths::from_env(&env(&[("HOME", "/h")])).unwrap();
        assert_eq!(p.home, PathBuf::from("/h"));
        assert_eq!(p.store, PathBuf::from("/h/.config/vibe"));
        assert_eq!(p.state_dir, PathBuf::from("/h/.local/state/vibe"));
        assert_eq!(p.claude_dir, PathBuf::from("/h/.claude"));
        assert_eq!(p.claude_config_env, None);
        assert_eq!(p.claude_json, PathBuf::from("/h/.claude.json"));
        assert_eq!(p.codex_dir, PathBuf::from("/h/.codex"));
        assert_eq!(p.factory_dir, PathBuf::from("/h/.factory"));
        assert_eq!(p.opencode_dir, PathBuf::from("/h/.config/opencode"));
        assert_eq!(p.agents_skills, PathBuf::from("/h/.agents/skills"));
        assert_eq!(p.store_json(), PathBuf::from("/h/.config/vibe/store.json"));
        assert_eq!(p.store_agents(), PathBuf::from("/h/.config/vibe/AGENTS.md"));
        assert_eq!(p.store_mcp(), PathBuf::from("/h/.config/vibe/mcp.json"));
        assert_eq!(p.store_skills(), PathBuf::from("/h/.config/vibe/skills"));
        assert_eq!(
            p.state_file(),
            PathBuf::from("/h/.local/state/vibe/state.json")
        );
        assert_eq!(p.lock_file(), PathBuf::from("/h/.local/state/vibe/lock"));
    }

    #[test]
    fn paths_honour_overrides() {
        let p = Paths::from_env(&env(&[
            ("HOME", "/h"),
            ("VIBE_STORE", "/s"),
            ("VIBE_STATE", "/t"),
            ("CODEX_HOME", "/cx"),
            ("CLAUDE_CONFIG_DIR", "/c"),
            ("XDG_CONFIG_HOME", "/x"),
        ]))
        .unwrap();
        assert_eq!(p.store, PathBuf::from("/s"));
        assert_eq!(p.state_dir, PathBuf::from("/t"));
        assert_eq!(p.codex_dir, PathBuf::from("/cx"));
        assert_eq!(p.claude_dir, PathBuf::from("/c"));
        assert_eq!(p.claude_config_env, Some(PathBuf::from("/c")));
        assert_eq!(p.claude_json, PathBuf::from("/c/.claude.json"));
        assert_eq!(p.opencode_dir, PathBuf::from("/x/opencode"));
    }

    #[test]
    fn claude_config_dir_equal_to_default_dir_still_moves_the_json() {
        let p =
            Paths::from_env(&env(&[("HOME", "/h"), ("CLAUDE_CONFIG_DIR", "/h/.claude")])).unwrap();
        assert_eq!(p.claude_dir, PathBuf::from("/h/.claude"));
        assert_eq!(p.claude_config_env, Some(PathBuf::from("/h/.claude")));
        assert_eq!(p.claude_json, PathBuf::from("/h/.claude/.claude.json"));
    }

    #[test]
    fn paths_on_windows_use_appdata_and_localappdata() {
        let env = Env::new(
            [
                ("USERPROFILE", "C:\\U"),
                ("APPDATA", "C:\\U\\AppData\\Roaming"),
                ("LOCALAPPDATA", "C:\\U\\AppData\\Local"),
            ]
            .map(|(k, v)| (k.to_string(), v.to_string())),
            true,
        );
        let p = Paths::from_env(&env).unwrap();
        assert_eq!(p.home, PathBuf::from("C:\\U"));
        assert_eq!(
            p.store,
            PathBuf::from("C:\\U\\AppData\\Roaming").join("vibe")
        );
        assert_eq!(
            p.state_dir,
            PathBuf::from("C:\\U\\AppData\\Local").join("vibe")
        );
    }

    #[test]
    fn paths_need_a_home() {
        assert!(Paths::from_env(&env(&[])).is_err());
    }

    #[test]
    fn harness_parse_roundtrip() {
        for h in Harness::ALL {
            assert_eq!(Harness::parse(h.id()), Some(h));
        }
        assert_eq!(Harness::parse("nope"), None);
        assert_eq!(Harness::parse("Claude"), None);
        let ids: Vec<&str> = Harness::ALL.iter().map(|h| h.id()).collect();
        assert_eq!(ids, ["claude", "codex", "opencode", "droid"]);
    }
}
