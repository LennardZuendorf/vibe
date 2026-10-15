#![cfg(unix)]
#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Requirement grammar: the substring-bug fixes (P3b-R16, R17, R3), SF24,
//! SF25 and the OpenSpec samples (P3b-R19).
mod common;

use common::{project, validate_json, write};
use std::path::PathBuf;

const HEAD: &str = "---\ntype: feature-product\nfeature: auth\nparent: ../../product.md\nsibling: tech.md\nupdated: 2026-01-15\n---\n\n";
const SCOPE: &str = "## Scope\n\n| Owns | Does not own |\n|---|---|\n| sessions | billing |\n\n";
const TECH: &str = "---\ntype: feature-tech\nfeature: auth\nparent: ../../tech.md\nsibling: product.md\nupdated: 2026-01-15\n---\n";

/// A project whose only feature product body is `body` (after the frontmatter).
fn feature(name: &str, body: &str) -> PathBuf {
    let p = project(name);
    write(
        &p,
        ".spec/product.md",
        "---\ntype: entrypoint\nscope: product\nchildren:\nupdated: 2026-01-15\n---\n",
    );
    write(
        &p,
        ".spec/features/auth/product.md",
        &format!("{HEAD}{body}"),
    );
    write(&p, ".spec/features/auth/tech.md", TECH);
    p
}

fn messages(p: &std::path::Path, check: &str) -> Vec<String> {
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
fn r_id_citations_are_word_bounded() {
    let p = feature(
        "gr-rid",
        &format!(
            "{SCOPE}### Requirement: Sign in (R1)\nUsers MUST sign in.\n#### Scenario: a\n- WHEN x\n- THEN y\n\n### Requirement: R10 — Sign out\nUsers MAY sign out.\n#### Scenario: b\n- WHEN x\n- THEN y\n"
        ),
    );
    write(
        &p,
        ".spec/features/auth/plan.md",
        "---\ntype: feature-plan\nfeature: auth\nparent: ../../plan.md\nupdated: 2026-01-15\n---\n\n## Requirements Trace\n\nCovers R10 and xR1 and R1x.\n\n### auth/1 — a\n",
    );
    let sf12 = messages(&p, "SF12");
    assert_eq!(
        sf12,
        ["features/auth: R1 in product.md not cited in plan.md (light traceability check)"],
        "{sf12:?}"
    );
}

#[test]
fn the_openspec_id_prefix_is_an_r_id() {
    let p = feature(
        "gr-rid-prefix",
        &format!(
            "{SCOPE}### Requirement: R7 — Export\nUsers MUST export.\n#### Scenario: a\n- WHEN x\n- THEN y\n"
        ),
    );
    write(
        &p,
        ".spec/features/auth/plan.md",
        "---\ntype: feature-plan\nfeature: auth\nparent: ../../plan.md\nupdated: 2026-01-15\n---\n\n## Requirements Trace\n\nNothing cited.\n\n### auth/1 — a\n",
    );
    assert_eq!(
        messages(&p, "SF12"),
        ["features/auth: R7 in product.md not cited in plan.md (light traceability check)"]
    );
}

#[test]
fn rfc_keywords_are_capital_whole_words() {
    let p = feature(
        "gr-rfc",
        &format!(
            "{SCOPE}### Requirement: Mayor sign in\nUsers mustard the shallow form; they MUSTN'T x.\n#### Scenario: a\n- WHEN x\n- THEN y\n\n### Requirement: Logout\nUsers MUST NOT stay.\n#### Scenario: b\n- WHEN x\n- THEN y\n"
        ),
    );
    let sf10 = messages(&p, "SF10");
    assert!(
        sf10.iter().any(
            |m| m.contains("missing RFC-2119 keyword (SHALL|MUST|SHOULD|MAY) -> Mayor sign in")
        ),
        "{sf10:?}"
    );
    assert!(!sf10.iter().any(|m| m.contains("-> Logout")), "{sf10:?}");
}

#[test]
fn scope_words_count_only_inside_the_scope_section() {
    let p = feature(
        "gr-scope",
        "## Scope\n\nTBD.\n\n## Notes\n\nThe downstream team does not own billing. Owns nothing.\n\n### Requirement: A\nUsers MUST a.\n#### Scenario: a\n- WHEN x\n- THEN y\n",
    );
    let sf8 = messages(&p, "SF8");
    assert_eq!(
        sf8,
        [
            "features/auth/product.md: Scope section missing 'Owns' column or row",
            "features/auth/product.md: Scope section missing 'Does not own' column or row",
        ],
        "{sf8:?}"
    );
    let ok = feature(
        "gr-scope-ok",
        "## Scope\n\nOwns invoices; does not own auth.\n\n### Requirement: A\nUsers MUST a.\n#### Scenario: a\n- WHEN x\n- THEN y\n",
    );
    assert!(messages(&ok, "SF8").is_empty());
    let scopes = feature(
        "gr-scopes",
        "## Scopes\n\n| Owns | Does not own |\n\n### Requirement: A\nUsers MUST a.\n#### Scenario: a\n- WHEN x\n- THEN y\n",
    );
    assert_eq!(
        messages(&scopes, "SF8"),
        ["features/auth/product.md: missing '## Scope' section (add Owns / Does not own table)"]
    );
}
