// vibe-spec: literal-file
#![cfg(unix)]
#![allow(clippy::unwrap_used, clippy::expect_used)]
//! `vibe-spec migrate`: scan, plan, `--dry-run` and `--check` (P4-R12, P4-R15).
//! This file builds legacy-root corpora, so it carries the file-level literal
//! marker (P4-R16a). In corpus text, `@` stands for the legacy root name.
mod common;

use common::{project, spec, write};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Expands `@` to the legacy root name.
fn d(text: &str) -> String {
    text.replace('@', ".spec")
}

fn repo(name: &str) -> PathBuf {
    let p = project(name);
    fs::remove_dir_all(p.join(".git")).unwrap();
    common::git(&p, "2026-03-01T12:00:00Z", &["init", "-q"]);
    p
}

fn put(p: &Path, rel: &str, text: &str) {
    write(p, &d(rel), &d(text));
}

/// A flat legacy tree plus config, scripts and a decoy outside the file set.
/// Six literals in six files.
fn corpus(name: &str) -> PathBuf {
    let p = repo(name);
    put(
        &p,
        "@/product.md",
        "---\ntype: entrypoint\nscope: x\nupdated: 2026-01-01\n---\n\n# Product\n\nSee @/tech.md\n",
    );
    put(
        &p,
        "@/tech.md",
        "---\ntype: entrypoint\nchildren: [tech-cli.md]\nupdated: 2026-01-01\n---\n\n# Tech\n\n[cli](tech-cli.md)\n",
    );
    put(
        &p,
        "@/tech-cli.md",
        "---\ntype: tech-topic\nparent: tech.md\nscope: cli\ncovers: dispatcher\nupdated: 2026-01-01\n---\n\n# CLI\n\n[tech.md](tech.md)\n\nFiles live in @/ here.\n",
    );
    put(
        &p,
        "@/plan.md",
        "---\ntype: entrypoint\nscope: plan\nupdated: 2026-01-01\n---\n\n# Plan\n\n- **D12 \u{2014} Orders in the skill.** Per-turn orders live in the skill. See @/tech.md.\n  Second line.\n- **D13 \u{2014} Carrier split.** Plugin carries the runtime.\n\n- **Other.** Kept.\n",
    );
    put(&p, "README.md", "Run `vibe-spec --root @`\n");
    put(
        &p,
        ".claude/settings.json",
        "{\"allow\": [\"Edit(@/**)\"]}\n",
    );
    put(&p, "tests/run.sh", "ROOT=\"$ROOT/@\"\n");
    put(&p, "src/decoy.txt", "@\n");
    p
}

/// Every path under `p` (except `.git`) with its bytes, links by target.
fn snapshot(p: &Path) -> BTreeMap<String, Vec<u8>> {
    fn go(base: &Path, dir: &Path, out: &mut BTreeMap<String, Vec<u8>>) {
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
            let meta = fs::symlink_metadata(&path).unwrap();
            if meta.file_type().is_symlink() {
                out.insert(
                    format!("{rel} -> link"),
                    fs::read_link(&path)
                        .unwrap()
                        .to_string_lossy()
                        .into_owned()
                        .into_bytes(),
                );
            } else if meta.is_dir() {
                out.insert(format!("{rel}/"), Vec::new());
                go(base, &path, out);
            } else {
                out.insert(rel, fs::read(&path).unwrap());
            }
        }
    }
    let mut out = BTreeMap::new();
    go(p, p, &mut out);
    out
}

fn json(out: &common::Out) -> serde_json::Value {
    serde_json::from_slice(&out.stdout)
        .unwrap_or_else(|e| panic!("not JSON ({e}): {}\n{}", out.text(), out.stderr))
}

fn rewrite_lines(v: &serde_json::Value, file: &str) -> Vec<u64> {
    v["rewrites"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["file"] == file)
        .map(|r| r["line"].as_u64().unwrap())
        .collect()
}

