//! `vibe-flow doctor` (mirrors `commands/doctor.mjs`, which mirrors
//! `doctor.sh` — the oracle).
//!
//! Warn-only, read-only, ALWAYS exit 0 — including on missing files, broken
//! symlinks and unreadable JSON. A broken install must still be able to report
//! on itself.
//!
//! Two oracle behaviours are reproduced on purpose, not "improved":
//! - the `tool.jq` check really runs `jq --version`, and jq's *absence* gates
//!   the machine, deps and cursor-validation checks onto their documented,
//!   sometimes more lenient, no-jq shortcuts;
//! - a same-named regular file where a directory is expected reports
//!   "is absent", exactly like bash `-d`.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::cmd::Outcome;
use crate::machine::Machine;

/// The four Claude adapter hook scripts the check knows by name.
const HOOK_SCRIPTS: [&str; 4] = [
    "session-start-doctrine.sh",
    "user-prompt-submit-inject.sh",
    "pre-tool-use-guard.sh",
    "stop-gate.sh",
];

/// Injectable environment for tests; production reads the process.
#[derive(Debug, Clone, Default)]
pub struct Options {
    /// `$HOME`, or `None` to read the process environment.
    pub home: Option<String>,
    /// Override the `jq --version` probe: `(present, version)`.
    pub jq: Option<(bool, String)>,
    /// Override the `node --version` probe: `(present, version)`.
    pub node: Option<(bool, String)>,
}

fn ok(id: &str, message: &str) -> String {
    format!("ok   {id} {message}\n")
}

fn warn(id: &str, message: &str) -> String {
    format!("warn {id} {message}\n")
}

fn is_regular_file(path: &Path) -> bool {
    std::fs::metadata(path).is_ok_and(|meta| meta.is_file())
}

fn is_directory(path: &Path) -> bool {
    std::fs::metadata(path).is_ok_and(|meta| meta.is_dir())
}

#[cfg(unix)]
fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(path)
        .is_ok_and(|meta| meta.is_file() && meta.permissions().mode() & 0o111 != 0)
}

#[cfg(not(unix))]
fn is_executable(path: &Path) -> bool {
    is_regular_file(path)
}

fn is_symlink(path: &Path) -> bool {
    std::fs::symlink_metadata(path).is_ok_and(|meta| meta.file_type().is_symlink())
}

/// The `core.spec` / `core.vibe` check: a real directory passes, a resolving
/// symlink passes and names its target, a broken link is called out.
fn check_link_or_dir(id: &str, path: &Path) -> String {
    let show = path.display();
    let Ok(meta) = std::fs::symlink_metadata(path) else {
        return warn(id, &format!("{show} is absent"));
    };
    if meta.file_type().is_symlink() {
        let target = std::fs::read_link(path)
            .map(|target| target.display().to_string())
            .unwrap_or_else(|_| "?".to_string());
        if path.exists() {
            return ok(id, &format!("{show} -> {target} (symlink resolves)"));
        }
        return warn(id, &format!("{show} -> {target} is a BROKEN symlink"));
    }
    if meta.is_dir() {
        return ok(id, &format!("{show} is a real directory"));
    }
    warn(id, &format!("{show} is absent"))
}

/// `(present, version)` from actually running `<tool> --version`.
fn tool_version(tool: &str) -> (bool, String) {
    let output = Command::new(tool)
        .arg("--version")
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output();
    match output {
        Ok(output) if output.status.success() => (
            true,
            String::from_utf8_lossy(&output.stdout).trim().to_string(),
        ),
        _ => (false, String::new()),
    }
}

fn check_tool_jq(jq: &(bool, String)) -> String {
    if jq.0 {
        ok("tool.jq", &format!("jq present ({})", jq.1))
    } else {
        warn(
            "tool.jq",
            "jq not installed (recommended, not required) — set-state writes the cursor via printf, the guard extracts paths via sed, state reads degrade to idle; cursor + manifest checks unverified",
        )
    }
}

