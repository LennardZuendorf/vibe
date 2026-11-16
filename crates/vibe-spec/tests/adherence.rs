#![cfg(unix)]
// vibe-spec: literal-file
#![allow(clippy::unwrap_used, clippy::expect_used)]
//! SF26–SF29: frontmatter values, children and backlinks, feature rows, freshness.
mod common;

use common::{git, project, validate_json, write};
use std::path::Path;

const PRODUCT: &str =
    "---\ntype: entrypoint\nscope: product\nchildren:\nupdated: 2026-01-15\n---\n\n# P\n";
const PLAN: &str = "---\ntype: entrypoint\nscope: implementation\nchildren:\nupdated: 2026-01-15\n---\n\n| Feature | Status |\n|---|---|\n| [auth](features/auth/) | IN PROGRESS |\n| `billing` | NOT STARTED |\n";
const FPRODUCT: &str = "---\ntype: feature-product\nfeature: {f}\nparent: ../../product.md\nsibling: tech.md\nupdated: 2026-01-15\n---\n\n## Scope\n\n| Owns | Does not own |\n|---|---|\n| {f} things | other |\n\n### Requirement: A\nUsers MUST a.\n\n#### Scenario: a\n- WHEN x\n- THEN y\n";
const FTECH: &str = "---\ntype: feature-tech\nfeature: {f}\nparent: ../../tech.md\nsibling: product.md\nupdated: 2026-01-15\n---\n";

fn tree(name: &str, features: &[&str]) -> vibe_core::testscratch::Scratch {
    let p = project(name);
    write(&p, ".spec/product.md", PRODUCT);
    write(&p, ".spec/plan.md", PLAN);
    for f in features {
        write(
            &p,
            &format!(".spec/features/{f}/product.md"),
            &FPRODUCT.replace("{f}", f),
        );
        write(
            &p,
            &format!(".spec/features/{f}/tech.md"),
            &FTECH.replace("{f}", f),
        );
    }
    p
}

