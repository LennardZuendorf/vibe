//! Runs oracle and candidate over a suite's corpus and compares the results.
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

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
    /// Stderr bytes. Compared only for cases that set `stderr: true`.
    pub stderr: Vec<u8>,
    /// Every file left in the sandbox (see [`snapshot`]).
    pub files: BTreeMap<String, Vec<u8>>,
}

/// The verdict for one run.
#[derive(Debug, PartialEq, Eq, Clone)]
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
    /// Commands candidate mode skipped because they are not ported yet.
    pub skipped: Vec<String>,
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

/// [`execute_with_timeout`] with a 30 s limit.
pub fn execute(argv: &[String], cwd: &Path, home: &Path) -> Result<(Option<i32>, Vec<u8>), String> {
    execute_with_timeout(argv, cwd, home, Duration::from_secs(30))
}

/// Runs `argv` in `cwd` with a cleared environment: only `PATH`, `HOME`,
/// `LC_ALL=C` and `TZ=UTC` are set, stdin is empty, stderr is discarded.
/// Returns the exit code and stdout. A run still going after `limit` is
/// killed and reported as an error.
pub fn execute_with_timeout(
    argv: &[String],
    cwd: &Path,
    home: &Path,
    limit: Duration,
) -> Result<(Option<i32>, Vec<u8>), String> {
    let (exit, stdout, _) = run_child(argv, cwd, home, None, &BTreeMap::new(), limit)?;
    Ok((exit, stdout))
}

/// Runs one side of a case: cleared env plus the case's extra variables,
/// stdin from a file when given (never a pipe), stderr captured, with the
/// same wall-clock discipline as [`execute_with_timeout`].
type Streams = (Option<i32>, Vec<u8>, Vec<u8>);

fn run_child(
    argv: &[String],
    cwd: &Path,
    home: &Path,
    stdin: Option<&Path>,
    env_extra: &BTreeMap<String, String>,
    limit: Duration,
) -> Result<Streams, String> {
    let (program, args) = argv.split_first().ok_or("empty argv")?;
    let stdin_file = match stdin {
        Some(path) => {
            let file = fs::File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
            Stdio::from(file)
        }
        None => Stdio::null(),
    };
    let mut child = Command::new(program)
        .args(args)
        .current_dir(cwd)
        .env_clear()
        .env("PATH", std::env::var_os("PATH").unwrap_or_default())
        .env("HOME", home)
        .env("LC_ALL", "C")
        .env("TZ", "UTC")
        .envs(env_extra)
        .stdin(stdin_file)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("{program}: {e}"))?;
    let mut out_pipe = child.stdout.take().ok_or("no stdout pipe")?;
    let out_reader = std::thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = out_pipe.read_to_end(&mut buf);
        buf
    });
    let mut err_pipe = child.stderr.take().ok_or("no stderr pipe")?;
    let err_reader = std::thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = err_pipe.read_to_end(&mut buf);
        buf
    });
    let start = Instant::now();
    let status = loop {
        match child.try_wait().map_err(|e| format!("{program}: {e}"))? {
            Some(status) => break status,
            None if start.elapsed() >= limit => {
                let _ = child.kill();
                let _ = child.wait();
                // A grandchild may still hold the pipe; the reader threads are
                // left to finish on their own.
                return Err(format!(
                    "{program}: timed out after {} ms",
                    limit.as_millis()
                ));
            }
            None => std::thread::sleep(Duration::from_millis(5)),
        }
    };
    let stdout = out_reader
        .join()
        .map_err(|_| format!("{program}: stdout reader panicked"))?;
    let stderr = err_reader
        .join()
        .map_err(|_| format!("{program}: stderr reader panicked"))?;
    Ok((status.code(), stdout, stderr))
}

