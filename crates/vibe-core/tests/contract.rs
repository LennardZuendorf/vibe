#![allow(clippy::unwrap_used, clippy::expect_used)]
//! `vibe_core::exit` matches contracts/exit-codes.json, and core holds no tool vocabulary.
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;
use vibe_core::exit;

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
fn exit_codes_match_the_contract() {
    let text = fs::read_to_string(repo().join("contracts/exit-codes.json")).unwrap();
    let contract: Value = serde_json::from_str(&text).unwrap();
    let meaning = |ctx: &str, code: u8| {
        contract["contexts"][ctx][code.to_string()]
            .as_str()
            .map(str::to_owned)
    };
    assert!(meaning("cli", exit::OK).is_some());
    assert!(meaning("cli", exit::FINDINGS).is_some());
    assert!(meaning("cli", exit::USAGE).is_some());
    assert!(meaning("vibe-spec-query", exit::NO_ROOT).is_some());
    let codes = [exit::OK, exit::FINDINGS, exit::USAGE, exit::NO_ROOT];
    assert_eq!(codes, [0, 1, 2, 3]);
}

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

/// Words that name a tool's vocabulary. Core must take these as input, never spell them.
const TOOL_VOCABULARY: [&str; 12] = [
    ".spec",
    "docs/spec",
    "docs/superpowers",
    "state-machine",
    "state.json",
    "policy.json",
    "AGENTS.md",
    "CLAUDE.md",
    "feature.",
    "strategy.",
    "user-prompt",
    "session-start",
];

#[test]
fn core_sources_hold_no_tool_vocabulary() {
    let mut files = Vec::new();
    rust_files(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("src"),
        &mut files,
    );
    assert!(files.len() >= 3, "scanned only {} files", files.len());
    for file in &files {
        let text = fs::read_to_string(file).unwrap();
        for word in TOOL_VOCABULARY {
            assert!(
                !text.contains(word),
                "{} spells tool vocabulary {word:?}",
                file.display()
            );
        }
    }
}
