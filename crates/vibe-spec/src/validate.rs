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

use crate::adherence;
use crate::check;
use crate::content;
use crate::pattern::{md_link_target, md_links, paren_links};
use crate::report::Report;
use crate::root::{Miss, Root};
use crate::text::{
    first_line, has_line_starting, is_space, join_raw, lines, md_files, name_of, read_lines,
    strip_prefix_or, sub_dirs, trim_start_space,
};

const ROOT_NAMES: [&str; 5] = [
    "product.md",
    "tech.md",
    "design.md",
    "plan.md",
    "lessons.md",
];
const ENTRYPOINTS: [&str; 4] = ["product.md", "tech.md", "design.md", "plan.md"];

/// `--format`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
pub enum Format {
    /// The oracle's coloured report.
    Text,
    /// Spec JSON v1 `validate`.
    Json,
}

/// Parsed `validate` flags.
pub struct Options {
    /// `--strict`.
    pub strict: bool,
    /// `--format` (`--json` sets `Json`).
    pub format: Format,
}

/// Runs the command.
pub fn run(found: &Result<Root, Miss>, cwd: &Path, opts: &Options) -> (u8, Vec<u8>) {
    let (dir, project, display): (PathBuf, PathBuf, String) = match found {
        Ok(root) => (root.dir.clone(), root.project.clone(), root.display()),
        Err(Miss::NotFound) if opts.format == Format::Json => {
            eprintln!("{}", Miss::NotFound.message());
            return (exit::NO_ROOT, Vec::new());
        }
        Err(Miss::NotFound) => (cwd.join(".spec"), cwd.to_path_buf(), ".spec".to_string()),
        Err(miss) => {
            eprintln!("{}", miss.message());
            return (miss.code(), Vec::new());
        }
    };
    let levels = match check::load(&project, opts.strict) {
        Ok(levels) => levels,
        Err(why) => {
            eprintln!("vibe-spec: {why}");
            return (exit::USAGE, Vec::new());
        }
    };
    let mut r = Report::new(&project, levels);
    r.line(format!("Validating {display}/...").as_bytes());
    r.line(b"");
    if !dir.is_dir() {
        r.at("");
        r.refuse(b"No spec root found. Run setup first: vibe-spec init");
        return finish(r, &display, opts.format, false);
    }
    let specs = md_files(&dir);
    if specs.is_empty() {
        r.at("");
        r.refuse(format!("No spec files found in {display}/").as_bytes());
        return finish(r, &display, opts.format, false);
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
        children(&mut r, &ctx, name);
    }
    let features = dir.join("features");
    if features.is_dir() {
        for fdir in sub_dirs(&features) {
            feature(&mut r, &ctx, &fdir);
        }
    }
    claude_md(&mut r, &project, &display);
    sf13(&mut r, &dir, &display);
    content::sf14(&mut r, &dir, &display);
    sf15(&mut r, &dir, &display);
    sf16(&mut r, &dir, &display);
    let files = adherence::flat_files(&dir);
    adherence::run(&mut r, &ctx, &files, false);
    if opts.strict && (r.examined() == 0 || r.levels().all_off()) {
        r.at("");
        r.refuse(b"validate --strict examined nothing (no spec file, or every check is off); refusing to report green");
    }
    finish(r, &display, opts.format, true)
}

/// The run's answer: the exit code and stdout in the chosen format. Text
/// mode closes a full run with the oracle's summary lines.
fn finish(mut r: Report, display: &str, format: Format, summary: bool) -> (u8, Vec<u8>) {
    let code = if r.errors > 0 {
        exit::FINDINGS
    } else {
        exit::OK
    };
    match format {
        Format::Json => (code, r.json(display, "flat")),
        Format::Text => {
            if summary {
                let (errors, warnings) = (r.errors, r.warnings);
                r.line(b"========================");
                r.line(format!("Errors:   {errors}").as_bytes());
                r.line(format!("Warnings: {warnings}").as_bytes());
            }
            (code, r.out)
        }
    }
}

/// Where the spec lives.
pub struct Ctx<'a> {
    /// The spec root (absolute).
    pub dir: &'a Path,
    /// The project directory: the oracle's cwd after it `cd`s to the root's parent.
    pub project: &'a Path,
    /// The root relative to the project, as the oracle spells `$SPEC_DIR` (`.spec`).
    pub display: &'a str,
}

