#![cfg(unix)]
#![allow(clippy::unwrap_used, clippy::expect_used)]
//! `.vibe/spec.json` strictness, `--strict`, allow markers (P3b-R5–R8).
mod common;

use common::{project, spec, write};

const ENTRY: &str = "---\ntype: entrypoint\nscope: product\nchildren:\nupdated: 2026-01-15\n---\n\n# P\n\n[f](features/ghost/)\n";

/// A tree whose only finding is one SF13 warning (a stale `features/ghost/` link).
fn sf13_tree(name: &str) -> std::path::PathBuf {
    let p = project(name);
    write(&p, ".spec/product.md", ENTRY);
    p
}

#[test]
fn a_legacy_check_can_be_turned_off_or_up() {
    let p = sf13_tree("str-legacy");
    let base = spec(&p, &["validate"], &[]);
    assert_eq!(base.code, 0, "{}", base.text());
    assert!(
        base.text().contains("WARN:  SF13: stale link"),
        "{}",
        base.text()
    );
    assert!(base.text().ends_with("Errors:   0\nWarnings: 1\n"));

    write(&p, ".vibe/spec.json", r#"{"strictness": {"SF-13": "off"}}"#);
    let off = spec(&p, &["validate"], &[]);
    assert_eq!(off.code, 0);
    let without: String = base
        .text()
        .lines()
        .filter(|l| !l.contains("SF13"))
        .map(|l| format!("{l}\n"))
        .collect();
    assert_eq!(off.text(), without.replace("Warnings: 1", "Warnings: 0"));

    write(
        &p,
        ".vibe/spec.json",
        r#"{"strictness": {"sf13": "error"}}"#,
    );
    let up = spec(&p, &["validate"], &[]);
    assert_eq!(up.code, 1);
    assert!(
        up.text().contains("\x1b[31m  ERROR: SF13: stale link"),
        "{}",
        up.text()
    );
    assert!(up.text().ends_with("Errors:   1\nWarnings: 0\n"));
}

#[test]
fn strict_promotes_warnings_and_keeps_off() {
    let p = sf13_tree("str-strict");
    let strict = spec(&p, &["validate", "--strict"], &[]);
    assert_eq!(strict.code, 1);
    assert!(strict.text().contains("ERROR: SF13"), "{}", strict.text());
    write(&p, ".vibe/spec.json", r#"{"strictness": {"SF13": "off"}}"#);
    let quiet = spec(&p, &["validate", "--strict"], &[]);
    assert_eq!(quiet.code, 0, "{}", quiet.text());
}

#[test]
fn an_allow_marker_beats_strict() {
    let p = sf13_tree("str-marker");
    write(
        &p,
        ".spec/product.md",
        &format!("{ENTRY}\n<!-- vibe-spec: allow SF-13 -->\n"),
    );
    write(
        &p,
        ".vibe/spec.json",
        r#"{"strictness": {"SF13": "error"}}"#,
    );
    let out = spec(&p, &["validate", "--strict"], &[]);
    assert_eq!(out.code, 0, "{}", out.text());
    assert!(!out.text().contains("SF13"), "{}", out.text());
}

#[test]
fn a_marker_only_covers_its_own_file() {
    let p = sf13_tree("str-marker-scope");
    write(
        &p,
        ".spec/tech.md",
        "---\ntype: entrypoint\nscope: technical\nchildren:\nupdated: 2026-01-15\n---\n\n<!-- vibe-spec: allow SF13 -->\n",
    );
    let out = spec(&p, &["validate"], &[]);
    assert!(
        out.text().contains("SF13: stale link in .spec/product.md"),
        "{}",
        out.text()
    );
}

#[test]
fn strict_refuses_when_every_check_is_off() {
    let p = sf13_tree("str-all-off");
    let all = r#"{"strictness": {"SF3":"off","SF4":"off","SF8":"off","SF9":"off","SF10":"off","SF11":"off","SF12":"off","SF13":"off","SF14":"off","SF15":"off","SF16":"off","I1":"off","SF20":"off","SF21":"off","SF22":"off","SF23":"off","SF24":"off","SF25":"off","SF26":"off","SF27":"off","SF28":"off","SF29":"off","SF30":"off","SF31":"off","SF32":"off","SF33":"off","SF34":"off","SF35":"off","SF36":"off"}}"#;
    write(&p, ".vibe/spec.json", all);
    let plain = spec(&p, &["validate"], &[]);
    assert_eq!(plain.code, 0);
    let strict = spec(&p, &["validate", "--strict"], &[]);
    assert_eq!(strict.code, 1, "{}", strict.text());
    assert!(
        strict
            .text()
            .contains("ERROR: validate --strict examined nothing"),
        "{}",
        strict.text()
    );
}

#[test]
fn bad_config_values_are_usage_errors() {
    let p = sf13_tree("str-bad");
    for bad in [
        r#"{"strictness": {"SF13": "loud"}}"#,
        r#"{"strictness": ["SF13"]}"#,
        r#"{"strictness": {"SF13": 1}}"#,
        "not json",
    ] {
        write(&p, ".vibe/spec.json", bad);
        let out = spec(&p, &["validate"], &[]);
        assert_eq!(out.code, 2, "{bad}: {}", out.text());
        assert!(out.stdout.is_empty(), "{bad}");
    }
}

#[test]
fn an_unknown_check_warns_and_is_ignored() {
    let p = sf13_tree("str-unknown");
    write(
        &p,
        ".vibe/spec.json",
        r#"{"strictness": {"SF99": "off", "SF13": "off"}}"#,
    );
    let out = spec(&p, &["validate"], &[]);
    assert_eq!(out.code, 0);
    assert!(
        out.stderr.contains("unknown check 'SF99'"),
        "{}",
        out.stderr
    );
    assert!(!out.text().contains("SF13"));
}

#[test]
fn an_explicit_root_still_reads_the_project_config() {
    let p = sf13_tree("str-explicit");
    write(&p, ".vibe/spec.json", r#"{"strictness": {"SF13": "off"}}"#);
    let out = spec(&p, &["--root", ".spec", "validate"], &[]);
    assert!(!out.text().contains("SF13"), "{}", out.text());
}

const SF23_OFF: &str = r#"{"strictness": {"SF23": "off"}}"#;

#[test]
fn no_spec_files_error_cannot_be_silenced() {
    let p = project("str-nofiles");
    write(&p, ".spec/notes.txt", "x");
    write(&p, ".vibe/spec.json", SF23_OFF);
    for args in [&["validate"][..], &["validate", "--strict"][..]] {
        let out = spec(&p, args, &[]);
        assert_eq!(out.code, 1, "{}", out.text());
        assert!(
            out.text().contains("ERROR: No spec files found in .spec/"),
            "{}",
            out.text()
        );
    }
}

#[test]
fn no_spec_root_error_cannot_be_silenced() {
    let p = project("str-noroot");
    write(&p, ".vibe/spec.json", SF23_OFF);
    let out = spec(&p, &["validate"], &[]);
    assert_eq!(out.code, 1, "{}", out.text());
    assert!(
        out.text()
            .contains("ERROR: No spec root found. Run setup first: vibe-spec init"),
        "{}",
        out.text()
    );
}
