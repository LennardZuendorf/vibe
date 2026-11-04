#![allow(clippy::unwrap_used, clippy::expect_used)]
//! `vibe spec` dispatch.
mod common;
use common::Home;

#[cfg(unix)]
fn script(home: &Home, rel: &str, body: &str) -> std::path::PathBuf {
    home.write_exec(rel, &format!("#!/bin/sh\n{body}\n"))
}

#[cfg(unix)]
#[test]
fn spec_dispatch_runs_vibe_spec_bin_and_passes_exit_code() {
    let home = Home::new("spec-bin");
    let fake = script(
        &home,
        "fake-spec",
        "printf '%s|' \"$@\" > \"$HOME/args\"\nexit 7",
    );
    let out = home.vibe(
        &["spec", "validate", "--strict"],
        &[("VIBE_SPEC_BIN", fake.to_str().unwrap())],
    );
    assert_eq!(out.code, 7, "{}", out.stderr);
    assert_eq!(home.read("args"), "validate|--strict|");
}

#[cfg(unix)]
#[test]
fn spec_dispatch_finds_vibe_spec_on_path() {
    let home = Home::new("spec-path");
    script(&home, "bin/vibe-spec", "echo from-path\nexit 3");
    let out = home.vibe(&["spec", "x"], &[]);
    assert_eq!(out.code, 3, "{}", out.stderr);
    assert_eq!(out.text(), "from-path\n");
}

#[test]
fn spec_missing_prints_one_advice_line_exit_0() {
    let home = Home::new("spec-missing");
    let out = home.vibe(&["spec", "validate"], &[]);
    assert_eq!(out.code, 0, "{}", out.stderr);
    assert_eq!(
        out.text(),
        "vibe: vibe-spec is not installed \u{2014} build it with `cargo build -p vibe-spec` or install the vibe-spec binary\n"
    );
}
