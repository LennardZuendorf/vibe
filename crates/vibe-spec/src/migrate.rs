//! `vibe-spec migrate` (P4-R12, P4-R15): moves the spec root and rewrites
//! legacy-root literals. This half scans, plans, and reports (`--dry-run`,
//! `--check`); it writes nothing. [`plan`] returns a [`Plan`] that the execute
//! half applies.
//!
//! # The plan
//!
//! - `moves`: `(from, to)` pairs, project-relative and `/`-joined, in the order
//!   to run them. The root move (when the root is not already at `to`) comes
//!   first; each layered move after it names its post-root-move paths.
//! - `rewrites`: line edits. `file` is the path the file has once every move
//!   has run, `line` is 1-based in the file as it is on disk now, and `old` is
//!   the original text of one or more consecutive lines from `line` (joined by
//!   `\n`, no final newline). `new` replaces them. Edits to one file never
//!   overlap, so apply them from the bottom up. The list is sorted by
//!   `(file, line)`.
//! - `created`: new files, post-move path and bytes. None exists on disk, and
//!   none is a move target.
//! - `scanned`: files read. `before`: literals found.
//!
//! The plan does not bump `updated:` and does not choose allow-markers; both
//! are the execute half's, as is the `before >= 1` population check
//! ([`check_population`]) that applies only when the root must move.
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use vibe_core::exit;
use vibe_core::fs::{Write, commit, restore};
use vibe_core::glob::glob_match;

use crate::init::{normalise, slash};
use crate::literal;
use crate::root::{Miss, Root, Source};
use crate::text::{Fences, today_utc};
use crate::validate;

/// The root hubs: `type: entrypoint` becomes `type: hub` in them.
const HUBS: [&str; 4] = ["product.md", "tech.md", "design.md", "plan.md"];

/// One line edit; see the module docs for the exact meaning of each field.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rewrite {
    /// The file's path after the moves.
    pub file: String,
    /// 1-based first line.
    pub line: usize,
    /// The original text of the replaced line(s).
    pub old: String,
    /// The replacement text.
    pub new: String,
}

/// What a migration would do.
#[derive(Debug)]
pub struct Plan {
    /// `(from, to)` moves, in order.
    pub moves: Vec<(String, String)>,
    /// Line edits, sorted by `(file, line)`.
    pub rewrites: Vec<Rewrite>,
    /// New files (path after the moves, bytes), sorted by path.
    pub created: Vec<(String, Vec<u8>)>,
    /// Files scanned.
    pub scanned: usize,
    /// Literals found.
    pub before: usize,
}

/// Maps a project-relative path to where it lives after the moves.
struct Mover {
    from: String,
    to: String,
    /// Root-relative layered renames, `(old, new)`.
    renames: Vec<(String, String)>,
}

impl Mover {
    fn map(&self, rel: &str) -> String {
        let rest = if rel == self.from {
            Some("")
        } else {
            rel.strip_prefix(&format!("{}/", self.from))
        };
        match rest {
            None => rel.to_string(),
            Some("") => self.to.clone(),
            Some(rest) => {
                let rest = self
                    .renames
                    .iter()
                    .find(|(old, _)| old == rest)
                    .map_or(rest, |(_, new)| new.as_str());
                format!("{}/{rest}", self.to)
            }
        }
    }

    /// The root-relative part of `rel`, when it lies inside the root.
    fn inside<'a>(&self, rel: &'a str) -> Option<&'a str> {
        rel.strip_prefix(&format!("{}/", self.from))
    }
}

/// The spec root, project-relative and `/`-joined.
pub fn root_rel(project: &Path, root: &Root) -> Result<String, String> {
    let dir = normalise(&root.dir);
    match dir.strip_prefix(normalise(project)) {
        Ok(rel) if !rel.as_os_str().is_empty() => Ok(slash(rel)),
        Ok(_) => Err("the spec root is the project directory itself".to_string()),
        Err(_) => Err(format!(
            "the spec root {} is outside the project; not migrating it",
            dir.display()
        )),
    }
}

/// `to` as a clean project-relative path.
fn check_to(to: &str) -> Result<String, String> {
    let mut parts: Vec<&str> = Vec::new();
    for seg in to.split('/') {
        match seg {
            "" | "." => {}
            ".." => return Err(format!("target {to} must not contain '..'")),
            s => parts.push(s),
        }
    }
    if parts.is_empty() || to.starts_with('/') {
        return Err(format!("target {to} must be a project-relative directory"));
    }
    let clean = parts.join("/");
    if !literal::find(clean.as_bytes()).is_empty() {
        return Err(format!(
            "target {clean} is itself a legacy literal; pick another directory"
        ));
    }
    Ok(clean)
}

