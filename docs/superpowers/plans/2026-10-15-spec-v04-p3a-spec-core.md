# Spec v0.4 Phase 3a — spec-core (F16) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Turn the `vibe-spec` stub into the spec tool's core: one root-discovery function, byte-identical Rust ports of `list-specs.sh`, `lessons-for.sh`, `scan-merges.sh` and `validate.sh`, the Spec JSON v1 queries (`root`, `lessons-for --json`, `plan`, `feature`), and parity in candidate mode — row F16 (#33).

**Architecture:** Parity first. Task 1 gives the harness the three duties F16 carries (run timeout, empty-directory comparison, per-argument-set pins) and lets unported commands stay self-check only. Each port then lands with its corpus cases, its pins and a "live" comparison on this repo's own `.spec/`, so every script is proven against its frozen oracle before F17 changes any behaviour. Consumers (skills, CI, flow templates) keep calling the bash scripts until the binary ships (F21s/F22s).

**Tech Stack:** Rust 1.88 MSRV / edition 2024 (`clap`, `serde`, `serde_json` only), bash oracles under `oracles/bash/spec/`, Node ≥18 for contract checks.

**Spec:** `docs/superpowers/specs/2026-10-13-spec-v04-design.md` (§3 F16 row, §7 Rust port, §9 Testing). Program index: `docs/superpowers/plans/2026-10-13-spec-v04.md`. Row detail: `docs/roadmap/issues/16-F16-spec-core.md`. Contracts: `.spec/tech-contracts.md` ("Spec JSON v1"), `contracts/spec-json.schema.json`. Discovery: `.spec/tech-spec.md` ("Root and discovery"). Carried duties: `.spec/tech-rust.md` ("Oracles and parity").

## Global Constraints

- Commits: Conventional Commits, imperative, lowercase, ≤50-char subject, no body, no footer.
- MSRV 1.88, edition 2024. Dependency allowlist: `clap` (derive), `serde`, `serde_json`. No `toml`, `regex`, `tokio`, `tempfile`. Every pattern match is hand-written.
- `vibe-core` holds no spec paths or spec vocabulary. Discovery, ports and queries live in `crates/vibe-spec`.
- No `unwrap`/`expect` outside tests (`clippy::unwrap_used`, `clippy::expect_used` deny). No panics on input: index with `get`, slice only after a length check.
- Exit codes: 0 ok · 1 findings/refused · 2 usage · 3 no spec tree. Exit 3 applies to `root`, `--json`, `plan`, `feature` only. The four ported commands keep the oracle's exit codes in text mode.
- Freeze an oracle before porting the script it proves. An oracle never changes. Oracle bugs become `expected-divergence` pins, fixed in Rust only.
- Parity compares exit code, stdout bytes and every file and directory left in the sandbox. A candidate output differs from its oracle only where a pin in `tests/parity/spec.json` says so.
- Every guard asserts its own population: corpus floors rise with every case added, and live parity names each command it compared.
- Work on bytes (`&[u8]`/`Vec<u8>`) for file content and output. The oracles run under `LC_ALL=C`: byte slices, byte order, byte padding.
- Run every affected suite and show its output before claiming a task done. Any task that adds or edits a tracked `*.sh` runs the CI shellcheck command.
- Never run timing suites (`node flow/engine/tests/run.mjs`, `bash tests/run.sh`) concurrently with other suites or `cargo` builds.
- Never Edit/Write a file under `tests/parity/corpus/`. Change `crates/vibe-parity/src/corpus_spec.rs` and regenerate with `cargo run --locked -q -p vibe-parity -- gen-spec-corpus oracles/bash/spec/reference/templates tests/parity/corpus/spec`.
- Tasks run with the flow cursor at `idle`. Every `.spec/` edit goes through `strategy.spec`.
- Outward-facing actions (push, PR, issue edits) need an explicit owner yes at that moment.
- Scratch files go in `.superpowers/p3a/` (gitignored), never `/tmp`.

## Review Focus

