#![allow(clippy::unwrap_used, clippy::expect_used)]
//! "Flow never parses spec markdown" is load-bearing (issue #28, B2).
//!
//! The flow half reads the spec only through `vibe-spec`'s CLI. If the crate
//! ever gained a dependency on spec's document model — the `vibe-spec` crate, a
//! markdown parser, or a module import of one — that comment would become a
//! lie the compiler happily keeps. This test states the mechanical property
//! instead: the crate's declared dependencies are a closed, named set, and no
//! source file imports a spec module.
//!
//! Scanning the manifest rather than one import site is deliberate: a new
//! dependency is reachable from any file, so the check has to be closed over
//! the crate, not over the spelling of today's imports.

use std::fs;
use std::path::{Path, PathBuf};

fn crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Every name declared under a dependency table of the crate's manifest.
fn declared_dependencies() -> Vec<String> {
    let manifest = fs::read_to_string(crate_dir().join("Cargo.toml")).unwrap();
    let mut deps = Vec::new();
    let mut in_table = false;
    for line in manifest.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            in_table = trimmed.contains("dependencies");
            continue;
        }
        if !in_table || trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if let Some((name, _)) = trimmed.split_once('=') {
            deps.push(name.trim().to_string());
        }
    }
    deps.sort();
    deps
}

/// Every `.rs` file under `src/`, recursively.
fn source_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            source_files(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
}

#[test]
fn the_crate_depends_on_nothing_spec_or_markdown_shaped() {
    let deps = declared_dependencies();
    assert_eq!(
        deps,
        ["clap", "serde_json", "vibe-core"],
        "the flow crate's dependency set changed. If the new dependency is \
         intentional, this test is the place to state why it does not give flow \
         a spec document model; do not widen it silently."
    );
    for dep in &deps {
        let lower = dep.to_lowercase();
        assert!(
            !lower.contains("spec"),
            "{dep}: flow must not depend on spec's crate"
        );
        for parser in ["markdown", "pulldown", "comrak", "cmark"] {
            assert!(
                !lower.contains(parser),
                "{dep}: flow must not link a markdown parser"
            );
        }
    }
}

#[test]
fn no_source_file_imports_a_spec_module() {
    let mut files = Vec::new();
    source_files(&crate_dir().join("src"), &mut files);
    assert!(!files.is_empty(), "no sources found to scan");
    for file in files {
        let text = fs::read_to_string(&file).unwrap();
        for needle in ["use vibe_spec", "vibe_spec::", "extern crate vibe_spec"] {
            assert!(
                !text.contains(needle),
                "{} imports spec's document model ({needle}); flow reads the spec \
                 only through the vibe-spec CLI",
                file.display()
            );
        }
    }
}
