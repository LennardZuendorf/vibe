//! Hook events: the guard (`pre-tool-use-guard`), the stop gate
//! (`stop-gate`), and the two composition events owned by `vibe-instruct`
//! (D25) that are explicit no-ops here until F12. The guard and the stop gate
//! are byte-faithful ports of `oracles/js/commands/hook.mjs`; the deliberate
//! v0.4 divergences (the runtime receipt path and the `.vibe/run/**`
//! own-runtime exclusion) are pinned in the parity suite, not silent.

pub mod guard;
pub mod inject;
pub mod session;
pub mod stop;

use std::fs;

use crate::cmd::Outcome;
use crate::paths::FlowPaths;

/// The event names the CLI accepts. The guard and the stop gate are
/// implemented; the composition events are explicit no-ops owned by
/// `vibe-instruct` (D25) until F12 flips the shim.
pub const GUARD: &str = "pre-tool-use-guard";
pub const STOP: &str = "stop-gate";
pub const SESSION_START: &str = "session-start";
pub const USER_PROMPT_SUBMIT: &str = "user-prompt-submit";

/// Runs one hook event over the stdin text.
pub fn run(paths: &FlowPaths, event: &str, stdin: &str) -> Outcome {
    match event {
        GUARD => guard::run(paths, stdin),
        STOP => stop::run(paths, stdin),
        SESSION_START => session::run(),
        USER_PROMPT_SUBMIT => inject::run(),
        // Deliberately reads no stdin: an unknown hook name is a typo, and
        // reading stdin from a terminal blocks until EOF.
        _ => Outcome::new(0, String::new(), String::new()),
    }
}

/// A verdict line: exactly `allow`, or `warn:`/`block:` with a reason.
pub(crate) fn is_verdict(text: &str) -> bool {
    if text.contains('\n') {
        return false;
    }
    if text == "allow" {
        return true;
    }
    matches!(text.as_bytes().first(), Some(b'w' | b'b'))
        && match text.split_once(':') {
            Some((kind, reason)) => matches!(kind, "warn" | "block") && !reason.is_empty(),
            None => false,
        }
}

/// Appends one line to the warnings relay. A missing data dir or an
/// unwritable log is a silent no-op — the relay is a nudge, never a failure.
pub(crate) fn append_warn_log(paths: &FlowPaths, line: &str) {
    if !paths.data_dir().is_dir() {
        return;
    }
    use std::io::Write;
    let Ok(mut file) = fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open(paths.warnings_log())
    else {
        return;
    };
    let _ = writeln!(file, "{line}");
}

/// Reads stdin, tolerating a closed stream.
pub(crate) fn read_stdin() -> String {
    let mut text = String::new();
    let _ = std::io::Read::read_to_string(&mut std::io::stdin(), &mut text);
    text
}

/// The ` -> ` separator git prints between a rename/copy's old and new path.
const ARROW: &str = " -> ";

/// The path of a porcelain row: strip the two status columns and the space,
/// then keep the part after the last ` -> ` for rename and copy rows.
pub(crate) fn porcelain_path(row: &str) -> &str {
    let raw = row.get(3..).unwrap_or("");
    match raw.rfind(ARROW) {
        Some(at) => &raw[at + ARROW.len()..],
        None => raw,
    }
}

#[cfg(test)]
mod tests {
    use super::{SESSION_START, USER_PROMPT_SUBMIT, is_verdict, porcelain_path, run};
    use crate::paths::{Env, FlowPaths};
    use std::path::Path;

    struct NoEnv;

    impl Env for NoEnv {
        fn get(&self, _key: &str) -> Option<String> {
            None
        }
    }

    fn fixture_paths() -> FlowPaths {
        let cwd = std::env::temp_dir();
        FlowPaths::resolve(Some(Path::new("/vibe-flow-hook-test")), None, &cwd, &NoEnv)
    }

    #[test]
    fn composition_events_are_silent_no_ops() {
        let paths = fixture_paths();
        for event in [SESSION_START, USER_PROMPT_SUBMIT, "unknown-event"] {
            let outcome = run(&paths, event, "{\"ignored\":true}");
            assert_eq!(outcome.code, 0, "{event}");
            assert!(outcome.stdout.is_empty(), "{event}");
            assert!(outcome.stderr.is_empty(), "{event}");
        }
    }

    #[test]
    fn verdicts_are_single_line_allow_warn_or_block() {
        assert!(is_verdict("allow"));
        assert!(is_verdict("warn:reason"));
        assert!(is_verdict("block:reason"));
        assert!(!is_verdict("warn:"));
        assert!(!is_verdict("block:"));
        assert!(!is_verdict("bogus"));
        assert!(!is_verdict("allow\nblock:x"));
        assert!(!is_verdict("warn:x\nblock:y"));
    }

    #[test]
    fn porcelain_path_strips_columns_and_keeps_the_rename_target() {
        assert_eq!(porcelain_path(" M src/a.rs"), "src/a.rs");
        assert_eq!(porcelain_path("R  old.rs -> new.rs"), "new.rs");
        assert_eq!(porcelain_path("?? .vibe/"), ".vibe/");
    }
}
