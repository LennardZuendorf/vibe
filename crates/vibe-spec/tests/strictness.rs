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

use common::{assert_spec_json, validate_json};
use std::path::PathBuf;

/// A project that makes check `id` report at least one finding whose
/// message contains `needle`, run with `args` and `env`.
struct Trigger {
    project: PathBuf,
    env: Vec<(String, String)>,
    args: Vec<String>,
    needle: &'static str,
}

fn plain(project: PathBuf, needle: &'static str) -> Option<Trigger> {
    Some(Trigger {
        project,
        env: Vec::new(),
        args: Vec::new(),
        needle,
    })
}

const FEATURE_PRODUCT: &str = "---\ntype: feature-product\nfeature: auth\nparent: ../../product.md\nsibling: tech.md\nupdated: 2026-01-15\n---\n\n## Scope\n\n| Owns | Does not own |\n|---|---|\n| sessions | billing |\n\n### Requirement: Sign in (R1)\nUsers MUST sign in.\n\n#### Scenario: ok\n- **WHEN** they log in\n- **THEN** they are in\n";
const FEATURE_TECH: &str = "---\ntype: feature-tech\nfeature: auth\nparent: ../../tech.md\nsibling: product.md\nupdated: 2026-01-15\n---\n\n# auth — Tech\n";

/// A clean flat tree with one feature; triggers add the one defect they need.
fn base(name: &str) -> PathBuf {
    let p = project(name);
    write(
        &p,
        ".spec/product.md",
        "---\ntype: entrypoint\nscope: product\nchildren:\nupdated: 2026-01-15\n---\n\n# P\n",
    );
    write(
        &p,
        ".spec/plan.md",
        "---\ntype: entrypoint\nscope: implementation\nchildren:\nupdated: 2026-01-15\n---\n\n| Feature | Status |\n|---|---|\n| auth | IN PROGRESS |\n",
    );
    write(&p, ".spec/features/auth/product.md", FEATURE_PRODUCT);
    write(&p, ".spec/features/auth/tech.md", FEATURE_TECH);
    p
}

