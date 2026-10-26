//! The pre-tool-use guard, ported from `oracles/js/commands/hook.mjs`
//! (lines 525-600). A block exits 2 with the oracle's exact bytes; a warn
//! exits 0 and lands in the warnings relay too; an unusable policy falls
//! back to spawning the bash `detect-context.sh decide`, exactly like the
//! engine.

use serde_json::Value;

use super::{Outcome, append_warn_log, is_verdict};
use crate::cursor;
use crate::paths::FlowPaths;
use crate::policy;

/// Runs the guard over the stdin payload.
pub fn run(paths: &FlowPaths, stdin: &str) -> Outcome {
    if stdin.is_empty() || !paths.detect_script().exists() {
        return Outcome::new(0, String::new(), String::new());
    }
    let parsed: Value = serde_json::from_str(stdin).unwrap_or(Value::Null);
    let tool_name = parsed
        .get("tool_name")
        .and_then(Value::as_str)
        .unwrap_or("");

    if tool_name == "Bash" {
        let command = parsed
            .get("tool_input")
            .and_then(|ti| ti.get("command"))
            .and_then(Value::as_str)
            .unwrap_or("");
        if command.is_empty() {
            return Outcome::new(0, String::new(), String::new());
        }
        if let Some(label) = sniff_bash(command) {
            append_warn_log(
                paths,
                &format!(
                    "guard: bash command appears to write {label} (bypasses the file-tool guard) (warn-only)"
                ),
            );
            return Outcome::new(
                0,
                String::new(),
                format!(
                    "vibe-guard: warn — a bash command looks like it writes {label} (a guarded path); prefer the flow's write surface over a raw shell redirect. (warn-only)\n"
                ),
            );
        }
        return Outcome::new(0, String::new(), String::new());
    }

    let Some(path) = target_path(&parsed) else {
        return Outcome::new(0, String::new(), String::new());
    };
    let path = strip_root_prefix(&path, paths.root());
    let path = path.strip_prefix("./").unwrap_or(path).to_string();

    let verdict = match in_process_verdict(paths, &path) {
        Some(v) => v,
        None => spawn_verdict(paths, &path),
    };

    if let Some(reason) = verdict.strip_prefix("block:") {
        return Outcome::new(
            2,
            String::new(),
            format!(
                "vibe-guard: BLOCKED — {reason}\nvibe-guard: transition with set-state.sh, or edit within the current state's write rules.\n"
            ),
        );
    }
    if let Some(reason) = verdict.strip_prefix("warn:") {
        append_warn_log(paths, &format!("guard: {reason}"));
        return Outcome::new(0, String::new(), format!("vibe-guard: warn — {reason}\n"));
    }
    Outcome::new(0, String::new(), String::new())
}

/// The write-invariant decision, answered in-process. `None` means the
/// policy data is unusable (errors, zero rules) or a warn/block carries no
/// reason — both fall back to the spawned script.
fn in_process_verdict(paths: &FlowPaths, path: &str) -> Option<String> {
    let loaded = policy::load_policy(paths.data_dir());
    if !loaded.errors.is_empty() || loaded.policy.rules.is_empty() {
        return None;
    }
    let state = match cursor::read_cursor(paths) {
        Ok(cur) => cursor::state_key(&cur.flow, &cur.phase),
        Err(_) => "idle".to_string(),
    };
    let decision = policy::decide(&loaded.policy, path, &state);
    match decision.verdict {
        policy::Verdict::Allow => Some("allow".to_string()),
        _ if decision.reason.is_empty() => None,
        policy::Verdict::Warn => Some(format!("warn:{}", decision.reason)),
        policy::Verdict::Block => Some(format!("block:{}", decision.reason)),
    }
}

/// Spawns the frozen bash `decide` as the fallback. The child gets no stdin
/// and anything that is not a well-formed verdict reads as `allow` — the
/// guard fails open only through the shape gate, never by forwarding
/// diagnostics.
fn spawn_verdict(paths: &FlowPaths, path: &str) -> String {
    use std::process::{Command, Stdio};
    let out = Command::new("bash")
        .arg(paths.detect_script())
        .arg("decide")
        .arg(path)
        .stdin(Stdio::null())
        .output();
    let Ok(out) = out else {
        return "allow".to_string();
    };
    if !out.status.success() {
        return "allow".to_string();
    }
    let text = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if is_verdict(&text) {
        text
    } else {
        "allow".to_string()
    }
}

/// The Bash write sniffer: the first guarded-path-class hit, or `None`.
fn sniff_bash(command: &str) -> Option<String> {
    if !has_write_op(command) {
        return None;
    }
    if let Some(spelling) = boundary_hit(command, &["docs/spec/lessons.md", ".spec/lessons.md"]) {
        return Some(spelling);
    }
    for stem in ["product", "tech", "design", "plan"] {
        let docs = format!("docs/spec/{stem}.md");
        let legacy = format!(".spec/{stem}.md");
        if let Some(spelling) = boundary_hit(command, &[docs.as_str(), legacy.as_str()]) {
            let root = if spelling.starts_with("docs/spec") {
                "docs/spec"
            } else {
                ".spec"
            };
            return Some(format!("a root {root}/{{product,tech,design,plan}}.md doc"));
        }
    }
    if boundary_hit(command, &[".vibe/run/flow/state.json"]).is_some() {
        return Some(".vibe/run/flow/state.json".to_string());
    }
    if command.contains(".agents/skills/vibe/state.json")
        || boundary_hit(command, &["flow/state.json"]).is_some()
    {
        if command.contains("set-state.sh") {
            return None;
        }
        return Some(".agents/skills/vibe/state.json (use set-state.sh)".to_string());
    }
    None
}

