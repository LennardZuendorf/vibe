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
    let cases: &[(&str, bool)] = &[
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
        // A dot before, or a file extension after, makes it a different name.
        ("see ..@", false),
        ("@.ts", false),
        ("@.js and more", false),
        ("@.md", false),
        ("the end of @.", true),
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
fn conflicting_flags_and_a_missing_target_are_usage_errors() {
    let p = corpus("migrate-usage");
    let before = snapshot(&p);
    let both = spec(
        &p,
        &["migrate", "--to", "docs/spec", "--dry-run", "--check"],
        &[],
    );
    assert_eq!(both.code, 2, "{}", both.stderr);
    let none = spec(&p, &["migrate"], &[]);
    assert_eq!(none.code, 2, "{}", none.stderr);
    assert_eq!(snapshot(&p), before);
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

#[test]
fn links_in_angle_brackets_reference_definitions_and_code_spans() {
    let p = repo("migrate-links-forms");
    put(&p, "README.md", "@\n");
    put(
        &p,
        "@/product.md",
        "---\ntype: entrypoint\n---\n\n[a](<../a b.md>) and [b](<../b.md#h> \"t\")\n\n[r]: ../x.md\n[s]: <../y z.md> \"title\"\n[web]: https://x.io/a\n\nCode `[c](../c.md)` and ``[d](../d.md)`` and [e](../e.md).\n\n    [r]: ../indented-code.md\n",
    );
    let out = spec(
        &p,
        &["migrate", "--to", "docs/spec", "--dry-run", "--json"],
        &[],
    );
    assert_eq!(out.code, 0, "{}", out.stderr);
    let v = json(&out);
    let news: Vec<&str> = v["rewrites"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["file"] == "docs/spec/product.md")
        .map(|r| r["new"].as_str().unwrap())
        .collect();
    assert_eq!(
        news,
        [
            "[a](<../../a b.md>) and [b](<../../b.md#h> \"t\")",
            "[r]: ../../x.md",
            "[s]: <../../y z.md> \"title\"",
            "Code `[c](../c.md)` and ``[d](../d.md)`` and [e](../../e.md).",
        ],
        "{v}"
    );
}

fn plan_of(p: &Path) -> common::Out {
    spec(
        p,
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
    )
}

#[test]
fn slugs_are_ascii_kebab() {
    let p = repo("migrate-slug-ascii");
    put(&p, "README.md", "@\n");
    put(
        &p,
        "@/plan.md",
        "---\ntype: entrypoint\n---\n\n- **D1 \u{2014} Today's plan, caf\u{e9}  --  now.** Body.\n- **D2 \u{2014} --A--.** Body.\n",
    );
    let out = plan_of(&p);
    assert_eq!(out.code, 0, "{}", out.stderr);
    let v = json(&out);
    assert_eq!(
        v["created"][0],
        "docs/spec/decisions/d1-todays-plan-caf-now.md"
    );
    assert_eq!(v["created"][1], "docs/spec/decisions/d2-a.md");

    let q = repo("migrate-slug-empty");
    put(
        &q,
        "@/plan.md",
        "---\ntype: entrypoint\n---\n\n- **D7 \u{2014} \u{e9}\u{e9}.** Body.\n",
    );
    let out = plan_of(&q);
    assert_eq!(out.code, 1, "{}", out.text());
    assert!(out.stdout.is_empty());
    assert!(out.stderr.contains("migrate: ERROR: "), "{}", out.stderr);
    assert!(out.stderr.contains("D7"), "{}", out.stderr);
}

#[test]
fn a_repeated_decision_number_is_refused() {
    let p = repo("migrate-dup-d");
    put(
        &p,
        "@/plan.md",
        "---\ntype: entrypoint\n---\n\n- **D12 \u{2014} First title.** A.\n- **D12 \u{2014} Other title.** B.\n",
    );
    let out = plan_of(&p);
    assert_eq!(out.code, 1, "{}", out.text());
    assert!(
        out.stderr
            .contains(&d("migrate: ERROR: @/plan.md: duplicate decision D12")),
        "{}",
        out.stderr
    );
}

#[test]
fn check_json_is_not_a_dry_run() {
    let p = corpus("migrate-check-json");
    let v = json(&spec(
        &p,
        &["migrate", "--to", "docs/spec", "--check", "--json"],
        &[],
    ));
    assert_eq!(v["dry_run"], false, "{v}");
    assert_eq!(v["check"], true, "{v}");
    let v = json(&spec(
        &p,
        &["migrate", "--to", "docs/spec", "--dry-run", "--json"],
        &[],
    ));
    assert_eq!(v["dry_run"], true, "{v}");
    assert_eq!(v["check"], false, "{v}");
}

#[test]
fn the_walk_skips_build_dirs_and_unrelated_trees() {
    use std::os::unix::fs::PermissionsExt;
    let p = corpus("migrate-prune");
    put(&p, "tests/target/x.md", "@ in a build dir\n");
    put(&p, "tests/node_modules/y.md", "@ in node_modules\n");
    put(&p, "tests/sub/target/z.md", "@ nested build dir\n");
    // A tree no include glob reaches is never opened.
    put(&p, "src/locked/a.txt", "@\n");
    fs::set_permissions(p.join("src/locked"), fs::Permissions::from_mode(0o000)).unwrap();
    let out = spec(
        &p,
        &["migrate", "--to", "docs/spec", "--dry-run", "--json"],
        &[],
    );
    fs::set_permissions(p.join("src/locked"), fs::Permissions::from_mode(0o755)).unwrap();
    assert_eq!(out.code, 0, "{}", out.stderr);
    let v = json(&out);
    assert_eq!(v["rewrites"].as_array().unwrap().len(), 6, "{v}");

    // A non-UTF-8 name aborts only where it would be scanned. Some file
    // systems refuse such names; then there is nothing to test.
    use std::os::unix::ffi::OsStrExt;
    let bad = std::ffi::OsStr::from_bytes(b"bad-\xff.txt");
    if fs::write(p.join("src").join(bad), "x").is_ok() {
        let out = spec(&p, &["migrate", "--to", "docs/spec", "--dry-run"], &[]);
        assert_eq!(out.code, 0, "{}", out.stderr);
        fs::write(p.join("tests").join(bad), "x").unwrap();
        let out = spec(&p, &["migrate", "--to", "docs/spec", "--dry-run"], &[]);
        assert_eq!(out.code, 1, "{}", out.text());
        assert!(out.stderr.contains("not UTF-8"), "{}", out.stderr);
    }
}

#[test]
fn crlf_endings_are_kept() {
    let p = repo("migrate-crlf");
    let crlf = |s: &str| d(s).replace('\n', "\r\n");
    write(
        &p,
        ".spec/tech.md",
        &crlf("---\ntype: entrypoint\nchildren: [tech-cli.md]\n---\n\nsee @/plan.md\n"),
    );
    write(
        &p,
        ".spec/tech-cli.md",
        &crlf("---\ntype: tech-topic\nparent: tech.md\nupdated: 2026-01-01\n---\n\n[t](tech.md)\n"),
    );
    write(
        &p,
        ".spec/plan.md",
        &crlf(
            "---\ntype: entrypoint\n---\n\n- **D3 \u{2014} Cr lf.** Line one.\n  Line two @/x.\n",
        ),
    );
    let out = plan_of(&p);
    assert_eq!(out.code, 0, "{}\n{}", out.text(), out.stderr);
    let v = json(&out);
    for r in v["rewrites"].as_array().unwrap() {
        let (old, new) = (r["old"].as_str().unwrap(), r["new"].as_str().unwrap());
        // Every line keeps its ending: each `\n` is preceded by `\r`, and the
        // last line of the text keeps the `\r` it had.
        assert_eq!(
            old.matches('\r').count(),
            old.matches('\n').count() + 1,
            "{r}"
        );
        assert_eq!(
            new.matches('\r').count(),
            new.matches('\n').count() + 1,
            "{r}"
        );
        assert!(!new.contains("\r\r"), "{r}");
    }
    assert_eq!(v["created"].as_array().unwrap().len(), 1);
}

// ---- execute (Task 11) ----

fn today() -> String {
    let out = std::process::Command::new("date")
        .args(["-u", "+%F"])
        .output()
        .unwrap();
    String::from_utf8(out.stdout).unwrap().trim().to_string()
}

/// Commits everything, dated in the past.
fn commit_all(p: &Path) {
    common::git(p, "2026-03-01T12:00:00Z", &["add", "-A"]);
    common::git(
        p,
        "2026-03-01T12:00:00Z",
        &["commit", "-q", "-m", "legacy tree"],
    );
}

fn status(p: &Path) -> String {
    common::git_out(p, &["status", "--porcelain"])
}

fn migrate(p: &Path, extra: &[&str]) -> common::Out {
    let mut args = vec!["migrate", "--to", "docs/spec"];
    args.extend_from_slice(extra);
    spec(p, &args, &[])
}

fn read(p: &Path, rel: &str) -> String {
    fs::read_to_string(p.join(rel)).unwrap_or_else(|e| panic!("{rel}: {e}"))
}

#[test]
fn migrates_a_flat_repo() {
    let p = corpus("migrate-flat");
    put(
        &p,
        ".vibe/spec.json",
        "{\n  \"tooth\": \"warn\",\n  \"migrate\": {\"floor\": 1}\n}\n",
    );
    commit_all(&p);
    let out = migrate(&p, &[]);
    assert_eq!(out.code, 0, "{}\n{}", out.text(), out.stderr);
    assert!(out.text().starts_with("migrate: done"), "{}", out.text());
    for name in ["product", "tech", "tech-cli", "plan"] {
        assert!(p.join(format!("docs/spec/{name}.md")).is_file(), "{name}");
    }
    assert!(!p.join(d(".spec")).exists());
    // The root key is merged in first; the other keys keep their order.
    let cfg = read(&p, ".vibe/spec.json");
    let v: serde_json::Value = serde_json::from_str(&cfg).unwrap();
    assert_eq!(v["root"], "docs/spec", "{cfg}");
    assert_eq!(v["tooth"], "warn");
    assert_eq!(v["migrate"]["floor"], 1);
    let at = |k: &str| cfg.find(k).unwrap();
    assert!(at("\"tooth\"") < at("\"migrate\""), "{cfg}");
    // Literals are rewritten in the file set, and only there.
    assert_eq!(read(&p, "README.md"), "Run `vibe-spec --root docs/spec`\n");
    assert_eq!(
        read(&p, ".claude/settings.json"),
        "{\"allow\": [\"Edit(docs/spec/**)\"]}\n"
    );
    assert_eq!(read(&p, "tests/run.sh"), "ROOT=\"$ROOT/docs/spec\"\n");
    assert_eq!(read(&p, "src/decoy.txt"), d("@\n"));
    assert!(read(&p, "docs/spec/product.md").contains("See docs/spec/tech.md"));
    let check = migrate(&p, &["--check"]);
    assert_eq!(check.code, 0, "{}\n{}", check.text(), check.stderr);
    // git sees renames, not a delete plus an add.
    let st = status(&p);
    let renames = st.lines().filter(|l| l.starts_with('R')).count();
    assert_eq!(renames, 4, "{st}");
    assert!(
        !st.lines()
            .any(|l| l.starts_with("D ") || l.starts_with("A ")),
        "{st}"
    );
}

#[test]
fn layered_execute() {
    let p = corpus("migrate-layered-run");
    // A root note outside the layered layout draws an SF30 warning.
    put(
        &p,
        "@/notes.md",
        "---\ntype: note\nupdated: 2026-01-01\n---\n\n# Notes\n",
    );
    commit_all(&p);
    let out = migrate(&p, &["--layout", "layered"]);
    assert_eq!(out.code, 0, "{}\n{}", out.text(), out.stderr);
    for hub in ["product", "tech", "design", "plan"] {
        let path = format!("docs/spec/{hub}.md");
        if p.join(&path).exists() {
            let text = read(&p, &path);
            assert!(text.contains("type: hub\n"), "{path}: {text}");
            assert!(!text.contains("type: entrypoint"), "{path}");
        }
    }
    assert!(p.join("docs/spec/tech/cli.md").is_file());
    assert!(!p.join("docs/spec/tech-cli.md").exists());
    let cli = read(&p, "docs/spec/tech/cli.md");
    assert!(
        cli.contains("type: tech\n") && cli.contains("applies: \"**\""),
        "{cli}"
    );
    assert!(cli.contains("[tech.md](../tech.md)"), "{cli}");
    let plan = read(&p, "docs/spec/plan.md");
    assert!(
        plan.contains("- [D12 \u{2014} Orders in the skill](decisions/d12-orders-in-the-skill.md)"),
        "{plan}"
    );
    assert!(
        p.join("docs/spec/decisions/d12-orders-in-the-skill.md")
            .is_file()
    );
    assert!(p.join("docs/spec/decisions/d13-carrier-split.md").is_file());
    assert!(read(&p, "docs/spec/tech.md").contains("tech/cli.md"));
    // The marker for the SF30 warning is inserted after the frontmatter and listed.
    let notes = read(&p, "docs/spec/notes.md");
    assert!(
        notes.contains("---\n<!-- vibe-spec: allow SF30 -->\n"),
        "{notes}"
    );
    assert!(
        out.text().contains("docs/spec/notes.md SF30"),
        "{}",
        out.text()
    );
    // Links resolve: no SF22 and no marker-able warning is left.
    let (_, v) = common::validate_json(&p, &[], &[]);
    let findings = v["findings"].as_array().unwrap();
    let left: Vec<&serde_json::Value> = findings
        .iter()
        .filter(|f| {
            ["SF22", "SF30", "SF31", "SF33", "SF34"].contains(&f["check"].as_str().unwrap())
        })
        .collect();
    assert!(left.is_empty(), "{left:?}");
    // The JSON run reports the same markers.
    let q = corpus("migrate-layered-json");
    put(
        &q,
        "@/notes.md",
        "---\ntype: note\nupdated: 2026-01-01\n---\n\n# Notes\n",
    );
    let out = migrate(&q, &["--layout", "layered", "--json"]);
    assert_eq!(out.code, 0, "{}\n{}", out.text(), out.stderr);
    let v = json(&out);
    assert_eq!(v["dry_run"], false);
    assert_eq!(
        v["markers"],
        serde_json::json!(["docs/spec/notes.md SF30"]),
        "{v}"
    );
}

fn is_root() -> bool {
    let out = std::process::Command::new("id").arg("-u").output().unwrap();
    String::from_utf8_lossy(&out.stdout).trim() == "0"
}

#[test]
fn revert_restores_the_tree() {
    use std::os::unix::fs::PermissionsExt;
    if is_root() {
        eprintln!("skipped: running as root, a read-only directory does not stop the write");
        return;
    }
    let p = corpus("migrate-revert");
    put(&p, "@/scratch.txt", "untracked\n");
    commit_all(&p);
    put(&p, "@/late.txt", "untracked later\n");
    let tree = snapshot(&p);
    let st = status(&p);
    // The last write (tests/run.sh) cannot create its temporary file.
    fs::set_permissions(p.join("tests"), fs::Permissions::from_mode(0o555)).unwrap();
    let out = migrate(&p, &["--layout", "layered"]);
    fs::set_permissions(p.join("tests"), fs::Permissions::from_mode(0o755)).unwrap();
    assert_eq!(out.code, 1, "{}\n{}", out.text(), out.stderr);
    assert!(out.stdout.is_empty(), "{}", out.text());
    assert!(
        out.stderr.starts_with("migrate: ERROR: tests/run.sh: "),
        "{}",
        out.stderr
    );
    assert!(
        out.stderr.ends_with(" \u{2014} reverted\n"),
        "{}",
        out.stderr
    );
    let now = snapshot(&p);
    let changed: Vec<&String> = now
        .keys()
        .chain(tree.keys())
        .filter(|k| now.get(*k) != tree.get(*k))
        .collect();
    assert!(changed.is_empty(), "tree differs at {changed:?}");
    assert_eq!(status(&p), st, "git status differs after the revert");
    // And a second attempt on the restored tree works.
    let again = migrate(&p, &["--layout", "layered"]);
    assert_eq!(again.code, 0, "{}\n{}", again.text(), again.stderr);
}

#[test]
fn non_git_project_renames() {
    let p = corpus("migrate-nogit");
    fs::remove_dir_all(p.join(".git")).unwrap();
    let out = migrate(&p, &[]);
    assert_eq!(out.code, 0, "{}\n{}", out.text(), out.stderr);
    assert!(p.join("docs/spec/tech-cli.md").is_file());
    assert!(!p.join(d(".spec")).exists());
    assert!(!p.join(".git").exists());
    assert_eq!(read(&p, "README.md"), "Run `vibe-spec --root docs/spec`\n");
}

#[test]
fn moved_and_rewritten_docs_get_today() {
    let p = corpus("migrate-today");
    put(
        &p,
        "@/notes.md",
        "---\ntype: note\n---\n\n# Notes (no updated key)\n",
    );
    commit_all(&p);
    let out = migrate(&p, &["--layout", "layered"]);
    assert_eq!(out.code, 0, "{}\n{}", out.text(), out.stderr);
    let today = today();
    let line = format!("updated: {today}\n");
    for rel in [
        "product.md",
        "tech.md",
        "plan.md",
        "tech/cli.md",
        "notes.md",
        "decisions/d12-orders-in-the-skill.md",
    ] {
        let text = read(&p, &format!("docs/spec/{rel}"));
        assert_eq!(text.matches("updated:").count(), 1, "{rel}: {text}");
        assert!(text.contains(&line), "{rel}: {text}");
    }
    // Only spec docs are bumped.
    assert_eq!(read(&p, "README.md"), "Run `vibe-spec --root docs/spec`\n");
    // Committed today, the docs are not older than their last commit.
    let date = format!("{today}T12:00:00Z");
    common::git(&p, &date, &["add", "-A"]);
    common::git(&p, &date, &["commit", "-q", "-m", "migrated"]);
    let (_, v) = common::validate_json(&p, &["--strict"], &[]);
    let sf29: Vec<&serde_json::Value> = v["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["check"] == "SF29")
        .collect();
    assert!(sf29.is_empty(), "{sf29:?}");
}

#[test]
fn legacy_root_still_found() {
    let a = corpus("migrate-legacy-a");
    let b = corpus("migrate-legacy-b");
    let before_b = snapshot(&b);
    let out = migrate(&a, &[]);
    assert_eq!(out.code, 0, "{}\n{}", out.text(), out.stderr);
    let v = json(&spec(&a, &["root", "--json"], &[]));
    assert_eq!(v["source"], "config", "{v}");
    assert!(v["root"].as_str().unwrap().ends_with("docs/spec"), "{v}");
    let v = json(&spec(&b, &["root", "--json"], &[]));
    assert_eq!(v["source"], "dot-spec", "{v}");
    assert_eq!(snapshot(&b), before_b);
}

#[test]
fn a_run_with_nothing_to_move_writes_nothing() {
    // No literal anywhere: a population miss before any write.
    let p = repo("migrate-nopop");
    put(&p, "@/product.md", "---\ntype: entrypoint\n---\n\n# P\n");
    let before = snapshot(&p);
    let out = migrate(&p, &[]);
    assert_eq!(out.code, 1, "{}", out.text());
    assert!(out.stderr.contains("migrate: ERROR: "), "{}", out.stderr);
    assert_eq!(snapshot(&p), before);
}

#[test]
fn malformed_config_json_fails_closed() {
    let p = corpus("migrate-badjson");
    put(&p, ".vibe/spec.json", "{\"tooth\": ");
    let before = snapshot(&p);
    let out = migrate(&p, &[]);
    assert_ne!(out.code, 0);
    assert!(out.stdout.is_empty());
    assert_eq!(snapshot(&p), before);
}

#[test]
fn links_inside_fenced_code_are_left_alone() {
    let p = repo("migrate-fence-links");
    put(&p, "README.md", "@\n");
    put(
        &p,
        "@/product.md",
        "---\ntype: entrypoint\n---\n\n```\n[a](../a.md) and @/x\n```\n\n[b](../b.md)\n",
    );
    let out = spec(
        &p,
        &["migrate", "--to", "docs/spec", "--dry-run", "--json"],
        &[],
    );
    assert_eq!(out.code, 0, "{}", out.stderr);
    let v = json(&out);
    let news: Vec<(u64, &str)> = v["rewrites"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["file"] == "docs/spec/product.md")
        .map(|r| (r["line"].as_u64().unwrap(), r["new"].as_str().unwrap()))
        .collect();
    // The literal inside the fence is still rewritten; its link is not.
    assert_eq!(
        news,
        [(6, "[a](../a.md) and docs/spec/x"), (9, "[b](../../b.md)")],
        "{v}"
    );
}

#[test]
fn slugs_drop_non_ascii_characters_instead_of_splitting_on_them() {
    let p = repo("migrate-slug-drop");
    put(&p, "README.md", "@\n");
    put(
        &p,
        "@/plan.md",
        "---\ntype: entrypoint\n---\n\n- **D5 \u{2014} Na\u{ef}ve plan.** Body.\n",
    );
    let v = json(&plan_of(&p));
    assert_eq!(
        v["created"][0], "docs/spec/decisions/d5-nave-plan.md",
        "{v}"
    );
}

#[test]
fn crlf_decision_files_keep_every_line_ending() {
    let p = repo("migrate-crlf-created");
    let crlf = |s: &str| d(s).replace('\n', "\r\n");
    write(
        &p,
        ".spec/tech.md",
        &crlf(
            "---\ntype: entrypoint\nchildren: [tech-cli.md]\nupdated: 2026-01-01\n---\n\nsee @/plan.md\n",
        ),
    );
    write(
        &p,
        ".spec/tech-cli.md",
        &crlf("---\ntype: tech-topic\nparent: tech.md\nupdated: 2026-01-01\n---\n\n[t](tech.md)\n"),
    );
    write(
        &p,
        ".spec/plan.md",
        &crlf(
            "---\ntype: entrypoint\nupdated: 2026-01-01\n---\n\n- **D3 \u{2014} Cr lf.** Line one.\n  Line two @/x.\n",
        ),
    );
    let out = migrate(&p, &["--layout", "layered"]);
    assert_eq!(out.code, 0, "{}\n{}", out.text(), out.stderr);
    let bytes = fs::read(p.join("docs/spec/decisions/d3-cr-lf.md")).unwrap();
    let want = format!(
        "---\r\ntype: decision\r\nupdated: {}\r\n---\r\n\r\n- **D3 \u{2014} Cr lf.** Line one.\r\n  Line two docs/spec/x.\r\n",
        today()
    );
    assert_eq!(String::from_utf8(bytes).unwrap(), want);
    // No file the run touched has a bare line feed.
    for rel in ["plan.md", "tech.md", "tech/cli.md"] {
        let text = fs::read(p.join("docs/spec").join(rel)).unwrap();
        for (i, b) in text.iter().enumerate() {
            if *b == b'\n' {
                assert!(i > 0 && text[i - 1] == b'\r', "{rel}: bare LF at {i}");
            }
        }
    }
}

/// A committed corpus with staged, unstaged and untracked changes in the
/// root, and a `CLAUDE.md` link that only breaks once the root has moved
/// (validate checks it against the root's new name).
fn staged_corpus(name: &str, breaking: bool) -> PathBuf {
    let p = corpus(name);
    put(
        &p,
        "CLAUDE.md",
        &format!(
            "[x]({})\n",
            if breaking {
                "docs/spec/gone.md"
            } else {
                "README.md"
            }
        ),
    );
    commit_all(&p);
    put(
        &p,
        "@/product.md",
        &d(
            "---\ntype: entrypoint\nscope: x\nupdated: 2026-01-01\n---\n\n# Product\n\nSee @/tech.md\nstaged line\n",
        ),
    );
    common::git(
        &p,
        "2026-03-02T12:00:00Z",
        &["add", "@/product.md".replace('@', ".spec").as_str()],
    );
    put(
        &p,
        "@/product.md",
        &d(
            "---\ntype: entrypoint\nscope: x\nupdated: 2026-01-01\n---\n\n# Product\n\nSee @/tech.md\nstaged line\nunstaged line\n",
        ),
    );
    put(&p, "@/new.md", "staged new file\n");
    common::git(&p, "2026-03-02T12:00:00Z", &["add", ".spec/new.md"]);
    put(&p, "@/untracked.txt", "untracked\n");
    p
}

fn git_state(p: &Path) -> Vec<String> {
    ["status --porcelain", "diff", "diff --cached", "ls-files -s"]
        .iter()
        .map(|a| common::git_out(p, &a.split(' ').collect::<Vec<_>>()))
        .collect()
}

#[test]
fn an_error_the_migration_adds_reverts_after_the_commit() {
    let p = staged_corpus("migrate-introduced", true);
    let tree = snapshot(&p);
    let state = git_state(&p);
    let out = migrate(&p, &[]);
    assert_eq!(out.code, 1, "{}\n{}", out.text(), out.stderr);
    assert!(out.stdout.is_empty(), "{}", out.text());
    assert_eq!(
        out.stderr,
        "migrate: ERROR: migration introduced 1 validate error(s) \u{2014} reverted\n"
    );
    let now = snapshot(&p);
    let changed: Vec<&String> = now
        .keys()
        .chain(tree.keys())
        .filter(|k| now.get(*k) != tree.get(*k))
        .collect();
    assert!(changed.is_empty(), "tree differs at {changed:?}");
    assert_eq!(git_state(&p), state, "git state differs after the revert");
}

#[test]
fn staged_changes_survive_a_migration() {
    let p = staged_corpus("migrate-staged", false);
    let out = migrate(&p, &[]);
    assert_eq!(out.code, 0, "{}\n{}", out.text(), out.stderr);
    // The staged blobs followed the renames.
    let staged = common::git_out(&p, &["show", ":docs/spec/product.md"]);
    assert!(
        staged.contains("staged line") && !staged.contains("unstaged line"),
        "{staged}"
    );
    assert!(read(&p, "docs/spec/product.md").contains("unstaged line"));
    assert_eq!(
        common::git_out(&p, &["show", ":docs/spec/new.md"]),
        "staged new file"
    );
    let st = status(&p);
    assert!(st.contains("A  docs/spec/new.md"), "{st}");
    assert!(st.contains("?? docs/spec/untracked.txt"), "{st}");
}

#[test]
fn pre_existing_validate_errors_are_noted_not_fatal() {
    let p = corpus("migrate-preexisting");
    put(&p, "CLAUDE.md", "[x](@/gone.md)\n");
    let before = spec(&p, &["validate", "--format", "json"], &[]);
    assert_eq!(json(&before)["errors"], 2, "{}", before.text());
    let q = corpus("migrate-preexisting-json");
    put(&q, "CLAUDE.md", "[x](@/gone.md)\n");
    let out = migrate(&p, &[]);
    assert_eq!(out.code, 0, "{}\n{}", out.text(), out.stderr);
    assert!(
        out.text()
            .contains("migrate: note: 2 pre-existing validate error(s)\n"),
        "{}",
        out.text()
    );
    let out = migrate(&q, &["--json"]);
    assert_eq!(out.code, 0, "{}\n{}", out.text(), out.stderr);
    common::assert_spec_json("migrate", &out.text());
    assert_eq!(json(&out)["pre_existing_errors"], 2);
}