fn trigger(id: &str) -> Option<Trigger> {
    let p = base(&format!("mx-{}", id.to_ascii_lowercase()));
    match id {
        "SF3" => {
            write(
                &p,
                ".spec/design.md",
                "---\ntype: entrypoint\nscope: design\nchildren:\ncolors:\nupdated: 2026-01-15\n---\n",
            );
            plain(p, "empty token group")
        }
        "SF4" => {
            write(
                &p,
                ".spec/design.md",
                "---\ntype: entrypoint\nscope: design\nchildren:\nupdated: 2026-01-15\n---\n",
            );
            let empty = p.join("emptybin");
            std::fs::create_dir_all(&empty).unwrap();
            Some(Trigger {
                project: p,
                env: vec![
                    ("VIBE_DESIGN_LINT".into(), "1".into()),
                    ("PATH".into(), empty.display().to_string()),
                ],
                args: Vec::new(),
                needle: "lint skipped",
            })
        }
        "SF8" => {
            write(
                &p,
                ".spec/features/auth/product.md",
                &FEATURE_PRODUCT.replace("## Scope", "## Notes"),
            );
            plain(p, "missing '## Scope'")
        }
        "SF9" => {
            write(
                &p,
                ".spec/features/auth/tech.md",
                &FEATURE_TECH.replace("sibling: product.md\n", ""),
            );
            plain(p, "missing 'sibling:'")
        }
        "SF24" => {
            write(
                &p,
                ".spec/features/auth/product.md",
                FEATURE_PRODUCT.split("### Requirement").next().unwrap(),
            );
            plain(p, "no '### Requirement:' blocks")
        }
        "SF25" => {
            write(
                &p,
                ".spec/features/auth/product.md",
                &FEATURE_PRODUCT.replace("- **THEN** they are in\n", ""),
            );
            plain(p, "has no THEN bullet")
        }
        "SF10" => {
            write(
                &p,
                ".spec/features/auth/product.md",
                &FEATURE_PRODUCT.replace("Users MUST sign in.", "Users sign in."),
            );
            plain(p, "missing RFC-2119 keyword")
        }
        "SF11" => {
            write(
                &p,
                ".spec/features/auth/plan.md",
                "---\ntype: feature-plan\nfeature: auth\nparent: ../../plan.md\nupdated: 2026-01-15\n---\n\nNo units. Covers R1.\n",
            );
            plain(p, "plan missing '### <feature>/<n>'")
        }
        "SF12" => {
            write(
                &p,
                ".spec/features/auth/plan.md",
                "---\ntype: feature-plan\nfeature: auth\nparent: ../../plan.md\nupdated: 2026-01-15\n---\n\n## Requirements Trace\n\n### auth/1 — a\n\n### auth/1 — b\n\nCovers R1.\n",
            );
            plain(p, "duplicate unit heading ID")
        }
        "SF13" => {
            write(
                &p,
                ".spec/product.md",
                "---\ntype: entrypoint\nscope: product\nchildren:\nupdated: 2026-01-15\n---\n\n[g](features/ghost/)\n",
            );
            plain(p, "SF13: stale link")
        }
        "SF14" => {
            write(
                &p,
                ".spec/features/billing/product.md",
                &FEATURE_PRODUCT.replace("feature: auth", "feature: billing"),
            );
            write(
                &p,
                ".spec/features/billing/tech.md",
                &FEATURE_TECH.replace("feature: auth", "feature: billing"),
            );
            plain(p, "SF14: scope conflict")
        }
        "SF15" => {
            write(
                &p,
                ".spec/product.md",
                &format!(
                    "---\ntype: entrypoint\nscope: product\nchildren:\nupdated: 2026-01-15\n---\n{}",
                    "x\n".repeat(301)
                ),
            );
            plain(p, "SF15:")
        }
        "SF16" => {
            write(
                &p,
                ".spec/lessons.md",
                "# Lessons\n\n### Untagged\n**Rule:** x\n",
            );
            plain(p, "has no **Tags:** line")
        }
        "I1" => {
            write(
                &p,
                ".spec/features/auth/plan.md",
                "---\ntype: feature-plan\nfeature: auth\nparent: ../../plan.md\nupdated: 2026-01-15\n---\n\n## Requirements Trace\n\nCovers R1.\n\n### auth/1 — a\n\nDepends on billing/2.\n",
            );
            plain(p, "cross-feature unit dependency")
        }
        "SF20" => {
            write(&p, ".spec/tech.md", "# no frontmatter\n");
            plain(p, "missing YAML frontmatter")
        }
        "SF21" => {
            write(
                &p,
                ".spec/notes.md",
                "---\ntype: entrypoint\nupdated: 2026-01-15\nchildren:\n---\n",
            );
            plain(p, "must start with")
        }
        "SF22" => {
            write(
                &p,
                ".spec/product.md",
                "---\ntype: entrypoint\nscope: product\nchildren:\nupdated: 2026-01-15\n---\n\n[x](missing.md)\n",
            );
            plain(p, "broken link to 'missing.md'")
        }
        "SF23" => {
            std::fs::remove_file(p.join(".spec/features/auth/tech.md")).unwrap();
            plain(p, "missing required 'tech.md'")
        }
        "SF26" => {
            write(
                &p,
                ".spec/product.md",
                "---\ntype: entry\nscope: product\nchildren:\nupdated: 2026-01-15\n---\n\n# P\n",
            );
            plain(p, "type 'entry' is not one of")
        }
        "SF27" => {
            write(
                &p,
                ".spec/tech.md",
                "---\ntype: entrypoint\nscope: technical\nchildren: [tech-api.md]\nupdated: 2026-01-15\n---\n",
            );
            plain(p, "child 'tech-api.md' listed in frontmatter")
        }
        "SF29" => {
            common::git(&p, "2026-03-01T12:00:00Z", &["init", "-q"]);
            common::git(&p, "2026-03-01T12:00:00Z", &["add", "-A"]);
            common::git(&p, "2026-03-01T12:00:00Z", &["commit", "-q", "-m", "init"]);
            plain(p, "older than its last commit")
        }
        "SF28" => {
            write(
                &p,
                ".spec/features/billing/product.md",
                &FEATURE_PRODUCT
                    .replace("feature: auth", "feature: billing")
                    .replace("sessions", "invoices"),
            );
            write(
                &p,
                ".spec/features/billing/tech.md",
                &FEATURE_TECH.replace("feature: auth", "feature: billing"),
            );
            plain(p, "no row in plan.md names 'billing'")
        }
        "SF30" => {
            let p = common::layered_tree("mx-sf30-l");
            write(&p, "docs/spec/notes.txt", "x\n");
            plain(p, "not in the layered layout")
        }
        "SF31" => {
            let p = common::layered_tree("mx-sf31-l");
            write(
                &p,
                "docs/spec/flows/checkout.md",
                &format!(
                    "---\ntype: flow\nupdated: 2026-01-15\n---\n{}",
                    "x\n".repeat(70)
                ),
            );
            plain(p, "over the 60-line note budget")
        }
        "SF32" => {
            let p = common::layered_tree("mx-sf32-l");
            write(
                &p,
                "docs/spec/flows/invoice.md",
                "---\ntype: flow\nupdated: 2026-01-15\n---\n\nPay.\n",
            );
            plain(p, "is also used by")
        }
        _ => None,
    }
}

