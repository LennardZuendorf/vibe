#![cfg(unix)]
#![allow(clippy::unwrap_used, clippy::expect_used)]
//! `vibe-spec promote` (F18): the oracle's streams and files for merge
//! blocks, then requirement deltas (Task 6) and superpowers (Task 7).
mod common;

use std::fs;
use std::path::PathBuf;

use common::{assert_spec_json, git, layered_tree, project, spec, staged_oracle, write};

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

const TOPIC: &str = "# auth — Product\n\n## Requirements\n\n### Requirement: Sign in (R1)\nThe system SHALL sign users in.\n\n#### Scenario: ok\n- **WHEN** valid\n- **THEN** in\n\n### Requirement: Sign out (R2)\nThe system SHALL sign users out.\n\n#### Scenario: out\n- **WHEN** asked\n- **THEN** out\n";
const DELTA: &str = "---\ntype: feature-product\nfeature: auth\nparent: ../../product-auth.md\n---\n\n# Feature: Auth — Product\n\n## MODIFIED Requirements\n\n### Requirement: Sign in (R1)\nThe system SHALL sign users in with a passkey.\n\n#### Scenario: ok\n- **WHEN** valid\n- **THEN** in\n\n## REMOVED Requirements\n\n### Requirement: Sign out (R2)\n**Reason**: sessions expire.\n\n## ADDED Requirements\n\n### Requirement: Enrol (R3)\nThe system MUST enrol passkeys.\n\n#### Scenario: enrol\n- **WHEN** added\n- **THEN** stored\n";
const AFTER: &str = "# auth — Product\n\n## Requirements\n\n### Requirement: Sign in (R1)\nThe system SHALL sign users in with a passkey.\n\n#### Scenario: ok\n- **WHEN** valid\n- **THEN** in\n\n### Requirement: Enrol (R3)\nThe system MUST enrol passkeys.\n\n#### Scenario: enrol\n- **WHEN** added\n- **THEN** stored\n";

fn delta_tree(name: &str, product: &str) -> PathBuf {
    let p = tree(name, TECH, "# Tech\n");
    write(&p, ".spec/product-auth.md", TOPIC);
    write(&p, ".spec/features/auth/product.md", product);
    p
}

#[test]
fn deltas_apply_after_merge_blocks() {
    let p = delta_tree("pr-delta", DELTA);
    let out = spec(&p, &["promote", "auth"], &[]);
    assert_eq!(out.code, 0, "{}", out.stderr);
    assert_eq!(
        out.text(),
        "PROMOTED 2 block(s) from .spec/features/auth/tech.md → .spec/tech.md (0 already present)\n\
         APPLIED 3 requirement change(s) from .spec/features/auth/product.md → .spec/product-auth.md (0 already applied)\n"
    );
    assert_eq!(
        fs::read_to_string(p.join(".spec/product-auth.md")).unwrap(),
        AFTER
    );
    let again = spec(&p, &["promote", "auth"], &[]);
    assert_eq!(
        again.text(),
        "No new blocks to promote — all 2 already present in .spec/tech.md\n\
         No requirement changes to apply — all 3 already in .spec/product-auth.md\n"
    );
    assert_eq!(
        fs::read_to_string(p.join(".spec/product-auth.md")).unwrap(),
        AFTER
    );
}

#[test]
fn delta_dry_run_writes_nothing() {
    let p = delta_tree("pr-delta-dry", DELTA);
    let out = spec(&p, &["promote", "auth", "--dry-run"], &[]);
    assert_eq!(out.code, 0, "{}", out.stderr);
    assert_eq!(
        out.text(),
        "--- would promote 2 block(s) to .spec/tech.md (0 already present) ---\n\
         \n## Auth\n\nSessions live in signed cookies.\n\
         \nTokens expire after 1 hour.\n\
         --- would apply 3 requirement change(s) to .spec/product-auth.md (0 already applied) ---\n\
         REMOVED Sign out (R2)\nMODIFIED Sign in (R1)\nADDED Enrol (R3)\n"
    );
    assert_eq!(
        fs::read_to_string(p.join(".spec/product-auth.md")).unwrap(),
        TOPIC
    );
    assert_eq!(
        fs::read_to_string(p.join(".spec/tech.md")).unwrap(),
        "# Tech\n"
    );
}

