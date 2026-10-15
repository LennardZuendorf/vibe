#![cfg(unix)]
#![allow(clippy::unwrap_used, clippy::expect_used)]
//! The layered layout (D44): detection and SF22/SF30-SF34 (P3b-R10, R11).
mod common;

use common::{layered_tree, spec, validate_json, write};
use std::path::Path;

fn findings(p: &Path) -> Vec<(String, String)> {
    let (_, v) = validate_json(p, &[], &[]);
    v["findings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| {
            (
                f["check"].as_str().unwrap().to_string(),
                f["message"].as_str().unwrap().to_string(),
            )
        })
        .collect()
}

fn append(p: &Path, rel: &str, text: &str) {
    let path = p.join(rel);
    let mut body = std::fs::read_to_string(&path).unwrap();
    body.push_str(text);
    std::fs::write(path, body).unwrap();
}

#[test]
fn a_hub_typed_root_is_layered() {
    let p = layered_tree("ly-detect");
    let (code, v) = validate_json(&p, &[], &[]);
    assert_eq!(v["layout"], "layered");
    assert_eq!(v["findings"], serde_json::json!([]), "{v:#}");
    assert_eq!(code, 0);
    assert!(v["examined"].as_u64().unwrap() >= 16, "{v:#}");
    let root = spec(&p, &["root", "--json"], &[]);
    assert!(
        root.text().contains("\"layout\":\"layered\""),
        "{}",
        root.text()
    );
    let text = spec(&p, &["validate"], &[]).text();
    assert!(
        !text.contains("--- README.md ---") && !text.contains("--- AGENTS.md ---"),
        "{text}"
    );
}