1. **A run that hangs, leaves an empty directory, or diverges on only one argument set.** Expected: the hang fails at the suite timeout, the stray directory fails on `files`, and a pin covers only the argument set it names. Tests: Task 1, `a_hanging_run_fails_at_the_timeout`, `an_extra_empty_directory_fails_on_files`, `a_pin_with_args_covers_only_that_run`.
2. **A `.vibe/spec.json` that holds only other keys** (F17's `strictness`, F19's `tooth`). Expected: discovery falls through to `docs/spec/` and `.spec/`, it does not fail. Test: Task 2, `a_config_without_root_falls_through`.
3. **A file whose last line has no newline.** The oracles disagree with each other: `lessons-for.sh` and `grep` keep that line, while `while read` loops (`scan-merges.sh`, validate's SF16 and `children:` loops) drop it. Expected: each port drops or keeps it exactly as its oracle does. Tests: corpus cases `lessons/no-newline` (Task 4), `features/no-trailing-newline` (Task 5), `malformed/rich-root` (Task 7).
4. **Link targets that are absolute or contain `(`.** The oracle builds `$SPEC_DIR/$target` as text, so `/x.md` becomes `.spec//x.md`, and `sed` takes the text after the last `(`. Expected: the port joins paths textually, never with `Path::join`, which would replace the base path. Tests: Task 6, `join_raw_never_replaces_the_base`, and the `[a](b(c).md)` link in `malformed/rich-root`.
5. **Multibyte text at a byte boundary, and quotes in JSON output.** Expected: the 60-byte preview cut and the table padding count bytes. Legacy `--format json` output is valid JSON. Where the oracle's output is invalid JSON, the port escapes it and a pin covers that one argument set. Tests: corpus cases `features/long-preview` and `features/quoted-preview` (Task 5), `lessons/quotes` (Task 4).

## Rulings (owner review)

These decide what the spec leaves open. Each lists its cost if wrong.

- **P3a-R1 — Scope.** The owner chose to split Phase 3: this plan covers F16 only. F17 (`…-p3b-spec-validators.md`) and F18 (`…-p3c-spec-delta.md`) are each written after their predecessor lands. Cost if wrong: none to the code; the index row changes (Task 9).
- **P3a-R2 — Reference platform.** CI's ubuntu job (GNU grep and sed, mawk, bash 5) is the reference. Local macOS runs must also pass. An oracle difference that comes from BSD tools gets a pin scoped with `"os": "macos"`; os-scoped pins are never stale-checked. Known case: SF15 prints `wc -l`'s count, which BSD pads with spaces. Cost if wrong: a BSD-only difference could hide behind an os pin.
- **P3a-R3 — Legacy argument surface.** `list-specs`, `lessons-for`, `scan-merges` and `validate` parse their own arguments, not with clap, and keep the oracle's exit codes: `Unknown option` and a missing `--format` value exit 1. Changes:
  - New flags: the global `--root <dir>` (placed before the subcommand), and for `lessons-for`, `--json` and `--tag <t>`.
  - `list-specs` ignores arguments, as its oracle does. `validate` rejects any argument with exit 2 until F17 adds its flags.
  - Cost if wrong: F17 replaces the hand parser for `validate`.
- **P3a-R4 — Discovery details.** Steps 3–5 run at each directory while walking up to the one holding `.git`, nearest hit first:
  - A `root` in `.vibe/spec.json` resolves against the directory holding `.vibe/`.
  - A config without `root` is not a hit.
  - A missing directory named by `--root`, `$VIBE_SPEC_ROOT` or a config `root` exits 3 and never falls through.
  - A malformed config exits 2.
  - `$SPEC_DIR` is ignored, with one stderr line pointing to `VIBE_SPEC_ROOT`.
  - The text-mode legacy commands treat "nothing found" exactly as the oracle treats a missing `.spec/`.
  - Cost if wrong: a user's `SPEC_DIR` stops working until they rename it.
- **P3a-R5 — Deliberate divergences.** Every one is pinned for its exact argument set:
  - Legacy `--format json` output is escaped (`lessons-for`, `scan-merges`).
  - The no-root `validate` message names `vibe-spec init` instead of the absolute path of the oracle's `setup.sh`.
  - Tag matching is a case-insensitive literal substring, not a BRE: the oracle's `grep -qi "$tag"` makes `b.sh` match `bash`.
  - Everything else is byte-identical, including ANSI colours and `echo -e` escapes in `validate`.
  - Cost if wrong: a consumer that relied on the raw bytes sees escapes.
- **P3a-R6 — Query shapes.**
  - `lessons-for --json <tags>` → `{v, tags, lessons[{title, rule, tags[], pattern?, date?, pinned?}]}`. A lesson without a title or a `**Rule:**` is left out; `pinned` means the lesson has a `**Pinned-by:**` line.
  - `plan --feature <f>` reads the newest `docs/superpowers/plans/YYYY-MM-DD-<f>.md` (one row per `### Task N:` heading; status `done` when all its checkboxes are ticked, `in-progress` when only some are, `todo` when none are). It falls back to the `| Unit | Status |` table in `<root>/features/<f>/plan.md`.
  - `feature <f>` → layer `superpowers` when a design or plan artifact exists, else `fallback` with the existing `product|tech|design|plan|research` docs.
  - Cost if wrong: these shapes are new; required fields are already fixed by F2.
- **P3a-R7 — Consumers stay on bash.** `spec/SKILL.md`, CI's validate step, `AGENTS.md`, `vibe.json` and the flow templates keep calling `spec/scripts/*.sh`. The binary is not distributable until F22, and the F25 pressure baseline (12/12) depends on the skill text staying as it is. F16 proves the two match by comparing them live on this repo. The switch, and the `$VIBE_SPEC_BIN` dev shim §7 names for it, move to F21s/F22s. Cost if wrong: §7's "same commit" rule is deferred, and both implementations coexist until F23s.
- **P3a-R8 — Parity entry point.** `tests/parity/run.sh` builds the binaries and runs the self-check, candidate and live comparisons. It replaces the command in CI's `parity` job. It does not join `tests/run.sh`, which runs in three CI jobs, including macOS bash 3.2; that would mean three Rust builds and BSD-tool drift. Cost if wrong: a developer who runs only `tests/run.sh` misses parity; the gate lists both.
- **P3a-R9 — Unported commands.** `check-drift`, `promote` and `setup`/`init` get `"candidate": null`: candidate mode skips them, and the self-check still runs them. F18 and F20 set their candidates. Cost if wrong: none.
- **P3a-R10 — Baseline.** `bash tests/run.sh` is already red on `main`: the engine suite's `doctor` perf budget fails on this machine (194 ms > 150 ms; `validate-state.sh` alone takes 81 ms). Judge every task by the delta: the same single failure, nothing new. Do not fix it here. Cost if wrong: none; the failure predates this branch.
- **P3a-R11 — Gate honesty.** Row F16 stays `IN PROGRESS — local gate green; CI pending push` until the owner approves a push and CI is green.
- **P3a-R12 — `--json` coverage.** §7 asks for `--json` on every command. F16 ships it where the F16 issue asks: `root`, `lessons-for`, `plan` and `feature`. `validate`'s JSON arrives with F17's `--format json|hook`. `list-specs` and `scan-merges` get `--json` when a consumer needs it (F19 hooks or F21s). Cost if wrong: two commands lack `--json` for one or two phases.

## F16 acceptance → tasks

| Acceptance (#33) | Where |
|---|---|
| oracle diff clean + population floor (≥1 spec file scanned) | Tasks 3–7 parity; Task 7 `malformed/empty-root` (zero root files → exit 1) |
| invocation from a subdirectory resolves the same root | Task 2 root tests; corpus `no-root/subdir`, `seeded/from-subdir` |
| exit 0/3 goldens (tree present / absent) | Task 2 `root` tests, Task 8 query tests, each validated against `contracts/spec-json.schema.json` |
| `bash tests/run.sh` green, `cargo test --workspace` green | Task 9 gate (R10 baseline) |
| oracles frozen before porting | Done in F3 (`oracles/bash/spec/scripts/`, `MANIFEST.sha256`) |

## File Structure

```text
crates/vibe-parity/src/suite.rs        # timeout_ms, optional candidate, pin args/os   (Task 1)
crates/vibe-parity/src/harness.rs      # timeout, per-arg pins, skipped commands       (Task 1)
crates/vibe-parity/src/sandbox.rs      # snapshot records directories                  (Task 1)
crates/vibe-parity/src/main.rs         # SKIP lines, skipped count                     (Task 1)
crates/vibe-parity/src/bin/parity-fixture.rs  # --mutate=dir|hang                     (Task 1)
crates/vibe-parity/src/corpus_spec.rs  # new cases                                (Tasks 3–7)
crates/vibe-spec/Cargo.toml            # + serde_json                                  (Task 2)
crates/vibe-spec/src/main.rs           # CLI, `cat!`, dispatch                    (Tasks 2–8)
crates/vibe-spec/src/root.rs           # discovery + `root`                            (Task 2)
crates/vibe-spec/src/text.rs           # byte helpers, echo -e, JSON strings      (Tasks 2, 4)
crates/vibe-spec/src/list_specs.rs     #                                               (Task 3)
crates/vibe-spec/src/lessons.rs        # legacy formats + Spec JSON                    (Task 4)
crates/vibe-spec/src/scan_merges.rs    #                                               (Task 5)
crates/vibe-spec/src/pattern.rs        # hand-written grep -o / awk matchers      (Tasks 6, 7)
crates/vibe-spec/src/validate.rs       # driver + structural checks                (Tasks 6, 7)
crates/vibe-spec/src/content.rs        # SF3, SF8–SF12, SF14, I1                       (Task 7)
crates/vibe-spec/src/query.rs          # plan, feature                                 (Task 8)
crates/vibe-spec/tests/common/mod.rs   # run candidate/oracle, schema check            (Task 2)
crates/vibe-spec/tests/{root,list_specs,lessons,scan_merges,validate,query}.rs
tests/parity/spec.json                 # candidates, pins, floors                 (Tasks 1, 3–7)
tests/parity/run.sh                    # self-check + candidate + live                 (Task 3)
.github/workflows/ci.yml               # parity job runs tests/parity/run.sh           (Task 3)
docs/superpowers/plans/2026-10-13-spec-v04.md  # index: phase 3 → 3a/3b/3c             (Task 9)
.spec/{tech-spec,tech-rust,tech-contracts,plan,lessons}.md                             (Task 9)
```

---

### Task 1: Parity harness — timeout, directories, per-run pins, unported commands

**Files:**
- Modify: `crates/vibe-parity/src/suite.rs` (structs `Suite`, `CommandSpec`, `Divergence`)
- Modify: `crates/vibe-parity/src/harness.rs` (`execute`, `run_side`, `run_command`, `run_suite`, `Report`)
- Modify: `crates/vibe-parity/src/sandbox.rs` (`snapshot`/`walk`)
- Modify: `crates/vibe-parity/src/main.rs` (`run`)
- Modify: `crates/vibe-parity/src/bin/parity-fixture.rs`
- Modify: `tests/parity/spec.json`
- Test: `crates/vibe-parity/tests/harness.rs`

**Interfaces:**
- Consumes: the Phase 2 harness (`Suite::load`, `run_suite`, `Mode`, `Status`, `Report`, `execute`).
- Produces:
  - `Suite.timeout_ms: u64` (default 30000).
  - `CommandSpec.candidate: Option<Vec<String>>` (`null` = not ported).
  - `Divergence.args: Option<Vec<String>>` and `Divergence.os: Option<String>`.
  - `Divergence::active(&self) -> bool`.
  - `Report.skipped: Vec<String>`.
  - `pub fn execute_with_timeout(argv: &[String], cwd: &Path, home: &Path, limit: Duration) -> Result<(Option<i32>, Vec<u8>), String>`.
  - `snapshot` keys for directories end in `/` and map to empty bytes.
  - Tasks 3–7 write pins as `{command, case, args?, field, os?, reason}`.

- [ ] **Step 1: Write the failing tests**

Add to the imports of `crates/vibe-parity/tests/harness.rs`:

```rust
use std::time::{Duration, Instant};
```

Append to `crates/vibe-parity/tests/harness.rs`:

```rust
#[test]
fn a_pin_with_args_covers_only_that_run() {
    let mut suite = base();
    suite["divergences"] = json!([
        { "command": "echo", "case": "basic/one", "args": ["a"], "field": "stdout", "reason": "test" }
    ]);
    suite["commands"][0]["candidate"] = json!(["{candidate}", "--mutate=stdout"]);
    let report = run("pin-args", suite, &candidate());
    assert_eq!(report.pinned(), 1, "{report:#?}");
    assert_eq!(report.failed(), 2, "{report:#?}");
    assert!(report.errors.is_empty(), "{:?}", report.errors);
}

#[test]
fn a_pin_with_args_that_match_no_run_is_stale() {
    let mut suite = base();
    suite["divergences"] = json!([
        { "command": "echo", "case": "basic/one", "args": ["zzz"], "field": "stdout", "reason": "test" }
    ]);
    let report = run("pin-args-stale", suite, &candidate());
    assert!(
        report
            .errors
            .iter()
            .any(|e| e.starts_with("stale pin: echo basic/one [\"zzz\"]")),
        "{:?}",
        report.errors
    );
}

#[test]
fn an_os_pin_is_inactive_elsewhere_and_never_stale() {
    let mut suite = base();
    suite["divergences"] = json!([
        { "command": "echo", "case": "basic/one", "field": "stdout", "os": "plan9", "reason": "test" }
    ]);
    suite["commands"][0]["candidate"] = json!(["{candidate}", "--mutate=stdout"]);
    let report = run("pin-os-other", suite.clone(), &candidate());
    assert_eq!(report.failed(), 3, "{report:#?}");
    assert!(report.errors.is_empty(), "{:?}", report.errors);

    suite["divergences"][0]["os"] = json!(std::env::consts::OS);
    let report = run("pin-os-here", suite, &candidate());
    assert_eq!(report.pinned(), 2, "{report:#?}");
    assert_eq!(report.failed(), 1, "{report:#?}");

    let mut suite = base();
    suite["divergences"] = json!([
        { "command": "echo", "case": "basic/one", "field": "stdout", "os": std::env::consts::OS, "reason": "test" }
    ]);
    let report = run("pin-os-unhit", suite, &candidate());
    assert!(report.ok(), "an unhit os pin is not stale: {:?}", report.errors);
}

#[test]
fn a_hanging_run_fails_at_the_timeout() {
    let mut suite = base();
    suite["timeout_ms"] = json!(300);
    suite["commands"][0]["candidate"] = json!(["{candidate}", "--mutate=hang"]);
    let start = Instant::now();
    let report = run("hang", suite, &candidate());
    assert!(start.elapsed() < Duration::from_secs(20), "took {:?}", start.elapsed());
    assert_eq!(report.failed(), 3, "{report:#?}");
    for r in &report.results {
        let Status::Fail(msgs) = &r.status else {
            panic!("{r:?}")
        };
        assert!(
            msgs.iter().any(|m| m.contains("timed out after 300 ms")),
            "{msgs:?}"
        );
    }
}

#[test]
fn an_extra_empty_directory_fails_on_files() {
    let mut suite = base();
    suite["commands"][0]["candidate"] = json!(["{candidate}", "--mutate=dir"]);
    let report = run("emptydir", suite, &candidate());
    assert_eq!(report.failed(), 3, "{report:#?}");
    for r in &report.results {
        let Status::Fail(msgs) = &r.status else {
            panic!("{r:?}")
        };
        assert!(msgs.iter().any(|m| m.contains("\"made/\"")), "{msgs:?}");
    }
}

#[test]
fn an_unported_command_is_skipped_in_candidate_mode_only() {
    let mut suite = base();
    suite["commands"][0]["candidate"] = Value::Null;
    let report = run("unported", suite.clone(), &candidate());
    assert_eq!(report.skipped, vec!["echo".to_string()]);
    assert!(report.results.is_empty());
    assert!(!report.ok(), "a candidate run that compared nothing must fail");

    let report = run("unported-self", suite, &Mode::SelfCheck);
    assert!(report.ok(), "{report:#?}");
    assert_eq!(report.results.len(), 3);
}

#[test]
fn the_cli_prints_skipped_commands() {
    let dir = work("cli-skip");
    let mut suite = base();
    suite["commands"][0]["candidate"] = Value::Null;
    let path = dir.join("suite.json");
    fs::write(&path, serde_json::to_vec(&suite).unwrap()).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vibe-parity"))
        .arg("run")
        .arg(&path)
        .arg("--candidate")
        .arg(env!("CARGO_BIN_EXE_parity-fixture"))
        .output()
        .unwrap();
    let text = String::from_utf8_lossy(&out.stdout);
    assert_eq!(out.status.code(), Some(1), "{text}");
    assert!(text.contains("SKIP  echo (not ported; self-check only)"), "{text}");
    assert!(text.contains(", 1 skipped ("), "{text}");
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test --locked -p vibe-parity --test harness 2>&1 | tail -20`
Expected: compile error (`no field skipped on type Report`), or the new tests FAIL.

- [ ] **Step 3: Extend the suite types**

In `crates/vibe-parity/src/suite.rs`, add to `Suite` after `divergences`:

```rust
    /// Wall-clock limit per run, in milliseconds. A run over it fails.
    #[serde(default = "default_timeout_ms")]
    pub timeout_ms: u64,
```

and below the struct:

```rust
fn default_timeout_ms() -> u64 {
    30_000
}
```

Replace `CommandSpec.candidate` with:

```rust
    /// Candidate argv. `{candidate}` expands to the candidate binary.
    /// `null` (or absent) means not ported yet: candidate mode skips the
    /// command; the self-check still runs it.
    #[serde(default)]
    pub candidate: Option<Vec<String>>,
```

Replace `Divergence` with:

```rust
/// A pinned `expected-divergence`: the candidate differs from the oracle on
/// purpose (an oracle bug fixed only in the port, or a deliberate change).
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Divergence {
    /// Command name.
    pub command: String,
    /// Case id, `<class>/<case>`.
    pub case: String,
    /// Only the run with exactly these extra arguments. Absent: every run of the case.
    #[serde(default)]
    pub args: Option<Vec<String>>,
    /// The field that differs.
    pub field: Field,
    /// Only on this `std::env::consts::OS`. Elsewhere the pin is inactive.
    /// An os-scoped pin is never reported stale: local tools vary.
    #[serde(default)]
    pub os: Option<String>,
    /// Why the difference is correct.
    pub reason: String,
}

impl Divergence {
    /// False when the pin is scoped to another operating system.
    pub fn active(&self) -> bool {
        self.os
            .as_deref()
            .is_none_or(|os| os == std::env::consts::OS)
    }
}
```

- [ ] **Step 4: Snapshot directories**

In `crates/vibe-parity/src/sandbox.rs`, replace the doc comment of `snapshot` with:

```rust
/// Maps every file and directory under `root`, except `.oracle/` and `.git/`
/// at the top, to its bytes. Keys are relative paths joined with `/`; a
/// directory's key ends in `/` and maps to no bytes, so an empty directory
/// left behind is a difference too.
```

and in `walk`, replace the branch `if kind.is_dir() { walk(&path, &rel, files)?; }` with:

```rust
        if kind.is_dir() {
            files.insert(format!("{rel}/"), Vec::new());
            walk(&path, &rel, files)?;
        }
```

- [ ] **Step 5: Timeout, per-run pins, skipped commands**

In `crates/vibe-parity/src/harness.rs`:

1. Change the imports to:

```rust
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};
```

2. Add to `Report`, after `errors`:

```rust
    /// Commands candidate mode skipped because they are not ported yet.
    pub skipped: Vec<String>,
```

3. Replace `execute` with:

```rust
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
    let (program, args) = argv.split_first().ok_or("empty argv")?;
    let mut child = Command::new(program)
        .args(args)
        .current_dir(cwd)
        .env_clear()
        .env("PATH", std::env::var_os("PATH").unwrap_or_default())
        .env("HOME", home)
        .env("LC_ALL", "C")
        .env("TZ", "UTC")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("{program}: {e}"))?;
    let mut pipe = child.stdout.take().ok_or("no stdout pipe")?;
    let reader = std::thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = pipe.read_to_end(&mut buf);
        buf
    });
    let start = Instant::now();
    let status = loop {
        match child.try_wait().map_err(|e| format!("{program}: {e}"))? {
            Some(status) => break status,
            None if start.elapsed() >= limit => {
                let _ = child.kill();
                let _ = child.wait();
                // A grandchild may still hold the pipe; the reader thread is
                // left to finish on its own.
                return Err(format!(
                    "{program}: timed out after {} ms",
                    limit.as_millis()
                ));
            }
            None => std::thread::sleep(Duration::from_millis(5)),
        }
    };
    let stdout = reader
        .join()
        .map_err(|_| format!("{program}: stdout reader panicked"))?;
    Ok((status.code(), stdout))
}
```

4. In `run_side`, add a parameter `limit: Duration` and call `execute_with_timeout(argv, &cwd, &home, limit)?` instead of `execute(argv, &cwd, &home)?`.

5. Replace `run_command` with:

```rust
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
                let outcome = run_side(suite, case, &with(&oracle_argv), work, limit).and_then(|o| {
                    run_side(suite, case, &with(&candidate_argv), work, limit).map(|c| (o, c))
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
```

6. In `run_suite`, replace the stale-pin loop with:

```rust
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
```

- [ ] **Step 6: Print skipped commands**

In `crates/vibe-parity/src/main.rs` `run`, after the loop over `report.errors`, add:

```rust
    for name in &report.skipped {
        println!("SKIP  {name} (not ported; self-check only)");
    }
```

and replace the summary `println!` with:

```rust
    println!(
        "parity[{}]: {} passed, {} pinned, {} failed, {} error(s), {} runs, {} skipped ({label})",
        suite.name,
        report.passed(),
        report.pinned(),
        report.failed(),
        report.errors.len(),
        report.results.len(),
        report.skipped.len()
    );
```

- [ ] **Step 7: Fixture mutants**

Replace `crates/vibe-parity/src/bin/parity-fixture.rs` `main` with:

```rust
fn main() -> ExitCode {
    let mut mutate = String::new();
    let mut args = Vec::new();
    for arg in std::env::args().skip(1) {
        match arg.strip_prefix("--mutate=") {
            Some(m) => mutate = m.to_string(),
            None => args.push(arg),
        }
    }
    if mutate == "hang" {
        std::thread::sleep(std::time::Duration::from_secs(60));
    }
    if mutate == "dir" && std::fs::create_dir("made").is_err() {
        return ExitCode::from(1);
    }
    let mark = if mutate == "stdout" { "!" } else { "" };
    println!("args:{}{mark}", args.join(" "));
    let body = if mutate == "file" { "y\n" } else { "x\n" };
    if std::fs::write("out.txt", body).is_err() {
        return ExitCode::from(1);
    }
    ExitCode::from(if mutate == "exit" { 3 } else { 0 })
}
```

and its doc line to: `` `--mutate=<exit|stdout|file|dir|hang>` makes it diverge on purpose so tests can prove the harness turns red.``

- [ ] **Step 8: Mark every spec command unported**

In `tests/parity/spec.json`, set `"candidate": null` on all seven commands. Tasks 3–5 and 7 set it back for the four ported ones.

- [ ] **Step 9: Run the tests to verify they pass**

```bash
cargo test --locked -p vibe-parity 2>&1 | grep -E 'test result|FAILED|panicked'
cargo clippy --workspace --all-targets --locked -- -D warnings 2>&1 | tail -1
cargo fmt --all --check && echo fmt-ok
cargo run --locked -q -p vibe-parity -- run tests/parity/spec.json --self | tail -1
```

Expected:
- every `test result: ok`;
- clippy `Finished`, and `fmt-ok`;
- self-check summary `parity[spec]: 61 passed, 0 pinned, 0 failed, 0 error(s), 61 runs, 0 skipped (self-check)`.

- [ ] **Step 10: Commit**

```bash
git add crates/vibe-parity tests/parity/spec.json
git commit -m "feat(parity): add timeout, dir and per-run pins"
```

### Task 2: `vibe-spec` CLI, root discovery and byte helpers

**Files:**
- Modify: `crates/vibe-spec/Cargo.toml`
- Modify: `crates/vibe-spec/src/main.rs`
- Create: `crates/vibe-spec/src/root.rs`
- Create: `crates/vibe-spec/src/text.rs`
- Create: `crates/vibe-spec/tests/common/mod.rs`
- Create: `crates/vibe-spec/tests/root.rs`
- Test: `crates/vibe-spec/tests/cli.rs` (unchanged; must stay green)

**Interfaces:**
- Consumes: `vibe_core::exit::{OK, FINDINGS, USAGE, NO_ROOT}`.
- Produces (crate-internal, used by Tasks 3–8):
  - `root::discover(cwd: &Path, flag: Option<&Path>, env: Option<&OsStr>) -> Result<Root, Miss>`
  - `root::Root { dir: PathBuf, project: PathBuf, source: Source, depth: usize }`, `Root::display(&self) -> String`
  - `root::Miss::{NotFound, Missing{..}, BadConfig{..}}`, `Miss::code(&self) -> u8`, `Miss::message(&self) -> String`
  - `root::command(found: &Result<Root, Miss>, json: bool) -> (u8, Vec<u8>)`
  - macro `cat!(parts…) -> Vec<u8>` (defined in `main.rs`, visible to every module)
  - `text::{lines, read_lines, first_line, has_line_starting, contains, contains_ci, remove_first, strip_prefix_or, is_space, trim_space, trim_start_space, echo_e, json_str, lossy, join_raw, name_of, md_files, sub_dirs}`
  - Test helpers `common::{repo, project, write, spec, oracle, assert_parity, assert_spec_json, Out}`
  - Every command function returns `(exit_code, stdout_bytes)`; `main` writes stdout once.

- [ ] **Step 1: Write the failing tests**

Create `crates/vibe-spec/tests/common/mod.rs`:

```rust
//! Shared helpers for the vibe-spec integration tests.
#![allow(dead_code, clippy::unwrap_used, clippy::expect_used)]
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// The repository root.
pub fn repo() -> PathBuf {
    fs::canonicalize(Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")).unwrap()
}

/// A fresh, canonical scratch project holding an empty `.git/` (bounds discovery).
pub fn project(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("vibe-spec-test-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join(".git")).unwrap();
    fs::canonicalize(dir).unwrap()
}

/// Writes `text` to `root/rel`, creating parents.
pub fn write(root: &Path, rel: &str, text: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

/// What a process produced.
#[derive(Debug)]
pub struct Out {
    pub code: i32,
    pub stdout: Vec<u8>,
    pub stderr: String,
}

impl Out {
    pub fn text(&self) -> String {
        String::from_utf8_lossy(&self.stdout).into_owned()
    }
}

fn run(mut cmd: Command, cwd: &Path, env: &[(&str, &str)]) -> Out {
    cmd.current_dir(cwd)
        .env_clear()
        .env("PATH", std::env::var_os("PATH").unwrap_or_default())
        .env("HOME", cwd)
        .env("LC_ALL", "C")
        .env("TZ", "UTC");
    for (k, v) in env {
        cmd.env(k, v);
    }
    let o = cmd.output().unwrap();
    Out {
        code: o.status.code().unwrap_or(-1),
        stdout: o.stdout,
        stderr: String::from_utf8_lossy(&o.stderr).into_owned(),
    }
}

/// Runs the candidate in `cwd` with a clean environment plus `env`.
pub fn spec(cwd: &Path, args: &[&str], env: &[(&str, &str)]) -> Out {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_vibe-spec"));
    cmd.args(args);
    run(cmd, cwd, env)
}

/// Runs the frozen oracle `oracles/bash/spec/scripts/<script>` the same way.
pub fn oracle(cwd: &Path, script: &str, args: &[&str]) -> Out {
    let mut cmd = Command::new("bash");
    cmd.arg(repo().join("oracles/bash/spec/scripts").join(script))
        .args(args);
    run(cmd, cwd, &[])
}

/// Asserts the candidate's `command args` and the oracle's `script args`
/// agree on exit code and stdout bytes when run in `cwd`.
pub fn assert_parity(cwd: &Path, command: &str, script: &str, args: &[&str]) {
    let want = oracle(cwd, script, args);
    let mut full = vec![command];
    full.extend_from_slice(args);
    let got = spec(cwd, &full, &[]);
    assert_eq!(
        (got.code, got.text()),
        (want.code, want.text()),
        "`vibe-spec {command} {args:?}` vs `{script}` in {}\ncandidate stderr: {}",
        cwd.display(),
        got.stderr
    );
}

/// Validates `json` against `$defs/<def>` of `contracts/spec-json.schema.json`
/// with the contracts' own validator (`contracts/lib/schema.mjs`).
#[cfg(unix)]
pub fn assert_spec_json(def: &str, json: &str) {
    let lib = repo().join("contracts/lib/schema.mjs");
    let schema = repo().join("contracts/spec-json.schema.json");
    let script = format!(
        "import {{ validate }} from 'file://{}';\n\
         import {{ readFileSync }} from 'node:fs';\n\
         const root = JSON.parse(readFileSync({:?}, 'utf8'));\n\
         const errs = validate(root, {{ $ref: '#/$defs/{def}' }}, JSON.parse(process.argv[1]));\n\
         if (errs.length) {{ console.error(errs.join('\\n')); process.exit(1); }}",
        lib.display(),
        schema.display().to_string()
    );
    let out = Command::new("node")
        .args(["--input-type=module", "-e", &script, json])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{def} does not validate: {}\n{json}",
        String::from_utf8_lossy(&out.stderr)
    );
}
```

Create `crates/vibe-spec/tests/root.rs`:

```rust
#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Root discovery: one order, from any directory (tech-spec.md, Root and discovery).
mod common;

use std::fs;

use common::{project, spec, write};
use serde_json::Value;

fn json(out: &common::Out) -> Value {
    serde_json::from_slice(&out.stdout).unwrap()
}

#[test]
fn dot_spec_from_a_subdirectory_resolves_like_the_root() {
    let p = project("subdir");
    fs::create_dir_all(p.join(".spec")).unwrap();
    fs::create_dir_all(p.join("src/deep")).unwrap();
    let top = spec(&p, &["root"], &[]);
    let deep = spec(&p.join("src/deep"), &["root"], &[]);
    assert_eq!(top.code, 0, "{}", top.stderr);
    assert_eq!(top.text(), format!("{}\n", p.join(".spec").display()));
    assert_eq!(deep.text(), top.text());
    let v = json(&spec(&p.join("src/deep"), &["root", "--json"], &[]));
    assert_eq!(v["source"], "dot-spec");
    assert_eq!(v["depth"], 2);
    assert_eq!(v["root"], p.join(".spec").to_string_lossy().as_ref());
}

#[test]
fn docs_spec_beats_dot_spec_at_the_same_level() {
    let p = project("docs-first");
    fs::create_dir_all(p.join(".spec")).unwrap();
    fs::create_dir_all(p.join("docs/spec")).unwrap();
    let v = json(&spec(&p, &["root", "--json"], &[]));
    assert_eq!(v["source"], "docs-spec");
    assert_eq!(v["depth"], 0);
}

#[test]
fn the_nearest_directory_wins_the_walk() {
    let p = project("nearest");
    fs::create_dir_all(p.join("docs/spec")).unwrap();
    fs::create_dir_all(p.join("pkg/.spec")).unwrap();
    let v = json(&spec(&p.join("pkg"), &["root", "--json"], &[]));
    assert_eq!(v["source"], "dot-spec");
    assert_eq!(v["root"], p.join("pkg/.spec").to_string_lossy().as_ref());
}

#[test]
fn a_config_root_is_a_hit() {
    let p = project("config");
    write(&p, ".vibe/spec.json", "{\"root\": \"specs\"}\n");
    fs::create_dir_all(p.join("specs")).unwrap();
    fs::create_dir_all(p.join(".spec")).unwrap();
    let v = json(&spec(&p, &["root", "--json"], &[]));
    assert_eq!(v["source"], "config");
    assert_eq!(v["root"], p.join("specs").to_string_lossy().as_ref());
}

#[test]
fn a_config_without_root_falls_through() {
    let p = project("config-other-keys");
    write(&p, ".vibe/spec.json", "{\"strictness\": {}, \"tooth\": \"warn\"}\n");
    fs::create_dir_all(p.join(".spec")).unwrap();
    let out = spec(&p, &["root", "--json"], &[]);
    assert_eq!(out.code, 0, "{}", out.stderr);
    assert_eq!(json(&out)["source"], "dot-spec");
}

#[test]
fn a_config_root_that_is_missing_exits_3_without_falling_through() {
    let p = project("config-missing");
    write(&p, ".vibe/spec.json", "{\"root\": \"gone\"}\n");
    fs::create_dir_all(p.join(".spec")).unwrap();
    let out = spec(&p, &["root"], &[]);
    assert_eq!(out.code, 3);
    assert!(out.stdout.is_empty());
    assert!(out.stderr.contains("from config"), "{}", out.stderr);
}

#[test]
fn a_malformed_config_is_a_usage_error() {
    let p = project("config-bad");
    write(&p, ".vibe/spec.json", "{not json");
    fs::create_dir_all(p.join(".spec")).unwrap();
    assert_eq!(spec(&p, &["root"], &[]).code, 2);
    write(&p, ".vibe/spec.json", "{\"root\": 7}");
    assert_eq!(spec(&p, &["root"], &[]).code, 2);
}

#[test]
fn the_flag_beats_the_environment_and_both_beat_the_walk() {
    let p = project("explicit");
    fs::create_dir_all(p.join(".spec")).unwrap();
    fs::create_dir_all(p.join("a")).unwrap();
    fs::create_dir_all(p.join("b")).unwrap();
    let env = spec(&p, &["root", "--json"], &[("VIBE_SPEC_ROOT", "b")]);
    assert_eq!(json(&env)["source"], "env");
    assert_eq!(json(&env)["root"], p.join("b").to_string_lossy().as_ref());
    let flag = spec(&p, &["--root", "a", "root", "--json"], &[("VIBE_SPEC_ROOT", "b")]);
    assert_eq!(json(&flag)["source"], "flag");
    assert_eq!(json(&flag)["root"], p.join("a").to_string_lossy().as_ref());
    let missing = spec(&p, &["--root", "nope", "root"], &[]);
    assert_eq!(missing.code, 3);
    assert!(missing.stderr.contains("from flag"), "{}", missing.stderr);
}

#[test]
fn no_tree_exits_3_and_the_walk_stops_at_git() {
    let p = project("none");
    fs::create_dir_all(p.join("src")).unwrap();
    let out = spec(&p.join("src"), &["root", "--json"], &[]);
    assert_eq!(out.code, 3);
    assert!(out.stdout.is_empty());

    // A decoy above the .git boundary is never reached.
    let outer = project("outer");
    fs::create_dir_all(outer.join(".spec")).unwrap();
    fs::remove_dir_all(outer.join(".git")).unwrap();
    let inner = outer.join("repo");
    fs::create_dir_all(inner.join(".git")).unwrap();
    assert_eq!(spec(&inner, &["root"], &[]).code, 3);
    // Control: without the inner .git the walk reaches the decoy.
    fs::remove_dir_all(inner.join(".git")).unwrap();
    assert_eq!(spec(&inner, &["root"], &[]).code, 0);
}

#[test]
fn spec_dir_is_ignored_with_a_warning() {
    let p = project("specdir");
    fs::create_dir_all(p.join(".spec")).unwrap();
    fs::create_dir_all(p.join("elsewhere")).unwrap();
    let out = spec(&p, &["root", "--json"], &[("SPEC_DIR", "elsewhere")]);
    assert_eq!(json(&out)["source"], "dot-spec");
    assert!(out.stderr.contains("SPEC_DIR is ignored"), "{}", out.stderr);
}

#[cfg(unix)]
#[test]
fn root_json_matches_the_contract() {
    let p = project("contract");
    fs::create_dir_all(p.join("docs/spec")).unwrap();
    let out = spec(&p, &["root", "--json"], &[]);
    common::assert_spec_json("root", out.text().trim_end());
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test --locked -p vibe-spec --test root 2>&1 | tail -5`
Expected: FAIL — `vibe-spec root` is an unknown subcommand (exit 2).

- [ ] **Step 3: Add `serde_json` to the crate**

In `crates/vibe-spec/Cargo.toml` `[dependencies]` add:

```toml
serde_json = { workspace = true }
```

- [ ] **Step 4: Write `text.rs`**

Create `crates/vibe-spec/src/text.rs`:

```rust
//! Byte-level helpers that reproduce how the bash oracles read and print
//! text. The oracles run under `LC_ALL=C`, so everything here works on
//! bytes: no Unicode case folding, byte order, byte lengths.
use std::ffi::OsString;
use std::path::{Path, PathBuf};

/// Lines as `grep`, `awk` and `while read -r l || [[ -n $l ]]` see them:
/// split on `\n`; a final segment without `\n` counts when non-empty.
pub fn lines(text: &[u8]) -> Vec<&[u8]> {
    let mut out: Vec<&[u8]> = text.split(|&b| b == b'\n').collect();
    if out.last().is_some_and(|l| l.is_empty()) {
        out.pop();
    }
    out
}

/// Lines as `while IFS= read -r line` sees them: a final segment without
/// `\n` is dropped.
pub fn read_lines(text: &[u8]) -> Vec<&[u8]> {
    let mut out: Vec<&[u8]> = text.split(|&b| b == b'\n').collect();
    out.pop();
    out
}

/// `head -1`: the bytes before the first `\n`.
pub fn first_line(text: &[u8]) -> &[u8] {
    text.split(|&b| b == b'\n').next().unwrap_or_default()
}

/// `grep -q '^<prefix>'`.
pub fn has_line_starting(text: &[u8], prefix: &[u8]) -> bool {
    lines(text).iter().any(|l| l.starts_with(prefix))
}

/// Byte substring test; the empty needle is always found.
pub fn contains(hay: &[u8], needle: &[u8]) -> bool {
    needle.is_empty() || hay.windows(needle.len()).any(|w| w == needle)
}

/// ASCII case-insensitive substring test (`grep -qi` with a literal under `LC_ALL=C`).
pub fn contains_ci(hay: &[u8], needle: &[u8]) -> bool {
    needle.is_empty()
        || hay
            .windows(needle.len())
            .any(|w| w.eq_ignore_ascii_case(needle))
}

/// `sed 's/<needle>//'`: `hay` without the first occurrence of `needle`.
pub fn remove_first(hay: &[u8], needle: &[u8]) -> Vec<u8> {
    if needle.is_empty() {
        return hay.to_vec();
    }
    match hay.windows(needle.len()).position(|w| w == needle) {
        Some(i) => [&hay[..i], &hay[i + needle.len()..]].concat(),
        None => hay.to_vec(),
    }
}

/// `${line#prefix}`: `line` without `prefix` when it starts with it.
pub fn strip_prefix_or<'a>(line: &'a [u8], prefix: &[u8]) -> &'a [u8] {
    line.strip_prefix(prefix).unwrap_or(line)
}

/// `[[:space:]]` under `LC_ALL=C`.
pub fn is_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

/// `s` without leading `[[:space:]]`.
pub fn trim_start_space(s: &[u8]) -> &[u8] {
    let n = s.iter().take_while(|&&b| is_space(b)).count();
    s.get(n..).unwrap_or_default()
}

/// `s` without leading or trailing `[[:space:]]`.
pub fn trim_space(s: &[u8]) -> &[u8] {
    let s = trim_start_space(s);
    let n = s.iter().rev().take_while(|&&b| is_space(b)).count();
    s.get(..s.len() - n).unwrap_or_default()
}

/// What bash's `echo -e "$msg"` prints, newline included. Escapes:
/// `\a \b \e \E \f \n \r \t \v \\`, `\0nnn` (≤3 octal digits),
/// `\xHH` (≤2 hex digits), `\uHHHH`, `\UHHHHHHHH`; `\c` ends the output
/// there, newline and all. Any other `\X` prints as is. A code point
/// ≥ 0x80 from `\u`/`\U` prints as UTF-8 (bash under a UTF-8 locale); no
/// corpus case uses one, because the oracle's bytes depend on the locale.
pub fn echo_e(msg: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(msg.len() + 1);
    let mut i = 0;
    while let Some(&b) = msg.get(i) {
        let Some(&c) = msg.get(i + 1).filter(|_| b == b'\\') else {
            out.push(b);
            i += 1;
            continue;
        };
        i += 2;
        match c {
            b'a' => out.push(0x07),
            b'b' => out.push(0x08),
            b'e' | b'E' => out.push(0x1b),
            b'f' => out.push(0x0c),
            b'n' => out.push(b'\n'),
            b'r' => out.push(b'\r'),
            b't' => out.push(b'\t'),
            b'v' => out.push(0x0b),
            b'\\' => out.push(b'\\'),
            b'c' => return out,
            b'0' => {
                let (v, n) = digits(msg.get(i..).unwrap_or_default(), 3, 8);
                out.push((v & 0xff) as u8);
                i += n;
            }
            b'x' | b'u' | b'U' => {
                let max = match c {
                    b'x' => 2,
                    b'u' => 4,
                    _ => 8,
                };
                let (v, n) = digits(msg.get(i..).unwrap_or_default(), max, 16);
                if n == 0 {
                    out.extend_from_slice(&[b'\\', c]);
                } else if c == b'x' {
                    out.push((v & 0xff) as u8);
                } else {
                    let ch = char::from_u32(v).unwrap_or(char::REPLACEMENT_CHARACTER);
                    let mut buf = [0u8; 4];
                    out.extend_from_slice(ch.encode_utf8(&mut buf).as_bytes());
                }
                i += n;
            }
            _ => out.extend_from_slice(&[b'\\', c]),
        }
    }
    out.push(b'\n');
    out
}

/// Up to `max` leading digits of `s` in `radix`: (value, digits consumed).
fn digits(s: &[u8], max: usize, radix: u32) -> (u32, usize) {
    let mut v: u32 = 0;
    let mut n = 0;
    for &b in s.iter().take(max) {
        let Some(d) = (b as char).to_digit(radix) else {
            break;
        };
        v = v.wrapping_mul(radix).wrapping_add(d);
        n += 1;
    }
    (v, n)
}

/// `bytes` as a JSON string literal, quotes included. Invalid UTF-8 becomes U+FFFD.
pub fn json_str(bytes: &[u8]) -> Vec<u8> {
    serde_json::to_string(&lossy(bytes))
        .unwrap_or_else(|_| "\"\"".to_string())
        .into_bytes()
}

/// `bytes` as a `String`, invalid UTF-8 replaced.
pub fn lossy(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

/// `"$dir/$rel"` as text: `rel` is appended after a `/`, never resolved, so
/// an absolute `rel` does not replace `dir` (unlike `Path::join`).
pub fn join_raw(dir: &Path, rel: &[u8]) -> PathBuf {
    let mut s: OsString = dir.as_os_str().to_os_string();
    s.push("/");
    s.push(os_from_bytes(rel));
    PathBuf::from(s)
}

#[cfg(unix)]
fn os_from_bytes(b: &[u8]) -> OsString {
    use std::os::unix::ffi::OsStrExt;
    std::ffi::OsStr::from_bytes(b).to_os_string()
}

#[cfg(not(unix))]
fn os_from_bytes(b: &[u8]) -> OsString {
    OsString::from(lossy(b))
}

/// The last path component as text.
pub fn name_of(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

fn sorted_entries(dir: &Path, keep: impl Fn(&Path, &str) -> bool) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut out: Vec<PathBuf> = entries
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            let name = name_of(p);
            !name.starts_with('.') && keep(p, &name)
        })
        .collect();
    out.sort_by(|a, b| {
        a.as_os_str()
            .as_encoded_bytes()
            .cmp(b.as_os_str().as_encoded_bytes())
    });
    out
}

