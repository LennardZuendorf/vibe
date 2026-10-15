//! `vibe-spec validate` — port of `spec/scripts/validate.sh`.
//!
//! F16 ports it byte for byte: ANSI colours, `echo -e` escapes, the error
//! and warning counters (including the one warning the oracle prints from a
//! pipeline subshell and so never counts), and the exit code. F17 changes
//! behaviour. Deliberate divergence (pinned): with no spec root, the error
//! names `vibe-spec init`, not the oracle's `setup.sh` path.
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use vibe_core::exit;

use crate::pattern::{md_link_target, md_links, paren_links};
use crate::root::{Miss, Root};
use crate::text::{
    echo_e, first_line, has_line_starting, is_space, join_raw, lines, md_files, name_of,
    read_lines, strip_prefix_or, sub_dirs, trim_start_space,
};

const ROOT_NAMES: [&str; 5] = [
    "product.md",
    "tech.md",
    "design.md",
    "plan.md",
    "lessons.md",
];
const ENTRYPOINTS: [&str; 4] = ["product.md", "tech.md", "design.md", "plan.md"];

/// validate's stdout and its counters.
#[derive(Default)]
pub struct Report {
    /// Everything printed so far.
    pub out: Vec<u8>,
    /// `ERRORS`.
    pub errors: u32,
    /// `WARNINGS`.
    pub warnings: u32,
}

impl Report {
    fn colour(&mut self, code: &str, tag: &str, msg: &[u8]) {
        self.out
            .extend(echo_e(&cat!("\\033[", code, "m  ", tag, msg, "\\033[0m")));
    }

    /// `red`: an error.
    pub fn red(&mut self, msg: &[u8]) {
        self.colour("31", "ERROR: ", msg);
        self.errors += 1;
    }

    /// `yellow`: a warning.
    pub fn yellow(&mut self, msg: &[u8]) {
        self.colour("33", "WARN:  ", msg);
        self.warnings += 1;
    }

    /// `yellow` called inside `cmd | while read`: printed, never counted,
    /// because the increment happens in a subshell.
    pub fn yellow_uncounted(&mut self, msg: &[u8]) {
        self.colour("33", "WARN:  ", msg);
    }

    /// `green`.
    pub fn green(&mut self, msg: &[u8]) {
        self.colour("32", "OK:    ", msg);
    }

    /// `info`.
    pub fn info(&mut self, msg: &[u8]) {
        self.colour("36", "INFO:  ", msg);
    }

    /// Plain `echo "$msg"`.
    pub fn line(&mut self, msg: &[u8]) {
        self.out.extend_from_slice(msg);
        self.out.push(b'\n');
    }
}

/// Runs the command.
pub fn run(found: &Result<Root, Miss>, cwd: &Path, args: &[String]) -> (u8, Vec<u8>) {
    if let Some(arg) = args.first() {
        eprintln!("vibe-spec validate: unexpected argument {arg} (F17 adds validate's flags)");
        return (exit::USAGE, Vec::new());
    }
    let (dir, project, display): (PathBuf, PathBuf, String) = match found {
        Ok(root) => (root.dir.clone(), root.project.clone(), root.display()),
        Err(Miss::NotFound) => (cwd.join(".spec"), cwd.to_path_buf(), ".spec".to_string()),
        Err(miss) => {
            eprintln!("{}", miss.message());
            return (miss.code(), Vec::new());
        }
    };
    let mut r = Report::default();
    r.line(format!("Validating {display}/...").as_bytes());
    r.line(b"");
    if !dir.is_dir() {
        r.red(b"No spec root found. Run setup first: vibe-spec init");
        return (exit::FINDINGS, r.out);
    }
    let specs = md_files(&dir);
    if specs.is_empty() {
        r.red(format!("No spec files found in {display}/").as_bytes());
        return (exit::FINDINGS, r.out);
    }
    let ctx = Ctx {
        dir: &dir,
        project: &project,
        display: &display,
    };
    for file in &specs {
        root_file(&mut r, &ctx, file);
    }
    for name in ENTRYPOINTS {
        children(&mut r, &dir, name);
    }
    let features = dir.join("features");
    if features.is_dir() {
        for fdir in sub_dirs(&features) {
            feature(&mut r, &ctx, &fdir);
        }
    }
    claude_md(&mut r, &project, &display);
    sf13(&mut r, &dir, &display);
    sf15(&mut r, &dir, &display);
    sf16(&mut r, &dir);
    let (errors, warnings) = (r.errors, r.warnings);
    r.line(b"========================");
    r.line(format!("Errors:   {errors}").as_bytes());
    r.line(format!("Warnings: {warnings}").as_bytes());
    (if errors > 0 { exit::FINDINGS } else { exit::OK }, r.out)
}