fn node_major(version: &str) -> Option<u32> {
    let digits: String = version
        .trim_start_matches('v')
        .chars()
        .take_while(char::is_ascii_digit)
        .collect();
    let rest = version.trim_start_matches('v').get(digits.len()..)?;
    if !rest.starts_with('.') || digits.is_empty() {
        return None;
    }
    digits.parse().ok()
}

fn check_tool_node(node: &(bool, String)) -> String {
    if node.0 {
        if let Some(major) = node_major(&node.1) {
            if major < 18 {
                return warn(
                    "tool.node",
                    &format!(
                        "node {} is older than the engine's minimum (18) — the hooks may fail and fall back to flow/hooks-fallback (guard + Stop gate) or no-op (inject + doctrine)",
                        node.1
                    ),
                );
            }
        }
        return ok("tool.node", &format!("node present ({})", node.1));
    }
    warn(
        "tool.node",
        "node not installed — the four .claude/hooks are the flow's enforcement, and without node they fall back to flow/hooks-fallback (guard + Stop gate) or no-op (inject + doctrine)",
    )
}

fn check_machine(vibe_dir: &Path, jq_present: bool) -> String {
    let path = vibe_dir.join("state-machine.json");
    if !is_regular_file(&path) {
        return warn(
            "machine",
            &format!(
                "state-machine.json missing at {} — flow harness incomplete",
                path.display()
            ),
        );
    }
    if jq_present {
        if Machine::load(vibe_dir).is_err() {
            return warn(
                "machine",
                "state-machine.json is present but not valid JSON",
            );
        }
    }
    ok("machine", "state-machine.json present")
}

/// jq's `.feature // "none"` interpolation: a string raw, anything else
/// compact JSON.
fn jq_feature_or_none(feature: &serde_json::Value) -> String {
    match feature {
        serde_json::Value::Null | serde_json::Value::Bool(false) => "none".to_string(),
        serde_json::Value::String(text) => text.clone(),
        other => serde_json::to_string(other).unwrap_or_else(|_| "none".to_string()),
    }
}

fn check_cursor(vibe_dir: &Path, jq_present: bool) -> String {
    let state_path = vibe_dir.join("state.json");
    if !is_regular_file(&state_path) {
        return ok("cursor", "no flow cursor (idle) — normal when not mid-flow");
    }
    if !jq_present {
        return ok(
            "cursor",
            "flow cursor present (unverified — jq missing; validate-state.sh needs jq)",
        );
    }
    let validate = vibe_dir.join("scripts").join("validate-state.sh");
    let valid = is_executable(&validate)
        && Command::new("bash")
            .arg(&validate)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|status| status.success());
    if !valid {
        return warn(
            "cursor",
            "flow cursor present but invalid — run validate-state.sh (or reseed from state.example.json)",
        );
    }
    let summary = match std::fs::read(&state_path)
        .ok()
        .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok())
    {
        Some(raw) if raw.is_object() => {
            let field = |key: &str| {
                raw.get(key)
                    .and_then(serde_json::Value::as_str)
                    .filter(|text| !text.is_empty())
                    .unwrap_or("idle")
                    .to_string()
            };
            let (flow, phase) = (field("flow"), field("phase"));
            let key = if flow == phase {
                flow
            } else {
                format!("{flow}.{phase}")
            };
            let feature = raw
                .get("feature")
                .cloned()
                .unwrap_or(serde_json::Value::Null);
            format!("{key} feature={}", jq_feature_or_none(&feature))
        }
        _ => "present".to_string(),
    };
    ok("cursor", &format!("flow cursor valid ({summary})"))
}

struct Adapter {
    lines: String,
    settings_present: bool,
    settings_text: String,
}

