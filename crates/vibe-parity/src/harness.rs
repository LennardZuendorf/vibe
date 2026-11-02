//! Runs oracle and candidate over a suite's corpus and compares the results.
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use crate::sandbox::{ORACLE_DIR, snapshot, stage};
use crate::suite::{
    Case, CommandSpec, Divergence, Expect, Field, FileExpect, FileSideExpect, Suite, exit_code,
};

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

/// A side's line bytes split into lines. No bytes means no lines: a trailing
/// newline opens no extra line, and an empty stream is not one empty line.
fn split_lines(bytes: &[u8]) -> Vec<&[u8]> {
    if bytes.is_empty() {
        return Vec::new();
    }
    let mut lines: Vec<&[u8]> = bytes.split(|&c| c == b'\n').collect();
    if lines.last().is_some_and(|line| line.is_empty()) {
        lines.pop();
    }
    lines
}

/// Renders one line's bytes as text a pin can hold: every byte that is part
/// of a valid UTF-8 sequence passes through, and every byte that is not
/// renders as `\xNN`. Without this, a preview cut inside a sequence and the
/// port's U+FFFD replacement would render identically and no pin could tell
/// them apart.
fn render_line(bytes: &[u8]) -> String {
    let mut out = String::new();
    let mut rest = bytes;
    loop {
        match std::str::from_utf8(rest) {
            Ok(valid) => {
                out.push_str(valid);
                return out;
            }
            Err(e) => {
                let good = e.valid_up_to();
                out.push_str(std::str::from_utf8(&rest[..good]).unwrap_or_default());
                let bad = e.error_len().unwrap_or(rest.len() - good);
                for byte in &rest[good..good + bad] {
                    out.push_str(&format!("\\x{byte:02x}"));
                }
                rest = &rest[good + bad..];
            }
        }
    }
}

/// The first line the two sides disagree on: its 1-based number, and each
/// side's line rendered. A side that has no such line is `None`.
fn first_differing_line(a: &[u8], b: &[u8]) -> Option<(usize, Option<String>, Option<String>)> {
    let (left, right) = (split_lines(a), split_lines(b));
    let same = left.iter().zip(&right).take_while(|(x, y)| x == y).count();
    if same == left.len() && same == right.len() {
        return None;
    }
    let side = |lines: &[&[u8]]| lines.get(same).map(|line| render_line(line));
    Some((same, side(&left), side(&right)))
}

/// The difference between two byte-unequal streams: the first line they
/// disagree on, or — when every line agrees and the streams differ only in
/// whether they end with a newline — a difference no pin can state. Splitting
/// into lines drops a final newline, so the two sides being equal *as lines*
/// is not equality: the byte comparison decides, this only describes it.
fn stream_difference(name: &str, oracle: &[u8], candidate: &[u8]) -> (String, Observed) {
    match first_differing_line(oracle, candidate) {
        Some((line, left, right)) => (
            format!(
                "{name} line {}: oracle {left:?}, candidate {right:?}",
                line + 1
            ),
            Observed::Line {
                oracle: left,
                candidate: right,
            },
        ),
        None => {
            let side = |end: &[u8]| {
                if end.ends_with(b"\n") {
                    "does"
                } else {
                    "does not"
                }
            };
            (
                format!(
                    "{name}: unpinnable — the lines all agree, and only the final \
                     newline differs (oracle {}, candidate {})",
                    side(oracle),
                    side(candidate)
                ),
                Observed::FinalNewline {
                    oracle: oracle.ends_with(b"\n"),
                    candidate: candidate.ends_with(b"\n"),
                },
            )
        }
    }
}

/// A difference between the two sides: the message a report prints, and the
/// values a pin must state to cover it.
#[derive(Debug)]
pub struct Difference {
    /// The field that differs.
    pub field: Field,
    /// One line naming the first difference, as a report prints it.
    pub message: String,
    /// The values that difference showed, for the pin to match against.
    pub observed: Observed,
}

