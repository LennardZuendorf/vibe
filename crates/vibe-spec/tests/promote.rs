#![cfg(unix)]
#![allow(clippy::unwrap_used, clippy::expect_used)]
//! `vibe-spec promote` (F18): the oracle's streams and files for merge
//! blocks, then requirement deltas (Task 6) and superpowers (Task 7).
mod common;

use std::fs;
use std::path::PathBuf;

use common::{project, spec, staged_oracle, write};

const TECH: &str = "---\ntype: feature-tech\n---\n\n<!-- merge -->\n## Auth\n\nSessions live in signed cookies.\n<!-- /merge -->\n\n<!-- merge -->\nTokens expire after 1 hour.\n<!-- /merge -->\n";
const PRESENT: &str =
    "# Tech\n\n## Auth\n\nSessions live in signed cookies.\n\nTokens expire after 1 hour.\n";

fn tree(name: &str, tech: &str, target: &str) -> PathBuf {
    let p = project(name);
    write(&p, ".spec/features/auth/tech.md", tech);
    write(&p, ".spec/tech.md", target);
    p
}

/// Twin trees: candidate and staged oracle give the same exit code, stdout,
/// stderr (when `stderr_too`) and resulting target bytes.
fn same(name: &str, tech: &str, target: &str, args: &[&str], stderr_too: bool) {
    let a = tree(&format!("{name}-o"), tech, target);
    let b = tree(&format!("{name}-c"), tech, target);
    let want = staged_oracle(&a, "promote.sh", args);
    let got = spec(&b, &[&["promote"], args].concat(), &[]);
    assert_eq!(
        (got.code, &got.stdout),
        (want.code, &want.stdout),
        "{name} {args:?}"
    );
    if stderr_too {
        assert_eq!(got.stderr, want.stderr, "{name} {args:?}");
    }
    assert_eq!(
        fs::read(b.join(".spec/tech.md")).unwrap(),
        fs::read(a.join(".spec/tech.md")).unwrap(),
        "{name} {args:?}"
    );
}

#[test]
fn promote_matches_the_oracle_on_every_stream() {
    let cases: Vec<(&str, &str, &str, Vec<&str>, bool)> = vec![
        ("pr-dry", TECH, "# Tech\n", vec!["auth", "--dry-run"], true),
        ("pr-real", TECH, "# Tech\n", vec!["auth"], true),
        ("pr-again", TECH, PRESENT, vec!["auth"], true),
        (
            "pr-again-dry",
            TECH,
            PRESENT,
            vec!["auth", "--dry-run"],
            true,
        ),
        ("pr-missing", TECH, "x\n", vec!["missing"], true),
        ("pr-flag-first", TECH, "x\n", vec!["--dry-run"], true),
        (
            "pr-target-missing",
            TECH,
            "x\n",
            vec!["auth", "--target", "nope.md"],
            true,
        ),
        (
            "pr-target",
            TECH,
            "x\n",
            vec!["auth", "--target", ".spec/tech.md", "--dry-run"],
            true,
        ),
        (
            "pr-nested",
            "<!-- merge -->\na\n<!-- merge -->\n",
            "x\n",
            vec!["auth"],
            true,
        ),
        (
            "pr-orphan",
            "<!-- /merge -->\n## bad\n<!-- merge -->\n",
            "x\n",
            vec!["auth"],
            true,
        ),
        (
            "pr-unclosed-eof",
            "<!-- merge -->\na\n<!-- /merge -->",
            "x\n",
            vec!["auth"],
            true,
        ),
        ("pr-none", "no blocks\n", "x\n", vec!["auth"], true),
        (
            "pr-empty-block",
            "<!-- merge -->\n<!-- /merge -->\n",
            "x\n",
            vec!["auth"],
            true,
        ),
        ("pr-unknown", TECH, "x\n", vec!["auth", "--bogus"], true),
        // bash's own `${2:?}` / `${1:?}` messages name the script: code only.
        (
            "pr-target-nopath",
            TECH,
            "x\n",
            vec!["auth", "--target"],
            false,
        ),
        ("pr-noargs", TECH, "x\n", vec![], false),
    ];
    for (name, tech, target, args, stderr_too) in cases {
        same(name, tech, target, &args, stderr_too);
    }
}
