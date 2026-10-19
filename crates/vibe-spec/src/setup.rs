//! `vibe-spec setup` (P4-R10): the pre-commit hook, the OpenCode agent files
//! and plugin shim, and spec's router block in AGENTS.md. Every file carries a
//! marker; a file without one is never replaced, and nothing is written when
//! any target is refused.
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use vibe_core::exit;

use crate::root::{Miss, Root};

const HOOK: &str = include_str!("templates/pre-commit.sh");
const PLUGIN: &str = include_str!("templates/opencode-plugin.js");
const FRAGMENT: &str = include_str!("templates/agents-fragment.md");
const PROMOTER: &str = include_str!("../../../spec/agents/spec-promoter.md");
const HEALTH: &str = include_str!("../../../spec/agents/spec-health.md");

const HOOK_MARKER: &str = "# vibe-spec pre-commit v1";
const MD_MARKER: &str = "<!-- vibe-spec:opencode v1 -->";
const JS_MARKER: &str = "// vibe-spec:opencode v1";
const ROUTER_START: &str = "<!-- vibe-spec:router:start -->";
const ROUTER_END: &str = "<!-- vibe-spec:router:end -->";

/// Which targets to set up.
pub struct Options {
    /// Install `<git hooks dir>/pre-commit`.
    pub git_hooks: bool,
    /// Write `.opencode/agent/*.md` and `.opencode/plugin/vibe-spec.js`.
    pub opencode: bool,
    /// Add or refresh the router block in `AGENTS.md`.
    pub agents_md: bool,
    /// Print Spec JSON v1.
    pub json: bool,
}

/// One planned file.
struct Target {
    path: PathBuf,
    label: String,
    bytes: Vec<u8>,
    exec: bool,
    /// The mode to keep when replacing an existing file.
    keep_mode: Option<u32>,
    unchanged: bool,
}

/// Runs the command.
pub fn run(_found: &Result<Root, Miss>, cwd: &Path, opts: &Options) -> (u8, Vec<u8>) {
    if !(opts.git_hooks || opts.opencode || opts.agents_md) {
        eprintln!("vibe-spec setup: pass at least one of --git-hooks, --opencode, --agents-md");
        return (exit::USAGE, Vec::new());
    }
    let project = crate::git::top(cwd).unwrap_or_else(|| cwd.to_path_buf());
    let mut errors: Vec<String> = Vec::new();
    let mut targets: Vec<Target> = Vec::new();
    let mut add = |r: Result<Target, String>| match r {
        Ok(t) => targets.push(t),
        Err(e) => errors.push(e),
    };

    if opts.git_hooks {
        add(hooks_dir(cwd).and_then(|dir| {
            marked(
                &project,
                dir.join("pre-commit"),
                HOOK_MARKER,
                HOOK.as_bytes().to_vec(),
                true,
            )
        }));
    }
    if opts.opencode {
        for (name, src) in [("spec-promoter", PROMOTER), ("spec-health", HEALTH)] {
            let path = project.join(".opencode/agent").join(format!("{name}.md"));
            add(opencode_agent(src)
                .map_err(|e| format!("{name}: {e}"))
                .and_then(|bytes| marked(&project, path, MD_MARKER, bytes, false)));
        }
        let path = project.join(".opencode/plugin/vibe-spec.js");
        add(marked(
            &project,
            path,
            JS_MARKER,
            PLUGIN.as_bytes().to_vec(),
            false,
        ));
    }
    if opts.agents_md {
        add(router(&project));
    }

    if !errors.is_empty() {
        for e in &errors {
            eprintln!("vibe-spec setup: {e}");
        }
        return (exit::FINDINGS, Vec::new());
    }

    let mut written = Vec::new();
    let mut skipped = Vec::new();
    for t in &targets {
        if t.unchanged {
            skipped.push(t.label.clone());
        } else if let Err(e) = write(t) {
            eprintln!("vibe-spec setup: {}: {e}", t.label);
            return (exit::FINDINGS, Vec::new());
        } else {
            written.push(t.label.clone());
        }
    }
    if opts.json {
        let v = serde_json::json!({"v": 1, "written": written, "skipped": skipped});
        return (exit::OK, format!("{v}\n").into_bytes());
    }
    let mut out = String::new();
    for w in &written {
        out.push_str(&format!("WROTE: {w}\n"));
    }
    for s in &skipped {
        out.push_str(&format!("SKIP: {s}\n"));
    }
    (exit::OK, out.into_bytes())
}