#[test]
fn a_refused_delta_writes_nothing_at_all() {
    let p = delta_tree(
        "pr-delta-bad",
        &DELTA.replace(
            "Sign in (R1)\nThe system SHALL sign users in with",
            "Ghost\nThe system SHALL haunt",
        ),
    );
    let out = spec(&p, &["promote", "auth"], &[]);
    assert_eq!(out.code, 1);
    assert!(out.text().is_empty());
    assert_eq!(
        out.stderr,
        "ERROR: MODIFIED 'Ghost' is not in the target (.spec/features/auth/product.md)\n"
    );
    assert_eq!(
        fs::read_to_string(p.join(".spec/product-auth.md")).unwrap(),
        TOPIC
    );
    assert_eq!(
        fs::read_to_string(p.join(".spec/tech.md")).unwrap(),
        "# Tech\n"
    );
}

#[test]
fn a_parent_outside_the_root_is_refused() {
    let p = delta_tree(
        "pr-delta-out",
        &DELTA.replace("../../product-auth.md", "../../../outside.md"),
    );
    let out = spec(&p, &["promote", "auth"], &[]);
    assert_eq!(out.code, 1);
    assert_eq!(
        out.stderr,
        "ERROR: parent '../../../outside.md' leaves the spec root (.spec/features/auth/product.md)\n"
    );
}

#[test]
fn deltas_run_without_merge_blocks_and_share_a_target() {
    let p = delta_tree("pr-delta-only", DELTA);
    write(&p, ".spec/features/auth/tech.md", "no blocks\n");
    let out = spec(&p, &["promote", "auth"], &[]);
    assert_eq!(out.code, 0, "{}", out.stderr);
    assert_eq!(
        out.stderr,
        "WARN: no <!-- merge --> blocks found in .spec/features/auth/tech.md\n"
    );
    assert_eq!(
        fs::read_to_string(p.join(".spec/product-auth.md")).unwrap(),
        AFTER
    );
    let shared = delta_tree("pr-delta-shared", DELTA);
    let out = spec(
        &shared,
        &["promote", "auth", "--target", ".spec/product-auth.md"],
        &[],
    );
    assert_eq!(out.code, 0, "{}", out.stderr);
    let text = fs::read_to_string(shared.join(".spec/product-auth.md")).unwrap();
    assert_eq!(
        text,
        format!(
            "{AFTER}\n## Auth\n\nSessions live in signed cookies.\n\nTokens expire after 1 hour.\n"
        )
    );
    assert_eq!(
        out.text(),
        "PROMOTED 2 block(s) from .spec/features/auth/tech.md → .spec/product-auth.md (0 already present)\n\
         APPLIED 3 requirement change(s) from .spec/features/auth/product.md → .spec/product-auth.md (0 already applied)\n"
    );
}

#[test]
fn unreadable_delta_files_fail_without_writing() {
    if is_root() {
        return;
    }
    for (file, shown) in [
        (
            ".spec/features/auth/product.md",
            ".spec/features/auth/product.md",
        ),
        (".spec/product-auth.md", ".spec/product-auth.md"),
    ] {
        for dry in [false, true] {
            let p = delta_tree(&format!("pr-delta-unread-{dry}-{}", file.len()), DELTA);
            let path = p.join(file);
            chmod(&path, 0o000);
            let args: Vec<&str> = if dry {
                vec!["promote", "auth", "--dry-run"]
            } else {
                vec!["promote", "auth"]
            };
            let got = spec(&p, &args, &[]);
            chmod(&path, 0o644);
            assert_eq!(got.code, 1, "{file} dry={dry} {}", got.stderr);
            assert!(got.stdout.is_empty(), "{file} dry={dry}");
            assert!(
                got.stderr
                    .starts_with(&format!("promote: ERROR: cannot read {shown}: ")),
                "{file} dry={dry} {}",
                got.stderr
            );
            assert_eq!(
                fs::read_to_string(p.join(".spec/tech.md")).unwrap(),
                "# Tech\n"
            );
        }
    }
}

