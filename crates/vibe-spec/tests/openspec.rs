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

/// An empty project with a `.spec` root and no features.
fn empty(name: &str) -> PathBuf {
    let p = project(name);
    write(&p, ".spec/product.md", "# P\n");
    p
}

#[test]
fn round_trip_is_byte_identical() {
    let a = fixture("os-rt-a");
    assert_eq!(spec(&a, &["export", "--openspec", "out"], &[]).code, 0);
    let b = empty("os-rt-b");
    let out_dir = a.join("out");
    let out = spec(
        &b,
        &["import", "--openspec", out_dir.to_str().unwrap()],
        &[],
    );
    assert_eq!(out.code, 0, "{}", out.stderr);
    let mut seen = 0;
    for feature in ["notes", "passkeys"] {
        let dir = a.join(".spec/features").join(feature);
        for e in fs::read_dir(&dir).unwrap() {
            let name = e.unwrap().file_name();
            let rel = format!(".spec/features/{feature}/{}", name.to_string_lossy());
            assert_eq!(
                fs::read(b.join(&rel)).unwrap(),
                fs::read(a.join(&rel)).unwrap(),
                "{rel}"
            );
            seen += 1;
        }
    }
    assert!(seen >= 4, "{seen}");
    assert!(!b.join(".spec/features/passkeys/design.md").exists());
    let json = spec(
        &empty("os-rt-json"),
        &["import", "--openspec", out_dir.to_str().unwrap(), "--json"],
        &[],
    );
    assert_eq!(json.code, 0, "{}", json.stderr);
    assert_eq!(json.text().lines().count(), 1);
    assert_spec_json("openspec", &json.text());
}

#[test]
fn import_refuses_edited_requirements() {
    let a = fixture("os-edit-a");
    assert_eq!(spec(&a, &["export", "--openspec", "out"], &[]).code, 0);
    let delta = a.join("out/openspec/changes/passkeys/specs/product-auth/spec.md");
    let text = fs::read_to_string(&delta)
        .unwrap()
        .replace("valid passkey", "valid key");
    fs::write(&delta, text).unwrap();
    let b = empty("os-edit-b");
    let out = spec(
        &b,
        &["import", "--openspec", a.join("out").to_str().unwrap()],
        &[],
    );
    assert_eq!(out.code, 1);
    assert_eq!(
        out.stderr,
        "import: passkeys: requirements were edited in OpenSpec; delete design.md's vibe blocks to import it as a new change\n"
    );
    assert!(!b.join(".spec/features/passkeys").exists());
    assert!(
        !b.join(".spec/features/notes").exists(),
        "nothing is written when any change refuses"
    );
}

#[test]
fn foreign_change_imports() {
    let src = project("os-foreign-src");
    write(
        &src,
        "openspec/changes/add-export/proposal.md",
        "# Proposal\n\n## Why\nUsers need their data.\n\n## What Changes\n- export\n",
    );
    write(
        &src,
        "openspec/changes/add-export/specs/product-auth/spec.md",
        "## ADDED Requirements\n\n### Requirement: Export\nThe system SHALL export CSV.\n\n#### Scenario: csv\n- **WHEN** asked\n- **THEN** csv\n\n## REMOVED Requirements\n\n- `### Requirement: Legacy dump`\n",
    );
    write(
        &src,
        "openspec/changes/add-export/tasks.md",
        "# Tasks\n\n## 1. Export\n\n- [x] 1.1 Writer\n- [ ] 1.2 Button\n",
    );
    let b = empty("os-foreign");
    write(&b, ".spec/product-auth.md", "# auth\n");
    let out = spec(&b, &["import", "--openspec", src.to_str().unwrap()], &[]);
    assert_eq!(out.code, 0, "{}", out.stderr);
    let product = read(&b, ".spec/features/add-export/product.md");
    let (head, body) = product.split_once("---\n\n").unwrap();
    assert!(head.starts_with("---\ntype: feature-product\nfeature: add-export\nsibling: tech.md\nparent: ../../product-auth.md\nupdated: "), "{head}");
    assert_eq!(
        body,
        "# Feature: add-export — Product\n\nUsers need their data.\n\n## ADDED Requirements\n\n### Requirement: Export\nThe system SHALL export CSV.\n\n#### Scenario: csv\n- **WHEN** asked\n- **THEN** csv\n\n## REMOVED Requirements\n\n### Requirement: Legacy dump\n"
    );
    let plan = read(&b, ".spec/features/add-export/plan.md");
    assert!(
        plan.contains("\n### add-export/1 — Writer\n\n### add-export/2 — Button\n"),
        "{plan}"
    );
    assert!(
        plan.ends_with("| add-export/1 | DONE |\n| add-export/2 | NOT STARTED |\n"),
        "{plan}"
    );
    assert!(b.join(".spec/features/add-export/tech.md").is_file());
}

