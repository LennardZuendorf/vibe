//! `vibe-spec list-specs` — port of `spec/scripts/list-specs.sh`
//! (byte-identical; arguments are ignored, as the oracle ignores them).
use std::path::Path;

use crate::root::{Miss, Root};
use crate::text::{lines, md_files, name_of, remove_first, sub_dirs};

/// Runs the command.
pub fn run(found: &Result<Root, Miss>) -> (u8, Vec<u8>) {
    let root = match found {
        Ok(root) => root,
        Err(Miss::NotFound) => {
            return (
                0,
                b"No .spec/ directory found in current project\n".to_vec(), // literal
            );
        }
        Err(miss) => {
            eprintln!("{}", miss.message());
            return (miss.code(), Vec::new());
        }
    };
    let specs = md_files(&root.dir);
    let feature_dirs = sub_dirs(&root.dir.join("features"));
    if specs.is_empty() && feature_dirs.is_empty() {
        return (
            0,
            format!("No spec files found in {}/\n", root.display()).into_bytes(),
        );
    }
    let mut out = Vec::new();
    for file in &specs {
        entry(&mut out, file, name_of(file).as_bytes());
    }
    if !feature_dirs.is_empty() {
        out.extend_from_slice(b"\n## Features\n");
        for dir in &feature_dirs {
            let feature = name_of(dir);
            out.extend(format!("\n### {feature}\n").bytes());
            let docs = md_files(dir);
            if docs.is_empty() {
                out.extend_from_slice(b"- (no docs)\n");
                continue;
            }
            for file in &docs {
                let display = format!("features/{feature}/{}", name_of(file));
                entry(&mut out, file, display.as_bytes());
            }
        }
    }
    (0, out)
}

/// `- <display> [<area>] (<type>: <scope>)`.
fn entry(out: &mut Vec<u8>, file: &Path, display: &[u8]) {
    let text = std::fs::read(file).unwrap_or_default();
    let ty = field(&text, b"type:", b"type: ");
    let scope = field(&text, b"scope:", b"scope: ");
    out.extend(cat!(
        "- ",
        display,
        " [",
        area(&name_of(file)),
        "] (",
        ty,
        ": ",
        scope,
        ")\n"
    ));
}

/// `grep -m1 "^<key>" | sed 's/<strip>//'`.
fn field(text: &[u8], key: &[u8], strip: &[u8]) -> Vec<u8> {
    lines(text)
        .into_iter()
        .find(|l| l.starts_with(key))
        .map(|l| remove_first(l, strip))
        .unwrap_or_default()
}

/// `area_from_name`.
fn area(name: &str) -> &'static str {
    match name {
        "design.md" => "design",
        "lessons.md" => "lessons",
        n if n == "product.md" || n.starts_with("product-") => "product",
        n if n == "tech.md" || n.starts_with("tech-") => "tech",
        n if n == "plan.md" || n.starts_with("plan-") => "plan",
        _ => "unknown",
    }
}