#[test]
fn a_failed_delta_write_restores_the_merge_target() {
    if is_root() {
        return;
    }
    let p = delta_tree(
        "pr-delta-restore",
        &DELTA.replace("../../product-auth.md", "../../sub/topic.md"),
    );
    write(&p, ".spec/sub/topic.md", TOPIC);
    chmod(&p.join(".spec/sub"), 0o555);
    let got = spec(&p, &["promote", "auth"], &[]);
    chmod(&p.join(".spec/sub"), 0o755);
    assert_eq!(got.code, 1, "{}", got.stderr);
    assert!(got.stdout.is_empty());
    assert!(
        got.stderr
            .starts_with("promote: ERROR: .spec/sub/topic.md: "),
        "{}",
        got.stderr
    );
    assert!(
        got.stderr.ends_with(" — restored .spec/tech.md\n"),
        "{}",
        got.stderr
    );
    assert_eq!(
        fs::read_to_string(p.join(".spec/tech.md")).unwrap(),
        "# Tech\n"
    );
    assert_eq!(
        fs::read_to_string(p.join(".spec/sub/topic.md")).unwrap(),
        TOPIC
    );
}

#[test]
fn dry_run_refuses_a_merge_induced_delta_conflict_like_the_real_run() {
    let block = "<!-- merge -->\n### Requirement: Sign in (R1)\nThe system SHALL sign users in quickly.\n<!-- /merge -->\n";
    let mut seen = Vec::new();
    for dry in [false, true] {
        let p = delta_tree(&format!("pr-delta-conflict-{dry}"), DELTA);
        write(&p, ".spec/features/auth/tech.md", block);
        let mut args = vec!["promote", "auth", "--target", ".spec/product-auth.md"];
        if dry {
            args.push("--dry-run");
        }
        let got = spec(&p, &args, &[]);
        assert_eq!(got.code, 1, "dry={dry} {}", got.stderr);
        assert!(got.stdout.is_empty(), "dry={dry}");
        assert_eq!(
            fs::read_to_string(p.join(".spec/product-auth.md")).unwrap(),
            TOPIC
        );
        seen.push((got.code, got.stderr));
    }
    assert_eq!(seen[0], seen[1]);
}

#[test]
fn a_missing_delta_target_is_reported_with_the_promote_prefix() {
    let p = delta_tree(
        "pr-delta-missing",
        &DELTA.replace("../../product-auth.md", "../../nope.md"),
    );
    let got = spec(&p, &["promote", "auth"], &[]);
    assert_eq!(got.code, 1);
    assert_eq!(got.stderr, "promote: ERROR: .spec/nope.md not found\n");
}

const STAMP: &str = "2026-01-01T00:00:00Z";

/// A committed repo on branch `feat` off `main`, with two artifacts bound
/// to `auth` and one that is not (`auth-v2`).
fn sp_repo(name: &str) -> PathBuf {
    let p = project(name);
    write(&p, ".spec/plan.md", "# Plan\n");
    write(
        &p,
        "docs/superpowers/specs/2026-01-02-auth-design.md",
        "# Auth design\n",
    );
    write(&p, "docs/superpowers/plans/2026-01-14-auth.md", "x\n");
    write(&p, "docs/superpowers/plans/2026-01-14-auth-v2.md", "x\n");
    git(&p, STAMP, &["init", "-q", "-b", "main"]);
    git(&p, STAMP, &["add", "-A"]);
    git(&p, STAMP, &["commit", "-q", "-m", "base"]);
    git(&p, STAMP, &["checkout", "-q", "-b", "feat"]);
    p
}

fn bound_exist(p: &std::path::Path) -> (bool, bool) {
    (
        p.join("docs/superpowers/specs/2026-01-02-auth-design.md")
            .is_file(),
        p.join("docs/superpowers/plans/2026-01-14-auth.md")
            .is_file(),
    )
}

