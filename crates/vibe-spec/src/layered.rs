//! The layered layout (D44, P3b-R10, R11): detection, the tree walk, and
//! the layered-only checks — SF22 links across notes, SF30 allowlist, SF31
//! budgets, SF32 names, SF33 typed notes, SF34 reachability; notes also get
//! the SF10/SF25 grammar checks.
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::path::{Component, Path, PathBuf};

use crate::adherence::{FEATURE_TYPES, LAYERED_TYPES, feature_files};
use crate::content;
use crate::frontmatter::{self, Value};
use crate::pattern::{md_link_target, md_links};
use crate::report::Report;
use crate::text::{Fences, join_raw, lines, trim_space};
use crate::validate::Ctx;

/// A root's layout.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Layout {
    /// Legacy `.spec/`-style layout. // literal
    Flat,
    /// D44 hubs and kind folders.
    Layered,
}

impl Layout {
    /// The Spec JSON spelling.
    pub fn as_str(self) -> &'static str {
        match self {
            Layout::Flat => "flat",
            Layout::Layered => "layered",
        }
    }
}

const HUBS: [&str; 4] = ["product.md", "tech.md", "design.md", "plan.md"];
const FEATURE_DOCS: [&str; 5] = [
    "product.md",
    "tech.md",
    "design.md",
    "plan.md",
    "research.md",
];

/// Kind folder → the note `type` it holds.
pub const KINDS: [(&str, &str); 7] = [
    ("areas", "area"),
    ("flows", "flow"),
    ("concepts", "concept"),
    ("platform", "platform"),
    ("tech", "tech"),
    ("decisions", "decision"),
    ("questions", "question"),
];

/// Layered when a root hub or `README.md` declares `type: hub` (P3b-R10).
pub fn detect(dir: &Path) -> Layout {
    let hub = HUBS.iter().chain(["README.md"].iter()).any(|name| {
        std::fs::read(dir.join(name))
            .ok()
            .and_then(|t| frontmatter::parse(&t))
            .is_some_and(|f| f.scalar("type") == Some("hub"))
    });
    if hub { Layout::Layered } else { Layout::Flat }
}

/// Where a file sits in a layered tree.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Place {
    /// Root `product|tech|design|plan.md`.
    Hub,
    /// Root `README.md`/`AGENTS.md`, or a `README.md` hub inside a kind folder.
    Readme,
    /// Root `lessons.md` (unbounded).
    Lessons,
    /// `parts/<part>/*.md`.
    PartHub,
    /// A note under a kind folder (also inside `parts/<part>/`), with its type.
    Note(&'static str),
    /// `features/<f>/{product,tech,design,plan,research}.md`.
    Feature,
    /// `archive/<name>/**`, `quick/<slug>.md`, `templates/**`.
    Exempt,
    /// Anything else (SF30).
    Other,
}

fn kind_type(folder: &str) -> Option<&'static str> {
    KINDS.iter().find(|(k, _)| *k == folder).map(|(_, t)| *t)
}

fn in_kind(kind: Option<&'static str>, rest: &[&str]) -> Place {
    match (kind, rest.last()) {
        (Some(_), Some(&"README.md" | &"AGENTS.md")) => Place::Readme,
        (Some(t), Some(last)) if last.ends_with(".md") => Place::Note(t),
        _ => Place::Other,
    }
}

/// Classifies a root-relative path given as components.
pub fn place(parts: &[&str]) -> Place {
    match parts {
        [name] if HUBS.contains(name) => Place::Hub,
        ["README.md"] | ["AGENTS.md"] => Place::Readme,
        ["lessons.md"] => Place::Lessons,
        ["features", _, doc] if FEATURE_DOCS.contains(doc) => Place::Feature,
        ["archive", _, _, ..] | ["templates", _, ..] => Place::Exempt,
        ["quick", slug] if slug.ends_with(".md") => Place::Exempt,
        ["parts", _, "AGENTS.md"] => Place::Readme,
        ["parts", _, name] if name.ends_with(".md") => Place::PartHub,
        ["parts", _, kind, rest @ ..] if !rest.is_empty() => in_kind(kind_type(kind), rest),
        [kind, rest @ ..] if !rest.is_empty() => in_kind(kind_type(kind), rest),
        _ => Place::Other,
    }
}

