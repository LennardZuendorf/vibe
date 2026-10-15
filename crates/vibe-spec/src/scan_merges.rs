//! `vibe-spec scan-merges` — port of `spec/scripts/scan-merges.sh`.
//!
//! Byte-identical, including its `while read` loop, which drops a final
//! line without a newline, and its 60-byte preview cut. Deliberate
//! divergence (pinned): `--format json` escapes its strings.
use std::path::{Path, PathBuf};

use vibe_core::exit;

use crate::root::{Miss, Root};
use crate::text::{json_str, name_of, read_lines, sub_dirs};

struct Block {
    feature: String,
    file: String,
    start: usize,
    end: usize,
    preview: Vec<u8>,
}

/// Runs the command.
pub fn run(found: &Result<Root, Miss>, cwd: &Path, args: &[String]) -> (u8, Vec<u8>) {
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
                    eprintln!("scan-merges: --format requires table|json|plain");
                    return (exit::FINDINGS, Vec::new());
                }
            }
        } else if let Some(v) = a.strip_prefix("--format=") {
            format = v.to_string();
        } else if a.starts_with('-') {
            eprintln!("Unknown option: {a}");
            return (exit::FINDINGS, Vec::new());
        } else {
            feature = Some(a.to_string());
        }
        i += 1;
    }
    // The oracle's paths are relative to the project ("$SPEC_DIR" = .spec);
    // without a root it looks for .spec/ under the cwd, which cannot exist.
    let (dir, display): (PathBuf, String) = match found {
        Ok(root) => (root.dir.clone(), root.display()),
        Err(Miss::NotFound) => (cwd.join(".spec"), ".spec".to_string()),
        Err(miss) => {
            eprintln!("{}", miss.message());
            return (miss.code(), Vec::new());
        }
    };
    let targets: Vec<(String, PathBuf)> = match &feature {
        Some(f) => vec![(f.clone(), dir.join("features").join(f).join("tech.md"))],
        None => sub_dirs(&dir.join("features"))
            .into_iter()
            .map(|d| (name_of(&d), d.join("tech.md")))
            .filter(|(_, p)| p.is_file())
            .collect(),
    };
    if targets.is_empty() {
        eprintln!("No feature tech.md files found.");
        return (exit::OK, Vec::new());
    }
    let mut unclosed = false;
    let mut blocks: Vec<Block> = Vec::new();
    for (feat, path) in &targets {
        if !path.is_file() {
            continue;
        }
        let file = format!("{display}/features/{feat}/tech.md");
        let text = std::fs::read(path).unwrap_or_default();
        let mut open: Option<(usize, Vec<u8>)> = None;
        for (n, line) in read_lines(&text).into_iter().enumerate() {
            let lineno = n + 1;
            if line == b"<!-- merge -->" {
                if let Some((start, _)) = &open {
                    eprintln!(
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
                        eprintln!(
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
            eprintln!("ERROR: unclosed <!-- merge --> block starting at line {start} in {file}");
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
