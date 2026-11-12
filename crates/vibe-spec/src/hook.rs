//! The Claude Code hook payload: the JSON a hook reads on stdin.
use std::path::{Path, PathBuf};

use serde_json::Value;

/// The fields every spec hook needs from the payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Payload {
    /// `hook_event_name`.
    pub event: String,
    /// `cwd`.
    pub cwd: PathBuf,
    /// `transcript_path`: for a subagent this is the main session's transcript.
    pub transcript: Option<PathBuf>,
    /// `agent_id`, present only for a subagent's tool calls.
    pub agent_id: Option<String>,
    /// `tool_name`.
    pub tool: Option<String>,
    /// `tool_input.file_path`, else `tool_input.notebook_path`.
    pub file: Option<String>,
    /// `tool_input.command`.
    pub command: Option<String>,
}

fn string(v: &Value, key: &str) -> Option<String> {
    v.get(key).and_then(Value::as_str).map(str::to_string)
}

/// Parses a hook payload; an error names the first problem.
pub fn parse(stdin: &[u8]) -> Result<Payload, String> {
    let v: Value = serde_json::from_slice(stdin).map_err(|e| format!("malformed payload: {e}"))?;
    if !v.is_object() {
        return Err("malformed payload: not a JSON object".to_string());
    }
    let event = string(&v, "hook_event_name").ok_or("payload has no hook_event_name")?;
    let cwd = string(&v, "cwd").ok_or("payload has no cwd")?;
    let input = v.get("tool_input").unwrap_or(&Value::Null);
    Ok(Payload {
        event,
        cwd: PathBuf::from(cwd),
        transcript: string(&v, "transcript_path").map(PathBuf::from),
        agent_id: string(&v, "agent_id"),
        tool: string(&v, "tool_name"),
        file: string(input, "file_path").or_else(|| string(input, "notebook_path")),
        command: string(input, "command"),
    })
}

/// What a path needs before the guard lets an edit through.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    /// The spec root and `.vibe/spec.json`: needs `spec`.
    Spec,
    /// `docs/superpowers/specs/`: needs `superpowers:brainstorming` or `spec`.
    Brainstorm,
    /// `docs/superpowers/plans/`: needs `superpowers:writing-plans` or `spec`.
    Plans,
}

impl Target {
    fn names(self) -> &'static [&'static str] {
        match self {
            Target::Spec => &["spec"],
            Target::Brainstorm => &["superpowers:brainstorming", "spec"],
            Target::Plans => &["superpowers:writing-plans", "spec"],
        }
    }
}

/// `/`-joined absolute-or-relative spelling with `.` and `..` resolved
/// lexically; backslashes count as separators.
fn lexical(path: &str) -> String {
    let path = path.replace('\\', "/");
    let absolute = path.starts_with('/');
    let mut parts: Vec<&str> = Vec::new();
    for part in path.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                if parts.last().is_some_and(|l| *l != "..") {
                    parts.pop();
                } else if !absolute {
                    parts.push("..");
                }
            }
            p => parts.push(p),
        }
    }
    let joined = parts.join("/");
    if absolute {
        format!("/{joined}")
    } else {
        joined
    }
}

/// File identity, not spelling: the longest existing ancestor is
/// canonicalized (symlinks resolved), the rest is kept lexically.
pub(crate) fn identity(path: &str) -> String {
    let lex = lexical(path);
    let mut rest: Vec<&str> = Vec::new();
    let mut head = lex.as_str();
    loop {
        if !head.is_empty()
            && let Ok(real) = std::fs::canonicalize(head)
        {
            let mut out = lexical(&real.to_string_lossy());
            for part in rest.iter().rev() {
                if !out.ends_with('/') {
                    out.push('/');
                }
                out.push_str(part);
            }
            return out;
        }
        match head.rsplit_once('/') {
            Some((up, last)) => {
                rest.push(last);
                head = up;
            }
            None => return lex,
        }
    }
}

/// `path` relative to `base` (both identity-normalized, `/`-joined).
pub(crate) fn under<'a>(base: &str, path: &'a str) -> Option<&'a str> {
    let base = base.trim_end_matches('/');
    let rest = path.strip_prefix(base)?;
    rest.strip_prefix('/')
}

pub(crate) fn is_absolute(file: &str) -> bool {
    let f = file.replace('\\', "/");
    let b = f.as_bytes();
    f.starts_with('/')
        || (b.len() > 2 && b[0].is_ascii_alphabetic() && b[1] == b':' && b[2] == b'/')
}

