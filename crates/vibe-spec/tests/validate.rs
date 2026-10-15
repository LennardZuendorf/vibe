#![cfg(unix)]
#![allow(clippy::unwrap_used, clippy::expect_used)]
//! `vibe-spec validate` is byte-identical to `validate.sh` (colours, `echo -e`
//! escapes, counters, exit code).
mod common;

use common::{assert_parity, project, write};

const ENTRY: &str = "---\ntype: entrypoint\nscope: product\nchildren:\n  - product-ux.md\nupdated: 2026-01-15\n---\n\n# P\n";
const FEATURE: &str = "---\ntype: feature-product\nfeature: auth\nparent: ../../product.md\nsibling: tech.md\nupdated: 2026-01-15\n---\n\n## Scope\n\n| Owns | Does not own |\n|---|---|\n| x | y |\n";
const FEATURE_TECH: &str = "---\ntype: feature-tech\nfeature: auth\nparent: ../../tech.md\nsibling: product.md\nupdated: 2026-01-15\n---\n\n[design](design.md) [plan](plan.md) [gone](gone.md) [abs](/nonexistent/x.md) [web](http://x.md) [ok](product.md#top)\n";

#[test]
fn a_clean_tree_matches() {
    let p = project("val-clean");
    write(&p, ".spec/product.md", ENTRY);
    write(
        &p,
        ".spec/product-ux.md",
        "---\ntype: product-topic\nparent: product.md\nscope: ux\ncovers: flows\nupdated: 2026-01-15\n---\n\n[up](product.md)\n",
    );
    write(
        &p,
        ".spec/lessons.md",
        "# Lessons\n\n### One\n**Tags:** a\n",
    );
    assert_parity(&p, "validate", "validate.sh", &[]);
    assert_parity(&p.join(".spec"), "validate", "validate.sh", &[]);
}

#[test]
fn root_layer_errors_match() {
    let p = project("val-root");
    write(&p, ".spec/product.md", "# no frontmatter\n");
    write(
        &p,
        ".spec/tech.md",
        "---\ntype: entrypoint\nchildren:\n  - tech-api.md\n  - tech-{topic}.md\n  - tech-gone.md\n-\tbare\nnext: x\n---\n\n[a](b(c).md) [t]({x}.md) [m](missing.md) [s](features/x/)\n",
    );
    write(
        &p,
        ".spec/plan.md",
        "---\ntype: entrypoint\nupdated: 2026-01-15\n---\n",
    );
    write(
        &p,
        ".spec/tech-api.md",
        "---\ntype: tech-topic\nupdated: 2026-01-15\n---\n",
    );
    write(
        &p,
        ".spec/tech-Bad_Topic.md",
        "---\ntype: tech-topic\nparent: tech.md\nscope: s\ncovers: c\nupdated: 2026-01-15\n---\n",
    );
    write(
        &p,
        ".spec/notes.md",
        "---\ntype: note\nupdated: 2026-01-15\n---\n",
    );
    write(
        &p,
        ".spec/design.md",
        "---\ntype: entrypoint\nchildren:\nupdated: 2026-01-15\n---\n",
    );
    write(
        &p,
        "CLAUDE.md",
        "[ok](.spec/plan.md) [bad](.spec/gone.md) [other](docs/x.md)\n",
    );
    assert_parity(&p, "validate", "validate.sh", &[]);
}

