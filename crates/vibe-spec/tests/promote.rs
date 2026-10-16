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

#[test]
fn promote_matches_the_oracle_on_edge_inputs() {
    let dup = "<!-- merge -->\nsame\n<!-- /merge -->\n<!-- merge -->\nsame\n<!-- /merge -->\n";
    same("pr-dup", dup, "# Tech\n", &["auth"], true);
    same("pr-dup-dry", dup, "# Tech\n", &["auth", "--dry-run"], true);
    // bash's own `${1:?}` message names the script: code only.
    same("pr-empty-name", TECH, "x\n", &[""], false);
    same("pr-sub", TECH, "x\n", &["auth/sub"], true);
    same("pr-dotdot", TECH, "x\n", &["../features/auth"], true);
    same("pr-no-eol", TECH, "# Tech", &["auth"], true);
}

#[test]
fn a_second_promote_is_idempotent_like_the_oracle() {
    let a = tree("pr-twice-o", TECH, "# Tech\n");
    let b = tree("pr-twice-c", TECH, "# Tech\n");
    for round in 0..2 {
        let want = staged_oracle(&a, "promote.sh", &["auth"]);
        let got = spec(&b, &["promote", "auth"], &[]);
        assert_eq!(
            (got.code, &got.stdout),
            (want.code, &want.stdout),
            "round {round}"
        );
        assert_eq!(got.stderr, want.stderr, "round {round}");
        assert_eq!(
            fs::read(b.join(".spec/tech.md")).unwrap(),
            fs::read(a.join(".spec/tech.md")).unwrap(),
            "round {round}"
        );
    }
}

/// Deliberate divergence: the port discovers the root by walking up.
#[test]
fn promote_from_a_subdirectory_uses_the_discovered_root() {
    let p = tree("pr-sub-cwd", TECH, "# Tech\n");
    fs::create_dir_all(p.join("sub")).unwrap();
    let got = spec(&p.join("sub"), &["promote", "auth"], &[]);
    assert_eq!(got.code, 0, "{}", got.stderr);
    let text = fs::read_to_string(p.join(".spec/tech.md")).unwrap();
    assert!(text.contains("Tokens expire after 1 hour."), "{text}");
}

#[cfg(unix)]
fn is_root() -> bool {
    std::process::Command::new("id")
        .arg("-u")
        .output()
        .map(|o| o.stdout.starts_with(b"0"))
        .unwrap_or(false)
}

#[cfg(unix)]
fn chmod(path: &std::path::Path, mode: u32) {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(mode)).unwrap();
}

#[test]
fn an_unreadable_target_fails_without_writing() {
    if is_root() {
        return;
    }
    for dry in [false, true] {
        let p = tree(&format!("pr-unread-target-{dry}"), TECH, "# Tech\n");
        let target = p.join(".spec/tech.md");
        chmod(&target, 0o000);
        let args: Vec<&str> = if dry {
            vec!["promote", "auth", "--dry-run"]
        } else {
            vec!["promote", "auth"]
        };
        let got = spec(&p, &args, &[]);
        chmod(&target, 0o644);
        assert_eq!(got.code, 1, "dry={dry} {}", got.stderr);
        assert!(got.stdout.is_empty(), "dry={dry}");
        assert!(
            got.stderr
                .starts_with("promote: ERROR: cannot read .spec/tech.md: "),
            "dry={dry} {}",
            got.stderr
        );
        assert_eq!(fs::read(&target).unwrap(), b"# Tech\n", "dry={dry}");
    }
}

#[test]
fn an_unreadable_feature_tech_fails() {
    if is_root() {
        return;
    }
    let p = tree("pr-unread-tech", TECH, "# Tech\n");
    let tech = p.join(".spec/features/auth/tech.md");
    chmod(&tech, 0o000);
    let got = spec(&p, &["promote", "auth"], &[]);
    chmod(&tech, 0o644);
    assert_eq!(got.code, 1, "{}", got.stderr);
    assert!(got.stdout.is_empty());
    assert!(
        got.stderr
            .starts_with("promote: ERROR: cannot read .spec/features/auth/tech.md: "),
        "{}",
        got.stderr
    );
    assert_eq!(fs::read(p.join(".spec/tech.md")).unwrap(), b"# Tech\n");
}