/// What one side actually showed for a differing field.
#[derive(Debug, PartialEq, Eq)]
pub enum Observed {
    /// Both exit codes; `None` means killed by a signal.
    Exit {
        /// The oracle's code.
        oracle: Option<i32>,
        /// The candidate's code.
        candidate: Option<i32>,
    },
    /// The first differing line on each side, rendered by [`render_line`];
    /// `None` when that side has no such line.
    Line {
        /// The oracle's line.
        oracle: Option<String>,
        /// The candidate's line.
        candidate: Option<String>,
    },
    /// Every line agrees and the streams differ only in whether they end with
    /// a newline: a byte difference with no differing line, so no pin can
    /// state it (see [`Observed::unpinnable`]).
    FinalNewline {
        /// Whether the oracle's stream ends with a newline.
        oracle: bool,
        /// Whether the candidate's stream ends with a newline.
        candidate: bool,
    },
    /// One record per snapshot key that differs.
    Files(Vec<FileObservation>),
}

impl Observed {
    /// True when no pin can state this difference: it is a byte difference
    /// with no line to address. A run carrying one fails as unpinnable rather
    /// than reporting `PINNED`, because a pin that matched it would be the
    /// field-name-only pin again — the values it names would describe nothing.
    fn unpinnable(&self) -> bool {
        match self {
            Observed::FinalNewline { .. } => true,
            Observed::Files(files) => files.iter().any(|file| file.tail_only),
            Observed::Exit { .. } | Observed::Line { .. } => false,
        }
    }
}

/// One differing snapshot key as observed.
#[derive(Debug, PartialEq, Eq)]
pub struct FileObservation {
    /// The snapshot key.
    pub key: String,
    /// What the oracle side showed for it.
    pub oracle: FileSideObserved,
    /// What the candidate side showed for it.
    pub candidate: FileSideObserved,
    /// Both sides left the key, their lines all agree, and the bytes differ
    /// only in whether the file ends with a newline. Then both sides read as
    /// `Present(None)` — which a pin could take for two empty files — so the
    /// key carries the fact that nothing line-shaped distinguishes them.
    pub tail_only: bool,
}

/// One side of a differing key.
#[derive(Debug, PartialEq, Eq)]
pub enum FileSideObserved {
    /// The side left no such key in its snapshot.
    Absent,
    /// The side left the key; `None` means it has no lines (a directory, or
    /// an empty file).
    Present(Option<String>),
}

impl Observed {
    /// The pin shape for these values: pasting it under `expect` states
    /// exactly what was observed.
    pub fn as_expect(&self) -> String {
        let value = match self {
            Observed::Exit { oracle, candidate } => {
                serde_json::json!({ "oracle": oracle, "candidate": candidate })
            }
            Observed::Line { oracle, candidate } => {
                serde_json::json!({ "oracle": oracle, "candidate": candidate })
            }
            Observed::FinalNewline { oracle, candidate } => serde_json::json!({
                "final_newline": { "oracle": oracle, "candidate": candidate },
            }),
            Observed::Files(files) => serde_json::json!({
                "files": files
                    .iter()
                    .map(|file| serde_json::json!({
                        "key": file.key,
                        "oracle": side_value(&file.oracle),
                        "candidate": side_value(&file.candidate),
                    }))
                    .collect::<Vec<_>>(),
            }),
        };
        value.to_string()
    }
}

fn side_value(side: &FileSideObserved) -> serde_json::Value {
    match side {
        FileSideObserved::Absent => serde_json::json!({ "present": false }),
        FileSideObserved::Present(line) => {
            serde_json::json!({ "present": true, "first_line": line })
        }
    }
}

/// What one side showed for `key`, given both snapshots' view of it. Two
/// present sides contribute their first differing line; a one-sided key
/// contributes its first line.
fn observe_key(
    key: &str,
    oracle: Option<&Vec<u8>>,
    candidate: Option<&Vec<u8>>,
) -> FileObservation {
    let side = |own: Option<&Vec<u8>>, other: Option<&Vec<u8>>| match (own, other) {
        (None, _) => FileSideObserved::Absent,
        (Some(own), Some(other)) => {
            FileSideObserved::Present(first_differing_line(own, other).and_then(|(_, own, _)| own))
        }
        (Some(own), None) => {
            FileSideObserved::Present(split_lines(own).first().map(|l| render_line(l)))
        }
    };
    let tail_only = matches!((oracle, candidate), (Some(oracle), Some(candidate))
        if first_differing_line(oracle, candidate).is_none());
    FileObservation {
        key: key.to_string(),
        oracle: side(oracle, candidate),
        candidate: side(candidate, oracle),
        tail_only,
    }
}