/// `"$dir"/*.md` under `nullglob`, files only, in byte order. A directory
/// named `*.md` is skipped (the oracle would error on it; no corpus case has one).
pub fn md_files(dir: &Path) -> Vec<PathBuf> {
    sorted_entries(dir, |p, name| name.ends_with(".md") && p.is_file())
}

/// `"$dir"/*/`: subdirectories (symlinks followed), in byte order.
pub fn sub_dirs(dir: &Path) -> Vec<PathBuf> {
    sorted_entries(dir, |p, _| p.is_dir())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lines_keep_a_final_unterminated_line_and_read_lines_drop_it() {
        assert_eq!(lines(b"a\nb"), vec![&b"a"[..], b"b"]);
        assert_eq!(lines(b"a\n\n"), vec![&b"a"[..], b""]);
        assert!(lines(b"").is_empty());
        assert_eq!(read_lines(b"a\nb"), vec![&b"a"[..]]);
        assert_eq!(read_lines(b"a\nb\n"), vec![&b"a"[..], b"b"]);
        assert!(read_lines(b"").is_empty());
    }

    #[test]
    fn echo_e_interprets_like_bash() {
        assert_eq!(echo_e(b"\\033[31m  ERROR: x\\033[0m"), b"\x1b[31m  ERROR: x\x1b[0m\n");
        assert_eq!(echo_e(b"a\\tb\\x41\\\\"), b"a\tbA\\\n");
        assert_eq!(echo_e(b"keep \\q and \\"), b"keep \\q and \\\n");
        assert_eq!(echo_e(b"stop\\chere"), b"stop");
        assert_eq!(echo_e(b"\\xZZ"), b"\\xZZ\n");
    }

    #[test]
    fn join_raw_never_replaces_the_base() {
        assert_eq!(join_raw(Path::new(".spec"), b"/x.md"), PathBuf::from(".spec//x.md"));
        assert_eq!(join_raw(Path::new(".spec"), b"a.md"), PathBuf::from(".spec/a.md"));
    }

    #[test]
    fn substring_helpers_follow_grep_and_sed() {
        assert!(contains_ci(b"**Tags:** Bash, Quoting", b"bash"));
        assert!(contains_ci(b"", b""));
        assert!(!contains_ci(b"bash", b"b.sh"));
        assert_eq!(remove_first(b"type: type: x", b"type: "), b"type: x");
        assert_eq!(remove_first(b"type:x", b"type: "), b"type:x");
        assert_eq!(trim_space(b" \t x y \r"), b"x y");
    }
}
```

- [ ] **Step 5: Write `root.rs`**

Create `crates/vibe-spec/src/root.rs`:

```rust
//! Spec root discovery (`.spec/tech-spec.md`, "Root and discovery"). One
//! function implements the order; every command goes through it.
//!
//! 1. `--root` 2. `$VIBE_SPEC_ROOT` 3. `.vibe/spec.json` `root`
//! 4. `docs/spec/` 5. `.spec/` 6. repeat 3–5 per parent, up to the
//! directory holding `.git` 7. nothing found → exit 3.
use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};
use vibe_core::exit;

/// The discovery step that found the root (`root.source` in Spec JSON v1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    /// `--root`.
    Flag,
    /// `$VIBE_SPEC_ROOT`.
    Env,
    /// `.vibe/spec.json` key `root`.
    Config,
    /// `docs/spec/`.
    DocsSpec,
    /// `.spec/` (legacy).
    DotSpec,
}

impl Source {
    /// The contract spelling.
    pub fn as_str(self) -> &'static str {
        match self {
            Source::Flag => "flag",
            Source::Env => "env",
            Source::Config => "config",
            Source::DocsSpec => "docs-spec",
            Source::DotSpec => "dot-spec",
        }
    }
}

/// A found spec root.
#[derive(Debug)]
pub struct Root {
    /// Absolute path of the spec root.
    pub dir: PathBuf,
    /// The project directory: where a walk step hit, or for `--root` and
    /// `$VIBE_SPEC_ROOT` the nearest ancestor of the cwd holding `.git`.
    pub project: PathBuf,
    /// The step that hit.
    pub source: Source,
    /// Parent directories walked before the hit; 0 for the cwd, flag and env.
    pub depth: usize,
}

impl Root {
    /// `dir` relative to `project`, `/`-joined (`.spec`, `docs/spec`); the
    /// absolute path when `dir` lies outside `project`.
    pub fn display(&self) -> String {
        match self.dir.strip_prefix(&self.project) {
            Ok(rel) if !rel.as_os_str().is_empty() => rel
                .components()
                .map(|c| c.as_os_str().to_string_lossy().into_owned())
                .collect::<Vec<_>>()
                .join("/"),
            _ => self.dir.to_string_lossy().into_owned(),
        }
    }
}

/// Why discovery found no root.
#[derive(Debug)]
pub enum Miss {
    /// No step hit.
    NotFound,
    /// `--root`, `$VIBE_SPEC_ROOT` or a config `root` names no directory.
    Missing {
        /// The step that named it.
        source: Source,
        /// The path it named.
        path: PathBuf,
    },
    /// `.vibe/spec.json` is unreadable, not a JSON object, or `root` is not a non-empty string.
    BadConfig {
        /// The config file.
        path: PathBuf,
        /// What is wrong with it.
        why: String,
    },
}

impl Miss {
    /// Exit code: 2 for a bad config, else 3.
    pub fn code(&self) -> u8 {
        match self {
            Miss::BadConfig { .. } => exit::USAGE,
            _ => exit::NO_ROOT,
        }
    }

    /// One stderr line.
    pub fn message(&self) -> String {
        match self {
            Miss::NotFound => "vibe-spec: no spec root (searched docs/spec/ and .spec/ up to the directory holding .git)".to_string(),
            Miss::Missing { source, path } => format!(
                "vibe-spec: spec root {} (from {}) is not a directory",
                path.display(),
                source.as_str()
            ),
            Miss::BadConfig { path, why } => format!("vibe-spec: {}: {why}", path.display()),
        }
    }
}

/// Runs the discovery order from `cwd`.
pub fn discover(cwd: &Path, flag: Option<&Path>, env: Option<&OsStr>) -> Result<Root, Miss> {
    if let Some(path) = flag {
        return explicit(cwd, path, Source::Flag);
    }
    if let Some(path) = env.filter(|p| !p.is_empty()) {
        return explicit(cwd, Path::new(path), Source::Env);
    }
    let mut dir = cwd.to_path_buf();
    let mut depth = 0;
    loop {
        if let Some(found) = at(&dir, depth)? {
            return Ok(found);
        }
        if dir.join(".git").exists() {
            return Err(Miss::NotFound);
        }
        match dir.parent() {
            Some(up) => {
                dir = up.to_path_buf();
                depth += 1;
            }
            None => return Err(Miss::NotFound),
        }
    }
}

fn explicit(cwd: &Path, path: &Path, source: Source) -> Result<Root, Miss> {
    let dir = cwd.join(path);
    if !dir.is_dir() {
        return Err(Miss::Missing { source, path: dir });
    }
    Ok(Root {
        dir,
        project: project_of(cwd),
        source,
        depth: 0,
    })
}

/// Steps 3–5 at one directory.
fn at(dir: &Path, depth: usize) -> Result<Option<Root>, Miss> {
    let config = dir.join(".vibe").join("spec.json");
    if config.is_file() {
        if let Some(rel) = config_root(&config)? {
            let spec = dir.join(rel);
            if !spec.is_dir() {
                return Err(Miss::Missing {
                    source: Source::Config,
                    path: spec,
                });
            }
            return Ok(Some(Root {
                dir: spec,
                project: dir.to_path_buf(),
                source: Source::Config,
                depth,
            }));
        }
    }
    for (rel, source) in [("docs/spec", Source::DocsSpec), (".spec", Source::DotSpec)] {
        let spec = dir.join(rel);
        if spec.is_dir() {
            return Ok(Some(Root {
                dir: spec,
                project: dir.to_path_buf(),
                source,
                depth,
            }));
        }
    }
    Ok(None)
}

/// The `root` key of a config file; `None` when it has none.
fn config_root(path: &Path) -> Result<Option<PathBuf>, Miss> {
    let bad = |why: String| Miss::BadConfig {
        path: path.to_path_buf(),
        why,
    };
    let text = std::fs::read_to_string(path).map_err(|e| bad(e.to_string()))?;
    let value: Value = serde_json::from_str(&text).map_err(|e| bad(format!("not JSON: {e}")))?;
    let Value::Object(map) = value else {
        return Err(bad("not a JSON object".to_string()));
    };
    match map.get("root") {
        None => Ok(None),
        Some(Value::String(s)) if !s.is_empty() => Ok(Some(PathBuf::from(s))),
        Some(_) => Err(bad("`root` must be a non-empty string".to_string())),
    }
}

/// The nearest ancestor of `cwd` (inclusive) holding `.git`; `cwd` when none does.
fn project_of(cwd: &Path) -> PathBuf {
    cwd.ancestors()
        .find(|d| d.join(".git").exists())
        .unwrap_or(cwd)
        .to_path_buf()
}

/// `vibe-spec root [--json]`.
pub fn command(found: &Result<Root, Miss>, json: bool) -> (u8, Vec<u8>) {
    let root = match found {
        Ok(root) => root,
        Err(miss) => {
            eprintln!("{}", miss.message());
            return (miss.code(), Vec::new());
        }
    };
    if !json {
        return (exit::OK, format!("{}\n", root.dir.display()).into_bytes());
    }
    let mut v = json!({
        "v": 1,
        "root": root.dir.to_string_lossy(),
        "source": root.source.as_str(),
    });
    if !matches!(root.source, Source::Flag | Source::Env) {
        v["depth"] = json!(root.depth);
    }
    (exit::OK, format!("{v}\n").into_bytes())
}
```

- [ ] **Step 6: Write `main.rs`**

Replace `crates/vibe-spec/src/main.rs` with:

```rust
//! `vibe-spec`: the spec tool. F16 ports the read-only spec scripts and adds
//! root discovery and the Spec JSON v1 queries.
use std::io::Write;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use vibe_core::exit;

/// Concatenates byte-slice-like values (`&str`, `&[u8]`, `Vec<u8>`, `String`) into one `Vec<u8>`.
macro_rules! cat {
    ($($part:expr),* $(,)?) => {{
        let mut v: Vec<u8> = Vec::new();
        $(v.extend_from_slice(AsRef::<[u8]>::as_ref(&$part));)*
        v
    }};
}

mod root;
mod text;