#[test]
fn literal_regex() {
    // One probe per line; the expected hits are listed by 1-based line number.
    let p = repo("migrate-regex");
    fs::create_dir_all(p.join("docs/spec")).unwrap();
    put(&p, "docs/spec/README.md", "---\ntype: hub\n---\n");
    let cases: [(&str, bool); 17] = [
        ("see @/plan.md", true),
        ("\"@\"", true),
        ("$ROOT/@", true),
        ("./@", true),
        ("@", true),
        ("strategy.spec", false),
        ("x.spec.ts", false),
        ("tech-spec.md", false),
        ("see .vibe/spec.json", false),
        ("@/x <!-- literal -->", false),
        ("@/x <!-- literal -->   ", false),
        ("@ // literal", false),
        ("@ # literal", false),
        // The tag must end the line and stand alone: none of these exempt.
        ("@ ## literal", true),
        ("@ <!-- literal --> more", true),
        ("## literal notes about @", true),
        ("@/x, then @/y", true),
    ];
    let body: String = cases.iter().map(|(l, _)| format!("{}\n", d(l))).collect();
    put(&p, "README.md", &body);
    let out = spec(
        &p,
        &["migrate", "--to", "docs/spec", "--check", "--json"],
        &[],
    );
    assert_eq!(out.code, 1, "{}\n{}", out.text(), out.stderr);
    let v = json(&out);
    let want: Vec<u64> = cases
        .iter()
        .enumerate()
        .filter(|(_, (_, hit))| *hit)
        .map(|(i, _)| i as u64 + 1)
        .collect();
    assert_eq!(rewrite_lines(&v, "README.md"), want, "{v}");
    // The rewritten line replaces the literal and keeps the rest.
    let first = &v["rewrites"][0];
    assert_eq!(first["old"], "see .spec/plan.md");
    assert_eq!(first["new"], "see docs/spec/plan.md");
}

#[test]
fn file_level_marker_exempts_a_file_only_in_its_first_five_lines() {
    let p = repo("migrate-marker");
    fs::create_dir_all(p.join("docs/spec")).unwrap();
    put(&p, "docs/spec/README.md", "---\ntype: hub\n---\n");
    put(&p, "a.md", "# a\n<!-- vibe-spec: literal-file -->\n@ one\n");
    put(
        &p,
        "b.md",
        "1\n2\n3\n4\n5\n<!-- vibe-spec: literal-file -->\n@ two\n",
    );
    put(
        &p,
        "c.md",
        "// vibe-spec: literal-file extra words\n@ three\n",
    );
    let out = spec(
        &p,
        &["migrate", "--to", "docs/spec", "--check", "--json"],
        &[],
    );
    assert_eq!(out.code, 1, "{}", out.stderr);
    let v = json(&out);
    let files: Vec<&str> = v["rewrites"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["file"].as_str().unwrap())
        .collect();
    assert_eq!(files, ["b.md", "c.md"], "{v}");
}

#[test]
fn dry_run_prints_the_table_and_writes_nothing() {
    let p = corpus("migrate-dry");
    let before = snapshot(&p);
    // From a subdirectory: `--to` is cwd-relative (as for `init`); output is project-relative.
    fs::create_dir_all(p.join("src/deep")).unwrap();
    let before = {
        let mut b = before;
        b.insert("src/deep/".to_string(), Vec::new());
        b
    };
    let out = spec(
        &p.join("src/deep"),
        &["migrate", "--to", "../../docs/spec", "--dry-run"],
        &[],
    );
    assert_eq!(out.code, 0, "{}\n{}", out.text(), out.stderr);
    let text = out.text();
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(
        lines[0], "migrate: dry run \u{2014} 6 literal(s) in 6 file(s)",
        "{text}"
    );
    for want in [
        "README.md:1  Run `vibe-spec --root .spec` \u{2192} Run `vibe-spec --root docs/spec`",
        "tests/run.sh:1  ROOT=\"$ROOT/.spec\" \u{2192} ROOT=\"$ROOT/docs/spec\"",
        ".claude/settings.json:1  {\"allow\": [\"Edit(.spec/**)\"]} \u{2192} {\"allow\": [\"Edit(docs/spec/**)\"]}",
        "docs/spec/product.md:9  See .spec/tech.md \u{2192} See docs/spec/tech.md",
        "docs/spec/tech-cli.md:13  Files live in .spec/ here. \u{2192} Files live in docs/spec/ here.",
    ] {
        assert!(lines.contains(&want), "missing {want:?} in\n{text}");
    }
    assert!(lines.contains(&"move: .spec \u{2192} docs/spec"), "{text}");
    assert!(!text.contains("decoy"), "{text}");
    assert_eq!(snapshot(&p), before, "dry run wrote something");
}