fn check_adapter(root: &Path) -> Adapter {
    let mut lines = String::new();
    let mut all_present = true;
    for script in HOOK_SCRIPTS {
        let path = root.join(".claude").join("hooks").join(script);
        if is_regular_file(&path) {
            lines.push_str(&ok(
                &format!("adapter.script.{script}"),
                &format!(".claude/hooks/{script} present"),
            ));
        } else {
            lines.push_str(&warn(
                &format!("adapter.script.{script}"),
                &format!(".claude/hooks/{script} missing — re-run install.sh"),
            ));
            all_present = false;
        }
    }
    let settings_path = root.join(".claude").join("settings.json");
    let settings_present = is_regular_file(&settings_path);
    let settings_text = if settings_present {
        std::fs::read_to_string(&settings_path).unwrap_or_default()
    } else {
        String::new()
    };
    if settings_present {
        let unwired: Vec<&str> = HOOK_SCRIPTS
            .iter()
            .copied()
            .filter(|script| !settings_text.contains(script))
            .collect();
        if unwired.is_empty() {
            lines.push_str(&ok(
                "adapter.activation",
                &format!(
                    "all {} vibe hooks wired in .claude/settings.json",
                    HOOK_SCRIPTS.len()
                ),
            ));
        } else {
            lines.push_str(&warn(
                "adapter.activation",
                &format!(
                    "hooks present but NOT wired in .claude/settings.json (issue #12 gap: {}) — re-run install.sh",
                    unwired.join(" ")
                ),
            ));
        }
    } else if all_present {
        lines.push_str(&warn(
            "adapter.activation",
            ".claude/settings.json absent — hooks not activated (issue #12 gap) — re-run install.sh",
        ));
    } else {
        lines.push_str(&warn(
            "adapter.activation",
            ".claude/settings.json absent and hook scripts missing — re-run install.sh",
        ));
    }
    Adapter {
        lines,
        settings_present,
        settings_text,
    }
}

/// `find <dir> -maxdepth N -type f -name plugin.json -path '*vibe*'` — the
/// starting point is never descended when it is a symlink (find's default
/// `-P`).
fn find_plugin_json(start: &Path, max_depth: u32) -> bool {
    if is_symlink(start) {
        return false;
    }
    fn visit(dir: &Path, level: u32, max_depth: u32) -> bool {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return false;
        };
        let child_level = level + 1;
        for entry in entries.flatten() {
            let path = entry.path();
            let Ok(kind) = entry.file_type() else {
                continue;
            };
            if kind.is_file()
                && entry.file_name() == "plugin.json"
                && path.to_string_lossy().contains("vibe")
            {
                return true;
            }
            if kind.is_dir() && child_level < max_depth && visit(&path, child_level, max_depth) {
                return true;
            }
        }
        false
    }
    visit(start, 0, max_depth)
}

/// `find <dir> -maxdepth N -iname <name>`; the starting point's own basename
/// counts.
fn find_iname_below(start: &Path, name: &str, max_depth: u32) -> bool {
    let target = name.to_lowercase();
    let base = start
        .file_name()
        .map(|value| value.to_string_lossy().to_lowercase());
    if base.as_deref() == Some(target.as_str()) {
        return true;
    }
    if is_symlink(start) {
        return false;
    }
    fn visit(dir: &Path, level: u32, target: &str, max_depth: u32) -> bool {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return false;
        };
        let child_level = level + 1;
        for entry in entries.flatten() {
            if entry.file_name().to_string_lossy().to_lowercase() == target {
                return true;
            }
            if entry.file_type().is_ok_and(|kind| kind.is_dir())
                && child_level < max_depth
                && visit(&entry.path(), child_level, target, max_depth)
            {
                return true;
            }
        }
        false
    }
    visit(start, 0, &target, max_depth)
}

fn dep_present(home: &str, name: &str) -> bool {
    if is_directory(Path::new(&format!("{home}/.claude/skills/{name}"))) {
        return true;
    }
    let plugins = PathBuf::from(format!("{home}/.claude/plugins"));
    if !is_directory(&plugins) {
        return false;
    }
    find_iname_below(&plugins, name, 5)
}