/// Where the spec lives.
struct Ctx<'a> {
    /// The spec root (absolute).
    dir: &'a Path,
    /// The project directory: the oracle's cwd after it `cd`s to the root's parent.
    project: &'a Path,
    /// The root relative to the project, as the oracle spells `$SPEC_DIR` (`.spec`).
    display: &'a str,
}

fn read(path: &Path) -> Vec<u8> {
    std::fs::read(path).unwrap_or_default()
}

/// The root-layer loop body for one `$SPEC_DIR/*.md` file.
fn root_file(r: &mut Report, ctx: &Ctx, file: &Path) {
    let name = name_of(file);
    let n = name.as_bytes();
    r.line(&cat!("--- ", n, " ---"));
    if name == "lessons.md" {
        r.green(&cat!(
            n,
            ": checked (lessons file, no frontmatter required)"
        ));
        r.line(b"");
        return;
    }
    let text = read(file);
    if first_line(&text) != b"---" {
        r.red(&cat!(n, ": missing YAML frontmatter"));
        return;
    }
    if !has_line_starting(&text, b"type:") {
        r.red(&cat!(n, ": missing 'type:' in frontmatter"));
    }
    if !has_line_starting(&text, b"updated:") {
        r.red(&cat!(n, ": missing 'updated:' in frontmatter"));
    }
    if has_line_starting(&text, b"type: entrypoint") && !has_line_starting(&text, b"children:") {
        if name == "plan.md" {
            r.yellow(&cat!(
                n,
                ": entrypoint has no 'children:' list (add sub-plans if needed)"
            ));
        } else {
            r.red(&cat!(n, ": entrypoint missing 'children:' list"));
        }
    }
    let branch = [
        &b"type: product-topic"[..],
        b"type: tech-topic",
        b"type: plan-topic",
    ];
    if branch.iter().any(|p| has_line_starting(&text, p)) {
        if !has_line_starting(&text, b"parent:") {
            r.red(&cat!(n, ": branch doc missing 'parent:' field"));
        }
        if !has_line_starting(&text, b"scope:") {
            r.yellow(&cat!(n, ": branch doc missing 'scope:' field"));
        }
        if !has_line_starting(&text, b"covers:") {
            r.yellow(&cat!(n, ": branch doc missing 'covers:' field"));
        }
    }
    if !ROOT_NAMES.contains(&name.as_str()) {
        if !["product-", "tech-", "plan-"]
            .iter()
            .any(|p| name.starts_with(p))
        {
            r.red(&cat!(n, ": must start with 'product-', 'tech-', or 'plan-' (e.g., product-design.md, tech-api.md, plan-editor.md)"));
        }
        if let Some(topic) = topic_of(&name)
            && !topic_ok(topic.as_bytes())
        {
            r.red(&cat!(n, ": topic must be lowercase with hyphens only, no leading/trailing hyphens (found: '", topic, "')"));
        }
    }
    if name == "design.md" {
        lint_design(r, ctx.project, &format!("{}/design.md", ctx.display), n);
    }
    for line in lines(&text) {
        for link in md_links(line) {
            let target = md_link_target(link);
            if target.is_empty()
                || !target.ends_with(b".md")
                || target.starts_with(b"http")
                || target.contains(&b'{')
            {
                continue;
            }
            let parent = file.parent().unwrap_or(ctx.dir);
            if !join_raw(ctx.dir, &target).is_file() && !join_raw(parent, &target).is_file() {
                r.red(&cat!(n, ": broken link to '", target, "'"));
            }
        }
    }
    r.green(&cat!(n, ": checked"));
    r.line(b"");
}

/// `^(product|tech|plan)-(.+)\.md$` → the topic.
fn topic_of(name: &str) -> Option<&str> {
    ["product-", "tech-", "plan-"].iter().find_map(|p| {
        let rest = name.strip_prefix(p)?;
        (rest.len() >= 4 && rest.ends_with(".md")).then(|| &rest[..rest.len() - 3])
    })
}

/// `^[a-z0-9]([a-z0-9-]*[a-z0-9])?$`.
fn topic_ok(t: &[u8]) -> bool {
    let edge = |c: &u8| c.is_ascii_lowercase() || c.is_ascii_digit();
    t.first().is_some_and(edge)
        && t.last().is_some_and(edge)
        && t.iter().all(|c| edge(c) || *c == b'-')
}

