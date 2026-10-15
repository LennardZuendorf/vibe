#![cfg(unix)]
#![allow(clippy::unwrap_used, clippy::expect_used)]
//! SF26–SF29: frontmatter values, children and backlinks, feature rows, freshness.
mod common;

use common::{project, validate_json, write};
use std::path::{Path, PathBuf};

const PRODUCT: &str =
    "---\ntype: entrypoint\nscope: product\nchildren:\nupdated: 2026-01-15\n---\n\n# P\n";
const PLAN: &str = "---\ntype: entrypoint\nscope: implementation\nchildren:\nupdated: 2026-01-15\n---\n\n| Feature | Status |\n|---|---|\n| [auth](features/auth/) | IN PROGRESS |\n| `billing` | NOT STARTED |\n";
const FPRODUCT: &str = "---\ntype: feature-product\nfeature: {f}\nparent: ../../product.md\nsibling: tech.md\nupdated: 2026-01-15\n---\n\n## Scope\n\n| Owns | Does not own |\n|---|---|\n| {f} things | other |\n\n### Requirement: A\nUsers MUST a.\n\n#### Scenario: a\n- WHEN x\n- THEN y\n";
const FTECH: &str = "---\ntype: feature-tech\nfeature: {f}\nparent: ../../tech.md\nsibling: product.md\nupdated: 2026-01-15\n---\n";

fn tree(name: &str, features: &[&str]) -> PathBuf {
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