fn found(p: &Path, check: &str) -> Vec<String> {
    let (_, v) = validate_json(p, &[], &[]);
    v["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["check"] == check)
        .map(|f| f["message"].as_str().unwrap().to_string())
        .collect()
}

#[test]
fn a_clean_tree_has_no_adherence_findings() {
    let p = tree("ad-clean", &["auth", "billing"]);
    for check in ["SF26", "SF27", "SF28"] {
        assert_eq!(found(&p, check), Vec::<String>::new(), "{check}");
    }
}

#[test]
fn frontmatter_values_are_checked() {
    let p = tree("ad-values", &["auth"]);
    write(
        &p,
        ".spec/product.md",
        &PRODUCT
            .replace("scope: product", "scope: marketing")
            .replace("2026-01-15", "{YYYY-MM-DD}"),
    );
    write(
        &p,
        ".spec/features/auth/research.md",
        &FTECH
            .replace("{f}", "auth")
            .replace("feature-tech", "feature-notes"),
    );
    let got = found(&p, "SF26");
    assert!(got.contains(&"product.md: scope 'marketing' is not one of product, technical, design, implementation".to_string()), "{got:?}");
    assert!(
        got.contains(&"product.md: updated '{YYYY-MM-DD}' is not a YYYY-MM-DD date".to_string()),
        "{got:?}"
    );
    assert!(
        got.iter()
            .any(|m| m.starts_with("features/auth/research.md: type 'feature-notes' is not one of")),
        "{got:?}"
    );
}

#[test]
fn a_bad_legacy_feature_type_is_sf20_only() {
    let p = tree("ad-legacy-type", &["auth"]);
    write(
        &p,
        ".spec/features/auth/tech.md",
        &FTECH
            .replace("{f}", "auth")
            .replace("feature-tech", "feature-notes"),
    );
    assert_eq!(found(&p, "SF26"), Vec::<String>::new());
    assert!(!found(&p, "SF20").is_empty());
}

#[test]
fn blank_or_non_scalar_values_are_invalid() {
    let p = tree("ad-blank", &[]);
    write(
        &p,
        ".spec/product.md",
        "---\ntype:\nscope: product\nchildren:\nupdated:\n---\n",
    );
    let got = found(&p, "SF26");
    assert!(
        got.contains(&"product.md: type is blank or not a scalar value".to_string()),
        "{got:?}"
    );
    assert!(
        got.contains(&"product.md: updated is blank or not a scalar value".to_string()),
        "{got:?}"
    );
    write(
        &p,
        ".spec/product.md",
        "---\ntype: entrypoint\nscope: product\nchildren:\nupdated: [2026-01-15]\n---\n",
    );
    let got = found(&p, "SF26");
    assert!(
        got.contains(&"product.md: updated is blank or not a scalar value".to_string()),
        "{got:?}"
    );
}

#[test]
fn inline_children_must_exist_and_backlink() {
    let p = tree("ad-children", &[]);
    write(
        &p,
        ".spec/tech.md",
        "---\ntype: entrypoint\nscope: technical\nchildren: [tech-api.md, tech-gone.md, tech-{topic}.md]\nupdated: 2026-01-15\n---\n",
    );
    write(
        &p,
        ".spec/tech-api.md",
        "---\ntype: tech-topic\nparent: product.md\nscope: api\ncovers: x\nupdated: 2026-01-15\n---\n",
    );
    let got = found(&p, "SF27");
    assert_eq!(
        got,
        [
            "tech-api.md: parent is 'product.md', but tech.md lists it as a child",
            "tech.md: child 'tech-gone.md' listed in frontmatter but file doesn't exist",
        ],
        "{got:?}"
    );
}

#[test]
fn a_leading_dot_slash_is_normalised() {
    let p = tree("ad-dotslash", &[]);
    write(
        &p,
        ".spec/tech.md",
        "---\ntype: entrypoint\nscope: technical\nchildren: [./tech-a.md, ./tech-b.md]\nupdated: 2026-01-15\n---\n",
    );
    write(
        &p,
        ".spec/tech-a.md",
        "---\ntype: tech-topic\nparent: \"./tech.md\"\nscope: a\ncovers: x\nupdated: 2026-01-15\n---\n",
    );
    write(
        &p,
        ".spec/tech-b.md",
        "---\ntype: tech-topic\nparent: product.md\nscope: b\ncovers: x\nupdated: 2026-01-15\n---\n",
    );
    let (_, v) = validate_json(&p, &[], &[]);
    let hits: Vec<_> = v["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["check"] == "SF27")
        .collect();
    assert_eq!(hits.len(), 1, "{hits:?}");
    assert_eq!(
        hits[0]["message"],
        "tech-b.md: parent is 'product.md', but tech.md lists it as a child"
    );
    assert!(
        hits[0]["file"]
            .as_str()
            .unwrap()
            .ends_with(".spec/tech-b.md"),
        "{hits:?}"
    );
}

#[test]
fn a_missing_block_child_is_reported_once() {
    let p = tree("ad-block", &[]);
    write(
        &p,
        ".spec/tech.md",
        "---\ntype: entrypoint\nscope: technical\nchildren:\n  - tech-gone.md\nupdated: 2026-01-15\n---\n",
    );
    assert_eq!(found(&p, "SF27"), Vec::<String>::new());
    assert_eq!(found(&p, "SF22").len(), 1);
}

#[test]
fn feature_folders_need_a_plan_row() {
    let p = tree("ad-rows", &["auth", "billing", "search"]);
    assert_eq!(
        found(&p, "SF28"),
        ["features/search/: no row in plan.md names 'search'"]
    );
    std::fs::remove_file(p.join(".spec/plan.md")).unwrap();
    assert_eq!(found(&p, "SF28").len(), 3);
}

#[test]
fn a_fenced_plan_row_does_not_count() {
    let p = tree("ad-fenced", &["auth", "billing", "fenced"]);
    write(
        &p,
        ".spec/plan.md",
        &format!("{PLAN}\n```\n| fenced | x |\n```\n\n~~~\n| fenced | y |\n~~~\n"),
    );
    assert_eq!(
        found(&p, "SF28"),
        ["features/fenced/: no row in plan.md names 'fenced'"]
    );
}

fn committed(name: &str, date: &str) -> vibe_core::testscratch::Scratch {
    let p = tree(name, &["auth"]);
    git(&p, date, &["init", "-q"]);
    git(&p, date, &["add", "-A"]);
    git(&p, date, &["commit", "-q", "-m", "init"]);
    p
}

/// `tree` with every stamp set to `date` and committed on that day.
fn stamped(name: &str, date: &str) -> vibe_core::testscratch::Scratch {
    let p = tree(name, &["auth"]);
    for rel in [
        ".spec/product.md",
        ".spec/plan.md",
        ".spec/features/auth/product.md",
        ".spec/features/auth/tech.md",
    ] {
        let text = std::fs::read_to_string(p.join(rel)).unwrap();
        write(&p, rel, &text.replace("2026-01-15", date));
    }
    git(&p, date, &["init", "-q"]);
    commit(&p, &format!("{date}T12:00:00Z"), "init");
    p
}

fn commit(p: &Path, date: &str, message: &str) {
    git(p, date, &["add", "-A"]);
    git(p, date, &["commit", "-q", "-m", message]);
}

/// `product.md` with `date` as its `updated:` stamp.
fn product(date: &str) -> String {
    PRODUCT.replace("2026-01-15", date)
}

#[test]
fn a_commit_that_leaves_the_stamp_alone_is_reported() {
    let p = stamped("ad-stale", "2026-11-01");
    write(
        &p,
        ".spec/product.md",
        &format!("{}\nAn edit.\n", product("2026-11-01")),
    );
    commit(&p, "2026-11-04T12:00:00Z", "edit the note, not the stamp");
    assert_eq!(
        found(&p, "SF29"),
        ["product.md: updated 2026-11-01 was not bumped by its last commit (2026-11-04)"]
            .map(str::to_string)
    );
}

#[test]
fn a_one_day_squash_slip_is_silent() {
    // The shards #64 shape: the note is stamped and edited on 2026-11-02, the
    // squash carries the merge day 2026-11-03, so the stamp did not move either.
    let p = stamped("ad-slip", "2026-11-02");
    write(
        &p,
        ".spec/product.md",
        &format!("{}\nAn edit.\n", product("2026-11-02")),
    );
    commit(&p, "2026-11-03T12:00:00Z", "stamp 11-02, merge 11-03");
    assert_eq!(found(&p, "SF29"), Vec::<String>::new());
}

#[test]
fn a_two_day_gap_with_an_unchanged_stamp_is_reported() {
    // One day is the squash window; two is a stamp nobody moved.
    let p = stamped("ad-two-day", "2026-11-01");
    write(
        &p,
        ".spec/product.md",
        &format!("{}\nAn edit.\n", product("2026-11-01")),
    );
    commit(&p, "2026-11-03T12:00:00Z", "edit");
    assert_eq!(
        found(&p, "SF29"),
        ["product.md: updated 2026-11-01 was not bumped by its last commit (2026-11-03)"]
            .map(str::to_string)
    );
}

#[test]
fn a_stamp_set_during_a_long_branch_survives_the_merge() {
    // The shards shape beyond one day: the stamp names the day the note was
    // written, the squash carries the merge day a week later.
    let p = stamped("ad-branch", "2026-11-01");
    write(
        &p,
        ".spec/product.md",
        &format!("{}\nAn edit.\n", product("2026-11-06")),
    );
    commit(&p, "2026-11-13T12:00:00Z", "stamp 11-06, merge 11-13");
    assert_eq!(found(&p, "SF29"), Vec::<String>::new());
}

#[test]
fn a_note_its_commit_adds_keeps_its_own_stamp() {
    // No previous version to be stale against: the author's stamp is the only record.
    let p = committed("ad-added", "2026-03-01T12:00:00Z");
    assert_eq!(found(&p, "SF29"), Vec::<String>::new());
}

#[test]
fn fresh_updated_is_silent() {
    let p = committed("ad-fresh", "2026-01-15T12:00:00Z");
    assert_eq!(found(&p, "SF29"), Vec::<String>::new());
}

#[test]
fn freshness_is_silent_without_history() {
    let bare = tree("ad-nogit", &["auth"]);
    assert_eq!(found(&bare, "SF29"), Vec::<String>::new(), "empty .git dir");
    let fresh = tree("ad-nocommit", &["auth"]);
    git(&fresh, "2026-03-01T12:00:00Z", &["init", "-q"]);
    assert_eq!(found(&fresh, "SF29"), Vec::<String>::new(), "no commits");
    let p = committed("ad-untracked", "2026-01-15T12:00:00Z");
    write(
        &p,
        ".spec/tech.md",
        "---\ntype: entrypoint\nscope: technical\nchildren:\nupdated: 2020-01-01\n---\n",
    );
    assert_eq!(found(&p, "SF29"), Vec::<String>::new(), "untracked file");
    let empty = p.join("nogit-bin");
    std::fs::create_dir_all(&empty).unwrap();
    let path = empty.display().to_string();
    let (code, v) = validate_json(
        &committed("ad-nobin", "2026-03-01T12:00:00Z"),
        &[],
        &[("PATH", path.as_str())],
    );
    assert_eq!(code, 0);
    assert!(
        v["findings"]
            .as_array()
            .unwrap()
            .iter()
            .all(|f| f["check"] != "SF29"),
        "no git binary"
    );
}

#[test]
fn an_outside_root_is_silent() {
    let p = committed("ad-outside", "2026-03-01T12:00:00Z");
    let outside = tree("ad-outside-root", &["auth"]);
    let root = outside.join(".spec");
    let out = common::spec(
        &p,
        &[
            "--root",
            root.to_str().unwrap(),
            "validate",
            "--format",
            "json",
        ],
        &[],
    );
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert!(
        v["findings"]
            .as_array()
            .unwrap()
            .iter()
            .all(|f| f["check"] != "SF29"),
        "{v:#}"
    );
}

#[test]
fn blank_or_bad_updated_is_left_to_sf26() {
    let p = committed("ad-blank", "2026-03-01T12:00:00Z");
    write(
        &p,
        ".spec/product.md",
        &PRODUCT.replace("updated: 2026-01-15", "updated:"),
    );
    common::git(&p, "2026-03-02T12:00:00Z", &["add", "-A"]);
    common::git(&p, "2026-03-02T12:00:00Z", &["commit", "-q", "-m", "blank"]);
    let got = found(&p, "SF29");
    assert!(got.iter().all(|m| !m.starts_with("product.md:")), "{got:?}");
}

fn root_findings(cwd: &Path, root: &Path) -> Vec<String> {
    let out = common::spec(
        cwd,
        &[
            "--root",
            root.to_str().unwrap(),
            "validate",
            "--format",
            "json",
        ],
        &[],
    );
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    v["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["check"] == "SF29")
        .map(|f| f["message"].as_str().unwrap().to_string())
        .collect()
}

#[test]
fn a_shallow_clone_is_silent() {
    let p = project("ad-shallow-src");
    write(&p, ".spec/product.md", PRODUCT);
    write(
        &p,
        ".spec/tech.md",
        &PRODUCT.replace("scope: product", "scope: technical"),
    );
    let old = "2026-01-15T12:00:00Z";
    git(&p, old, &["init", "-q"]);
    git(&p, old, &["add", "-A"]);
    git(&p, old, &["commit", "-q", "-m", "docs"]);
    write(&p, "other.txt", "x");
    let new = "2026-03-01T12:00:00Z";
    git(&p, new, &["add", "-A"]);
    git(&p, new, &["commit", "-q", "-m", "other"]);
    let destination = project("ad-shallow-dst");
    let dest = destination.join("clone");
    let url = format!("file://{}", p.display());
    git(
        &p,
        new,
        &["clone", "-q", "--depth", "1", &url, dest.to_str().unwrap()],
    );
    assert_eq!(
        common::git_out(&dest, &["rev-parse", "--is-shallow-repository"]),
        "true"
    );
    assert_eq!(found(&dest, "SF29"), Vec::<String>::new());
}

#[test]
fn a_pathspec_looking_directory_is_literal() {
    let p = tree("ad-pathspec", &["auth"]);
    let odd = p.join(":(top)odd");
    std::fs::create_dir_all(&odd).unwrap();
    std::fs::rename(p.join(".spec"), odd.join(".spec")).unwrap();
    let d = "2026-01-15T12:00:00Z";
    git(&p, d, &["init", "-q"]);
    git(&p, d, &["add", "-A"]);
    git(&p, d, &["commit", "-q", "-m", "init"]);
    // A second commit that leaves the stamp alone: the finding proves the path matched.
    write(
        &p,
        ":(top)odd/.spec/product.md",
        &format!("{PRODUCT}\nAn edit.\n"),
    );
    commit(&p, "2026-03-01T12:00:00Z", "edit");
    let got = root_findings(&p, &odd.join(".spec"));
    assert!(got.iter().any(|m| m.starts_with("product.md:")), "{got:?}");
}

#[test]
fn quoted_file_names_still_match() {
    let p = tree("ad-quoted", &["auth"]);
    write(
        &p,
        ".spec/tech-a\"b.md",
        &PRODUCT.replace("scope: product", "scope: technical"),
    );
    let d = "2026-01-15T12:00:00Z";
    git(&p, d, &["init", "-q"]);
    git(&p, d, &["add", "-A"]);
    git(&p, d, &["commit", "-q", "-m", "init"]);
    // A second commit that leaves the stamp alone: the finding proves the name matched.
    write(
        &p,
        ".spec/tech-a\"b.md",
        &format!(
            "{}\nAn edit.\n",
            PRODUCT.replace("scope: product", "scope: technical")
        ),
    );
    commit(&p, "2026-03-01T12:00:00Z", "edit");
    let got = found(&p, "SF29");
    assert!(
        got.iter().any(|m| m.starts_with("tech-a\"b.md:")),
        "{got:?}"
    );
}

#[test]
fn a_changed_doc_edited_without_a_bump_fails_under_strict() {
    // Committed with the file's own date, so the commit-date check is silent.
    let p = committed("ad-edit-nobump", "2026-01-15T12:00:00Z");
    assert_eq!(found(&p, "SF29"), Vec::<String>::new());
    write(
        &p,
        ".spec/product.md",
        &format!("{PRODUCT}\nAn edit with no bump.\n"),
    );
    let out = common::spec(
        &p,
        &["validate", "--changed", "--strict", "--format", "json"],
        &[],
    );
    assert_eq!(out.code, 1, "{}{}", out.text(), out.stderr);
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let hits: Vec<&serde_json::Value> = v["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["check"] == "SF29")
        .collect();
    assert_eq!(hits.len(), 1, "{v}");
    assert_eq!(hits[0]["severity"], "error", "{v}");
    assert!(
        hits[0]["message"]
            .as_str()
            .unwrap()
            .starts_with("product.md: updated 2026-01-15 was not bumped by this edit"),
        "{v}"
    );
    // An unchanged stale doc keeps the commit-date wording (no --changed).
    let out = common::spec(&p, &["validate", "--format", "json"], &[]);
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert!(
        v["findings"]
            .as_array()
            .unwrap()
            .iter()
            .all(|f| f["check"] != "SF29"),
        "the uncommitted edit is not judged by the last commit: {v}"
    );
}

#[test]
fn a_changed_doc_that_bumps_the_stamp_is_silent() {
    let p = committed("ad-edit-bump", "2026-01-15T12:00:00Z");
    write(
        &p,
        ".spec/product.md",
        &format!("{}\nAn edit.\n", product("2026-05-05")),
    );
    let out = common::spec(
        &p,
        &["validate", "--changed", "--strict", "--format", "json"],
        &[],
    );
    assert_eq!(out.code, 0, "{}{}", out.text(), out.stderr);
}

#[test]
fn a_changed_doc_whose_stamp_names_today_is_silent() {
    // The edit is today's, and the stamp already says today: nothing is stale.
    let today = today();
    let p = stamped("ad-edit-today", &today);
    write(
        &p,
        ".spec/product.md",
        &format!("{}\nAn edit.\n", product(&today)),
    );
    let out = common::spec(
        &p,
        &["validate", "--changed", "--strict", "--format", "json"],
        &[],
    );
    assert_eq!(out.code, 0, "{}{}", out.text(), out.stderr);
}

/// Today's UTC date, as the checker reads the clock.
fn today() -> String {
    let out = std::process::Command::new("date")
        .arg("-u")
        .arg("+%F")
        .output()
        .unwrap();
    String::from_utf8(out.stdout).unwrap().trim().to_string()
}