#[test]
fn features_sf13_sf15_and_sf16_match() {
    let p = project("val-features");
    write(
        &p,
        ".spec/product.md",
        &format!(
            "{ENTRY}\n[f](features/auth/) [g](features/ghost/) [a](archive/old/) [t](features/{{x}}/)\n"
        ),
    );
    write(
        &p,
        ".spec/product-ux.md",
        "---\ntype: product-topic\nparent: product.md\nscope: ux\ncovers: flows\nupdated: 2026-01-15\n---\n",
    );
    write(&p, ".spec/features/auth/product.md", FEATURE);
    write(&p, ".spec/features/auth/tech.md", FEATURE_TECH);
    write(&p, ".spec/features/auth/design.md", "no frontmatter\n");
    write(
        &p,
        ".spec/features/billing/product.md",
        "---\ntype: product\nfeature: billing\nparent: ../../product.md\nsibling: tech.md\nupdated: 2026-01-15\n---\n\n## Scope\n\nOwns invoices; does not own auth.\n",
    );
    write(
        &p,
        ".spec/plan.md",
        &format!(
            "---\ntype: entrypoint\nchildren:\nupdated: 2026-01-15\n---\n{}",
            "line\n".repeat(301)
        ),
    );
    write(
        &p,
        ".spec/lessons.md",
        "# Lessons\n\n### Escape \\t in titles\n**Rule:** x\n\n### Tagged\n**Tags:** a\n\n### Last\n**Tags:** no newline",
    );
    if cfg!(target_os = "macos") {
        // BSD wc pads the SF15 count (P3a-R2); compare everything else.
        let want = common::oracle(&p, "validate.sh", &[]).text();
        let got = common::spec(&p, &["validate"], &[]).text();
        let squash = |s: &str| s.split_whitespace().collect::<Vec<_>>().join(" ");
        assert_eq!(squash(&got), squash(&want));
    } else {
        assert_parity(&p, "validate", "validate.sh", &[]);
    }
}

#[test]
fn degenerate_trees_match() {
    let empty = project("val-empty");
    write(&empty, ".spec/notes.txt", "x\n");
    assert_parity(&empty, "validate", "validate.sh", &[]);
}

#[test]
fn no_root_exits_1_and_names_init() {
    let p = project("val-none");
    let out = common::spec(&p, &["validate"], &[]);
    assert_eq!(out.code, 1);
    assert_eq!(
        out.text(),
        "Validating .spec/...\n\n\x1b[31m  ERROR: No spec root found. Run setup first: vibe-spec init\x1b[0m\n"
    );
}

#[test]
fn arguments_are_a_usage_error_until_f17() {
    let p = project("val-args");
    write(&p, ".spec/product.md", ENTRY);
    assert_eq!(common::spec(&p, &["validate", "--strict"], &[]).code, 2);
}

#[test]
fn design_lint_degrades_like_the_oracle() {
    use std::os::unix::fs::PermissionsExt;
    let p = project("val-lint");
    write(&p, ".spec/product.md", ENTRY);
    write(
        &p,
        ".spec/product-ux.md",
        "---\ntype: product-topic\nparent: product.md\nscope: ux\ncovers: flows\nupdated: 2026-01-15\n---\n",
    );
    write(
        &p,
        ".spec/design.md",
        "---\ntype: entrypoint\nchildren:\nupdated: 2026-01-15\n---\n",
    );
    let bin = p.join("fakebin");
    for (name, body) in [
        ("node", "#!/bin/sh\nexit 0\n"),
        (
            "npx",
            "#!/bin/sh\necho \"lint: $4 has issues\" >&2\necho 'second\\tline'\nexit 1\n",
        ),
        ("curl", "#!/bin/sh\nexit 0\n"),
    ] {
        write(&p, &format!("fakebin/{name}"), body);
        std::fs::set_permissions(bin.join(name), std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    let path = format!("{}:{}", bin.display(), std::env::var("PATH").unwrap());
    let env = [("VIBE_DESIGN_LINT", "1"), ("PATH", path.as_str())];
    let want = common::oracle_env(&p, "validate.sh", &[], &env);
    let got = common::spec(&p, &["validate"], &env);
    assert_eq!((got.code, got.text()), (want.code, want.text()));
    assert!(
        got.text().contains("lint: .spec/design.md has issues"),
        "{}",
        got.text()
    );
}

#[test]
fn sf15_threshold_follows_bash_arithmetic() {
    let p = project("val-sf15");
    write(
        &p,
        ".spec/product.md",
        &format!("{ENTRY}{}", "x\n".repeat(4)),
    );
    write(&p, ".spec/plan.md", "");
    for v in ["010", "1x", "-5", "0x10", "09", " 5 ", "+7"] {
        let env = [("SPEC_ROOT_MAX_LINES", v)];
        let want = common::oracle_env(&p, "validate.sh", &[], &env);
        let got = common::spec(&p, &["validate"], &env);
        // BSD wc pads the SF15 count (P3a-R2); squash whitespace.
        let squash = |s: &str| s.split_whitespace().collect::<Vec<_>>().join(" ");
        assert_eq!(
            (got.code, squash(&got.text())),
            (want.code, squash(&want.text())),
            "SPEC_ROOT_MAX_LINES={v:?}"
        );
    }
}