/// A write-shaped operator anywhere in the command, mirroring the oracle's
/// `(>>?|(^|\s)(tee|truncate|mv|cp|rm)(\s|$)|(^|\s)sed(\s|$).*-i)`: a
/// redirect is a raw substring; the utilities must stand as whole
/// whitespace-delimited words (`rm;` is NOT a hit); `sed` as a word hits
/// when `-i` appears anywhere after it.
fn has_write_op(command: &str) -> bool {
    if command.contains('>') {
        return true;
    }
    let delimited = |word: &str| -> bool {
        let mut from = 0;
        while let Some(at) = command[from..].find(word) {
            let start = from + at;
            let end = start + word.len();
            let before = start == 0 || command[..start].ends_with(char::is_whitespace);
            let after = end == command.len() || command[end..].starts_with(char::is_whitespace);
            if before && after {
                return true;
            }
            from = start + 1;
        }
        false
    };
    for word in ["tee", "truncate", "mv", "cp", "rm"] {
        if delimited(word) {
            return true;
        }
    }
    let sed_tail = |word: &str| -> Option<usize> {
        let mut from = 0;
        while let Some(at) = command[from..].find(word) {
            let start = from + at;
            let end = start + word.len();
            let before = start == 0 || command[..start].ends_with(char::is_whitespace);
            let after = end == command.len() || command[end..].starts_with(char::is_whitespace);
            if before && after {
                return Some(end);
            }
            from = start + 1;
        }
        None
    };
    match sed_tail("sed") {
        Some(end) => command[end..].contains("-i"),
        None => false,
    }
}

/// The first spelling present with a word boundary before it: the char
/// before the hit (when any) must not be alphanumeric or underscore.
fn boundary_hit(command: &str, spellings: &[&str]) -> Option<String> {
    for spelling in spellings {
        let mut from = 0;
        while let Some(at) = command[from..].find(*spelling) {
            let start = from + at;
            let boundary = start == 0
                || !command[..start]
                    .chars()
                    .next_back()
                    .is_some_and(|c| c.is_alphanumeric() || c == '_');
            if boundary {
                return Some((*spelling).to_string());
            }
            from = start + spelling.len();
        }
    }
    None
}

/// The tool payload's target path: `file_path`, else `notebook_path`.
fn target_path(parsed: &Value) -> Option<String> {
    let path = parsed
        .get("tool_input")
        .and_then(|ti| ti.get("file_path"))
        .or_else(|| {
            parsed
                .get("tool_input")
                .and_then(|ti| ti.get("notebook_path"))
        })
        .and_then(Value::as_str)?;
    if path.is_empty() {
        None
    } else {
        Some(path.to_string())
    }
}

/// Strips the `<root>/` prefix, like the oracle's `stripLeadingRoot`.
fn strip_root_prefix<'a>(path: &'a str, root: &std::path::Path) -> &'a str {
    let root = root.to_string_lossy();
    let root = root.strip_suffix('/').unwrap_or(&root);
    path.strip_prefix(&format!("{root}/")).unwrap_or(path)
}

#[cfg(test)]
mod tests {
    use super::{has_write_op, sniff_bash};

    #[test]
    fn write_ops_cover_redirects_utilities_and_in_place_sed() {
        assert!(has_write_op("echo x >> notes.md"));
        assert!(has_write_op("printf x > notes.md"));
        assert!(has_write_op("cat x | tee notes.md"));
        assert!(has_write_op("mv a b"));
        assert!(has_write_op("cp a b"));
        assert!(has_write_op("rm a"));
        assert!(has_write_op("sed -i 's/a/b/' notes.md"));
        assert!(!has_write_op("sed 's/a/b/' notes.md"));
        assert!(!has_write_op("grep foo notes.md"));
        assert!(!has_write_op("cat notes.md"));
        assert!(!has_write_op("rm;"));
    }

    #[test]
    fn sniffing_labels_the_new_cursor_before_the_legacy_one() {
        assert_eq!(
            sniff_bash("echo x >> .vibe/run/flow/state.json").as_deref(),
            Some(".vibe/run/flow/state.json")
        );
        assert_eq!(
            sniff_bash("echo x >> .agents/skills/vibe/state.json").as_deref(),
            Some(".agents/skills/vibe/state.json (use set-state.sh)")
        );
        assert_eq!(
            sniff_bash("echo x >> docs/spec/lessons.md").as_deref(),
            Some("docs/spec/lessons.md")
        );
        assert_eq!(
            sniff_bash("echo x >> docs/spec/tech.md").as_deref(),
            Some("a root docs/spec/{product,tech,design,plan}.md doc")
        );
    }

    #[test]
    fn the_legacy_set_state_carve_out_stays_but_a_read_is_silent() {
        assert_eq!(
            sniff_bash(
                "bash .agents/skills/vibe/scripts/set-state.sh feature.impl >> .agents/skills/vibe/state.json"
            ),
            None
        );
        assert_eq!(sniff_bash("cat docs/spec/lessons.md"), None);
        assert_eq!(sniff_bash("echo x >> unrelated.md"), None);
    }
}
