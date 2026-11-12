//! `vibe-spec lessons-for` — port of `spec/scripts/lessons-for.sh`, plus
//! `--json` (Spec JSON v1 `lessons-for`) and `--tag <t>`.
//!
//! Deliberate divergences (pinned in tests/parity/spec.json): tags match as
//! whole, case-insensitive tags, not as regexes or substrings; the legacy
//! `--format json` array escapes its strings.
use serde_json::{Value, json};
use vibe_core::exit;

use crate::root::{Miss, Root};
use crate::text::{
    is_date, is_space, json_str, lines, lossy, remove_first, strip_prefix_or, trim_space,
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
pub fn run(
    cx: &crate::api::Execution<'_>,
    found: &Result<Root, Miss>,
    args: &[String],
) -> (u8, Vec<u8>) {
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
                    diag!(
                        cx,
                        "lessons-for: --format requires a value (markdown|inject|json)"
                    );
                    return (exit::USAGE, Vec::new());
                }
            },
            "--json" => spec_json = true,
            "--tag" => match args.get(i + 1) {
                Some(v) if v.is_empty() => {
                    diag!(cx, "vibe-spec: a lessons-for tag must not be empty");
                    return (exit::USAGE, Vec::new());
                }
                Some(v) => {
                    tags.push(v.clone());
                    i += 2;
                    continue;
                }
                None => {
                    diag!(cx, "lessons-for: --tag requires a value");
                    return (exit::USAGE, Vec::new());
                }
            },
            "markdown" | "inject" | "json" => format = Format::parse(a),
            _ if a.starts_with("--format=") => format = Format::parse(&a["--format=".len()..]),
            _ if a.starts_with('-') => {
                diag!(cx, "Unknown option: {a}");
                return (exit::USAGE, Vec::new());
            }
            _ => tags.push(a.to_string()),
        }
        i += 1;
    }
    if spec_json {
        return answer(cx, found, &tags);
    }
    let file = match found {
        Ok(root) => root.dir.join("lessons.md"),
        Err(Miss::NotFound) => {
            diag!(
                cx,
                "vibe-spec lessons-for: no spec root (searched docs/spec/ and the legacy root up to .git)"
            );
            return (exit::OK, Vec::new());
        }
        Err(miss) => {
            diag!(cx, "{}", miss.message());
            return (miss.code(), Vec::new());
        }
    };
    if !file.is_file() {
        let show = found.as_ref().map(Root::display).unwrap_or_default();
        diag!(cx, "vibe-spec lessons-for: {show}/lessons.md not found");
        return (exit::OK, Vec::new());
    }
    if tags.is_empty() {
        diag!(
            cx,
            "usage: vibe-spec lessons-for <tag> [<tag>...] [--format markdown|inject|json]"
        );
        return (exit::USAGE, Vec::new());
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

/// The first `**Tags:**` line, split on `,`: trimmed (CR included), lower-cased, empty entries dropped.
fn tags_of(block: &[u8]) -> Vec<String> {
    let line = lines(block)
        .into_iter()
        .find(|l| l.starts_with(b"**Tags:**"))
        .unwrap_or_default();
    line.get(b"**Tags:**".len()..)
        .unwrap_or_default()
        .split(|&c| c == b',')
        .map(trim_space)
        .filter(|t| !t.is_empty())
        .map(|t| lossy(t).to_ascii_lowercase())
        .collect()
}

/// Any requested tag equals one of the lesson's tags, ignoring ASCII case
/// (an empty tag always matches).
fn matches(block: &[u8], tags: &[String]) -> bool {
    let own = tags_of(block);
    tags.iter()
        .any(|t| t.is_empty() || own.contains(&t.to_ascii_lowercase()))
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
fn answer(
    cx: &crate::api::Execution<'_>,
    found: &Result<Root, Miss>,
    tags: &[String],
) -> (u8, Vec<u8>) {
    let root = match found {
        Ok(root) => root,
        Err(miss) => {
            diag!(cx, "{}", miss.message());
            return (miss.code(), Vec::new());
        }
    };
    if tags.is_empty() {
        diag!(cx, "usage: vibe-spec lessons-for --json <tag> [<tag>...]");
        return (exit::USAGE, Vec::new());
    }
    if tags.iter().any(String::is_empty) {
        diag!(cx, "vibe-spec: a lessons-for tag must not be empty");
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

#[derive(Clone, Debug)]
pub struct LessonsRequest {
    pub selector: crate::ContextSelector,
    pub tags: Vec<String>,
}
pub(crate) fn data(text: &[u8], tags: &[String]) -> Result<Value, crate::SpecError> {
    if tags.is_empty() || tags.iter().any(String::is_empty) {
        return Err(crate::SpecError::new(
            "invalid_tags",
            "at least one nonempty tag is required",
        ));
    }
    let lessons: Vec<_> = blocks(text)
        .iter()
        .filter(|b| matches(b, tags))
        .filter_map(|b| lesson(b))
        .collect();
    Ok(json!({"v":1,"tags":tags,"lessons":lessons}))
}

pub(crate) fn managed_tags(args: &[String]) -> Result<Vec<String>, crate::SpecError> {
    let mut tags = Vec::new();
    let mut i = 0;
    while let Some(arg) = args.get(i) {
        match arg.as_str() {
            "--json" | "markdown" | "inject" | "json" => {}
            "--format" => {
                i += 1;
                if args.get(i).is_none() {
                    return Err(crate::SpecError::new(
                        "invalid_tags",
                        "--format requires a value",
                    ));
                }
            }
            "--tag" => {
                i += 1;
                tags.push(
                    args.get(i)
                        .ok_or_else(|| {
                            crate::SpecError::new("invalid_tags", "--tag requires a value")
                        })?
                        .clone(),
                );
            }
            a if a.starts_with("--format=") => {}
            a if a.starts_with('-') => {
                return Err(crate::SpecError::new(
                    "invalid_tags",
                    format!("Unknown option: {a}"),
                ));
            }
            _ => tags.push(arg.clone()),
        }
        i += 1;
    }
    Ok(tags)
}

pub(crate) fn render_managed(
    text: &[u8],
    args: &[String],
) -> Result<(u8, Vec<u8>), crate::SpecError> {
    if crate::wants_help(args) {
        return Ok((0, USAGE.as_bytes().to_vec()));
    }
    let tags = managed_tags(args)?;
    if args.iter().any(|a| a == "--json") {
        return Ok((0, format!("{}\n", data(text, &tags)?).into_bytes()));
    }
    if tags.is_empty() {
        return Err(crate::SpecError::new(
            "invalid_tags",
            "at least one nonempty tag is required",
        ));
    }
    let mut format = Format::Markdown;
    for (n, arg) in args.iter().enumerate() {
        if arg == "--format" {
            format = Format::parse(args.get(n + 1).map_or("", String::as_str));
        } else if let Some(v) = arg.strip_prefix("--format=") {
            format = Format::parse(v);
        } else if matches!(arg.as_str(), "markdown" | "inject" | "json") {
            format = Format::parse(arg);
        }
    }
    let matched: Vec<_> = blocks(text)
        .into_iter()
        .filter(|b| matches(b, &tags))
        .collect();
    if matched.is_empty() {
        return Ok((0, Vec::new()));
    }
    let mut out = Vec::new();
    match format {
        Format::Markdown => {
            for block in matched {
                out.extend(cat!(block, "\n---\n"));
            }
        }
        Format::Inject => {
            out.extend(format!("<!-- lessons: {} -->\n", tags.join(" ")).bytes());
            for block in matched {
                out.extend(cat!(block, "\n\n"));
            }
            out.extend_from_slice(b"<!-- /lessons -->\n");
        }
        Format::Json => {
            out.push(b'[');
            for (n, block) in matched.iter().enumerate() {
                if n > 0 {
                    out.push(b',');
                }
                out.extend(legacy_json(block));
            }
            out.extend_from_slice(b"]\n");
        }
    }
    Ok((0, out))
}