/// The vibe spec tool.
#[derive(Parser)]
#[command(name = "vibe-spec", version, about, arg_required_else_help = true)]
struct Cli {
    /// Spec root directory; skips discovery. Goes before the command.
    #[arg(long, value_name = "DIR")]
    root: Option<PathBuf>,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Print the spec root and how it was found.
    Root {
        /// Print Spec JSON v1.
        #[arg(long)]
        json: bool,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let cwd = match std::env::current_dir() {
        Ok(dir) => dir,
        Err(e) => {
            eprintln!("vibe-spec: cannot read the current directory: {e}");
            return ExitCode::from(exit::USAGE);
        }
    };
    if std::env::var_os("SPEC_DIR").is_some() {
        eprintln!("vibe-spec: SPEC_DIR is ignored; set VIBE_SPEC_ROOT instead");
    }
    let env_root = std::env::var_os("VIBE_SPEC_ROOT");
    let found = root::discover(&cwd, cli.root.as_deref(), env_root.as_deref());
    let (code, out) = match cli.command {
        Command::Root { json } => root::command(&found, json),
    };
    emit(&out);
    ExitCode::from(code)
}

/// Writes `bytes` to stdout. A closed pipe is not an error.
fn emit(bytes: &[u8]) {
    let mut out = std::io::stdout().lock();
    let _ = out.write_all(bytes);
    let _ = out.flush();
}
```

Until Tasks 3–7 add callers, `cat!`, most `text` helpers, `Root::display` and `Root::project` are unused. Clippy runs with `-D warnings`, so put this directly after the `//!` doc lines of `main.rs`:

```rust
// F16 in progress: callers arrive in Tasks 3–7; Task 7 removes this.
#![allow(dead_code, unused_macros)]
```

- [ ] **Step 7: Run the tests to verify they pass**

```bash
cargo test --locked -p vibe-spec 2>&1 | grep -E 'test result|FAILED|panicked'
cargo clippy --workspace --all-targets --locked -- -D warnings 2>&1 | tail -1
cargo fmt --all --check && echo fmt-ok
cargo test --locked -p vibe-parity --test workspace_graph 2>&1 | grep 'test result'
```

Expected:
- `test result: ok` for the unit tests, `cli`, and `root` (11 tests);
- clippy `Finished`, and `fmt-ok`;
- `workspace_graph`: ok, since `serde_json` is on the allowlist.

- [ ] **Step 8: Commit**

```bash
git add crates/vibe-spec Cargo.lock
git commit -m "feat(spec): add root discovery and byte helpers"
```

### Task 3: Port `list-specs`; parity entry point and CI

**Files:**
- Create: `crates/vibe-spec/src/list_specs.rs`
- Modify: `crates/vibe-spec/src/main.rs` (add `mod list_specs;`, the `ListSpecs` variant, dispatch)
- Create: `crates/vibe-spec/tests/list_specs.rs`
- Modify: `crates/vibe-parity/src/corpus_spec.rs` (case `features/empty-feature`)
- Regenerate: `tests/parity/corpus/spec/`
- Modify: `tests/parity/spec.json` (`list-specs` candidate; `features` floor)
- Create: `tests/parity/run.sh`
- Modify: `.github/workflows/ci.yml` (job `parity`)

**Interfaces:**
- Consumes: `root::{Root, Miss}`; `text::{lines, md_files, sub_dirs, name_of, remove_first}`; `cat!`.
- Produces:
  - `list_specs::run(found: &Result<Root, Miss>) -> (u8, Vec<u8>)`.
  - The legacy subcommand shape that Tasks 4–6 reuse: a variant with `#[command(disable_help_flag = true)]` and `#[arg(trailing_var_arg = true, allow_hyphen_values = true)] args: Vec<String>`.
  - `tests/parity/run.sh` with a `live <command> <oracle script> [args…]` function that later tasks extend.

- [ ] **Step 1: Write the failing tests**

Create `crates/vibe-spec/tests/list_specs.rs`:

```rust
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
    write(&p, ".spec/tech-api.md", "---\ntype: tech-topic\nscope:x\n---\n");
    write(&p, ".spec/odd.md", "no frontmatter, type: inline\n");
    write(&p, ".spec/lessons.md", "# Lessons\n");
    write(&p, ".spec/features/auth/product.md", "---\ntype: feature-product\nscope: auth\n---\n");
    write(&p, ".spec/features/auth/plan.md", "---\ntype: feature-plan\n---\n");
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
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test --locked -p vibe-spec --test list_specs 2>&1 | tail -5`
Expected: FAIL — the candidate exits 2 (`unrecognized subcommand 'list-specs'`).

- [ ] **Step 3: Write `list_specs.rs`**

Create `crates/vibe-spec/src/list_specs.rs`:

```rust
//! `vibe-spec list-specs` — port of `spec/scripts/list-specs.sh`
//! (byte-identical; arguments are ignored, as the oracle ignores them).
use std::path::Path;

use crate::root::{Miss, Root};
use crate::text::{lines, md_files, name_of, remove_first, sub_dirs};

/// Runs the command.
pub fn run(found: &Result<Root, Miss>) -> (u8, Vec<u8>) {
    let root = match found {
        Ok(root) => root,
        Err(Miss::NotFound) => {
            return (0, b"No .spec/ directory found in current project\n".to_vec());
        }
        Err(miss) => {
            eprintln!("{}", miss.message());
            return (miss.code(), Vec::new());
        }
    };
    let specs = md_files(&root.dir);
    let feature_dirs = sub_dirs(&root.dir.join("features"));
    if specs.is_empty() && feature_dirs.is_empty() {
        return (0, format!("No spec files found in {}/\n", root.display()).into_bytes());
    }
    let mut out = Vec::new();
    for file in &specs {
        entry(&mut out, file, name_of(file).as_bytes());
    }
    if !feature_dirs.is_empty() {
        out.extend_from_slice(b"\n## Features\n");
        for dir in &feature_dirs {
            let feature = name_of(dir);
            out.extend(format!("\n### {feature}\n").bytes());
            let docs = md_files(dir);
            if docs.is_empty() {
                out.extend_from_slice(b"- (no docs)\n");
                continue;
            }
            for file in &docs {
                let display = format!("features/{feature}/{}", name_of(file));
                entry(&mut out, file, display.as_bytes());
            }
        }
    }
    (0, out)
}

/// `- <display> [<area>] (<type>: <scope>)`.
fn entry(out: &mut Vec<u8>, file: &Path, display: &[u8]) {
    let text = std::fs::read(file).unwrap_or_default();
    let ty = field(&text, b"type:", b"type: ");
    let scope = field(&text, b"scope:", b"scope: ");
    out.extend(cat!(
        "- ",
        display,
        " [",
        area(&name_of(file)),
        "] (",
        ty,
        ": ",
        scope,
        ")\n"
    ));
}

/// `grep -m1 "^<key>" | sed 's/<strip>//'`.
fn field(text: &[u8], key: &[u8], strip: &[u8]) -> Vec<u8> {
    lines(text)
        .into_iter()
        .find(|l| l.starts_with(key))
        .map(|l| remove_first(l, strip))
        .unwrap_or_default()
}

/// `area_from_name`.
fn area(name: &str) -> &'static str {
    match name {
        "design.md" => "design",
        "lessons.md" => "lessons",
        n if n == "product.md" || n.starts_with("product-") => "product",
        n if n == "tech.md" || n.starts_with("tech-") => "tech",
        n if n == "plan.md" || n.starts_with("plan-") => "plan",
        _ => "unknown",
    }
}
```

- [ ] **Step 4: Wire the subcommand**

In `crates/vibe-spec/src/main.rs`, add `mod list_specs;` after `mod root;`, add this variant to `Command`:

```rust
    /// List spec files (port of list-specs.sh).
    #[command(disable_help_flag = true)]
    ListSpecs {
        /// Ignored, as the oracle ignores them.
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
```

and this arm to the dispatch `match`:

```rust
        Command::ListSpecs { args: _ } => list_specs::run(&found),
```

- [ ] **Step 5: Run the tests to verify they pass**

Run: `cargo test --locked -p vibe-spec 2>&1 | grep -E 'test result|FAILED|panicked'`
Expected: every `test result: ok`.

- [ ] **Step 6: Add the corpus case and switch the command to candidate mode**

In `crates/vibe-parity/src/corpus_spec.rs` `cases()`, append to the returned `vec!`:

```rust
        CaseDef {
            class: "features",
            name: "empty-feature",
            cwd: "",
            runs: &[],
            files: with(feature(FEATURE_TECH, &[]), &[(".spec/features/draft/notes.txt", "x\n")]),
        },
```

In `tests/parity/spec.json`, set the `list-specs` command's `"candidate"` to `["{candidate}", "list-specs"]` and the floor `"features"` to `3`. Then:

```bash
cargo run --locked -q -p vibe-parity -- gen-spec-corpus oracles/bash/spec/reference/templates tests/parity/corpus/spec
```

Expected: `gen-spec-corpus: 13 cases written to tests/parity/corpus/spec`.

- [ ] **Step 7: Write `tests/parity/run.sh`**

```bash
#!/usr/bin/env bash
# Spec parity (F16): the frozen bash oracles against themselves, against the
# vibe-spec candidate over the generated corpus, and against the candidate on
# this repo's own spec tree ("live"). Needs cargo. Exit 0 only when all pass.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"
command -v cargo >/dev/null 2>&1 || { echo "parity: cargo not found on PATH" >&2; exit 1; }
cargo build --locked -q -p vibe-spec -p vibe-parity
BIN="${CARGO_TARGET_DIR:-$ROOT/target}/debug"
SCRATCH="$(mktemp -d)"
trap 'rm -rf "$SCRATCH"' EXIT

echo "=== self-check (oracle vs oracle) ==="
"$BIN/vibe-parity" run tests/parity/spec.json --self | grep -v '^PASS '
echo "=== candidate (oracle vs vibe-spec) ==="
"$BIN/vibe-parity" run tests/parity/spec.json --candidate "$BIN/vibe-spec" | grep -v '^PASS '

echo "=== live (this repo's spec tree) ==="
fails=0
compared=0
live() { # <vibe-spec command> <oracle script> [args...]
  local command="$1" script="$2" orc=0 crc=0
  shift 2
  env -i PATH="$PATH" HOME="$HOME" LC_ALL=C TZ=UTC \
    bash "oracles/bash/spec/scripts/$script" "$@" >"$SCRATCH/oracle" 2>/dev/null || orc=$?
  if [[ "$(uname)" == Darwin ]]; then
    # BSD wc pads validate's SF15 count (P3a-R2); the port prints it unpadded.
    sed -E 's/(SF15: [^ ]+ is) +/\1 /' "$SCRATCH/oracle" >"$SCRATCH/oracle.n"
    mv "$SCRATCH/oracle.n" "$SCRATCH/oracle"
  fi
  env -i PATH="$PATH" HOME="$HOME" LC_ALL=C TZ=UTC \
    "$BIN/vibe-spec" "$command" "$@" >"$SCRATCH/candidate" 2>/dev/null || crc=$?
  compared=$((compared + 1))
  if [[ "$orc" == "$crc" ]] && cmp -s "$SCRATCH/oracle" "$SCRATCH/candidate"; then
    echo "PASS  live $command $*"
  else
    echo "FAIL  live $command $* (exit: oracle $orc, candidate $crc)"
    diff "$SCRATCH/oracle" "$SCRATCH/candidate" | head -20 || true
    fails=$((fails + 1))
  fi
}
live list-specs list-specs.sh
[[ "$compared" -ge 1 ]] || { echo "parity: live compared nothing" >&2; exit 1; }
[[ "$fails" -eq 0 ]] || { echo "parity: $fails live mismatch(es)" >&2; exit 1; }
echo "parity: self-check, candidate and live ($compared) green"
```

Run: `chmod +x tests/parity/run.sh`

- [ ] **Step 8: Point CI's parity job at it**

In `.github/workflows/ci.yml`, replace the `parity` job's comment, `name` and last step with:

```yaml
  parity:
    # F3/F16: the frozen spec oracles against themselves, against the
    # vibe-spec candidate over the generated corpus, and live on this repo's
    # spec tree. Commands not ported yet stay self-check only.
    name: parity — spec oracles vs vibe-spec
```

```yaml
      - name: Spec parity (self-check, candidate, live)
        run: bash tests/parity/run.sh
```

- [ ] **Step 9: Run parity and the shell checks**

```bash
bash tests/parity/run.sh
git ls-files '*.sh' tests/parity/run.sh | grep -Ev '^oracles/(bash|js)/' | sort -u | xargs shellcheck; echo "shellcheck rc=$?"
cargo test --locked -p vibe-parity --test spec_corpus 2>&1 | grep 'test result'
```

Expected:
- self-check: `0 failed, 0 error(s)`;
- candidate: `list-specs` runs `PASS`; `SKIP` lines for the six other commands; summary `0 failed, 0 error(s)`, `6 skipped`;
- `PASS  live list-specs`, then `parity: self-check, candidate and live (1) green`;
- `shellcheck rc=0`, and `spec_corpus` ok.

- [ ] **Step 10: Commit**

```bash
git add crates tests/parity .github/workflows/ci.yml
git commit -m "feat(spec): port list-specs with parity"
```

### Task 4: Port `lessons-for`, plus `--json` and `--tag`

**Files:**
- Create: `crates/vibe-spec/src/lessons.rs`
- Modify: `crates/vibe-spec/src/text.rs` (add `is_date`)
- Modify: `crates/vibe-spec/src/main.rs` (`mod lessons;`, `LessonsFor` variant, dispatch)
- Create: `crates/vibe-spec/tests/lessons.rs`
- Modify: `crates/vibe-parity/src/corpus_spec.rs` (cases `lessons/no-newline`, `lessons/edge-args`)
- Regenerate: `tests/parity/corpus/spec/`
- Modify: `tests/parity/spec.json` (candidate, pins, `lessons` floor)
- Modify: `tests/parity/run.sh` (live lines)

**Interfaces:**
- Consumes: `root::{Root, Miss}`; `text::{lines, contains_ci, remove_first, strip_prefix_or, trim_space, is_space, json_str, lossy}`; `cat!`.
- Produces:
  - `lessons::run(found: &Result<Root, Miss>, args: &[String]) -> (u8, Vec<u8>)`.
  - `text::is_date(b: &[u8]) -> bool` (Task 8 reuses it).

- [ ] **Step 1: Write the failing tests**

Create `crates/vibe-spec/tests/lessons.rs`:

```rust
#![allow(clippy::unwrap_used, clippy::expect_used)]
//! `vibe-spec lessons-for`: legacy formats match `lessons-for.sh`; `--json`
//! answers Spec JSON v1.
mod common;

use common::{project, spec, write};
use serde_json::{Value, json};

const LESSONS: &str = "# Lessons\n\nIntro text.\n\n\
### Quote every expansion\n**Pattern:** Seen once.\n**Rule:** Write \"$var\".\n**Tags:** bash, quoting\n**Date:** 2026-01-02\n\n\
### Pinned rule\n**Rule:** Read lessons first.\n**Tags:** spec\n**Date:** someday\n**Pinned-by:** review\n\n\
### No rule here\n**Tags:** bash\n";

#[cfg(unix)]
#[test]
fn legacy_formats_match_the_oracle() {
    use common::assert_parity;
    let p = project("lessons-legacy");
    write(&p, ".spec/lessons.md", LESSONS);
    for args in [
        &["bash"][..],
        &["BASH", "spec"],
        &["--format", "inject", "spec"],
        &["--format=markdown", "quoting"],
        &["json", "spec"],
        &["--format", "xml", "bash"],
        &["nomatch"],
        &[""],
        &["--bogus"],
        &["--format"],
        &[],
    ] {
        assert_parity(&p, "lessons-for", "lessons-for.sh", args);
    }
    let none = project("lessons-none");
    assert_parity(&none, "lessons-for", "lessons-for.sh", &[]);
    assert_parity(&none, "lessons-for", "lessons-for.sh", &["bash"]);
}

#[test]
fn json_answers_spec_json_v1() {
    let p = project("lessons-json");
    write(&p, ".spec/lessons.md", LESSONS);
    let out = spec(&p, &["lessons-for", "--json", "bash", "--tag", "spec"], &[]);
    assert_eq!(out.code, 0, "{}", out.stderr);
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(
        v,
        json!({
            "v": 1,
            "tags": ["bash", "spec"],
            "lessons": [
                {
                    "title": "Quote every expansion",
                    "rule": "Write \"$var\".",
                    "tags": ["bash", "quoting"],
                    "pattern": "Seen once.",
                    "date": "2026-01-02"
                },
                { "title": "Pinned rule", "rule": "Read lessons first.", "tags": ["spec"], "pinned": true }
            ]
        })
    );
    #[cfg(unix)]
    common::assert_spec_json("lessons-for", out.text().trim_end());
}

#[test]
fn json_exit_codes_follow_the_contract() {
    let p = project("lessons-json-codes");
    std::fs::create_dir_all(p.join(".spec")).unwrap();
    let empty = spec(&p, &["lessons-for", "--json", "bash"], &[]);
    assert_eq!(empty.code, 0);
    assert_eq!(empty.text(), "{\"lessons\":[],\"tags\":[\"bash\"],\"v\":1}\n");
    assert_eq!(spec(&p, &["lessons-for", "--json"], &[]).code, 2);
    let none = project("lessons-json-none");
    assert_eq!(spec(&none, &["lessons-for", "--json", "bash"], &[]).code, 3);
    assert_eq!(spec(&none, &["lessons-for", "bash"], &[]).code, 0, "legacy no-root stays exit 0");
}

#[test]
fn tags_match_literally_not_as_regex() {
    let p = project("lessons-literal");
    write(&p, ".spec/lessons.md", LESSONS);
    let out = spec(&p, &["lessons-for", "b.sh"], &[]);
    assert_eq!(out.code, 0);
    assert!(out.stdout.is_empty(), "b.sh must not match bash: {}", out.text());
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test --locked -p vibe-spec --test lessons 2>&1 | tail -5`
Expected: FAIL — unrecognized subcommand `lessons-for`.

- [ ] **Step 3: Add `is_date` to `text.rs`**

```rust
/// `YYYY-MM-DD` (digits and dashes only; the calendar is not checked).
pub fn is_date(b: &[u8]) -> bool {
    b.len() == 10
        && b.iter().enumerate().all(|(i, c)| match i {
            4 | 7 => *c == b'-',
            _ => c.is_ascii_digit(),
        })
}
```

- [ ] **Step 4: Write `lessons.rs`**

Create `crates/vibe-spec/src/lessons.rs`:

```rust
//! `vibe-spec lessons-for` — port of `spec/scripts/lessons-for.sh`, plus
//! `--json` (Spec JSON v1 `lessons-for`) and `--tag <t>`.
//!
//! Deliberate divergences (pinned in tests/parity/spec.json): tags match as
//! case-insensitive literal substrings, not as regexes; the legacy
//! `--format json` array escapes its strings.
use serde_json::{Value, json};
use vibe_core::exit;

use crate::root::{Miss, Root};
use crate::text::{contains_ci, is_date, is_space, json_str, lines, lossy, remove_first, strip_prefix_or, trim_space};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Format {
    Markdown,
    Inject,
    Json,
}

impl Format {
    fn parse(s: &str) -> Format {
        match s {
            "inject" => Format::Inject,
            "json" => Format::Json,
            _ => Format::Markdown,
        }
    }
}

/// Runs the command.
pub fn run(found: &Result<Root, Miss>, args: &[String]) -> (u8, Vec<u8>) {
    let mut format = Format::Markdown;
    let mut tags: Vec<String> = Vec::new();
    let mut spec_json = false;
    let mut i = 0;
    while let Some(arg) = args.get(i) {
        let a = arg.as_str();
        match a {
            "--format" => match args.get(i + 1).filter(|v| !v.is_empty()) {
                Some(v) => {
                    format = Format::parse(v);
                    i += 2;
                    continue;
                }
                None => {
                    eprintln!("lessons-for: --format requires a value (markdown|inject|json)");
                    return (exit::FINDINGS, Vec::new());
                }
            },
            "--json" => spec_json = true,
            "--tag" => match args.get(i + 1) {
                Some(v) => {
                    tags.push(v.clone());
                    i += 2;
                    continue;
                }
                None => {
                    eprintln!("lessons-for: --tag requires a value");
                    return (exit::USAGE, Vec::new());
                }
            },
            "markdown" | "inject" | "json" => format = Format::parse(a),
            _ if a.starts_with("--format=") => format = Format::parse(&a["--format=".len()..]),
            _ if a.starts_with('-') => {
                eprintln!("Unknown option: {a}");
                return (exit::FINDINGS, Vec::new());
            }
            _ => tags.push(a.to_string()),
        }
        i += 1;
    }
    if spec_json {
        return answer(found, &tags);
    }
    let file = match found {
        Ok(root) => root.dir.join("lessons.md"),
        Err(Miss::NotFound) => return (exit::OK, Vec::new()),
        Err(miss) => {
            eprintln!("{}", miss.message());
            return (miss.code(), Vec::new());
        }
    };
    if !file.is_file() {
        return (exit::OK, Vec::new());
    }
    if tags.is_empty() {
        eprintln!("usage: vibe-spec lessons-for <tag> [<tag>...] [--format markdown|inject|json]");
        return (exit::FINDINGS, Vec::new());
    }
    let text = std::fs::read(&file).unwrap_or_default();
    let matched: Vec<Vec<u8>> = blocks(&text)
        .into_iter()
        .filter(|b| matches(b, &tags))
        .collect();
    if matched.is_empty() {
        return (exit::OK, Vec::new());
    }
    let mut out = Vec::new();
    match format {
        Format::Inject => {
            out.extend(format!("<!-- lessons: {} -->\n", tags.join(" ")).bytes());
            for b in &matched {
                out.extend(cat!(b, "\n\n"));
            }
            out.extend_from_slice(b"<!-- /lessons -->\n");
        }
        Format::Json => {
            out.push(b'[');
            for (n, b) in matched.iter().enumerate() {
                if n > 0 {
                    out.push(b',');
                }
                out.extend(legacy_json(b));
            }
            out.extend_from_slice(b"]\n");
        }
        Format::Markdown => {
            for b in &matched {
                out.extend(cat!(b, "\n---\n"));
            }
        }
    }
    (exit::OK, out)
}

/// `^###[[:space:]]`.
fn is_heading(line: &[u8]) -> bool {
    line.starts_with(b"###") && line.get(3).is_some_and(|&c| is_space(c))
}

/// Lesson blocks: each `### ` heading with the lines up to the next one, joined by `\n`.
fn blocks(text: &[u8]) -> Vec<Vec<u8>> {
    let mut out = Vec::new();
    let mut current: Option<Vec<u8>> = None;
    for line in lines(text) {
        if is_heading(line) {
            out.extend(current.take());
            current = Some(line.to_vec());
        } else if let Some(block) = current.as_mut() {
            block.push(b'\n');
            block.extend_from_slice(line);
        }
    }
    out.extend(current);
    out
}

/// The first `**Tags:**` line contains any tag (an empty tag always matches).
fn matches(block: &[u8], tags: &[String]) -> bool {
    let tags_line = lines(block)
        .into_iter()
        .find(|l| l.starts_with(b"**Tags:**"))
        .unwrap_or_default();
    tags.iter().any(|t| contains_ci(tags_line, t.as_bytes()))
}

/// Every line starting with `key`, `strip` removed once, joined by `\n`, trailing newlines dropped.
fn joined(block: &[u8], key: &[u8], strip: &[u8]) -> Vec<u8> {
    let mut v = lines(block)
        .into_iter()
        .filter(|l| l.starts_with(key))
        .map(|l| remove_first(l, strip))
        .collect::<Vec<_>>()
        .join(&b'\n');
    while v.last() == Some(&b'\n') {
        v.pop();
    }
    v
}

/// One element of the legacy `--format json` array (strings escaped).
fn legacy_json(block: &[u8]) -> Vec<u8> {
    let title = strip_prefix_or(lines(block).first().copied().unwrap_or_default(), b"### ");
    cat!(
        "{\"title\":",
        json_str(title),
        ",\"pattern\":",
        json_str(&joined(block, b"**Pattern:**", b"**Pattern:** ")),
        ",\"rule\":",
        json_str(&joined(block, b"**Rule:**", b"**Rule:** ")),
        ",\"tags\":",
        json_str(&joined(block, b"**Tags:**", b"**Tags:** ")),
        ",\"date\":",
        json_str(&joined(block, b"**Date:**", b"**Date:** ")),
        "}"
    )
}

/// `--json`: Spec JSON v1 `lessons-for`.
fn answer(found: &Result<Root, Miss>, tags: &[String]) -> (u8, Vec<u8>) {
    let root = match found {
        Ok(root) => root,
        Err(miss) => {
            eprintln!("{}", miss.message());
            return (miss.code(), Vec::new());
        }
    };
    if tags.is_empty() {
        eprintln!("usage: vibe-spec lessons-for --json <tag> [<tag>...]");
        return (exit::USAGE, Vec::new());
    }
    let text = std::fs::read(root.dir.join("lessons.md")).unwrap_or_default();
    let lessons: Vec<Value> = blocks(&text)
        .iter()
        .filter(|b| matches(b, tags))
        .filter_map(|b| lesson(b))
        .collect();
    let v = json!({ "v": 1, "tags": tags, "lessons": lessons });
    (exit::OK, format!("{v}\n").into_bytes())
}

/// One Spec JSON lesson; `None` without a title or a rule.
fn lesson(block: &[u8]) -> Option<Value> {
    let ls = lines(block);
    let title = trim_space(strip_prefix_or(ls.first().copied()?, b"###"));
    let value = |key: &[u8]| {
        ls.iter()
            .find(|l| l.starts_with(key))
            .map(|l| trim_space(l.get(key.len()..).unwrap_or_default()).to_vec())
    };
    let rule = value(b"**Rule:**").filter(|r| !r.is_empty())?;
    if title.is_empty() {
        return None;
    }
    let tags: Vec<String> = value(b"**Tags:**")
        .map(|t| {
            t.split(|&c| c == b',')
                .map(trim_space)
                .filter(|t| !t.is_empty())
                .map(lossy)
                .collect()
        })
        .unwrap_or_default();
    let mut v = json!({ "title": lossy(title), "rule": lossy(&rule), "tags": tags });
    if let Some(p) = value(b"**Pattern:**").filter(|p| !p.is_empty()) {
        v["pattern"] = json!(lossy(&p));
    }
    if let Some(d) = value(b"**Date:**").filter(|d| is_date(d)) {
        v["date"] = json!(lossy(&d));
    }
    if ls.iter().any(|l| l.starts_with(b"**Pinned-by:**")) {
        v["pinned"] = json!(true);
    }
    Some(v)
}
```

- [ ] **Step 5: Wire the subcommand**

In `main.rs`: add `mod lessons;`, the variant

```rust
    /// Lessons matching tags (port of lessons-for.sh; `--json` for Spec JSON v1).
    #[command(disable_help_flag = true)]
    LessonsFor {
        /// Tags, `--format <markdown|inject|json>`, `--tag <t>`, `--json`.
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
```

and the arm `Command::LessonsFor { args } => lessons::run(&found, &args),`.

- [ ] **Step 6: Run the tests to verify they pass**

Run: `cargo test --locked -p vibe-spec 2>&1 | grep -E 'test result|FAILED|panicked'`
Expected: every `test result: ok`.

If `legacy_formats_match_the_oracle` fails only on `--bogus`, clap is consuming the leading hyphen. Keep `allow_hyphen_values` and add `num_args = 0..`, then rerun. Never weaken the test.

- [ ] **Step 7: Corpus cases, pins, candidate**

Append to `cases()` in `corpus_spec.rs`:

```rust
        CaseDef {
            class: "lessons",
            name: "no-newline",
            cwd: "",
            runs: &[("lessons-for", &[&["bash"], &["--format", "json", "bash"], &["spec"]])],
            files: vec![(
                ".spec/lessons.md".into(),
                format!(
                    "{LESSONS_HEAD}{}### Last lesson\n**Rule:** No newline at the end.\n**Tags:** spec, bash",
                    lesson("First", "Has a newline.", "bash")
                ),
            )],
        },
        CaseDef {
            class: "lessons",
            name: "edge-args",
            cwd: "",
            runs: &[(
                "lessons-for",
                &[
                    &["--format"],
                    &["--format", "xml", "bash"],
                    &["inject", "bash"],
                    &["b.sh"],
                    &[""],
                    &["BASH"],
                ],
            )],
            files: vec![(".spec/lessons.md".into(), tagged.clone())],
        },
```

(`tagged` already exists in `cases()`. Change its first use to `tagged.clone()` if the borrow checker asks.)

In `tests/parity/spec.json`:
- set `lessons-for`'s `"candidate"` to `["{candidate}", "lessons-for"]`;
- set the floor `"lessons"` to `4`;
- add to `"divergences"`:

```json
    { "command": "lessons-for", "case": "lessons/quotes", "args": ["--format", "json", "escape"], "field": "stdout", "reason": "the oracle prints quotes and backslashes raw (invalid JSON); the port escapes them" },
    { "command": "lessons-for", "case": "lessons/edge-args", "args": ["b.sh"], "field": "stdout", "reason": "the oracle's grep -qi treats the tag as a regex (b.sh matches bash); the port matches literally" }
```

Regenerate the corpus (expected `15 cases written`).

- [ ] **Step 8: Live lines**

In `tests/parity/run.sh`, after `live list-specs list-specs.sh` add:

```bash
live lessons-for lessons-for.sh spec
live lessons-for lessons-for.sh --format inject bash testing
```

(Live never compares `--format json`: this repo's lessons hold quotes, so that output diverges by design, R5.)

- [ ] **Step 9: Run parity**

Run: `bash tests/parity/run.sh`
Expected:
- candidate: exactly two `PIN` lines, `lessons-for lessons/edge-args [b.sh]` and `lessons-for lessons/quotes [--format json escape]`;
- `0 failed, 0 error(s)`, `5 skipped`;
- three `PASS  live` lines, then `(3) green`.

- [ ] **Step 10: Commit**

```bash
git add crates tests/parity
git commit -m "feat(spec): port lessons-for with spec json"
```

### Task 5: Port `scan-merges`

**Files:**
- Create: `crates/vibe-spec/src/scan_merges.rs`
- Modify: `crates/vibe-spec/src/main.rs` (`mod scan_merges;`, `ScanMerges` variant, dispatch)
- Create: `crates/vibe-spec/tests/scan_merges.rs`
- Modify: `crates/vibe-parity/src/corpus_spec.rs` (cases `features/table-preview`, `features/long-preview`, `features/quoted-preview`, `features/no-trailing-newline`)
- Regenerate: `tests/parity/corpus/spec/`
- Modify: `tests/parity/spec.json`, `tests/parity/run.sh`

**Interfaces:**
- Consumes: `root::{Root, Miss}`; `text::{read_lines, json_str, sub_dirs, name_of}`; `cat!`.
- Produces: `scan_merges::run(found: &Result<Root, Miss>, cwd: &Path, args: &[String]) -> (u8, Vec<u8>)`.

- [ ] **Step 1: Write the failing tests**

Create `crates/vibe-spec/tests/scan_merges.rs`:

```rust
#![cfg(unix)]
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
        &format!("{HEAD}<!-- merge -->\n\n| Key | Value |\n<!-- /merge -->\n<!-- /merge -->\n<!-- merge -->\nx\n<!-- merge -->\nnested\n<!-- /merge -->\n"),
    );
    write(
        &p,
        ".spec/features/billing-and-invoicing-long-name/tech.md",
        &format!("{HEAD}<!-- merge -->\n{}\n<!-- /merge -->\n", "y".repeat(80)),
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
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test --locked -p vibe-spec --test scan_merges 2>&1 | tail -5`
Expected: FAIL — unrecognized subcommand `scan-merges`.

- [ ] **Step 3: Write `scan_merges.rs`**

Create `crates/vibe-spec/src/scan_merges.rs`:

```rust
//! `vibe-spec scan-merges` — port of `spec/scripts/scan-merges.sh`.
//!
//! Byte-identical, including its `while read` loop, which drops a final
//! line without a newline, and its 60-byte preview cut. Deliberate
//! divergence (pinned): `--format json` escapes its strings.
use std::path::{Path, PathBuf};

use vibe_core::exit;

use crate::root::{Miss, Root};
use crate::text::{json_str, name_of, read_lines, sub_dirs};

struct Block {
    feature: String,
    file: String,
    start: usize,
    end: usize,
    preview: Vec<u8>,
}

/// Runs the command.
pub fn run(found: &Result<Root, Miss>, cwd: &Path, args: &[String]) -> (u8, Vec<u8>) {
    let mut feature: Option<String> = None;
    let mut format = String::from("table");
    let mut i = 0;
    while let Some(arg) = args.get(i) {
        let a = arg.as_str();
        if a == "--format" {
            match args.get(i + 1).filter(|v| !v.is_empty()) {
                Some(v) => {
                    format = v.clone();
                    i += 2;
                    continue;
                }
                None => {
                    eprintln!("scan-merges: --format requires table|json|plain");
                    return (exit::FINDINGS, Vec::new());
                }
            }
        } else if let Some(v) = a.strip_prefix("--format=") {
            format = v.to_string();
        } else if a.starts_with('-') {
            eprintln!("Unknown option: {a}");
            return (exit::FINDINGS, Vec::new());
        } else {
            feature = Some(a.to_string());
        }
        i += 1;
    }
    // The oracle's paths are relative to the project ("$SPEC_DIR" = .spec);
    // without a root it looks for .spec/ under the cwd, which cannot exist.
    let (dir, display): (PathBuf, String) = match found {
        Ok(root) => (root.dir.clone(), root.display()),
        Err(Miss::NotFound) => (cwd.join(".spec"), ".spec".to_string()),
        Err(miss) => {
            eprintln!("{}", miss.message());
            return (miss.code(), Vec::new());
        }
    };
    let targets: Vec<(String, PathBuf)> = match &feature {
        Some(f) => vec![(f.clone(), dir.join("features").join(f).join("tech.md"))],
        None => sub_dirs(&dir.join("features"))
            .into_iter()
            .map(|d| (name_of(&d), d.join("tech.md")))
            .filter(|(_, p)| p.is_file())
            .collect(),
    };
    if targets.is_empty() {
        eprintln!("No feature tech.md files found.");
        return (exit::OK, Vec::new());
    }
    let mut unclosed = false;
    let mut blocks: Vec<Block> = Vec::new();
    for (feat, path) in &targets {
        if !path.is_file() {
            continue;
        }
        let file = format!("{display}/features/{feat}/tech.md");
        let text = std::fs::read(path).unwrap_or_default();
        let mut open: Option<(usize, Vec<u8>)> = None;
        for (n, line) in read_lines(&text).into_iter().enumerate() {
            let lineno = n + 1;
            if line == b"<!-- merge -->" {
                if let Some((start, _)) = &open {
                    eprintln!("ERROR: unclosed <!-- merge --> at line {start} in {file} (nested open at {lineno})");
                    unclosed = true;
                    continue;
                }
                open = Some((lineno, Vec::new()));
            } else if line == b"<!-- /merge -->" {
                match open.take() {
                    Some((start, preview)) => blocks.push(Block {
                        feature: feat.clone(),
                        file: file.clone(),
                        start,
                        end: lineno,
                        preview,
                    }),
                    None => {
                        eprintln!("ERROR: <!-- /merge --> without opener at line {lineno} in {file}");
                        unclosed = true;
                    }
                }
            } else if let Some((_, preview)) = open.as_mut() {
                if preview.is_empty() && !line.is_empty() {
                    *preview = line.get(..60).unwrap_or(line).to_vec();
                }
            }
        }
        if let Some((start, _)) = open {
            eprintln!("ERROR: unclosed <!-- merge --> block starting at line {start} in {file}");
            unclosed = true;
        }
    }
    if blocks.is_empty() && !unclosed {
        return (exit::OK, b"No merge blocks found.\n".to_vec());
    }
    let mut out = Vec::new();
    match format.as_str() {
        "json" => {
            out.push(b'[');
            for (n, b) in blocks.iter().enumerate() {
                if n > 0 {
                    out.push(b',');
                }
                out.extend(cat!(
                    "{\"feature\":",
                    json_str(b.feature.as_bytes()),
                    ",\"file\":",
                    json_str(b.file.as_bytes()),
                    format!(",\"start\":{},\"end\":{},\"preview\":", b.start, b.end),
                    json_str(&b.preview),
                    "}"
                ));
            }
            out.extend_from_slice(b"]\n");
        }
        "plain" => {
            for b in &blocks {
                out.extend(cat!(format!("{}:{}-{}: ", b.file, b.start, b.end), b.preview, "\n"));
            }
        }
        _ => {
            row(&mut out, b"feature", b"file", b"lines", b"preview");
            row(&mut out, b"-------", b"----", b"-----", b"-------");
            for b in &blocks {
                let lines = format!("{}-{}", b.start, b.end);
                row(&mut out, b.feature.as_bytes(), b.file.as_bytes(), lines.as_bytes(), &b.preview);
            }
        }
    }
    (if unclosed { exit::FINDINGS } else { exit::OK }, out)
}

/// `printf '%-25s %-45s %-12s %s\n'`: pads by bytes, never truncates.
fn row(out: &mut Vec<u8>, a: &[u8], b: &[u8], c: &[u8], d: &[u8]) {
    for (cell, width) in [(a, 25), (b, 45), (c, 12)] {
        out.extend_from_slice(cell);
        out.resize(out.len() + width.saturating_sub(cell.len()), b' ');
        out.push(b' ');
    }
    out.extend_from_slice(d);
    out.push(b'\n');
}
```

- [ ] **Step 4: Wire the subcommand**

In `main.rs`: add `mod scan_merges;`, the variant

```rust
    /// Report `<!-- merge -->` blocks in feature tech docs (port of scan-merges.sh).
    #[command(disable_help_flag = true)]
    ScanMerges {
        /// `[<feature>] [--format table|json|plain]`.
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
```

and the arm `Command::ScanMerges { args } => scan_merges::run(&found, &cwd, &args),`.

- [ ] **Step 5: Run the tests to verify they pass**

Run: `cargo test --locked -p vibe-spec 2>&1 | grep -E 'test result|FAILED|panicked'`
Expected: every `test result: ok`.

- [ ] **Step 6: Corpus cases, pins, candidate**

In `corpus_spec.rs`, add these constants after `FEATURE_TECH_UNCLOSED`:

```rust
const TECH_HEAD: &str = "---\ntype: feature-tech\nupdated: 2026-01-15\n---\n\n# auth — Tech\n\n";
```

and append to `cases()`:

```rust
        CaseDef {
            class: "features",
            name: "table-preview",
            cwd: "",
            runs: &[("scan-merges", &[&[], &["--format", "json"], &["--format", "plain"]])],
            files: feature(
                &format!("{TECH_HEAD}<!-- merge -->\n| Key | Value |\n|---|---|\n<!-- /merge -->\n"),
                &[],
            ),
        },
        CaseDef {
            class: "features",
            name: "long-preview",
            cwd: "",
            runs: &[("scan-merges", &[&[], &["--format", "json"], &["--format", "plain"]])],
            files: feature(
                &format!("{TECH_HEAD}<!-- merge -->\n{}é and more text after it\n<!-- /merge -->\n", "a".repeat(59)),
                &[],
            ),
        },
        CaseDef {
            class: "features",
            name: "quoted-preview",
            cwd: "",
            runs: &[("scan-merges", &[&[], &["--format", "json"]])],
            files: feature(
                &format!("{TECH_HEAD}<!-- merge -->\nUse \"signed\" cookies, not C:\\tokens\n<!-- /merge -->\n"),
                &[],
            ),
        },
        CaseDef {
            class: "features",
            name: "no-trailing-newline",
            cwd: "",
            runs: &[("scan-merges", &[&[], &["--format", "json"]])],
            files: feature(&format!("{TECH_HEAD}<!-- merge -->\nLast block.\n<!-- /merge -->"), &[]),
        },
```

In `tests/parity/spec.json`:
- set `scan-merges`'s `"candidate"` to `["{candidate}", "scan-merges"]`;
- set the floor `"features"` to `7`;
- add to `"divergences"`:

```json
    { "command": "scan-merges", "case": "features/quoted-preview", "args": ["--format", "json"], "field": "stdout", "reason": "the oracle prints quotes and backslashes raw (invalid JSON); the port escapes them" },
    { "command": "scan-merges", "case": "features/long-preview", "args": ["--format", "json"], "field": "stdout", "reason": "the oracle cuts the preview inside a UTF-8 sequence and prints the raw byte (invalid JSON); the port emits valid JSON with U+FFFD" }
```

Regenerate the corpus (expected `19 cases written`).

- [ ] **Step 7: Live line**

In `tests/parity/run.sh`, add `live scan-merges scan-merges.sh` after the lessons lines.

- [ ] **Step 8: Run parity**

Run: `bash tests/parity/run.sh`
Expected:
- candidate: four `PIN` lines (two lessons-for, two scan-merges);
- the `table-preview`, `long-preview` and `no-trailing-newline` runs without `--format json` all PASS;
- `0 failed, 0 error(s)`, `4 skipped`;
- four `PASS  live` lines.

- [ ] **Step 9: Commit**

```bash
git add crates tests/parity
git commit -m "feat(spec): port scan-merges"
```

### Task 6: Port `validate` — driver and structural checks

**Files:**
- Create: `crates/vibe-spec/src/pattern.rs`
- Create: `crates/vibe-spec/src/validate.rs`
- Modify: `crates/vibe-spec/src/main.rs` (`mod pattern; mod validate;`, `Validate` variant, dispatch)
- Create: `crates/vibe-spec/tests/validate.rs`

**Interfaces:**
- Consumes: `root::{Root, Miss}`; `text::*`; `cat!`.
- Produces (Task 7 builds on these):
  - `validate::Report { out: Vec<u8>, errors: u32, warnings: u32 }`, with methods `red`, `yellow`, `yellow_uncounted`, `green`, `info` and `line`, each taking `&[u8]`.
  - `validate::run(found: &Result<Root, Miss>, cwd: &Path, args: &[String]) -> (u8, Vec<u8>)`.
  - Private functions `root_file`, `children`, `feature`, `feature_doc`, `claude_md`, `sf13`, `sf15`, `sf16` and `lint_design`. Task 7 inserts calls into `root_file` and `feature`, and an `sf14` call into `run`.
  - `pattern::{md_links, md_link_target, paren_links}`.

This task ports every check except the content checks: SF3 tokens, SF8 scope, SF9 feature frontmatter, SF10 requirements, SF11 plan structure and milestones, SF12 traceability, I1 cross-feature dependencies and SF14 scope conflicts. Task 7 adds those, then switches `validate` to candidate mode. The differential tests below use trees that trigger none of the content checks:
- feature docs carry `feature:`, `parent:` and `sibling:`;
- products have a full Scope section;
- no feature has a `plan.md`;
- no doc has `### Requirement:`, a `## M<n>` milestone or a design-token group.

- [ ] **Step 1: Write the failing tests**

Create `crates/vibe-spec/tests/validate.rs`:

```rust
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
    write(&p, ".spec/product-ux.md", "---\ntype: product-topic\nparent: product.md\nscope: ux\ncovers: flows\nupdated: 2026-01-15\n---\n\n[up](product.md)\n");
    write(&p, ".spec/lessons.md", "# Lessons\n\n### One\n**Tags:** a\n");
    assert_parity(&p, "validate", "validate.sh", &[]);
    assert_parity(&p.join(".spec"), "validate", "validate.sh", &[]);
}

#[test]
fn root_layer_errors_match() {
    let p = project("val-root");
    write(&p, ".spec/product.md", "# no frontmatter\n");
    write(&p, ".spec/tech.md", "---\ntype: entrypoint\nchildren:\n  - tech-api.md\n  - tech-{topic}.md\n  - tech-gone.md\n-\tbare\nnext: x\n---\n\n[a](b(c).md) [t]({x}.md) [m](missing.md) [s](features/x/)\n");
    write(&p, ".spec/plan.md", "---\ntype: entrypoint\nupdated: 2026-01-15\n---\n");
    write(&p, ".spec/tech-api.md", "---\ntype: tech-topic\nupdated: 2026-01-15\n---\n");
    write(&p, ".spec/tech-Bad_Topic.md", "---\ntype: tech-topic\nparent: tech.md\nscope: s\ncovers: c\nupdated: 2026-01-15\n---\n");
    write(&p, ".spec/notes.md", "---\ntype: note\nupdated: 2026-01-15\n---\n");
    write(&p, ".spec/design.md", "---\ntype: entrypoint\nchildren:\nupdated: 2026-01-15\n---\n");
    write(&p, "CLAUDE.md", "[ok](.spec/plan.md) [bad](.spec/gone.md) [other](docs/x.md)\n");
    assert_parity(&p, "validate", "validate.sh", &[]);
}

#[test]
fn features_sf13_sf15_and_sf16_match() {
    let p = project("val-features");
    write(&p, ".spec/product.md", &format!("{ENTRY}\n[f](features/auth/) [g](features/ghost/) [a](archive/old/) [t](features/{{x}}/)\n"));
    write(&p, ".spec/product-ux.md", "---\ntype: product-topic\nparent: product.md\nscope: ux\ncovers: flows\nupdated: 2026-01-15\n---\n");
    write(&p, ".spec/features/auth/product.md", FEATURE);
    write(&p, ".spec/features/auth/tech.md", FEATURE_TECH);
    write(&p, ".spec/features/auth/design.md", "no frontmatter\n");
    write(&p, ".spec/features/billing/product.md", "---\ntype: product\nfeature: billing\nparent: ../../product.md\nsibling: tech.md\nupdated: 2026-01-15\n---\n\n## Scope\n\nOwns invoices; does not own auth.\n");
    write(&p, ".spec/plan.md", &format!("---\ntype: entrypoint\nchildren:\nupdated: 2026-01-15\n---\n{}", "line\n".repeat(301)));
    write(&p, ".spec/lessons.md", "# Lessons\n\n### Escape \\t in titles\n**Rule:** x\n\n### Tagged\n**Tags:** a\n\n### Last\n**Tags:** no newline");
    assert_parity(&p, "validate", "validate.sh", &[]);
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
```

The `features_sf13_sf15_and_sf16_match` test writes a 306-line `plan.md` to trigger SF15. On macOS the oracle's BSD `wc -l` pads the count (R2). Wrap that one assertion:

```rust
    if cfg!(target_os = "macos") {
        // BSD wc pads the SF15 count (P3a-R2); compare everything else.
        let want = common::oracle(&p, "validate.sh", &[]).text();
        let got = common::spec(&p, &["validate"], &[]).text();
        let squash = |s: &str| s.split_whitespace().collect::<Vec<_>>().join(" ");
        assert_eq!(squash(&got), squash(&want));
    } else {
        assert_parity(&p, "validate", "validate.sh", &[]);
    }
```

(Use this block in place of the plain `assert_parity` in that test.)

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test --locked -p vibe-spec --test validate 2>&1 | tail -5`
Expected: FAIL — unrecognized subcommand `validate`.

- [ ] **Step 3: Write `pattern.rs`**

Create `crates/vibe-spec/src/pattern.rs`:

```rust
//! Hand-written versions of the oracles' `grep -oE`, `sed` and `awk`
//! patterns (no `regex` crate). Each function names the pattern it ports.
//! `grep -o` semantics: leftmost match, scanning resumes after it.

/// `grep -oE '\[[^]]*\]\([^)]+\)'`: markdown links in one line.
pub fn md_links(line: &[u8]) -> Vec<&[u8]> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < line.len() {
        if line[i] == b'[' {
            if let Some(end) = link_end(line, i) {
                out.push(&line[i..=end]);
                i = end + 1;
                continue;
            }
        }
        i += 1;
    }
    out
}

/// The index of the `)` closing a link that starts at `line[start] == '['`.
fn link_end(line: &[u8], start: usize) -> Option<usize> {
    let close = start + 1 + line.get(start + 1..)?.iter().position(|&c| c == b']')?;
    if line.get(close + 1) != Some(&b'(') {
        return None;
    }
    let body = close + 2;
    let paren = body + line.get(body..)?.iter().position(|&c| c == b')')?;
    (paren > body).then_some(paren)
}

/// `sed 's/.*(\(.*\))/\1/' | sed 's/#.*//'` on one link: the text after
/// the last `(` up to the final `)`, cut at the first `#`.
pub fn md_link_target(link: &[u8]) -> Vec<u8> {
    let inner = match link.iter().rposition(|&c| c == b'(') {
        Some(open) if link.last() == Some(&b')') && open + 1 < link.len() => &link[open + 1..link.len() - 1],
        _ => link,
    };
    match inner.iter().position(|&c| c == b'#') {
        Some(hash) => inner[..hash].to_vec(),
        None => inner.to_vec(),
    }
}

/// `grep -oE '\(features/[^)]+\)|\(archive/[^)]+\)' | sed 's/^(//;s/)$//'`.
pub fn paren_links(line: &[u8]) -> Vec<&[u8]> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < line.len() {
        let rest = &line[i..];
        let kind = [&b"(features/"[..], b"(archive/"]
            .into_iter()
            .find(|k| rest.starts_with(k));
        if let Some(kind) = kind {
            let body = i + kind.len();
            if let Some(len) = line[body..].iter().position(|&c| c == b')') {
                if len > 0 {
                    out.push(&line[i + 1..body + len]);
                    i = body + len + 1;
                    continue;
                }
            }
        }
        i += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn md_links_match_grep() {
        assert_eq!(md_links(b"[x](a) [y](b#c)"), vec![&b"[x](a)"[..], b"[y](b#c)"]);
        assert_eq!(md_links(b"[[a](b)"), vec![&b"[[a](b)"[..]]);
        assert!(md_links(b"[a]() [b] (c)").is_empty());
        assert_eq!(md_links(b"[a](b(c).md)"), vec![&b"[a](b(c)"[..]]);
    }

    #[test]
    fn md_link_target_matches_sed() {
        assert_eq!(md_link_target(b"[a](b(c).md)"), b"c).md");
        assert_eq!(md_link_target(b"[a](tech.md#top)"), b"tech.md");
        assert_eq!(md_link_target(b"[a](#top)"), b"");
    }

    #[test]
    fn paren_links_match_grep() {
        assert_eq!(
            paren_links(b"(features/a/) (archive/b) (features/) (other/c)"),
            vec![&b"features/a/"[..], b"archive/b"]
        );
    }
}
```

`[a](b(c).md)` shows how the two patterns combine:
- `grep -oE '\[[^]]*\]\([^)]+\)'` stops at the first `)`, so the match is `[a](b(c)`.
- `md_link_target` then returns `c` for that match.

The unit test above shows the string-level rule. The `root_layer_errors_match` differential test pins what the oracle prints end to end.

- [ ] **Step 4: Write `validate.rs`**

Create `crates/vibe-spec/src/validate.rs`:

```rust
//! `vibe-spec validate` — port of `spec/scripts/validate.sh`.
//!
//! F16 ports it byte for byte: ANSI colours, `echo -e` escapes, the error
//! and warning counters (including the one warning the oracle prints from a
//! pipeline subshell and so never counts), and the exit code. F17 changes
//! behaviour. Deliberate divergence (pinned): with no spec root, the error
//! names `vibe-spec init`, not the oracle's `setup.sh` path.
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use vibe_core::exit;

use crate::pattern::{md_link_target, md_links, paren_links};
use crate::root::{Miss, Root};
use crate::text::{echo_e, first_line, has_line_starting, is_space, join_raw, lines, md_files, name_of, read_lines, strip_prefix_or, sub_dirs, trim_start_space};

const ROOT_NAMES: [&str; 5] = ["product.md", "tech.md", "design.md", "plan.md", "lessons.md"];
const ENTRYPOINTS: [&str; 4] = ["product.md", "tech.md", "design.md", "plan.md"];

/// validate's stdout and its counters.
#[derive(Default)]
pub struct Report {
    /// Everything printed so far.
    pub out: Vec<u8>,
    /// `ERRORS`.
    pub errors: u32,
    /// `WARNINGS`.
    pub warnings: u32,
}

impl Report {
    fn colour(&mut self, code: &str, tag: &str, msg: &[u8]) {
        self.out.extend(echo_e(&cat!("\\033[", code, "m  ", tag, msg, "\\033[0m")));
    }

    /// `red`: an error.
    pub fn red(&mut self, msg: &[u8]) {
        self.colour("31", "ERROR: ", msg);
        self.errors += 1;
    }

    /// `yellow`: a warning.
    pub fn yellow(&mut self, msg: &[u8]) {
        self.colour("33", "WARN:  ", msg);
        self.warnings += 1;
    }

    /// `yellow` called inside `cmd | while read`: printed, never counted,
    /// because the increment happens in a subshell.
    pub fn yellow_uncounted(&mut self, msg: &[u8]) {
        self.colour("33", "WARN:  ", msg);
    }

    /// `green`.
    pub fn green(&mut self, msg: &[u8]) {
        self.colour("32", "OK:    ", msg);
    }

    /// `info`.
    pub fn info(&mut self, msg: &[u8]) {
        self.colour("36", "INFO:  ", msg);
    }

    /// Plain `echo "$msg"`.
    pub fn line(&mut self, msg: &[u8]) {
        self.out.extend_from_slice(msg);
        self.out.push(b'\n');
    }
}

/// Runs the command.
pub fn run(found: &Result<Root, Miss>, cwd: &Path, args: &[String]) -> (u8, Vec<u8>) {
    if let Some(arg) = args.first() {
        eprintln!("vibe-spec validate: unexpected argument {arg} (F17 adds validate's flags)");
        return (exit::USAGE, Vec::new());
    }
    let (dir, project, display): (PathBuf, PathBuf, String) = match found {
        Ok(root) => (root.dir.clone(), root.project.clone(), root.display()),
        Err(Miss::NotFound) => (cwd.join(".spec"), cwd.to_path_buf(), ".spec".to_string()),
        Err(miss) => {
            eprintln!("{}", miss.message());
            return (miss.code(), Vec::new());
        }
    };
    let mut r = Report::default();
    r.line(format!("Validating {display}/...").as_bytes());
    r.line(b"");
    if !dir.is_dir() {
        r.red(b"No spec root found. Run setup first: vibe-spec init");
        return (exit::FINDINGS, r.out);
    }
    let specs = md_files(&dir);
    if specs.is_empty() {
        r.red(format!("No spec files found in {display}/").as_bytes());
        return (exit::FINDINGS, r.out);
    }
    let ctx = Ctx { dir: &dir, project: &project, display: &display };
    for file in &specs {
        root_file(&mut r, &ctx, file);
    }
    for name in ENTRYPOINTS {
        children(&mut r, &dir, name);
    }
    let features = dir.join("features");
    if features.is_dir() {
        for fdir in sub_dirs(&features) {
            feature(&mut r, &ctx, &fdir);
        }
    }
    claude_md(&mut r, &project, &display);
    sf13(&mut r, &dir, &display);
    sf15(&mut r, &dir, &display);
    sf16(&mut r, &dir);
    let (errors, warnings) = (r.errors, r.warnings);
    r.line(b"========================");
    r.line(format!("Errors:   {errors}").as_bytes());
    r.line(format!("Warnings: {warnings}").as_bytes());
    (if errors > 0 { exit::FINDINGS } else { exit::OK }, r.out)
}

/// Where the spec lives.
struct Ctx<'a> {
    /// The spec root (absolute).
    dir: &'a Path,
    /// The project directory: the oracle's cwd after it `cd`s to the root's parent.
    project: &'a Path,
    /// The root relative to the project, as the oracle spells `$SPEC_DIR` (`.spec`).
    display: &'a str,
}

fn read(path: &Path) -> Vec<u8> {
    std::fs::read(path).unwrap_or_default()
}

/// The root-layer loop body for one `$SPEC_DIR/*.md` file.
fn root_file(r: &mut Report, ctx: &Ctx, file: &Path) {
    let name = name_of(file);
    let n = name.as_bytes();
    r.line(&cat!("--- ", n, " ---"));
    if name == "lessons.md" {
        r.green(&cat!(n, ": checked (lessons file, no frontmatter required)"));
        r.line(b"");
        return;
    }
    let text = read(file);
    if first_line(&text) != b"---" {
        r.red(&cat!(n, ": missing YAML frontmatter"));
        return;
    }
    if !has_line_starting(&text, b"type:") {
        r.red(&cat!(n, ": missing 'type:' in frontmatter"));
    }
    if !has_line_starting(&text, b"updated:") {
        r.red(&cat!(n, ": missing 'updated:' in frontmatter"));
    }
    if has_line_starting(&text, b"type: entrypoint") && !has_line_starting(&text, b"children:") {
        if name == "plan.md" {
            r.yellow(&cat!(n, ": entrypoint has no 'children:' list (add sub-plans if needed)"));
        } else {
            r.red(&cat!(n, ": entrypoint missing 'children:' list"));
        }
    }
    let branch = [&b"type: product-topic"[..], b"type: tech-topic", b"type: plan-topic"];
    if branch.iter().any(|p| has_line_starting(&text, p)) {
        if !has_line_starting(&text, b"parent:") {
            r.red(&cat!(n, ": branch doc missing 'parent:' field"));
        }
        if !has_line_starting(&text, b"scope:") {
            r.yellow(&cat!(n, ": branch doc missing 'scope:' field"));
        }
        if !has_line_starting(&text, b"covers:") {
            r.yellow(&cat!(n, ": branch doc missing 'covers:' field"));
        }
    }
    if !ROOT_NAMES.contains(&name.as_str()) {
        if !["product-", "tech-", "plan-"].iter().any(|p| name.starts_with(p)) {
            r.red(&cat!(n, ": must start with 'product-', 'tech-', or 'plan-' (e.g., product-design.md, tech-api.md, plan-editor.md)"));
        }
        if let Some(topic) = topic_of(&name) {
            if !topic_ok(topic.as_bytes()) {
                r.red(&cat!(n, ": topic must be lowercase with hyphens only, no leading/trailing hyphens (found: '", topic, "')"));
            }
        }
    }
    if name == "design.md" {
        lint_design(r, ctx.project, &format!("{}/design.md", ctx.display), n);
    }
    for line in lines(&text) {
        for link in md_links(line) {
            let target = md_link_target(link);
            if target.is_empty() || !target.ends_with(b".md") || target.starts_with(b"http") || target.contains(&b'{') {
                continue;
            }
            let parent = file.parent().unwrap_or(ctx.dir);
            if !join_raw(ctx.dir, &target).is_file() && !join_raw(parent, &target).is_file() {
                r.red(&cat!(n, ": broken link to '", target, "'"));
            }
        }
    }
    r.green(&cat!(n, ": checked"));
    r.line(b"");
}

/// `^(product|tech|plan)-(.+)\.md$` → the topic.
fn topic_of(name: &str) -> Option<&str> {
    ["product-", "tech-", "plan-"].iter().find_map(|p| {
        let rest = name.strip_prefix(p)?;
        (rest.len() >= 4 && rest.ends_with(".md")).then(|| &rest[..rest.len() - 3])
    })
}

/// `^[a-z0-9]([a-z0-9-]*[a-z0-9])?$`.
fn topic_ok(t: &[u8]) -> bool {
    let edge = |c: &u8| c.is_ascii_lowercase() || c.is_ascii_digit();
    t.first().is_some_and(edge)
        && t.last().is_some_and(edge)
        && t.iter().all(|c| edge(c) || *c == b'-')
}

/// "Validate entrypoint children exist" for one entrypoint.
fn children(r: &mut Report, dir: &Path, name: &str) {
    let path = dir.join(name);
    if !path.is_file() {
        return;
    }
    let text = read(&path);
    let mut in_children = false;
    for line in read_lines(&text) {
        if line == b"children:" {
            in_children = true;
            continue;
        }
        if !in_children {
            continue;
        }
        match list_item(line) {
            Some(child) => {
                if !child.contains(&b'{') && !join_raw(dir, child).is_file() {
                    r.red(&cat!(name, ": child '", child, "' listed in frontmatter but file doesn't exist"));
                }
            }
            None => in_children = false,
        }
    }
}

/// `^[[:space:]]*-[[:space:]]+(.*)` → the text after the dash and its spaces.
fn list_item(line: &[u8]) -> Option<&[u8]> {
    let rest = trim_start_space(line).strip_prefix(b"-")?;
    let after = trim_start_space(rest);
    (after.len() < rest.len()).then_some(after)
}

/// "Validate feature folders" for one feature directory.
fn feature(r: &mut Report, ctx: &Ctx, fdir: &Path) {
    let fname = name_of(fdir);
    let label = format!("features/{fname}");
    r.line(format!("--- {label}/ ---").as_bytes());
    for required in ["product.md", "tech.md"] {
        let file = fdir.join(required);
        if !file.is_file() {
            r.red(format!("{label}/: missing required '{required}'").as_bytes());
            continue;
        }
        let doc = format!("{label}/{required}");
        let text = read(&file);
        if !feature_doc(r, &text, &doc, required) {
            continue;
        }
        for line in lines(&text) {
            for link in md_links(line) {
                let target = md_link_target(link);
                if target.is_empty() || !target.ends_with(b".md") || target.starts_with(b"http") {
                    continue;
                }
                if (target == b"design.md" || target == b"plan.md") && !join_raw(fdir, &target).is_file() {
                    continue;
                }
                if !resolve_in(fdir, &target).is_file() {
                    r.red(&cat!(doc, ": broken link to '", target, "'"));
                }
            }
        }
    }
    for optional in ["design.md", "plan.md"] {
        let file = fdir.join(optional);
        if !file.is_file() {
            continue;
        }
        let doc = format!("{label}/{optional}");
        let text = read(&file);
        if !feature_doc(r, &text, &doc, optional) {
            continue;
        }
        if optional == "design.md" {
            lint_design(r, ctx.project, &format!("{}/{label}/design.md", ctx.display), doc.as_bytes());
        }
    }
    r.green(format!("{label}/: checked").as_bytes());
    r.line(b"");
}

/// Frontmatter, `type:`/`updated:` and expected-type checks of one feature
/// doc. False when it has no frontmatter (the oracle `continue`s).
fn feature_doc(r: &mut Report, text: &[u8], doc: &str, file: &str) -> bool {
    if first_line(text) != b"---" {
        r.red(format!("{doc}: missing YAML frontmatter").as_bytes());
        return false;
    }
    if !has_line_starting(text, b"type:") {
        r.red(format!("{doc}: missing 'type:' in frontmatter").as_bytes());
    }
    if !has_line_starting(text, b"updated:") {
        r.red(format!("{doc}: missing 'updated:' in frontmatter").as_bytes());
    }
    let expected = format!("feature-{}", file.trim_end_matches(".md"));
    if !has_line_starting(text, format!("type: {expected}").as_bytes()) {
        r.yellow(format!("{doc}: expected 'type: {expected}'").as_bytes());
    }
    true
}

/// `cd "$feature_dir" && [[ -f "$target" ]]`: absolute targets stay absolute.
fn resolve_in(dir: &Path, target: &[u8]) -> PathBuf {
    if target.first() == Some(&b'/') {
        join_raw(Path::new(""), target.get(1..).unwrap_or_default())
    } else {
        join_raw(dir, target)
    }
}

/// "Validate CLAUDE.md links": only links into the spec root are checked.
fn claude_md(r: &mut Report, project: &Path, display: &str) {
    let path = project.join("CLAUDE.md");
    if !path.is_file() {
        return;
    }
    r.line(b"--- CLAUDE.md ---");
    let prefix = format!("{display}/");
    let text = read(&path);
    for line in lines(&text) {
        for link in md_links(line) {
            let target = md_link_target(link);
            let into_spec = target.starts_with(prefix.as_bytes())
                && target.ends_with(b".md")
                && target.len() >= prefix.len() + 3;
            if into_spec && !join_raw(project, &target).is_file() {
                r.red(&cat!("CLAUDE.md: broken link to '", target, "'"));
            }
        }
    }
    r.green(b"CLAUDE.md: checked");
    r.line(b"");
}

/// SF13: stale non-`.md` links to `features/` or `archive/` from the root hubs.
fn sf13(r: &mut Report, dir: &Path, display: &str) {
    for name in ENTRYPOINTS {
        let path = dir.join(name);
        if !path.is_file() {
            continue;
        }
        let text = read(&path);
        for line in lines(&text) {
            for target in paren_links(line) {
                if target.ends_with(b".md") || target.contains(&b'{') || join_raw(dir, target).exists() {
                    continue;
                }
                r.yellow(&cat!("SF13: stale link in ", display, "/", name, " → ", target, " (not found)"));
            }
        }
    }
}

/// SF15: root spec length over `$SPEC_ROOT_MAX_LINES` (default 300) newlines.
fn sf15(r: &mut Report, dir: &Path, display: &str) {
    let raw = std::env::var("SPEC_ROOT_MAX_LINES")
        .ok()
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| "300".to_string());
    // bash evaluates a non-numeric value as 0.
    let max: usize = raw.parse().unwrap_or(0);
    for name in ROOT_NAMES {
        let path = dir.join(name);
        if !path.is_file() {
            continue;
        }
        let count = read(&path).iter().filter(|&&b| b == b'\n').count();
        if count > max {
            r.yellow(format!("SF15: {display}/{name} is {count} lines (>{raw}); consider splitting into a branch doc (product-{{topic}}.md, tech-{{topic}}.md)").as_bytes());
        }
    }
}