/// The metadata of `project/rel` without following a link; `None` when absent.
fn lstat(project: &Path, rel: &str) -> Result<Option<fs::Metadata>, String> {
    match fs::symlink_metadata(project.join(rel)) {
        Ok(m) => Ok(Some(m)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(format!("{rel}: {e}")),
    }
}

/// No component of `rel` (from the project down) may be a symlink.
fn no_links_on(project: &Path, rel: &str) -> Result<(), String> {
    let mut cur = String::new();
    for part in rel.split('/') {
        if !cur.is_empty() {
            cur.push('/');
        }
        cur.push_str(part);
        match lstat(project, &cur)? {
            Some(m) if m.file_type().is_symlink() => {
                return Err(format!("{cur} is a symlink; not moving through it"));
            }
            Some(_) => {}
            None => break,
        }
    }
    Ok(())
}

/// The root move is safe: a real directory, a free target, no nesting.
fn guard_move(project: &Path, from: &str, to: &str) -> Result<(), String> {
    no_links_on(project, from)?;
    no_links_on(project, to)?;
    if lstat(project, from)?.is_none_or(|m| !m.is_dir()) {
        return Err(format!("{from} is not a directory"));
    }
    if lstat(project, to)?.is_some() {
        return Err(format!("{to} already exists"));
    }
    if to.starts_with(&format!("{from}/")) || from.starts_with(&format!("{to}/")) {
        return Err(format!(
            "{from} and {to} are nested; pick a separate target"
        ));
    }
    Ok(())
}

/// Layered renames for the top-level `tech-<t>.md` notes of the root at
/// `from`; refuses `product-*.md` and `plan-*.md`.
fn layered_renames(project: &Path, from: &str) -> Result<Vec<(String, String)>, String> {
    let dir = project.join(from);
    let mut names: Vec<String> = Vec::new();
    for e in fs::read_dir(&dir).map_err(|e| format!("{from}: {e}"))? {
        let e = e.map_err(|e| format!("{from}: {e}"))?;
        let name = e
            .file_name()
            .into_string()
            .map_err(|_| format!("{from}: a file name is not UTF-8"))?;
        let is_file = e.file_type().is_ok_and(|t| t.is_file());
        if is_file && name.ends_with(".md") {
            names.push(name);
        }
    }
    names.sort();
    let mut out = Vec::new();
    for name in &names {
        if name.starts_with("product-") || name.starts_with("plan-") {
            return Err(format!("{from}/{name}: place product/plan topics by hand"));
        }
        let topic = name
            .strip_prefix("tech-")
            .and_then(|t| t.strip_suffix(".md"))
            .filter(|t| !t.is_empty());
        if let Some(topic) = topic {
            let new = format!("tech/{topic}.md");
            if lstat(project, &format!("{from}/{new}"))?.is_some() {
                return Err(format!(
                    "{from}/{new} already exists; cannot move {from}/{name} onto it"
                ));
            }
            out.push((name.clone(), new));
        }
    }
    Ok(out)
}

/// A glob that matches `text` literally.
fn escape(text: &str) -> String {
    let mut out = String::new();
    for c in text.chars() {
        if matches!(c, '*' | '?' | '[' | ']' | '\\') {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

/// Directories no run descends into.
const SKIP_DIRS: [&str; 3] = [".git", "target", "node_modules"];

/// Whether an include glob can match something below (or at) directory `dir`.
/// A segment holding `**` may match any depth; every other segment matches
/// exactly one level.
fn viable(include: &[String], dir: &str) -> bool {
    include.iter().any(|g| {
        let pat: Vec<&str> = g.split('/').collect();
        for (i, seg) in dir.split('/').enumerate() {
            match pat.get(i) {
                None => return false,
                Some(p) if p.contains("**") => return true,
                Some(p) if !glob_match(p, seg) => return false,
                Some(_) => {}
            }
        }
        true
    })
}

/// Every regular file in the project (links not followed) that an include
/// glob selects and no exclude glob removes, sorted. Directories no include
/// glob reaches, `.git`, `target` and `node_modules` are never opened. A
/// name that is not UTF-8 aborts the plan only where it would be scanned.
fn collect(project: &Path, include: &[String], exclude: &[String]) -> Result<Vec<String>, String> {
    // A directory is pruned when an exclude glob takes everything below it.
    let pruned = |dir: &str| {
        exclude.iter().any(|g| {
            glob_match(g, &format!("{dir}/\u{1}")) && glob_match(g, &format!("{dir}/\u{1}/\u{1}"))
        })
    };
    let selected = |rel: &str| {
        include.iter().any(|g| glob_match(g, rel)) && !exclude.iter().any(|g| glob_match(g, rel))
    };
    let mut out = Vec::new();
    let mut stack = vec![String::new()];
    while let Some(dir) = stack.pop() {
        let shown = if dir.is_empty() { "." } else { dir.as_str() };
        let entries = fs::read_dir(project.join(&dir)).map_err(|e| format!("{shown}: {e}"))?;
        for e in entries {
            let e = e.map_err(|e| format!("{shown}: {e}"))?;
            let os_name = e.file_name();
            let utf8 = os_name.to_str().is_some();
            let name = os_name.to_string_lossy().into_owned();
            let rel = if dir.is_empty() {
                name.clone()
            } else {
                format!("{dir}/{name}")
            };
            let kind = e.file_type().map_err(|e| format!("{rel}: {e}"))?;
            if kind.is_symlink() || name == ".git" {
                continue;
            }
            if kind.is_dir() {
                if SKIP_DIRS.contains(&name.as_str()) || !viable(include, &rel) || pruned(&rel) {
                    continue;
                }
                if !utf8 {
                    return Err(format!("{rel}: a directory name is not UTF-8"));
                }
                stack.push(rel);
            } else if kind.is_file() && selected(&rel) {
                if !utf8 {
                    return Err(format!("{rel}: a file name is not UTF-8"));
                }
                out.push(rel);
            }
        }
    }
    out.sort();
    Ok(out)
}

/// `dir` of a project-relative path, as parts.
fn parent_parts(rel: &str) -> Vec<String> {
    let mut parts: Vec<String> = rel.split('/').map(str::to_string).collect();
    parts.pop();
    parts
}

/// `path` resolved from `dir`; `None` when it climbs out of the project.
fn resolve(dir: &[String], path: &str) -> Option<Vec<String>> {
    let mut out = dir.to_vec();
    for seg in path.split('/') {
        match seg {
            "" | "." => {}
            ".." => {
                out.pop()?;
            }
            s => out.push(s.to_string()),
        }
    }
    Some(out)
}

/// The relative path from directory `dir` to `target`.
fn relative(dir: &[String], target: &[String]) -> String {
    let common = dir.iter().zip(target).take_while(|(a, b)| a == b).count();
    let mut parts: Vec<&str> = vec![".."; dir.len() - common];
    parts.extend(target.iter().skip(common).map(String::as_str));
    if parts.is_empty() {
        ".".to_string()
    } else {
        parts.join("/")
    }
}

/// The new text of one markdown link target, `None` when it stays valid.
fn fix_target(target: &str, old_file: &str, new_file: &str, mover: &Mover) -> Option<String> {
    if target.is_empty() || target.starts_with('#') || target.starts_with('/') {
        return None;
    }
    if target.find(':').is_some_and(|c| !target[..c].contains('/')) {
        return None;
    }
    let cut = target.find(['#', '?']).unwrap_or(target.len());
    let (path, suffix) = target.split_at(cut);
    if path.is_empty() {
        return None;
    }
    let old_abs = resolve(&parent_parts(old_file), path)?;
    let new_target: Vec<String> = mover
        .map(&old_abs.join("/"))
        .split('/')
        .map(str::to_string)
        .collect();
    let new_dir = parent_parts(new_file);
    if resolve(&new_dir, path).as_ref() == Some(&new_target) {
        return None;
    }
    let mut rel = relative(&new_dir, &new_target);
    if path.ends_with('/') && !rel.ends_with('/') {
        rel.push('/');
    }
    Some(format!("{rel}{suffix}"))
}

/// `target` (plain or in angle brackets) re-resolved; `None` when unchanged.
fn fix_wrapped(target: &str, old_file: &str, new_file: &str, mover: &Mover) -> Option<String> {
    match target.strip_prefix('<').and_then(|t| t.strip_suffix('>')) {
        Some(inner) => fix_target(inner, old_file, new_file, mover).map(|n| format!("<{n}>")),
        None => fix_target(target, old_file, new_file, mover),
    }
}

/// A reference definition `[label]: target "title"` (up to three spaces of
/// indent) with its target re-resolved; `None` when the line is not one or
/// the target stays valid.
fn fix_definition(text: &str, old_file: &str, new_file: &str, mover: &Mover) -> Option<String> {
    let indent = text.chars().take_while(|c| *c == ' ').count();
    if indent > 3 {
        return None;
    }
    let body = text.get(indent..)?.strip_prefix('[')?;
    let close = body.find("]:")?;
    if close == 0 || body.get(..close)?.contains(['[', ']']) {
        return None;
    }
    let after = body.get(close + 2..)?;
    let lead = after.len() - after.trim_start_matches(' ').len();
    let rest = after.get(lead..)?;
    let end = if rest.starts_with('<') {
        rest.find('>').map(|i| i + 1)?
    } else {
        rest.find(char::is_whitespace).unwrap_or(rest.len())
    };
    let (target, tail) = rest.split_at(end);
    let new = fix_wrapped(target, old_file, new_file, mover)?;
    let head = text.get(..text.len() - tail.len() - target.len())?;
    Some(format!("{head}{new}{tail}"))
}

/// Splits `text` into `(is_code, piece)` runs; a code span is a backtick run
/// up to the next run of the same length (an unclosed run is plain text).
fn code_spans(text: &str) -> Vec<(bool, &str)> {
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let (mut plain, mut i) = (0, 0);
    while i < bytes.len() {
        if bytes.get(i) != Some(&b'`') {
            i += 1;
            continue;
        }
        let run = bytes.iter().skip(i).take_while(|b| **b == b'`').count();
        let mut j = i + run;
        let mut close = None;
        while j < bytes.len() {
            if bytes.get(j) == Some(&b'`') {
                let n = bytes.iter().skip(j).take_while(|b| **b == b'`').count();
                if n == run {
                    close = Some(j + n);
                    break;
                }
                j += n;
            } else {
                j += 1;
            }
        }
        match close {
            Some(end) => {
                out.push((false, text.get(plain..i).unwrap_or_default()));
                out.push((true, text.get(i..end).unwrap_or_default()));
                plain = end;
                i = end;
            }
            None => i += run,
        }
    }
    out.push((false, text.get(plain..).unwrap_or_default()));
    out
}

/// Inline links `](target)` of a text piece, re-resolved.
fn fix_inline(text: &str, old_file: &str, new_file: &str, mover: &Mover) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some((head, tail)) = rest.split_once("](") {
        out.push_str(head);
        out.push_str("](");
        let end = if tail.starts_with('<') {
            tail.find('>').map_or(tail.len(), |i| i + 1)
        } else {
            tail.find(|c: char| c == ')' || c.is_whitespace())
                .unwrap_or(tail.len())
        };
        let (target, after) = tail.split_at(end);
        match fix_wrapped(target, old_file, new_file, mover) {
            Some(new) => out.push_str(&new),
            None => out.push_str(target),
        }
        rest = after;
    }
    out.push_str(rest);
    out
}

/// `line` with every relative markdown link (inline or reference
/// definition, outside code spans) re-resolved for the file's move.
fn fix_links(line: &[u8], old_file: &str, new_file: &str, mover: &Mover) -> Vec<u8> {
    let Ok(text) = std::str::from_utf8(line) else {
        return line.to_vec();
    };
    if let Some(new) = fix_definition(text, old_file, new_file, mover) {
        return new.into_bytes();
    }
    code_spans(text)
        .into_iter()
        .map(|(code, piece)| {
            if code {
                piece.to_string()
            } else {
                fix_inline(piece, old_file, new_file, mover)
            }
        })
        .collect::<String>()
        .into_bytes()
}

/// A multi-line edit: lines `start..end` (0-based) become `new`.
struct Edit {
    start: usize,
    end: usize,
    new: String,
}

/// The line index of the closing `---` of a leading frontmatter block.
fn fm_close(lines: &[String]) -> Option<usize> {
    if lines.first()?.trim_end() != "---" {
        return None;
    }
    lines
        .iter()
        .enumerate()
        .skip(1)
        .find(|(_, l)| l.trim_end() == "---")
        .map(|(i, _)| i)
}

/// Replaces `old` by `new` as a whole token (not inside a longer path or word).
fn replace_token(line: &str, old: &str, new: &str) -> String {
    let edge = |c: char| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '/' | '.');
    let mut out = String::new();
    let mut rest = line;
    while let Some(at) = rest.find(old) {
        let (head, tail) = rest.split_at(at);
        out.push_str(head);
        let after = tail.get(old.len()..).unwrap_or_default();
        let before_ok = out.chars().last().is_none_or(|c| !edge(c));
        let after_ok = after.chars().next().is_none_or(|c| !edge(c));
        out.push_str(if before_ok && after_ok { new } else { old });
        rest = after;
    }
    out.push_str(rest);
    out
}

/// Root hubs: `type: entrypoint` becomes `type: hub`; names in `children:`
/// follow the renames. Edits `lines` in place.
fn hub_edits(lines: &mut [String], renames: &[(String, String)]) {
    let Some(close) = fm_close(lines) else {
        return;
    };
    let mut in_children = false;
    for line in lines.iter_mut().take(close).skip(1) {
        if line.trim_end() == "type: entrypoint" {
            let cr = if line.ends_with('\r') { "\r" } else { "" };
            *line = format!("type: hub{cr}");
            continue;
        }
        if line.starts_with("children:") {
            in_children = true;
        } else if !line.starts_with([' ', '\t', '-']) {
            in_children = false;
        }
        if in_children {
            for (old, new) in renames {
                *line = replace_token(line, old, new);
            }
        }
    }
}

/// `lines` joined by `\n`, each ending in `\r` when `crlf`.
fn join_lines(lines: &[String], crlf: bool) -> String {
    let cr = if crlf { "\r" } else { "" };
    lines
        .iter()
        .map(|l| format!("{}{cr}", l.trim_end_matches('\r')))
        .collect::<Vec<_>>()
        .join("\n")
}

/// A `tech-<t>.md` note: `type: tech`, `applies: "**"`, no `parent:`,
/// `covers:` or `scope:`.
fn tech_edit(lines: &[String], today: &str) -> Option<Edit> {
    const APPLIES: &str = "applies: \"**\"";
    let crlf = lines.first().is_some_and(|l| l.ends_with('\r'));
    let Some(close) = fm_close(lines) else {
        let first = lines.first().map_or("", String::as_str);
        let new = [
            "---",
            "type: tech",
            APPLIES,
            &format!("updated: {today}"),
            "---",
            "",
            first,
        ]
        .map(str::to_string);
        return Some(Edit {
            start: 0,
            end: 1,
            new: join_lines(&new, crlf),
        });
    };
    let mut groups: Vec<(String, Vec<String>)> = Vec::new();
    for line in lines.iter().take(close).skip(1) {
        let key = (!line.starts_with([' ', '\t', '-', '#']))
            .then(|| line.split_once(':').map(|(k, _)| k.trim().to_string()))
            .flatten();
        match (key, groups.last_mut()) {
            (Some(key), _) => groups.push((key, vec![line.clone()])),
            (None, Some(last)) => last.1.push(line.clone()),
            (None, None) => groups.push((String::new(), vec![line.clone()])),
        }
    }
    let has = |k: &str| groups.iter().any(|(key, _)| key == k);
    let mut out = vec!["---".to_string()];
    if !has("type") {
        out.push("type: tech".to_string());
        if !has("applies") {
            out.push(APPLIES.to_string());
        }
    }
    for (key, body) in &groups {
        match key.as_str() {
            "parent" | "covers" | "scope" => {}
            "type" => {
                out.push("type: tech".to_string());
                if !has("applies") {
                    out.push(APPLIES.to_string());
                }
            }
            _ => out.extend(body.iter().cloned()),
        }
    }
    out.push("---".to_string());
    let new = join_lines(&out, crlf);
    (new != lines.get(..=close)?.join("\n")).then_some(Edit {
        start: 0,
        end: close + 1,
        new,
    })
}

/// `- **D<n> — <title>.**`: the number and the title.
fn parse_decision(line: &str) -> Option<(String, String)> {
    let rest = line.strip_prefix("- **D")?;
    let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
    if digits.is_empty() {
        return None;
    }
    let rest = rest.get(digits.len()..)?.strip_prefix(" \u{2014} ")?;
    let (title, _) = rest.split_once(".**")?;
    (!title.trim().is_empty()).then(|| (digits, title.trim().to_string()))
}

/// Kebab-case ASCII title (`[a-z0-9-]`), at most six words; apostrophes and
/// other non-ASCII characters are dropped. `None` when nothing is left.
fn slug(title: &str) -> Option<String> {
    let dashed: String = title
        .chars()
        .filter(|c| !matches!(c, '\'' | '\u{2019}') && c.is_ascii())
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect();
    let words: Vec<&str> = dashed
        .split('-')
        .filter(|w| !w.is_empty())
        .take(6)
        .collect();
    (!words.is_empty()).then(|| words.join("-"))
}

/// A line that ends a D-bullet when it follows the bullet.
fn ends_bullet(line: &str) -> bool {
    line.trim().is_empty()
        || line.starts_with('#')
        || line.starts_with("- ")
        || line.starts_with("* ")
        || line.starts_with("+ ")
}

/// One D-bullet found in `plan.md`.
struct Decision {
    start: usize,
    end: usize,
    num: String,
    title: String,
    name: String,
}

/// The D-bullets of `lines` (fenced code skipped), in order. A repeated
/// number or a title with no ASCII file name is an error naming `rel`.
fn decisions(rel: &str, lines: &[String]) -> Result<Vec<Decision>, String> {
    let mut seen: BTreeSet<String> = BTreeSet::new();
    let start = fm_close(lines).map_or(0, |c| c + 1);
    let mut fences = Fences::new();
    let mut out = Vec::new();
    let mut i = 0;
    while let Some(line) = lines.get(i) {
        let fenced = fences.skip(line.as_bytes());
        if let Some((num, title)) = (i >= start && !fenced)
            .then(|| parse_decision(line))
            .flatten()
        {
            let mut end = i + 1;
            while lines.get(end).is_some_and(|l| !ends_bullet(l)) {
                end += 1;
            }
            if !seen.insert(num.clone()) {
                return Err(format!("{rel}: duplicate decision D{num}"));
            }
            let Some(slug) = slug(&title) else {
                return Err(format!(
                    "{rel}: D{num}: the title gives no ASCII file name; place it by hand"
                ));
            };
            let name = format!("d{num}-{slug}.md");
            out.push(Decision {
                start: i,
                end,
                num,
                title,
                name,
            });
            i = end;
        } else {
            i += 1;
        }
    }
    Ok(out)
}

/// Accumulates the plan across files.
struct State {
    rewrites: Vec<Rewrite>,
    created: Vec<(String, Vec<u8>)>,
    before: usize,
}

/// One line after the link pass and the literal rewrite.
struct Ctx<'a> {
    mover: &'a Mover,
    today: &'a str,
}

fn transform(
    ctx: &Ctx,
    line: &[u8],
    links: bool,
    has_literal: bool,
    (old_file, new_file): (&str, &str),
    at: usize,
) -> Result<Vec<u8>, String> {
    let mut cur = if links {
        fix_links(line, old_file, new_file, ctx.mover)
    } else {
        line.to_vec()
    };
    if has_literal {
        if std::str::from_utf8(line).is_err() {
            return Err(format!(
                "{old_file}:{at}: a literal on a line that is not UTF-8; fix it by hand"
            ));
        }
        cur = literal::rewrite(&cur, literal::LEGACY, &ctx.mover.to, &ctx.mover.renames);
    }
    Ok(cur)
}

fn text_of(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

/// Scans one file: literal and link rewrites, then the layered edits.
fn process(
    ctx: &Ctx,
    project: &Path,
    rel: &str,
    text: &[u8],
    layered: bool,
    state: &mut State,
) -> Result<(), String> {
    let mover = ctx.mover;
    let new_path = mover.map(rel);
    let inside = mover.inside(rel);
    let links = inside.is_some() && rel.ends_with(".md");
    let exempt = literal::file_exempt(text);
    let orig: Vec<&[u8]> = text.split(|b| *b == b'\n').collect();
    let mut fences = Fences::new();
    let mut fixed: Vec<Vec<u8>> = Vec::with_capacity(orig.len());
    for (i, line) in orig.iter().enumerate() {
        let fenced = fences.skip(line);
        let hits = if exempt { 0 } else { literal::find(line).len() };
        state.before += hits;
        fixed.push(transform(
            ctx,
            line,
            links && !fenced,
            hits > 0,
            (rel, &new_path),
            i + 1,
        )?);
    }

    let mut edits: Vec<Edit> = Vec::new();
    let top = inside.filter(|r| !r.contains('/'));
    if layered && let Some(name) = top {
        let mut lines = fixed
            .iter()
            .map(|l| String::from_utf8(l.clone()))
            .collect::<Result<Vec<String>, _>>()
            .map_err(|_| format!("{rel}: not UTF-8; place it by hand"))?;
        if HUBS.contains(&name) {
            hub_edits(&mut lines, &mover.renames);
        }
        if name.starts_with("tech-") && name.ends_with(".md") {
            edits.extend(tech_edit(&lines, ctx.today));
        }
        if name == "plan.md" {
            for d in decisions(rel, &lines)? {
                let path = format!("{}/decisions/{}", mover.to, d.name);
                let current = format!("{}/decisions/{}", mover.from, d.name);
                if lstat(project, &current)?.is_some() {
                    return Err(format!("{path} already exists"));
                }
                let mut body: Vec<String> = Vec::new();
                for (k, line) in orig.iter().enumerate().take(d.end).skip(d.start) {
                    let hits = if exempt { 0 } else { literal::find(line).len() };
                    let out = transform(ctx, line, true, hits > 0, (rel, &path), k + 1)?;
                    body.push(text_of(&out));
                }
                let cr = orig.get(d.end - 1).is_some_and(|l| l.ends_with(b"\r"));
                let eol = if cr { "\r\n" } else { "\n" };
                let bytes = format!(
                    "---{eol}type: decision{eol}updated: {}{eol}---{eol}{eol}{}\n",
                    ctx.today,
                    body.join("\n")
                );
                state.created.push((path, bytes.into_bytes()));
                edits.push(Edit {
                    start: d.start,
                    end: d.end,
                    new: format!(
                        "- [D{} \u{2014} {}](decisions/{}){}",
                        d.num,
                        d.title,
                        d.name,
                        if cr { "\r" } else { "" }
                    ),
                });
            }
        }
        fixed = lines.into_iter().map(String::into_bytes).collect();
    }

    let mut covered = vec![false; orig.len()];
    for e in &edits {
        for flag in covered.iter_mut().take(e.end).skip(e.start) {
            *flag = true;
        }
    }
    for (i, (old, new)) in orig.iter().zip(&fixed).enumerate() {
        if !covered.get(i).copied().unwrap_or(false) && old != &new.as_slice() {
            state.rewrites.push(Rewrite {
                file: new_path.clone(),
                line: i + 1,
                old: text_of(old),
                new: text_of(new),
            });
        }
    }
    for e in edits {
        let old: Vec<String> = orig
            .iter()
            .take(e.end)
            .skip(e.start)
            .map(|l| text_of(l))
            .collect();
        state.rewrites.push(Rewrite {
            file: new_path.clone(),
            line: e.start + 1,
            old: old.join("\n"),
            new: e.new,
        });
    }
    Ok(())
}

/// Scans the configured file set and plans the migration of `root` to the
/// project-relative directory `to`. Plans only; writes nothing. An `Err` is
/// the cause, without the `migrate: ERROR:` prefix.
pub fn plan(project: &Path, root: &Root, to: &str, layered: bool) -> Result<Plan, String> {
    let cfg = crate::root::migrate_config(project)?;
    let to = check_to(to)?;
    let from = root_rel(project, root)?;
    let moving = from != to;
    if moving {
        guard_move(project, &from, &to)?;
    }
    let renames = if layered {
        layered_renames(project, &from)?
    } else {
        Vec::new()
    };
    let mut moves: Vec<(String, String)> = Vec::new();
    if moving {
        moves.push((from.clone(), to.clone()));
    }
    for (old, new) in &renames {
        moves.push((format!("{to}/{old}"), format!("{to}/{new}")));
    }

    let mut include: Vec<String> = vec![format!("{}/**", escape(&to))];
    if moving {
        include.push(format!("{}/**", escape(&from)));
    }
    include.extend(
        [
            ".vibe/*.json",
            "content/policy.json",
            ".claude/**",
            ".agents/**",
            "tests/**",
            "*.md",
            "flow/reference/templates/**",
        ]
        .map(str::to_string),
    );
    include.extend(cfg.include.iter().cloned());
    let mut files: BTreeSet<String> = collect(project, &include, &cfg.exclude)?
        .into_iter()
        .collect();
    if layered {
        // The layered mechanics read the root's top-level notes whatever the globs say.
        for e in fs::read_dir(project.join(&from)).map_err(|e| format!("{from}: {e}"))? {
            let e = e.map_err(|e| format!("{from}: {e}"))?;
            if let Ok(name) = e.file_name().into_string()
                && name.ends_with(".md")
                && e.file_type().is_ok_and(|t| t.is_file())
            {
                files.insert(format!("{from}/{name}"));
            }
        }
    }

    let mover = Mover { from, to, renames };
    let today = today_utc();
    let ctx = Ctx {
        mover: &mover,
        today: &today,
    };
    let mut state = State {
        rewrites: Vec::new(),
        created: Vec::new(),
        before: 0,
    };
    let mut scanned = 0;
    for rel in &files {
        let bytes = fs::read(project.join(rel)).map_err(|e| format!("{rel}: {e}"))?;
        if bytes.contains(&0) {
            continue;
        }
        scanned += 1;
        process(&ctx, project, rel, &bytes, layered, &mut state)?;
    }
    if scanned < cfg.floor {
        return Err(format!("scanned {scanned} file(s), floor {}", cfg.floor));
    }

    state
        .rewrites
        .sort_by(|a, b| (&a.file, a.line).cmp(&(&b.file, b.line)));
    state.created.sort_by(|a, b| a.0.cmp(&b.0));
    for r in &state.rewrites {
        let had = r
            .old
            .split('\n')
            .any(|l| !literal::find(l.as_bytes()).is_empty());
        let left = r
            .new
            .split('\n')
            .any(|l| !literal::find(l.as_bytes()).is_empty());
        if had && left {
            return Err(format!(
                "{}:{}: the rewrite leaves a literal",
                r.file, r.line
            ));
        }
    }
    Ok(Plan {
        moves,
        rewrites: state.rewrites,
        created: state.created,
        scanned,
        before: state.before,
    })
}

/// The population check for a run that moves the root: at least one literal.
pub fn check_population(plan: &Plan, moving: bool) -> Result<(), String> {
    if moving && plan.before == 0 {
        return Err(format!("no {} literal found", literal::LEGACY));
    }
    Ok(())
}

/// Parsed `migrate` flags.
pub struct Options {
    /// `--to`, relative to the current directory.
    pub to: PathBuf,
    /// `--layout layered`.
    pub layered: bool,
    /// `--dry-run`.
    pub dry_run: bool,
    /// `--check`.
    pub check: bool,
    /// `--json`.
    pub json: bool,
}

fn escaped(text: &str) -> String {
    text.replace('\r', "").replace('\n', "\u{23ce}")
}

/// Files with a literal among the rewrites.
fn literal_files(plan: &Plan) -> usize {
    plan.rewrites
        .iter()
        .filter(|r| {
            r.old
                .split('\n')
                .any(|l| !literal::find(l.as_bytes()).is_empty())
        })
        .map(|r| r.file.as_str())
        .collect::<BTreeSet<_>>()
        .len()
}

fn table(plan: &Plan) -> String {
    plan.rewrites
        .iter()
        .map(|r| {
            format!(
                "{}:{}  {} \u{2192} {}\n",
                r.file,
                r.line,
                escaped(&r.old),
                escaped(&r.new)
            )
        })
        .collect()
}

fn render_json(
    from: &str,
    to: &str,
    plan: &Plan,
    mode: (bool, bool),
    markers: &[String],
) -> Vec<u8> {
    let v = serde_json::json!({
        "v": 1,
        "from": from,
        "to": to,
        "moves": plan.moves.iter().map(|(a, b)| serde_json::json!({"from": a, "to": b})).collect::<Vec<_>>(),
        "rewrites": plan.rewrites.iter().map(|r| serde_json::json!({
            "file": r.file, "line": r.line, "old": r.old, "new": r.new,
        })).collect::<Vec<_>>(),
        "created": plan.created.iter().map(|(p, _)| p.as_str()).collect::<Vec<_>>(),
        "markers": markers,
        "dry_run": mode.0,
        "check": mode.1,
    });
    format!("{v}\n").into_bytes()
}

fn render_text(label: &str, plan: &Plan, markers: &[String]) -> Vec<u8> {
    let mut out = format!(
        "migrate: {label} \u{2014} {} literal(s) in {} file(s)\n",
        plan.before,
        literal_files(plan)
    );
    out.push_str(&table(plan));
    for (a, b) in &plan.moves {
        out.push_str(&format!("move: {a} \u{2192} {b}\n"));
    }
    for (path, bytes) in &plan.created {
        out.push_str(&format!("create: {path} ({} bytes)\n", bytes.len()));
    }
    for m in markers {
        out.push_str(&format!("marker: {m}\n"));
    }
    out.push_str(&format!("scanned: {} file(s)\n", plan.scanned));
    out.into_bytes()
}

/// Runs the command.
pub fn run(found: &Result<Root, Miss>, cwd: &Path, opts: &Options) -> (u8, Vec<u8>) {
    let root = match found {
        Ok(root) => root,
        Err(miss) => {
            eprintln!("{}", miss.message());
            return (miss.code(), Vec::new());
        }
    };
    let project = normalise(&root.project);
    let target = normalise(&cwd.join(&opts.to));
    let to = match target.strip_prefix(&project) {
        Ok(rel) if !rel.as_os_str().is_empty() => slash(rel),
        _ => {
            eprintln!(
                "vibe-spec migrate: --to {} must be a directory inside the project {}",
                opts.to.display(),
                project.display()
            );
            return (exit::USAGE, Vec::new());
        }
    };
    if let Err(why) = crate::root::migrate_config(&root.project) {
        eprintln!("vibe-spec migrate: {why}");
        return (exit::USAGE, Vec::new());
    }
    let fail = |why: String| {
        eprintln!("migrate: ERROR: {why}");
        (exit::FINDINGS, Vec::new())
    };
    let from = match root_rel(&root.project, root) {
        Ok(from) => from,
        Err(why) => return fail(why),
    };
    let plan = match plan(&root.project, root, &to, opts.layered && !opts.check) {
        Ok(plan) => plan,
        Err(why) => return fail(why),
    };
    let show = |label: &str, markers: &[String]| {
        if opts.json {
            render_json(&from, &to, &plan, (opts.dry_run, opts.check), markers)
        } else {
            render_text(label, &plan, markers)
        }
    };
    if opts.check {
        let code = if plan.before == 0 {
            exit::OK
        } else {
            exit::FINDINGS
        };
        return (code, show("check", &[]));
    }
    let moving = from != to;
    if !moving && plan.moves.is_empty() && plan.rewrites.is_empty() && plan.created.is_empty() {
        let out = if opts.json {
            show("", &[])
        } else {
            b"migrate: already migrated\n".to_vec()
        };
        return (exit::OK, out);
    }
    if let Err(why) = check_population(&plan, moving) {
        return fail(why);
    }
    if opts.dry_run {
        return (exit::OK, show("dry run", &[]));
    }
    match execute(&root.project, &from, &to, &plan) {
        Ok(markers) => (exit::OK, show("done", &markers)),
        Err(why) => fail(why),
    }
}

/// One step to undo, in the order it ran.
enum Step {
    Mkdir(PathBuf),
    Move { from: String, to: String, git: bool },
}

/// What a run has done so far, so a failure can undo it.
struct Journal {
    project: PathBuf,
    steps: Vec<Step>,
    /// Writes that went through (never a failed one: `commit` undoes those).
    writes: Vec<Write>,
}

/// `git -C <project> --literal-pathspecs mv -- <from> <to>`, with git's own error text.
fn git_mv(project: &Path, from: &str, to: &str) -> Result<(), String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(project)
        .arg("--literal-pathspecs")
        .args(["mv", "--", from, to])
        .stdin(Stdio::null())
        .output()
        .map_err(|e| format!("git mv {from}: {e}"))?;
    if out.status.success() {
        return Ok(());
    }
    Err(format!(
        "git mv {from} {to}: {}",
        String::from_utf8_lossy(&out.stderr).trim()
    ))
}

/// True when git tracks something at `rel`.
fn tracked(project: &Path, rel: &str) -> bool {
    crate::git::git(project, &["ls-files", "-z", "--", rel]).is_some_and(|b| !b.is_empty())
}

impl Journal {
    /// Creates the missing directories down to `dir`, never through a link.
    fn mkdirs(&mut self, dir: &Path) -> Result<(), String> {
        let rel = dir
            .strip_prefix(&self.project)
            .map_err(|_| format!("{}: outside the project", dir.display()))?;
        let mut cur = self.project.clone();
        for c in rel.components() {
            cur.push(c);
            match fs::symlink_metadata(&cur) {
                Ok(m) if m.is_dir() => {}
                Ok(_) => {
                    return Err(format!(
                        "{}: not a directory (or a link); refusing to write through it",
                        slash(cur.strip_prefix(&self.project).unwrap_or(&cur))
                    ));
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                    fs::create_dir(&cur).map_err(|e| format!("{}: {e}", cur.display()))?;
                    self.steps.push(Step::Mkdir(cur.clone()));
                }
                Err(e) => return Err(format!("{}: {e}", cur.display())),
            }
        }
        Ok(())
    }

    /// `git mv` when git tracks `from`, otherwise a rename.
    fn mv(&mut self, from: &str, to: &str) -> Result<(), String> {
        let target = self.project.join(to);
        if let Some(parent) = target.parent() {
            self.mkdirs(parent)?;
        }
        let git = tracked(&self.project, from);
        if git {
            git_mv(&self.project, from, to)?;
        } else {
            fs::rename(self.project.join(from), self.project.join(to))
                .map_err(|e| format!("{from}: {e}"))?;
        }
        self.steps.push(Step::Move {
            from: from.to_string(),
            to: to.to_string(),
            git,
        });
        Ok(())
    }

    /// Puts the originals back, reverses the moves and removes the
    /// directories it made, newest first. Returns what could not be undone.
    fn rollback(&self) -> Vec<String> {
        let (_, mut failed) = restore(self.writes.iter().rev());
        for step in self.steps.iter().rev() {
            let done = match step {
                Step::Mkdir(dir) => {
                    fs::remove_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))
                }
                Step::Move {
                    from,
                    to,
                    git: true,
                } => git_mv(&self.project, to, from),
                Step::Move {
                    from,
                    to,
                    git: false,
                } => fs::rename(self.project.join(to), self.project.join(from))
                    .map_err(|e| format!("{to}: {e}")),
            };
            if let Err(why) = done {
                failed.push(why);
            }
        }
        failed
    }
}