#[test]
fn import_refuses() {
    let a = fixture("os-refuse-a");
    assert_eq!(spec(&a, &["export", "--openspec", "out"], &[]).code, 0);
    let out_dir = a.join("out");
    let exists = spec(
        &a,
        &["import", "--openspec", out_dir.to_str().unwrap()],
        &[],
    );
    assert_eq!(exists.code, 1);
    assert_eq!(
        exists.stderr,
        "import: notes: .spec/features/notes/ already exists\nimport: passkeys: .spec/features/passkeys/ already exists\n"
    );
    let src = project("os-refuse-src");
    write(
        &src,
        "openspec/changes/two-caps/specs/a/spec.md",
        "## ADDED Requirements\n### Requirement: A\nThe system SHALL a.\n#### Scenario: a\n- x\n",
    );
    write(
        &src,
        "openspec/changes/two-caps/specs/b/spec.md",
        "## ADDED Requirements\n### Requirement: B\nThe system SHALL b.\n#### Scenario: b\n- x\n",
    );
    write(
        &src,
        "openspec/changes/renamed/specs/a/spec.md",
        "## RENAMED Requirements\n- FROM: `### Requirement: A`\n- TO: `### Requirement: B`\n",
    );
    write(&src, "openspec/changes/Bad_Id/proposal.md", "# P\n");
    let out = spec(
        &empty("os-refuse-b"),
        &["import", "--openspec", src.to_str().unwrap()],
        &[],
    );
    assert_eq!(out.code, 1);
    assert_eq!(
        out.stderr,
        "import: Bad_Id: not a valid feature name (lowercase-kebab)\n\
         import: renamed: RENAMED Requirements is not supported\n\
         import: two-caps: touches 2 capabilities; a vibe feature targets one document\n"
    );
    let one = spec(
        &empty("os-refuse-one"),
        &[
            "import",
            "--openspec",
            src.to_str().unwrap(),
            "--feature",
            "nope",
        ],
        &[],
    );
    assert_eq!(
        (one.code, one.stderr.as_str()),
        (1, "import: no change 'nope' to import\n")
    );
}