/// Every non-hidden file under `dir` with its root-relative components,
/// byte-sorted. Symlinks are not followed.
pub fn walk(dir: &Path) -> Vec<(PathBuf, Vec<String>)> {
    fn go(dir: &Path, prefix: &[String], out: &mut Vec<(PathBuf, Vec<String>)>) {
        let Ok(read) = std::fs::read_dir(dir) else {
            return;
        };
        let mut entries: Vec<_> = read.flatten().collect();
        entries.sort_by_key(std::fs::DirEntry::file_name);
        for e in entries {
            let name = e.file_name().to_string_lossy().into_owned();
            if name.starts_with('.') {
                continue;
            }
            let mut parts = prefix.to_vec();
            parts.push(name);
            if e.file_type().is_ok_and(|t| t.is_dir()) {
                go(&e.path(), &parts, out);
            } else {
                out.push((e.path(), parts));
            }
        }
    }
    let mut out = Vec::new();
    go(dir, &[], &mut out);
    out
}

/// The files SF26/SF29 read in a layered tree.
pub fn spec_files(dir: &Path) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = walk(dir)
        .into_iter()
        .filter(|(_, parts)| {
            let refs: Vec<&str> = parts.iter().map(String::as_str).collect();
            matches!(
                place(&refs),
                Place::Hub | Place::Readme | Place::PartHub | Place::Note(_)
            )
        })
        .map(|(p, _)| p)
        .collect();
    out.extend(feature_files(dir));
    out
}

fn kebab(s: &str) -> bool {
    !s.is_empty()
        && s.split('-').all(|w| {
            !w.is_empty()
                && w.bytes()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
        })
}

/// SF31: hubs ≤80 lines, notes ≤60 (`lines` counts a last line without `\n`).
fn budget(r: &mut Report, rel: &str, place: Place, text: &[u8]) {
    let hub_typed = frontmatter::parse(text).is_some_and(|f| f.scalar("type") == Some("hub"));
    let (limit, what) = match place {
        Place::Hub | Place::Readme | Place::PartHub => (80, "hub"),
        Place::Note(_) if hub_typed => (80, "hub"),
        Place::Note(_) => (60, "note"),
        _ => return,
    };
    let n = text.iter().filter(|&&b| b == b'\n').count();
    if n > limit {
        r.warn("SF31", format!("{rel}: {n} lines, over the {limit}-line {what} budget — split it into a folder with its own hub").as_bytes());
    }
}

/// SF32: lowercase-kebab folders and files; note names unique across the tree.
fn names(
    r: &mut Report,
    rel: &str,
    parts: &[&str],
    place: Place,
    stems: &mut BTreeMap<String, String>,
) {
    for (i, part) in parts.iter().enumerate() {
        let last = i + 1 == parts.len();
        if last && (*part == "README.md" || *part == "AGENTS.md") {
            break;
        }
        let stem = if last {
            part.strip_suffix(".md").unwrap_or(part)
        } else {
            part
        };
        if !kebab(stem) {
            r.warn(
                "SF32",
                format!("{rel}: '{part}' is not lowercase-kebab").as_bytes(),
            );
            return;
        }
    }
    if matches!(place, Place::Note(_)) {
        let stem = parts
            .last()
            .and_then(|p| p.strip_suffix(".md"))
            .unwrap_or_default()
            .to_string();
        match stems.get(&stem) {
            Some(other) => r.warn(
                "SF32",
                format!("{rel}: name '{stem}' is also used by {other}").as_bytes(),
            ),
            None => {
                stems.insert(stem, rel.to_string());
            }
        }
    }
}

/// A link target that is not a local path: empty (`#anchor`), a template, or a URL scheme.
fn skip_target(t: &[u8]) -> bool {
    let colon = t.iter().position(|&c| c == b':');
    let slash = t.iter().position(|&c| c == b'/');
    t.is_empty()
        || t.contains(&b'{')
        || matches!((colon, slash), (Some(c), s) if s.is_none_or(|s| c < s))
}