#[test]
fn json_shape_and_exit_codes() {
    let p = corpus("migrate-json");
    let out = spec(
        &p,
        &["migrate", "--to", "docs/spec", "--dry-run", "--json"],
        &[],
    );
    assert_eq!(out.code, 0, "{}", out.stderr);
    let v = json(&out);
    assert_eq!(v["v"], 1);
    assert_eq!(v["from"], ".spec");
    assert_eq!(v["to"], "docs/spec");
    assert_eq!(v["dry_run"], true);
    assert_eq!(
        v["moves"],
        serde_json::json!([{"from": ".spec", "to": "docs/spec"}])
    );
    assert_eq!(v["created"], serde_json::json!([]));
    assert_eq!(v["markers"], serde_json::json!([]));
    assert_eq!(v["rewrites"].as_array().unwrap().len(), 6);
    // Sorted by file, then line.
    let files: Vec<&str> = v["rewrites"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["file"].as_str().unwrap())
        .collect();
    let mut sorted = files.clone();
    sorted.sort_unstable();
    assert_eq!(files, sorted);

    // No spec root: exit 3, empty stdout.
    let bare = repo("migrate-noroot");
    let out = spec(
        &bare,
        &["migrate", "--to", "docs/spec", "--dry-run", "--json"],
        &[],
    );
    assert_eq!((out.code, out.stdout.len()), (3, 0), "{}", out.stderr);
}

#[test]
fn bare_migrate_refuses_until_execute_exists_and_writes_nothing() {
    let p = corpus("migrate-bare");
    let before = snapshot(&p);
    let out = spec(&p, &["migrate", "--to", "docs/spec"], &[]);
    assert_eq!(out.code, 2, "{}", out.stderr);
    assert_eq!(snapshot(&p), before);
    let both = spec(
        &p,
        &["migrate", "--to", "docs/spec", "--dry-run", "--check"],
        &[],
    );
    assert_eq!(both.code, 2, "{}", both.stderr);
    let none = spec(&p, &["migrate"], &[]);
    assert_eq!(none.code, 2, "{}", none.stderr);
}

#[test]
fn floor_miss_aborts() {
    let p = corpus("migrate-floor");
    put(&p, ".vibe/spec.json", "{\"migrate\": {\"floor\": 999}}\n");
    let before = snapshot(&p);
    let out = spec(&p, &["migrate", "--to", "docs/spec", "--dry-run"], &[]);
    assert_eq!(out.code, 1, "{}", out.text());
    assert!(out.stdout.is_empty(), "{}", out.text());
    let line = out
        .stderr
        .lines()
        .find(|l| l.starts_with("migrate: ERROR: scanned "))
        .unwrap_or_else(|| panic!("{}", out.stderr));
    assert!(line.ends_with(" file(s), floor 999"), "{line}");
    assert_eq!(snapshot(&p), before);
    // The floor guards --check too: a sweep that scanned nothing is not clean.
    let check = spec(&p, &["migrate", "--to", "docs/spec", "--check"], &[]);
    assert_eq!(check.code, 1, "{}", check.stderr);
    assert!(check.stderr.contains("floor 999"), "{}", check.stderr);
}

#[test]
fn nothing_to_migrate() {
    let p = repo("migrate-nothing");
    put(&p, "@/product.md", "---\ntype: entrypoint\n---\n\n# P\n");
    put(&p, "README.md", "no literal here\n");
    let before = snapshot(&p);
    let out = spec(&p, &["migrate", "--to", "docs/spec", "--dry-run"], &[]);
    assert_eq!(out.code, 1, "{}", out.text());
    assert!(
        out.stderr
            .contains(&d("migrate: ERROR: no @ literal found")),
        "{}",
        out.stderr
    );
    assert_eq!(snapshot(&p), before);
}

