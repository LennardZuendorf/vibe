#![cfg(unix)]
#![allow(clippy::unwrap_used, clippy::expect_used)]
//! The committed spec corpus is exactly what the generator writes, and the
//! frozen spec oracles are deterministic over it.
use std::fs;
use std::path::{Path, PathBuf};

use vibe_parity::corpus_spec::{MARKER, generate};
use vibe_parity::harness::{Mode, execute, run_suite};
use vibe_parity::sandbox::{ORACLE_DIR, snapshot, stage};
use vibe_parity::suite::Suite;

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn scratch(name: &str) -> PathBuf {
    let dir =
        std::env::temp_dir().join(format!("vibe-parity-corpus-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

const TEMPLATES: &str = "oracles/bash/spec/reference/templates";

#[test]
fn committed_corpus_matches_the_generator() {
    let out = scratch("regen").join("spec");
    let n = generate(&repo().join(TEMPLATES), &out).unwrap();
    assert!(n >= 12, "generated only {n} cases");
    let committed = snapshot(&repo().join("tests/parity/corpus/spec")).unwrap();
    let fresh = snapshot(&out).unwrap();
    assert_eq!(
        committed.keys().collect::<Vec<_>>(),
        fresh.keys().collect::<Vec<_>>(),
        "file sets differ: run `cargo run -p vibe-parity -- gen-spec-corpus {TEMPLATES} tests/parity/corpus/spec`"
    );
    for (path, bytes) in &committed {
        assert_eq!(
            Some(bytes),
            fresh.get(path),
            "{path} differs from the generator output"
        );
    }
}

#[test]
fn the_generator_refuses_a_directory_it_did_not_write() {
    let out = scratch("refuse");
    fs::write(out.join("keep.txt"), b"mine").unwrap();
    let err = generate(&repo().join(TEMPLATES), &out).unwrap_err();
    assert!(err.contains(MARKER), "{err}");
    assert_eq!(fs::read(out.join("keep.txt")).unwrap(), b"mine");
}

#[test]
fn spec_oracles_are_deterministic_over_the_corpus() {
    let suite = Suite::load(&repo().join("tests/parity/spec.json")).unwrap();
    let report = run_suite(&suite, &Mode::SelfCheck, &scratch("self"));
    assert!(report.ok(), "{:#?}", report.errors);
    assert!(
        report.results.len() >= 60,
        "only {} runs",
        report.results.len()
    );
}

#[test]
fn every_oracle_command_runs() {
    let suite = Suite::load(&repo().join("tests/parity/spec.json")).unwrap();
    let mut printed = 0;
    for (i, cmd) in suite.commands.iter().enumerate() {
        let mut printed_here = false;
        for (j, class) in cmd.classes.iter().enumerate() {
            let case = suite.cases(class).unwrap().into_iter().next().unwrap();
            let dir = scratch(&format!("runs-{i}-{j}"));
            let dest = dir.join("sandbox");
            stage(&case.tree(), &suite.oracle_root, &dest).unwrap();
            let oracle = dest.join(ORACLE_DIR).to_string_lossy().into_owned();
            let mut argv: Vec<String> = cmd
                .oracle
                .iter()
                .map(|a| a.replace("{oracle}", &oracle))
                .collect();
            argv.extend(case.arg_sets(&cmd.name).into_iter().next().unwrap());
            let (code, out) = execute(&argv, &dest.join(&case.spec.cwd), &dir).unwrap();
            assert_ne!(code, Some(127), "{} did not start: {argv:?}", cmd.name);
            assert!(code.is_some(), "{} was killed by a signal", cmd.name);
            printed_here |= !out.is_empty();
        }
        printed += usize::from(printed_here);
    }
    assert!(
        printed >= 5,
        "only {printed} of {} oracle commands printed",
        suite.commands.len()
    );
}

#[test]
fn no_root_cases_stop_at_the_sandbox_git_dir() {
    const NONE: &str = "No .spec/ directory found in current project\n";
    let dir = scratch("escape");
    let dest = dir.join("sandbox");
    fs::create_dir_all(dir.join(".spec")).unwrap();
    fs::write(
        dir.join(".spec/product.md"),
        "---\ntype: entrypoint\nscope: product\nupdated: 2026-01-15\n---\n\n# Decoy\n",
    )
    .unwrap();
    let case = repo().join("tests/parity/corpus/spec/no-root/subdir");
    stage(&case.join("tree"), &repo().join("oracles/bash/spec"), &dest).unwrap();
    let script = dest.join(ORACLE_DIR).join("scripts/list-specs.sh");
    let argv = ["bash".to_string(), script.to_string_lossy().into_owned()];
    let (code, out) = execute(&argv, &dest.join("src/deep"), &dir).unwrap();
    assert_eq!(code, Some(0));
    assert_eq!(String::from_utf8(out).unwrap(), NONE);

    fs::remove_dir_all(dest.join(".git")).unwrap();
    let (_, out) = execute(&argv, &dest.join("src/deep"), &dir).unwrap();
    let out = String::from_utf8(out).unwrap();
    assert!(
        out.contains("product.md"),
        "control: without .git the walk must reach the decoy, got {out:?}"
    );
}

#[test]
fn every_case_tree_turns_the_f17_checks_off() {
    let want = fs::read_to_string(repo().join("tests/parity/legacy-strictness.json")).unwrap();
    let corpus = repo().join("tests/parity/corpus/spec");
    let mut cases = 0;
    for class in fs::read_dir(&corpus)
        .unwrap()
        .flatten()
        .filter(|e| e.path().is_dir())
    {
        for case in fs::read_dir(class.path())
            .unwrap()
            .flatten()
            .filter(|e| e.path().is_dir())
        {
            let got = fs::read_to_string(case.path().join("tree/.vibe/spec.json"))
                .unwrap_or_else(|e| panic!("{}: {e}", case.path().display()));
            assert_eq!(got, want, "{}", case.path().display());
            cases += 1;
        }
    }
    assert!(cases >= 25, "only {cases} cases");
}
