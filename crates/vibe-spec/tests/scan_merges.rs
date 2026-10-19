#![cfg(unix)]
// vibe-spec: literal-file
#![allow(clippy::unwrap_used, clippy::expect_used)]
//! `vibe-spec scan-merges` is byte-identical to `scan-merges.sh`.
mod common;

use common::{assert_parity, project, write};

const HEAD: &str = "---\ntype: feature-tech\nupdated: 2026-01-15\n---\n\n";

#[test]
fn matches_the_oracle_across_formats_and_errors() {
    let p = project("merges");
    write(
        &p,
        ".spec/features/auth/tech.md",
        &format!(
            "{HEAD}<!-- merge -->\n\n| Key | Value |\n<!-- /merge -->\n<!-- /merge -->\n<!-- merge -->\nx\n<!-- merge -->\nnested\n<!-- /merge -->\n"
        ),
    );
    write(
        &p,
        ".spec/features/billing-and-invoicing-long-name/tech.md",
        &format!(
            "{HEAD}<!-- merge -->\n{}\n<!-- /merge -->\n",
            "y".repeat(80)
        ),
    );
    write(&p, ".spec/features/draft/notes.md", "no tech\n");
    for args in [
        &[][..],
        &["--format", "plain"],
        &["--format=table"],
        &["auth"],
        &["missing"],
        &["--bogus"],
        &["--format"],
        &[""],
        &["auth", ""],
        &["--"],
        &["--", "auth"],
        &["auth/"],
        &["/auth"],
        &["//auth"],
        &["./auth"],
        &["."],
        &[".."],
    ] {
        assert_parity(&p, "scan-merges", "scan-merges.sh", args);
    }
}

#[test]
fn matches_the_oracle_without_features_or_root() {
    let p = project("merges-empty");
    write(&p, ".spec/product.md", "x\n");
    assert_parity(&p, "scan-merges", "scan-merges.sh", &[]);
    assert_parity(&p, "scan-merges", "scan-merges.sh", &["auth"]);
    let none = project("merges-none");
    assert_parity(&none, "scan-merges", "scan-merges.sh", &[]);
    assert_parity(&none, "scan-merges", "scan-merges.sh", &["auth"]);
}

#[test]
fn sorts_prefix_named_features_like_the_glob() {
    let p = project("merges-prefix");
    for f in ["spec", "spec-delta"] {
        write(
            &p,
            &format!(".spec/features/{f}/tech.md"),
            &format!("{HEAD}<!-- merge -->\nx\n<!-- /merge -->\n"),
        );
    }
    assert_parity(&p, "scan-merges", "scan-merges.sh", &[]);
}