#[test]
fn migrate_twice_is_a_no_op() {
    // The state after a migration: root at docs/spec, no literal left.
    let p = repo("migrate-twice");
    put(&p, "docs/spec/product.md", "---\ntype: hub\n---\n\n# P\n");
    put(&p, "README.md", "points at docs/spec\n");
    let before = snapshot(&p);
    for args in [
        vec!["migrate", "--to", "docs/spec", "--dry-run"],
        vec!["migrate", "--to", "docs/spec"],
        vec![
            "migrate",
            "--to",
            "docs/spec",
            "--layout",
            "layered",
            "--dry-run",
        ],
    ] {
        let out = spec(&p, &args, &[]);
        assert_eq!(out.code, 0, "{args:?}: {}\n{}", out.text(), out.stderr);
        assert_eq!(out.text(), "migrate: already migrated\n", "{args:?}");
        assert_eq!(snapshot(&p), before, "{args:?}");
    }
    let out = spec(
        &p,
        &["migrate", "--to", "docs/spec", "--dry-run", "--json"],
        &[],
    );
    assert_eq!(out.code, 0);
    let v = json(&out);
    assert_eq!(v["moves"], serde_json::json!([]));
    assert_eq!(v["rewrites"], serde_json::json!([]));
}

#[test]
fn literals_remaining_after_the_move_are_rewritten_without_a_move() {
    let p = repo("migrate-remain");
    put(&p, "docs/spec/product.md", "---\ntype: hub\n---\n\n# P\n");
    put(&p, "README.md", "old pointer @/plan.md\n");
    let out = spec(
        &p,
        &["migrate", "--to", "docs/spec", "--dry-run", "--json"],
        &[],
    );
    assert_eq!(out.code, 0, "{}", out.stderr);
    let v = json(&out);
    assert_eq!(v["moves"], serde_json::json!([]));
    assert_eq!(v["rewrites"].as_array().unwrap().len(), 1);
}

#[cfg(unix)]
#[test]
fn symlinks_are_not_followed() {
    let p = repo("migrate-links");
    put(&p, "@/product.md", "---\ntype: entrypoint\n---\n\n# P\n");
    put(&p, "tests/real.md", "line @ here\n");
    std::os::unix::fs::symlink("real.md", p.join("tests/link.md")).unwrap();
    std::os::unix::fs::symlink("../tests", p.join(".claude-tests")).unwrap();
    put(&p, "elsewhere/x.md", "@ hidden\n");
    fs::create_dir_all(p.join(".claude")).unwrap();
    std::os::unix::fs::symlink("../elsewhere", p.join(".claude/dirlink")).unwrap();
    let before = snapshot(&p);
    let out = spec(
        &p,
        &["migrate", "--to", "docs/spec", "--dry-run", "--json"],
        &[],
    );
    assert_eq!(out.code, 0, "{}", out.stderr);
    let v = json(&out);
    let files: Vec<&str> = v["rewrites"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["file"].as_str().unwrap())
        .collect();
    assert_eq!(files, ["tests/real.md"], "{v}");
    assert!(!out.text().contains("link.md"), "{}", out.text());
    assert_eq!(snapshot(&p), before);

    // A symlinked root is refused: the move would not follow it.
    let q = repo("migrate-rootlink");
    fs::create_dir_all(q.join("store")).unwrap();
    put(
        &q,
        "store/product.md",
        "---\ntype: entrypoint\n---\n\n# P\n",
    );
    std::os::unix::fs::symlink("store", q.join(".spec")).unwrap();
    put(&q, "README.md", "@\n");
    let out = spec(&q, &["migrate", "--to", "docs/spec", "--dry-run"], &[]);
    assert_eq!(out.code, 1, "{}", out.text());
    assert!(out.stderr.contains("symlink"), "{}", out.stderr);
}

#[test]
fn check_mode() {
    let p = corpus("migrate-check");
    let before = snapshot(&p);
    let out = spec(&p, &["migrate", "--to", "docs/spec", "--check"], &[]);
    assert_eq!(out.code, 1, "{}", out.stderr);
    let text = out.text();
    assert_eq!(
        text.lines().next().unwrap(),
        "migrate: check \u{2014} 6 literal(s) in 6 file(s)",
        "{text}"
    );
    assert!(text.contains("README.md:1  "), "{text}");
    assert_eq!(snapshot(&p), before);

    // A clean corpus: root at docs/spec, nothing left.
    let q = repo("migrate-check-clean");
    put(&q, "docs/spec/product.md", "---\ntype: hub\n---\n\n# P\n");
    put(&q, "README.md", "clean\n");
    let out = spec(&q, &["migrate", "--to", "docs/spec", "--check"], &[]);
    assert_eq!(out.code, 0, "{}\n{}", out.text(), out.stderr);
    assert_eq!(
        out.text().lines().next().unwrap(),
        "migrate: check \u{2014} 0 literal(s) in 0 file(s)"
    );
    let out = spec(
        &q,
        &["migrate", "--to", "docs/spec", "--check", "--json"],
        &[],
    );
    assert_eq!(out.code, 0);
    assert_eq!(json(&out)["rewrites"], serde_json::json!([]));
}

