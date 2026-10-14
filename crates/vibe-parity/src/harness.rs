//! Runs oracle and candidate over a suite's corpus and compares the results.
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::sandbox::{ORACLE_DIR, snapshot, stage};
use crate::suite::{Case, CommandSpec, Field, Suite};

/// What the candidate side of each run is.
#[derive(Debug, Clone)]
pub enum Mode {
    /// Run the oracle twice. Proves the corpus and oracle are deterministic.
    SelfCheck,
    /// Run the candidate binary at this path.
    Candidate(PathBuf),
}

/// What one side of a run produced.
#[derive(Debug, PartialEq, Eq)]
pub struct Outcome {
    /// Exit code; `None` when killed by a signal.
    pub exit: Option<i32>,
    /// Stdout bytes.
    pub stdout: Vec<u8>,
    /// Every file left in the sandbox (see [`snapshot`]).
    pub files: BTreeMap<String, Vec<u8>>,
}

/// The verdict for one run.
#[derive(Debug, PartialEq, Eq)]
pub enum Status {
    /// Oracle and candidate agree.
    Pass,
    /// They differ only on pinned fields.
    Pinned(Vec<Field>),
    /// They differ on an unpinned field, or the run could not execute.
    Fail(Vec<String>),
}

/// One command over one case with one argument list.
#[derive(Debug)]
pub struct RunResult {
    /// Command name.
    pub command: String,
    /// Case id, `<class>/<case>`.
    pub case: String,
    /// Extra arguments for this run.
    pub args: Vec<String>,
    /// The verdict.
    pub status: Status,
}

/// Everything a suite run found.
#[derive(Debug, Default)]
pub struct Report {
    /// One entry per run.
    pub results: Vec<RunResult>,
    /// Suite-level problems: floors, unknown classes, commands over no class,
    /// case directories without `case.json`, unknown `runs` keys, stale or
    /// unmatched pins.
    pub errors: Vec<String>,
}

impl Report {
    fn count(&self, f: impl Fn(&Status) -> bool) -> usize {
        self.results.iter().filter(|r| f(&r.status)).count()
    }

    /// Runs that passed.
    pub fn passed(&self) -> usize {
        self.count(|s| matches!(s, Status::Pass))
    }

    /// Runs that differ only on pinned fields.
    pub fn pinned(&self) -> usize {
        self.count(|s| matches!(s, Status::Pinned(_)))
    }

    /// Runs that failed.
    pub fn failed(&self) -> usize {
        self.count(|s| matches!(s, Status::Fail(_)))
    }

    /// True when at least one run happened, none failed, and there are no suite errors.
    pub fn ok(&self) -> bool {
        !self.results.is_empty() && self.failed() == 0 && self.errors.is_empty()
    }
}

fn expand(argv: &[String], oracle: &Path, candidate: Option<&Path>) -> Vec<String> {
    let oracle = oracle.to_string_lossy();
    let candidate = candidate
        .map(|c| c.to_string_lossy().into_owned())
        .unwrap_or_default();
    argv.iter()
        .map(|a| {
            a.replace("{oracle}", &oracle)
                .replace("{candidate}", &candidate)
        })
        .collect()
}

/// Runs `argv` in `cwd` with a cleared environment: only `PATH`, `HOME`,
/// `LC_ALL=C` and `TZ=UTC` are set, and stdin is empty. Returns the exit
/// code and stdout.
pub fn execute(argv: &[String], cwd: &Path, home: &Path) -> Result<(Option<i32>, Vec<u8>), String> {
    let (program, args) = argv.split_first().ok_or("empty argv")?;
    let out = Command::new(program)
        .args(args)
        .current_dir(cwd)
        .env_clear()
        .env("PATH", std::env::var_os("PATH").unwrap_or_default())
        .env("HOME", home)
        .env("LC_ALL", "C")
        .env("TZ", "UTC")
        .stdin(Stdio::null())
        .output()
        .map_err(|e| format!("{program}: {e}"))?;
    Ok((out.status.code(), out.stdout))
}

fn run_side(suite: &Suite, case: &Case, argv: &[String], work: &Path) -> Result<Outcome, String> {
    let dest = work.join("sandbox");
    let home = work.join("home");
    stage(&case.tree(), &suite.oracle_root, &dest)?;
    if home.exists() {
        fs::remove_dir_all(&home).map_err(|e| format!("{}: {e}", home.display()))?;
    }
    fs::create_dir_all(&home).map_err(|e| format!("{}: {e}", home.display()))?;
    let cwd = dest.join(&case.spec.cwd);
    if !cwd.is_dir() {
        return Err(format!(
            "{}: cwd {:?} is not a directory in the tree",
            case.id(),
            case.spec.cwd
        ));
    }
    let (exit, stdout) = execute(argv, &cwd, &home)?;
    let files = snapshot(&dest)?;
    Ok(Outcome {
        exit,
        stdout,
        files,
    })
}

fn first_line_diff(a: &[u8], b: &[u8]) -> String {
    let (la, lb): (Vec<&[u8]>, Vec<&[u8]>) = (
        a.split(|&c| c == b'\n').collect(),
        b.split(|&c| c == b'\n').collect(),
    );
    let n = la.iter().zip(&lb).take_while(|(x, y)| x == y).count();
    let show = |l: &Vec<&[u8]>| l.get(n).map(|s| String::from_utf8_lossy(s).into_owned());
    format!(
        "stdout line {}: oracle {:?}, candidate {:?}",
        n + 1,
        show(&la),
        show(&lb)
    )
}

