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
    let budget = format!(
        "---\ntype: concept\nupdated: 2026-01-15\n---\n{}last line, no newline",
        "x\n".repeat(56)
    );
    std::fs::write(p.join("docs/spec/concepts/invoice.md"), budget).unwrap();
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
