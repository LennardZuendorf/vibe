//! The layered layout (D44, P3b-R10, R11): detection, the tree walk, and
//! the layered-only checks — SF22 links across notes, SF30 allowlist, SF31
//! budgets, SF32 names (Task 10 adds SF33, SF34).
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::adherence::feature_files;
use crate::frontmatter;
use crate::pattern::{md_link_target, md_links};
use crate::report::Report;
use crate::text::{Fences, join_raw, lines, trim_space};
use crate::validate::Ctx;

/// A root's layout.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Layout {
    /// Legacy `.spec/`-style layout.
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

/// SF22 for a layered file: every local link resolves (relative to the file; `/` = the project).
fn links(r: &mut Report, ctx: &Ctx, path: &Path, rel: &str, text: &[u8]) {
    let base = path.parent().unwrap_or(ctx.dir);
    let mut fences = Fences::new();
    for line in lines(text) {
        if fences.skip(line) {
            continue;
        }
        for link in md_links(line) {
            let target = clean_target(&md_link_target(link));
            if skip_target(&target) {
                continue;
            }
            let resolved = match target.strip_prefix(b"/") {
                Some(abs) => join_raw(ctx.project, abs),
                None => join_raw(base, &target),
            };
            if !resolved.exists() {
                r.warn("SF22", &cat!(rel, ": broken link to '", target, "'"));
            }
        }
    }
}

/// The layered-only checks.
pub fn run(r: &mut Report, ctx: &Ctx) {
    let mut stems: BTreeMap<String, String> = BTreeMap::new();
    for (path, parts) in walk(ctx.dir) {
        let refs: Vec<&str> = parts.iter().map(String::as_str).collect();
        let rel = parts.join("/");
        let place = place(&refs);
        let shown = format!("{}/{rel}", ctx.display);
        if place == Place::Other {
            r.at(&shown);
            r.warn("SF30", format!("{rel}: not in the layered layout (hubs, README.md, AGENTS.md, lessons.md, kind folders, parts/<part>/, features/<f>/, archive/, quick/, templates/)").as_bytes());
            continue;
        }
        if matches!(place, Place::Exempt | Place::Feature | Place::Lessons) {
            continue;
        }
        r.examine(&shown);
        let text = std::fs::read(&path).unwrap_or_default();
        budget(r, &rel, place, &text);
        names(r, &rel, &refs, place, &mut stems);
        if place != Place::Hub {
            links(r, ctx, &path, &rel, &text);
        }
    }
}