#[test]
fn checklist_names_targets_per_layout() {
    let p = sp_repo("pr-sp-flat");
    let out = spec(&p, &["promote", "auth"], &[]);
    assert_eq!(out.code, 0, "{}", out.stderr);
    assert_eq!(
        out.text(),
        "superpowers artifacts for 'auth':\n\
         \x20 docs/superpowers/plans/2026-01-14-auth.md\n\
         \x20 docs/superpowers/specs/2026-01-02-auth-design.md\n\
         Promote durable facts, then run: vibe-spec promote auth --done\n\
         \x20 [ ] chosen approach \u{2192} .spec/plan.md\n\
         \x20 [ ] new nouns \u{2192} .spec/product.md\n\
         \x20 [ ] built places (+code:) \u{2192} .spec/tech.md\n\
         \x20 [ ] rules \u{2192} .spec/tech.md\n\
         \x20 [ ] failures \u{2192} .spec/lessons.md\n"
    );
    let l = layered_tree("pr-sp-layered");
    write(&l, "docs/superpowers/plans/2026-01-14-auth.md", "x\n");
    let out = spec(&l, &["promote", "auth"], &[]);
    assert_eq!(
        out.text(),
        "superpowers artifacts for 'auth':\n\
         \x20 docs/superpowers/plans/2026-01-14-auth.md\n\
         Promote durable facts, then run: vibe-spec promote auth --done\n\
         \x20 [ ] chosen approach \u{2192} docs/spec/decisions/\n\
         \x20 [ ] new nouns \u{2192} docs/spec/concepts/\n\
         \x20 [ ] built places (+code:) \u{2192} docs/spec/areas/\n\
         \x20 [ ] rules \u{2192} docs/spec/tech/\n\
         \x20 [ ] failures \u{2192} docs/spec/lessons.md\n"
    );
}

#[test]
fn done_refuses_when_the_spec_root_is_unchanged() {
    let p = sp_repo("pr-done-same");
    let out = spec(&p, &["promote", "auth", "--done"], &[]);
    assert_eq!(out.code, 1);
    assert!(out.text().is_empty());
    assert!(
        out.stderr
            .starts_with("promote: refused \u{2014} .spec/ is unchanged since "),
        "{}",
        out.stderr
    );
    assert_eq!(bound_exist(&p), (true, true));
}

#[test]
fn done_deletes_after_promotion() {
    let p = sp_repo("pr-done-ok");
    write(&p, ".spec/plan.md", "# Plan\n\nD47 promoted.\n");
    let dry = spec(&p, &["promote", "auth", "--done", "--dry-run"], &[]);
    assert_eq!(
        dry.text(),
        "WOULD DELETE docs/superpowers/plans/2026-01-14-auth.md\nWOULD DELETE docs/superpowers/specs/2026-01-02-auth-design.md\n"
    );
    assert_eq!(bound_exist(&p), (true, true));
    let out = spec(&p, &["promote", "auth", "--done"], &[]);
    assert_eq!(out.code, 0, "{}", out.stderr);
    assert_eq!(
        out.text(),
        "DELETED docs/superpowers/plans/2026-01-14-auth.md\nDELETED docs/superpowers/specs/2026-01-02-auth-design.md\n"
    );
    assert_eq!(bound_exist(&p), (false, false));
    assert!(
        p.join("docs/superpowers/plans/2026-01-14-auth-v2.md")
            .is_file()
    );
}

#[test]
fn done_counts_untracked_files() {
    let p = sp_repo("pr-done-new");
    write(&p, ".spec/decisions.md", "# new\n");
    assert_eq!(spec(&p, &["promote", "auth", "--done"], &[]).code, 0);
    assert_eq!(bound_exist(&p), (false, false));
}