/// "Validate entrypoint children exist" for one entrypoint.
fn children(r: &mut Report, dir: &Path, name: &str) {
    let path = dir.join(name);
    if !path.is_file() {
        return;
    }
    let text = read(&path);
    let mut in_children = false;
    for line in read_lines(&text) {
        if line == b"children:" {
            in_children = true;
            continue;
        }
        if !in_children {
            continue;
        }
        match list_item(line) {
            Some(child) => {
                if !child.contains(&b'{') && !join_raw(dir, child).is_file() {
                    r.red(&cat!(
                        name,
                        ": child '",
                        child,
                        "' listed in frontmatter but file doesn't exist"
                    ));
                }
            }
            None => in_children = false,
        }
    }
}

/// `^[[:space:]]*-[[:space:]]+(.*)` → the text after the dash and its spaces.
fn list_item(line: &[u8]) -> Option<&[u8]> {
    let rest = trim_start_space(line).strip_prefix(b"-")?;
    let after = trim_start_space(rest);
    (after.len() < rest.len()).then_some(after)
}

/// "Validate feature folders" for one feature directory.
fn feature(r: &mut Report, ctx: &Ctx, fdir: &Path) {
    let fname = name_of(fdir);
    let label = format!("features/{fname}");
    r.line(format!("--- {label}/ ---").as_bytes());
    for required in ["product.md", "tech.md"] {
        let file = fdir.join(required);
        if !file.is_file() {
            r.red(format!("{label}/: missing required '{required}'").as_bytes());
            continue;
        }
        let doc = format!("{label}/{required}");
        let text = read(&file);
        if !feature_doc(r, &text, &doc, required) {
            continue;
        }
        for line in lines(&text) {
            for link in md_links(line) {
                let target = md_link_target(link);
                if target.is_empty() || !target.ends_with(b".md") || target.starts_with(b"http") {
                    continue;
                }
                if (target == b"design.md" || target == b"plan.md")
                    && !join_raw(fdir, &target).is_file()
                {
                    continue;
                }
                if !resolve_in(fdir, &target).is_file() {
                    r.red(&cat!(doc, ": broken link to '", target, "'"));
                }
            }
        }
    }
    for optional in ["design.md", "plan.md"] {
        let file = fdir.join(optional);
        if !file.is_file() {
            continue;
        }
        let doc = format!("{label}/{optional}");
        let text = read(&file);
        if !feature_doc(r, &text, &doc, optional) {
            continue;
        }
        if optional == "design.md" {
            lint_design(
                r,
                ctx.project,
                &format!("{}/{label}/design.md", ctx.display),
                doc.as_bytes(),
            );
        }
    }
    r.green(format!("{label}/: checked").as_bytes());
    r.line(b"");
}

/// Frontmatter, `type:`/`updated:` and expected-type checks of one feature
/// doc. False when it has no frontmatter (the oracle `continue`s).
fn feature_doc(r: &mut Report, text: &[u8], doc: &str, file: &str) -> bool {
    if first_line(text) != b"---" {
        r.red(format!("{doc}: missing YAML frontmatter").as_bytes());
        return false;
    }
    if !has_line_starting(text, b"type:") {
        r.red(format!("{doc}: missing 'type:' in frontmatter").as_bytes());
    }
    if !has_line_starting(text, b"updated:") {
        r.red(format!("{doc}: missing 'updated:' in frontmatter").as_bytes());
    }
    let expected = format!("feature-{}", file.trim_end_matches(".md"));
    if !has_line_starting(text, format!("type: {expected}").as_bytes()) {
        r.yellow(format!("{doc}: expected 'type: {expected}'").as_bytes());
    }
    true
}

/// `cd "$feature_dir" && [[ -f "$target" ]]`: absolute targets stay absolute.
fn resolve_in(dir: &Path, target: &[u8]) -> PathBuf {
    if target.first() == Some(&b'/') {
        join_raw(Path::new(""), target.get(1..).unwrap_or_default())
    } else {
        join_raw(dir, target)
    }
}

/// "Validate CLAUDE.md links": only links into the spec root are checked.
fn claude_md(r: &mut Report, project: &Path, display: &str) {
    let path = project.join("CLAUDE.md");
    if !path.is_file() {
        return;
    }
    r.line(b"--- CLAUDE.md ---");
    let prefix = format!("{display}/");
    let text = read(&path);
    for line in lines(&text) {
        for link in md_links(line) {
            let target = md_link_target(link);
            let into_spec = target.starts_with(prefix.as_bytes())
                && target.ends_with(b".md")
                && target.len() >= prefix.len() + 3;
            if into_spec && !join_raw(project, &target).is_file() {
                r.red(&cat!("CLAUDE.md: broken link to '", target, "'"));
            }
        }
    }
    r.green(b"CLAUDE.md: checked");
    r.line(b"");
}