fn check_instruction_coverage(vibe_dir: &Path, adapter: &Adapter, home: &str) -> String {
    let skill_md = vibe_dir.join("SKILL.md");
    let doctrine_block = std::fs::read_to_string(&skill_md)
        .map(|text| text.contains("<!-- vibe:doctrine -->"))
        .unwrap_or(false);
    let session_start_wired =
        adapter.settings_present && adapter.settings_text.contains("session-start-doctrine.sh");
    let plugins = PathBuf::from(format!("{home}/.claude/plugins"));
    let plugin_installed = is_directory(&plugins) && find_plugin_json(&plugins, 6);
    if doctrine_block || session_start_wired || plugin_installed {
        let mut carriers = Vec::new();
        if doctrine_block {
            carriers.push("doctrine block");
        }
        if session_start_wired {
            carriers.push("SessionStart hook");
        }
        if plugin_installed {
            carriers.push("per-user plugin");
        }
        return ok(
            "instruction.coverage",
            &format!("doctrine reaches the agent via: {}", carriers.join(", ")),
        );
    }
    warn(
        "instruction.coverage",
        "no doctrine coverage — no <!-- vibe:doctrine --> block, no wired SessionStart hook, no per-user plugin; run install.sh (--local or --global) / setup.apply",
    )
}

fn check_deps(vibe_dir: &Path, jq_present: bool, home: &str) -> String {
    let path = vibe_dir.join("reference").join("deps.json");
    if !is_regular_file(&path) {
        return warn(
            "deps.manifest",
            &format!("deps.json missing at {}", path.display()),
        );
    }
    if !jq_present {
        return warn(
            "deps.manifest",
            "deps.json present but jq unavailable — cannot read dependency list",
        );
    }
    let Ok(bytes) = std::fs::read(&path) else {
        return warn("deps.manifest", "deps.json is not valid JSON");
    };
    let Ok(raw) = serde_json::from_slice::<serde_json::Value>(&bytes) else {
        return warn("deps.manifest", "deps.json is not valid JSON");
    };
    if raw.is_null() || raw == serde_json::Value::Bool(false) {
        return warn("deps.manifest", "deps.json is not valid JSON");
    }
    let mut lines = ok(
        "deps.manifest",
        &format!("dependency manifest valid ({})", path.display()),
    );
    let deps = raw
        .get("deps")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default();
    for dep in deps {
        let Some(name) = dep
            .get("name")
            .and_then(serde_json::Value::as_str)
            .filter(|text| !text.is_empty())
        else {
            continue;
        };
        let kind = dep
            .get("kind")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("");
        let degrade = dep
            .get("degrade")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("");
        if dep_present(home, name) {
            lines.push_str(&ok(
                &format!("dep.{name}"),
                &format!("{kind} '{name}' present on disk"),
            ));
        } else {
            lines.push_str(&warn(
                &format!("dep.{name}"),
                &format!("{kind} '{name}' not found — degrade: {degrade}"),
            ));
        }
    }
    lines
}

/// The root the report names: three levels up when `skills_dir` really is
/// `<root>/.agents/skills`, else the project root.
fn root_for_report(paths: &crate::paths::FlowPaths) -> PathBuf {
    let skills = crate::orders::skills_dir(paths);
    if let Some(agents) = skills.parent() {
        if agents.file_name().is_some_and(|name| name == ".agents") {
            if let Some(root) = agents.parent() {
                return root.to_path_buf();
            }
        }
    }
    paths.root().to_path_buf()
}

