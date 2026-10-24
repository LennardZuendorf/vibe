//! `vibe-spec lessons-for` — port of `spec/scripts/lessons-for.sh`, plus
//! `--json` (Spec JSON v1 `lessons-for`) and `--tag <t>`.
//!
//! Deliberate divergences (pinned in tests/parity/spec.json): tags match as
//! case-insensitive literal substrings, not as regexes; the legacy
//! `--format json` array escapes its strings.
use serde_json::{Value, json};
use vibe_core::exit;

use crate::root::{Miss, Root};
use crate::text::{
    contains_ci, is_date, is_space, json_str, lines, lossy, remove_first, strip_prefix_or,
    trim_space,
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Format {
    Markdown,
    Inject,
    Json,
}

impl Format {
    fn parse(s: &str) -> Format {
        match s {
            "inject" => Format::Inject,
            "json" => Format::Json,
            _ => Format::Markdown,
        }
    }
}

const USAGE: &str = "Usage: vibe-spec lessons-for <tag> [<tag>...] [--format markdown|inject|json] [--tag <tag>] [--json]\n\nPrint the lessons whose Tags line matches a tag. --json prints Spec JSON v1.\n";

/// Runs the command.
pub fn run(found: &Result<Root, Miss>, args: &[String]) -> (u8, Vec<u8>) {
    if crate::wants_help(args) {
        return (exit::OK, USAGE.as_bytes().to_vec());
    }
    let mut format = Format::Markdown;
    let mut tags: Vec<String> = Vec::new();
    let mut spec_json = false;
    let mut i = 0;
    while let Some(arg) = args.get(i) {
        let a = arg.as_str();
        match a {
            "--format" => match args.get(i + 1).filter(|v| !v.is_empty()) {
                Some(v) => {
                    format = Format::parse(v);
                    i += 2;
                    continue;
                }
                None => {
                    eprintln!("lessons-for: --format requires a value (markdown|inject|json)");
                    return (exit::FINDINGS, Vec::new());
                }
            },
            "--json" => spec_json = true,
            "--tag" => match args.get(i + 1) {
                Some(v) if v.is_empty() => {
                    eprintln!("vibe-spec: a lessons-for tag must not be empty");
                    return (exit::USAGE, Vec::new());
                }
                Some(v) => {
                    tags.push(v.clone());
                    i += 2;
                    continue;
                }
                None => {
                    eprintln!("lessons-for: --tag requires a value");
                    return (exit::USAGE, Vec::new());
                }
            },
            "markdown" | "inject" | "json" => format = Format::parse(a),
            _ if a.starts_with("--format=") => format = Format::parse(&a["--format=".len()..]),
            _ if a.starts_with('-') => {
                eprintln!("Unknown option: {a}");
                return (exit::FINDINGS, Vec::new());
            }
            _ => tags.push(a.to_string()),
        }
        i += 1;
    }
    if spec_json {
        return answer(found, &tags);
    }
    let file = match found {
        Ok(root) => root.dir.join("lessons.md"),
        Err(Miss::NotFound) => return (exit::OK, Vec::new()),
        Err(miss) => {
            eprintln!("{}", miss.message());
            return (miss.code(), Vec::new());
        }
    };
    if !file.is_file() {
        return (exit::OK, Vec::new());
    }
    if tags.is_empty() {
        eprintln!("usage: vibe-spec lessons-for <tag> [<tag>...] [--format markdown|inject|json]");
        return (exit::FINDINGS, Vec::new());
    }
    let text = std::fs::read(&file).unwrap_or_default();
    let matched: Vec<Vec<u8>> = blocks(&text)
        .into_iter()
        .filter(|b| matches(b, &tags))
        .collect();
    if matched.is_empty() {
        return (exit::OK, Vec::new());
    }
    let mut out = Vec::new();
    match format {
        Format::Inject => {
            out.extend(format!("<!-- lessons: {} -->\n", tags.join(" ")).bytes());
            for b in &matched {
                out.extend(cat!(b, "\n\n"));
            }
            out.extend_from_slice(b"<!-- /lessons -->\n");
        }
        Format::Json => {
            out.push(b'[');
            for (n, b) in matched.iter().enumerate() {
                if n > 0 {
                    out.push(b',');
                }
                out.extend(legacy_json(b));
            }
            out.extend_from_slice(b"]\n");
        }
        Format::Markdown => {
            for b in &matched {
                out.extend(cat!(b, "\n---\n"));
            }
        }
    }
    (exit::OK, out)
}

/// `^###[[:space:]]`.
fn is_heading(line: &[u8]) -> bool {
    line.starts_with(b"###") && line.get(3).is_some_and(|&c| is_space(c))
}

/// Lesson blocks: each `### ` heading with the lines up to the next one, joined by `\n`.
fn blocks(text: &[u8]) -> Vec<Vec<u8>> {
    let mut out = Vec::new();
    let mut current: Option<Vec<u8>> = None;
    for line in lines(text) {
        if is_heading(line) {
            out.extend(current.take());
            current = Some(line.to_vec());
        } else if let Some(block) = current.as_mut() {
            block.push(b'\n');
            block.extend_from_slice(line);
        }
    }
    out.extend(current);
    out
}

/// The first `**Tags:**` line contains any tag (an empty tag always matches).
fn matches(block: &[u8], tags: &[String]) -> bool {
    let tags_line = lines(block)
        .into_iter()
        .find(|l| l.starts_with(b"**Tags:**"))
        .unwrap_or_default();
    tags.iter().any(|t| contains_ci(tags_line, t.as_bytes()))
}

/// Every line starting with `key`, `strip` removed once, joined by `\n`, trailing newlines dropped.
fn joined(block: &[u8], key: &[u8], strip: &[u8]) -> Vec<u8> {
    let mut v = lines(block)
        .into_iter()
        .filter(|l| l.starts_with(key))
        .map(|l| remove_first(l, strip))
        .collect::<Vec<_>>()
        .join(&b'\n');
    while v.last() == Some(&b'\n') {
        v.pop();
    }
    v
}

/// One element of the legacy `--format json` array (strings escaped).
fn legacy_json(block: &[u8]) -> Vec<u8> {
    let title = strip_prefix_or(lines(block).first().copied().unwrap_or_default(), b"### ");
    cat!(
        "{\"title\":",
        json_str(title),
        ",\"pattern\":",
        json_str(&joined(block, b"**Pattern:**", b"**Pattern:** ")),
        ",\"rule\":",
        json_str(&joined(block, b"**Rule:**", b"**Rule:** ")),
        ",\"tags\":",
        json_str(&joined(block, b"**Tags:**", b"**Tags:** ")),
        ",\"date\":",
        json_str(&joined(block, b"**Date:**", b"**Date:** ")),
        "}"
    )
}

/// `--json`: Spec JSON v1 `lessons-for`.
fn answer(found: &Result<Root, Miss>, tags: &[String]) -> (u8, Vec<u8>) {
    let root = match found {
        Ok(root) => root,
        Err(miss) => {
            eprintln!("{}", miss.message());
            return (miss.code(), Vec::new());
        }
    };
    if tags.is_empty() {
        eprintln!("usage: vibe-spec lessons-for --json <tag> [<tag>...]");
        return (exit::USAGE, Vec::new());
    }
    if tags.iter().any(String::is_empty) {
        eprintln!("vibe-spec: a lessons-for tag must not be empty");
        return (exit::USAGE, Vec::new());
    }
    let text = std::fs::read(root.dir.join("lessons.md")).unwrap_or_default();
    let lessons: Vec<Value> = blocks(&text)
        .iter()
        .filter(|b| matches(b, tags))
        .filter_map(|b| lesson(b))
        .collect();
    let v = json!({ "v": 1, "tags": tags, "lessons": lessons });
    (exit::OK, format!("{v}\n").into_bytes())
}

/// One Spec JSON lesson; `None` without a title or a rule.
fn lesson(block: &[u8]) -> Option<Value> {
    let ls = lines(block);
    let title = trim_space(strip_prefix_or(ls.first().copied()?, b"###"));
    let value = |key: &[u8]| {
        ls.iter()
            .find(|l| l.starts_with(key))
            .map(|l| trim_space(l.get(key.len()..).unwrap_or_default()).to_vec())
    };
    let rule = value(b"**Rule:**").filter(|r| !r.is_empty())?;
    if title.is_empty() {
        return None;
    }
    let tags: Vec<String> = value(b"**Tags:**")
        .map(|t| {
            t.split(|&c| c == b',')
                .map(trim_space)
                .filter(|t| !t.is_empty())
                .map(lossy)
                .collect()
        })
        .unwrap_or_default();
    let mut v = json!({ "title": lossy(title), "rule": lossy(&rule), "tags": tags });
    if let Some(p) = value(b"**Pattern:**").filter(|p| !p.is_empty()) {
        v["pattern"] = json!(lossy(&p));
    }
    if let Some(d) = value(b"**Date:**").filter(|d| is_date(d)) {
        v["date"] = json!(lossy(&d));
    }
    if ls.iter().any(|l| l.starts_with(b"**Pinned-by:**")) {
        v["pinned"] = json!(true);
    }
    Some(v)
}
