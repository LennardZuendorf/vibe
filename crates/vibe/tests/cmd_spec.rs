#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Embedded spec dispatch (spec-projects-mcp/1).
mod common;
use common::Home;

#[cfg(unix)]
#[test]
fn spec_projects_mcp_1_embedded_without_peer() {
    let home = Home::new("spec-embedded");
    let fake = home.write_exec(
        "fake-spec",
        "#!/bin/sh\nprintf called > \"$HOME/peer-called\"\nexit 7\n",
    );
    let out = home.vibe(
        &["spec", "--help"],
        &[("VIBE_SPEC_BIN", fake.to_str().unwrap())],
    );
    assert_eq!(out.code, 0, "{}", out.stderr);
    assert!(out.text().contains("Usage: vibe-spec"), "{}", out.text());
    assert!(
        out.stderr.contains("VIBE_SPEC_BIN is deprecated"),
        "{}",
        out.stderr
    );
    assert!(!home.dir.join("peer-called").exists());
}

#[test]
fn spec_embedded_ignores_path_peer() {
    let home = Home::new("spec-no-peer");
    #[cfg(unix)]
    home.write_exec(
        "bin/vibe-spec",
        "#!/bin/sh\nprintf called > \"$HOME/peer-called\"\nexit 7\n",
    );
    let out = home.vibe(&["spec", "root"], &[]);
    assert_eq!(out.code, 3, "{}", out.stderr);
    assert!(out.stderr.contains("no spec root"), "{}", out.stderr);
    assert!(!home.path("peer-called").exists());
}

#[test]
fn embedded_spec_retains_the_raw_double_dash_tail() {
    // spec-projects-mcp/1: outer clap must not consume legacy spec arguments.
    let home = Home::new("spec-raw-tail");
    home.write("docs/spec/lessons.md", "# Lessons\n");
    let out = home.vibe(&["spec", "lessons-for", "--"], &[]);
    assert_eq!(out.code, 2, "{}", out.stderr);
    assert_eq!(out.stderr, "Unknown option: --\n");
}