#[test]
fn done_refuses_without_a_base() {
    let bare = project("pr-done-nogit");
    write(&bare, ".spec/plan.md", "x\n");
    write(&bare, "docs/superpowers/plans/2026-01-14-auth.md", "x\n");
    let out = spec(&bare, &["promote", "auth", "--done"], &[]);
    assert_eq!(out.code, 1);
    assert_eq!(
        out.stderr,
        "promote: refused \u{2014} no branch base found (pass --base <ref>); nothing deleted\n"
    );
    assert!(
        bare.join("docs/superpowers/plans/2026-01-14-auth.md")
            .is_file()
    );
    let p = sp_repo("pr-done-badbase");
    write(&p, ".spec/plan.md", "changed\n");
    let bad = spec(&p, &["promote", "auth", "--done", "--base", "nope"], &[]);
    assert_eq!(bad.code, 1);
    assert_eq!(
        bad.stderr,
        "promote: refused \u{2014} no branch base found (pass --base <ref>); nothing deleted\n"
    );
    assert_eq!(bound_exist(&p), (true, true));
}

#[test]
fn done_rejects_unsafe_names() {
    let p = sp_repo("pr-done-name");
    write(&p, ".spec/plan.md", "changed\n");
    let out = spec(&p, &["promote", "../auth", "--done"], &[]);
    assert_eq!(out.code, 1);
    assert_eq!(
        out.stderr,
        "promote: refused \u{2014} invalid feature name \"../auth\" (lowercase-kebab); nothing deleted\n"
    );
    assert_eq!(bound_exist(&p), (true, true));
    let none = spec(&p, &["promote", "billing", "--done"], &[]);
    assert_eq!(
        (none.code, none.stderr.as_str()),
        (1, "promote: no superpowers artifacts for 'billing'\n")
    );
}

#[test]
fn json_validates() {
    let p = sp_repo("pr-json");
    let check = spec(&p, &["promote", "auth", "--json"], &[]);
    assert_eq!(check.code, 0, "{}", check.stderr);
    assert_spec_json("promote", &check.text());
    write(&p, ".spec/plan.md", "changed\n");
    let done = spec(&p, &["promote", "auth", "--done", "--json"], &[]);
    assert_eq!(done.code, 0, "{}", done.stderr);
    assert_spec_json("promote", &done.text());
    let v: serde_json::Value = serde_json::from_slice(&done.stdout).unwrap();
    assert_eq!(v["deleted"].as_array().unwrap().len(), 2);
    let d = delta_tree("pr-json-delta", DELTA);
    let both = spec(&d, &["promote", "auth", "--json", "--dry-run"], &[]);
    assert_spec_json("promote", &both.text());
    let v: serde_json::Value = serde_json::from_slice(&both.stdout).unwrap();
    assert_eq!(
        v["delta"]["changes"][0],
        serde_json::json!({ "op": "REMOVED", "name": "Sign out (R2)", "applied": true })
    );
    assert_eq!(v["merge"]["appended"], 2);
}

#[test]
fn done_reports_every_path_it_could_not_delete() {
    let p = sp_repo("pr-done-fail");
    write(&p, ".spec/plan.md", "changed\n");
    let plans = p.join("docs/superpowers/plans");
    let specs = p.join("docs/superpowers/specs");
    if is_root() {
        return;
    }
    chmod(&plans, 0o555);
    let out = spec(&p, &["promote", "auth", "--done"], &[]);
    chmod(&plans, 0o755);
    assert_eq!(out.code, 1);
    assert!(
        out.stderr
            .contains("docs/superpowers/plans/2026-01-14-auth.md"),
        "{}",
        out.stderr
    );
    assert!(out.stderr.contains("not deleted"), "{}", out.stderr);
    assert!(plans.join("2026-01-14-auth.md").is_file());
    assert!(!specs.join("2026-01-02-auth-design.md").exists());
    // JSON mode: the same failure prints the actual deleted list.
    write(
        &p,
        "docs/superpowers/specs/2026-01-02-auth-design.md",
        "# Auth design\n",
    );
    chmod(&plans, 0o555);
    let j = spec(&p, &["promote", "auth", "--done", "--json"], &[]);
    chmod(&plans, 0o755);
    assert_eq!(j.code, 1);
    assert!(j.stderr.contains("not deleted"), "{}", j.stderr);
    assert_spec_json("promote", &j.text());
    let v: serde_json::Value = serde_json::from_slice(&j.stdout).unwrap();
    assert_eq!(
        v["deleted"],
        serde_json::json!(["docs/superpowers/specs/2026-01-02-auth-design.md"])
    );
    assert_eq!(v["artifacts"].as_array().unwrap().len(), 2);
}

