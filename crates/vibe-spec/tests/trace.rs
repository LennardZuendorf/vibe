#![cfg(unix)]
#![allow(clippy::unwrap_used, clippy::expect_used)]
//! `vibe-spec trace` over a synthetic repo (P3b-R13).
mod common;

use common::{assert_spec_json, git, git_out, project, spec, write};
use std::path::PathBuf;

const ENTRY: &str = "---\ntype: entrypoint\nscope: product\nchildren:\nupdated: 2026-01-15\n---\n";
const AUTH_PLAN: &str = "---\ntype: feature-plan\n---\n\n### auth/1 — Login\n\n### auth/2 — Logout\n\n### auth/3 — Reset\n\n```\n### auth/4 — fenced, not a unit\n```\n";

/// Units auth/1–3 and billing/1. Commit 1 cites auth/1; commit 2 cites
/// auth/9 and auth/4 (no such units); tests/ cites auth/2, billing/1 and
/// unknown/4 (not a feature).
fn repo(name: &str) -> (PathBuf, String) {
    let p = project(name);
    write(&p, ".spec/product.md", ENTRY);
    write(&p, ".spec/features/auth/plan.md", AUTH_PLAN);
    write(
        &p,
        ".spec/features/billing/plan.md",
        "---\ntype: feature-plan\n---\n\n### billing/1 — Invoice\n",
    );
    write(
        &p,
        "tests/auth_test.sh",
        "# covers auth/2 and billing/1; unknown/4 is not a feature\n",
    );
    let d = "2026-02-01T12:00:00Z";
    git(&p, d, &["init", "-q"]);
    git(&p, d, &["add", "-A"]);
    git(&p, d, &["commit", "-q", "-m", "feat: add login (auth/1)"]);
    let first = git_out(&p, &["rev-parse", "HEAD"]);
    write(&p, "notes.txt", "x\n");
    git(&p, d, &["add", "-A"]);
    git(
        &p,
        d,
        &["commit", "-q", "-m", "fix: touch auth/9 and auth/4"],
    );
    (p, first)
}

#[test]
fn trace_reports_uncited_units_and_orphans() {
    let (p, _) = repo("tr-all");
    let out = spec(&p, &["trace"], &[]);
    assert_eq!(out.code, 1, "{}{}", out.text(), out.stderr);
    let text = out.text();
    assert!(
        text.starts_with("trace: 4 units, 3 cited; commits: all\n"),
        "{text}"
    );
    assert!(text.contains("UNCITED  auth/3\n"), "{text}");
    assert!(text.contains("ORPHAN   auth/4  commit "), "{text}");
    assert!(text.contains("ORPHAN   auth/9  commit "), "{text}");
    assert!(!text.contains("unknown/4"), "{text}");
    let json = spec(&p, &["trace", "--json"], &[]);
    let v: serde_json::Value = serde_json::from_slice(&json.stdout).unwrap();
    assert_spec_json("trace", &v.to_string());
    assert_eq!(v["uncited"], serde_json::json!(["auth/3"]));
    assert_eq!(v["orphans"].as_array().unwrap().len(), 2);
    assert_eq!(
        (
            v["units"].as_u64(),
            v["cited"].as_u64(),
            v["history"].as_str()
        ),
        (Some(4), Some(3), Some("all"))
    );
}

#[test]
fn since_limits_the_commits() {
    let (p, first) = repo("tr-since");
    let out = spec(&p, &["trace", "--since", &first, "--json"], &[]);
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["uncited"], serde_json::json!(["auth/1", "auth/3"]));
    assert_eq!(v["history"], "since");
    assert_eq!(v["since"], first);
    assert_spec_json("trace", &v.to_string());
}

#[test]
fn a_clean_trace_exits_0() {
    let (p, _) = repo("tr-clean");
    write(&p, "tests/more_test.sh", "# auth/3\n");
    git(&p, "2026-02-02T12:00:00Z", &["add", "-A"]);
    git(
        &p,
        "2026-02-02T12:00:00Z",
        &["commit", "-q", "-m", "test: cover reset"],
    );
    let mut plan = AUTH_PLAN.to_string();
    plan.push_str("\n### auth/4 — Now real\n\n### auth/9 — Also real\n");
    write(&p, ".spec/features/auth/plan.md", &plan);
    let out = spec(&p, &["trace"], &[]);
    assert_eq!(out.code, 0, "{}", out.text());
}

#[test]
fn trace_without_git_uses_tests_only() {
    let p = project("tr-nogit");
    write(&p, ".spec/product.md", ENTRY);
    write(&p, ".spec/features/auth/plan.md", AUTH_PLAN);
    write(&p, "tests/a.sh", "# auth/1 auth/2 auth/3\n");
    let out = spec(&p, &["trace", "--json"], &[]);
    assert_eq!(out.code, 0, "{}", out.stderr);
    assert!(out.stderr.contains("no git history"), "{}", out.stderr);
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["history"], "none");
}

#[test]
fn trace_usage_and_root_errors() {
    let (p, _) = repo("tr-bad");
    assert_eq!(spec(&p, &["trace", "--since", "no-such-ref"], &[]).code, 2);
    assert_eq!(spec(&p, &["trace", "--since", "--all"], &[]).code, 2);
    let none = project("tr-noroot");
    assert_eq!(spec(&none, &["trace"], &[]).code, 3);
    let empty = project("tr-nounits");
    write(&empty, ".spec/product.md", ENTRY);
    let out = spec(&empty, &["trace"], &[]);
    assert_eq!(out.code, 1);
    assert!(out.stderr.contains("nothing to trace"), "{}", out.stderr);
}

#[test]
fn fenced_citations_in_markdown_tests_do_not_count() {
    let p = project("tr-fence");
    write(&p, ".spec/product.md", ENTRY);
    write(
        &p,
        ".spec/features/auth/plan.md",
        "---\ntype: feature-plan\n---\n\n### auth/1 — Login\n",
    );
    write(&p, "tests/notes.md", "```\nauth/1\n```\n");
    let out = spec(&p, &["trace"], &[]);
    assert_eq!(out.code, 1, "{}", out.text());
    assert!(out.text().contains("UNCITED  auth/1\n"), "{}", out.text());
    write(&p, "tests/notes.md", "```\nauth/1\n```\nauth/1 outside\n");
    assert_eq!(spec(&p, &["trace"], &[]).code, 0);
}

#[test]
fn a_repo_without_commits_has_no_history() {
    let p = project("tr-unborn");
    write(&p, ".spec/product.md", ENTRY);
    write(&p, ".spec/features/auth/plan.md", AUTH_PLAN);
    write(&p, "tests/a.sh", "# auth/1 auth/2 auth/3\n");
    git(&p, "2026-02-01T12:00:00Z", &["init", "-q"]);
    let out = spec(&p, &["trace", "--json"], &[]);
    assert_eq!(out.code, 0, "{}", out.stderr);
    assert!(out.stderr.contains("no git history"), "{}", out.stderr);
    assert_eq!(spec(&p, &["trace", "--since", "HEAD"], &[]).code, 2);
}