/// Reads a regular file (never a link).
fn read_regular(project: &Path, rel: &str) -> Result<Vec<u8>, String> {
    let path = project.join(rel);
    let meta = fs::symlink_metadata(&path).map_err(|e| format!("{rel}: {e}"))?;
    if !meta.is_file() {
        return Err(format!("{rel}: not a regular file"));
    }
    fs::read(&path).map_err(|e| format!("{rel}: {e}"))
}

/// Applies the planned line edits to `bytes`, bottom-up. A line that no
/// longer matches the plan is an error.
fn apply(file: &str, bytes: &[u8], edits: &[&Rewrite]) -> Result<Vec<u8>, String> {
    let mut lines: Vec<Vec<u8>> = bytes.split(|b| *b == b'\n').map(<[u8]>::to_vec).collect();
    for e in edits.iter().rev() {
        let old: Vec<&str> = e.old.split('\n').collect();
        let at = e.line.saturating_sub(1);
        let matches = lines.get(at..at + old.len()).is_some_and(|cur| {
            cur.iter()
                .zip(&old)
                .all(|(l, o)| std::str::from_utf8(l).is_ok_and(|s| s == *o))
        });
        if !matches {
            return Err(format!(
                "{file}:{}: the line differs from the plan (or is not UTF-8)",
                e.line
            ));
        }
        lines.splice(
            at..at + old.len(),
            e.new.split('\n').map(|l| l.as_bytes().to_vec()),
        );
    }
    Ok(lines.join(&b'\n'))
}

