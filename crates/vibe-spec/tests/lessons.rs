#![allow(clippy::unwrap_used, clippy::expect_used)]
//! `vibe-spec lessons-for`: legacy formats match `lessons-for.sh`; `--json`
//! answers Spec JSON v1.
mod common;

use common::{project, spec, write};
use serde_json::{Value, json};

const LESSONS: &str = "# Lessons\n\nIntro text.\n\n\
### Quote every expansion\n**Pattern:** Seen once.\n**Rule:** Write \"$var\".\n**Tags:** bash, quoting\n**Date:** 2026-01-02\n\n\
### Pinned rule\n**Rule:** Read lessons first.\n**Tags:** spec\n**Date:** someday\n**Pinned-by:** review\n\n\
### No rule here\n**Tags:** bash\n";

#[cfg(unix)]
#[test]
fn legacy_formats_match_the_oracle() {
    use common::assert_parity;
    let p = project("lessons-legacy");
    write(&p, ".spec/lessons.md", LESSONS);
    for args in [
        &["bash"][..],
        &["BASH", "spec"],
        &["--format", "inject", "spec"],
        &["--format=markdown", "quoting"],
        &["json", "spec"],
        &["--format", "xml", "bash"],
        &["nomatch"],
        &[""],
        &["--bogus"],
        &["--format"],
        &["--"],
        &[],
    ] {
        assert_parity(&p, "lessons-for", "lessons-for.sh", args);
    }
    let none = project("lessons-none");
    assert_parity(&none, "lessons-for", "lessons-for.sh", &[]);
    assert_parity(&none, "lessons-for", "lessons-for.sh", &["bash"]);
}

#[test]
fn json_answers_spec_json_v1() {
    let p = project("lessons-json");
    write(&p, ".spec/lessons.md", LESSONS);
    let out = spec(&p, &["lessons-for", "--json", "bash", "--tag", "spec"], &[]);
    assert_eq!(out.code, 0, "{}", out.stderr);
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(
        v,
        json!({
            "v": 1,
            "tags": ["bash", "spec"],
            "lessons": [
                {
                    "title": "Quote every expansion",
                    "rule": "Write \"$var\".",
                    "tags": ["bash", "quoting"],
                    "pattern": "Seen once.",
                    "date": "2026-01-02"
                },
                { "title": "Pinned rule", "rule": "Read lessons first.", "tags": ["spec"], "pinned": true }
            ]
        })
    );
    #[cfg(unix)]
    common::assert_spec_json("lessons-for", out.text().trim_end());
}

#[test]
fn json_exit_codes_follow_the_contract() {
    let p = project("lessons-json-codes");
    std::fs::create_dir_all(p.join(".spec")).unwrap();
    let empty = spec(&p, &["lessons-for", "--json", "bash"], &[]);
    assert_eq!(empty.code, 0);
    assert_eq!(
        empty.text(),
        "{\"lessons\":[],\"tags\":[\"bash\"],\"v\":1}\n"
    );
    assert_eq!(spec(&p, &["lessons-for", "--json"], &[]).code, 2);
    let none = project("lessons-json-none");
    assert_eq!(spec(&none, &["lessons-for", "--json", "bash"], &[]).code, 3);
    assert_eq!(
        spec(&none, &["lessons-for", "bash"], &[]).code,
        0,
        "legacy no-root stays exit 0"
    );
}

#[test]
fn tags_match_literally_not_as_regex() {
    let p = project("lessons-literal");
    write(&p, ".spec/lessons.md", LESSONS);
    let out = spec(&p, &["lessons-for", "b.sh"], &[]);
    assert_eq!(out.code, 0);
    assert!(
        out.stdout.is_empty(),
        "b.sh must not match bash: {}",
        out.text()
    );
}

#[test]
fn json_mode_rejects_an_empty_tag() {
    let p = project("lessons-json-empty-tag");
    write(&p, ".spec/lessons.md", LESSONS);
    for args in [
        &["lessons-for", "--json", ""][..],
        &["lessons-for", "--tag", ""],
        &["lessons-for", "--json", "bash", "--tag", ""],
    ] {
        let out = spec(&p, args, &[]);
        assert_eq!(out.code, 2, "{args:?}: {}", out.text());
        assert!(out.stdout.is_empty(), "{args:?}: {}", out.text());
        assert!(out.stderr.contains("empty"), "{args:?}: {}", out.stderr);
    }
}
