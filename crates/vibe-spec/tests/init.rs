#![cfg(unix)]
// vibe-spec: literal-file
#![allow(clippy::unwrap_used, clippy::expect_used)]
//! `vibe-spec init`: seeds a layered spec root (P4-R11).
mod common;

use common::{assert_spec_json, project, spec, write};
use std::fs;
use std::path::{Path, PathBuf};

const FILES: [&str; 7] = [
    "README.md",
    "AGENTS.md",
    "product.md",
    "tech.md",
    "design.md",
    "plan.md",
    "lessons.md",
];

fn repo(name: &str) -> PathBuf {
    let p = project(name);
    fs::remove_dir_all(p.join(".git")).unwrap();
    common::git(&p, "2026-03-01T12:00:00Z", &["init", "-q"]);
    p
}

fn lines(text: &str) -> Vec<&str> {
    text.lines().collect()
}

fn entries(dir: &Path) -> Vec<String> {
    let mut v: Vec<String> = fs::read_dir(dir)
        .map(|r| {
            r.flatten()
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .collect()
        })
        .unwrap_or_default();
    v.sort();
    v
}

#[test]
fn seeds_a_layered_root() {
    let p = repo("init-seed");
    // From a subdirectory: the project is the git top, not the cwd.
    fs::create_dir_all(p.join("sub/deep")).unwrap();
    let out = spec(&p.join("sub/deep"), &["init"], &[]);
    assert_eq!(out.code, 0, "{}", out.stderr);
    let want: Vec<String> = FILES.iter().map(|f| format!("OK: docs/spec/{f}")).collect();
    let text = out.text();
    assert_eq!(
        lines(&text),
        want.iter().map(String::as_str).collect::<Vec<_>>()
    );
    for f in FILES {
        assert!(p.join("docs/spec").join(f).is_file(), "{f}");
    }
    assert!(!p.join("sub/deep/docs").exists());

    let root = spec(&p, &["root", "--json"], &[]);
    let v: serde_json::Value = serde_json::from_slice(&root.stdout).unwrap();
    assert_eq!(v["layout"], "layered", "{}", root.text());

    let strict = spec(&p, &["validate", "--strict"], &[]);
    assert_eq!(strict.code, 0, "{}\n{}", strict.text(), strict.stderr);
    // SF29 reads git dates: commit, then validate again from the subdirectory.
    common::git(&p, "2026-03-01T12:00:00Z", &["add", "-A"]);
    common::git(&p, "2026-03-01T12:00:00Z", &["commit", "-q", "-m", "seed"]);
    let strict = spec(&p.join("sub"), &["validate", "--strict"], &[]);
    assert_eq!(strict.code, 0, "{}\n{}", strict.text(), strict.stderr);

    for hub in ["product.md", "tech.md", "design.md", "plan.md", "README.md"] {
        let text = fs::read_to_string(p.join("docs/spec").join(hub)).unwrap();
        let l = lines(&text);
        assert_eq!(l[0], "---", "{hub}");
        assert_eq!(l[1], "type: hub", "{hub}");
        assert!(l[2].starts_with("updated: 20"), "{hub}: {}", l[2]);
        assert!(!text.contains("{{"), "{hub}");
    }
}

#[test]
fn never_overwrites() {
    let p = repo("init-keep");
    let mine = "---\ntype: hub\nupdated: 2026-01-01\n---\n\n# Mine\n";
    write(&p, "docs/spec/product.md", mine);
    let out = spec(&p, &["init"], &[]);
    assert_eq!(out.code, 0, "{}", out.stderr);
    let text = out.text();
    let l = lines(&text);
    assert!(l.contains(&"SKIP: docs/spec/product.md"), "{text}");
    assert!(l.contains(&"OK: docs/spec/tech.md"), "{text}");
    assert!(!l.contains(&"OK: docs/spec/product.md"), "{text}");
    assert_eq!(
        fs::read_to_string(p.join("docs/spec/product.md")).unwrap(),
        mine
    );
    // A second run changes nothing and skips all seven.
    let again = spec(&p, &["init"], &[]);
    assert_eq!(again.code, 0);
    let t = again.text();
    let want: Vec<String> = FILES
        .iter()
        .map(|f| format!("SKIP: docs/spec/{f}"))
        .collect();
    assert_eq!(
        lines(&t),
        want.iter().map(String::as_str).collect::<Vec<_>>()
    );
}

