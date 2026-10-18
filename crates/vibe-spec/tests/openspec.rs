#![cfg(unix)]
#![allow(clippy::unwrap_used, clippy::expect_used)]
//! `vibe-spec export|import --openspec` (F18, P3c-R14–R16).
mod common;

use std::fs;
use std::path::{Path, PathBuf};

use common::{assert_spec_json, project, repo, spec, write};

/// Copies `tests/openspec/fixture/` into a fresh project.
fn fixture(name: &str) -> PathBuf {
    fn copy(from: &Path, to: &Path) {
        fs::create_dir_all(to).unwrap();
        for e in fs::read_dir(from).unwrap() {
            let e = e.unwrap();
            if e.file_type().unwrap().is_dir() {
                copy(&e.path(), &to.join(e.file_name()));
            } else {
                fs::copy(e.path(), to.join(e.file_name())).unwrap();
            }
        }
    }
    let p = project(name);
    copy(&repo().join("tests/openspec/fixture"), &p);
    p
}

fn read(p: &Path, rel: &str) -> String {
    fs::read_to_string(p.join(rel)).unwrap()
}

const SPEC: &str = "# product-auth Specification\n\n## Purpose\nRequirements of .spec/product-auth.md in the vibe spec, exported by vibe-spec.\n\n## Requirements\n\n### Requirement: Sign in (R1)\n\nThe system SHALL issue a session on valid credentials.\n\n#### Scenario: Valid credentials\n\n- **WHEN** a user submits valid credentials\n- **THEN** a session is issued\n\n### Requirement: Sign out (R2)\n\nThe system SHALL end the session on request.\n\n#### Scenario: Sign out\n\n- **WHEN** the user signs out\n- **THEN** the session ends\n";
const DELTA: &str = "## ADDED Requirements\n\n### Requirement: Passkey enrolment (R3)\n\nThe system MUST let a signed-in user enrol a passkey.\n\n#### Scenario: Enrol\n\n- **WHEN** a signed-in user adds a passkey\n- **THEN** the passkey is stored\n\n## MODIFIED Requirements\n\n### Requirement: Sign in (R1)\n\nThe system SHALL issue a session on a valid passkey.\n\n#### Scenario: Valid credentials\n\n- **WHEN** a user presents a valid passkey\n- **THEN** a session is issued\n\n## REMOVED Requirements\n\n### Requirement: Sign out (R2)\n**Reason**: sessions expire on their own.\n";

#[test]
fn export_writes_the_openspec_layout() {
    let p = fixture("os-export");
    let out = spec(&p, &["export", "--openspec", "out"], &[]);
    assert_eq!(out.code, 0, "{}", out.stderr);
    assert_eq!(
        out.text(),
        "EXPORTED 8 file(s) to out/openspec/\n\
         \x20 openspec/specs/product-auth/spec.md\n\
         \x20 openspec/changes/notes/.openspec.yaml\n\
         \x20 openspec/changes/notes/proposal.md\n\
         \x20 openspec/changes/notes/design.md\n\
         \x20 openspec/changes/passkeys/specs/product-auth/spec.md\n\
         \x20 openspec/changes/passkeys/proposal.md\n\
         \x20 openspec/changes/passkeys/tasks.md\n\
         \x20 openspec/changes/passkeys/design.md\n"
    );
    assert_eq!(read(&p, "out/openspec/specs/product-auth/spec.md"), SPEC);
    assert_eq!(
        read(
            &p,
            "out/openspec/changes/passkeys/specs/product-auth/spec.md"
        ),
        DELTA
    );
    assert_eq!(
        read(&p, "out/openspec/changes/passkeys/tasks.md"),
        "# Tasks\n\n## 1. passkeys\n\n- [x] 1.1 WebAuthn ceremony\n- [ ] 1.2 Enrolment screen\n"
    );
    assert_eq!(
        read(&p, "out/openspec/changes/passkeys/proposal.md"),
        "# Proposal: passkeys\n\n## Why\nPasskeys replace passwords for sign-in.\n\n## What Changes\n- Exported from `.spec/features/passkeys/` by vibe-spec.\n"
    );
    let design = read(&p, "out/openspec/changes/passkeys/design.md");
    assert!(design.contains("<!-- vibe:export v1 -->\n"), "{design}");
    let tech = read(&p, ".spec/features/passkeys/tech.md");
    assert!(
        design.contains(&format!(
            "\n## vibe:tech.md {}\n\n````text\n{tech}````\n",
            tech.len()
        )),
        "{design}"
    );
    let plan = read(&p, ".spec/features/passkeys/plan.md");
    assert!(
        design.contains(&format!(
            "\n## vibe:plan.md {}\n\n```text\n{plan}\n```\n",
            plan.len()
        )),
        "{design}"
    );
    assert!(!p.join("out/openspec/project.md").exists());
    assert_eq!(
        read(&p, "out/openspec/changes/notes/.openspec.yaml"),
        "schema: spec-driven\nskip_specs: true\n"
    );
}

