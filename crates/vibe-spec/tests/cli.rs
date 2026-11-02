#![allow(clippy::unwrap_used, clippy::expect_used)]
mod common;

use std::path::Path;
use std::process::Command;

fn run(args: &[&str]) -> (i32, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_vibe-spec"))
        .args(args)
        .output()
        .expect("spawn vibe-spec");
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

#[test]
fn version_exits_zero() {
    let (code, out, _) = run(&["--version"]);
    assert_eq!(code, 0);
    assert_eq!(out, "vibe-spec 0.4.0\n");
}

#[test]
fn no_arguments_is_a_usage_error() {
    let (code, out, err) = run(&[]);
    assert_eq!(code, 2, "stdout={out} stderr={err}");
    assert!(err.contains("Usage: vibe-spec"), "{err}");
}

#[test]
fn unknown_command_is_a_usage_error() {
    let (code, _, err) = run(&["frobnicate"]);
    assert_eq!(code, 2);
    assert!(err.contains("frobnicate"), "{err}");
}

#[cfg(unix)]
#[test]
fn non_utf8_arguments_never_panic() {
    use std::os::unix::ffi::OsStrExt;
    let bad = std::ffi::OsStr::from_bytes(&[0xff, b'x']);
    for args in [
        vec![
            std::ffi::OsStr::new("--root"),
            bad,
            std::ffi::OsStr::new("scan-merges"),
        ],
        vec![
            std::ffi::OsStr::new("--root"),
            bad,
            std::ffi::OsStr::new("validate"),
        ],
        vec![
            std::ffi::OsStr::new("--root"),
            bad,
            std::ffi::OsStr::new("lessons-for"),
        ],
    ] {
        let out = Command::new(env!("CARGO_BIN_EXE_vibe-spec"))
            .args(&args)
            .output()
            .expect("spawn vibe-spec");
        assert_ne!(
            out.status.code(),
            Some(101),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        assert!(out.status.code().is_some());
    }
}

/// Every file under `dir` with its bytes, sorted: a before/after snapshot.
fn snapshot(dir: &Path) -> Vec<(String, Vec<u8>)> {
    fn walk(base: &Path, dir: &Path, out: &mut Vec<(String, Vec<u8>)>) {
        let mut entries: Vec<_> = std::fs::read_dir(dir).unwrap().flatten().collect();
        entries.sort_by_key(std::fs::DirEntry::path);
        for e in entries {
            let path = e.path();
            let rel = path.strip_prefix(base).unwrap().display().to_string();
            if path.is_dir() {
                out.push((format!("{rel}/"), Vec::new()));
                walk(base, &path, out);
            } else {
                out.push((rel, std::fs::read(&path).unwrap()));
            }
        }
    }
    let mut out = Vec::new();
    walk(dir, dir, &mut out);
    out
}

const TAGGED: &str =
    "# Lessons\n\n### Dash h\n**Rule:** Keep it.\n**Tags:** -h\n**Date:** 2026-01-02\n";

fn fixture(name: &str) -> std::path::PathBuf {
    let p = common::project(name);
    common::write(&p, ".vibe/spec.json", common::LEGACY_STRICTNESS);
    common::write(&p, "docs/spec/lessons.md", TAGGED);
    common::write(&p, "docs/spec/product.md", "---\ntype: hub\n---\n# P\n");
    common::write(&p, "docs/spec/features/demo/tech.md", "# Tech\n");
    p
}

#[test]
fn raw_tail_commands_print_usage_for_help() {
    // skill-revision/12
    let p = fixture("cli-help");
    let before = snapshot(&p);
    let cases: [(&str, &[&str]); 8] = [
        ("list-specs", &["--help"]),
        ("list-specs", &["-h"]),
        ("lessons-for", &["--help"]),
        ("lessons-for", &["bash", "-h"]),
        ("scan-merges", &["--help"]),
        ("scan-merges", &["-h"]),
        ("promote", &["--help"]),
        ("promote", &["foo", "--help"]),
    ];
    for (cmd, tail) in cases {
        let mut args = vec![cmd];
        args.extend_from_slice(tail);
        let out = common::spec(&p, &args, &[]);
        assert_eq!(out.code, 0, "{args:?}: {}", out.stderr);
        let text = out.text();
        assert!(
            text.starts_with(&format!("Usage: vibe-spec {cmd}")),
            "{args:?}: {text}"
        );
        assert_eq!(snapshot(&p), before, "{args:?} changed files");
    }
}

#[test]
fn help_as_an_option_value_is_not_help() {
    // skill-revision/12 controls
    let p = fixture("cli-help-value");
    let out = common::spec(&p, &["lessons-for", "--tag", "-h"], &[]);
    assert_eq!(out.code, 0, "{}", out.stderr);
    assert!(out.text().contains("### Dash h"), "{}", out.text());
    let out = common::spec(&p, &["lessons-for", "--format", "-h"], &[]);
    assert!(!out.text().contains("Usage: vibe-spec"), "{}", out.text());
    assert_ne!(out.code, 0);
}

#[test]
fn top_level_help_documents_the_environment_and_exit_codes() {
    // skill-revision/12
    let (code, out, _) = run(&["--help"]);
    assert_eq!(code, 0);
    assert!(out.contains("VIBE_SPEC_ROOT"), "{out}");
    assert!(out.contains("2 usage"), "{out}");
    assert!(!out.contains("port of"), "{out}");
}
