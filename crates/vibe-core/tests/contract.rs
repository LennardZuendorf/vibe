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

/// Spec-path and file words that name a tool's vocabulary. State and channel
/// names are derived from their source files, so they cannot drift from the list.
const STATIC_VOCABULARY: [&str; 8] = [
    ".spec",
    "docs/spec",
    "docs/superpowers",
    "state-machine",
    "state.json",
    "policy.json",
    "AGENTS.md",
    "CLAUDE.md",
];

fn object_keys(file: &str, field: &str) -> Vec<String> {
    let text = fs::read_to_string(repo().join(file)).unwrap();
    let json: Value = serde_json::from_str(&text).unwrap();
    json[field].as_object().unwrap().keys().cloned().collect()
}

/// Every dotted flow state name and its flow prefix (`quick.verify`, `quick.`),
/// plus every injection channel name, plus the static words.
fn tool_vocabulary() -> Vec<String> {
    let mut words: Vec<String> = STATIC_VOCABULARY.iter().map(|w| (*w).to_owned()).collect();
    let states: Vec<String> = object_keys("flow/state-machine.json", "states")
        .into_iter()
        .filter(|s| s.contains('.'))
        .collect();
    assert!(states.len() >= 12, "derived only {} states", states.len());
    for state in states {
        if let Some((flow, _)) = state.split_once('.') {
            words.push(format!("{flow}."));
        }
        words.push(state);
    }
    let channels = object_keys("flow/content/vibe.default.json", "channels");
    assert!(
        channels.len() >= 5,
        "derived only {} channels",
        channels.len()
    );
    words.extend(channels);
    words
}

#[test]
fn core_sources_hold_no_tool_vocabulary() {
    let vocabulary = tool_vocabulary();
    let mut files = Vec::new();
    rust_files(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("src"),
        &mut files,
    );
    assert!(files.len() >= 3, "scanned only {} files", files.len());
    for file in &files {
        let text = fs::read_to_string(file).unwrap();
        for word in &vocabulary {
            assert!(
                !text.contains(word.as_str()),
                "{} spells tool vocabulary {word:?}",
                file.display()
            );
        }
    }
}
