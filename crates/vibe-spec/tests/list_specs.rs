#![cfg(unix)]
#![allow(clippy::unwrap_used, clippy::expect_used)]
//! `vibe-spec list-specs` is byte-identical to `list-specs.sh`.
mod common;

use common::{assert_parity, project, write};

const DOC: &str = "---\ntype: entrypoint\nscope: product\nupdated: 2026-01-15\n---\n\n# P\n";

#[test]
fn matches_the_oracle_on_root_docs_and_features() {
    let p = project("ls-full");
    write(&p, ".spec/product.md", DOC);
    write(
        &p,
        ".spec/tech-api.md",
        "---\ntype: tech-topic\nscope:x\n---\n",
    );
    write(&p, ".spec/odd.md", "no frontmatter, type: inline\n");
    write(&p, ".spec/lessons.md", "# Lessons\n");
    write(
        &p,
        ".spec/features/auth/product.md",
        "---\ntype: feature-product\nscope: auth\n---\n",
    );
    write(
        &p,
        ".spec/features/auth/plan.md",
        "---\ntype: feature-plan\n---\n",
    );
    write(&p, ".spec/features/empty/notes.txt", "x\n");
    write(&p, "src/deep/x.txt", "x\n");
    assert_parity(&p, "list-specs", "list-specs.sh", &[]);
    assert_parity(&p.join("src/deep"), "list-specs", "list-specs.sh", &[]);
    assert_parity(&p, "list-specs", "list-specs.sh", &["ignored", "--args"]);
}

#[test]
fn matches_the_oracle_without_specs_or_root() {
    let p = project("ls-empty");
    write(&p, ".spec/notes.txt", "x\n");
    assert_parity(&p, "list-specs", "list-specs.sh", &[]);
    let none = project("ls-none");
    assert_parity(&none, "list-specs", "list-specs.sh", &[]);
}

#[test]
fn lists_md_entries_of_any_type_like_the_glob() {
    let p = project("ls-types");
    write(&p, ".spec/product.md", DOC);
    write(&p, ".spec/features/auth/product.md", DOC);
    for base in [".spec", ".spec/features/auth"] {
        let dir = p.join(base);
        std::fs::create_dir_all(dir.join("dir.md")).unwrap();
        std::os::unix::fs::symlink("nowhere", dir.join("broken.md")).unwrap();
    }
    assert_parity(&p, "list-specs", "list-specs.sh", &[]);
}

#[test]
fn sorts_prefix_named_features_like_the_glob() {
    let p = project("ls-prefix");
    write(&p, ".spec/product.md", DOC);
    for f in ["spec", "spec-delta"] {
        write(
            &p,
            &format!(".spec/features/{f}/product.md"),
            "---\ntype: feature-product\nscope: x\n---\n",
        );
    }
    assert_parity(&p, "list-specs", "list-specs.sh", &[]);
}