/// A link target as a path: no `?query`, no CommonMark title, no `<...>`, trimmed.
fn clean_target(raw: &[u8]) -> Vec<u8> {
    let mut t = trim_space(raw);
    if let Some(inner) = t.strip_prefix(b"<") {
        t = inner
            .iter()
            .position(|&c| c == b'>')
            .map_or(inner, |e| &inner[..e]);
    } else if let Some(i) = t
        .windows(2)
        .position(|w| w[0].is_ascii_whitespace() && matches!(w[1], b'"' | b'\''))
    {
        t = &t[..i];
    }
    if let Some(q) = t.iter().position(|&c| c == b'?') {
        t = &t[..q];
    }
    trim_space(t).to_vec()
}

/// `line` without inline code spans (`` `…` ``, a run of N backticks to the
/// next run of exactly N). An unclosed run stays as written.
fn without_code_spans(line: &[u8]) -> Vec<u8> {
    let run = |from: usize| line[from..].iter().take_while(|&&c| c == b'`').count();
    let mut out = Vec::with_capacity(line.len());
    let mut i = 0;
    while i < line.len() {
        if line[i] != b'`' {
            out.push(line[i]);
            i += 1;
            continue;
        }
        let n = run(i);
        let mut j = i + n;
        let mut close = None;
        while j < line.len() {
            if line[j] == b'`' {
                let m = run(j);
                if m == n {
                    close = Some(j + m);
                    break;
                }
                j += m;
            } else {
                j += 1;
            }
        }
        match close {
            Some(end) => i = end,
            None => {
                out.extend_from_slice(&line[i..i + n]);
                i += n;
            }
        }
    }
    out
}

/// The raw targets of every link in `text`, fenced code and inline code spans skipped.
fn link_targets(text: &[u8]) -> Vec<Vec<u8>> {
    let mut fences = Fences::new();
    let mut out = Vec::new();
    for line in lines(text) {
        if fences.skip(line) {
            continue;
        }
        let line = without_code_spans(line);
        out.extend(md_links(&line).into_iter().map(md_link_target));
    }
    out
}

/// Where a local link points.
struct Link {
    /// The cleaned target, as written.
    target: Vec<u8>,
    /// Inside the spec root: the walk's rel key it resolves to.
    key: Option<String>,
    /// The target exists (in the walk's rel set, or on disk outside the root).
    exists: bool,
}

/// `p` with `.` dropped and `..` applied, without touching the disk.
pub(crate) fn lexical(p: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for c in p.components() {
        match c {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            c => out.push(c),
        }
    }
    out
}

/// The one layered link resolver (SF22 and SF34). `None` for a target that
/// is not a local path. Order: clean, skip, `/` = project-absolute else
/// relative to the file's directory, `.`/`..`, then a directory means its
/// `README.md`. Inside the spec root the walk's `rels` decide, byte-exact
/// (so a case-only difference is broken on every OS); outside it `exists()`.
fn resolve(ctx: &Ctx, rels: &BTreeSet<String>, dir_rel: &[&str], raw: &[u8]) -> Option<Link> {
    let target = clean_target(raw);
    if skip_target(&target) {
        return None;
    }
    let root = lexical(ctx.dir);
    let project = lexical(ctx.project);
    let (mut path, rest) = match target.strip_prefix(b"/") {
        Some(abs) => (project.clone(), abs),
        None => (
            dir_rel.iter().fold(root.clone(), |p, d| p.join(d)),
            &target[..],
        ),
    };
    for seg in rest.split(|&c| c == b'/') {
        match seg {
            b"" | b"." => {}
            b".." => {
                path.pop();
            }
            _ => path = join_raw(&path, seg),
        }
    }
    let (key, exists) = match path.strip_prefix(&root) {
        Ok(inside) => {
            let inside = inside
                .components()
                .map(|c| c.as_os_str().to_string_lossy().into_owned())
                .collect::<Vec<_>>()
                .join("/");
            let readme = if inside.is_empty() {
                "README.md".to_string()
            } else {
                format!("{inside}/README.md")
            };
            let key = [inside, readme]
                .into_iter()
                .find(|k| !k.is_empty() && rels.contains(k));
            let exists = key.is_some();
            (key, exists)
        }
        // Outside the spec root a target must still stay in the project.
        Err(_) => (None, path.starts_with(&project) && path.exists()),
    };
    Some(Link {
        target,
        key,
        exists,
    })
}