/// Sets `updated:` in a leading frontmatter block, keeping line endings.
/// Text without frontmatter is returned as it is.
fn bump_updated(bytes: &[u8], today: &str) -> Vec<u8> {
    let mut lines: Vec<Vec<u8>> = bytes.split(|b| *b == b'\n').map(<[u8]>::to_vec).collect();
    let is = |l: &[u8], text: &str| l.strip_suffix(b"\r").unwrap_or(l) == text.as_bytes();
    let Some(first) = lines.first() else {
        return bytes.to_vec();
    };
    if !is(first, "---") {
        return bytes.to_vec();
    }
    let Some(close) = lines
        .iter()
        .skip(1)
        .position(|l| is(l, "---"))
        .map(|i| i + 1)
    else {
        return bytes.to_vec();
    };
    let cr: &[u8] = if first.ends_with(b"\r") { b"\r" } else { b"" };
    let mut line = format!("updated: {today}").into_bytes();
    line.extend_from_slice(cr);
    match lines
        .iter()
        .take(close)
        .skip(1)
        .position(|l| l.starts_with(b"updated:"))
    {
        Some(i) => {
            if let Some(slot) = lines.get_mut(i + 1) {
                *slot = line;
            }
        }
        None => lines.insert(close, line),
    }
    lines.join(&b'\n')
}