/// Registered checks that have no trigger yet; each later task moves its ids
/// into `trigger`. Task 13 requires this to be empty.
const PENDING: &[&str] = &["SF33", "SF34", "SF35", "SF36"];

fn registered() -> Vec<String> {
    let p = base("mx-ids");
    let (_, v) = validate_json(&p, &[], &[]);
    v["checks"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["id"].as_str().unwrap().to_string())
        .collect()
}

fn run_with(t: &Trigger, config: Option<(&str, &str)>) -> (i32, serde_json::Value) {
    match config {
        Some((id, level)) => write(
            &t.project,
            ".vibe/spec.json",
            &format!(r#"{{"strictness": {{"{id}": "{level}"}}}}"#),
        ),
        None => {
            let _ = std::fs::remove_file(t.project.join(".vibe/spec.json"));
        }
    }
    let env: Vec<(&str, &str)> = t
        .env
        .iter()
        .map(|(k, v)| (k.as_str(), v.as_str()))
        .collect();
    let args: Vec<&str> = t.args.iter().map(String::as_str).collect();
    validate_json(&t.project, &args, &env)
}

fn hits<'a>(v: &'a serde_json::Value, id: &str, needle: &str) -> Vec<&'a serde_json::Value> {
    v["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["check"] == id && f["message"].as_str().unwrap().contains(needle))
        .collect()
}

#[test]
fn every_check_is_either_triggered_or_pending() {
    let all = registered();
    for id in PENDING {
        assert!(
            all.iter().any(|r| r == id),
            "{id}: PENDING but not registered"
        );
    }
    for id in registered() {
        let pending = PENDING.contains(&id.as_str());
        assert!(
            trigger(&id).is_some() != pending,
            "{id}: needs exactly one of a trigger or a PENDING entry"
        );
    }
}