#[test]
fn export_json_validates_and_refuses_an_existing_dir() {
    let p = fixture("os-export-json");
    let out = spec(&p, &["export", "--openspec", "out", "--json"], &[]);
    assert_eq!(out.code, 0, "{}", out.stderr);
    assert_spec_json("openspec", &out.text());
    let again = spec(&p, &["export", "--openspec", "out"], &[]);
    assert_eq!(again.code, 1);
    assert_eq!(again.stderr, "export: out/openspec already exists\n");
}

#[test]
fn export_refuses_a_modified_that_drops_a_scenario() {
    let p = fixture("os-export-drop");
    let product = read(&p, ".spec/features/passkeys/product.md")
        .replace("#### Scenario: Valid credentials", "#### Scenario: Passkey");
    write(&p, ".spec/features/passkeys/product.md", &product);
    let out = spec(&p, &["export", "--openspec", "out"], &[]);
    assert_eq!(out.code, 1);
    assert_eq!(
        out.stderr,
        "export: .spec/features/passkeys/product.md: MODIFIED 'Sign in (R1)' drops scenario(s) the target still has: Valid credentials\n"
    );
    assert!(!p.join("out").exists());
}

#[test]
fn export_refuses_duplicate_names() {
    let p = fixture("os-export-dup");
    let topic = read(&p, ".spec/product-auth.md").replace("Sign out (R2)", "Sign in (R1)");
    write(&p, ".spec/product-auth.md", &topic);
    let out = spec(&p, &["export", "--openspec", "out"], &[]);
    assert_eq!(out.code, 1);
    assert!(
        out.stderr
            .contains("export: .spec/product-auth.md: duplicate requirement 'Sign in (R1)'\n"),
        "{}",
        out.stderr
    );
    assert!(!p.join("out").exists());
}

#[test]
fn export_refuses_renamed_and_notes_stray_files() {
    let p = fixture("os-export-renamed");
    write(&p, ".spec/features/notes/extra.txt", "x\n");
    let out = spec(&p, &["export", "--openspec", "out"], &[]);
    assert_eq!(out.code, 0, "{}", out.stderr);
    assert_eq!(
        out.stderr,
        "export: .spec/features/notes/extra.txt: not exported\n"
    );
    fs::remove_dir_all(p.join("out")).unwrap();
    let product = format!(
        "{}\n## RENAMED Requirements\n\n### Requirement: A\n",
        read(&p, ".spec/features/notes/product.md")
    );
    write(&p, ".spec/features/notes/product.md", &product);
    let out = spec(&p, &["export", "--openspec", "out"], &[]);
    assert_eq!(out.code, 1);
    assert!(
        out.stderr.contains("RENAMED Requirements is not supported"),
        "{}",
        out.stderr
    );
    assert!(!p.join("out").exists());
}

#[test]
fn export_without_a_root_exits_3_with_empty_stdout() {
    let p = project("os-export-noroot");
    let out = spec(&p, &["export", "--openspec", "out", "--json"], &[]);
    assert_eq!(out.code, 3);
    assert_eq!(out.text(), "");
    assert!(!p.join("out").exists());
}

#[test]
fn export_refuses_a_symlinked_destination_and_writes_nothing_outside() {
    use std::os::unix::fs::symlink;
    let p = fixture("os-export-link");
    let outside = project("os-export-outside");
    symlink(&outside, p.join("out")).unwrap();
    let out = spec(&p, &["export", "--openspec", "out"], &[]);
    assert_eq!(out.code, 1);
    assert_eq!(out.stderr, "export: out is a symlink\n");
    assert!(!outside.join("openspec").exists());
    // A dangling `openspec` symlink inside a real dir counts as existing.
    fs::remove_file(p.join("out")).unwrap();
    fs::create_dir(p.join("out")).unwrap();
    symlink(outside.join("nowhere"), p.join("out/openspec")).unwrap();
    let out = spec(&p, &["export", "--openspec", "out"], &[]);
    assert_eq!(out.code, 1);
    assert_eq!(out.stderr, "export: out/openspec already exists\n");
    assert!(!outside.join("nowhere").exists());
}