/// `git rev-parse --git-path hooks`, absolute; honours worktrees and `core.hooksPath`.
fn hooks_dir(cwd: &Path) -> Result<PathBuf, String> {
    let out = Command::new("git")
        .args(["rev-parse", "--git-path", "hooks"])
        .current_dir(cwd)
        .output()
        .map_err(|e| format!("cannot run git: {e}"))?;
    if !out.status.success() {
        return Err("not a git repository".to_string());
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let rel = text.trim_end_matches('\n');
    if rel.is_empty() {
        return Err("git printed no hooks path".to_string());
    }
    Ok(cwd.join(rel))
}

fn label(project: &Path, path: &Path) -> String {
    path.strip_prefix(project)
        .unwrap_or(path)
        .to_string_lossy()
        .into_owned()
}

/// The existing file at `path`: `None` when absent; an error for a symlink or
/// a non-file (also for a symlinked `.opencode` directory).
fn existing(project: &Path, path: &Path) -> Result<Option<(Vec<u8>, u32)>, String> {
    let shown = label(project, path);
    if let Ok(rel) = path.strip_prefix(project) {
        let mut cur = project.to_path_buf();
        for part in rel.components() {
            cur.push(part);
            match fs::symlink_metadata(&cur) {
                Ok(m) if m.file_type().is_symlink() => {
                    return Err(format!(
                        "{shown}: {} is a symlink; not writing through it",
                        label(project, &cur)
                    ));
                }
                Ok(_) => {}
                Err(_) => return Ok(None),
            }
        }
    }
    match fs::symlink_metadata(path) {
        Err(_) => Ok(None),
        Ok(m) if !m.is_file() => Err(format!("{shown}: not a regular file")),
        Ok(m) => {
            let bytes = fs::read(path).map_err(|e| format!("{shown}: {e}"))?;
            Ok(Some((bytes, mode_of(&m))))
        }
    }
}

#[cfg(unix)]
fn mode_of(m: &fs::Metadata) -> u32 {
    use std::os::unix::fs::PermissionsExt;
    m.permissions().mode() & 0o7777
}

#[cfg(not(unix))]
fn mode_of(_: &fs::Metadata) -> u32 {
    0o644
}

fn contains(hay: &[u8], needle: &str) -> bool {
    find(hay, needle).is_some()
}

fn find(hay: &[u8], needle: &str) -> Option<usize> {
    let n = needle.as_bytes();
    hay.windows(n.len()).position(|w| w == n)
}

/// A file that must carry `marker`: new, or ours.
fn marked(
    project: &Path,
    path: PathBuf,
    marker: &str,
    bytes: Vec<u8>,
    exec: bool,
) -> Result<Target, String> {
    let shown = label(project, &path);
    let old = existing(project, &path)?;
    let mut unchanged = false;
    if let Some((old_bytes, mode)) = &old {
        if !contains(old_bytes, marker) {
            return Err(format!(
                "{shown} exists and is not managed by vibe-spec (no `{marker}`); not replacing it"
            ));
        }
        unchanged = *old_bytes == bytes && (!exec || mode & 0o111 == 0o111);
    }
    Ok(Target {
        label: shown,
        path,
        bytes,
        exec,
        keep_mode: None,
        unchanged,
    })
}

/// The AGENTS.md router block: appended, or replaced in place.
fn router(project: &Path) -> Result<Target, String> {
    let path = project.join("AGENTS.md");
    let shown = label(project, &path);
    let block = FRAGMENT.trim_end_matches('\n');
    let old = existing(project, &path)?;
    let (bytes, keep_mode) = match &old {
        None => (format!("{block}\n").into_bytes(), None),
        Some((text, mode)) => {
            let start = find(text, ROUTER_START);
            let end = find(text, ROUTER_END);
            let bytes = match (start, end) {
                (None, None) => {
                    let mut v = text.clone();
                    if !v.is_empty() {
                        if !v.ends_with(b"\n") {
                            v.push(b'\n');
                        }
                        v.push(b'\n');
                    }
                    v.extend_from_slice(block.as_bytes());
                    v.push(b'\n');
                    v
                }
                (Some(s), Some(e)) if s < e => {
                    let mut v = text[..s].to_vec();
                    v.extend_from_slice(block.as_bytes());
                    v.extend_from_slice(&text[e + ROUTER_END.len()..]);
                    v
                }
                _ => {
                    return Err(format!(
                        "{shown} has an unpaired or misordered vibe-spec router marker; fix it by hand"
                    ));
                }
            };
            (bytes, Some(*mode))
        }
    };
    let unchanged = old.as_ref().is_some_and(|(t, _)| *t == bytes);
    Ok(Target {
        label: shown,
        path,
        bytes,
        exec: false,
        keep_mode,
        unchanged,
    })
}

fn write(t: &Target) -> Result<(), String> {
    if let Some(dir) = t.path.parent() {
        fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    vibe_core::fs::write_atomic(&t.path, &t.bytes).map_err(|e| e.to_string())?;
    set_mode(
        &t.path,
        t.keep_mode.unwrap_or(if t.exec { 0o755 } else { 0o644 }),
    )
}

#[cfg(unix)]
fn set_mode(path: &Path, mode: u32) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(mode)).map_err(|e| e.to_string())
}