/// SF16: lessons without a `**Tags:**` line. A final line without a
/// newline is not read (the oracle's `while read` drops it).
fn sf16(r: &mut Report, dir: &Path) {
    let path = dir.join("lessons.md");
    if !path.is_file() {
        return;
    }
    let text = read(&path);
    let mut title: Vec<u8> = Vec::new();
    let mut has_tags = false;
    let flag = |r: &mut Report, title: &[u8], has_tags: bool| {
        if !title.is_empty() && !has_tags {
            r.yellow(&cat!("SF16: lesson '", title, "' in lessons.md has no **Tags:** line"));
        }
    };
    for line in read_lines(&text) {
        if line.starts_with(b"###") && line.get(3).is_some_and(|&c| is_space(c)) {
            flag(r, &title, has_tags);
            title = strip_prefix_or(line, b"### ").to_vec();
            has_tags = false;
        } else if line.starts_with(b"**Tags:**") {
            has_tags = true;
        }
    }
    flag(r, &title, has_tags);
}

/// SF4: the optional external design.md linter (`VIBE_DESIGN_LINT=1`),
/// advisory and graceful-degrade, run from the project like the oracle.
fn lint_design(r: &mut Report, project: &Path, rel: &str, label: &[u8]) {
    if std::env::var_os("VIBE_DESIGN_LINT").as_deref() != Some(OsStr::new("1")) {
        return;
    }
    if !on_path("node") || !on_path("npx") {
        r.yellow(&cat!(label, ": design.md lint skipped (node/npx not found)"));
        return;
    }
    let reachable = Command::new("curl")
        .args(["-fsS", "--max-time", "3", "https://registry.npmjs.org/"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|s| s.success());
    if !reachable {
        r.yellow(&cat!(label, ": design.md lint skipped (npm registry unreachable)"));
        return;
    }
    let (ok, mut text) = match Command::new("npx")
        .args(["--yes", "@google/design.md", "lint", rel])
        .current_dir(project)
        .stdin(Stdio::null())
        .output()
    {
        Ok(o) => (o.status.success(), [o.stdout, o.stderr].concat()),
        Err(_) => (false, Vec::new()),
    };
    if ok {
        return;
    }
    r.yellow(&cat!(label, ": design.md lint reported issues (advisory)"));
    while text.last() == Some(&b'\n') {
        text.pop();
    }
    for line in text.split(|&b| b == b'\n') {
        if !line.is_empty() {
            r.yellow(&cat!(label, ":   ", line));
        }
    }
}

/// `command -v <name>`: an executable file in a `PATH` directory.
fn on_path(name: &str) -> bool {
    let Some(path) = std::env::var_os("PATH") else {
        return false;
    };
    std::env::split_paths(&path).any(|d| executable(&d.join(name)))
}

#[cfg(unix)]
fn executable(p: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    p.metadata().is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
}

#[cfg(not(unix))]
fn executable(p: &Path) -> bool {
    p.is_file()
}
```

- [ ] **Step 5: Wire the subcommand**

In `main.rs`: add `mod pattern;` and `mod validate;`, the variant

```rust
    /// Validate the spec tree (port of validate.sh).
    #[command(disable_help_flag = true)]
    Validate {
        /// None yet; F17 adds `--strict`, `--changed`, `--file`, `--format`.
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
```

and the arm `Command::Validate { args } => validate::run(&found, &cwd, &args),`.

- [ ] **Step 6: Add the SF4 lint test**

Append to `crates/vibe-spec/tests/validate.rs`:

```rust
#[test]
fn design_lint_degrades_like_the_oracle() {
    use std::os::unix::fs::PermissionsExt;
    let p = project("val-lint");
    write(&p, ".spec/product.md", ENTRY);
    write(&p, ".spec/product-ux.md", "---\ntype: product-topic\nparent: product.md\nscope: ux\ncovers: flows\nupdated: 2026-01-15\n---\n");
    write(&p, ".spec/design.md", "---\ntype: entrypoint\nchildren:\nupdated: 2026-01-15\n---\n");
    let bin = p.join("fakebin");
    for (name, body) in [
        ("node", "#!/bin/sh\nexit 0\n"),
        ("npx", "#!/bin/sh\necho \"lint: $4 has issues\"\necho 'second\\tline'\nexit 1\n"),
        ("curl", "#!/bin/sh\nexit 0\n"),
    ] {
        write(&p, &format!("fakebin/{name}"), body);
        std::fs::set_permissions(bin.join(name), std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    let path = format!("{}:{}", bin.display(), std::env::var("PATH").unwrap());
    let env = [("VIBE_DESIGN_LINT", "1"), ("PATH", path.as_str())];
    let want = {
        let mut cmd = std::process::Command::new("bash");
        cmd.arg(common::repo().join("oracles/bash/spec/scripts/validate.sh"))
            .current_dir(&p)
            .env_clear()
            .env("HOME", &p)
            .env("LC_ALL", "C")
            .env("TZ", "UTC");
        for (k, v) in env {
            cmd.env(k, v);
        }
        cmd.output().unwrap()
    };
    let got = common::spec(&p, &["validate"], &env);
    assert_eq!(got.text(), String::from_utf8_lossy(&want.stdout));
    assert!(got.text().contains("lint: .spec/design.md has issues"), "{}", got.text());
}
```

- [ ] **Step 7: Run the tests to verify they pass**

```bash
cargo test --locked -p vibe-spec 2>&1 | grep -E 'test result|FAILED|panicked'
cargo clippy --workspace --all-targets --locked -- -D warnings 2>&1 | tail -1
cargo fmt --all --check && echo fmt-ok
```

Expected: every `test result: ok`; clippy `Finished`; `fmt-ok`.

When a differential test fails, its message shows both outputs. Fix the port, never the fixture: the oracle defines the bytes.

- [ ] **Step 8: Commit**

```bash
git add crates/vibe-spec
git commit -m "feat(spec): port validate structural checks"
```

### Task 7: Port `validate` — content checks; candidate mode

**Files:**
- Modify: `crates/vibe-spec/src/pattern.rs` (unit-ID, cross-feature and R-ID matchers)
- Create: `crates/vibe-spec/src/content.rs`
- Modify: `crates/vibe-spec/src/validate.rs` (call sites)
- Modify: `crates/vibe-spec/src/main.rs` (`mod content;`)
- Modify: `crates/vibe-spec/tests/validate.rs`
- Modify: `crates/vibe-parity/src/corpus_spec.rs` (cases `malformed/rich-root`, `malformed/empty-root`, `features/rich-features`, `filled/long-root`)
- Regenerate: `tests/parity/corpus/spec/`
- Modify: `tests/parity/spec.json`, `tests/parity/run.sh`

**Interfaces:**
- Consumes: `validate::Report` and its methods; `text::*`; `pattern::*`.
- Produces:
  - In `pattern`: `unit_ids`, `is_unit_heading`, `is_unit_row`, `cross_feature_deps` and `r_ids`.
  - In `content`: `design_tokens`, `feature_scope`, `feature_frontmatter`, `requirements_scenarios`, `plan_structure`, `root_plan_milestones`, `id_traceability`, `cross_deps` and `sf14`.
  - `validate` in candidate mode.

- [ ] **Step 1: Write the failing tests**

Append to `crates/vibe-spec/tests/validate.rs`:

```rust
const AUTH_PRODUCT: &str = "---\ntype: feature-product\nfeature: auth\nparent: ../../product.md\nsibling: tech.md\nupdated: 2026-01-15\n---\n\n## Scope\n\n| | |\n|---|---|\n| **Owns** | session cookies, login form, ui |\n| **Does not own** | billing |\n\n## Requirements\n\n### Requirement: Sign in (R1)\nUsers MUST sign in.\n#### Scenario: ok\nWorks.\n\n### Requirement: Sign out (R10)\nThey leave.\n\n### Requirement:\nEmpty title.\n\n## Requirements Trace\n\n| R2 | missing |\n";
const AUTH_PLAN: &str = "---\ntype: feature-plan\nfeature: auth\nparent: ../../plan.md\nupdated: 2026-01-15\n---\n\nCovers R10.\n\n### auth/1 — Login\n\n### auth/1 — Login again\n\n### billing/2 — Cross\n\nDepends on billing/3, auth/2 and x_y/4.\n";
const BILLING_PRODUCT: &str = "---\ntype: feature-product\nfeature: billing\nparent: ../../product.md\nsibling: tech.md\nupdated: 2026-01-15\n---\n\n## Scope\n\n| Owns | Does not own |\n|---|---|\n| `Session Cookies` | auth |\n| invoices | |\n";

#[test]
fn content_checks_match() {
    let p = project("val-content");
    write(&p, ".spec/product.md", "---\ntype: entrypoint\nscope: product\nchildren:\nupdated: 2026-01-15\n---\n\n### Requirement: Users sign in\nThey do.\n\n### Requirement: Export SHALL work\n#### Scenario: ok\n");
    write(&p, ".spec/plan.md", "---\ntype: entrypoint\nupdated: 2026-01-15\n---\n\n## M1 — Alpha\n\n### auth/1 — leaked\n\n| auth/2 | row |\n|  AB12  | caps |\n\n## Notes\n\n### auth/3 — fine\n");
    write(&p, ".spec/design.md", "---\ntype: entrypoint\nchildren:\ncolors:\ntypography:\n  body: Inter\nrounded:\n# note\nspacing: 4px\ncomponents:\nupdated: 2026-01-15\n---\n");
    write(&p, ".spec/features/auth/product.md", AUTH_PRODUCT);
    write(&p, ".spec/features/auth/tech.md", "---\ntype: feature-tech\nupdated: 2026-01-15\n---\n");
    write(&p, ".spec/features/auth/plan.md", AUTH_PLAN);
    write(&p, ".spec/features/auth/design.md", "---\ntype: feature-design\nfeature: auth\nparent: x\nupdated: 2026-01-15\ncolors:\n---\n");
    write(&p, ".spec/features/billing/product.md", BILLING_PRODUCT);
    write(&p, ".spec/features/billing/plan.md", "---\ntype: feature-plan\nupdated: 2026-01-15\n---\n\nNo units.\n");
    write(&p, ".spec/features/billing/tech.md", "---\ntype: feature-tech\nfeature: billing\nparent: x\nsibling: y\nupdated: 2026-01-15\n---\n");
    assert_parity(&p, "validate", "validate.sh", &[]);
}

#[test]
fn milestone_warnings_print_but_do_not_count() {
    let p = project("val-subshell");
    write(&p, ".spec/plan.md", "---\ntype: entrypoint\nchildren:\nupdated: 2026-01-15\n---\n\n## M1\n\n### auth/1 — leaked\n");
    let out = common::spec(&p, &["validate"], &[]);
    assert!(out.text().contains("milestone section embeds unit heading"), "{}", out.text());
    assert!(out.text().ends_with("Errors:   0\nWarnings: 0\n"), "{}", out.text());
    assert_parity(&p, "validate", "validate.sh", &[]);
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test --locked -p vibe-spec --test validate 2>&1 | tail -8`
Expected: `content_checks_match` and `milestone_warnings_print_but_do_not_count` FAIL (no content checks yet); the Task 6 tests still pass.

- [ ] **Step 3: Add the matchers to `pattern.rs`**

Append above the `tests` module:

```rust
fn digits_at(s: &[u8], i: usize) -> usize {
    s.get(i..)
        .map(|r| r.iter().take_while(|c| c.is_ascii_digit()).count())
        .unwrap_or(0)
}

/// `[a-z0-9][a-z0-9-]*/[0-9]+` at `s[i..]`: the match length.
fn unit_slash_at(s: &[u8], i: usize) -> Option<usize> {
    let first = *s.get(i)?;
    if !(first.is_ascii_lowercase() || first.is_ascii_digit()) {
        return None;
    }
    let mut j = i + 1;
    while s
        .get(j)
        .is_some_and(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || *c == b'-')
    {
        j += 1;
    }
    if s.get(j) != Some(&b'/') {
        return None;
    }
    let d = digits_at(s, j + 1);
    (d > 0).then_some(j + 1 + d - i)
}

/// `[A-Z]{1,4}[0-9]+` at `s[i..]`. A run of five or more capitals fails at
/// `i`: its shorter prefixes are followed by a capital, not a digit.
fn unit_caps_at(s: &[u8], i: usize) -> Option<usize> {
    let caps = s.get(i..)?.iter().take_while(|c| c.is_ascii_uppercase()).count();
    if caps == 0 || caps > 4 {
        return None;
    }
    let d = digits_at(s, i + caps);
    (d > 0).then_some(caps + d)
}

/// `([a-z0-9][a-z0-9-]*/[0-9]+|[A-Z]{1,4}[0-9]+)` at `s[i..]` (the two
/// branches start with disjoint bytes, so at most one matches).
fn unit_at(s: &[u8], i: usize) -> Option<usize> {
    unit_slash_at(s, i).or_else(|| unit_caps_at(s, i))
}

/// `grep -oE '([a-z0-9][a-z0-9-]*/[0-9]+|[A-Z]{1,4}[0-9]+)'`.
pub fn unit_ids(line: &[u8]) -> Vec<&[u8]> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < line.len() {
        match unit_at(line, i) {
            Some(n) => {
                out.push(&line[i..i + n]);
                i += n;
            }
            None => i += 1,
        }
    }
    out
}

/// `^### ([a-z0-9][a-z0-9-]*/[0-9]+|[A-Z]{1,4}[0-9]+)`.
pub fn is_unit_heading(line: &[u8]) -> bool {
    line.starts_with(b"### ") && unit_at(line, 4).is_some()
}

/// `^\|[[:space:]]*(<unit>)[[:space:]]*\|`.
pub fn is_unit_row(line: &[u8]) -> bool {
    let spaces = |from: usize| {
        line.get(from..)
            .map(|r| r.iter().take_while(|&&c| crate::text::is_space(c)).count())
            .unwrap_or(0)
    };
    if line.first() != Some(&b'|') {
        return false;
    }
    let i = 1 + spaces(1);
    let Some(n) = unit_at(line, i) else {
        return false;
    };
    let j = i + n;
    line.get(j + spaces(j)) == Some(&b'|')
}

fn word(c: u8) -> bool {
    c.is_ascii_alphanumeric() || c == b'_'
}

/// `grep -oE '\b[a-z][a-z0-9-]*/[0-9]+\b'`.
pub fn cross_feature_deps(line: &[u8]) -> Vec<&[u8]> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < line.len() {
        let starts = (i == 0 || !word(line[i - 1])) && line[i].is_ascii_lowercase();
        if starts {
            let mut j = i + 1;
            while line
                .get(j)
                .is_some_and(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || *c == b'-')
            {
                j += 1;
            }
            if line.get(j) == Some(&b'/') {
                let k = j + 1 + digits_at(line, j + 1);
                if k > j + 1 && !line.get(k).is_some_and(|&c| word(c)) {
                    out.push(&line[i..k]);
                    i = k;
                    continue;
                }
            }
        }
        i += 1;
    }
    out
}

/// The R-IDs awk's `emit_r_ids` prints: repeated
/// `match(s, /(^|[^A-Za-z0-9_])R[0-9]+([^A-Za-z0-9_]|$)/)`, each match
/// consuming its trailing boundary byte, so `^` can match again after it.
pub fn r_ids(s: &[u8]) -> Vec<&[u8]> {
    let mut out = Vec::new();
    let mut rest = s;
    'scan: loop {
        for p in 0..rest.len() {
            let r = if p == 0 && rest[0] == b'R' {
                0
            } else if !word(rest[p]) && rest.get(p + 1) == Some(&b'R') {
                p + 1
            } else {
                continue;
            };
            let d = digits_at(rest, r + 1);
            if d == 0 {
                continue;
            }
            let end = r + 1 + d;
            match rest.get(end) {
                Some(&c) if word(c) => continue,
                Some(_) => {
                    out.push(&rest[r..end]);
                    rest = &rest[end + 1..];
                    continue 'scan;
                }
                None => {
                    out.push(&rest[r..end]);
                    break 'scan;
                }
            }
        }
        break;
    }
    out
}
```

Add to the `tests` module (the expected values were checked against BSD grep 2.6.0-FreeBSD on this machine):

```rust
    #[test]
    fn unit_ids_match_grep() {
        assert_eq!(
            unit_ids(b"### auth/1 SHA256 ABCDE1 x"),
            vec![&b"auth/1"[..], b"SHA256", b"BCDE1"]
        );
        assert!(is_unit_heading(b"### vibe-flow/2 - x"));
        assert!(is_unit_heading(b"### M1 x"));
        assert!(!is_unit_heading(b"####  auth/1"));
        assert!(is_unit_row(b"|  AB12  | caps |"));
        assert!(is_unit_row(b"| auth/2 | row |"));
        assert!(!is_unit_row(b"| auth/2x | row |"));
    }

    #[test]
    fn cross_feature_deps_match_grep() {
        assert_eq!(
            cross_feature_deps(b"see a-b/2 and x/1y c/3 _d/4"),
            vec![&b"a-b/2"[..], b"c/3"]
        );
    }

    #[test]
    fn r_ids_match_awk() {
        assert_eq!(r_ids(b"R1,R2 (R10) xR3 R4x R5"), vec![&b"R1"[..], b"R2", b"R10", b"R5"]);
        assert_eq!(r_ids(b"### Requirement: Sign in (R1)\nUsers"), vec![&b"R1"[..]]);
    }
```

- [ ] **Step 4: Write `content.rs`**

Create `crates/vibe-spec/src/content.rs`:

```rust
//! validate's content checks (SF3, SF8–SF12, SF14, I1): ports of the awk
//! and grep blocks in `oracles/bash/spec/scripts/validate.sh`, rule order
//! and all. F17 fixes their known bugs (R-ID prefix, RFC-2119 boundary,
//! Scope scoping); F16 reproduces them.
use std::collections::BTreeSet;
use std::path::Path;

use crate::pattern::{cross_feature_deps, is_unit_heading, is_unit_row, r_ids, unit_ids};
use crate::text::{contains, contains_ci, first_line, has_line_starting, is_space, lines, name_of, sub_dirs, trim_space};
use crate::validate::Report;

const TOKEN_GROUPS: [&[u8]; 5] = [b"colors", b"typography", b"rounded", b"spacing", b"components"];

fn flush<'a>(pending: &mut Option<&'a [u8]>, has_child: &mut bool, found: &mut Vec<&'a [u8]>) {
    if let Some(p) = pending.take() {
        if !*has_child && TOKEN_GROUPS.contains(&p) {
            found.push(p);
        }
    }
    *has_child = false;
}

/// awk `is_token_key`: `^[a-z]+:` naming a token group with nothing after the colon.
fn token_key(line: &[u8]) -> Option<&[u8]> {
    let n = line.iter().take_while(|c| c.is_ascii_lowercase()).count();
    if n == 0 || line.get(n) != Some(&b':') {
        return None;
    }
    let key = &line[..n];
    let rest = line.get(n + 1..).unwrap_or_default();
    (TOKEN_GROUPS.contains(&key) && trim_space(rest).is_empty()).then_some(key)
}

/// SF3: empty design-token groups in the frontmatter. No END rule: a
/// frontmatter that never closes flushes nothing.
pub fn design_tokens(r: &mut Report, text: &[u8], label: &[u8]) {
    if first_line(text) != b"---" {
        return;
    }
    let mut delim = 0;
    let mut pending: Option<&[u8]> = None;
    let mut has_child = false;
    let mut found: Vec<&[u8]> = Vec::new();
    for line in lines(text) {
        if line == b"---" {
            delim += 1;
            if delim == 2 {
                flush(&mut pending, &mut has_child, &mut found);
                break;
            }
            continue;
        }
        if delim != 1 {
            continue;
        }
        match line.first() {
            Some(&c) if !is_space(c) && c != b'#' => {
                flush(&mut pending, &mut has_child, &mut found);
                if let Some(key) = token_key(line) {
                    pending = Some(key);
                }
            }
            Some(&c) if is_space(c) => {
                if pending.is_some() {
                    has_child = true;
                }
            }
            _ => {}
        }
    }
    for group in found {
        r.yellow(&cat!(label, ": empty token group '", group, "'"));
    }
}

/// SF8: the feature product's Scope section.
pub fn feature_scope(r: &mut Report, text: &[u8], label: &[u8]) {
    if !has_line_starting(text, b"## Scope") {
        r.yellow(&cat!(label, ": missing '## Scope' section (add Owns / Does not own table)"));
        return;
    }
    if !contains_ci(text, b"owns") {
        r.yellow(&cat!(label, ": Scope section missing 'Owns' column or row"));
    }
    if !contains_ci(text, b"does not own") {
        r.yellow(&cat!(label, ": Scope section missing 'Does not own' column or row"));
    }
}

/// SF9: feature frontmatter fields (`sibling:` for product and tech only).
pub fn feature_frontmatter(r: &mut Report, text: &[u8], label: &[u8], kind: &str) {
    if first_line(text) != b"---" {
        return;
    }
    if !has_line_starting(text, b"feature:") {
        r.yellow(&cat!(label, ": missing 'feature:' in frontmatter"));
    }
    if !has_line_starting(text, b"parent:") {
        r.yellow(&cat!(label, ": missing 'parent:' in frontmatter"));
    }
    if (kind == "product" || kind == "tech") && !has_line_starting(text, b"sibling:") {
        r.yellow(&cat!(label, ": missing 'sibling:' in frontmatter"));
    }
}

/// `toupper(s) ~ /(SHALL|MUST|SHOULD|MAY)/` (substring; F17 adds the word boundary).
fn rfc(s: &[u8]) -> bool {
    let up = s.to_ascii_uppercase();
    [&b"SHALL"[..], b"MUST", b"SHOULD", b"MAY"]
        .iter()
        .any(|k| contains(&up, k))
}

/// SF10: `### Requirement:` blocks need an RFC-2119 keyword and a scenario.
/// A requirement with an empty title is never reported (the oracle's
/// `req_line != ""` guard).
pub fn requirements_scenarios(r: &mut Report, text: &[u8], label: &[u8]) {
    if !has_line_starting(text, b"### Requirement:") {
        return;
    }
    let mut issues: Vec<Vec<u8>> = Vec::new();
    let mut req: Vec<u8> = Vec::new();
    let mut has_rfc = false;
    let mut scenarios = 0;
    let mut in_req = false;
    let finish = |issues: &mut Vec<Vec<u8>>, req: &[u8], has_rfc: bool, scenarios: usize| {
        if req.is_empty() {
            return;
        }
        if !has_rfc {
            issues.push(cat!("requirement missing RFC-2119 keyword (SHALL|MUST|SHOULD|MAY) -> ", req));
        }
        if scenarios == 0 {
            issues.push(cat!("requirement has zero #### Scenario: blocks -> ", req));
        }
    };
    for line in lines(text) {
        if let Some(rest) = line.strip_prefix(b"### Requirement:") {
            finish(&mut issues, &req, has_rfc, scenarios);
            let title = rest.iter().position(|&c| !is_space(c)).map_or(&b""[..], |i| &rest[i..]);
            req = title.to_vec();
            has_rfc = rfc(&req);
            scenarios = 0;
            in_req = true;
        } else if line.starts_with(b"#### Scenario:") {
            if in_req {
                scenarios += 1;
            }
        } else if line.starts_with(b"### ") || line.starts_with(b"## ") {
            if in_req {
                finish(&mut issues, &req, has_rfc, scenarios);
                in_req = false;
                req.clear();
                has_rfc = false;
                scenarios = 0;
            }
        } else if in_req && rfc(line) {
            has_rfc = true;
        }
    }
    if in_req {
        finish(&mut issues, &req, has_rfc, scenarios);
    }
    for issue in &issues {
        r.yellow(&cat!(label, ": ", issue));
    }
    if !issues.is_empty() {
        r.yellow(&cat!(label, ": requirement hint — use '### Requirement: Title SHALL ...' with one or more '#### Scenario: ...' blocks"));
    }
}

/// SF11: a feature plan needs unit headings and a Requirements Trace.
pub fn plan_structure(r: &mut Report, text: &[u8], label: &[u8]) {
    if !lines(text).iter().any(|l| is_unit_heading(l)) {
        r.yellow(&cat!(label, ": plan missing '### <feature>/<n>' unit sections (e.g. '### vibe-flow/1 — ...')"));
    }
    if !contains_ci(text, b"requirements trace") {
        r.yellow(&cat!(label, ": plan missing 'Requirements Trace' section linking R-IDs to units"));
    }
}

/// SF11: unit detail inside a root-plan milestone. The oracle prints these
/// from `awk | while read`, a subshell, so they are never counted.
pub fn root_plan_milestones(r: &mut Report, text: &[u8], label: &[u8]) {
    let mut in_milestone = false;
    for line in lines(text) {
        if line.starts_with(b"## M") && line.get(4).is_some_and(u8::is_ascii_digit) {
            in_milestone = true;
            continue;
        }
        if line.starts_with(b"## ") {
            in_milestone = false;
        }
        if in_milestone && is_unit_heading(line) {
            r.yellow_uncounted(&cat!(label, ": milestone section embeds unit heading (move to feature plan) -> ", line));
        }
        if in_milestone && is_unit_row(line) {
            r.yellow_uncounted(&cat!(label, ": milestone section embeds unit table row (move to feature plan) -> ", line));
        }
    }
}

/// The R-IDs of a feature product: lines of the Requirements Trace section,
/// and each `### Requirement:` block, in the oracle's awk rule order.
fn product_r_ids(text: &[u8]) -> BTreeSet<Vec<u8>> {
    let mut ids = BTreeSet::new();
    let mut emit = |s: &[u8]| {
        for id in r_ids(s) {
            ids.insert(id.to_vec());
        }
    };
    let mut in_trace = false;
    let mut in_req = false;
    let mut buf: Vec<u8> = Vec::new();
    for line in lines(text) {
        if line.to_ascii_lowercase().starts_with(b"## requirements trace") {
            in_trace = true;
            continue;
        }
        if line.starts_with(b"## ") {
            in_trace = false;
        }
        if in_trace {
            emit(line);
            continue;
        }
        if line.starts_with(b"### Requirement:") {
            if in_req {
                emit(&buf);
            }
            in_req = true;
            buf = line.to_vec();
        } else if line.starts_with(b"### ") || line.starts_with(b"## ") {
            if in_req {
                emit(&buf);
                in_req = false;
                buf.clear();
            }
        } else if in_req {
            buf.push(b'\n');
            buf.extend_from_slice(line);
        }
    }
    if in_req {
        emit(&buf);
    }
    ids
}

/// SF12: duplicate unit IDs, and product R-IDs not cited in the plan
/// (`grep -q` substring: `R1` counts as cited by `R10`).
pub fn id_traceability(r: &mut Report, product: &Path, plan: &[u8], label: &[u8]) {
    let mut ids: Vec<&[u8]> = lines(plan)
        .into_iter()
        .filter(|l| is_unit_heading(l))
        .flat_map(unit_ids)
        .collect();
    ids.sort();
    let mut dups: Vec<&[u8]> = Vec::new();
    for pair in ids.windows(2) {
        if pair[0] == pair[1] && dups.last() != Some(&pair[0]) {
            dups.push(pair[0]);
        }
    }
    for dup in dups {
        r.yellow(&cat!(label, ": duplicate unit heading ID '", dup, "' in plan"));
    }
    if !product.is_file() {
        return;
    }
    let ids = product_r_ids(&std::fs::read(product).unwrap_or_default());
    if ids.is_empty() {
        r.info(&cat!(label, ": no R-IDs in product.md — R-ID trace check skipped (tag requirement headings '(R1)' to enable)"));
        return;
    }
    for id in ids {
        if !contains(plan, &id) {
            r.yellow(&cat!(label, ": ", id, " in product.md not cited in plan.md (light traceability check)"));
        }
    }
}

/// I1: unit references to other features in a feature plan.
pub fn cross_deps(r: &mut Report, plan: &[u8], feature: &str, label: &[u8]) {
    let own = |d: &[u8]| {
        d.strip_prefix(feature.as_bytes())
            .and_then(|rest| rest.strip_prefix(b"/"))
            .is_some_and(|n| !n.is_empty() && n.iter().all(u8::is_ascii_digit))
    };
    let deps: BTreeSet<&[u8]> = lines(plan)
        .into_iter()
        .flat_map(cross_feature_deps)
        .filter(|d| !own(d))
        .collect();
    for dep in deps {
        r.yellow(&cat!(label, ": cross-feature unit dependency '", dep, "' — use a whole-feature gate in the root plan Feature Sequence, not a unit edge"));
    }
}

/// awk `norm`: drop `` ` `` and `*`, trim, ASCII-lowercase.
fn norm(s: &[u8]) -> Vec<u8> {
    let kept: Vec<u8> = s.iter().copied().filter(|&c| c != b'`' && c != b'*').collect();
    trim_space(&kept).to_ascii_lowercase()
}

/// The Owns items of one feature product, in the oracle's order.
fn owns_items(text: &[u8]) -> Vec<Vec<u8>> {
    let mut out = Vec::new();
    let mut column = false;
    for line in lines(text) {
        let starts = line.iter().position(|&c| !is_space(c));
        if starts.and_then(|i| line.get(i)) != Some(&b'|') {
            column = false;
            continue;
        }
        let fields: Vec<&[u8]> = line.split(|&c| c == b'|').collect();
        let raw3 = fields.get(2).copied().unwrap_or_default();
        let c2 = norm(fields.get(1).copied().unwrap_or_default());
        let c3 = norm(raw3);
        if c2 == b"owns" && c3 == b"does not own" {
            column = true;
            continue;
        }
        if c2 == b"owns" {
            out.extend(raw3.split(|&c| c == b',').map(norm).filter(|x| x.len() >= 5));
            continue;
        }
        if c2 == b"does not own" {
            column = false;
            continue;
        }
        if c2.is_empty() || c2.iter().all(|&c| c == b'-') {
            continue;
        }
        if column && c2.len() >= 5 {
            out.push(c2);
        }
    }
    out
}

/// SF14: two features owning the same item.
pub fn sf14(r: &mut Report, dir: &Path) {
    let mut seen: Vec<(Vec<u8>, String)> = Vec::new();
    for fdir in sub_dirs(&dir.join("features")) {
        let product = fdir.join("product.md");
        if !product.is_file() {
            continue;
        }
        let feature = name_of(&fdir);
        for item in owns_items(&std::fs::read(&product).unwrap_or_default()) {
            if item.is_empty() {
                continue;
            }
            match seen.iter().find(|(i, _)| *i == item) {
                Some((_, other)) if *other != feature => {
                    r.yellow(&cat!("SF14: scope conflict — '", other, "' and '", feature, "' both own: ", item));
                }
                Some(_) => {}
                None => seen.push((item, feature.clone())),
            }
        }
    }
}
```

The oracle passes items through awk `-v` (backslash escapes) and compares them with awk `==` (numeric when both look numeric). No corpus item contains a backslash or is all digits, so the port compares bytes and says so here.

- [ ] **Step 5: Insert the call sites into `validate.rs`**

Add `use crate::content;`, then, in oracle order:

In `root_file`, replace the `if name == "design.md" { … }` block with:

```rust
    if name == "design.md" {
        content::design_tokens(r, &text, n);
        lint_design(r, ctx.project, &format!("{}/design.md", ctx.display), n);
    }
    if name == "plan.md" {
        content::root_plan_milestones(r, &text, n);
    }
    content::requirements_scenarios(r, &text, n);
```

In `feature`, required loop, directly after `if !feature_doc(…) { continue; }`:

```rust
        let kind = required.trim_end_matches(".md");
        content::feature_frontmatter(r, &text, doc.as_bytes(), kind);
        if required == "product.md" {
            content::feature_scope(r, &text, doc.as_bytes());
        }
        content::requirements_scenarios(r, &text, doc.as_bytes());
```

In `feature`, optional loop, replace everything after `if !feature_doc(…) { continue; }` with:

```rust
        content::feature_frontmatter(r, &text, doc.as_bytes(), optional.trim_end_matches(".md"));
        if optional == "design.md" {
            content::design_tokens(r, &text, doc.as_bytes());
            lint_design(r, ctx.project, &format!("{}/{label}/design.md", ctx.display), doc.as_bytes());
        }
        if optional == "plan.md" {
            content::plan_structure(r, &text, doc.as_bytes());
            content::id_traceability(r, &fdir.join("product.md"), &text, label.as_bytes());
            content::cross_deps(r, &text, &fname, doc.as_bytes());
        }
        content::requirements_scenarios(r, &text, doc.as_bytes());
```

In `run`, between `sf13(…)` and `sf15(…)`:

```rust
    content::sf14(&mut r, &dir);
```

Add `mod content;` to `main.rs`, and delete the `#![allow(dead_code, unused_macros)]` line and its comment that Task 2 added. If clippy then reports an unused helper, delete the helper. Do not restore the allow.

- [ ] **Step 6: Run the tests to verify they pass**

```bash
cargo test --locked -p vibe-spec 2>&1 | grep -E 'test result|FAILED|panicked'
cargo clippy --workspace --all-targets --locked -- -D warnings 2>&1 | tail -1
```

Expected: every `test result: ok`; clippy `Finished`.

- [ ] **Step 7: Corpus cases**

In `corpus_spec.rs`, add the constants after `TECH_HEAD`:

```rust
const RICH_PRODUCT: &str = "---\ntype: entrypoint\nscope: product\nupdated: 2026-01-15\n---\n\n# Demo — Product\n\n### Requirement: Users sign in\nThey do.\n\n### Requirement:\nEmpty title.\n\n### Requirement: Export SHALL work\n#### Scenario: happy path\nGiven data.\n\n## Links\n\n[a](b(c).md) [web](https://x.md) [tpl]({name}.md) [top](tech.md#top) [dir](features/ghost/) (archive/old/)\n";
const RICH_PLAN: &str = "---\ntype: entrypoint\nscope: plan\nupdated: 2026-01-15\n---\n\n# Plan\n\n## M1 — Alpha\n\n### auth/1 — leaked unit\n\n| auth/2 | leaked row |\n\n## Notes\n\n### auth/3 — fine here\n";
const RICH_DESIGN: &str = "---\ntype: entrypoint\nscope: design\nchildren:\ncolors:\ntypography:\n  body: Inter\nrounded:\n# a comment\nspacing: 4px\ncomponents:\nupdated: 2026-01-15\n---\n\n# Design\n";
const RICH_TECH: &str = "---\ntype: entrypoint\nscope: technical\nchildren:\n  - tech-api.md\n  - tech-{topic}.md\n  - tech-missing.md\nupdated: 2026-01-15\n---\n\n# Tech\n";
const AUTH_PRODUCT: &str = "---\ntype: feature-product\nfeature: auth\nparent: ../../product.md\nsibling: tech.md\nupdated: 2026-01-15\n---\n\n# auth — Product\n\n## Scope\n\n| | |\n|---|---|\n| **Owns** | session cookies, login form, ui |\n| **Does not own** | billing |\n\n## Requirements\n\n### Requirement: Sign in (R1)\nUsers MUST sign in.\n#### Scenario: ok\nWorks.\n\n### Requirement: Sign out (R10)\nUsers MAY sign out.\n\n## Requirements Trace\n\n| R2 | missing |\n";
const AUTH_PLAN: &str = "---\ntype: feature-plan\nfeature: auth\nparent: ../../plan.md\nupdated: 2026-01-15\n---\n\n# auth — Plan\n\nCovers R10.\n\n### auth/1 — Login\n\n### auth/1 — Login again\n\n### billing/2 — Cross\n\nDepends on billing/3 and auth/2.\n";
const AUTH_TECH: &str = "---\ntype: feature-tech\nfeature: auth\nparent: ../../tech.md\nsibling: product.md\nupdated: 2026-01-15\n---\n\n# auth — Tech\n\n[design](design.md) [plan](plan.md) [gone](gone.md) [abs](/nonexistent/x.md)\n";
const BILLING_PRODUCT: &str = "---\ntype: feature-product\nfeature: billing\nparent: ../../product.md\nsibling: tech.md\nupdated: 2026-01-15\n---\n\n# billing — Product\n\n## Scope\n\n| Owns | Does not own |\n|---|---|\n| `Session Cookies` | auth |\n| invoices | |\n";
const BILLING_PLAN: &str = "---\ntype: feature-plan\nfeature: billing\nparent: ../../plan.md\nupdated: 2026-01-15\n---\n\n# billing — Plan\n\n## Requirements Trace\n\nNone yet.\n";
```

and append to `cases()`:

```rust
        CaseDef {
            class: "malformed",
            name: "rich-root",
            cwd: "",
            runs: &[],
            files: with(
                filled(),
                &[
                    (".spec/product.md", RICH_PRODUCT),
                    (".spec/plan.md", RICH_PLAN),
                    (".spec/design.md", RICH_DESIGN),
                    (".spec/tech.md", RICH_TECH),
                    (".spec/tech-api.md", branch_doc),
                    (".spec/tech-Bad_Topic.md", "---\ntype: tech-topic\nparent: tech.md\nscope: s\ncovers: c\nupdated: 2026-01-15\n---\n"),
                    (".spec/tech-orphan.md", "---\ntype: tech-topic\nupdated: 2026-01-15\n---\n"),
                    (".spec/notes.md", "---\ntype: note\nupdated: 2026-01-15\n---\n"),
                    (
                        ".spec/lessons.md",
                        "# Lessons\n\n### Tagged\n**Tags:** spec\n\n### Escape \\t in titles\n**Rule:** untagged.\n\n### Last\n**Rule:** x\n**Tags:** spec",
                    ),
                    ("CLAUDE.md", "[ok](.spec/product.md) [bad](.spec/gone.md)\n"),
                ],
            ),
        },
        CaseDef {
            class: "malformed",
            name: "empty-root",
            cwd: "",
            runs: &[],
            files: vec![(".spec/notes.txt".into(), "no markdown here\n".into())],
        },
        CaseDef {
            class: "features",
            name: "rich-features",
            cwd: "",
            runs: &[],
            files: with(
                filled(),
                &[
                    (".spec/features/auth/product.md", AUTH_PRODUCT),
                    (".spec/features/auth/plan.md", AUTH_PLAN),
                    (".spec/features/auth/tech.md", AUTH_TECH),
                    (".spec/features/auth/design.md", "# no frontmatter\n"),
                    (".spec/features/billing/product.md", BILLING_PRODUCT),
                    (".spec/features/billing/plan.md", BILLING_PLAN),
                ],
            ),
        },
        CaseDef {
            class: "filled",
            name: "long-root",
            cwd: "",
            runs: &[],
            files: with(
                filled(),
                &[(".spec/product.md", &doc("entrypoint", "product", &"Line.\n".repeat(301)))],
            ),
        },
```

(`branch_doc` already exists in `cases()`.)

- [ ] **Step 8: Pins, floors, candidate, live**

In `tests/parity/spec.json`:
- set `validate`'s `"candidate"` to `["{candidate}", "validate"]`;
- set the floors to `"filled": 3`, `"features": 8`, `"malformed": 4`;
- add to `"divergences"`:

```json
    { "command": "validate", "case": "no-root/bare", "args": [], "field": "stdout", "reason": "the oracle prints its own setup.sh path; the port names vibe-spec init" },
    { "command": "validate", "case": "no-root/subdir", "args": [], "field": "stdout", "reason": "the oracle prints its own setup.sh path; the port names vibe-spec init" },
    { "command": "validate", "case": "filled/long-root", "args": [], "field": "stdout", "os": "macos", "reason": "BSD wc -l pads the SF15 count with spaces; the port prints it unpadded like GNU wc (P3a-R2)" }
```

In `tests/parity/run.sh`, add `live validate validate.sh` after the scan-merges line.

Regenerate the corpus (expected `23 cases written`).

- [ ] **Step 9: Run parity**

Run: `bash tests/parity/run.sh`
Expected:
- candidate: the four `PIN` lines from Tasks 4–5, plus `validate no-root/bare []` and `validate no-root/subdir []`; on macOS also `validate filled/long-root []`;
- `0 failed, 0 error(s)`, `3 skipped` (check-drift, promote, setup);
- five `PASS  live` lines.

A FAIL line names the first differing stdout line: fix the port, never the corpus.

- [ ] **Step 10: Commit**

```bash
git add crates tests/parity
git commit -m "feat(spec): port validate content checks"
```

### Task 8: `plan` and `feature` queries (Spec JSON v1)

**Files:**
- Create: `crates/vibe-spec/src/query.rs`
- Modify: `crates/vibe-spec/src/main.rs` (`mod query;`, `Plan` and `Feature` variants, dispatch)
- Create: `crates/vibe-spec/tests/query.rs`

**Interfaces:**
- Consumes: `root::{Root, Miss}`; `text::{lines, lossy, trim_space, trim_start_space, is_date}`; `vibe_core::exit`.
- Produces:
  - `query::plan(found: &Result<Root, Miss>, feature: &str, json: bool) -> (u8, Vec<u8>)`.
  - `query::feature(found: &Result<Root, Miss>, name: &str, json: bool) -> (u8, Vec<u8>)`.
  - CLI: `vibe-spec plan --feature <f> [--json]` and `vibe-spec feature <f> [--json]`.

- [ ] **Step 1: Write the failing tests**

Create `crates/vibe-spec/tests/query.rs`:

```rust
#![allow(clippy::unwrap_used, clippy::expect_used)]
//! `plan` and `feature` answer Spec JSON v1 from superpowers artifacts, or
//! from the fallback feature folder.
mod common;

use common::{project, spec, write};
use serde_json::{Value, json};

const SP_PLAN: &str = "# X Plan\n\n### Task 1: Freeze the oracles\n\n- [x] **Step 1**\n- [x] **Step 2**\n\n### Task 2: Port\n\n- [x] **Step 1**\n- [ ] **Step 2**\n\n### Task 3\n\n- [ ] **Step 1**\n\n## Notes\n\n- [x] not a task step\n";
const FALLBACK_PLAN: &str = "---\ntype: feature-plan\n---\n\n## Status\n\n| Unit | Status |\n|---|---|\n| auth/1 | DONE |\n| auth/2 | IN PROGRESS |\n|  |  |\n\nAfter the table.\n";

fn v(out: &common::Out) -> Value {
    assert_eq!(out.code, 0, "{}", out.stderr);
    serde_json::from_slice(&out.stdout).unwrap()
}

#[test]
fn plan_reads_the_newest_superpowers_plan() {
    let p = project("q-plan-sp");
    std::fs::create_dir_all(p.join(".spec")).unwrap();
    write(&p, "docs/superpowers/plans/2026-01-01-csv-export.md", "### Task 1: Old\n");
    write(&p, "docs/superpowers/plans/2026-02-01-csv-export.md", SP_PLAN);
    write(&p, "docs/superpowers/plans/2026-03-01-csv-export-v2.md", "### Task 9: Other feature\n");
    let out = spec(&p, &["plan", "--feature", "csv-export", "--json"], &[]);
    assert_eq!(
        v(&out),
        json!({
            "v": 1,
            "feature": "csv-export",
            "source": { "kind": "plan", "path": "docs/superpowers/plans/2026-02-01-csv-export.md" },
            "rows": [
                { "id": "Task 1", "status": "done", "title": "Freeze the oracles" },
                { "id": "Task 2", "status": "in-progress", "title": "Port" },
                { "id": "Task 3", "status": "todo" }
            ]
        })
    );
    #[cfg(unix)]
    common::assert_spec_json("plan", out.text().trim_end());
    let text = spec(&p, &["plan", "--feature", "csv-export"], &[]);
    assert_eq!(text.text(), "Task 1\tdone\tFreeze the oracles\nTask 2\tin-progress\tPort\nTask 3\ttodo\n");
}

#[test]
fn plan_falls_back_to_the_unit_status_table() {
    let p = project("q-plan-fb");
    write(&p, ".spec/features/auth/plan.md", FALLBACK_PLAN);
    let out = spec(&p, &["plan", "--feature", "auth", "--json"], &[]);
    assert_eq!(
        v(&out),
        json!({
            "v": 1,
            "feature": "auth",
            "source": { "kind": "plan", "path": ".spec/features/auth/plan.md" },
            "rows": [
                { "id": "auth/1", "status": "DONE" },
                { "id": "auth/2", "status": "IN PROGRESS" }
            ]
        })
    );
    #[cfg(unix)]
    common::assert_spec_json("plan", out.text().trim_end());
}

#[test]
fn feature_reports_its_layer_and_docs() {
    let p = project("q-feature");
    write(&p, ".spec/features/auth/product.md", "x\n");
    write(&p, ".spec/features/auth/plan.md", "x\n");
    write(&p, ".spec/features/auth/notes.md", "x\n");
    write(&p, "docs/superpowers/specs/2026-02-01-csv-export-design.md", "x\n");
    let sp = spec(&p, &["feature", "csv-export", "--json"], &[]);
    assert_eq!(
        v(&sp),
        json!({
            "v": 1, "feature": "csv-export", "layer": "superpowers",
            "docs": [{ "kind": "design", "path": "docs/superpowers/specs/2026-02-01-csv-export-design.md" }]
        })
    );
    let fb = spec(&p, &["feature", "auth", "--json"], &[]);
    assert_eq!(
        v(&fb),
        json!({
            "v": 1, "feature": "auth", "layer": "fallback",
            "docs": [
                { "kind": "product", "path": ".spec/features/auth/product.md" },
                { "kind": "plan", "path": ".spec/features/auth/plan.md" }
            ]
        })
    );
    #[cfg(unix)]
    {
        common::assert_spec_json("feature", sp.text().trim_end());
        common::assert_spec_json("feature", fb.text().trim_end());
    }
    let text = spec(&p, &["feature", "auth"], &[]);
    assert_eq!(text.text(), "layer: fallback\nproduct\t.spec/features/auth/product.md\nplan\t.spec/features/auth/plan.md\n");
}

#[test]
fn exit_codes_follow_the_contract() {
    let p = project("q-codes");
    std::fs::create_dir_all(p.join(".spec")).unwrap();
    assert_eq!(spec(&p, &["plan", "--feature", "nope", "--json"], &[]).code, 1);
    assert_eq!(spec(&p, &["feature", "nope"], &[]).code, 1);
    assert_eq!(spec(&p, &["feature", "Bad_Name"], &[]).code, 2);
    assert_eq!(spec(&p, &["plan", "--feature", "../x"], &[]).code, 2);
    assert_eq!(spec(&p, &["plan"], &[]).code, 2);
    let none = project("q-none");
    assert_eq!(spec(&none, &["plan", "--feature", "auth", "--json"], &[]).code, 3);
    assert_eq!(spec(&none, &["feature", "auth"], &[]).code, 3);
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test --locked -p vibe-spec --test query 2>&1 | tail -5`
Expected: FAIL — unrecognized subcommand `plan`.

- [ ] **Step 3: Write `query.rs`**

Create `crates/vibe-spec/src/query.rs`:

```rust
//! `vibe-spec plan --feature <f>` and `vibe-spec feature <f>`: the Spec
//! JSON v1 `plan` and `feature` queries (contracts/spec-json.schema.json).
//!
//! superpowers artifacts come first (D45): the newest
//! `docs/superpowers/plans/YYYY-MM-DD-<f>.md` and
//! `docs/superpowers/specs/YYYY-MM-DD-<f>-design.md` under the project.
//! Without them, the fallback folder `<root>/features/<f>/` answers.
use std::path::{Path, PathBuf};

use serde_json::{Value, json};
use vibe_core::exit;

use crate::root::{Miss, Root};
use crate::text::{is_date, lines, lossy, trim_space, trim_start_space};

/// superpowers' artifact directories, relative to the project.
const SP_SPECS: &str = "docs/superpowers/specs";
const SP_PLANS: &str = "docs/superpowers/plans";
/// Fallback feature docs, in report order (`doc.kind`).
const FALLBACK_DOCS: [&str; 5] = ["product", "tech", "design", "plan", "research"];

struct Row {
    id: String,
    status: String,
    title: Option<String>,
}

/// `^[a-z0-9][a-z0-9-]*$` (`feature-name` in the schema).
fn valid_name(name: &str) -> bool {
    let b = name.as_bytes();
    b.first().is_some_and(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
        && b.iter().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || *c == b'-')
}

fn checked<'a>(found: &'a Result<Root, Miss>, cmd: &str, name: &str) -> Result<&'a Root, (u8, Vec<u8>)> {
    if !valid_name(name) {
        eprintln!("vibe-spec {cmd}: invalid feature name {name:?} (lowercase-kebab)");
        return Err((exit::USAGE, Vec::new()));
    }
    found.as_ref().map_err(|miss| {
        eprintln!("{}", miss.message());
        (miss.code(), Vec::new())
    })
}

/// The newest `YYYY-MM-DD-<feature><suffix>` file in `dir`.
fn newest(dir: &Path, feature: &str, suffix: &str) -> Option<PathBuf> {
    let want = format!("-{feature}{suffix}");
    let mut hits: Vec<String> = std::fs::read_dir(dir)
        .ok()?
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| {
            let b = n.as_bytes();
            b.len() == 10 + want.len() && is_date(&b[..10]) && &b[10..] == want.as_bytes()
        })
        .collect();
    hits.sort();
    hits.pop().map(|n| dir.join(n))
}

/// `path` relative to `project`, `/`-joined.
fn rel(project: &Path, path: &Path) -> String {
    let p = path.strip_prefix(project).unwrap_or(path);
    p.components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/")
}

/// `vibe-spec plan --feature <f> [--json]`.
pub fn plan(found: &Result<Root, Miss>, feature: &str, json_out: bool) -> (u8, Vec<u8>) {
    let root = match checked(found, "plan", feature) {
        Ok(root) => root,
        Err(e) => return e,
    };
    let (path, rows) = match newest(&root.project.join(SP_PLANS), feature, ".md") {
        Some(p) => {
            let rows = task_rows(&std::fs::read(&p).unwrap_or_default());
            (p, rows)
        }
        None => {
            let p = root.dir.join("features").join(feature).join("plan.md");
            if !p.is_file() {
                eprintln!("vibe-spec plan: no plan for feature {feature}");
                return (exit::FINDINGS, Vec::new());
            }
            let rows = table_rows(&std::fs::read(&p).unwrap_or_default());
            (p, rows)
        }
    };
    if !json_out {
        let mut out = String::new();
        for r in &rows {
            out.push_str(&format!("{}\t{}", r.id, r.status));
            if let Some(t) = &r.title {
                out.push_str(&format!("\t{t}"));
            }
            out.push('\n');
        }
        return (exit::OK, out.into_bytes());
    }
    let rows: Vec<Value> = rows
        .iter()
        .map(|r| {
            let mut v = json!({ "id": r.id, "status": r.status });
            if let Some(t) = &r.title {
                v["title"] = json!(t);
            }
            v
        })
        .collect();
    let v = json!({
        "v": 1,
        "feature": feature,
        "rows": rows,
        "source": { "kind": "plan", "path": rel(&root.project, &path) },
    });
    (exit::OK, format!("{v}\n").into_bytes())
}

/// Rows of a superpowers plan: one per `### Task <n>[: title]`, status from
/// its checkboxes up to the next `##`/`###` heading.
fn task_rows(text: &[u8]) -> Vec<Row> {
    let mut rows: Vec<Row> = Vec::new();
    let mut counts: Option<(usize, usize)> = None;
    for line in lines(text) {
        if line.starts_with(b"## ") || line.starts_with(b"### ") {
            close(&mut rows, &mut counts);
            if let Some((id, title)) = task_heading(line) {
                rows.push(Row { id, status: String::new(), title });
                counts = Some((0, 0));
            }
            continue;
        }
        if let Some((done, open)) = counts.as_mut() {
            let t = trim_start_space(line);
            if t.starts_with(b"- [x]") || t.starts_with(b"- [X]") {
                *done += 1;
            } else if t.starts_with(b"- [ ]") {
                *open += 1;
            }
        }
    }
    close(&mut rows, &mut counts);
    rows
}

fn close(rows: &mut [Row], counts: &mut Option<(usize, usize)>) {
    if let (Some((done, open)), Some(last)) = (counts.take(), rows.last_mut()) {
        last.status = match (done, open) {
            (0, _) => "todo",
            (_, 0) => "done",
            _ => "in-progress",
        }
        .to_string();
    }
}

/// `### Task <n>[: <title>]` → (`Task <n>`, title).
fn task_heading(line: &[u8]) -> Option<(String, Option<String>)> {
    let rest = line.strip_prefix(b"### Task ")?;
    let n = rest.iter().take_while(|c| c.is_ascii_digit()).count();
    if n == 0 {
        return None;
    }
    let after = rest.get(n..).unwrap_or_default();
    if !(after.is_empty() || after.first() == Some(&b':')) {
        return None;
    }
    let title = trim_space(after.strip_prefix(b":").unwrap_or(after));
    Some((
        format!("Task {}", lossy(&rest[..n])),
        (!title.is_empty()).then(|| lossy(title)),
    ))
}

/// The trimmed cells of a `| a | b |` row; empty for a non-table line.
fn cells(line: &[u8]) -> Vec<&[u8]> {
    let t = trim_space(line);
    let Some(inner) = t.strip_prefix(b"|") else {
        return Vec::new();
    };
    let inner = inner.strip_suffix(b"|").unwrap_or(inner);
    inner.split(|&c| c == b'|').map(trim_space).collect()
}

/// Rows of a fallback feature plan: the `| Unit | Status |` table.
fn table_rows(text: &[u8]) -> Vec<Row> {
    let ls = lines(text);
    let Some(head) = ls.iter().position(|l| {
        let c = cells(l);
        c.first() == Some(&&b"Unit"[..]) && c.get(1) == Some(&&b"Status"[..])
    }) else {
        return Vec::new();
    };
    ls.get(head + 2..)
        .unwrap_or_default()
        .iter()
        .take_while(|l| trim_start_space(l).starts_with(b"|"))
        .filter_map(|l| {
            let c = cells(l);
            let (id, status) = (c.first()?, c.get(1)?);
            (!id.is_empty() && !status.is_empty()).then(|| Row {
                id: lossy(id),
                status: lossy(status),
                title: None,
            })
        })
        .collect()
}

/// `vibe-spec feature <f> [--json]`.
pub fn feature(found: &Result<Root, Miss>, name: &str, json_out: bool) -> (u8, Vec<u8>) {
    let root = match checked(found, "feature", name) {
        Ok(root) => root,
        Err(e) => return e,
    };
    let mut docs: Vec<(&str, PathBuf)> = Vec::new();
    if let Some(p) = newest(&root.project.join(SP_SPECS), name, "-design.md") {
        docs.push(("design", p));
    }
    if let Some(p) = newest(&root.project.join(SP_PLANS), name, ".md") {
        docs.push(("plan", p));
    }
    let layer = if docs.is_empty() {
        let dir = root.dir.join("features").join(name);
        if !dir.is_dir() {
            eprintln!("vibe-spec feature: no feature {name}");
            return (exit::FINDINGS, Vec::new());
        }
        for kind in FALLBACK_DOCS {
            let p = dir.join(format!("{kind}.md"));
            if p.is_file() {
                docs.push((kind, p));
            }
        }
        "fallback"
    } else {
        "superpowers"
    };
    if !json_out {
        let mut out = format!("layer: {layer}\n");
        for (kind, p) in &docs {
            out.push_str(&format!("{kind}\t{}\n", rel(&root.project, p)));
        }
        return (exit::OK, out.into_bytes());
    }
    let docs: Vec<Value> = docs
        .iter()
        .map(|(kind, p)| json!({ "kind": kind, "path": rel(&root.project, p) }))
        .collect();
    let v = json!({ "v": 1, "feature": name, "layer": layer, "docs": docs });
    (exit::OK, format!("{v}\n").into_bytes())
}
```

- [ ] **Step 4: Wire the subcommands**

In `main.rs`: add `mod query;`, the variants

```rust
    /// A feature's plan rows (Spec JSON v1 `plan` with --json).
    Plan {
        /// Feature name (lowercase-kebab).
        #[arg(long)]
        feature: String,
        /// Print Spec JSON v1.
        #[arg(long)]
        json: bool,
    },
    /// A feature's layer and docs (Spec JSON v1 `feature` with --json).
    Feature {
        /// Feature name (lowercase-kebab).
        name: String,
        /// Print Spec JSON v1.
        #[arg(long)]
        json: bool,
    },
```

and the arms:

```rust
        Command::Plan { feature, json } => query::plan(&found, &feature, json),
        Command::Feature { name, json } => query::feature(&found, &name, json),
```

- [ ] **Step 5: Run the tests to verify they pass**

```bash
cargo test --locked -p vibe-spec 2>&1 | grep -E 'test result|FAILED|panicked'
cargo clippy --workspace --all-targets --locked -- -D warnings 2>&1 | tail -1
cargo fmt --all --check && echo fmt-ok
```

Expected: every `test result: ok`; clippy `Finished`; `fmt-ok`.

- [ ] **Step 6: Dogfood on this repo**

```bash
cargo run --locked -q -p vibe-spec -- plan --feature spec-v04-p2-rust-foundations
cargo run --locked -q -p vibe-spec -- feature spec-v04 --json
```

Expected:
- The first prints nine `Task <n>\t<status>\t<title>` lines (Phase 2's tasks).
- The second prints `"layer":"superpowers"`, with the design doc `docs/superpowers/specs/2026-10-13-spec-v04-design.md` and the plan `docs/superpowers/plans/2026-10-13-spec-v04.md`.

- [ ] **Step 7: Commit**

```bash
git add crates/vibe-spec
git commit -m "feat(spec): add plan and feature queries"
```

### Task 9: Docs, program index and phase gate

**Files:**
- Modify (inside `strategy.spec`):
  - `.spec/tech-spec.md`, `.spec/tech-rust.md`, `.spec/tech-contracts.md`, `.spec/plan.md`, `.spec/lessons.md`;
  - `AGENTS.md`, inside the generated markers only (via `regen-active-rules.sh`).
- Modify: `docs/superpowers/plans/2026-10-13-spec-v04.md` (phase 3 → 3a/3b/3c)

**Interfaces:**
- Consumes: everything above.
- Produces: the F17 baseline. `p3b` is written against the `validate`, `content` and `pattern` modules as they landed.

- [ ] **Step 1: Run every affected check and keep the output**

Run each alone, in this order:

```bash
cargo test --workspace --locked 2>&1 | grep -E 'test result|FAILED'
cargo clippy --workspace --all-targets --locked -- -D warnings 2>&1 | tail -1
cargo fmt --all --check && echo fmt-ok
cargo tree --workspace -d 2>&1 | head -2
bash tests/parity/run.sh 2>&1 | tail -20
bash oracles/check.sh
bash contracts/run.sh | tail -1
git ls-files '*.sh' | grep -Ev '^oracles/(bash|js)/' | xargs shellcheck; echo "shellcheck rc=$?"
bash spec/scripts/validate.sh | tail -2; echo "validate rc=$?"
bash spec/scripts/check-drift.sh; echo "drift rc=$?"
bash tests/run.sh 2>&1 | tail -12
```

Expected:
- every cargo result `ok`, and `nothing to print`;
- parity: the self-check is all pass; the candidate shows the six F16 `PIN` lines (seven on macOS), `0 failed, 0 error(s)`, `3 skipped`; five `PASS  live` lines;
- `oracles: 43 files match MANIFEST.sha256`, and contracts OK;
- `shellcheck rc=0`, `validate rc=0`, `drift rc=0`;
- `tests/run.sh`: every suite `PASS` except `engine`, whose only failure is the pre-existing `doctor` perf budget (P3a-R10). Any other failure is new and blocks the gate.

- [ ] **Step 2: Update the program index**

In `docs/superpowers/plans/2026-10-13-spec-v04.md`, replace the phase 3 row with:

```markdown
| 3a | `2026-10-15-spec-v04-p3a-spec-core.md` | F16 | parity green per ported script (corpus candidate + live on this repo); exit 0/3 goldens; F16 acceptance |
| 3b | `…-p3b-spec-validators.md` (written after 3a) | F17 | F17 acceptance; one landed mutant per fixed bug; warn/error/off matrix |
| 3c | `…-p3c-spec-delta.md` (written after 3b) | F18 | F18 acceptance; `expected-divergence` cases pinned; `openspec validate` on an export |
```

and change "Five phase plans in gate order" to "Phase plans in gate order (phase 3 split into 3a–3c by owner ruling P3a-R1)".

- [ ] **Step 3: Update the spec docs**

```bash
bash .agents/skills/vibe/scripts/set-state.sh strategy.brainstorm
bash .agents/skills/vibe/scripts/set-state.sh strategy.spec
```

`.spec/tech-spec.md`, section "Shipped today": replace the last sentence ("The Rust port replaces these scripts.") with:

```markdown
F16 adds the `vibe-spec` binary (dev build: `cargo build -p vibe-spec`).
Every command goes through one discovery function. It offers `root`,
byte-identical ports of `list-specs`, `lessons-for`, `scan-merges` and
`validate`, and the `lessons-for --json`, `plan` and `feature` queries.
`tests/parity/run.sh` proves the ports against the frozen oracles, over the
corpus and live on this repo. Skills, CI and flow keep calling the scripts
until the binary ships (F21–F22).
```

`.spec/tech-rust.md`:
- Replace the "**Shipped today:**" paragraph with:

```markdown
**Shipped today:** `flow/engine/` (Node/ESM) and the bash scripts still
drive everything. F2–F4 delivered the foundations: frozen oracles under
`oracles/` (43 files, `MANIFEST.sha256`), the Cargo workspace (`vibe-core`
with glob matching and atomic write, `vibe-spec`, `vibe-parity`), and the
parity harness. F16 made `vibe-spec` the first candidate. `list-specs`,
`lessons-for`, `scan-merges` and `validate` are parity-green over
`tests/parity/corpus/spec/` and live on this repo (`tests/parity/run.sh`).
`check-drift`, `promote` and `init` stay self-check only until F18/F20.
```

- Replace the paragraph that starts "Known pins for F16:" with:

```markdown
Pins are per argument set and may be scoped to one OS (`"os": "macos"`
for BSD-tool differences; an os pin is never stale-checked). F16's pins:
legacy `--format json` output is escaped (`lessons-for`, `scan-merges`);
tags match literally, not as regexes; the no-root `validate` error names
`vibe-spec init`; BSD `wc -l` pads the SF15 count.
```

- In "Carried into later rows", delete the clause starting "F16 adds a run timeout", up to and including "before candidate mode reaches CI". End the sentence after the F6 clause.

`.spec/tech-contracts.md`, section "Spec JSON v1": after the paragraph that starts "`doc.kind` is", add:

```markdown
`plan` rows come from the newest `docs/superpowers/plans/YYYY-MM-DD-<f>.md`:
one row per `### Task N:`, with status `done`, `in-progress` or `todo` from
its checkboxes. Without that plan, they come from the `| Unit | Status |`
table of `<root>/features/<f>/plan.md`. `feature` reports the layer
`superpowers` when a design or plan artifact exists. Otherwise it reports
`fallback`, with the folder's `product|tech|design|plan|research` docs.
```

`.spec/plan.md`: set the Status cell of row F16 to `IN PROGRESS — local gate green; CI pending push`.

`.spec/lessons.md`: add one lesson per failed command, reverted change or late-found defect in this phase (Pattern / Rule / Tags / Date: today). Bump `updated:` on every edited spec doc. Then:

```bash
bash .agents/skills/vibe/scripts/regen-active-rules.sh
bash spec/scripts/validate.sh | tail -2
bash spec/scripts/check-drift.sh
bash tests/parity/run.sh 2>&1 | tail -3
bash .agents/skills/vibe/scripts/set-state.sh idle
```

(The live `validate` and `lessons-for` comparisons now cover the edited docs.)

- [ ] **Step 4: Commit**

```bash
git add .spec AGENTS.md docs/superpowers/plans/2026-10-13-spec-v04.md
git commit -m "docs(spec): record f16 spec core"
```

- [ ] **Step 5: Owner gate**

Report to the owner:
- the Step 1 outputs;
- the pin list;
- the live parity lines;
- the open items below.

Ask for push approval. After CI is green on every job, set F16 to `DONE` in a `strategy.spec` commit `docs(spec): close f16 row`. Then write `p3b` (F17) against the landed `validate`, `content` and `pattern` modules.

**Owner items (not in this plan's commits):**
- The `AGENTS.md` "Commands" section is owner prose. It misses `cargo build -p vibe-spec` and `bash tests/parity/run.sh`.
- The items carried from Phase 2 are still open.