/// The directory components of a root-relative file path.
fn dir_of(rel: &str) -> Vec<&str> {
    let mut d: Vec<&str> = rel.split('/').collect();
    d.pop();
    d
}

/// SF22 for a layered file: every local link resolves (relative to the file; `/` = the project).
fn links(r: &mut Report, ctx: &Ctx, rels: &BTreeSet<String>, rel: &str, text: &[u8]) {
    let dir = dir_of(rel);
    for raw in link_targets(text) {
        if let Some(link) = resolve(ctx, rels, &dir, &raw)
            && !link.exists
        {
            r.warn("SF22", &cat!(rel, ": broken link to '", link.target, "'"));
        }
    }
}

/// SF33: notes carry the type of their folder (or `hub`), `updated:`, and
/// `code:` (areas, existing paths from the project) or `applies:` (tech);
/// root and part hubs declare `type: hub`.
fn typed(r: &mut Report, ctx: &Ctx, rel: &str, place: Place, text: &[u8]) {
    match place {
        Place::Note(want) => {
            let Some(fm) = frontmatter::parse(text) else {
                r.warn(
                    "SF33",
                    format!("{rel}: no frontmatter (needs 'type: {want}' and 'updated:')")
                        .as_bytes(),
                );
                return;
            };
            // A blank or out-of-vocabulary `type` is SF26's.
            match fm.get("type") {
                None => r.warn(
                    "SF33",
                    format!("{rel}: missing 'type:' (expected '{want}')").as_bytes(),
                ),
                Some(Value::Scalar(t))
                    if t != want
                        && t != "hub"
                        && (LAYERED_TYPES.contains(&t.as_str())
                            || FEATURE_TYPES.contains(&t.as_str())) =>
                {
                    r.warn(
                        "SF33",
                        format!("{rel}: type '{t}' does not match its folder (expected '{want}')")
                            .as_bytes(),
                    );
                }
                Some(_) => {}
            }
            // A blank `updated:` is SF26's.
            if fm.get("updated").is_none() {
                r.warn("SF33", format!("{rel}: missing 'updated:'").as_bytes());
            }
            match want {
                "area" => {
                    let code = fm.list("code");
                    if code.is_empty() {
                        r.warn(
                            "SF33",
                            format!("{rel}: an area needs 'code:' (a path or a list of paths)")
                                .as_bytes(),
                        );
                    }
                    for c in code {
                        let p = join_raw(ctx.project, c.trim_start_matches('/').as_bytes());
                        if !p.exists() {
                            r.warn(
                                "SF33",
                                format!("{rel}: code path '{c}' does not exist").as_bytes(),
                            );
                        }
                    }
                }
                "tech" if fm.list("applies").is_empty() => {
                    r.warn(
                        "SF33",
                        format!("{rel}: a tech note needs 'applies:' (a glob)").as_bytes(),
                    );
                }
                _ => {}
            }
        }
        Place::Hub | Place::PartHub => {
            // A root hub with no frontmatter or no `type` is SF20's; a blank
            // or out-of-vocabulary `type` is SF26's. A part hub has no
            // legacy check, so its missing `type` stays here.
            let fm = frontmatter::parse(text);
            let wrong = match fm.as_ref().map(|f| f.get("type")) {
                None => place == Place::PartHub,
                Some(None) => place == Place::PartHub,
                Some(Some(Value::Scalar(t))) => {
                    t != "hub"
                        && (LAYERED_TYPES.contains(&t.as_str())
                            || FEATURE_TYPES.contains(&t.as_str()))
                }
                Some(Some(_)) => false,
            };
            if wrong {
                r.warn(
                    "SF33",
                    format!("{rel}: a hub declares 'type: hub'").as_bytes(),
                );
            }
        }
        _ => {}
    }
}

