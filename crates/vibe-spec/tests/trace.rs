#![cfg(unix)]
// vibe-spec: literal-file
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

fn json_of(p: &std::path::Path, args: &[&str]) -> serde_json::Value {
    let out = spec(p, args, &[]);
    serde_json::from_slice(&out.stdout)
        .unwrap_or_else(|e| panic!("{e}: {}{}", out.text(), out.stderr))
}

#[test]
fn a_shallow_clone_skips_commit_citations() {
    let p = project("tr-shallow");
    write(&p, ".spec/product.md", ENTRY);
    write(
        &p,
        ".spec/features/auth/plan.md",
        "---\ntype: feature-plan\n---\n\n### auth/1 — A\n\n### auth/2 — B\n",
    );
    let d = "2026-02-01T12:00:00Z";
    git(&p, d, &["init", "-q"]);
    git(&p, d, &["add", "-A"]);
    git(&p, d, &["commit", "-q", "-m", "feat: auth/1"]);
    write(&p, "n.txt", "1\n");
    git(&p, d, &["add", "-A"]);
    git(&p, d, &["commit", "-q", "-m", "feat: auth/2"]);
    write(&p, "n.txt", "2\n");
    git(&p, d, &["add", "-A"]);
    git(&p, d, &["commit", "-q", "-m", "chore: more"]);
    assert_eq!(spec(&p, &["trace"], &[]).code, 0);
    let dest = project("tr-shallow-clone");
    let url = format!("file://{}", p.display());
    git(&dest, d, &["clone", "-q", "--depth", "1", &url, "c"]);
    let c = dest.join("c");
    let out = spec(&c, &["trace", "--json"], &[]);
    assert!(
        out.stderr.contains(
            "vibe-spec trace: shallow clone: commit citations skipped; fetch full history"
        ),
        "{}",
        out.stderr
    );
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["history"], "none");
    assert_spec_json("trace", &v.to_string());
    let ok = spec(&c, &["trace", "--since", "HEAD~0"], &[]);
    assert_eq!(ok.code, 1, "{}{}", ok.text(), ok.stderr);
    assert_eq!(spec(&c, &["trace", "--since", "no-such-ref"], &[]).code, 2);
}

#[test]
fn control_bytes_in_a_message_do_not_break_records() {
    let p = project("tr-ctl");
    write(&p, ".spec/product.md", ENTRY);
    write(
        &p,
        ".spec/features/auth/plan.md",
        "---\ntype: feature-plan\n---\n\n### auth/1 — A\n",
    );
    let d = "2026-02-01T12:00:00Z";
    git(&p, d, &["init", "-q"]);
    git(&p, d, &["add", "-A"]);
    git(
        &p,
        d,
        &["commit", "-q", "-m", "x\u{1e}\u{1f} auth/1 auth/9"],
    );
    let head = git_out(&p, &["rev-parse", "HEAD"]);
    let v = json_of(&p, &["trace", "--json"]);
    assert_eq!(v["cited"], 1);
    assert_eq!(
        v["orphans"],
        serde_json::json!([{"unit": "auth/9", "source": format!("commit {}", &head[..7])}])
    );
}

#[test]
fn a_bad_since_beats_the_zero_unit_refusal() {
    let empty = project("tr-since-nounits");
    write(&empty, ".spec/product.md", ENTRY);
    assert_eq!(spec(&empty, &["trace", "--since", "nope"], &[]).code, 2);
    assert_eq!(spec(&empty, &["trace"], &[]).code, 1);
}

#[test]
fn only_exact_unit_headings_register() {
    let p = project("tr-headings");
    write(&p, ".spec/product.md", ENTRY);
    write(
        &p,
        ".spec/features/auth/plan.md",
        "---\ntype: feature-plan\n---\n\n### auth/2.1 — dotted\n\n### auth/3\n\n### auth/4 Title\n\n### auth/5x\n",
    );
    write(&p, "tests/a.sh", "# auth/2 auth/3 auth/4\n");
    let v = json_of(&p, &["trace", "--json"]);
    assert_eq!(v["units"], 2, "{v}");
    assert_eq!(
        v["orphans"],
        serde_json::json!([{"unit": "auth/2", "source": "tests/a.sh"}])
    );
}