/// Every regular `.md` file under `dir` (links are never followed), as
/// project-relative `/`-joined paths.
fn md_under(project: &Path, rel: &str, out: &mut BTreeSet<String>) -> Result<(), String> {
    for e in fs::read_dir(project.join(rel)).map_err(|e| format!("{rel}: {e}"))? {
        let e = e.map_err(|e| format!("{rel}: {e}"))?;
        let Ok(name) = e.file_name().into_string() else {
            continue;
        };
        let child = format!("{rel}/{name}");
        let t = e.file_type().map_err(|e| format!("{child}: {e}"))?;
        if t.is_dir() {
            md_under(project, &child, out)?;
        } else if t.is_file() && name.ends_with(".md") {
            out.insert(child);
        }
    }
    Ok(())
}

/// The end of the JSON string that starts at `at`.
fn string_end(b: &[u8], at: usize) -> usize {
    let mut i = at + 1;
    while let Some(&c) = b.get(i) {
        match c {
            b'\\' => i += 2,
            b'"' => return i,
            _ => i += 1,
        }
    }
    b.len().saturating_sub(1)
}

/// Sets the top-level `root` key of the JSON object `text`, keeping every
/// other key, its order and its formatting. A new key goes first. Anything
/// that is not a JSON object with a string `root` (or none) is an error.
fn set_root(text: &str, to: &str) -> Result<String, String> {
    let want: serde_json::Value =
        serde_json::from_str(text).map_err(|e| format!(".vibe/spec.json: {e}"))?;
    let Some(obj) = want.as_object() else {
        return Err(".vibe/spec.json: not a JSON object".to_string());
    };
    if obj.get("root").is_some_and(|r| !r.is_string()) {
        return Err(".vibe/spec.json: root must be a string".to_string());
    }
    let value = serde_json::Value::String(to.to_string()).to_string();
    let b = text.as_bytes();
    let (mut i, mut depth, mut last) = (0, 0usize, 0u8);
    let mut open = None;
    let mut out = None;
    while i < b.len() && out.is_none() {
        match b[i] {
            b'"' => {
                let end = string_end(b, i);
                let is_key = depth == 1 && (last == b'{' || last == b',');
                if is_key
                    && serde_json::from_str::<String>(&text[i..=end]).is_ok_and(|k| k == "root")
                {
                    let mut j = end + 1;
                    while b
                        .get(j)
                        .is_some_and(|c| c.is_ascii_whitespace() || *c == b':')
                    {
                        j += 1;
                    }
                    let vend = string_end(b, j);
                    out = Some(format!("{}{value}{}", &text[..j], &text[vend + 1..]));
                }
                last = b'"';
                i = end + 1;
                continue;
            }
            b'{' | b'[' => {
                depth += 1;
                if b[i] == b'{' && open.is_none() {
                    open = Some(i);
                }
            }
            b'}' | b']' => depth = depth.saturating_sub(1),
            c if c.is_ascii_whitespace() => {
                i += 1;
                continue;
            }
            _ => {}
        }
        last = b[i];
        i += 1;
    }
    let merged = match (out, open) {
        (Some(done), _) => done,
        (None, Some(o)) => {
            let rest = &text[o + 1..];
            let eol = if text.contains("\r\n") { "\r\n" } else { "\n" };
            if obj.is_empty() {
                let close = rest.find('}').unwrap_or(0);
                format!(
                    "{}{{{eol}  \"root\": {value}{eol}{}",
                    &text[..o],
                    &rest[close..]
                )
            } else if let Some(nl) = rest.find('\n').filter(|nl| rest[..*nl].trim().is_empty()) {
                let indent: String = rest[nl + 1..]
                    .chars()
                    .take_while(|c| *c == ' ' || *c == '\t')
                    .collect();
                format!("{}{{{eol}{indent}\"root\": {value},{rest}", &text[..o])
            } else {
                let gap = if rest.starts_with(char::is_whitespace) {
                    ""
                } else {
                    " "
                };
                format!("{}{{\"root\": {value},{gap}{rest}", &text[..o])
            }
        }
        (None, None) => return Err(".vibe/spec.json: not a JSON object".to_string()),
    };
    // The result must be the same object with only `root` changed.
    let mut check: serde_json::Value =
        serde_json::from_str(&merged).map_err(|e| format!(".vibe/spec.json: {e}"))?;
    let got = check.get("root").cloned();
    if got.as_ref().and_then(|v| v.as_str()) != Some(to) {
        return Err(".vibe/spec.json: could not set root".to_string());
    }
    let mut expect = want;
    if let (Some(e), Some(c)) = (expect.as_object_mut(), check.as_object_mut()) {
        e.insert(
            "root".to_string(),
            serde_json::Value::String(to.to_string()),
        );
        if e != c {
            return Err(
                ".vibe/spec.json: could not set root without changing other keys".to_string(),
            );
        }
    }
    Ok(merged)
}