/// Applies a case's stamps, git sandboxing, stdin file and extra env to a
/// freshly staged sandbox, then runs `argv` there. Returns the outcome.
fn run_side(
    suite: &Suite,
    case: &Case,
    argv: &[String],
    work: &Path,
    limit: Duration,
) -> Result<Outcome, String> {
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
    apply_stamps(&dest, &case.spec.stamps)?;
    let git = case.spec.git.as_ref();
    if git.is_some_and(|g| g.init) {
        sandbox_git(&dest, git.is_some_and(|g| g.commit))?;
    }
    let stdin_path = if case.spec.stdin.is_empty() {
        None
    } else {
        let path = work.join("stdin.bin");
        fs::write(&path, case.spec.stdin.as_bytes())
            .map_err(|e| format!("{}: {e}", path.display()))?;
        Some(path)
    };
    // The discovery ceiling is unconditional: an empty `.git/` stub bounds
    // marker searches but NOT git's own repository discovery, and a child
    // that runs git must never see the enclosing real repository.
    let mut env_extra = case.spec.env.clone();
    env_extra.insert(
        "GIT_CEILING_DIRECTORIES".to_string(),
        work.display().to_string(),
    );
    if git.is_some() {
        env_extra.insert("GIT_CONFIG_GLOBAL".to_string(), "/dev/null".to_string());
        env_extra.insert("GIT_CONFIG_SYSTEM".to_string(), "/dev/null".to_string());
    }
    let (exit, stdout, stderr) =
        run_child(argv, &cwd, &home, stdin_path.as_deref(), &env_extra, limit)?;
    let files = snapshot(&dest)?;
    Ok(Outcome {
        exit,
        stdout,
        stderr,
        files,
    })
}

/// Sets the mtimes a case's `stamps` map names. A path that does not exist
/// fails loudly — a missing input must never read as "no stamp".
fn apply_stamps(dest: &Path, stamps: &BTreeMap<String, String>) -> Result<(), String> {
    use std::fs::FileTimes;
    for (rel, stamp) in stamps {
        let secs =
            crate::suite::parse_rfc3339(stamp).map_err(|e| format!("stamps[{rel:?}]: {e}"))?;
        let time = std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(secs);
        let path = dest.join(rel);
        let file = fs::OpenOptions::new()
            .write(true)
            .open(&path)
            .map_err(|e| format!("{}: {e}", path.display()))?;
        let times = FileTimes::new().set_modified(time);
        file.set_times(times)
            .map_err(|e| format!("{}: {e}", path.display()))?;
    }
    Ok(())
}