fn read(path: &Path) -> Vec<u8> {
    std::fs::read(path).unwrap_or_default()
}

/// The root-layer loop body for one `$SPEC_DIR/*.md` file.
fn root_file(r: &mut Report, ctx: &Ctx, file: &Path) {
    let name = name_of(file);
    let n = name.as_bytes();
    let dsp = ctx.display;
    r.line(&cat!("--- ", n, " ---"));
    r.examine(&format!("{dsp}/{name}"));
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
        r.error("SF20", &cat!(n, ": missing YAML frontmatter"));
        return;
    }
    if !has_line_starting(&text, b"type:") {
        r.error("SF20", &cat!(n, ": missing 'type:' in frontmatter"));
    }
    if !has_line_starting(&text, b"updated:") {
        r.error("SF20", &cat!(n, ": missing 'updated:' in frontmatter"));
    }
    if has_line_starting(&text, b"type: entrypoint") && !has_line_starting(&text, b"children:") {
        if name == "plan.md" {
            r.warn(
                "SF20",
                &cat!(
                    n,
                    ": entrypoint has no 'children:' list (add sub-plans if needed)"
                ),
            );
        } else {
            r.error("SF20", &cat!(n, ": entrypoint missing 'children:' list"));
        }
    }
    let branch = [
        &b"type: product-topic"[..],
        b"type: tech-topic",
        b"type: plan-topic",
    ];
    if branch.iter().any(|p| has_line_starting(&text, p)) {
        if !has_line_starting(&text, b"parent:") {
            r.error("SF20", &cat!(n, ": branch doc missing 'parent:' field"));
        }
        if !has_line_starting(&text, b"scope:") {
            r.warn("SF20", &cat!(n, ": branch doc missing 'scope:' field"));
        }
        if !has_line_starting(&text, b"covers:") {
            r.warn("SF20", &cat!(n, ": branch doc missing 'covers:' field"));
        }
    }
    if !ROOT_NAMES.contains(&name.as_str()) {
        if !["product-", "tech-", "plan-"]
            .iter()
            .any(|p| name.starts_with(p))
        {
            r.error("SF21", &cat!(n, ": must start with 'product-', 'tech-', or 'plan-' (e.g., product-design.md, tech-api.md, plan-editor.md)"));
        }
        if let Some(topic) = topic_of(&name)
            && !topic_ok(topic.as_bytes())
        {
            r.error("SF21", &cat!(n, ": topic must be lowercase with hyphens only, no leading/trailing hyphens (found: '", topic, "')"));
        }
    }
    if name == "design.md" {
        content::design_tokens(r, &text, n);
        lint_design(r, ctx.project, &format!("{}/design.md", ctx.display), n);
    }
    if name == "plan.md" {
        content::root_plan_milestones(r, &text, n);
    }
    content::requirements_scenarios(r, &text, n);
    content::scenario_grammar(r, &text, n);
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
                r.error("SF22", &cat!(n, ": broken link to '", target, "'"));
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
fn children(r: &mut Report, ctx: &Ctx, name: &str) {
    let dir = ctx.dir;
    let path = dir.join(name);
    if !path.is_file() {
        return;
    }
    r.at(&format!("{}/{name}", ctx.display));
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
                    r.error(
                        "SF22",
                        &cat!(
                            name,
                            ": child '",
                            child,
                            "' listed in frontmatter but file doesn't exist"
                        ),
                    );
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
    let dsp = ctx.display;
    r.line(format!("--- {label}/ ---").as_bytes());
    for required in ["product.md", "tech.md"] {
        let file = fdir.join(required);
        if !file.is_file() {
            r.at(&format!("{dsp}/{label}/{required}"));
            r.error(
                "SF23",
                format!("{label}/: missing required '{required}'").as_bytes(),
            );
            continue;
        }
        let doc = format!("{label}/{required}");
        r.examine(&format!("{dsp}/{label}/{required}"));
        let text = read(&file);
        if !feature_doc(r, &text, &doc, required) {
            continue;
        }
        let kind = required.trim_end_matches(".md");
        content::feature_frontmatter(r, &text, doc.as_bytes(), kind);
        if required == "product.md" {
            content::feature_scope(r, &text, doc.as_bytes());
            content::requirement_population(r, &text, doc.as_bytes());
        }
        content::requirements_scenarios(r, &text, doc.as_bytes());
        content::scenario_grammar(r, &text, doc.as_bytes());
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
                    r.error("SF22", &cat!(doc, ": broken link to '", target, "'"));
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
        r.examine(&format!("{dsp}/{label}/{optional}"));
        let text = read(&file);
        if !feature_doc(r, &text, &doc, optional) {
            continue;
        }
        content::feature_frontmatter(r, &text, doc.as_bytes(), optional.trim_end_matches(".md"));
        if optional == "design.md" {
            content::design_tokens(r, &text, doc.as_bytes());
            lint_design(
                r,
                ctx.project,
                &format!("{}/{label}/design.md", ctx.display),
                doc.as_bytes(),
            );
        }
        if optional == "plan.md" {
            content::plan_structure(r, &text, doc.as_bytes());
            content::id_traceability(r, &fdir.join("product.md"), &text, label.as_bytes());
            content::cross_deps(r, &text, &fname, doc.as_bytes());
        }
        content::requirements_scenarios(r, &text, doc.as_bytes());
        content::scenario_grammar(r, &text, doc.as_bytes());
    }
    r.green(format!("{label}/: checked").as_bytes());
    r.line(b"");
}