#[test]
fn import_refuses_unsafe_names_and_symlinks() {
    use std::os::unix::fs::symlink;
    let src = project("os-unsafe-src");
    write(&src, "openspec/changes/ok/proposal.md", "# P\n");
    let b = empty("os-unsafe");
    for bad in ["../evil", "-x", "/abs", ".."] {
        let out = spec(
            &b,
            &[
                "import",
                "--openspec",
                src.to_str().unwrap(),
                &format!("--feature={bad}"),
            ],
            &[],
        );
        assert_eq!(out.code, 1, "{bad}");
        assert_eq!(out.stderr, format!("import: no change '{bad}' to import\n"));
    }
    assert!(!b.join(".spec/features").exists());
    // A symlinked change dir is refused, never followed.
    let outside = project("os-unsafe-outside");
    write(&outside, "proposal.md", "# P\n");
    symlink(&outside, src.join("openspec/changes/linked")).unwrap();
    let out = spec(&b, &["import", "--openspec", src.to_str().unwrap()], &[]);
    assert_eq!(out.code, 1);
    assert_eq!(out.stderr, "import: linked: is a symlink, not followed\n");
    assert!(!b.join(".spec/features").exists());
    fs::remove_file(src.join("openspec/changes/linked")).unwrap();
    // A symlinked features dir is refused and nothing lands behind it.
    let target = project("os-unsafe-features");
    symlink(&target, b.join(".spec/features")).unwrap();
    let out = spec(&b, &["import", "--openspec", src.to_str().unwrap()], &[]);
    assert_eq!(out.code, 1);
    assert_eq!(out.stderr, "import: .spec/features is a symlink\n");
    assert!(!target.join("ok").exists());
    // A symlinked design.md is refused.
    fs::remove_file(b.join(".spec/features")).unwrap();
    symlink(
        outside.join("proposal.md"),
        src.join("openspec/changes/ok/design.md"),
    )
    .unwrap();
    let out = spec(&b, &["import", "--openspec", src.to_str().unwrap()], &[]);
    assert_eq!(out.code, 1);
    assert!(out.stderr.contains("import: ok: "), "{}", out.stderr);
    assert!(out.stderr.contains("is a symlink"), "{}", out.stderr);
    assert!(!b.join(".spec/features").exists());
}

#[test]
fn import_without_a_root_exits_3_with_empty_stdout() {
    let p = project("os-import-noroot");
    let out = spec(&p, &["import", "--openspec", "out", "--json"], &[]);
    assert_eq!(out.code, 3);
    assert_eq!(out.text(), "");
}

#[test]
fn import_without_openspec_dir_refuses() {
    let b = empty("os-import-none");
    let out = spec(&b, &["import", "--openspec", "nowhere"], &[]);
    assert_eq!(
        (out.code, out.stderr.as_str()),
        (1, "import: no openspec/ under nowhere\n")
    );
}

/// `<root>/openspec/changes/<id>/design.md` carrying `blocks` after the marker.
fn carried_design(blocks: &str) -> String {
    format!("# Design\n\n<!-- vibe:export v1 -->\n{blocks}")
}

#[test]
fn import_pins_carried_edge_cases() {
    let a = empty("os-edge-a");
    let dir = ".spec/features/edge";
    write(
        &a,
        &format!("{dir}/product.md"),
        "---\ntype: feature-product\nfeature: edge\nupdated: 2026-01-15\n---\n\n# Feature: edge — Product\n\nEdge.\n",
    );
    write(&a, &format!("{dir}/design.md"), "# D\n```\ncode\n```");
    write(&a, &format!("{dir}/research.md"), "");
    assert_eq!(spec(&a, &["export", "--openspec", "out"], &[]).code, 0);
    let exported = read(&a, "out/openspec/changes/edge/design.md");
    assert!(
        exported.contains("````text\n# D\n```\ncode\n```\n````\n"),
        "{exported}"
    );
    let b = empty("os-edge-b");
    let out = spec(
        &b,
        &["import", "--openspec", a.join("out").to_str().unwrap()],
        &[],
    );
    assert_eq!(out.code, 0, "{}", out.stderr);
    for f in ["product.md", "design.md", "research.md"] {
        let rel = format!("{dir}/{f}");
        assert_eq!(
            fs::read(b.join(&rel)).unwrap(),
            fs::read(a.join(&rel)).unwrap(),
            "{rel}"
        );
    }
    // A bad closing fence refuses.
    let bad = exported.replacen("\n````\n", "\n```\n", 1);
    write(&a, "out/openspec/changes/edge/design.md", &bad);
    let out = spec(
        &empty("os-edge-c"),
        &["import", "--openspec", a.join("out").to_str().unwrap()],
        &[],
    );
    assert_eq!(out.code, 1);
    assert_eq!(
        out.stderr,
        "import: edge: design.md has a malformed vibe block\n"
    );
    // Duplicate carried names refuse.
    let block = "\n## vibe:product.md 2\n\n```text\nx\n```\n";
    write(
        &a,
        "out/openspec/changes/edge/design.md",
        &carried_design(&format!("{block}{block}")),
    );
    let out = spec(
        &empty("os-edge-d"),
        &["import", "--openspec", a.join("out").to_str().unwrap()],
        &[],
    );
    assert_eq!(out.code, 1);
    assert_eq!(
        out.stderr,
        "import: edge: design.md carries 'product.md' twice\n"
    );
}