#[test]
fn to_flag_and_discovered_root() {
    let p = repo("init-to");
    let out = spec(&p, &["init", "--to", "x/spec"], &[]);
    assert_eq!(out.code, 0, "{}", out.stderr);
    assert!(out.text().lines().any(|l| l == "OK: x/spec/README.md"));
    assert!(p.join("x/spec/lessons.md").is_file());
    assert!(!p.join("docs").exists());

    let q = repo("init-dot");
    fs::create_dir(q.join(".spec")).unwrap();
    let out = spec(&q, &["init"], &[]);
    assert_eq!(out.code, 0, "{}", out.stderr);
    assert!(
        out.text().lines().any(|l| l == "OK: .spec/README.md"),
        "{}",
        out.text()
    );
    assert!(!q.join("docs").exists());
    assert_eq!(entries(&q.join(".spec")).len(), 7);

    // --to wins over a discovered root; a target outside the project is refused.
    let r = repo("init-win");
    fs::create_dir(r.join(".spec")).unwrap();
    assert_eq!(spec(&r, &["init", "--to", "docs/spec"], &[]).code, 0);
    assert!(entries(&r.join(".spec")).is_empty());
    let outside = spec(&r, &["init", "--to", "../elsewhere"], &[]);
    assert_eq!(outside.code, 1, "{}", outside.stderr);
    assert!(outside.stdout.is_empty());
}

#[test]
fn refuses_a_symlinked_target() {
    let p = repo("init-link");
    let real = project("init-link-real");
    fs::create_dir_all(p.join("docs")).unwrap();
    std::os::unix::fs::symlink(&real, p.join("docs/spec")).unwrap();
    let out = spec(&p, &["init"], &[]);
    assert_eq!(out.code, 1, "{}", out.stderr);
    assert!(out.stderr.contains("symlink"), "{}", out.stderr);
    assert!(out.stdout.is_empty());
    assert_eq!(entries(&real), [".git"]);

    // A symlinked parent, reached through `..` in --to.
    let q = repo("init-link2");
    let real2 = project("init-link-real2");
    std::os::unix::fs::symlink(&real2, q.join("docs")).unwrap();
    fs::create_dir(q.join("sub")).unwrap();
    let out = spec(&q.join("sub"), &["init", "--to", "../docs/spec"], &[]);
    assert_eq!(out.code, 1, "{}", out.stderr);
    assert_eq!(entries(&real2), [".git"]);

    // A symlinked single file is refused too, and nothing else is written.
    let r = repo("init-link3");
    fs::create_dir_all(r.join("docs/spec")).unwrap();
    std::os::unix::fs::symlink(real.join("x.md"), r.join("docs/spec/plan.md")).unwrap();
    let out = spec(&r, &["init"], &[]);
    assert_eq!(out.code, 1, "{}", out.stderr);
    assert!(
        entries(&r.join("docs/spec")) == ["plan.md"],
        "{:?}",
        entries(&r.join("docs/spec"))
    );
    assert!(!real.join("x.md").exists());
}

#[test]
fn json_shape() {
    let p = repo("init-json");
    write(&p, "docs/spec/tech.md", "---\ntype: hub\n---\n\nmine\n");
    let out = spec(&p, &["init", "--json"], &[]);
    assert_eq!(out.code, 0, "{}", out.stderr);
    assert_spec_json("init", &out.text());
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["v"], 1);
    assert_eq!(v["root"], "docs/spec");
    assert_eq!(v["skipped"], serde_json::json!(["docs/spec/tech.md"]));
    assert_eq!(v["created"].as_array().unwrap().len(), 6);
    assert!(out.text().ends_with('\n') && out.text().matches('\n').count() == 1);
}