type Case = (&'static str, &'static str, &'static str, fn(&Path));

#[test]
fn layout_rules_fail_when_broken() {
    let cases: [Case; 7] = [
        (
            "ly-other",
            "SF30",
            "notes.txt: not in the layered layout",
            |p| write(p, "docs/spec/notes.txt", "x\n"),
        ),
        (
            "ly-stray",
            "SF30",
            "random/x.md: not in the layered layout",
            |p| write(p, "docs/spec/random/x.md", "x\n"),
        ),
        (
            "ly-note-budget",
            "SF31",
            "flows/checkout.md: 68 lines, over the 60-line note budget",
            |p| append(p, "docs/spec/flows/checkout.md", &"x\n".repeat(62)),
        ),
        (
            "ly-hub-budget",
            "SF31",
            "over the 80-line hub budget",
            |p| append(p, "docs/spec/tech.md", &"x\n".repeat(80)),
        ),
        (
            "ly-kebab",
            "SF32",
            "concepts/Invoice.md: 'Invoice.md' is not lowercase-kebab",
            |p| {
                std::fs::rename(
                    p.join("docs/spec/concepts/invoice.md"),
                    p.join("docs/spec/concepts/Invoice.md"),
                )
                .unwrap();
                let hub = std::fs::read_to_string(p.join("docs/spec/product.md"))
                    .unwrap()
                    .replace("concepts/invoice.md", "concepts/Invoice.md");
                std::fs::write(p.join("docs/spec/product.md"), hub).unwrap();
            },
        ),
        (
            "ly-unique",
            "SF32",
            "flows/invoice.md: name 'invoice' is also used by concepts/invoice.md",
            |p| {
                write(
                    p,
                    "docs/spec/flows/invoice.md",
                    "---\ntype: flow\nupdated: 2026-01-15\n---\n\nPaying one.\n",
                );
                append(p, "docs/spec/product.md", "- [Pay](flows/invoice.md)\n");
            },
        ),
        (
            "ly-link",
            "SF22",
            "flows/checkout.md: broken link to 'gone.md'",
            |p| {
                append(
                    p,
                    "docs/spec/flows/checkout.md",
                    "[x](gone.md) [ok](#top) [web](https://x.io) [t]({x}.md)\n",
                )
            },
        ),
    ];
    for (name, check, needle, mutate) in cases {
        let p = layered_tree(name);
        mutate(&p);
        let got = findings(&p);
        assert!(
            got.iter().any(|(c, m)| c == check && m.contains(needle)),
            "{name}: want {check} {needle:?}\n{got:#?}"
        );
    }
}

#[test]
fn layered_edge_inputs() {
    let p = layered_tree("ly-edge");
    std::fs::write(
        p.join("docs/spec/flows/checkout.md"),
        "---\r\ntype: flow\r\nupdated: 2026-01-15\r\n---\r\n\r\nCart.\r\n",
    )
    .unwrap();
    write(&p, "docs/spec/areas/.DS_Store", "binary");
    // 60 newlines plus an unterminated tail: `wc -l` says 60, so no SF31.
    let body = |n: usize| {
        format!(
            "---\ntype: concept\nupdated: 2026-01-15\n---\n{}last line, no newline",
            "x\n".repeat(n)
        )
    };
    std::fs::write(p.join("docs/spec/concepts/invoice.md"), body(56)).unwrap();
    assert!(
        !findings(&p).iter().any(|(c, _)| c == "SF31"),
        "60 newlines is within budget"
    );
    std::fs::write(p.join("docs/spec/concepts/invoice.md"), body(57)).unwrap();
    let got = findings(&p);
    assert!(
        got.iter()
            .any(|(c, m)| c == "SF31" && m.starts_with("concepts/invoice.md: 61 lines")),
        "{got:#?}"
    );
    assert!(
        !got.iter().any(|(c, _)| c == "SF30"),
        "dotfile must be ignored: {got:#?}"
    );
    assert!(
        !got.iter().any(|(_, m)| m.contains("flows/checkout.md")),
        "CRLF note is fine: {got:#?}"
    );
}

#[test]
fn a_flat_root_stays_flat() {
    let p = common::project("ly-flat");
    write(
        &p,
        ".spec/product.md",
        "---\ntype: entrypoint\nscope: product\nchildren:\nupdated: 2026-01-15\n---\n",
    );
    write(&p, ".spec/notes.txt", "x\n");
    let (_, v) = validate_json(&p, &[], &[]);
    assert_eq!(v["layout"], "flat");
    assert!(
        v["findings"]
            .as_array()
            .unwrap()
            .iter()
            .all(|f| f["check"] != "SF30")
    );
}

#[test]
fn layered_runs_the_shared_adherence_checks() {
    // SF26 runs on layered notes (type vocabulary), SF28 on features.
    let p = layered_tree("ly-adherence");
    write(
        &p,
        "docs/spec/flows/checkout.md",
        "---\ntype: bogus\nupdated: 2026-01-15\n---\n\nCart.\n",
    );
    write(
        &p,
        "docs/spec/features/billing/product.md",
        "---\ntype: feature-product\nupdated: 2026-01-15\n---\n\n# P\n",
    );
    let got = findings(&p);
    assert!(
        got.iter()
            .any(|(c, m)| c == "SF26" && m.contains("flows/checkout.md: type 'bogus'")),
        "{got:#?}"
    );
    assert!(
        got.iter()
            .any(|(c, m)| c == "SF28" && m.contains("features/billing/")),
        "{got:#?}"
    );
}

#[test]
fn layered_feature_docs_keep_type_checking() {
    // Legacy SF20 still runs on layered features, so SF26 skips the four
    // legacy-typed docs; research.md is only SF26's.
    let p = layered_tree("ly-feature-types");
    write(
        &p,
        "docs/spec/features/checkout/product.md",
        "---\ntype: wrong\nupdated: 2026-01-15\n---\n\n# P\n",
    );
    write(
        &p,
        "docs/spec/features/checkout/research.md",
        "---\ntype: wrong\nupdated: 2026-01-15\n---\n\n# R\n",
    );
    let got = findings(&p);
    assert!(
        got.iter().any(|(c, m)| c == "SF20"
            && m.contains("features/checkout/product.md: expected 'type: feature-product'")),
        "{got:#?}"
    );
    assert!(
        got.iter()
            .any(|(c, m)| c == "SF26" && m.contains("features/checkout/research.md: type 'wrong'")),
        "{got:#?}"
    );
    assert!(
        !got.iter()
            .any(|(c, m)| c == "SF26" && m.contains("features/checkout/product.md")),
        "legacy-typed docs stay SF20's: {got:#?}"
    );
}

#[test]
fn part_hubs_and_nested_agents_are_exempt_from_name_uniqueness() {
    let p = layered_tree("ly-fix-names");
    assert!(findings(&p).is_empty(), "two parts must stay clean");
    write(&p, "docs/spec/areas/AGENTS.md", "# Areas\n");
    write(&p, "docs/spec/flows/AGENTS.md", &"x\n".repeat(70));
    let got = findings(&p);
    assert!(
        !got.iter().any(|(c, _)| c == "SF32" || c == "SF31"),
        "nested AGENTS.md is exempt: {got:#?}"
    );
}

#[test]
fn note_names_are_unique_across_parts() {
    // D44: names are unique across the whole tree, parts included.
    let p = layered_tree("ly-part-dup");
    let note = "---\ntype: concept\nupdated: 2026-01-15\n---\n\nA bill.\n";
    write(&p, "docs/spec/parts/web/concepts/invoice.md", note);
    append(
        &p,
        "docs/spec/parts/web/product.md",
        "- [Invoice](concepts/invoice.md)\n",
    );
    let got = findings(&p);
    assert!(
        got.iter()
            .any(|(c, m)| c == "SF32" && m.contains("name 'invoice' is also used by")),
        "{got:#?}"
    );
}

#[test]
fn other_root_files_are_left_to_sf30() {
    for (name, body) in [
        (
            "ly-gloss-fm",
            "---\ntype: concept\nupdated: 2026-01-15\n---\n\nWords.\n",
        ),
        ("ly-gloss-plain", "Words.\n"),
    ] {
        let p = layered_tree(name);
        write(&p, "docs/spec/glossary.md", body);
        let (code, v) = validate_json(&p, &[], &[]);
        let got: Vec<(&str, &str)> = v["findings"]
            .as_array()
            .unwrap()
            .iter()
            .map(|f| (f["check"].as_str().unwrap(), f["message"].as_str().unwrap()))
            .collect();
        assert!(got.iter().all(|(c, _)| *c == "SF30"), "{name}: {got:#?}");
        assert_eq!(got.len(), 1, "{name}: {got:#?}");
        assert_eq!(code, 0, "{name}");
    }
}

#[test]
fn a_part_folder_root_raises_no_sf21() {
    let p = layered_tree("ly-subroot");
    let out = spec(&p, &["--root", "docs/spec/areas/billing", "validate"], &[]);
    assert!(
        out.text().contains("Validating"),
        "{}{}",
        out.text(),
        out.stderr
    );
    assert!(!out.text().contains("SF21"), "{}", out.text());
    assert!(!out.text().contains("must start with"), "{}", out.text());
}

#[test]
fn link_targets_are_cleaned_before_resolving() {
    let p = layered_tree("ly-linkforms");
    append(
        &p,
        "docs/spec/flows/checkout.md",
        "[a](../concepts/invoice.md?x=1) [b](../concepts/invoice.md \"Title\") [c](../concepts/invoice.md 'Title') [d](<../concepts/invoice.md>) [e]( ../concepts/invoice.md ) [f](<../concepts/invoice.md> \"T\")\n",
    );
    let got = findings(&p);
    assert!(got.is_empty(), "{got:#?}");
    append(
        &p,
        "docs/spec/flows/checkout.md",
        "[g](gone.md \"Title\")\n",
    );
    let got = findings(&p);
    assert!(
        got.iter()
            .any(|(c, m)| c == "SF22" && m.contains("broken link to 'gone.md'")),
        "{got:#?}"
    );
}

#[test]
fn typed_notes_and_reachability_fail_when_broken() {
    let cases: [Case; 8] = [
        (
            "ly-type",
            "SF33",
            "platform/errors.md: type 'flow' does not match its folder (expected 'platform')",
            |p| {
                write(
                    p,
                    "docs/spec/platform/errors.md",
                    "---\ntype: flow\nupdated: 2026-01-15\n---\n\nTyped.\n",
                )
            },
        ),
        (
            "ly-notype",
            "SF33",
            "questions/pricing.md: missing 'type:' (expected 'question')",
            |p| {
                write(
                    p,
                    "docs/spec/questions/pricing.md",
                    "---\nupdated: 2026-01-15\n---\n\nMonthly?\n",
                )
            },
        ),
        (
            "ly-noupdated",
            "SF33",
            "decisions/d01-use-rust.md: missing 'updated:'",
            |p| {
                write(
                    p,
                    "docs/spec/decisions/d01-use-rust.md",
                    "---\ntype: decision\n---\n\nRust.\n",
                )
            },
        ),
        (
            "ly-nofm",
            "SF33",
            "concepts/invoice.md: no frontmatter",
            |p| write(p, "docs/spec/concepts/invoice.md", "A bill.\n"),
        ),
        (
            "ly-code",
            "SF33",
            "areas/billing/invoices.md: code path 'src/gone.rs' does not exist",
            |p| {
                write(
                    p,
                    "docs/spec/areas/billing/invoices.md",
                    "---\ntype: area\ncode: [src/billing.rs, src/gone.rs]\nupdated: 2026-01-15\n---\n\nMonthly.\n",
                )
            },
        ),
        (
            "ly-nocode",
            "SF33",
            "parts/mobile/areas/screens.md: an area needs 'code:'",
            |p| {
                write(
                    p,
                    "docs/spec/parts/mobile/areas/screens.md",
                    "---\ntype: area\nupdated: 2026-01-15\n---\n\nTwo.\n",
                )
            },
        ),
        (
            "ly-applies",
            "SF33",
            "tech/rust.md: a tech note needs 'applies:'",
            |p| {
                write(
                    p,
                    "docs/spec/tech/rust.md",
                    "---\ntype: tech\nupdated: 2026-01-15\n---\n\nEdition 2024.\n",
                )
            },
        ),
        (
            "ly-orphan",
            "SF34",
            "flows/refund.md: not reachable from a hub",
            |p| {
                write(
                    p,
                    "docs/spec/flows/refund.md",
                    "---\ntype: flow\nupdated: 2026-01-15\n---\n\nMoney back.\n",
                )
            },
        ),
    ];
    for (name, check, needle, mutate) in cases {
        let p = layered_tree(name);
        mutate(&p);
        let got = findings(&p);
        assert!(
            got.iter().any(|(c, m)| c == check && m.contains(needle)),
            "{name}: want {check} {needle:?}\n{got:#?}"
        );
    }
}

fn only(p: &Path, check: &str, needle: &str) {
    let got = findings(p);
    assert_eq!(got.len(), 1, "exactly one finding: {got:#?}");
    assert!(
        got[0].0 == check && got[0].1.contains(needle),
        "want {check} {needle:?}: {got:#?}"
    );
}

#[test]
fn a_hub_with_a_wrong_vocabulary_type_is_sf26_only() {
    let p = layered_tree("ly-hubtype-vocab");
    write(
        &p,
        "docs/spec/design.md",
        "---\ntype: entrypoint\nscope: design\nchildren:\nupdated: 2026-01-15\n---\n\n# Design\n",
    );
    only(&p, "SF26", "design.md: type 'entrypoint' is not one of");
}

#[test]
fn a_hub_with_a_note_type_is_sf33_only() {
    let p = layered_tree("ly-hubtype-area");
    write(
        &p,
        "docs/spec/design.md",
        "---\ntype: area\nupdated: 2026-01-15\n---\n\n# Design\n",
    );
    only(&p, "SF33", "design.md: a hub declares 'type: hub'");
}

#[test]
fn a_root_hub_without_type_is_sf20_only() {
    let p = layered_tree("ly-hubtype-none");
    write(
        &p,
        "docs/spec/design.md",
        "---\nupdated: 2026-01-15\n---\n\n# Design\n",
    );
    only(&p, "SF20", "design.md: missing 'type:' in frontmatter");
}

#[test]
fn a_part_hub_without_type_stays_sf33() {
    let p = layered_tree("ly-parthub-none");
    write(
        &p,
        "docs/spec/parts/mobile/product.md",
        "---\nupdated: 2026-01-15\n---\n\n# Mobile\n\n- [Screens](areas/screens.md)\n",
    );
    only(
        &p,
        "SF33",
        "parts/mobile/product.md: a hub declares 'type: hub'",
    );
}

#[test]
fn a_note_with_a_blank_updated_is_sf26_only() {
    let p = layered_tree("ly-updated-blank");
    write(
        &p,
        "docs/spec/flows/checkout.md",
        "---\ntype: flow\nupdated:\n---\n\nCart.\n",
    );
    only(&p, "SF26", "flows/checkout.md");
}

#[test]
fn reachability_follows_links_through_folder_hubs() {
    let p = layered_tree("ly-reach");
    let hub = std::fs::read_to_string(p.join("docs/spec/product.md"))
        .unwrap()
        .replace("- [Billing](areas/billing/README.md)\n", "");
    std::fs::write(p.join("docs/spec/product.md"), hub).unwrap();
    let got: Vec<String> = findings(&p)
        .into_iter()
        .filter(|(c, _)| c == "SF34")
        .map(|(_, m)| m)
        .collect();
    assert_eq!(
        got,
        [
            "areas/billing/README.md: not reachable from a hub (link it from a hub or its folder README.md)",
            "areas/billing/invoices.md: not reachable from a hub (link it from a hub or its folder README.md)",
        ]
    );
}

#[test]
fn grammar_runs_on_notes() {
    let p = layered_tree("ly-grammar");
    write(
        &p,
        "docs/spec/flows/checkout.md",
        "---\ntype: flow\nupdated: 2026-01-15\n---\n\n### Requirement: Pay\nThe system SHALL charge.\n\n#### Scenario: ok\n- WHEN paying\n",
    );
    let got = findings(&p);
    assert!(got.iter().any(|(c, m)| c == "SF25" && m == "flows/checkout.md: scenario 'ok' has no THEN bullet"), "{got:#?}");
}

fn of(p: &Path, checks: &[&str]) -> Vec<String> {
    findings(p)
        .into_iter()
        .filter(|(c, _)| checks.contains(&c.as_str()))
        .map(|(c, m)| format!("{c} {m}"))
        .collect()
}

const NOTE: &str = "---\ntype: flow\nupdated: 2026-01-15\n---\n\nx\n";

#[test]
fn reach_follows_directory_links() {
    let p = layered_tree("ly-dirlinks");
    let hub = std::fs::read_to_string(p.join("docs/spec/product.md"))
        .unwrap()
        .replace("areas/billing/README.md", "areas/billing/")
        .replace("- [Checkout](flows/checkout.md)\n", "- [Flows](flows)\n");
    std::fs::write(p.join("docs/spec/product.md"), hub).unwrap();
    write(
        &p,
        "docs/spec/flows/README.md",
        "---\ntype: hub\nupdated: 2026-01-15\n---\n\n# Flows\n\n- [Checkout](checkout.md)\n",
    );
    assert_eq!(of(&p, &["SF34"]), Vec::<String>::new());
}

#[test]
fn nested_agents_files_are_exempt_from_sf33_and_sf34() {
    let p = layered_tree("ly-agents");
    for rel in [
        "parts/mobile/AGENTS.md",
        "areas/AGENTS.md",
        "areas/billing/AGENTS.md",
    ] {
        write(&p, &format!("docs/spec/{rel}"), "# Rules\n\nPlain.\n");
    }
    assert_eq!(of(&p, &["SF33", "SF34"]), Vec::<String>::new());
}

#[test]
fn sf33_leaves_vocabulary_problems_to_sf26() {
    let p = layered_tree("ly-vocab");
    write(
        &p,
        "docs/spec/flows/refund.md",
        "---\ntype: widget\nupdated: 2026-01-15\n---\n\nx\n",
    );
    write(
        &p,
        "docs/spec/flows/blank.md",
        "---\ntype:\nupdated: 2026-01-15\n---\n\nx\n",
    );
    let got = of(&p, &["SF26", "SF33"]);
    assert!(
        got.iter()
            .all(|m| m.starts_with("SF26 ") || m.contains("does not match")),
        "{got:#?}"
    );
    assert!(!got.iter().any(|m| m.starts_with("SF33")), "{got:#?}");
    assert!(
        got.iter().any(|m| m.contains("flows/refund.md")),
        "{got:#?}"
    );
    assert!(got.iter().any(|m| m.contains("flows/blank.md")), "{got:#?}");
    write(
        &p,
        "docs/spec/flows/mixed.md",
        "---\ntype: concept\nupdated: 2026-01-15\n---\n\nx\n",
    );
    let got = of(&p, &["SF26", "SF33"]);
    assert!(
        got.iter()
            .any(|m| m.starts_with("SF33 flows/mixed.md: type 'concept'")),
        "{got:#?}"
    );
    assert!(
        !got.iter()
            .any(|m| m.starts_with("SF26") && m.contains("mixed")),
        "{got:#?}"
    );
}

#[test]
fn a_feature_type_in_a_kind_folder_is_a_mismatch() {
    let p = layered_tree("ly-feature-type");
    write(
        &p,
        "docs/spec/flows/stray.md",
        "---\ntype: feature-tech\nupdated: 2026-01-15\n---\n\nx\n",
    );
    let got = of(&p, &["SF26", "SF33"]);
    assert_eq!(
        got,
        vec![
            "SF33 flows/stray.md: type 'feature-tech' does not match its folder (expected 'flow')"
                .to_string()
        ],
        "{got:#?}"
    );
}

#[test]
fn a_root_hub_without_frontmatter_is_sf20s() {
    let p = layered_tree("ly-hubnofm");
    write(&p, "docs/spec/design.md", "# Design\n");
    assert_eq!(of(&p, &["SF33"]), Vec::<String>::new());
}

#[test]
fn code_requirement_follows_the_folder() {
    let p = layered_tree("ly-folder-code");
    write(&p, "docs/spec/areas/odd.md", NOTE);
    let got = of(&p, &["SF33"]);
    assert!(
        got.iter()
            .any(|m| m.contains("areas/odd.md: an area needs 'code:'")),
        "{got:#?}"
    );
}

#[test]
fn sf10_skips_fenced_samples_in_notes() {
    let p = layered_tree("ly-fenced");
    write(
        &p,
        "docs/spec/flows/sample.md",
        "---\ntype: flow\nupdated: 2026-01-15\n---\n\n```\n### Requirement: Sample\nnothing\n```\n",
    );
    assert_eq!(of(&p, &["SF10"]), Vec::<String>::new());
}

#[test]
fn reach_ignores_fences_survives_cycles_and_fragments() {
    let p = layered_tree("ly-reach2");
    append(
        &p,
        "docs/spec/product.md",
        "\n```\n[Refund](flows/refund.md)\n```\n",
    );
    write(&p, "docs/spec/flows/refund.md", NOTE);
    let got = of(&p, &["SF34"]);
    assert_eq!(got.len(), 1, "{got:#?}");
    assert!(got[0].contains("flows/refund.md"), "{got:#?}");

    let p = layered_tree("ly-reach3");
    append(&p, "docs/spec/product.md", "\n- [A](flows/a.md#top)\n");
    write(&p, "docs/spec/flows/a.md", &format!("{NOTE}[b](b.md)\n"));
    write(&p, "docs/spec/flows/b.md", &format!("{NOTE}[a](a.md)\n"));
    assert_eq!(of(&p, &["SF34"]), Vec::<String>::new());
}

fn sf22(p: &Path) -> Vec<String> {
    findings(p)
        .into_iter()
        .filter(|(c, _)| c == "SF22")
        .map(|(_, m)| m)
        .collect()
}

#[test]
fn a_layered_root_hub_uses_the_layered_link_checker() {
    let p = layered_tree("ly-hublinks");
    write(&p, "docs/spec/flows/a.md", NOTE);
    append(
        &p,
        "docs/spec/design.md",
        "\n```\n[example](does-not-exist.md)\n```\n\n[a](/docs/spec/flows/a.md)\n",
    );
    assert!(findings(&p).is_empty(), "{:#?}", findings(&p));
    append(&p, "docs/spec/design.md", "\n[x](missing.png)\n");
    let (code, v) = validate_json(&p, &[], &[]);
    let got: Vec<(&str, &str, &str)> = v["findings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| {
            (
                f["check"].as_str().unwrap(),
                f["severity"].as_str().unwrap(),
                f["message"].as_str().unwrap(),
            )
        })
        .collect();
    assert_eq!(
        got,
        [("SF22", "warning", "design.md: broken link to 'missing.png'")]
    );
    assert_eq!(code, 0);
}