#[test]
fn warn_error_off_matrix() {
    let mut covered: Vec<String> = Vec::new();
    for id in registered() {
        let Some(t) = trigger(&id) else { continue };
        let (warn_code, warn) = run_with(&t, Some((&id, "warn")));
        assert_eq!(warn_code, 0, "{id} warn must exit 0\n{warn:#}");
        let w = hits(&warn, &id, t.needle);
        assert!(
            !w.is_empty(),
            "{id} warn: no finding with {:?}\n{warn:#}",
            t.needle
        );
        assert!(
            w.iter().all(|f| f["severity"] == "warning"),
            "{id} warn: {w:?}"
        );
        let (code, error) = run_with(&t, Some((&id, "error")));
        let e = hits(&error, &id, t.needle);
        assert!(
            !e.is_empty() && e.iter().all(|f| f["severity"] == "error"),
            "{id} error: {e:?}"
        );
        assert_eq!(code, 1, "{id} error must exit 1");
        let (_, off) = run_with(&t, Some((&id, "off")));
        assert!(
            off["findings"]
                .as_array()
                .unwrap()
                .iter()
                .all(|f| f["check"] != id.as_str()),
            "{id} off: {off:#}"
        );
        assert!(
            hits(&off, &id, t.needle).is_empty()
                && off["findings"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .all(|f| !f["message"].as_str().unwrap().contains(t.needle)),
            "{id} off: a message still names {:?}\n{off:#}",
            t.needle
        );
        covered.push(id.clone());
    }
    let expected: Vec<String> = registered()
        .into_iter()
        .filter(|id| !PENDING.contains(&id.as_str()))
        .collect();
    assert_eq!(
        covered, expected,
        "matrix must cover every non-pending check"
    );
}

#[test]
fn json_answers_validate_against_the_schema() {
    let t = trigger("SF13").unwrap();
    let (code, v) = run_with(&t, None);
    assert_eq!(code, 0);
    assert_spec_json("validate", &v.to_string());
    assert_eq!(v["layout"], "flat");
    assert!(v["examined"].as_u64().unwrap() >= 4);
    let (_, strict) = validate_json(&t.project, &["--strict"], &[]);
    assert_eq!(strict["strict"], true);
    assert_spec_json("validate", &strict.to_string());
}

#[test]
fn json_without_a_root_exits_3() {
    let p = project("mx-noroot");
    let out = spec(&p, &["validate", "--json"], &[]);
    assert_eq!(out.code, 3);
    assert!(out.stdout.is_empty());
    let text = spec(&p, &["validate"], &[]);
    assert_eq!(text.code, 1, "text mode keeps the oracle's exit");
}

#[test]
fn json_survives_hostile_bytes() {
    let p = base("mx-hostile");
    std::fs::write(
        p.join(".spec/lessons.md"),
        b"# Lessons\n\n### Tab\there \x01 \"q\" \\ \xff\xfe x\n**Rule:** y\n",
    )
    .unwrap();
    std::fs::write(p.join(".spec/features/auth/plan.md"), b"---\ntype: feature-plan\nupdated: 2026-01-15\n---\n\n### auth/1 \xff\xfe\n\n### auth/1 again\n").unwrap();
    let out = spec(&p, &["validate", "--json"], &[]);
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_spec_json("validate", &v.to_string());
    assert!(
        v["findings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|f| f["check"] == "SF16")
    );
    assert!(
        v["findings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|f| f["message"].as_str().unwrap().contains('\u{fffd}')),
        "{v:#}"
    );
}

#[test]
fn json_without_spec_files_reports_sf23() {
    let p = project("mx-nofiles");
    write(&p, ".spec/notes.txt", "x");
    let out = spec(&p, &["validate", "--json"], &[]);
    assert_eq!(out.code, 1, "{}", out.text());
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_spec_json("validate", &v.to_string());
    let f = v["findings"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["check"] == "SF23")
        .unwrap_or_else(|| panic!("{v:#}"));
    assert!(
        f["message"]
            .as_str()
            .unwrap()
            .contains("No spec files found in .spec/")
    );
    assert!(f.get("file").is_none(), "{f}");
}

/// The `file` of every finding of `id` in `v`.
fn files_of(v: &serde_json::Value, id: &str) -> Vec<String> {
    v["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["check"] == id)
        .map(|f| f["file"].as_str().unwrap_or("<none>").to_string())
        .collect()
}

#[test]
fn finding_files_name_the_checked_file() {
    // A root file, a feature doc, a missing required file and SF14's feature.
    for (id, file) in [
        ("SF22", ".spec/product.md"),
        ("SF11", ".spec/features/auth/plan.md"),
        ("SF23", ".spec/features/auth/tech.md"),
        ("SF14", ".spec/features/billing/product.md"),
    ] {
        let t = trigger(id).unwrap();
        let (_, v) = run_with(&t, None);
        let files = files_of(&v, id);
        assert!(!files.is_empty(), "{id}: no finding\n{v:#}");
        assert!(
            files.iter().all(|f| f == file),
            "{id}: {files:?}, want {file}"
        );
    }

    // CLAUDE.md: a broken link into the spec root.
    let p = base("mx-file-claude");
    write(&p, "CLAUDE.md", "# C\n\n[x](.spec/missing.md)\n");
    let (_, v) = validate_json(&p, &[], &[]);
    assert_eq!(files_of(&v, "SF22"), ["CLAUDE.md"], "{v:#}");

    // A `--root` outside the project: the file is absolute.
    let outside = base("mx-file-outside");
    write(
        &outside,
        ".spec/product.md",
        "---\ntype: entrypoint\nscope: product\nchildren:\nupdated: 2026-01-15\n---\n\n[x](missing.md)\n",
    );
    let here = project("mx-file-here");
    let root = outside.join(".spec");
    let out = spec(
        &here,
        &["--root", root.to_str().unwrap(), "validate", "--json"],
        &[],
    );
    let v: serde_json::Value =
        serde_json::from_slice(&out.stdout).unwrap_or_else(|e| panic!("{e}: {}", out.text()));
    let want = format!("{}/product.md", root.display());
    assert_eq!(files_of(&v, "SF22"), [want], "{v:#}");
}