/// The `root` value of `.vibe/spec.json` text, when it is a string.
fn root_of(text: &str) -> Option<String> {
    let v: serde_json::Value = serde_json::from_str(text).ok()?;
    v.get("root")?.as_str().map(str::to_string)
}

/// Inserts one allow-marker per check id after the frontmatter (at the top
/// when there is none), with the file's own line ending.
fn insert_markers(bytes: &[u8], ids: &BTreeSet<&str>) -> Vec<u8> {
    let mut lines: Vec<Vec<u8>> = bytes.split(|b| *b == b'\n').map(<[u8]>::to_vec).collect();
    let is = |l: &[u8]| l.strip_suffix(b"\r").unwrap_or(l) == b"---";
    let crlf = lines.first().is_some_and(|l| l.ends_with(b"\r"));
    let at = match lines.first() {
        Some(f) if is(f) => lines
            .iter()
            .skip(1)
            .position(|l| is(l))
            .map_or(0, |i| i + 2),
        _ => 0,
    };
    let marks: Vec<Vec<u8>> = ids
        .iter()
        .map(|id| {
            let mut m = format!("<!-- vibe-spec: allow {id} -->").into_bytes();
            if crlf {
                m.push(b'\r');
            }
            m
        })
        .collect();
    lines.splice(at..at, marks);
    lines.join(&b'\n')
}