#[test]
fn an_absolute_link_from_a_hub_reaches_the_note() {
    let p = layered_tree("ly-abs-reach");
    write(&p, "docs/spec/flows/a.md", NOTE);
    append(&p, "docs/spec/README.md", "\n[a](/docs/spec/flows/a.md)\n");
    assert!(findings(&p).is_empty(), "{:#?}", findings(&p));
}

#[test]
fn a_link_that_differs_only_in_case_is_broken() {
    let p = layered_tree("ly-case");
    write(&p, "docs/spec/flows/b.md", NOTE);
    append(&p, "docs/spec/product.md", "\n[b](flows/b.md)\n");
    append(&p, "docs/spec/flows/checkout.md", "\n[b](B.md)\n");
    assert_eq!(
        sf22(&p),
        ["flows/checkout.md: broken link to 'B.md'"],
        "{:#?}",
        findings(&p)
    );
    append(&p, "docs/spec/product.md", "\n[b](flows/B.md)\n");
    assert_eq!(sf22(&p).len(), 2, "{:#?}", findings(&p));
}

#[test]
fn inline_code_spans_hold_no_links() {
    let p = layered_tree("ly-codespan");
    append(
        &p,
        "docs/spec/flows/checkout.md",
        "\nWrite links like `[Trip](no-such-note.md)` and ``[T](x.md)``.\n",
    );
    assert!(findings(&p).is_empty(), "{:#?}", findings(&p));
    write(&p, "docs/spec/flows/c.md", NOTE);
    append(
        &p,
        "docs/spec/product.md",
        "\nOnly in code: `[C](flows/c.md)`\n",
    );
    let got = findings(&p);
    assert!(
        got.iter()
            .any(|(c, m)| c == "SF34" && m.starts_with("flows/c.md")),
        "a code-span link reaches nothing: {got:#?}"
    );
}

fn examined(p: &Path) -> u64 {
    validate_json(p, &[], &[]).1["examined"].as_u64().unwrap()
}

#[test]
fn examined_counts_lessons_in_both_layouts() {
    let lay = layered_tree("ly-examine-lessons");
    let with = examined(&lay);
    std::fs::remove_file(lay.join("docs/spec/lessons.md")).unwrap();
    assert_eq!(with, examined(&lay) + 1, "layered lessons.md");

    let flat = common::project("ly-examine-flat");
    write(
        &flat,
        ".spec/product.md",
        "---\ntype: entrypoint\nscope: product\nchildren:\nupdated: 2026-01-15\n---\n",
    );
    let without = examined(&flat);
    write(&flat, ".spec/lessons.md", "# Lessons\n");
    assert_eq!(examined(&flat), without + 1, "flat lessons.md");
}
