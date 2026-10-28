#![allow(clippy::unwrap_used, clippy::expect_used)]
// vibe-spec: literal-file
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

#[test]
fn usage_errors_exit_2() {
    // skill-revision/13: the contract's usage code, messages unchanged.
    let p = project("lessons-usage");
    write(&p, ".spec/lessons.md", LESSONS);
    for (args, msg) in [
        (&["lessons-for", "--bogus"][..], "Unknown option: --bogus"),
        (&["lessons-for", "--"], "Unknown option: --"),
        (&["lessons-for"], "usage: vibe-spec lessons-for"),
        (&["lessons-for", "--format"], "--format requires a value"),
    ] {
        let out = spec(&p, args, &[]);
        assert_eq!(out.code, 2, "{args:?}: {}", out.stderr);
        assert!(out.stdout.is_empty(), "{args:?}: {}", out.text());
        assert!(out.stderr.contains(msg), "{args:?}: {}", out.stderr);
    }
    // Controls: no match is still success.
    let out = spec(&p, &["lessons-for", "nomatch"], &[]);
    assert_eq!(out.code, 0, "{}", out.stderr);
}

#[test]
fn a_missing_root_or_lessons_file_is_named_on_stderr() {
    // skill-revision/14
    let none = project("lessons-noroot");
    let out = spec(&none, &["lessons-for", "bash"], &[]);
    assert_eq!(out.code, 0, "{}", out.stderr);
    assert!(out.stdout.is_empty());
    assert_eq!(out.stderr.trim_end().lines().count(), 1, "{}", out.stderr);
    assert!(out.stderr.contains("no spec root"), "{}", out.stderr);
    assert!(!out.stderr.contains(".spec"), "{}", out.stderr);
    let bare = project("lessons-nofile");
    write(&bare, "docs/spec/product.md", "# P\n");
    let out = spec(&bare, &["lessons-for", "bash"], &[]);
    assert_eq!(out.code, 0, "{}", out.stderr);
    assert!(out.stdout.is_empty());
    assert_eq!(out.stderr.trim_end().lines().count(), 1, "{}", out.stderr);
    assert!(out.stderr.contains("lessons.md"), "{}", out.stderr);
}

#[test]
fn tags_match_whole_tags_only() {
    // skill-revision/15
    let p = project("lessons-whole");
    write(
        &p,
        ".spec/lessons.md",
        "# Lessons\n\n### Plain\n**Rule:** A.\n**Tags:** decision\n\n### Short\n**Rule:** B.\n**Tags:** ci\n",
    );
    let out = spec(&p, &["lessons-for", "ci"], &[]);
    assert_eq!(out.code, 0, "{}", out.stderr);
    assert!(out.text().contains("### Short"), "{}", out.text());
    assert!(!out.text().contains("### Plain"), "{}", out.text());
    let json = spec(&p, &["lessons-for", "--json", "ci"], &[]);
    let v: Value = serde_json::from_slice(&json.stdout).unwrap();
    assert_eq!(v["lessons"].as_array().unwrap().len(), 1, "{}", json.text());
}

#[test]
fn tags_ignore_case_spaces_and_crlf() {
    // skill-revision/15 (Review Focus 2)
    let p = project("lessons-crlf");
    write(
        &p,
        ".spec/lessons.md",
        "# Lessons\r\n\r\n### Mixed\r\n**Rule:** A.\r\n**Tags:** CI , Decision\r\n",
    );
    let hit = spec(&p, &["lessons-for", "ci"], &[]);
    assert!(hit.text().contains("### Mixed"), "{}", hit.text());
    let hit = spec(&p, &["lessons-for", "decision"], &[]);
    assert!(hit.text().contains("### Mixed"), "{}", hit.text());
    let miss = spec(&p, &["lessons-for", "de"], &[]);
    assert!(miss.stdout.is_empty(), "{}", miss.text());
    // Control: an empty tag still matches every lesson.
    let all = spec(&p, &["lessons-for", ""], &[]);
    assert!(all.text().contains("### Mixed"), "{}", all.text());
}