#[test]
fn binary_and_huge_test_files_are_skipped() {
    let p = project("tr-binary");
    write(&p, ".spec/product.md", ENTRY);
    write(
        &p,
        ".spec/features/auth/plan.md",
        "---\ntype: feature-plan\n---\n\n### auth/1 — A\n",
    );
    write(&p, "tests/a.sh", "# auth/1\n");
    std::fs::write(p.join("tests/blob.bin"), b"\0auth/9\n").unwrap();
    let mut big = b"auth/8\n".to_vec();
    big.resize(1024 * 1024 + 1, b' ');
    std::fs::write(p.join("tests/big.txt"), big).unwrap();
    let out = spec(&p, &["trace", "--json"], &[]);
    assert_eq!(out.code, 0, "{}{}", out.text(), out.stderr);
}

#[test]
fn json_shape_and_empty_stdout_on_failure() {
    let (p, _) = repo("tr-shape");
    let v = json_of(&p, &["trace", "--json"]);
    assert_eq!(v["history"], "all");
    assert!(v.get("since").is_none(), "{v}");
    let empty = project("tr-shape-empty");
    write(&empty, ".spec/product.md", ENTRY);
    for (cwd, args, code) in [
        (&empty, vec!["trace", "--json"], 1),
        (&p, vec!["trace", "--json", "--since", "no-such-ref"], 2),
        (&project("tr-shape-noroot"), vec!["trace", "--json"], 3),
    ] {
        let out = spec(cwd, &args, &[]);
        assert_eq!(out.code, code, "{}", out.stderr);
        assert!(out.stdout.is_empty(), "{}", out.text());
    }
}

#[test]
fn trace_ignores_commits_outside_a_nested_project() {
    let top = project("tr-nested");
    let p = top.join("app");
    write(&p, ".git-keep", "");
    std::fs::remove_dir_all(top.join(".git")).unwrap();
    write(&p, ".spec/product.md", ENTRY);
    write(
        &p,
        ".spec/features/auth/plan.md",
        "---\ntype: feature-plan\n---\n\n### auth/1 — Login\n",
    );
    let d = "2026-02-01T12:00:00Z";
    git(&top, d, &["init", "-q"]);
    git(&top, d, &["add", "-A"]);
    git(&top, d, &["commit", "-q", "-m", "feat: login (auth/1)"]);
    write(&top, "elsewhere.txt", "x\n");
    git(&top, d, &["add", "-A"]);
    git(
        &top,
        d,
        &["commit", "-q", "-m", "fix: outside only, auth/9"],
    );
    let out = spec(&p, &["trace"], &[]);
    assert_eq!(out.code, 0, "{}{}", out.text(), out.stderr);
    assert!(!out.text().contains("auth/9"), "{}", out.text());
}

#[test]
fn trace_at_the_repo_top_keeps_merge_and_empty_commits() {
    let p = project("tr-top-merges");
    write(&p, ".spec/product.md", ENTRY);
    write(
        &p,
        ".spec/features/auth/plan.md",
        "---\ntype: feature-plan\n---\n\n### auth/1 — A\n\n### auth/2 — B\n\n### auth/3 — C\n",
    );
    let d = "2026-02-01T12:00:00Z";
    git(&p, d, &["init", "-q", "-b", "main"]);
    git(&p, d, &["add", "-A"]);
    git(&p, d, &["commit", "-q", "-m", "feat: a (auth/1)"]);
    git(&p, d, &["checkout", "-q", "-b", "br"]);
    write(&p, "x.txt", "x\n");
    git(&p, d, &["add", "-A"]);
    git(&p, d, &["commit", "-q", "-m", "chore: x"]);
    git(&p, d, &["checkout", "-q", "main"]);
    git(
        &p,
        d,
        &["merge", "-q", "--no-ff", "br", "-m", "Merge: auth/2"],
    );
    git(
        &p,
        d,
        &["commit", "-q", "--allow-empty", "-m", "note auth/3"],
    );
    let out = spec(&p, &["trace"], &[]);
    assert_eq!(out.code, 0, "{}{}", out.text(), out.stderr);
}
