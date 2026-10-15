#![cfg(unix)]
#![allow(clippy::unwrap_used, clippy::expect_used)]
//! SF35 artifact names and SF36 plan `**Spec:**` paths (P3b-R11).
mod common;

use common::{project, validate_json, write};
use std::path::{Path, PathBuf};

fn sp(name: &str) -> PathBuf {
    let p = project(name);
    write(
        &p,
        ".spec/product.md",
        "---\ntype: entrypoint\nscope: product\nchildren:\nupdated: 2026-01-15\n---\n",
    );
    write(
        &p,
        "docs/superpowers/specs/2026-01-02-auth-design.md",
        "# Auth design\n",
    );
    write(
        &p,
        "docs/superpowers/plans/2026-01-03-auth.md",
        "# Auth plan\n\n**Spec:** `docs/superpowers/specs/2026-01-02-auth-design.md` (§2). Index: `docs/x.md`.\n",
    );
    write(
        &p,
        "docs/superpowers/plans/2026-01-04-auth-p2.md",
        "# P2\n\n**Spec:** [design](../specs/2026-01-02-auth-design.md#goal)\n",
    );
    p
}

fn found(p: &Path) -> Vec<(String, String)> {
    let (_, v) = validate_json(p, &[], &[]);
    v["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| matches!(f["check"].as_str(), Some("SF35" | "SF36")))
        .map(|f| {
            (
                f["check"].as_str().unwrap().to_string(),
                f["message"].as_str().unwrap().to_string(),
            )
        })
        .collect()
}

#[test]
fn good_artifacts_pass() {
    assert_eq!(found(&sp("sp-ok")), Vec::<(String, String)>::new());
}

#[test]
fn bad_names_fail() {
    let p = sp("sp-names");
    write(
        &p,
        "docs/superpowers/specs/2026-01-02-auth.md",
        "# no -design\n",
    );
    write(
        &p,
        "docs/superpowers/plans/notes.md",
        "**Spec:** `docs/superpowers/specs/2026-01-02-auth-design.md`\n",
    );
    write(
        &p,
        "docs/superpowers/plans/2026-13-40-x.md",
        "**Spec:** `docs/superpowers/specs/2026-01-02-auth-design.md`\n",
    );
    write(
        &p,
        "docs/superpowers/plans/2026-01-05-Bad_Topic.md",
        "**Spec:** `docs/superpowers/specs/2026-01-02-auth-design.md`\n",
    );
    write(&p, "docs/superpowers/plans/drafts/x.md", "x\n");
    write(&p, "docs/superpowers/plans/.DS_Store", "x");
    let got = found(&p);
    let sf35: Vec<&str> = got
        .iter()
        .filter(|(c, _)| c == "SF35")
        .map(|(_, m)| m.as_str())
        .collect();
    assert_eq!(
        sf35,
        [
            "docs/superpowers/specs/2026-01-02-auth.md: not named YYYY-MM-DD-<topic>-design.md",
            "docs/superpowers/plans/2026-01-05-Bad_Topic.md: not named YYYY-MM-DD-<topic>.md",
            "docs/superpowers/plans/2026-13-40-x.md: not named YYYY-MM-DD-<topic>.md",
            "docs/superpowers/plans/drafts: not a file (subfolders are not read)",
            "docs/superpowers/plans/notes.md: not named YYYY-MM-DD-<topic>.md",
        ],
        "{got:#?}"
    );
}

#[test]
fn plan_spec_lines_must_resolve() {
    let p = sp("sp-links");
    write(
        &p,
        "docs/superpowers/plans/2026-01-06-nospec.md",
        "# No spec line\n",
    );
    write(
        &p,
        "docs/superpowers/plans/2026-01-07-gone.md",
        "**Spec:** `docs/superpowers/specs/2026-01-01-gone-design.md`\n",
    );
    write(
        &p,
        "docs/superpowers/plans/2026-01-08-nopath.md",
        "**Spec:** the auth design\n",
    );
    for (n, body) in [
        ("09-anchor", "**Spec:** `#anchor`\n"),
        ("10-dir", "**Spec:** `docs`\n"),
        (
            "11-angle",
            "**Spec:** [d](<../specs/2026-01-02-auth-design.md>)\n",
        ),
        ("12-up", "**Spec:** `../../../../outside-sp-up.md`\n"),
    ] {
        write(&p, &format!("docs/superpowers/plans/2026-01-{n}.md"), body);
    }
    let outside = p.parent().unwrap().join("outside-sp-up.md");
    std::fs::write(&outside, "# outside\n").unwrap();
    let abs = outside.to_string_lossy().into_owned();
    write(
        &p,
        "docs/superpowers/plans/2026-01-13-abs.md",
        &format!("**Spec:** `{abs}`\n"),
    );
    let sf36: Vec<String> = found(&p)
        .into_iter()
        .filter(|(c, _)| c == "SF36")
        .map(|(_, m)| m)
        .collect();
    let no_path = "the '**Spec:**' line names no path \u{2014} put the path in backticks or a link";
    let want = [
        "docs/superpowers/plans/2026-01-06-nospec.md: no '**Spec:**' line naming the design it implements".to_string(),
        "docs/superpowers/plans/2026-01-07-gone.md: **Spec:** path 'docs/superpowers/specs/2026-01-01-gone-design.md' does not exist".to_string(),
        format!("docs/superpowers/plans/2026-01-08-nopath.md: {no_path}"),
        format!("docs/superpowers/plans/2026-01-09-anchor.md: {no_path}"),
        "docs/superpowers/plans/2026-01-10-dir.md: **Spec:** path 'docs' is not a file".to_string(),
        "docs/superpowers/plans/2026-01-12-up.md: **Spec:** path '../../../../outside-sp-up.md' is outside the project".to_string(),
        format!("docs/superpowers/plans/2026-01-13-abs.md: **Spec:** path '{abs}' is outside the project"),
    ];
    assert_eq!(sf36, want);
}

#[test]
fn this_repo_s_artifacts_pass() {
    let (_, v) = validate_json(&common::repo(), &[], &[]);
    let bad: Vec<&serde_json::Value> = v["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| matches!(f["check"].as_str(), Some("SF35" | "SF36")))
        .collect();
    assert!(bad.is_empty(), "{bad:#?}");
}

#[test]
fn fenced_spec_line_does_not_count() {
    let p = sp("sp-fence");
    write(
        &p,
        "docs/superpowers/plans/2026-01-09-fenced.md",
        "# Fenced\n\n```\n**Spec:** `docs/superpowers/specs/2026-01-02-auth-design.md`\n```\n",
    );
    let sf36: Vec<String> = found(&p)
        .into_iter()
        .filter(|(c, _)| c == "SF36")
        .map(|(_, m)| m)
        .collect();
    assert_eq!(
        sf36,
        [
            "docs/superpowers/plans/2026-01-09-fenced.md: no '**Spec:**' line naming the design it implements"
        ]
    );
}

#[test]
fn layered_projects_get_sf36_too() {
    let p = common::layered_tree("sp-layered");
    write(
        &p,
        "docs/superpowers/plans/2026-01-03-auth.md",
        "# No spec line\n",
    );
    assert_eq!(
        found(&p),
        [(
            "SF36".to_string(),
            "docs/superpowers/plans/2026-01-03-auth.md: no '**Spec:**' line naming the design it implements".to_string()
        )]
    );
}

#[test]
fn symlinked_plans_follow_the_link() {
    let p = sp("sp-symlink");
    write(
        &p,
        "docs/real/target.md",
        "**Spec:** `docs/superpowers/specs/2026-01-02-auth-design.md`\n",
    );
    std::os::unix::fs::symlink(
        p.join("docs/real/target.md"),
        p.join("docs/superpowers/plans/2026-01-14-linked.md"),
    )
    .unwrap();
    assert_eq!(found(&p), Vec::<(String, String)>::new());
}