/// The identity-normalized `file`, resolved against `project` when relative.
fn resolve(project: &str, file: &str) -> String {
    if is_absolute(file) {
        identity(file)
    } else {
        identity(&format!("{project}/{file}"))
    }
}

/// Classifies `file`; the `String` is its project-relative, `/`-joined path.
/// `None` when it is outside the project or not a target.
pub fn classify(project: &Path, root: &Path, file: &str) -> Option<(Target, String)> {
    let project = identity(&project.to_string_lossy());
    let target = resolve(&project, file);
    let rel = under(&project, &target)?.to_string();
    let root = identity(&root.to_string_lossy());
    let kind = if under(&root, &target).is_some() || rel == ".vibe/spec.json" {
        Target::Spec
    } else if rel.starts_with("docs/superpowers/specs/") {
        Target::Brainstorm
    } else if rel.starts_with("docs/superpowers/plans/") {
        Target::Plans
    } else {
        return None;
    };
    Some((kind, rel))
}

fn block_line(target: Target, rel: &str) -> String {
    let skills = target
        .names()
        .iter()
        .map(|n| format!("Skill({n})"))
        .collect::<Vec<_>>()
        .join(" or ");
    format!("blocked: invoke {skills} before editing {rel}")
}

/// Prints one fail-open line and allows.
fn allow(cx: &crate::api::Execution<'_>, cause: &str) -> (u8, Vec<u8>) {
    diag!(cx, "vibe-spec hook guard: allowing — {cause}");
    (0, Vec::new())
}

/// A shell command that writes (redirect, `tee`, `sed -i`, `cp`, `mv`)?
fn writes(command: &str) -> bool {
    let words: Vec<&str> = words(command);
    command.contains('>')
        || words.iter().any(|w| matches!(*w, "tee" | "cp" | "mv"))
        || (words.contains(&"sed")
            && words
                .iter()
                .any(|w| w.starts_with("-i") || *w == "--in-place"))
}

fn words(command: &str) -> Vec<&str> {
    command
        .split(|c: char| {
            c.is_whitespace() || matches!(c, '>' | '<' | ';' | '|' | '&' | '(' | ')' | '"' | '\'')
        })
        .filter(|w| !w.is_empty())
        .collect()
}

/// The PreToolUse tooth (P4-R3, P4-R4): `(0|2, stdout)`; stderr is printed here.
pub fn guard(cx: &crate::api::Execution<'_>, stdin: &[u8]) -> (u8, Vec<u8>) {
    let payload = match parse(stdin) {
        Ok(p) => p,
        Err(e) => return allow(cx, &e),
    };
    match managed_context(cx, &payload.cwd) {
        Ok(Some(context)) => return managed_guard(cx, &payload, &context),
        Ok(None) => {}
        Err(error) => return allow(cx, &error.to_string()),
    }
    let tool = payload.tool.as_deref().unwrap_or("");
    let is_bash = tool == "Bash";
    if !is_bash && payload.file.is_none() {
        return (0, Vec::new());
    }
    let found = crate::root::discover(&payload.cwd, None, env_root(cx).as_deref());
    let root = match found {
        Ok(root) => root,
        Err(crate::root::Miss::NotFound) => return allow(cx, "no spec root"),
        Err(miss) => return allow(cx, &miss.message()),
    };
    if is_bash {
        let command = payload.command.as_deref().unwrap_or("");
        if writes(command) {
            let cwd = payload.cwd.to_string_lossy().into_owned();
            let hit = words(command).into_iter().find_map(|w| {
                let file = if is_absolute(w) {
                    w.to_string()
                } else {
                    format!("{cwd}/{w}")
                };
                classify(&root.project, &root.dir, &file)
            });
            if let Some((_, rel)) = hit {
                diag!(
                    cx,
                    "vibe-spec: warning: a shell write to {rel} bypasses the spec tooth"
                );
            }
        }
        return (0, Vec::new());
    }
    let file = payload.file.as_deref().unwrap_or("");
    let cwd = payload.cwd.to_string_lossy().into_owned();
    let absolute = if is_absolute(file) {
        file.to_string()
    } else {
        format!("{cwd}/{file}")
    };
    let Some((target, rel)) = classify(&root.project, &root.dir, &absolute) else {
        let project = identity(&root.project.to_string_lossy());
        if under(&project, &resolve(&project, &absolute)).is_none() {
            return allow(cx, "the target is outside the project");
        }
        return (0, Vec::new());
    };
    let Some(transcript) = payload.transcript.as_deref() else {
        return allow(cx, "the payload has no transcript_path");
    };
    match crate::transcript::loaded(transcript, payload.agent_id.as_deref(), target.names()) {
        Ok(true) => return (0, Vec::new()),
        Ok(false) => {}
        Err(e) => {
            return allow(
                cx,
                &format!("unreadable transcript {}: {e}", transcript.display()),
            );
        }
    }
    let line = block_line(target, &rel);
    match crate::root::tooth(&root.project) {
        Ok(crate::root::Tooth::Block) => {
            diag!(cx, "{line}");
            (2, Vec::new())
        }
        Ok(crate::root::Tooth::Warn) => {
            diag!(cx, "vibe-spec hook guard: warn — {line}");
            (0, Vec::new())
        }
        Err(e) => allow(cx, &e),
    }
}

