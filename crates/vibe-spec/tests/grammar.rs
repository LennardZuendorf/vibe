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

const REQ: &str = "### Requirement: A\nUsers MUST a.\n#### Scenario: a\n- WHEN x\n- THEN y\n";
const MISSING_BOTH: [&str; 2] = [
    "features/auth/product.md: Scope section missing 'Owns' column or row",
    "features/auth/product.md: Scope section missing 'Does not own' column or row",
];

#[test]
fn fenced_lines_neither_start_nor_end_the_scope_section() {
    let a = feature(
        "gr-scope-fence-end",
        &format!("## Scope\n\n```\n## sample\n```\n\nOwns a; does not own b.\n\n{REQ}"),
    );
    assert!(messages(&a, "SF8").is_empty(), "{:?}", messages(&a, "SF8"));
    let b = feature(
        "gr-scope-fence-start",
        &format!("```\n## Scope\n\nOwns a; does not own b.\n```\n\n## Scope\n\nTBD.\n\n{REQ}"),
    );
    assert_eq!(messages(&b, "SF8"), MISSING_BOTH);
    let c = feature(
        "gr-scope-fence-words",
        &format!("## Scope\n\n```\nOwns a; does not own b.\n```\n\n{REQ}"),
    );
    assert_eq!(messages(&c, "SF8"), MISSING_BOTH);
}

#[test]
fn the_scope_heading_allows_trailing_blanks_only() {
    let p = feature(
        "gr-scope-indent",
        &format!("  ## Scope\n\nOwns a; does not own b.\n\n{REQ}"),
    );
    assert_eq!(
        messages(&p, "SF8"),
        ["features/auth/product.md: missing '## Scope' section (add Owns / Does not own table)"]
    );
    let ok = feature(
        "gr-scope-trail",
        &format!("## Scope  \t\n\nOwns a; does not own b.\n\n{REQ}"),
    );
    assert!(messages(&ok, "SF8").is_empty());
}

#[test]
fn scope_section_boundaries() {
    let sub = feature(
        "gr-scope-h3",
        &format!("## Scope\n\n### Detail\n\nOwns a; does not own b.\n\n{REQ}"),
    );
    assert!(messages(&sub, "SF8").is_empty());
    let top = feature(
        "gr-scope-h1",
        &format!("## Scope\n\nTBD.\n\n# Top\n\nOwns a; does not own b.\n\n{REQ}"),
    );
    assert_eq!(messages(&top, "SF8"), MISSING_BOTH);
    let crlf = feature(
        "gr-scope-crlf",
        "## Scope\r\n\r\nOwns a; does not own b.\r\n\r\n## Next\r\n",
    );
    assert!(messages(&crlf, "SF8").is_empty());
}

/// OpenSpec's requirement grammar (P3b-R19): `### Requirement:`, a SHALL or
/// MUST sentence, `#### Scenario:`, `- **WHEN**` / `- **THEN**` bullets.
const OPENSPEC_AUTH: &str = "### Requirement: User Authentication\nThe system SHALL issue a session token on successful login.\n\n#### Scenario: Valid credentials\n- **WHEN** a user submits valid credentials\n- **THEN** a session token is returned\n\n#### Scenario: Invalid credentials\n- **WHEN** a user submits a wrong password\n- **THEN** an error is returned\n- **AND** no token is issued\n";
const OPENSPEC_EXPORT: &str = "### Requirement: R2 — Data export\nUsers MUST be able to export their data as CSV.\n\n#### Scenario: Export with a fenced sample\n- **GIVEN** an account with data\n- **WHEN** the user requests an export\n- **THEN** a CSV file is produced\n\n```text\n#### Scenario: not a scenario\n- WHEN ignored\n```\n";

fn grammar(p: &std::path::Path) -> Vec<String> {
    let (_, v) = validate_json(p, &[], &[]);
    v["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| ["SF10", "SF24", "SF25"].contains(&f["check"].as_str().unwrap()))
        .map(|f| {
            format!(
                "{} {}",
                f["check"].as_str().unwrap(),
                f["message"].as_str().unwrap()
            )
        })
        .collect()
}

#[test]
fn openspec_samples_pass() {
    let p = feature(
        "gr-os",
        &format!("{SCOPE}## Requirements\n\n{OPENSPEC_AUTH}\n{OPENSPEC_EXPORT}"),
    );
    assert_eq!(grammar(&p), Vec::<String>::new());
}

#[test]
fn mutated_openspec_samples_fail() {
    let cases: [(&str, String, &str); 5] = [
        ("gr-os-noshall", OPENSPEC_AUTH.replace("SHALL ", ""), "SF10"),
        (
            "gr-os-shallow",
            OPENSPEC_AUTH.replace("SHALL", "SHALLOW"),
            "SF10",
        ),
        (
            "gr-os-nothen",
            OPENSPEC_AUTH.replace("- **THEN** a session token is returned\n", ""),
            "SF25",
        ),
        (
            "gr-os-nowhen",
            OPENSPEC_AUTH.replace("- **WHEN** a user submits a wrong password\n", ""),
            "SF25",
        ),
        (
            "gr-os-noscenario",
            "### Requirement: Bare\nThe system SHALL exist.\n".to_string(),
            "SF10",
        ),
    ];
    for (name, body, check) in cases {
        let p = feature(name, &format!("{SCOPE}{body}"));
        let got = grammar(&p);
        assert!(
            got.iter().any(|m| m.starts_with(check)),
            "{name}: want {check}, got {got:?}"
        );
    }
}

#[test]
fn zero_requirements_are_reported() {
    let p = feature("gr-zero", SCOPE);
    assert_eq!(
        grammar(&p),
        [
            "SF24 features/auth/product.md: no '### Requirement:' blocks — the requirement checks examined nothing"
        ]
    );
    let empty = feature(
        "gr-empty-title",
        &format!("{SCOPE}### Requirement:\nUsers MUST x.\n#### Scenario: a\n- WHEN x\n- THEN y\n"),
    );
    assert!(
        grammar(&empty).contains(
            &"SF24 features/auth/product.md: requirement with an empty title".to_string()
        )
    );
}

#[test]
fn fenced_examples_do_not_count() {
    let p = feature(
        "gr-fenced",
        &format!(
            "{SCOPE}```markdown\n### Requirement: Example only\nThe system SHALL x.\n#### Scenario: e\n- WHEN a\n```\n"
        ),
    );
    let got = grammar(&p);
    assert!(
        got.iter()
            .any(|m| m.starts_with("SF24") && m.contains("no '### Requirement:' blocks")),
        "{got:?}"
    );
    assert!(!got.iter().any(|m| m.starts_with("SF25")), "{got:?}");
}