#[test]
fn an_unreadable_artifact_dir_fails_closed() {
    if is_root() {
        return;
    }
    let p = sp_repo("pr-sp-unread");
    write(&p, ".spec/plan.md", "changed\n");
    let plans = p.join("docs/superpowers/plans");
    chmod(&plans, 0o000);
    let out = spec(&p, &["promote", "auth", "--done"], &[]);
    let chk = spec(&p, &["promote", "auth"], &[]);
    chmod(&plans, 0o755);
    assert_eq!((out.code, chk.code), (1, 1));
    assert!(out.stderr.contains("cannot read"), "{}", out.stderr);
    assert!(out.text().is_empty());
    assert_eq!(bound_exist(&p), (true, true));
}

#[test]
fn a_symlinked_artifact_dir_is_refused_everywhere() {
    let p = sp_repo("pr-sp-symlink");
    write(&p, ".spec/plan.md", "changed\n");
    let outside = project("pr-sp-outside");
    write(&outside, "2026-01-14-auth.md", "keep\n");
    let plans = p.join("docs/superpowers/plans");
    fs::remove_dir_all(&plans).unwrap();
    std::os::unix::fs::symlink(&outside, &plans).unwrap();
    for args in [&["promote", "auth", "--done"][..], &["promote", "auth"][..]] {
        let out = spec(&p, args, &[]);
        assert_eq!(out.code, 1, "{args:?}: {}", out.stderr);
        assert!(out.text().is_empty());
        assert!(
            out.stderr
                .contains("docs/superpowers/plans is a symlink \u{2014} refusing to follow it"),
            "{}",
            out.stderr
        );
        assert!(outside.join("2026-01-14-auth.md").is_file());
    }
}

#[test]
fn json_without_anything_to_report_still_prints_one_object() {
    let p = project("pr-json-empty");
    write(&p, ".spec/plan.md", "# Plan\n");
    write(&p, ".spec/features/auth/tech.md", "# Tech\n");
    write(&p, ".spec/tech.md", "# Tech\n");
    let out = spec(&p, &["promote", "auth", "--json"], &[]);
    assert_eq!(out.code, 0, "{}", out.stderr);
    assert_spec_json("promote", &out.text());
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["artifacts"], serde_json::json!([]));
    assert_eq!(v["deleted"], serde_json::json!([]));
}

#[test]
fn json_without_a_root_exits_3_with_empty_stdout() {
    let bare = project("pr-json-noroot");
    for args in [
        &["promote", "auth", "--json"][..],
        &["promote", "auth", "--done", "--json"][..],
    ] {
        let out = spec(&bare, args, &[]);
        assert_eq!(out.code, 3, "{args:?}: {}", out.stderr);
        assert!(out.stdout.is_empty());
    }
    assert_eq!(spec(&bare, &["promote", "auth"], &[]).code, 1);
}

#[test]
fn a_leading_dash_name_is_refused_and_other_slugs_do_not_bind() {
    let p = sp_repo("pr-done-slug");
    write(&p, ".spec/plan.md", "changed\n");
    let out = spec(&p, &["promote", "-auth", "--done"], &[]);
    assert_eq!(out.code, 1);
    assert!(
        out.stderr.contains("invalid feature name"),
        "{}",
        out.stderr
    );
    write(
        &p,
        "docs/superpowers/specs/2026-01-14-auth-v2-design.md",
        "x\n",
    );
    let out = spec(&p, &["promote", "auth", "--done"], &[]);
    assert_eq!(out.code, 0, "{}", out.stderr);
    assert!(
        p.join("docs/superpowers/specs/2026-01-14-auth-v2-design.md")
            .is_file()
    );
    assert!(
        p.join("docs/superpowers/plans/2026-01-14-auth-v2.md")
            .is_file()
    );
}