/// The cap in ms for a validate hook; `VIBE_SPEC_HOOK_CAP_MS` overrides it (tests).
fn cap_ms(cx: &crate::api::Execution<'_>, default: u64) -> u64 {
    cx.var("VIBE_SPEC_HOOK_CAP_MS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

/// The post-edit and stop validate hooks (P4-R5): never blocks, so the
/// exit code is always 0. `PostToolUse` validates the edited file within
/// 500 ms; `Stop` validates the changed files within 2 000 ms.
pub fn validate(cx: &crate::api::Execution<'_>, stdin: &[u8], _cwd: &Path) -> (u8, Vec<u8>) {
    let payload = match parse(stdin) {
        Ok(p) => p,
        Err(e) => return allow_validate(cx, &e),
    };
    let (file, changed, default_cap) = match payload.event.as_str() {
        "PostToolUse" => {
            let Some(file) = payload.file.clone().filter(|f| f.ends_with(".md")) else {
                return (0, Vec::new());
            };
            let abs = if is_absolute(&file) {
                file
            } else {
                format!("{}/{file}", payload.cwd.to_string_lossy())
            };
            (Some(abs), false, 500)
        }
        "Stop" => (None, true, 2000),
        other => return allow_validate(cx, &format!("unknown event {other}")),
    };
    let cap = cap_ms(cx, default_cap);
    let event = payload.event.clone();
    let cwd = payload.cwd.clone();
    let env = env_root(cx);
    let (tx, rx) = std::sync::mpsc::channel();
    let runtime = cx.runtime.clone();
    std::thread::spawn(move || {
        let cx = &crate::api::Execution::new(&runtime);
        let mut selected_runtime = runtime.clone();
        selected_runtime.cwd = cwd.clone();
        let selected_cx = crate::api::Execution::new(&selected_runtime);
        match managed_context(&selected_cx, &cwd) {
            Ok(Some(_)) => {
                let result = crate::SpecApi::new(selected_runtime.clone())
                    .and_then(|api| {
                        crate::transaction::read_typed(
                            &api,
                            &crate::ContextSelector::default(),
                            |context| {
                                let scope = if context.code_available {
                                    crate::ValidationScope::WithCode
                                } else {
                                    crate::ValidationScope::SpecOnly
                                };
                                crate::validate::context_result(
                                    &selected_runtime,
                                    context,
                                    scope,
                                    false,
                                )
                            },
                        )
                    })
                    .map(|(_, result)| {
                        crate::validate::render_managed(
                            result,
                            crate::validate::Format::Hook,
                            Some(&event),
                        )
                    })
                    .map_err(|error| error.to_string());
                let _ = tx.send((Some(result), Vec::new()));
                return;
            }
            Err(error) => {
                let _ = tx.send((Some(Err(error.to_string())), Vec::new()));
                return;
            }
            Ok(None) => {}
        }
        let found = crate::root::discover(&cwd, None, env.as_deref());
        let out = match found {
            Ok(_) => Some(crate::validate::run_checked(
                cx,
                &found,
                &cwd,
                &crate::validate::Options {
                    strict: false,
                    format: crate::validate::Format::Hook,
                    changed,
                    file,
                    event: Some(event),
                },
            )),
            Err(_) => None,
        };
        let diagnostics = cx.take_diagnostics();
        let _ = tx.send((out, diagnostics));
    });
    let received =
        rx.recv_timeout(std::time::Duration::from_millis(cap))
            .map(|(out, diagnostics)| {
                cx.diagnostic_bytes(&diagnostics);
                out
            });
    match received {
        Ok(Some(Ok((_, out)))) => (0, out),
        Ok(Some(Err(why))) => allow_validate(cx, &why),
        Ok(None) => (0, Vec::new()),
        Err(_) => {
            diag!(cx, "vibe-spec hook validate: skipped — over {cap} ms");
            (0, Vec::new())
        }
    }
}

fn allow_validate(cx: &crate::api::Execution<'_>, cause: &str) -> (u8, Vec<u8>) {
    diag!(cx, "vibe-spec hook validate: skipped — {cause}");
    (0, Vec::new())
}

/// The router skill: frontmatter plus body (P4-R6).
const USING_SPEC: &str = include_str!("../../../spec/using-spec/SKILL.md");

/// Which JSON shape the host harness reads from a `SessionStart` hook.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Harness {
    /// Claude Code: `hookSpecificOutput.additionalContext`.
    Claude,
    /// Cursor: `additional_context`.
    Cursor,
    /// Any other harness: `additionalContext`.
    Other,
}

impl Harness {
    /// Detects the harness from the environment (P4-R6).
    pub fn detect(var: impl Fn(&str) -> bool) -> Harness {
        if var("CLAUDE_PLUGIN_ROOT") || var("CLAUDE_PROJECT_DIR") {
            Harness::Claude
        } else if var("CURSOR_PLUGIN_ROOT") {
            Harness::Cursor
        } else {
            Harness::Other
        }
    }
}

/// `$VIBE_SPEC_ROOT`, as the CLI reads it.
fn env_root(cx: &crate::api::Execution<'_>) -> Option<std::ffi::OsString> {
    cx.var_os("VIBE_SPEC_ROOT")
}

/// The skill text after its frontmatter.
fn router_body() -> &'static str {
    USING_SPEC
        .strip_prefix("---\n")
        .and_then(|rest| rest.split_once("\n---\n"))
        .map_or(USING_SPEC, |(_, body)| body)
        .trim()
}