#[test]
fn include_and_exclude_shape_the_file_set() {
    let p = corpus("migrate-globs");
    put(&p, "docs/notes/a.md", "@ in notes\n");
    put(&p, "docs/notes/skip.md", "@ skipped\n");
    put(
        &p,
        ".vibe/spec.json",
        "{\"migrate\": {\"include\": [\"docs/notes/**\"], \"exclude\": [\"tests/**\", \"docs/notes/skip.md\"]}}\n",
    );
    let out = spec(
        &p,
        &["migrate", "--to", "docs/spec", "--dry-run", "--json"],
        &[],
    );
    assert_eq!(out.code, 0, "{}", out.stderr);
    let v = json(&out);
    let files: Vec<&str> = v["rewrites"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["file"].as_str().unwrap())
        .collect();
    assert!(files.contains(&"docs/notes/a.md"), "{files:?}");
    assert!(!files.contains(&"docs/notes/skip.md"), "{files:?}");
    assert!(!files.contains(&"tests/run.sh"), "{files:?}");
    assert!(files.contains(&"README.md"), "{files:?}");
}

#[test]
fn bad_migrate_config_is_a_usage_error() {
    for (name, cfg) in [
        ("unknown", "{\"migrate\": {\"floors\": 1}}"),
        ("floor-type", "{\"migrate\": {\"floor\": \"1\"}}"),
        ("floor-negative", "{\"migrate\": {\"floor\": -1}}"),
        ("include-type", "{\"migrate\": {\"include\": \"a\"}}"),
        ("include-item", "{\"migrate\": {\"include\": [1]}}"),
        ("exclude-empty", "{\"migrate\": {\"exclude\": [\"\"]}}"),
        ("section-type", "{\"migrate\": []}"),
    ] {
        let p = corpus(&format!("migrate-badcfg-{name}"));
        put(&p, ".vibe/spec.json", cfg);
        let before = snapshot(&p);
        let out = spec(&p, &["migrate", "--to", "docs/spec", "--dry-run"], &[]);
        assert_eq!(out.code, 2, "{name}: {}\n{}", out.text(), out.stderr);
        assert!(out.stderr.contains("migrate"), "{name}: {}", out.stderr);
        assert_eq!(snapshot(&p), before, "{name}");
    }
}