const MARKABLE: [&str; 5] = ["SF22", "SF30", "SF31", "SF33", "SF34"];

/// The warnings of the new root that a marker can carry: `(file, check)`.
fn markable(project: &Path, to: &str) -> Result<BTreeSet<(String, String)>, String> {
    let root = Root {
        dir: project.join(to),
        project: project.to_path_buf(),
        source: Source::Flag,
        depth: 0,
    };
    let (_, out) = validate::run_checked(
        &Ok(root),
        project,
        &validate::Options {
            strict: false,
            format: validate::Format::Json,
            changed: false,
            file: None,
            event: None,
        },
    )?;
    let v: serde_json::Value =
        serde_json::from_slice(&out).map_err(|e| format!("validating the new root: {e}"))?;
    let prefix = format!("{to}/");
    Ok(v["findings"]
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or_default()
        .iter()
        .filter(|f| f["severity"] == "warning")
        .filter_map(|f| {
            let check = f["check"].as_str().filter(|c| MARKABLE.contains(c))?;
            let file = f["file"]
                .as_str()
                .filter(|f| f.starts_with(&prefix) && f.ends_with(".md"))?;
            Some((file.to_string(), check.to_string()))
        })
        .collect())
}

/// Moves, rewrites, creates and verifies; undoes everything on failure.
/// Returns the markers inserted (`<file> <check>`). An `Err` is the cause
/// and says what happened to the tree (`reverted`).
fn execute(project: &Path, from: &str, to: &str, plan: &Plan) -> Result<Vec<String>, String> {
    let mut j = Journal {
        project: project.to_path_buf(),
        steps: Vec::new(),
        writes: Vec::new(),
    };
    match steps(&mut j, from, to, plan) {
        Ok(markers) => Ok(markers),
        Err(cause) => {
            let failed = j.rollback();
            if failed.is_empty() {
                Err(format!("{cause} \u{2014} reverted"))
            } else {
                Err(format!(
                    "{cause} \u{2014} revert incomplete: {}",
                    failed.join("; ")
                ))
            }
        }
    }
}