fn tree(p: &Path) -> Vec<(String, Vec<u8>)> {
    fn go(base: &Path, dir: &Path, out: &mut Vec<(String, Vec<u8>)>) {
        for e in fs::read_dir(dir).unwrap().flatten() {
            if e.file_name() == ".git" {
                continue;
            }
            let path = e.path();
            let rel = path
                .strip_prefix(base)
                .unwrap()
                .to_string_lossy()
                .into_owned();
            if path.is_dir() {
                go(base, &path, out);
            } else {
                out.push((rel, fs::read(&path).unwrap()));
            }
        }
    }
    let mut out = Vec::new();
    go(p, p, &mut out);
    out.sort();
    out
}

#[test]
fn refuses_a_populated_flat_root() {
    for (name, rel) in [
        ("init-flat-legacy", ".spec"),
        ("init-flat-docs", "docs/spec"),
    ] {
        let p = repo(name);
        write(
            &p,
            &format!("{rel}/product.md"),
            "---\ntype: entrypoint\n---\n\n# P\n",
        );
        let before = tree(&p);
        let args: Vec<&str> = if rel == "docs/spec" {
            vec!["init", "--to", "docs/spec"]
        } else {
            vec!["init"]
        };
        let out = spec(&p, &args, &[]);
        assert_eq!(out.code, 1, "{rel}: {}", out.stderr);
        assert!(out.stdout.is_empty(), "{rel}");
        // A legacy root migrates to docs/spec; `migrate --to .spec` is refused.
        let to = if rel == ".spec" { "docs/spec" } else { rel };
        let want = format!(
            "init: ERROR: {rel} is a flat spec root; run vibe-spec migrate --to {to} --layout layered"
        );
        assert!(
            out.stderr.lines().any(|l| l == want),
            "{rel}: {}",
            out.stderr
        );
        assert_eq!(tree(&p), before, "{rel}");
    }
    // An empty legacy root still seeds.
    let p = repo("init-empty-legacy");
    fs::create_dir_all(p.join(".spec")).unwrap();
    let out = spec(&p, &["init"], &[]);
    assert_eq!(out.code, 0, "{}", out.stderr);
    assert!(p.join(".spec/README.md").is_file());
}

#[test]
fn seeds_the_lesson_format_and_a_plan_without_a_backlog() {
    // skill-revision/15
    let p = repo("init-formats");
    assert_eq!(spec(&p, &["init"], &[]).code, 0);
    let lessons = fs::read_to_string(p.join("docs/spec/lessons.md")).unwrap();
    for needle in [
        "### ",
        "**Pattern:**",
        "**Rule:**",
        "**Tags:**",
        "**Date:**",
    ] {
        assert!(
            lessons.contains(needle),
            "lessons.md lacks {needle}: {lessons}"
        );
    }
    let plan = fs::read_to_string(p.join("docs/spec/plan.md")).unwrap();
    assert!(!plan.contains("open work"), "{plan}");
    let strict = spec(&p, &["validate", "--strict"], &[]);
    assert_eq!(strict.code, 0, "{}\n{}", strict.text(), strict.stderr);
    // The format sample must not read as a lesson.
    let none = spec(&p, &["lessons-for", "bash", "--json"], &[]);
    let v: serde_json::Value = serde_json::from_slice(&none.stdout).unwrap();
    assert_eq!(v["lessons"].as_array().unwrap().len(), 0, "{}", none.text());
    let markdown = spec(&p, &["lessons-for", ""], &[]);
    assert!(markdown.stdout.is_empty(), "{}", markdown.text());
}

#[test]
fn the_router_fragment_points_to_the_readme_install_section() {
    // skill-revision/15
    let p = repo("init-router");
    let out = spec(&p, &["setup", "--agents-md"], &[]);
    assert_eq!(out.code, 0, "{}", out.stderr);
    let agents = fs::read_to_string(p.join("AGENTS.md")).unwrap();
    assert!(agents.contains("README Install section"), "{agents}");
    assert!(!agents.contains("cargo build"), "{agents}");
}