#[test]
fn layered_plan() {
    let p = corpus("migrate-layered");
    let before = snapshot(&p);
    let out = spec(
        &p,
        &[
            "migrate",
            "--to",
            "docs/spec",
            "--layout",
            "layered",
            "--dry-run",
            "--json",
        ],
        &[],
    );
    assert_eq!(out.code, 0, "{}\n{}", out.text(), out.stderr);
    let v = json(&out);
    assert_eq!(
        v["moves"],
        serde_json::json!([
            {"from": ".spec", "to": "docs/spec"},
            {"from": "docs/spec/tech-cli.md", "to": "docs/spec/tech/cli.md"},
        ]),
        "{v}"
    );
    let created: Vec<&str> = v["created"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c.as_str().unwrap())
        .collect();
    assert_eq!(
        created,
        [
            "docs/spec/decisions/d12-orders-in-the-skill.md",
            "docs/spec/decisions/d13-carrier-split.md"
        ]
    );
    let rw = |file: &str, line: u64| -> serde_json::Value {
        v["rewrites"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["file"] == file && r["line"] == line)
            .cloned()
            .unwrap_or_else(|| panic!("no rewrite {file}:{line} in {v}"))
    };
    // Hubs: entrypoint becomes hub.
    assert_eq!(rw("docs/spec/product.md", 2)["new"], "type: hub");
    assert_eq!(rw("docs/spec/tech.md", 2)["new"], "type: hub");
    // The hub's children and body links follow the move.
    assert_eq!(rw("docs/spec/tech.md", 3)["new"], "children: [tech/cli.md]");
    assert_eq!(rw("docs/spec/tech.md", 9)["new"], "[cli](tech/cli.md)");
    // The moved note: frontmatter retyped, parent/scope/covers dropped, back link fixed.
    let fm = rw("docs/spec/tech/cli.md", 1);
    assert_eq!(
        fm["new"],
        "---\ntype: tech\napplies: \"**\"\nupdated: 2026-01-01\n---"
    );
    assert_eq!(
        fm["old"],
        "---\ntype: tech-topic\nparent: tech.md\nscope: cli\ncovers: dispatcher\nupdated: 2026-01-01\n---"
    );
    assert_eq!(
        rw("docs/spec/tech/cli.md", 11)["new"],
        "[tech.md](../tech.md)"
    );
    // plan.md: each D-bullet (two lines for D12) becomes a link.
    let d12 = rw("docs/spec/plan.md", 9);
    assert!(
        d12["old"].as_str().unwrap().ends_with("Second line."),
        "{d12}"
    );
    assert_eq!(
        d12["new"],
        "- [D12 \u{2014} Orders in the skill](decisions/d12-orders-in-the-skill.md)"
    );
    assert_eq!(
        rw("docs/spec/plan.md", 11)["new"],
        "- [D13 \u{2014} Carrier split](decisions/d13-carrier-split.md)"
    );
    // The decision record: frontmatter plus the bullet text, literals rewritten.
    let record =
        v["rewrites"].as_array().unwrap().iter().any(|r| {
            r["file"] == "docs/spec/plan.md" && r["old"].as_str().unwrap().contains("D12")
        });
    assert!(record);
    assert_eq!(snapshot(&p), before);

    // The text form lists moves and creations.
    let text = spec(
        &p,
        &[
            "migrate",
            "--to",
            "docs/spec",
            "--layout",
            "layered",
            "--dry-run",
        ],
        &[],
    )
    .text();
    assert!(
        text.contains("move: docs/spec/tech-cli.md \u{2192} docs/spec/tech/cli.md"),
        "{text}"
    );
    assert!(
        text.contains("create: docs/spec/decisions/d12-orders-in-the-skill.md"),
        "{text}"
    );
}

#[test]
fn layered_decision_bodies_and_title_slugs() {
    let p = repo("migrate-slug");
    put(
        &p,
        "@/plan.md",
        "---\ntype: entrypoint\n---\n\n- **D8 \u{2014} One two three four five six seven eight.** Body @/x.md and [t](tech.md).\n\n## Heading\n- **D9 \u{2014} Keep it.** Tail.\n# End\n",
    );
    put(&p, "@/tech.md", "---\ntype: entrypoint\n---\n");
    let out = spec(
        &p,
        &[
            "migrate",
            "--to",
            "docs/spec",
            "--layout",
            "layered",
            "--dry-run",
            "--json",
        ],
        &[],
    );
    assert_eq!(out.code, 0, "{}\n{}", out.text(), out.stderr);
    let v = json(&out);
    let created = v["created"].as_array().unwrap();
    assert_eq!(
        created[0],
        "docs/spec/decisions/d8-one-two-three-four-five-six.md"
    );
    assert_eq!(created[1], "docs/spec/decisions/d9-keep-it.md");
    // A D-bullet ends at a blank line, a heading or the next bullet.
    let d9 = v["rewrites"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["old"].as_str().unwrap().contains("D9"))
        .unwrap();
    assert_eq!(d9["old"], "- **D9 \u{2014} Keep it.** Tail.");
}

#[test]
fn layered_refuses_product_and_plan_topics() {
    for name in ["product-x.md", "plan-x.md"] {
        let p = corpus("migrate-refuse");
        put(&p, &format!("@/{name}"), "---\ntype: entrypoint\n---\n");
        let before = snapshot(&p);
        let out = spec(
            &p,
            &[
                "migrate",
                "--to",
                "docs/spec",
                "--layout",
                "layered",
                "--dry-run",
            ],
            &[],
        );
        assert_eq!(out.code, 1, "{}", out.text());
        assert!(out.stdout.is_empty());
        assert!(
            out.stderr.contains(&d(&format!(
                "migrate: ERROR: @/{name}: place product/plan topics by hand"
            ))),
            "{}",
            out.stderr
        );
        assert_eq!(snapshot(&p), before);
    }
}