#[cfg(not(unix))]
fn set_mode(_: &Path, _: u32) -> Result<(), String> {
    Ok(())
}

/// Converts a Claude agent file to OpenCode's format: `description`,
/// `mode: subagent`, `model` and a `tools` map; the body stays verbatim.
fn opencode_agent(src: &str) -> Result<Vec<u8>, String> {
    let rest = src.strip_prefix("---\n").ok_or("no frontmatter")?;
    let end = rest.find("\n---\n").ok_or("unterminated frontmatter")?;
    let (front, body) = (&rest[..end], &rest[end + 5..]);
    let (mut description, mut model, mut tools) = (None, None, Vec::new());
    for line in front.lines() {
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let value = value.trim();
        match key.trim() {
            "description" => description = Some(value.to_string()),
            "model" => model = Some(value.to_string()),
            "tools" => tools = value.split(',').map(|t| t.trim().to_lowercase()).collect(),
            _ => {}
        }
    }
    let description = description.ok_or("no description")?;
    let mut out = String::from("---\n");
    out.push_str(&format!(
        "description: {}\n",
        serde_json::Value::String(description)
    ));
    out.push_str("mode: subagent\n");
    let id = match model.as_deref() {
        Some("sonnet") => Some("anthropic/claude-sonnet-4-5"),
        Some("haiku") => Some("anthropic/claude-haiku-4-5"),
        Some("opus") => Some("anthropic/claude-opus-4-5"),
        _ => None,
    };
    if let Some(id) = id {
        out.push_str(&format!("model: {id}\n"));
    }
    out.push_str("tools:\n");
    for t in ["read", "write", "edit", "bash"] {
        out.push_str(&format!("  {t}: {}\n", tools.iter().any(|x| x == t)));
    }
    for t in tools
        .iter()
        .filter(|t| !["read", "write", "edit", "bash"].contains(&t.as_str()))
    {
        out.push_str(&format!("  {t}: true\n"));
    }
    out.push_str("---\n");
    out.push_str(MD_MARKER);
    out.push('\n');
    out.push_str(body);
    Ok(out.into_bytes())
}
