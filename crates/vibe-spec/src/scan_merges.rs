//! `vibe-spec scan-merges` — port of `spec/scripts/scan-merges.sh`.
//!
//! Byte-identical, including its `while read` loop, which drops a final
//! line without a newline, and its 60-byte preview cut. Deliberate
//! divergence (pinned): `--format json` escapes its strings.
use std::path::{Path, PathBuf};

use vibe_core::exit;

use crate::root::{Miss, Root};
use crate::text::{join_raw, json_str, name_of, read_lines, sub_dirs};

struct Block {
    feature: String,
    file: String,
    start: usize,
    end: usize,
    preview: Vec<u8>,
}

const USAGE: &str = "Usage: vibe-spec scan-merges [<feature>] [--format table|json|plain]\n\nReport pending merge blocks in feature tech docs.\n";

/// Runs the command.
pub fn run(
    cx: &crate::api::Execution<'_>,
    found: &Result<Root, Miss>,
    cwd: &Path,
    args: &[String],
) -> (u8, Vec<u8>) {
    if crate::wants_help(args) {
        return (exit::OK, USAGE.as_bytes().to_vec());
    }
    let mut feature: Option<String> = None;
    let mut format = String::from("table");
    let mut i = 0;
    while let Some(arg) = args.get(i) {
        let a = arg.as_str();
        if a == "--format" {
            match args.get(i + 1).filter(|v| !v.is_empty()) {
                Some(v) => {
                    format = v.clone();
                    i += 2;
                    continue;
                }
                None => {
                    diag!(cx, "scan-merges: --format requires table|json|plain");
                    return (exit::USAGE, Vec::new());
                }
            }
        } else if let Some(v) = a.strip_prefix("--format=") {
            format = v.to_string();
        } else if a.starts_with('-') {
            diag!(cx, "Unknown option: {a}");
            return (exit::USAGE, Vec::new());
        } else {
            feature = Some(a.to_string()).filter(|f| !f.is_empty());
        }
        i += 1;
    }
    // The oracle's paths are relative to the project ("$SPEC_DIR" = .spec); // literal
    // without a root it looks for .spec/ under the cwd, which cannot exist. // literal
    let (dir, display): (PathBuf, String) = match found {
        Ok(root) => (root.dir.clone(), root.display()),
        Err(Miss::NotFound) => (cwd.join(".spec"), ".spec".to_string()), // literal
        Err(miss) => {
            diag!(cx, "{}", miss.message());
            return (miss.code(), Vec::new());
        }
    };
    let targets: Vec<(String, String, PathBuf)> = match &feature {
        Some(f) => vec![(
            feature_name(f),
            f.clone(),
            join_raw(&dir.join("features"), f.as_bytes()).join("tech.md"),
        )],
        None => sub_dirs(&dir.join("features"))
            .into_iter()
            .map(|d| (name_of(&d), name_of(&d), d.join("tech.md")))
            .filter(|(_, _, p)| p.is_file())
            .collect(),
    };
    if targets.is_empty() {
        diag!(cx, "No feature tech.md files found.");
        return (exit::OK, Vec::new());
    }
    let mut unclosed = false;
    let mut blocks: Vec<Block> = Vec::new();
    for (feat, rel, path) in &targets {
        if !path.is_file() {
            continue;
        }
        let file = format!("{display}/features/{rel}/tech.md");
        let text = std::fs::read(path).unwrap_or_default();
        let mut open: Option<(usize, Vec<u8>)> = None;
        for (n, line) in read_lines(&text).into_iter().enumerate() {
            let lineno = n + 1;
            if line == b"<!-- merge -->" {
                if let Some((start, _)) = &open {
                    diag!(
                        cx,
                        "ERROR: unclosed <!-- merge --> at line {start} in {file} (nested open at {lineno})"
                    );
                    unclosed = true;
                    continue;
                }
                open = Some((lineno, Vec::new()));
            } else if line == b"<!-- /merge -->" {
                match open.take() {
                    Some((start, preview)) => blocks.push(Block {
                        feature: feat.clone(),
                        file: file.clone(),
                        start,
                        end: lineno,
                        preview,
                    }),
                    None => {
                        diag!(
                            cx,
                            "ERROR: <!-- /merge --> without opener at line {lineno} in {file}"
                        );
                        unclosed = true;
                    }
                }
            } else if let Some((_, preview)) = open.as_mut()
                && preview.is_empty()
                && !line.is_empty()
            {
                *preview = line.get(..60).unwrap_or(line).to_vec();
            }
        }
        if let Some((start, _)) = open {
            diag!(
                cx,
                "ERROR: unclosed <!-- merge --> block starting at line {start} in {file}"
            );
            unclosed = true;
        }
    }
    if blocks.is_empty() && !unclosed {
        return (exit::OK, b"No merge blocks found.\n".to_vec());
    }
    let mut out = Vec::new();
    match format.as_str() {
        "json" => {
            out.push(b'[');
            for (n, b) in blocks.iter().enumerate() {
                if n > 0 {
                    out.push(b',');
                }
                out.extend(cat!(
                    "{\"feature\":",
                    json_str(b.feature.as_bytes()),
                    ",\"file\":",
                    json_str(b.file.as_bytes()),
                    format!(",\"start\":{},\"end\":{},\"preview\":", b.start, b.end),
                    json_str(&b.preview),
                    "}"
                ));
            }
            out.extend_from_slice(b"]\n");
        }
        "plain" => {
            for b in &blocks {
                out.extend(cat!(
                    format!("{}:{}-{}: ", b.file, b.start, b.end),
                    b.preview,
                    "\n"
                ));
            }
        }
        _ => {
            row(&mut out, b"feature", b"file", b"lines", b"preview");
            row(&mut out, b"-------", b"----", b"-----", b"-------");
            for b in &blocks {
                let lines = format!("{}-{}", b.start, b.end);
                row(
                    &mut out,
                    b.feature.as_bytes(),
                    b.file.as_bytes(),
                    lines.as_bytes(),
                    &b.preview,
                );
            }
        }
    }
    (if unclosed { exit::FINDINGS } else { exit::OK }, out)
}

