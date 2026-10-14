//! `vibe-parity`: run a parity suite.
//!
//! Usage:
//!   vibe-parity run <suite.json> --self
//!   vibe-parity run <suite.json> --candidate <binary>
//!   vibe-parity gen-spec-corpus <templates-dir> <out-dir>
//!
//! Exit 0 when every run passes or is pinned, 1 on any failure or suite
//! error, 2 on a usage error.
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use vibe_core::exit;
use vibe_parity::harness::{Mode, Status, run_suite};
use vibe_parity::suite::Suite;

const USAGE: &str = "usage: vibe-parity run <suite.json> (--self | --candidate <binary>)\n       vibe-parity gen-spec-corpus <templates-dir> <out-dir>";

fn usage(msg: &str) -> ExitCode {
    eprintln!("vibe-parity: {msg}\n{USAGE}");
    ExitCode::from(exit::USAGE)
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let rest: Vec<&str> = args.iter().map(String::as_str).collect();
    match rest.as_slice() {
        ["run", suite, "--self"] => run(Path::new(suite), Mode::SelfCheck),
        ["run", suite, "--candidate", bin] => match std::fs::canonicalize(bin) {
            Ok(path) => run(Path::new(suite), Mode::Candidate(path)),
            Err(e) => usage(&format!("candidate {bin}: {e}")),
        },
        ["gen-spec-corpus", templates, out] => {
            match vibe_parity::corpus_spec::generate(Path::new(templates), Path::new(out)) {
                Ok(n) => {
                    println!("gen-spec-corpus: {n} cases written to {out}");
                    ExitCode::from(exit::OK)
                }
                Err(e) => {
                    eprintln!("vibe-parity: {e}");
                    ExitCode::from(exit::FINDINGS)
                }
            }
        }
        _ => usage("bad arguments"),
    }
}

fn run(suite_path: &Path, mode: Mode) -> ExitCode {
    let suite = match Suite::load(suite_path) {
        Ok(suite) => suite,
        Err(e) => return usage(&e),
    };
    let work: PathBuf = std::env::temp_dir().join(format!("vibe-parity-{}", std::process::id()));
    let report = run_suite(&suite, &mode, &work);
    let _ = std::fs::remove_dir_all(&work);
    for r in &report.results {
        let args = r.args.join(" ");
        match &r.status {
            Status::Pass => println!("PASS  {} {} [{args}]", r.command, r.case),
            Status::Pinned(fields) => {
                println!("PIN   {} {} [{args}] {fields:?}", r.command, r.case)
            }
            Status::Fail(msgs) => {
                println!("FAIL  {} {} [{args}]", r.command, r.case);
                for m in msgs {
                    println!("      {m}");
                }
            }
        }
    }
    for e in &report.errors {
        println!("ERROR {e}");
    }
    let label = match mode {
        Mode::SelfCheck => "self-check".to_string(),
        Mode::Candidate(p) => format!("candidate {}", p.display()),
    };
    println!(
        "parity[{}]: {} passed, {} pinned, {} failed, {} error(s), {} runs ({label})",
        suite.name,
        report.passed(),
        report.pinned(),
        report.failed(),
        report.errors.len(),
        report.results.len()
    );
    ExitCode::from(if report.ok() {
        exit::OK
    } else {
        exit::FINDINGS
    })
}