/// Lists the fields on which `oracle` and `candidate` differ, each with a message.
pub fn compare(oracle: &Outcome, candidate: &Outcome) -> Vec<(Field, String)> {
    let mut diffs = Vec::new();
    if oracle.exit != candidate.exit {
        diffs.push((
            Field::Exit,
            format!(
                "exit: oracle {:?}, candidate {:?}",
                oracle.exit, candidate.exit
            ),
        ));
    }
    if oracle.stdout != candidate.stdout {
        diffs.push((
            Field::Stdout,
            first_line_diff(&oracle.stdout, &candidate.stdout),
        ));
    }
    if oracle.files != candidate.files {
        let keys: BTreeSet<&String> = oracle.files.keys().chain(candidate.files.keys()).collect();
        let differing: Vec<&str> = keys
            .into_iter()
            .filter(|k| oracle.files.get(*k) != candidate.files.get(*k))
            .map(String::as_str)
            .collect();
        diffs.push((Field::Files, format!("files differ: {differing:?}")));
    }
    diffs
}

fn check_floors(suite: &Suite, errors: &mut Vec<String>) {
    for cmd in &suite.commands {
        if cmd.classes.is_empty() {
            errors.push(format!("command {} runs over no class", cmd.name));
        }
    }
    let used: BTreeSet<&str> = suite
        .commands
        .iter()
        .flat_map(|c| c.classes.iter().map(String::as_str))
        .collect();
    for class in &used {
        if !suite.floors.contains_key(*class) {
            errors.push(format!("class {class} has no floor"));
        }
    }
    for (class, &floor) in &suite.floors {
        if !used.contains(class.as_str()) {
            errors.push(format!("floor for class {class}, which no command uses"));
        }
        if floor == 0 {
            errors.push(format!("class {class}: a floor of 0 asserts nothing"));
            continue;
        }
        match suite.cases(class) {
            Ok(cases) if cases.len() < floor => {
                errors.push(format!(
                    "class {class}: {} case(s), floor {floor}",
                    cases.len()
                ));
            }
            Ok(_) => {}
            Err(e) => errors.push(e),
        }
    }
}

fn judge(diffs: Vec<(Field, String)>, pins: &[(usize, Field)], hits: &mut [bool]) -> Status {
    let mut unpinned = Vec::new();
    let mut pinned = Vec::new();
    for (field, msg) in diffs {
        match pins.iter().find(|(_, f)| *f == field) {
            Some(&(i, f)) => {
                if let Some(hit) = hits.get_mut(i) {
                    *hit = true;
                }
                pinned.push(f);
            }
            None => unpinned.push(msg),
        }
    }
    if !unpinned.is_empty() {
        Status::Fail(unpinned)
    } else if pinned.is_empty() {
        Status::Pass
    } else {
        Status::Pinned(pinned)
    }
}

fn run_command(
    suite: &Suite,
    cmd: &CommandSpec,
    mode: &Mode,
    work: &Path,
    report: &mut Report,
    hits: &mut [bool],
) {
    let oracle_dir = work.join("sandbox").join(ORACLE_DIR);
    let candidate = match mode {
        Mode::SelfCheck => None,
        Mode::Candidate(path) => Some(path.as_path()),
    };
    let oracle_argv = expand(&cmd.oracle, &oracle_dir, None);
    let candidate_argv = match candidate {
        None => oracle_argv.clone(),
        Some(_) => expand(&cmd.candidate, &oracle_dir, candidate),
    };
    for class in &cmd.classes {
        let cases = match suite.cases(class) {
            Ok(cases) => cases,
            Err(e) => {
                report.errors.push(e);
                continue;
            }
        };
        for case in &cases {
            let pins: Vec<(usize, Field)> = match mode {
                Mode::SelfCheck => Vec::new(),
                Mode::Candidate(_) => suite
                    .divergences
                    .iter()
                    .enumerate()
                    .filter(|(_, d)| d.command == cmd.name && d.case == case.id())
                    .map(|(i, d)| (i, d.field))
                    .collect(),
            };
            for args in case.arg_sets(&cmd.name) {
                let with = |base: &[String]| base.iter().chain(&args).cloned().collect::<Vec<_>>();
                let outcome = run_side(suite, case, &with(&oracle_argv), work).and_then(|o| {
                    run_side(suite, case, &with(&candidate_argv), work).map(|c| (o, c))
                });
                let status = match outcome {
                    Ok((o, c)) => judge(compare(&o, &c), &pins, hits),
                    Err(e) => Status::Fail(vec![e]),
                };
                report.results.push(RunResult {
                    command: cmd.name.clone(),
                    case: case.id(),
                    args,
                    status,
                });
            }
        }
    }
}

/// Runs every command of `suite` over its classes. `work` is a scratch
/// directory; each run stages its sandbox at the same path, so absolute paths
/// in output match between oracle and candidate.
pub fn run_suite(suite: &Suite, mode: &Mode, work: &Path) -> Report {
    let mut report = Report::default();
    check_floors(suite, &mut report.errors);
    let mut hits = vec![false; suite.divergences.len()];
    for cmd in &suite.commands {
        run_command(suite, cmd, mode, work, &mut report, &mut hits);
    }
    if matches!(mode, Mode::Candidate(_)) {
        for (pin, hit) in suite.divergences.iter().zip(&hits) {
            if !hit {
                report.errors.push(format!(
                    "stale pin: {} {} {:?} no longer diverges (or matches no run)",
                    pin.command, pin.case, pin.field
                ));
            }
        }
    }
    report
}