/// SF13: stale non-`.md` links to `features/` or `archive/` from the root hubs.
fn sf13(r: &mut Report, dir: &Path, display: &str) {
    for name in ENTRYPOINTS {
        let path = dir.join(name);
        if !path.is_file() {
            continue;
        }
        let text = read(&path);
        for line in lines(&text) {
            for target in paren_links(line) {
                if target.ends_with(b".md")
                    || target.contains(&b'{')
                    || join_raw(dir, target).exists()
                {
                    continue;
                }
                r.yellow(&cat!(
                    "SF13: stale link in ",
                    display,
                    "/",
                    name,
                    " → ",
                    target,
                    " (not found)"
                ));
            }
        }
    }
}

/// SF15: root spec length over `$SPEC_ROOT_MAX_LINES` (default 300) newlines.
fn sf15(r: &mut Report, dir: &Path, display: &str) {
    let raw = std::env::var("SPEC_ROOT_MAX_LINES")
        .ok()
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| "300".to_string());
    // bash evaluates a non-numeric value as 0.
    let max: usize = raw.parse().unwrap_or(0);
    for name in ROOT_NAMES {
        let path = dir.join(name);
        if !path.is_file() {
            continue;
        }
        let count = read(&path).iter().filter(|&&b| b == b'\n').count();
        if count > max {
            r.yellow(format!("SF15: {display}/{name} is {count} lines (>{raw}); consider splitting into a branch doc (product-{{topic}}.md, tech-{{topic}}.md)").as_bytes());
        }
    }
}

/// SF16: lessons without a `**Tags:**` line. A final line without a
/// newline is not read (the oracle's `while read` drops it).
fn sf16(r: &mut Report, dir: &Path) {
    let path = dir.join("lessons.md");
    if !path.is_file() {
        return;
    }
    let text = read(&path);
    let mut title: Vec<u8> = Vec::new();
    let mut has_tags = false;
    let flag = |r: &mut Report, title: &[u8], has_tags: bool| {
        if !title.is_empty() && !has_tags {
            r.yellow(&cat!(
                "SF16: lesson '",
                title,
                "' in lessons.md has no **Tags:** line"
            ));
        }
    };
    for line in read_lines(&text) {
        if line.starts_with(b"###") && line.get(3).is_some_and(|&c| is_space(c)) {
            flag(r, &title, has_tags);
            title = strip_prefix_or(line, b"### ").to_vec();
            has_tags = false;
        } else if line.starts_with(b"**Tags:**") {
            has_tags = true;
        }
    }
    flag(r, &title, has_tags);
}

/// SF4: the optional external design.md linter (`VIBE_DESIGN_LINT=1`),
/// advisory and graceful-degrade, run from the project like the oracle.
fn lint_design(r: &mut Report, project: &Path, rel: &str, label: &[u8]) {
    if std::env::var_os("VIBE_DESIGN_LINT").as_deref() != Some(OsStr::new("1")) {
        return;
    }
    if !on_path("node") || !on_path("npx") {
        r.yellow(&cat!(
            label,
            ": design.md lint skipped (node/npx not found)"
        ));
        return;
    }
    let reachable = Command::new("curl")
        .args(["-fsS", "--max-time", "3", "https://registry.npmjs.org/"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|s| s.success());
    if !reachable {
        r.yellow(&cat!(
            label,
            ": design.md lint skipped (npm registry unreachable)"
        ));
        return;
    }
    let (ok, mut text) = match Command::new("npx")
        .args(["--yes", "@google/design.md", "lint", rel])
        .current_dir(project)
        .stdin(Stdio::null())
        .output()
    {
        Ok(o) => (o.status.success(), [o.stdout, o.stderr].concat()),
        Err(_) => (false, Vec::new()),
    };
    if ok {
        return;
    }
    r.yellow(&cat!(label, ": design.md lint reported issues (advisory)"));
    while text.last() == Some(&b'\n') {
        text.pop();
    }
    for line in text.split(|&b| b == b'\n') {
        if !line.is_empty() {
            r.yellow(&cat!(label, ":   ", line));
        }
    }
}

/// `command -v <name>`: an executable file in a `PATH` directory.
fn on_path(name: &str) -> bool {
    let Some(path) = std::env::var_os("PATH") else {
        return false;
    };
    std::env::split_paths(&path).any(|d| executable(&d.join(name)))
}

#[cfg(unix)]
fn executable(p: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    p.metadata()
        .is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
}

#[cfg(not(unix))]
fn executable(p: &Path) -> bool {
    p.is_file()
}
