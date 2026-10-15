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
        common::legacy(&p);
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
fn unknown_arguments_are_a_usage_error() {
    let p = project("val-args");
    write(&p, ".spec/product.md", ENTRY);
    assert_eq!(common::spec(&p, &["validate", "--bogus"], &[]).code, 2);
    assert_eq!(common::spec(&p, &["validate", "extra"], &[]).code, 2);
    assert_ne!(common::spec(&p, &["validate", "--strict"], &[]).code, 2);
}

#[test]
fn design_lint_degrades_like_the_oracle() {
    use std::os::unix::fs::PermissionsExt;
    let p = project("val-lint");
    common::legacy(&p);
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
    common::legacy(&p);
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

const AUTH_PRODUCT: &str = "---\ntype: feature-product\nfeature: auth\nparent: ../../product.md\nsibling: tech.md\nupdated: 2026-01-15\n---\n\n## Scope\n\n| | |\n|---|---|\n| **Owns** | session cookies, login form, ui |\n| **Does not own** | billing |\n\n## Requirements\n\n### Requirement: Sign in (R1)\nUsers MUST sign in.\n#### Scenario: ok\nWorks.\n\n### Requirement: Sign out (R10)\nThey leave.\n\n### Requirement:\nEmpty title.\n\n## Requirements Trace\n\n| R2 | missing |\n";
const AUTH_PLAN: &str = "---\ntype: feature-plan\nfeature: auth\nparent: ../../plan.md\nupdated: 2026-01-15\n---\n\nCovers R10.\n\n### auth/1 — Login\n\n### auth/1 — Login again\n\n### billing/2 — Cross\n\nDepends on billing/3, auth/2 and x_y/4.\n";
const BILLING_PRODUCT: &str = "---\ntype: feature-product\nfeature: billing\nparent: ../../product.md\nsibling: tech.md\nupdated: 2026-01-15\n---\n\n## Scope\n\n| Owns | Does not own |\n|---|---|\n| `Session Cookies` | auth |\n| invoices | |\n";

#[test]
fn content_checks_match() {
    let p = project("val-content");
    write(
        &p,
        ".spec/product.md",
        "---\ntype: entrypoint\nscope: product\nchildren:\nupdated: 2026-01-15\n---\n\n### Requirement: Users sign in\nThey do.\n\n### Requirement: Export SHALL work\n#### Scenario: ok\n",
    );
    write(
        &p,
        ".spec/plan.md",
        "---\ntype: entrypoint\nupdated: 2026-01-15\n---\n\n## M1 — Alpha\n\n### auth/1 — leaked\n\n| auth/2 | row |\n|  AB12  | caps |\n\n## Notes\n\n### auth/3 — fine\n",
    );
    write(
        &p,
        ".spec/design.md",
        "---\ntype: entrypoint\nchildren:\ncolors:\ntypography:\n  body: Inter\nrounded:\n# note\nspacing: 4px\ncomponents:\nupdated: 2026-01-15\n---\n",
    );
    write(&p, ".spec/features/auth/product.md", AUTH_PRODUCT);
    write(
        &p,
        ".spec/features/auth/tech.md",
        "---\ntype: feature-tech\nupdated: 2026-01-15\n---\n",
    );
    write(&p, ".spec/features/auth/plan.md", AUTH_PLAN);
    write(
        &p,
        ".spec/features/auth/design.md",
        "---\ntype: feature-design\nfeature: auth\nparent: x\nupdated: 2026-01-15\ncolors:\n---\n",
    );
    write(&p, ".spec/features/billing/product.md", BILLING_PRODUCT);
    write(
        &p,
        ".spec/features/billing/plan.md",
        "---\ntype: feature-plan\nupdated: 2026-01-15\n---\n\nNo units.\n",
    );
    write(
        &p,
        ".spec/features/billing/tech.md",
        "---\ntype: feature-tech\nfeature: billing\nparent: x\nsibling: y\nupdated: 2026-01-15\n---\n",
    );
    assert_parity(&p, "validate", "validate.sh", &[]);
}

#[test]
fn milestone_warnings_print_but_do_not_count() {
    let p = project("val-subshell");
    write(
        &p,
        ".spec/plan.md",
        "---\ntype: entrypoint\nchildren:\nupdated: 2026-01-15\n---\n\n## M1\n\n### auth/1 — leaked\n",
    );
    let out = common::spec(&p, &["validate"], &[]);
    assert!(
        out.text().contains("milestone section embeds unit heading"),
        "{}",
        out.text()
    );
    assert!(
        out.text().ends_with("Errors:   0\nWarnings: 0\n"),
        "{}",
        out.text()
    );
    assert_parity(&p, "validate", "validate.sh", &[]);
}

#[test]
fn claude_md_links_to_an_outside_root_are_checked_as_is() {
    let p = project("val-claude-abs");
    let outside = project("val-claude-abs-root");
    let root = outside.join(".spec");
    write(&outside, ".spec/product.md", ENTRY);
    let link = format!("{}/product.md", root.display());
    write(&p, "CLAUDE.md", &format!("# C\n\n[spec]({link})\n"));
    let out = common::spec(&p, &["--root", root.to_str().unwrap(), "validate"], &[]);
    assert!(
        !out.text().contains("CLAUDE.md: broken link"),
        "{}",
        out.text()
    );
}