#[test]
fn import_refuses_symlinked_specs() {
    use std::os::unix::fs::symlink;
    let src = project("os-speclink-src");
    let real = project("os-speclink-real");
    write(
        &real,
        "spec.md",
        "## ADDED Requirements\n\n### Requirement: A\nThe system SHALL a.\n\n#### Scenario: a\n- x\n",
    );
    fs::create_dir_all(src.join("openspec/changes/ok/specs/cap")).unwrap();
    symlink(
        real.join("spec.md"),
        src.join("openspec/changes/ok/specs/cap/spec.md"),
    )
    .unwrap();
    let b = empty("os-speclink-b");
    let out = spec(&b, &["import", "--openspec", src.to_str().unwrap()], &[]);
    assert_eq!(out.code, 1);
    assert_eq!(
        out.stderr,
        "import: ok: specs/cap/spec.md is a symlink, not followed\n"
    );
    assert!(!b.join(".spec/features").exists());
    fs::remove_dir_all(src.join("openspec/changes/ok/specs/cap")).unwrap();
    symlink(&real, src.join("openspec/changes/ok/specs/cap")).unwrap();
    let out = spec(&b, &["import", "--openspec", src.to_str().unwrap()], &[]);
    assert_eq!(out.code, 1);
    assert_eq!(
        out.stderr,
        "import: ok: specs/cap is a symlink, not followed\n"
    );
    assert!(!b.join(".spec/features").exists());
}

#[test]
fn import_refuses_a_features_file() {
    let src = project("os-featfile-src");
    write(&src, "openspec/changes/ok/proposal.md", "# P\n");
    let b = empty("os-featfile");
    write(&b, ".spec/features", "not a dir\n");
    let out = spec(&b, &["import", "--openspec", src.to_str().unwrap()], &[]);
    assert_eq!(out.code, 1);
    assert_eq!(
        out.stderr,
        "import: ERROR: .spec/features is not a directory\n"
    );
    assert_eq!(read(&b, ".spec/features"), "not a dir\n");
}

#[test]
fn import_skips_dot_dirs_in_changes() {
    let src = project("os-dot-src");
    write(&src, "openspec/changes/ok/proposal.md", "# P\n");
    write(&src, "openspec/changes/.git/HEAD", "ref\n");
    let b = empty("os-dot");
    let out = spec(&b, &["import", "--openspec", src.to_str().unwrap()], &[]);
    assert_eq!(out.code, 0, "{}", out.stderr);
    assert_eq!(out.stderr, "");
    assert!(b.join(".spec/features/ok/product.md").is_file());
}

#[test]
fn a_quoted_marker_is_not_a_carried_design() {
    let src = project("os-quote-src");
    let design = "# Design\n\nThe `<!-- vibe:export v1 -->` marker starts carried files.\n\n## vibe:product.md 2\n\n```text\nx\n```\n";
    write(&src, "openspec/changes/ok/design.md", design);
    let b = empty("os-quote");
    let out = spec(&b, &["import", "--openspec", src.to_str().unwrap()], &[]);
    assert_eq!(out.code, 0, "{}", out.stderr);
    assert_eq!(read(&b, ".spec/features/ok/design.md"), design);
    assert!(read(&b, ".spec/features/ok/product.md").contains("Imported from OpenSpec change ok."));
}