#[test]
fn layered_refuses_a_clash_with_an_existing_note() {
    let p = corpus("migrate-clash");
    put(&p, "@/tech/cli.md", "---\ntype: tech\n---\n");
    let out = spec(
        &p,
        &[
            "migrate",
            "--to",
            "docs/spec",
            "--layout",
            "layered",
            "--dry-run",
        ],
        &[],
    );
    assert_eq!(out.code, 1, "{}", out.text());
    assert!(out.stderr.contains("migrate: ERROR: "), "{}", out.stderr);
    assert!(out.stderr.contains("tech/cli.md"), "{}", out.stderr);
}

#[test]
fn an_existing_target_is_refused() {
    let p = corpus("migrate-exists");
    put(&p, "docs/spec/x.md", "# x\n");
    // Discovery prefers docs/spec, so name the legacy root explicitly.
    let out = spec(
        &p,
        &[
            "--root",
            ".spec",
            "migrate",
            "--to",
            "docs/spec",
            "--dry-run",
        ],
        &[],
    );
    assert_eq!(out.code, 1, "{}", out.text());
    assert!(
        out.stderr
            .contains("migrate: ERROR: docs/spec already exists"),
        "{}",
        out.stderr
    );
}

#[test]
fn a_non_utf8_literal_line_fails_closed() {
    let p = repo("migrate-bytes");
    put(&p, "@/product.md", "---\ntype: entrypoint\n---\n");
    let mut bytes = d("bad @ ").into_bytes();
    bytes.extend_from_slice(&[0xff, 0xfe, b'\n']);
    fs::write(p.join("README.md"), bytes).unwrap();
    let out = spec(&p, &["migrate", "--to", "docs/spec", "--dry-run"], &[]);
    assert_eq!(out.code, 1, "{}", out.text());
    assert!(out.stderr.contains("README.md:1"), "{}", out.stderr);
}

#[test]
fn to_outside_the_project_is_a_usage_error() {
    let p = corpus("migrate-outside");
    for to in ["../elsewhere", "/abs/elsewhere", "."] {
        let out = spec(&p, &["migrate", "--to", to, "--dry-run"], &[]);
        assert_eq!(out.code, 2, "{to}: {}\n{}", out.text(), out.stderr);
    }
}

#[test]
fn links_follow_a_root_move_and_keep_valid_links() {
    let p = repo("migrate-links-depth");
    put(
        &p,
        "@/product.md",
        "---\ntype: entrypoint\n---\n\n[code](../src/a.rs) [self](./tech.md#x) [web](https://x.io/a) [anchor](#top)\n```\n[fenced](../src/b.rs)\n```\n",
    );
    put(&p, "@/tech.md", "---\ntype: entrypoint\n---\n");
    put(&p, "README.md", "see @/product.md\n");
    let out = spec(
        &p,
        &["migrate", "--to", "docs/spec", "--dry-run", "--json"],
        &[],
    );
    assert_eq!(out.code, 0, "{}", out.stderr);
    let v = json(&out);
    let rw: Vec<&serde_json::Value> = v["rewrites"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["file"] == "docs/spec/product.md")
        .collect();
    assert_eq!(rw.len(), 1, "{v}");
    assert_eq!(
        rw[0]["new"],
        "[code](../../src/a.rs) [self](./tech.md#x) [web](https://x.io/a) [anchor](#top)"
    );
}

#[test]
fn a_layered_root_in_its_final_state_is_already_migrated() {
    let p = repo("migrate-layered-done");
    put(
        &p,
        "docs/spec/README.md",
        "---\ntype: hub\nupdated: 2026-01-01\n---\n\n[t](tech/cli.md)\n",
    );
    put(
        &p,
        "docs/spec/tech/cli.md",
        "---\ntype: tech\napplies: \"**\"\nupdated: 2026-01-01\n---\n\n[up](../README.md)\n",
    );
    put(
        &p,
        "docs/spec/plan.md",
        "---\ntype: hub\nupdated: 2026-01-01\n---\n\n- [D12 \u{2014} Orders](decisions/d12-orders.md)\n",
    );
    let before = snapshot(&p);
    let out = spec(
        &p,
        &[
            "migrate",
            "--to",
            "docs/spec",
            "--layout",
            "layered",
            "--dry-run",
        ],
        &[],
    );
    assert_eq!(out.code, 0, "{}\n{}", out.text(), out.stderr);
    assert_eq!(out.text(), "migrate: already migrated\n");
    assert_eq!(snapshot(&p), before);
}
