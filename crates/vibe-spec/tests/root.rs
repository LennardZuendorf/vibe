#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Root discovery: one order, from any directory (tech-spec.md, Root and discovery).
mod common;

use std::fs;

use common::{project, spec, write};
use serde_json::Value;

fn json(out: &common::Out) -> Value {
    serde_json::from_slice(&out.stdout).unwrap()
}

#[test]
fn dot_spec_from_a_subdirectory_resolves_like_the_root() {
    let p = project("subdir");
    fs::create_dir_all(p.join(".spec")).unwrap();
    fs::create_dir_all(p.join("src/deep")).unwrap();
    let top = spec(&p, &["root"], &[]);
    let deep = spec(&p.join("src/deep"), &["root"], &[]);
    assert_eq!(top.code, 0, "{}", top.stderr);
    assert_eq!(top.text(), format!("{}\n", p.join(".spec").display()));
    assert_eq!(deep.text(), top.text());
    let v = json(&spec(&p.join("src/deep"), &["root", "--json"], &[]));
    assert_eq!(v["source"], "dot-spec");
    assert_eq!(v["depth"], 2);
    assert_eq!(v["root"], p.join(".spec").to_string_lossy().as_ref());
}

#[test]
fn docs_spec_beats_dot_spec_at_the_same_level() {
    let p = project("docs-first");
    fs::create_dir_all(p.join(".spec")).unwrap();
    fs::create_dir_all(p.join("docs/spec")).unwrap();
    let v = json(&spec(&p, &["root", "--json"], &[]));
    assert_eq!(v["source"], "docs-spec");
    assert_eq!(v["depth"], 0);
}

#[test]
fn the_nearest_directory_wins_the_walk() {
    let p = project("nearest");
    fs::create_dir_all(p.join("docs/spec")).unwrap();
    fs::create_dir_all(p.join("pkg/.spec")).unwrap();
    let v = json(&spec(&p.join("pkg"), &["root", "--json"], &[]));
    assert_eq!(v["source"], "dot-spec");
    assert_eq!(v["root"], p.join("pkg/.spec").to_string_lossy().as_ref());
}

#[test]
fn a_config_root_is_a_hit() {
    let p = project("config");
    write(&p, ".vibe/spec.json", "{\"root\": \"specs\"}\n");
    fs::create_dir_all(p.join("specs")).unwrap();
    fs::create_dir_all(p.join(".spec")).unwrap();
    let v = json(&spec(&p, &["root", "--json"], &[]));
    assert_eq!(v["source"], "config");
    assert_eq!(v["root"], p.join("specs").to_string_lossy().as_ref());
}

#[test]
fn a_config_without_root_falls_through() {
    let p = project("config-other-keys");
    write(
        &p,
        ".vibe/spec.json",
        "{\"strictness\": {}, \"tooth\": \"warn\"}\n",
    );
    fs::create_dir_all(p.join(".spec")).unwrap();
    let out = spec(&p, &["root", "--json"], &[]);
    assert_eq!(out.code, 0, "{}", out.stderr);
    assert_eq!(json(&out)["source"], "dot-spec");
}

#[test]
fn a_config_root_that_is_missing_exits_3_without_falling_through() {
    let p = project("config-missing");
    write(&p, ".vibe/spec.json", "{\"root\": \"gone\"}\n");
    fs::create_dir_all(p.join(".spec")).unwrap();
    let out = spec(&p, &["root"], &[]);
    assert_eq!(out.code, 3);
    assert!(out.stdout.is_empty());
    assert!(out.stderr.contains("from config"), "{}", out.stderr);
}

#[test]
fn a_malformed_config_is_a_usage_error() {
    let p = project("config-bad");
    write(&p, ".vibe/spec.json", "{not json");
    fs::create_dir_all(p.join(".spec")).unwrap();
    assert_eq!(spec(&p, &["root"], &[]).code, 2);
    write(&p, ".vibe/spec.json", "{\"root\": 7}");
    assert_eq!(spec(&p, &["root"], &[]).code, 2);
}

#[test]
fn the_flag_beats_the_environment_and_both_beat_the_walk() {
    let p = project("explicit");
    fs::create_dir_all(p.join(".spec")).unwrap();
    fs::create_dir_all(p.join("a")).unwrap();
    fs::create_dir_all(p.join("b")).unwrap();
    let env = spec(&p, &["root", "--json"], &[("VIBE_SPEC_ROOT", "b")]);
    assert_eq!(json(&env)["source"], "env");
    assert_eq!(json(&env)["root"], p.join("b").to_string_lossy().as_ref());
    let flag = spec(
        &p,
        &["--root", "a", "root", "--json"],
        &[("VIBE_SPEC_ROOT", "b")],
    );
    assert_eq!(json(&flag)["source"], "flag");
    assert_eq!(json(&flag)["root"], p.join("a").to_string_lossy().as_ref());
    let missing = spec(&p, &["--root", "nope", "root"], &[]);
    assert_eq!(missing.code, 3);
    assert!(missing.stderr.contains("from flag"), "{}", missing.stderr);
}

#[test]
fn no_tree_exits_3_and_the_walk_stops_at_git() {
    let p = project("none");
    fs::create_dir_all(p.join("src")).unwrap();
    let out = spec(&p.join("src"), &["root", "--json"], &[]);
    assert_eq!(out.code, 3);
    assert!(out.stdout.is_empty());

    // A decoy above the .git boundary is never reached.
    let outer = project("outer");
    fs::create_dir_all(outer.join(".spec")).unwrap();
    fs::remove_dir_all(outer.join(".git")).unwrap();
    let inner = outer.join("repo");
    fs::create_dir_all(inner.join(".git")).unwrap();
    assert_eq!(spec(&inner, &["root"], &[]).code, 3);
    // Control: without the inner .git the walk reaches the decoy.
    fs::remove_dir_all(inner.join(".git")).unwrap();
    assert_eq!(spec(&inner, &["root"], &[]).code, 0);
}

#[test]
fn spec_dir_is_ignored_with_a_warning() {
    let p = project("specdir");
    fs::create_dir_all(p.join(".spec")).unwrap();
    fs::create_dir_all(p.join("elsewhere")).unwrap();
    let out = spec(&p, &["root", "--json"], &[("SPEC_DIR", "elsewhere")]);
    assert_eq!(json(&out)["source"], "dot-spec");
    assert!(out.stderr.contains("SPEC_DIR is ignored"), "{}", out.stderr);
}

#[cfg(unix)]
#[test]
fn root_json_matches_the_contract() {
    let p = project("contract");
    fs::create_dir_all(p.join("docs/spec")).unwrap();
    let out = spec(&p, &["root", "--json"], &[]);
    common::assert_spec_json("root", out.text().trim_end());
}