/// Frontmatter, `type:`/`updated:` and expected-type checks of one feature
/// doc. False when it has no frontmatter (the oracle `continue`s).
fn feature_doc(r: &mut Report, text: &[u8], doc: &str, file: &str) -> bool {
    if first_line(text) != b"---" {
        r.error(
            "SF20",
            format!("{doc}: missing YAML frontmatter").as_bytes(),
        );
        return false;
    }
    if !has_line_starting(text, b"type:") {
        r.error(
            "SF20",
            format!("{doc}: missing 'type:' in frontmatter").as_bytes(),
        );
    }
    if !has_line_starting(text, b"updated:") {
        r.error(
            "SF20",
            format!("{doc}: missing 'updated:' in frontmatter").as_bytes(),
        );
    }
    let expected = format!("feature-{}", file.trim_end_matches(".md"));
    if !has_line_starting(text, format!("type: {expected}").as_bytes()) {
        r.warn(
            "SF20",
            format!("{doc}: expected 'type: {expected}'").as_bytes(),
        );
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
    r.examine("CLAUDE.md");
    let prefix = format!("{display}/");
    let text = read(&path);
    for line in lines(&text) {
        for link in md_links(line) {
            let target = md_link_target(link);
            let into_spec = target.starts_with(prefix.as_bytes())
                && target.ends_with(b".md")
                && target.len() >= prefix.len() + 3;
            // An outside `--root` displays as an absolute path; check it as is.
            let checked = resolve_in(project, &target);
            if into_spec && !checked.is_file() {
                r.error("SF22", &cat!("CLAUDE.md: broken link to '", target, "'"));
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
        r.at(&format!("{display}/{name}"));
        let text = read(&path);
        for line in lines(&text) {
            for target in paren_links(line) {
                if target.ends_with(b".md")
                    || target.contains(&b'{')
                    || join_raw(dir, target).exists()
                {
                    continue;
                }
                r.warn(
                    "SF13",
                    &cat!(
                        "SF13: stale link in ",
                        display,
                        "/",
                        name,
                        " → ",
                        target,
                        " (not found)"
                    ),
                );
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
    // `[[ $count -gt $max_lines ]]` is bash arithmetic; an invalid value is
    // an arithmetic error, so the comparison never succeeds.
    let max = parse_arith(&raw);
    for name in ROOT_NAMES {
        let path = dir.join(name);
        if !path.is_file() {
            continue;
        }
        r.at(&format!("{display}/{name}"));
        let count = read(&path).iter().filter(|&&b| b == b'\n').count();
        if max.is_some_and(|m| i64::try_from(count).is_ok_and(|c| c > m)) {
            r.warn("SF15", format!("SF15: {display}/{name} is {count} lines (>{raw}); consider splitting into a branch doc (product-{{topic}}.md, tech-{{topic}}.md)").as_bytes());
        }
    }
}

/// The subset of bash integer arithmetic SF15 needs: surrounding blanks,
/// optional sign, `0x` hex, leading-`0` octal, else decimal. `None` is an
/// arithmetic error (bad digit, bad base, overflow).
fn parse_arith(raw: &str) -> Option<i64> {
    let t = raw.trim_matches(|c: char| c.is_ascii_whitespace());
    let (neg, body) = match t.as_bytes().first()? {
        b'-' => (true, &t[1..]),
        b'+' => (false, &t[1..]),
        _ => (false, t),
    };
    let (radix, digits) =
        if let Some(h) = body.strip_prefix("0x").or_else(|| body.strip_prefix("0X")) {
            (16, h)
        } else if body.len() > 1 && body.starts_with('0') {
            (8, &body[1..])
        } else {
            (10, body)
        };
    if digits.is_empty() || !digits.bytes().all(|b| (b as char).is_digit(radix)) {
        return None;
    }
    let v = i64::from_str_radix(digits, radix).ok()?;
    Some(if neg { -v } else { v })
}

/// SF16: lessons without a `**Tags:**` line. A final line without a
/// newline is not read (the oracle's `while read` drops it).
fn sf16(r: &mut Report, dir: &Path, display: &str) {
    let path = dir.join("lessons.md");
    if !path.is_file() {
        return;
    }
    r.at(&format!("{display}/lessons.md"));
    let text = read(&path);
    let mut title: Vec<u8> = Vec::new();
    let mut has_tags = false;
    let flag = |r: &mut Report, title: &[u8], has_tags: bool| {
        if !title.is_empty() && !has_tags {
            r.warn(
                "SF16",
                &cat!(
                    "SF16: lesson '",
                    title,
                    "' in lessons.md has no **Tags:** line"
                ),
            );
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
        r.warn(
            "SF4",
            &cat!(label, ": design.md lint skipped (node/npx not found)"),
        );
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
        r.warn(
            "SF4",
            &cat!(label, ": design.md lint skipped (npm registry unreachable)"),
        );
        return;
    }
    let (ok, mut text) = match lint_command(rel)
        .current_dir(project)
        .stdin(Stdio::null())
        .output()
    {
        Ok(o) if cfg!(unix) => (o.status.success(), o.stdout),
        Ok(o) => (o.status.success(), [o.stdout, o.stderr].concat()),
        Err(_) => (false, Vec::new()),
    };
    if ok {
        return;
    }
    r.warn(
        "SF4",
        &cat!(label, ": design.md lint reported issues (advisory)"),
    );
    while text.last() == Some(&b'\n') {
        text.pop();
    }
    for line in text.split(|&b| b == b'\n') {
        if !line.is_empty() {
            r.warn("SF4", &cat!(label, ":   ", line));
        }
    }
}

/// `$(npx … 2>&1)`: on unix one shell pipe keeps stdout and stderr in write order.
#[cfg(unix)]
fn lint_command(rel: &str) -> Command {
    let mut cmd = Command::new("sh");
    cmd.args([
        "-c",
        "npx --yes @google/design.md lint \"$1\" 2>&1",
        "sh",
        rel,
    ]);
    cmd
}

#[cfg(not(unix))]
fn lint_command(rel: &str) -> Command {
    let mut cmd = Command::new("npx");
    cmd.args(["--yes", "@google/design.md", "lint", rel]);
    cmd
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

#[cfg(test)]
mod tests {
    use super::parse_arith;

    #[test]
    fn parse_arith_follows_bash() {
        assert_eq!(parse_arith("300"), Some(300));
        assert_eq!(parse_arith(" 300 "), Some(300));
        assert_eq!(parse_arith("010"), Some(8));
        assert_eq!(parse_arith("0x10"), Some(16));
        assert_eq!(parse_arith("-5"), Some(-5));
        assert_eq!(parse_arith("+7"), Some(7));
        assert_eq!(parse_arith("0"), Some(0));
        assert_eq!(parse_arith("1x"), None);
        assert_eq!(parse_arith("09"), None);
        assert_eq!(parse_arith("abc"), None);
        assert_eq!(parse_arith("99999999999999999999"), None);
    }
}