fn steps(j: &mut Journal, from: &str, to: &str, plan: &Plan) -> Result<Vec<String>, String> {
    const CONFIG: &str = ".vibe/spec.json";
    let project = j.project.clone();
    let moving = from != to;
    for (a, b) in &plan.moves {
        j.mv(a, b)?;
    }
    let today = today_utc();

    let mut edits: BTreeMap<&str, Vec<&Rewrite>> = BTreeMap::new();
    for r in &plan.rewrites {
        edits.entry(r.file.as_str()).or_default().push(r);
    }
    // Spec docs this run moved or rewrote get today's `updated:`.
    let mut docs: BTreeSet<String> = BTreeSet::new();
    let under = format!("{to}/");
    if moving {
        md_under(&project, to, &mut docs)?;
    } else {
        docs.extend(
            plan.moves
                .iter()
                .map(|(_, t)| t.clone())
                .filter(|t| t.ends_with(".md")),
        );
        docs.extend(
            edits
                .keys()
                .filter(|f| f.starts_with(&under) && f.ends_with(".md"))
                .map(|f| (*f).to_string()),
        );
    }
    let mut files: BTreeSet<String> = docs.clone();
    files.extend(edits.keys().map(|f| (*f).to_string()));

    // path -> (original, new bytes)
    let mut finals: BTreeMap<String, (Option<Vec<u8>>, Vec<u8>)> = BTreeMap::new();
    for rel in &files {
        let original = read_regular(&project, rel)?;
        let mut bytes = match edits.get(rel.as_str()) {
            Some(list) => apply(rel, &original, list)?,
            None => original.clone(),
        };
        if docs.contains(rel) {
            bytes = bump_updated(&bytes, &today);
        }
        finals.insert(rel.clone(), (Some(original), bytes));
    }
    for (rel, bytes) in &plan.created {
        if fs::symlink_metadata(project.join(rel)).is_ok() {
            return Err(format!("{rel} already exists"));
        }
        finals.insert(rel.clone(), (None, bytes.clone()));
    }
    let existing = match fs::symlink_metadata(project.join(CONFIG)) {
        Ok(_) => Some(finals.get(CONFIG).map_or_else(
            || read_regular(&project, CONFIG),
            |(_, now)| Ok(now.clone()),
        )?),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => return Err(format!("{CONFIG}: {e}")),
    };
    let config = match &existing {
        Some(bytes) => {
            let text = std::str::from_utf8(bytes)
                .map_err(|_| format!("{CONFIG}: not UTF-8; fix it by hand"))?;
            (moving || root_of(text).as_deref() != Some(to))
                .then(|| set_root(text, to))
                .transpose()?
        }
        None => moving.then(|| {
            format!(
                "{{\n  \"root\": {}\n}}\n",
                serde_json::Value::String(to.to_string())
            )
        }),
    };
    if let Some(text) = config {
        let original = match finals.remove(CONFIG) {
            Some((orig, _)) => orig,
            None => existing,
        };
        finals.insert(CONFIG.to_string(), (original, text.into_bytes()));
    }

    let mut writes: Vec<Write> = Vec::new();
    let config_entry = finals.remove(CONFIG);
    for (rel, (original, bytes)) in finals
        .into_iter()
        .chain(config_entry.map(|e| (CONFIG.to_string(), e)))
    {
        if original.as_deref() == Some(bytes.as_slice()) {
            continue;
        }
        let target = project.join(&rel);
        if let Some(parent) = target.parent() {
            j.mkdirs(parent)?;
        }
        writes.push(Write {
            path: project.join(&rel),
            show: rel,
            bytes,
            original,
        });
    }
    commit(&writes)?;
    j.writes.extend(writes);
    #[cfg(unix)]
    for (rel, _) in &plan.created {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(project.join(rel), fs::Permissions::from_mode(0o644))
            .map_err(|e| format!("{rel}: {e}"))?;
    }

    // After the write the legacy literal is gone everywhere in the set.
    let new_root = Root {
        dir: project.join(to),
        project: project.clone(),
        source: Source::Flag,
        depth: 0,
    };
    let after = self::plan(&project, &new_root, to, false)?;
    if after.before != 0 {
        return Err(format!(
            "{} {} literal(s) remain after the rewrite",
            after.before,
            literal::LEGACY
        ));
    }

    // Allow-markers for the warnings the move leaves behind (P4-R14). A
    // marker adds lines, so a budget warning can follow: look again, a few times.
    let mut marked: BTreeSet<(String, String)> = BTreeSet::new();
    for _ in 0..3 {
        let fresh: Vec<(String, String)> = markable(&project, to)?
            .into_iter()
            .filter(|m| !marked.contains(m))
            .collect();
        if fresh.is_empty() {
            break;
        }
        let mut by_file: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
        for (file, check) in &fresh {
            by_file.entry(file).or_default().insert(check);
        }
        let mut batch: Vec<Write> = Vec::new();
        for (file, ids) in by_file {
            let original = read_regular(&project, file)?;
            let bytes = insert_markers(&original, &ids);
            batch.push(Write {
                path: project.join(file),
                show: file.to_string(),
                bytes,
                original: Some(original),
            });
        }
        commit(&batch)?;
        j.writes.extend(batch);
        marked.extend(fresh);
    }
    Ok(marked
        .into_iter()
        .map(|(file, check)| format!("{file} {check}"))
        .collect())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    #[test]
    fn set_root_keeps_other_keys_and_their_order() {
        let cases = [
            ("{}", "{\n  \"root\": \"d/s\"\n}"),
            ("{}\n", "{\n  \"root\": \"d/s\"\n}\n"),
            (
                "{\n  \"tooth\": \"warn\",\n  \"x\": [1, 2]\n}\n",
                "{\n  \"root\": \"d/s\",\n  \"tooth\": \"warn\",\n  \"x\": [1, 2]\n}\n",
            ),
            (
                "{\r\n\t\"a\": 1\r\n}",
                "{\r\n\t\"root\": \"d/s\",\r\n\t\"a\": 1\r\n}",
            ),
            ("{\"a\":1}", "{\"root\": \"d/s\", \"a\":1}"),
            ("{ \"a\":1}", "{\"root\": \"d/s\", \"a\":1}"),
            (
                "{\"a\": 1, \"root\": \"old\" , \"b\": 2}",
                "{\"a\": 1, \"root\": \"d/s\" , \"b\": 2}",
            ),
            // A nested or quoted `root` is not the top-level key.
            (
                "{\"n\": {\"root\": \"in\"}, \"s\": \"\\\"root\\\": 1\"}",
                "{\"root\": \"d/s\", \"n\": {\"root\": \"in\"}, \"s\": \"\\\"root\\\": 1\"}",
            ),
        ];
        for (input, want) in cases {
            assert_eq!(set_root(input, "d/s").unwrap(), want, "{input:?}");
        }
    }

    #[test]
    fn set_root_fails_closed() {
        for bad in ["", "{\"a\": ", "[]", "\"x\"", "{\"root\": 3}", "{\"a\":1,}"] {
            assert!(set_root(bad, "d/s").is_err(), "{bad:?}");
        }
    }

    #[test]
    fn bump_updated_keeps_line_endings() {
        let up = |s: &str| String::from_utf8(bump_updated(s.as_bytes(), "2030-01-02")).unwrap();
        assert_eq!(
            up("---\ntype: x\nupdated: 2026-01-01\n---\nbody\n"),
            "---\ntype: x\nupdated: 2030-01-02\n---\nbody\n"
        );
        assert_eq!(
            up("---\r\ntype: x\r\n---\r\nbody\r\n"),
            "---\r\ntype: x\r\nupdated: 2030-01-02\r\n---\r\nbody\r\n"
        );
        assert_eq!(up("no frontmatter\n"), "no frontmatter\n");
        assert_eq!(up("---\nunclosed\n"), "---\nunclosed\n");
    }

    #[test]
    fn markers_go_after_the_frontmatter() {
        let ids: BTreeSet<&str> = ["SF30", "SF22"].into_iter().collect();
        let put = |s: &str| String::from_utf8(insert_markers(s.as_bytes(), &ids)).unwrap();
        assert_eq!(
            put("---\ntype: x\n---\n\nbody\n"),
            "---\ntype: x\n---\n<!-- vibe-spec: allow SF22 -->\n<!-- vibe-spec: allow SF30 -->\n\nbody\n"
        );
        assert_eq!(
            put("---\r\ntype: x\r\n---\r\nbody"),
            "---\r\ntype: x\r\n---\r\n<!-- vibe-spec: allow SF22 -->\r\n<!-- vibe-spec: allow SF30 -->\r\nbody"
        );
        assert_eq!(
            put("body\n"),
            "<!-- vibe-spec: allow SF22 -->\n<!-- vibe-spec: allow SF30 -->\nbody\n"
        );
    }
}