#[test]
fn export_fails_closed_on_an_unreadable_file() {
    use std::os::unix::fs::PermissionsExt;
    let p = fixture("os-export-unreadable");
    let tech = p.join(".spec/features/passkeys/tech.md");
    fs::set_permissions(&tech, fs::Permissions::from_mode(0o000)).unwrap();
    if fs::read(&tech).is_ok() {
        return; // running as root: permissions do not bind
    }
    let out = spec(&p, &["export", "--openspec", "out"], &[]);
    fs::set_permissions(&tech, fs::Permissions::from_mode(0o644)).unwrap();
    assert_eq!(out.code, 1);
    assert!(
        out.stderr.starts_with("export: ") && out.stderr.contains("tech.md"),
        "{}",
        out.stderr
    );
    assert!(!p.join("out").exists());
}

const BLOCK: &str = "## Requirements\n\n### Requirement: A\n\nThe system SHALL a.\n\n#### Scenario: a\n\n- **WHEN** a\n- **THEN** b\n";

/// `x.md` exports to `specs/x/spec.md`; `x/spec.md.md` exports to
/// `specs/x/spec.md/spec.md`, whose parent is the first file. The second
/// write fails after `openspec/` exists, on every platform.
fn mid_write_failure(name: &str) -> PathBuf {
    let p = fixture(name);
    write(&p, ".spec/x.md", BLOCK);
    write(&p, ".spec/x/spec.md.md", BLOCK);
    p
}

#[test]
fn export_rollback_removes_every_directory_it_created() {
    let p = mid_write_failure("os-export-rollback-a");
    write(&p, "keep/user.txt", "mine\n");
    let out = spec(&p, &["export", "--openspec", "keep/a/b/c"], &[]);
    assert_eq!(out.code, 1, "{}", out.stderr);
    assert_eq!(out.text(), "");
    assert_eq!(read(&p, "keep/user.txt"), "mine\n");
    assert!(
        !p.join("keep/a").exists(),
        "left {:?}",
        fs::read_dir(p.join("keep")).unwrap().count()
    );
    assert_eq!(fs::read_dir(p.join("keep")).unwrap().count(), 1);
}

#[test]
fn export_rollback_keeps_a_preexisting_destination() {
    let p = mid_write_failure("os-export-rollback-b");
    write(&p, "keep/user.txt", "mine\n");
    let out = spec(&p, &["export", "--openspec", "keep"], &[]);
    assert_eq!(out.code, 1, "{}", out.stderr);
    assert_eq!(read(&p, "keep/user.txt"), "mine\n");
    assert!(!p.join("keep/openspec").exists());
    assert_eq!(fs::read_dir(p.join("keep")).unwrap().count(), 1);
}

#[test]
fn export_keeps_x_md_and_x_md_md_distinct() {
    let p = fixture("os-export-md-md");
    write(&p, ".spec/x.md", BLOCK);
    write(&p, ".spec/y.md.md", BLOCK);
    let out = spec(&p, &["export", "--openspec", "out"], &[]);
    assert_eq!(out.code, 0, "{}", out.stderr);
    assert!(p.join("out/openspec/specs/y.md/spec.md").is_file());
    assert!(p.join("out/openspec/specs/x/spec.md").is_file());
    assert!(!p.join("out/openspec/specs/y/spec.md").exists());
    assert!(read(&p, "out/openspec/specs/y.md/spec.md").contains("# y.md Specification"));
}

#[test]
fn export_refuses_a_destination_inside_the_spec_root() {
    for dir in [".spec", ".spec/out", "./.spec/features/../out"] {
        let p = fixture("os-export-inside");
        let out = spec(&p, &["export", "--openspec", dir], &[]);
        assert_eq!(out.code, 1, "{dir}: {}", out.stderr);
        assert_eq!(
            out.stderr,
            format!("export: ERROR: {dir} is inside the spec root — refusing to export into it\n")
        );
        assert!(!p.join(".spec/openspec").exists());
        assert!(!p.join(".spec/out").exists());
    }
}