/// Refreshes `<project>/.vibe/spec-session` without following links; the
/// error names why the marker was skipped.
fn touch_marker(project: &Path) -> Result<(), String> {
    let vibe = project.join(".vibe");
    match std::fs::symlink_metadata(&vibe) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            std::fs::create_dir(&vibe).map_err(|e| format!(".vibe cannot be created ({e})"))?;
        }
        Err(e) => return Err(format!(".vibe cannot be read ({e})")),
        Ok(m) if m.file_type().is_symlink() => return Err(".vibe is a symlink".to_string()),
        Ok(m) if !m.is_dir() => return Err(".vibe is not a directory".to_string()),
        Ok(_) => {}
    }
    let marker = vibe.join("spec-session");
    if std::fs::symlink_metadata(&marker).is_ok_and(|m| m.file_type().is_symlink()) {
        return Err(".vibe/spec-session is a symlink".to_string());
    }
    vibe_core::fs::write_atomic(&marker, b"")
        .map_err(|e| format!(".vibe/spec-session cannot be written ({e})"))
}

/// `hook session-start`: injects the `using-spec` router and, inside a
/// project with a spec root, touches `.vibe/spec-session`. Silent (`{}`)
/// where no spec root exists: the router is for a repo that keeps a spec.
/// A bad config or a missing named root still routes. Always exits 0.
pub fn session_start(
    cx: &crate::api::Execution<'_>,
    stdin: &[u8],
    cwd: &Path,
    harness: Harness,
) -> (u8, Vec<u8>) {
    let dir = parse(stdin).map_or_else(|_| cwd.to_path_buf(), |p| p.cwd);
    let managed = match managed_context(cx, &dir) {
        Ok(Some(_)) => true,
        Ok(None) => false,
        Err(error) => {
            diag!(cx, "vibe-spec hook session-start: skipped — {error}");
            return (0, b"{}\n".to_vec());
        }
    };
    if !managed {
        match crate::root::discover(&dir, None, env_root(cx).as_deref()) {
            Err(crate::root::Miss::NotFound) => return (0, b"{}\n".to_vec()),
            // The marker only matters where git cannot say what changed.
            Ok(root) => {
                if crate::git::top(cx, &root.project).is_none()
                    && let Err(why) = touch_marker(&root.project)
                {
                    diag!(cx, "vibe-spec hook session-start: marker skipped — {why}");
                }
            }
            Err(_) => {}
        }
    }
    let text = format!(
        "<EXTREMELY_IMPORTANT>\n{}\n</EXTREMELY_IMPORTANT>",
        router_body()
    );
    let value = match harness {
        Harness::Claude => serde_json::json!({"hookSpecificOutput": {
            "hookEventName": "SessionStart", "additionalContext": text}}),
        Harness::Cursor => serde_json::json!({"additional_context": text}),
        Harness::Other => serde_json::json!({"additionalContext": text}),
    };
    let mut out = value.to_string().into_bytes();
    out.push(b'\n');
    (0, out)
}