/// Lists the fields on which `oracle` and `candidate` differ, each with a
/// message and the values it showed. File keys are first passed through the
/// suite's aliases and file bytes through its normalizers, on both sides, so
/// the same logical file written to different paths or with nondeterministic
/// bytes compares equal. Stderr is compared only for cases that opt in.
pub fn compare(
    suite: &Suite,
    compare_stderr: bool,
    oracle: &Outcome,
    candidate: &Outcome,
) -> Result<Vec<Difference>, String> {
    let mut diffs = Vec::new();
    if oracle.exit != candidate.exit {
        diffs.push(Difference {
            field: Field::Exit,
            message: format!(
                "exit: oracle {:?}, candidate {:?}",
                oracle.exit, candidate.exit
            ),
            observed: Observed::Exit {
                oracle: oracle.exit,
                candidate: candidate.exit,
            },
        });
    }
    // Bytes decide, not lines: splitting drops a final newline, so two streams
    // that differ only there are equal as lines and must still fail.
    if oracle.stdout != candidate.stdout {
        let (message, observed) = stream_difference("stdout", &oracle.stdout, &candidate.stdout);
        diffs.push(Difference {
            field: Field::Stdout,
            message,
            observed,
        });
    }
    if compare_stderr && oracle.stderr != candidate.stderr {
        let (message, observed) = stream_difference("stderr", &oracle.stderr, &candidate.stderr);
        diffs.push(Difference {
            field: Field::Stderr,
            message,
            observed,
        });
    }
    let oracle_files = suite.canonical_files(oracle.files.clone())?;
    let candidate_files = suite.canonical_files(candidate.files.clone())?;
    if oracle_files != candidate_files {
        let keys: BTreeSet<&String> = oracle_files.keys().chain(candidate_files.keys()).collect();
        let differing: Vec<&String> = keys
            .into_iter()
            .filter(|k| oracle_files.get(*k) != candidate_files.get(*k))
            .collect();
        let observed: Vec<FileObservation> = differing
            .iter()
            .map(|key| observe_key(key, oracle_files.get(*key), candidate_files.get(*key)))
            .collect();
        let tail_only: Vec<&str> = observed
            .iter()
            .filter(|file| file.tail_only)
            .map(|file| file.key.as_str())
            .collect();
        let message = if tail_only.is_empty() {
            format!("files differ: {differing:?}")
        } else {
            format!(
                "files: unpinnable — key(s) {tail_only:?}: the lines all agree, and only \
                 the final newline differs"
            )
        };
        diffs.push(Difference {
            field: Field::Files,
            message,
            observed: Observed::Files(observed),
        });
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

/// Decides one run: every difference must be covered by a pin that states
/// *these* values. A pin whose field matches but whose values do not is
/// `pin drift` — distinct from `stale pin` (a pin that no longer matches a
/// difference at all) — and fails the run, because the pin's reason now
/// describes the opposite of what happens. A pin that met a difference on its
/// field is claimed either way: it is not stale, it is wrong. A difference no
/// pin can state (a byte difference with no differing line) fails as
/// unpinnable, whatever pins the field carries.
fn judge(diffs: Vec<Difference>, pins: &[(usize, &Divergence)], claimed: &mut [bool]) -> Status {
    let mut failures = Vec::new();
    let mut pinned = Vec::new();
    for diff in diffs {
        let covering: Vec<&(usize, &Divergence)> = pins
            .iter()
            .filter(|(_, pin)| pin.field == diff.field)
            .collect();
        if diff.observed.unpinnable() {
            // A real difference the pins on this field meet but cannot state:
            // claim them (they are not stale — the field still diverges) and
            // fail the run, because no pin can carry these values.
            for (index, _) in &covering {
                if let Some(claim) = claimed.get_mut(*index) {
                    *claim = true;
                }
            }
            failures.push(diff.message);
            continue;
        }
        if covering.is_empty() {
            failures.push(diff.message);
            continue;
        }
        for (index, _) in &covering {
            if let Some(claim) = claimed.get_mut(*index) {
                *claim = true;
            }
        }
        match covering
            .iter()
            .find(|(_, pin)| pin_matches(&pin.expect, &diff.observed))
        {
            Some(_) => pinned.push(diff.field),
            None => failures.push(format!(
                "pin drift: pin {}: {}: expected {}, observed {}",
                covering[0].0,
                diff.field.name(),
                covering
                    .iter()
                    .map(|(_, pin)| pin.expect.as_json())
                    .collect::<Vec<_>>()
                    .join(" or "),
                diff.observed.as_expect()
            )),
        }
    }
    if !failures.is_empty() {
        Status::Fail(failures)
    } else if pinned.is_empty() {
        Status::Pass
    } else {
        Status::Pinned(pinned)
    }
}

/// True when `observed` is the difference `expect` states.
fn pin_matches(expect: &Expect, observed: &Observed) -> bool {
    match observed {
        Observed::Exit { oracle, candidate } => {
            exit_code(&expect.oracle) == *oracle && exit_code(&expect.candidate) == *candidate
        }
        Observed::Line { oracle, candidate } => {
            line_matches(&expect.oracle, oracle) && line_matches(&expect.candidate, candidate)
        }
        // No pin states a line-less difference; `judge` fails these as
        // unpinnable before asking.
        Observed::FinalNewline { .. } => false,
        Observed::Files(found) => expect.files.as_deref().is_some_and(|pins| {
            pins.len() == found.len()
                && pins
                    .iter()
                    .all(|pin| found.iter().any(|file| file_pin_matches(pin, file)))
        }),
    }
}

fn file_pin_matches(pin: &FileExpect, found: &FileObservation) -> bool {
    pin.key == found.key
        && file_side_matches(&pin.oracle, &found.oracle)
        && file_side_matches(&pin.candidate, &found.candidate)
}

fn file_side_matches(pin: &FileSideExpect, found: &FileSideObserved) -> bool {
    match (pin.present, found) {
        (false, FileSideObserved::Absent) => true,
        (true, FileSideObserved::Present(line)) => {
            line_prefix_matches(pin.first_line.as_deref(), line.as_deref())
        }
        _ => false,
    }
}

/// A pin's expected side is a prefix; `null` means the side has no such line
/// and `""` means the line is empty (never "matches anything").
fn line_matches(expected: &serde_json::Value, observed: &Option<String>) -> bool {
    match expected.as_str() {
        Some(prefix) => line_prefix_matches(Some(prefix), observed.as_deref()),
        None => expected.is_null() && observed.is_none(),
    }
}

fn line_prefix_matches(expected: Option<&str>, observed: Option<&str>) -> bool {
    match (expected, observed) {
        (None, None) => true,
        (Some(""), Some(line)) => line.is_empty(),
        (Some(prefix), Some(line)) => line.starts_with(prefix),
        _ => false,
    }
}

fn run_command(
    suite: &Suite,
    cmd: &CommandSpec,
    mode: &Mode,
    work: &Path,
    report: &mut Report,
    claimed: &mut [bool],
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
                let pins: Vec<(usize, &Divergence)> = match mode {
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
                        .collect(),
                };
                let with = |base: &[String]| base.iter().chain(&args).cloned().collect::<Vec<_>>();
                let outcome =
                    run_side(suite, case, &with(&oracle_argv), work, limit).and_then(|o| {
                        run_side(suite, case, &with(&candidate_argv), work, limit).map(|c| (o, c))
                    });
                let status = match outcome {
                    Ok((o, c)) => match compare(suite, case.spec.stderr, &o, &c) {
                        Ok(diffs) => judge(diffs, &pins, claimed),
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
    let mut claimed = vec![false; suite.divergences.len()];
    for cmd in &suite.commands {
        run_command(suite, cmd, mode, work, &mut report, &mut claimed);
    }
    if matches!(mode, Mode::Candidate(_)) {
        for (pin, claimed) in suite.divergences.iter().zip(&claimed) {
            if !claimed && pin.os.is_none() {
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