/// `git init` (+ one setup commit) inside the sandbox, with the ambient
/// identity and signing neutralized. A non-zero exit fails the case loudly:
/// a git-dependent case must never run against an unsetup repository.
fn sandbox_git(dest: &Path, commit: bool) -> Result<(), String> {
    let run = |args: &[&str]| -> Result<(), String> {
        let out = Command::new("git")
            .args(args)
            .current_dir(dest)
            .env_clear()
            .env("PATH", std::env::var_os("PATH").unwrap_or_default())
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_SYSTEM", "/dev/null")
            .output()
            .map_err(|e| format!("git {}: {e}", args.first().unwrap_or(&"")))?;
        if !out.status.success() {
            return Err(format!(
                "git {}: {}",
                args.first().unwrap_or(&""),
                String::from_utf8_lossy(&out.stderr).trim_end()
            ));
        }
        Ok(())
    };
    run(&["init"])?;
    if commit {
        run(&[
            "-c",
            "user.email=t@t",
            "-c",
            "user.name=t",
            "-c",
            "commit.gpgsign=false",
            "-c",
            "tag.gpgsign=false",
            "-c",
            "core.hooksPath=/dev/null",
            "add",
            "-A",
        ])?;
        run(&[
            "-c",
            "user.email=t@t",
            "-c",
            "user.name=t",
            "-c",
            "commit.gpgsign=false",
            "-c",
            "tag.gpgsign=false",
            "-c",
            "core.hooksPath=/dev/null",
            "commit",
            "-m",
            "parity setup",
        ])?;
    }
    Ok(())
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

/// Lists the fields on which `oracle` and `candidate` differ, each with a
/// message. File keys are first passed through the suite's aliases and file
/// bytes through its normalizers, on both sides, so the same logical file
/// written to different paths or with nondeterministic bytes compares equal.
/// Stderr is compared only for cases that opt in.
pub fn compare(
    suite: &Suite,
    compare_stderr: bool,
    oracle: &Outcome,
    candidate: &Outcome,
) -> Result<Vec<(Field, String)>, String> {
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
    if compare_stderr && oracle.stderr != candidate.stderr {
        diffs.push((
            Field::Stderr,
            format!(
                "stderr line 1: oracle {:?}, candidate {:?}",
                oracle
                    .stderr
                    .split(|&c| c == b'\n')
                    .next()
                    .unwrap_or_default(),
                candidate
                    .stderr
                    .split(|&c| c == b'\n')
                    .next()
                    .unwrap_or_default()
            ),
        ));
    }
    let oracle_files = suite.canonical_files(oracle.files.clone())?;
    let candidate_files = suite.canonical_files(candidate.files.clone())?;
    if oracle_files != candidate_files {
        let keys: BTreeSet<&String> = oracle_files.keys().chain(candidate_files.keys()).collect();
        let differing: Vec<&str> = keys
            .into_iter()
            .filter(|k| oracle_files.get(*k) != candidate_files.get(*k))
            .map(String::as_str)
            .collect();
        diffs.push((Field::Files, format!("files differ: {differing:?}")));
    }
    Ok(diffs)
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
    let oracle_argv = expand(&cmd.oracle, &oracle_dir, None);
    let candidate_argv = match (mode, &cmd.candidate) {
        (Mode::SelfCheck, _) => oracle_argv.clone(),
        (Mode::Candidate(path), Some(argv)) => expand(argv, &oracle_dir, Some(path)),
        (Mode::Candidate(_), None) => {
            report.skipped.push(cmd.name.clone());
            return;
        }
    };
    let limit = Duration::from_millis(suite.timeout_ms);
    for class in &cmd.classes {
        let cases = match suite.cases(class) {
            Ok(cases) => cases,
            Err(e) => {
                report.errors.push(e);
                continue;
            }
        };
        for case in &cases {
            for args in case.arg_sets(&cmd.name) {
                let pins: Vec<(usize, Field)> = match mode {
                    Mode::SelfCheck => Vec::new(),
                    Mode::Candidate(_) => suite
                        .divergences
                        .iter()
                        .enumerate()
                        .filter(|(_, d)| {
                            d.command == cmd.name
                                && d.case == case.id()
                                && d.active()
                                && d.args.as_ref().is_none_or(|a| *a == args)
                        })
                        .map(|(i, d)| (i, d.field))
                        .collect(),
                };
                let with = |base: &[String]| base.iter().chain(&args).cloned().collect::<Vec<_>>();
                let outcome =
                    run_side(suite, case, &with(&oracle_argv), work, limit).and_then(|o| {
                        run_side(suite, case, &with(&candidate_argv), work, limit).map(|c| (o, c))
                    });
                let status = match outcome {
                    Ok((o, c)) => match compare(suite, case.spec.stderr, &o, &c) {
                        Ok(diffs) => judge(diffs, &pins, hits),
                        Err(e) => Status::Fail(vec![e]),
                    },
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
            if !hit && pin.os.is_none() {
                let args = pin
                    .args
                    .as_ref()
                    .map(|a| format!(" {a:?}"))
                    .unwrap_or_default();
                report.errors.push(format!(
                    "stale pin: {} {}{args} {:?} no longer diverges (or matches no run)",
                    pin.command, pin.case, pin.field
                ));
            }
        }
    }
    report
}

// Test-only re-exports of the private case-field helpers: the integration
// tests prove stdin/env/stderr delivery, stamp application and git setup
// directly, without going through a whole suite run each time.
#[doc(hidden)]
pub fn run_child_for_tests(
    argv: &[String],
    cwd: &Path,
    home: &Path,
    stdin: Option<&Path>,
    env_extra: &BTreeMap<String, String>,
) -> Result<Streams, String> {
    run_child(argv, cwd, home, stdin, env_extra, Duration::from_secs(30))
}

#[doc(hidden)]
pub fn apply_stamps_for_tests(
    dest: &Path,
    stamps: &BTreeMap<String, String>,
) -> Result<(), String> {
    apply_stamps(dest, stamps)
}

#[doc(hidden)]
pub fn sandbox_git_for_tests(dest: &Path, commit: bool) -> Result<(), String> {
    sandbox_git(dest, commit)
}