fn managed_context(
    cx: &crate::api::Execution<'_>,
    cwd: &Path,
) -> Result<Option<crate::SpecContext>, crate::SpecError> {
    let mut runtime = cx.runtime.clone();
    runtime.cwd = cwd.to_owned();
    let registry = crate::registry::store_root(&runtime)?.join("spec/registry.json");
    if std::fs::symlink_metadata(registry).is_err_and(|e| e.kind() == std::io::ErrorKind::NotFound)
    {
        return Ok(None);
    }

    match crate::context::resolve_for_routing(&runtime, &crate::ContextSelector::default()) {
        Ok(context) if context.mode == crate::StorageMode::GlobalFirst => Ok(Some(
            crate::SpecApi::new(runtime)?.resolve_context(&crate::ContextSelector::default())?,
        )),
        Ok(_) => Ok(None),
        Err(error)
            if !matches!(
                error.code.as_str(),
                "invalid_registry" | "invalid_project" | "ambiguous_project" | "context_mismatch"
            ) =>
        {
            Ok(None)
        }
        Err(error) => Err(error),
    }
}
fn managed_classify(context: &crate::SpecContext, absolute: &str) -> Option<(Target, String)> {
    let path = identity(absolute);
    if let Some(checkout) = context.checkout.as_ref()
        && path == identity(&checkout.join(".vibe/spec.json").to_string_lossy())
    {
        return Some((Target::Spec, ".vibe/spec.json".into()));
    }
    for (root, target, label) in [
        (&context.spec_root, Target::Spec, "spec"),
        (
            &context.artifact_root.join("specs"),
            Target::Brainstorm,
            "superpowers/specs",
        ),
        (
            &context.artifact_root.join("plans"),
            Target::Plans,
            "superpowers/plans",
        ),
    ] {
        let base = identity(&root.to_string_lossy());
        if let Some(rel) = under(&base, &path) {
            return Some((target, format!("{label}/{rel}")));
        }
    }
    None
}
fn managed_guard(
    cx: &crate::api::Execution<'_>,
    payload: &Payload,
    context: &crate::SpecContext,
) -> (u8, Vec<u8>) {
    let absolute = |file: &str| {
        if is_absolute(file) {
            file.to_string()
        } else {
            format!("{}/{file}", payload.cwd.display())
        }
    };
    if payload.tool.as_deref() == Some("Bash") {
        let command = payload.command.as_deref().unwrap_or_default();
        if writes(command)
            && let Some((_, rel)) = words(command)
                .into_iter()
                .find_map(|w| managed_classify(context, &absolute(w)))
        {
            diag!(
                cx,
                "vibe-spec: warning: a shell write to {rel} bypasses the spec tooth"
            );
        }
        return (0, Vec::new());
    }
    let Some((target, rel)) = payload
        .file
        .as_deref()
        .and_then(|file| managed_classify(context, &absolute(file)))
    else {
        return (0, Vec::new());
    };
    let Some(transcript) = payload.transcript.as_deref() else {
        return allow(cx, "the payload has no transcript_path");
    };
    match crate::transcript::loaded(transcript, payload.agent_id.as_deref(), target.names()) {
        Ok(true) => return (0, Vec::new()),
        Err(error) => return allow(cx, &error.to_string()),
        Ok(false) => {}
    }
    let line = block_line(target, &rel);
    let tooth = context
        .checkout
        .as_deref()
        .map(crate::root::tooth)
        .unwrap_or(Ok(crate::root::Tooth::Block));
    match tooth {
        Ok(crate::root::Tooth::Block) => {
            diag!(cx, "{line}");
            (2, Vec::new())
        }
        Ok(crate::root::Tooth::Warn) => {
            diag!(cx, "vibe-spec hook guard: warn — {line}");
            (0, Vec::new())
        }
        Err(error) => allow(cx, &error),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backslash_paths_classify() {
        let got = classify(
            Path::new(r"C:\p"),
            Path::new(r"C:\p\docs\spec"),
            r"C:\p\docs\spec\a.md",
        );
        assert_eq!(got, Some((Target::Spec, "docs/spec/a.md".to_string())));
        let got = classify(
            Path::new(r"C:\p"),
            Path::new(r"C:\p\docs\spec"),
            r"C:\p\docs\superpowers\plans\x.md",
        );
        assert_eq!(
            got,
            Some((Target::Plans, "docs/superpowers/plans/x.md".to_string()))
        );
        assert_eq!(
            classify(
                Path::new(r"C:\p"),
                Path::new(r"C:\p\docs\spec"),
                r"C:\p\src\a.rs"
            ),
            None
        );
    }
}