/// SF34: every note, part hub and folder README is reachable from the root
/// hubs, README.md or AGENTS.md by local links (fences and code spans skipped).
fn reach(
    r: &mut Report,
    ctx: &Ctx,
    files: &[(PathBuf, Vec<String>)],
    rels: &BTreeSet<String>,
    texts: &BTreeMap<String, Vec<u8>>,
) {
    let mut seen: BTreeSet<String> = HUBS
        .iter()
        .chain(["README.md", "AGENTS.md"].iter())
        .map(|s| s.to_string())
        .filter(|s| rels.contains(s))
        .collect();
    let mut queue: VecDeque<String> = seen.iter().cloned().collect();
    while let Some(rel) = queue.pop_front() {
        let text = texts.get(&rel).map_or(&[][..], Vec::as_slice);
        let dir = dir_of(&rel);
        for raw in link_targets(text) {
            if let Some(next) = resolve(ctx, rels, &dir, &raw).and_then(|l| l.key)
                && seen.insert(next.clone())
            {
                queue.push_back(next);
            }
        }
    }
    for (_, parts) in files {
        let refs: Vec<&str> = parts.iter().map(String::as_str).collect();
        let reachable_kind = match place(&refs) {
            Place::Note(_) | Place::PartHub => true,
            Place::Readme => refs.len() > 1 && refs.last() != Some(&"AGENTS.md"),
            _ => false,
        };
        let rel = parts.join("/");
        if reachable_kind && !seen.contains(&rel) {
            r.at(&format!("{}/{rel}", ctx.display));
            r.warn(
                "SF34",
                format!(
                    "{rel}: not reachable from a hub (link it from a hub or its folder README.md)"
                )
                .as_bytes(),
            );
        }
    }
}

/// `text` without fenced code lines (and fence delimiters).
fn unfenced(text: &[u8]) -> Vec<u8> {
    let mut fences = Fences::new();
    let mut out = Vec::with_capacity(text.len());
    for line in lines(text) {
        if !fences.skip(line) {
            out.extend_from_slice(line);
            out.push(b'\n');
        }
    }
    out
}

/// The layered-only checks.
pub fn run(r: &mut Report, ctx: &Ctx) {
    let mut stems: BTreeMap<String, String> = BTreeMap::new();
    let files = walk(ctx.dir);
    let rels: BTreeSet<String> = files.iter().map(|(_, parts)| parts.join("/")).collect();
    let mut texts: BTreeMap<String, Vec<u8>> = BTreeMap::new();
    for (path, parts) in &files {
        let refs: Vec<&str> = parts.iter().map(String::as_str).collect();
        let rel = parts.join("/");
        let place = place(&refs);
        let shown = format!("{}/{rel}", ctx.display);
        if place == Place::Other {
            r.at(&shown);
            r.warn("SF30", format!("{rel}: not in the layered layout (hubs, README.md, AGENTS.md, lessons.md, kind folders, parts/<part>/, features/<f>/, archive/, quick/, templates/)").as_bytes());
            continue;
        }
        if place == Place::Lessons {
            r.examine(&shown);
        }
        if matches!(place, Place::Exempt | Place::Feature | Place::Lessons) {
            continue;
        }
        r.examine(&shown);
        let text = std::fs::read(path).unwrap_or_default();
        budget(r, &rel, place, &text);
        names(r, &rel, &refs, place, &mut stems);
        links(r, ctx, &rels, &rel, &text);
        typed(r, ctx, &rel, place, &text);
        texts.insert(rel.clone(), text.clone());
        let grammar = match place {
            Place::Note(_) | Place::PartHub => true,
            Place::Readme => refs.len() > 1,
            _ => false,
        };
        if grammar {
            content::requirements_scenarios(r, &unfenced(&text), rel.as_bytes());
            content::scenario_grammar(r, &text, rel.as_bytes());
        }
    }
    reach(r, ctx, &files, &rels, &texts);
}