/// `printf '%-25s %-45s %-12s %s\n'`: pads by bytes, never truncates.
fn row(out: &mut Vec<u8>, a: &[u8], b: &[u8], c: &[u8], d: &[u8]) {
    for (cell, width) in [(a, 25_usize), (b, 45), (c, 12)] {
        out.extend_from_slice(cell);
        out.resize(out.len() + width.saturating_sub(cell.len()), b' ');
        out.push(b' ');
    }
    out.extend_from_slice(d);
    out.push(b'\n');
}

/// `basename "$(dirname "$SPEC_DIR/features/$f/tech.md")"`, on bytes. The
/// path is `.../features/<f>/tech.md`; `<f>` may hold slashes, `.` or `..`.
/// `f` is never empty. Only slashes leave `features` as the last component.
fn feature_name(f: &str) -> String {
    let trimmed = f.trim_end_matches('/');
    if trimmed.is_empty() {
        return "features".to_string();
    }
    trimmed.rsplit('/').next().unwrap_or(trimmed).to_string()
}

pub(crate) fn data(
    files: &crate::documents::Files,
    feature: Option<&str>,
) -> Result<serde_json::Value, crate::SpecError> {
    if feature.is_some_and(|f| !crate::query::valid_name(f)) {
        return Err(crate::SpecError::new(
            "invalid_feature",
            "expected lowercase-kebab feature name",
        ));
    }
    let mut blocks = Vec::new();
    let mut diagnostics = Vec::new();
    for (path, text) in files
        .iter()
        .filter(|(p, _)| p.starts_with("spec/features/") && p.ends_with("/tech.md"))
    {
        let name = path.split('/').nth(2).unwrap_or_default();
        if feature.is_some_and(|f| f != name) {
            continue;
        }
        let mut open = None;
        for (index, line) in read_lines(text).into_iter().enumerate() {
            let n = index + 1;
            if line == b"<!-- merge -->" {
                if open.is_some() {
                    diagnostics.push(serde_json::json!({"code":"invalid_merge","path":path,"line":n,"message":"nested merge block"}));
                } else {
                    open = Some((n, Vec::new()));
                }
            } else if line == b"<!-- /merge -->" {
                if let Some((start, preview)) = open.take() {
                    blocks.push(serde_json::json!({"feature":name,"file":path,"start":start,"end":n,"preview":String::from_utf8_lossy(&preview)}));
                } else {
                    diagnostics.push(serde_json::json!({"code":"invalid_merge","path":path,"line":n,"message":"merge closer without opener"}));
                }
            } else if let Some((_, preview)) = open.as_mut()
                && preview.is_empty()
                && !line.is_empty()
            {
                *preview = line.get(..60).unwrap_or(line).to_vec();
            }
        }
        if open.is_some() {
            diagnostics.push(serde_json::json!({"code":"invalid_merge","path":path,"message":"unclosed merge block"}));
        }
    }
    Ok(serde_json::json!({"blocks":blocks,"diagnostics":diagnostics}))
}

pub(crate) fn render_managed(
    files: &crate::documents::Files,
    args: &[String],
) -> Result<(u8, Vec<u8>), crate::SpecError> {
    if crate::wants_help(args) {
        return Ok((0, USAGE.as_bytes().to_vec()));
    }
    let mut feature = None;
    let mut format = "table";
    let mut i = 0;
    while let Some(arg) = args.get(i) {
        if arg == "--format" {
            i += 1;
            format = args.get(i).ok_or_else(|| {
                crate::SpecError::new(
                    "invalid_request",
                    "scan-merges: --format requires table|json|plain",
                )
            })?;
        } else if let Some(value) = arg.strip_prefix("--format=") {
            format = value;
        } else if arg.starts_with('-') {
            return Err(crate::SpecError::new(
                "invalid_request",
                format!("Unknown option: {arg}"),
            ));
        } else {
            feature = Some(arg.as_str());
        }
        i += 1;
    }
    let result = data(files, feature)?;
    let blocks = result["blocks"]
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or_default();
    let code = if result["diagnostics"]
        .as_array()
        .is_some_and(|d| !d.is_empty())
    {
        1
    } else {
        0
    };
    if format == "json" {
        return Ok((code, format!("{}\n", result["blocks"]).into_bytes()));
    }
    if blocks.is_empty() {
        return Ok((code, b"No merge blocks found.\n".to_vec()));
    }
    let mut out = Vec::new();
    if format != "plain" {
        row(&mut out, b"feature", b"file", b"lines", b"preview");
    }
    for block in blocks {
        let feature = block["feature"].as_str().unwrap_or_default();
        let path = block["file"].as_str().unwrap_or_default();
        let preview = block["preview"].as_str().unwrap_or_default();
        let start = block["start"].as_u64().unwrap_or(0);
        let end = block["end"].as_u64().unwrap_or(0);
        if format == "plain" {
            out.extend(format!("{path}:{start}-{end}: {preview}\n").bytes());
        } else {
            row(
                &mut out,
                feature.as_bytes(),
                path.as_bytes(),
                format!("{start}-{end}").as_bytes(),
                preview.as_bytes(),
            );
        }
    }
    Ok((code, out))
}
