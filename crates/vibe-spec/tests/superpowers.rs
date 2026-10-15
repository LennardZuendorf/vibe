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
            "docs/superpowers/plans/drafts: not named YYYY-MM-DD-<topic>.md",
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
    let sf36: Vec<String> = found(&p)
        .into_iter()
        .filter(|(c, _)| c == "SF36")
        .map(|(_, m)| m)
        .collect();
    assert_eq!(
        sf36,
        [
            "docs/superpowers/plans/2026-01-06-nospec.md: no '**Spec:**' line naming the design it implements",
            "docs/superpowers/plans/2026-01-07-gone.md: **Spec:** path 'docs/superpowers/specs/2026-01-01-gone-design.md' does not exist",
            "docs/superpowers/plans/2026-01-08-nopath.md: the '**Spec:**' line names no path",
        ]
    );
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