/// `doctor`: the report, exit 0.
pub fn run(paths: &crate::paths::FlowPaths, options: &Options) -> Outcome {
    let home = options.home.clone().unwrap_or_else(|| {
        std::env::var("HOME")
            .ok()
            .filter(|value| !value.is_empty())
            .unwrap_or_default()
    });
    let jq = options.jq.clone().unwrap_or_else(|| tool_version("jq"));
    let node = options.node.clone().unwrap_or_else(|| tool_version("node"));
    let vibe_dir = paths.data_dir();
    let skills_dir = crate::orders::skills_dir(paths);
    let root = root_for_report(paths);

    let mut stdout = format!("# vibe doctor — {}\n", root.display());
    stdout.push_str(&check_tool_jq(&jq));
    stdout.push_str(&check_tool_node(&node));
    stdout.push_str(&check_link_or_dir("core.spec", &skills_dir.join("spec")));
    stdout.push_str(&check_link_or_dir("core.vibe", vibe_dir));
    stdout.push_str(&check_machine(vibe_dir, jq.0));
    stdout.push_str(&check_cursor(vibe_dir, jq.0));
    let adapter = check_adapter(&root);
    stdout.push_str(&adapter.lines);
    stdout.push_str(&check_instruction_coverage(vibe_dir, &adapter, &home));
    stdout.push_str(&check_deps(vibe_dir, jq.0, &home));
    Outcome::new(0, stdout, String::new())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::{Options, run};
    use crate::paths::{Env, FlowPaths};
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};

    static COUNTER: AtomicU64 = AtomicU64::new(0);

    struct NoEnv;

    impl Env for NoEnv {
        fn get(&self, _key: &str) -> Option<String> {
            None
        }
    }

    fn scratch(name: &str) -> PathBuf {
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "vibe-flow-doctor-{name}-{}-{n}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("create scratch");
        dir
    }

    fn copy(src: &str, dest: &Path) {
        if let Some(dir) = dest.parent() {
            fs::create_dir_all(dir).expect("dir");
        }
        fs::copy(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../")
                .join(src),
            dest,
        )
        .expect("copy");
    }

    fn options() -> Options {
        Options {
            home: Some(scratch("home").display().to_string()),
            jq: Some((true, "jq-1.7".to_string())),
            node: Some((true, "v24.0.0".to_string())),
        }
    }

    #[test]
    fn a_complete_install_reports_ok_lines_in_the_oracles_order() {
        let root = scratch("ok");
        copy("flow/SKILL.md", &root.join(".agents/skills/vibe/SKILL.md"));
        copy(
            "flow/state-machine.json",
            &root.join(".agents/skills/vibe/state-machine.json"),
        );
        copy(
            "flow/content/policy.json",
            &root.join(".agents/skills/vibe/content/policy.json"),
        );
        copy(
            "flow/scripts/validate-state.sh",
            &root.join(".agents/skills/vibe/scripts/validate-state.sh"),
        );
        copy(
            "flow/reference/deps.json",
            &root.join(".agents/skills/vibe/reference/deps.json"),
        );
        fs::write(
            root.join(".agents/skills/vibe/state.json"),
            "{\"flow\":\"idle\",\"phase\":\"idle\",\"feature\":null,\"updated\":\"2026-01-01T00:00:00Z\"}\n",
        )
        .expect("cursor");
        for script in super::HOOK_SCRIPTS {
            fs::create_dir_all(root.join(".claude/hooks")).expect("hooks");
            fs::write(root.join(".claude/hooks").join(script), "#!/bin/sh\n").expect("hook");
        }
        let settings = format!("{{\"hooks\":\"{}\"}}\n", super::HOOK_SCRIPTS.join(" "));
        fs::write(root.join(".claude/settings.json"), settings).expect("settings");
        let paths = FlowPaths::resolve(Some(&root), None, &root, &NoEnv);
        let out = run(&paths, &options());
        assert_eq!(out.code, 0);
        assert!(out.stderr.is_empty());
        let ids: Vec<&str> = out
            .stdout
            .lines()
            .skip(1)
            .map(|line| line.split_whitespace().nth(1).unwrap_or(""))
            .collect();
        assert_eq!(
            &ids[..13],
            [
                "tool.jq",
                "tool.node",
                "core.spec",
                "core.vibe",
                "machine",
                "cursor",
                "adapter.script.session-start-doctrine.sh",
                "adapter.script.user-prompt-submit-inject.sh",
                "adapter.script.pre-tool-use-guard.sh",
                "adapter.script.stop-gate.sh",
                "adapter.activation",
                "instruction.coverage",
                "deps.manifest",
            ]
        );
        assert!(ids[13..].iter().all(|id| id.starts_with("dep.")), "{ids:?}");
        assert!(
            out.stdout
                .contains("ok   machine state-machine.json present")
        );
        assert!(
            out.stdout
                .contains("ok   cursor flow cursor valid (idle feature=none)"),
            "{}",
            out.stdout
        );
    }

    #[test]
    fn an_empty_tree_warns_everywhere_and_still_exits_zero() {
        let root = scratch("empty");
        let paths = FlowPaths::resolve(Some(&root), None, &root, &NoEnv);
        let out = run(&paths, &options());
        assert_eq!(out.code, 0);
        assert!(out.stdout.contains("warn core.spec "));
        assert!(out.stdout.contains("warn core.vibe "));
        assert!(out.stdout.contains("warn machine "));
        assert!(
            out.stdout
                .contains("ok   cursor no flow cursor (idle) — normal when not mid-flow"),
            "{}",
            out.stdout
        );
        assert!(out.stdout.contains("warn instruction.coverage "));
        assert!(out.stdout.contains("warn deps.manifest "));
    }

    #[test]
    fn a_corrupt_machine_is_reported_as_invalid_json() {
        let root = scratch("bad-machine");
        copy("flow/SKILL.md", &root.join(".agents/skills/vibe/SKILL.md"));
        fs::write(
            root.join(".agents/skills/vibe/state-machine.json"),
            "{ nope",
        )
        .expect("machine");
        let paths = FlowPaths::resolve(Some(&root), None, &root, &NoEnv);
        let out = run(&paths, &options());
        assert!(
            out.stdout
                .contains("warn machine state-machine.json is present but not valid JSON"),
            "{}",
            out.stdout
        );
    }

    #[test]
    fn an_absent_jq_uses_the_oracles_lenient_branches() {
        let root = scratch("no-jq");
        copy(
            "flow/state-machine.json",
            &root.join(".agents/skills/vibe/state-machine.json"),
        );
        copy(
            "flow/reference/deps.json",
            &root.join(".agents/skills/vibe/reference/deps.json"),
        );
        fs::create_dir_all(root.join(".agents/skills/vibe")).expect("vibe");
        fs::write(root.join(".agents/skills/vibe/state.json"), "{ not json").expect("cursor");
        let paths = FlowPaths::resolve(Some(&root), None, &root, &NoEnv);
        let mut opts = options();
        opts.jq = Some((false, String::new()));
        let out = run(&paths, &opts);
        assert!(out.stdout.contains("warn tool.jq "));
        // Without jq the oracle skips validation entirely and calls the cursor
        // present-but-unverified rather than invalid.
        assert!(
            out.stdout
                .contains("ok   cursor flow cursor present (unverified — jq missing; validate-state.sh needs jq)"),
            "{}",
            out.stdout
        );
        assert!(
            out.stdout
                .contains("warn deps.manifest deps.json present but jq unavailable")
        );
    }

    #[test]
    fn a_same_named_regular_file_where_a_dir_is_expected_is_absent() {
        let root = scratch("file-not-dir");
        fs::create_dir_all(root.join(".agents/skills")).expect("skills");
        fs::write(root.join(".agents/skills/vibe"), "not a dir").expect("file");
        let paths = FlowPaths::resolve(Some(&root), None, &root, &NoEnv);
        let out = run(&paths, &options());
        assert!(
            out.stdout.contains(".agents/skills/vibe is absent"),
            "{}",
            out.stdout
        );
    }

    #[test]
    fn a_broken_symlink_is_named_as_broken() {
        #[cfg(unix)]
        {
            let root = scratch("broken-link");
            fs::create_dir_all(root.join(".agents/skills")).expect("skills");
            std::os::unix::fs::symlink("missing", root.join(".agents/skills/vibe")).expect("link");
            let paths = FlowPaths::resolve(Some(&root), None, &root, &NoEnv);
            let out = run(&paths, &options());
            assert!(
                out.stdout.contains("warn core.vibe ")
                    && out.stdout.contains("is a BROKEN symlink"),
                "{}",
                out.stdout
            );
        }
    }

    #[test]
    fn an_old_node_is_warned_about() {
        let root = scratch("old-node");
        let paths = FlowPaths::resolve(Some(&root), None, &root, &NoEnv);
        let mut opts = options();
        opts.node = Some((true, "v16.20.0".to_string()));
        let out = run(&paths, &opts);
        assert!(
            out.stdout
                .contains("warn tool.node node v16.20.0 is older than the engine's minimum (18)"),
            "{}",
            out.stdout
        );
    }
}
