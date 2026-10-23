# Spec v0.4 Phase 3b — spec-validators (F17) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Turn `vibe-spec validate` from a byte-parity port into the spec validator row F17 (#34) asks for. That means:
- a check registry with per-check strictness, `--strict` and `--format json`;
- the four substring-bug fixes;
- OpenSpec grammar checks with one R-ID matcher;
- flat and layered adherence checks and the superpowers artifact checks;
- `vibe-spec trace`.

**Architecture:** Every finding names one registered check (`SF3` … `SF36`, `I1`). A `Report` resolves each finding's level from `.vibe/spec.json` `strictness` (else the finding's own default), applies `--strict` and allow markers, and renders text (byte-identical to F16 for legacy checks) or Spec JSON. Parity keeps proving the legacy engine. Every corpus tree carries a `.vibe/spec.json` that turns the F17-added checks off; the oracle ignores that file. Each deliberate fix is pinned. New checks are proven by Rust fixture tests and a warn/error/off matrix over every registered check.

**Tech Stack:** Rust 1.88 MSRV / edition 2024 (`clap`, `serde`, `serde_json` only), frozen bash oracles under `oracles/bash/spec/`, `git` (subprocess) for freshness and `trace`, Node ≥18 for contract checks.

**Spec:** `docs/superpowers/specs/2026-10-13-spec-v04-design.md`. The relevant sections are §4 (layout and validators, D44), §7 (command surface) and §9 (testing). Supporting docs:
- Program index: `docs/superpowers/plans/2026-10-13-spec-v04.md` (row 3b).
- Row detail: `docs/roadmap/issues/17-F17-spec-validators.md`.
- Strictness and adherence: `.spec/tech-spec.md` ("Strictness", "Adherence checks", "Migrate").
- Contracts: `.spec/tech-contracts.md` ("Spec JSON v1", "Exit-code contract").
- Predecessor: `docs/superpowers/plans/2026-10-15-spec-v04-p3a-spec-core.md`.

## Global Constraints

- Commits: Conventional Commits, imperative, lowercase, ≤50-char subject, no body, no footer.
- MSRV 1.88, edition 2024. The dependency allowlist is `clap` (derive), `serde` and `serde_json`. No `toml`, `regex`, `tokio`, `tempfile` or `git2`. Every pattern match is hand-written. Git runs as a subprocess.
- `vibe-core` holds no spec paths or spec vocabulary. Everything in this plan lives in `crates/vibe-spec`, except the corpus generator in `crates/vibe-parity`.
- No `unwrap`/`expect` outside tests (`clippy::unwrap_used` and `clippy::expect_used` deny). No panics on input: index with `get`, and slice only after a length check.
- Exit codes are 0 ok · 1 findings/refused · 2 usage · 3 no spec tree.
  - `validate` in text mode keeps the oracle's codes, so a missing root exits 1.
  - `validate --format json` and `trace` exit 3 when no root is found.
- Strictness: every F17 check ships `warn` by default (D35). No check is flipped to `error` in this repo's config in this plan.
- With the F17 checks set `off`, `validate`'s stdout is byte-identical to the oracle's, except where a pin in `tests/parity/spec.json` says otherwise. An oracle never changes. The live bash scripts under `spec/scripts/` do not change either (P3b-R12).
- Every guard asserts its own population: corpus floors rise with cases, the matrix test covers every registered check, and every fixed bug gets a landed mutant that fails its test.
- Line scanners over Markdown that this plan adds skip fenced code (lesson "Line-based Markdown scanners must skip fenced code"). The legacy checks keep the oracle's behaviour.
- Test git sandboxes set `commit.gpgsign false` and `tag.gpgsign false`, and check every setup command's exit status (lesson "Sandbox git repos in tests inherit the user's signing config").
- Run every affected suite and show its output before claiming a task done. Never run `bash tests/run.sh`, `node flow/engine/tests/run.mjs` or `bash tests/parity/run.sh` concurrently with each other or with `cargo` builds.
- Never Edit or Write a file under `tests/parity/corpus/`. Change `crates/vibe-parity/src/corpus_spec.rs` and regenerate with `cargo run --locked -q -p vibe-parity -- gen-spec-corpus oracles/bash/spec/reference/templates tests/parity/corpus/spec`.
- Tasks run with the flow cursor at `idle`. Every `.spec/` edit goes through `strategy.spec` (`set-state.sh strategy.brainstorm` → `strategy.spec` → back to `idle`).
- Outward-facing actions (push, PR, issue edits) need an explicit owner yes at that moment.
- Scratch files go in `.superpowers/p3b/` (gitignored), never `/tmp`.

## Review Focus

1. **A config that changes a legacy check, combined with an allow marker.** Expected:
   - With `"SF-12": "off"` the SF12 lines disappear and every other byte of the text output stays the same.
   - With `"SF-12": "error"` those lines print as `ERROR:` and the run exits 1.
   - An allow marker beats both the config and `--strict`.

   Tests: Task 2, `a_legacy_check_can_be_turned_off_or_up`, `an_allow_marker_beats_strict`.
2. **Fenced examples inside requirements and scenarios.** OpenSpec specs often show sample output in fences. Expected: a `#### Scenario:` or `- WHEN` inside a fence counts for nothing in SF24 and SF25. Test: Task 6, `fenced_examples_do_not_count`.
3. **Git edge cases for freshness and trace:**
   - a project with no commits;
   - an untracked file;
   - a spec root outside the repository;
   - a missing `git` binary.

   Expected: SF29 stays silent, and `trace` falls back to `tests/` with a note on stderr. Tests: Task 8, `freshness_is_silent_without_history`; Task 12, `trace_without_git_uses_tests_only`.
4. **A layered tree with awkward inputs:**
   - CRLF line endings;
   - a last line with no newline;
   - an uppercase file name;
   - a nested topic folder with its own hub;
   - a dotfile.

   Expected:
   - Budgets count `\n` bytes.
   - Uppercase names fail SF32.
   - Nested topic folders are allowed.
   - Dotfiles are ignored.

   Tests: Task 9, `layered_edge_inputs`.
5. **JSON output with hostile messages:**
   - control characters;
   - invalid UTF-8 in a file name or heading;
   - a missing root.

   Expected:
   - The output is valid JSON and validates against `$defs/validate`.
   - A missing root gives exit 3 with empty stdout.

   Test: Task 3, `json_survives_hostile_bytes`.

## Rulings (owner review)

The owner chose R1–R3 in this session. R4–R21 decide what the spec leaves open; each lists its cost if wrong.

- **P3b-R1 — Scope.** One plan for all of F17 (owner). Cost: a long plan. The tasks are cut so a reviewer can reject one without the others.
- **P3b-R2 — Parity through the behaviour change (owner).**
  - Corpus trees carry `.vibe/spec.json` from the single source `tests/parity/legacy-strictness.json`, which sets every F17-added check `off`. The oracle ignores it, so the legacy engine stays byte-proven over the corpus.
  - Each of the three output-changing fixes gets a corpus case and a pin. The fourth fix is the new check SF24, which is off in the corpus.
  - `live validate` is removed from `tests/parity/run.sh`. `list-specs`, `lessons-for` and `scan-merges` stay live.
  - The Rust parity tests in `crates/vibe-spec/tests/validate.rs` write the same config before comparing.
  - Cost: the real repo's `validate` output is no longer compared with the oracle.
- **P3b-R3 — Unified R-ID (owner).** One word-bounded matcher for the feature-local ID `R<digits>`:
  - `pattern::r_ids`, extended to every place an R-ID is read.
  - It recognises the `(R1)` suffix and the OpenSpec `### Requirement: R1 — name` prefix.
  - SF12 compares sets, not substrings.
  - The doc-only `R-1` form in `spec/reference/product.md` becomes `R1`.
  - Cost: none known. `R-1` was never parsed.
- **P3b-R4 — Check ids.** Every finding carries one id from the registry in `src/check.rs`:
  - Legacy, numbered by the oracle: SF3, SF4, SF8–SF16, I1.
  - Legacy families the oracle left unnumbered: SF20 frontmatter presence, SF21 file naming, SF22 link resolution, SF23 spec tree and required files.
  - F17: SF24–SF36 (table in R11).
  - SF1, SF2, SF5–SF7 and SF17–SF19 stay unused, because `spec/tests/run.sh` uses them as historic unit ids.
  - Config keys accept `SF-nn`, `SFnn` and `I1`, case-insensitive. Output prints `SFnn`.
  - Cost: none known. Renumbering later breaks configs.
- **P3b-R5 — Levels.**
  - Each finding has a default severity: legacy errors stay errors, everything else is a warning.
  - A configured level (`warn|error|off`) replaces the default for every finding of that check.
  - `--strict` turns every resulting `warn` into `error`. `off` drops the finding: it is not printed and not counted.
  - The oracle's never-counted SF11 milestone warnings stay uncounted at `warn` and count once they resolve to `error`.
  - Cost: a config can downgrade a legacy error. That is the owner's choice to make.
- **P3b-R6 — Config.**
  - Strictness lives in `<project>/.vibe/spec.json` key `strictness`, an object of check → level. Only `validate` reads it.
  - An unknown check id prints one stderr warning and is ignored.
  - These exit 2: a non-object `strictness`, a level outside `warn|error|off`, or an unreadable or non-JSON file.
  - Cost: a typo in an id only warns.
- **P3b-R7 — Zero-input refusal.** `validate --strict` refuses when it examined no file, or when every registered check resolves `off`. It prints one `ERROR:` line that no config can silence, and exits 1. Cost: none known.
- **P3b-R8 — Allow markers.** `<!-- vibe-spec: allow SF-nn -->` anywhere in a file suppresses that check's findings for that file, before strictness and `--strict` apply. F20's migrate inserts them (`.spec/tech-spec.md` "Migrate"). Cost: a stale marker hides a real finding until someone deletes it.
- **P3b-R9 — Flags.**
  - F17 ships `validate --strict`, `--format text|json` and `--json` (= `--format json`).
  - F19 adds `--changed`, `--file` and `--format hook` when it wires the hooks, because it owns their predicates and time caps.
  - `validate --format json` with no root exits 3 with empty stdout; text mode keeps the oracle's exit 1.
  - Cost: F19 extends this parser.
- **P3b-R10 — Layout.**
  - Detection: a root is `layered` when one of its root `product.md`, `tech.md`, `design.md`, `plan.md` or `README.md` has a frontmatter line `type: hub`. Otherwise it is `flat`, the schema's existing name for legacy.
  - In layered mode, root `README.md` and `AGENTS.md` skip the root-file loop, and SF15 is replaced by the SF31 budgets.
  - `root --json` reports `layout`.
  - Cost: a layered tree with no hub-typed root file validates as flat until one is added.
- **P3b-R11 — Where each F17 check runs.**

  | id | check | flat | layered |
  |---|---|---|---|
  | SF24 | requirement population: a fallback `features/<f>/product.md` with zero `### Requirement:` blocks, or a requirement with an empty title (bug d) | yes | yes |
  | SF25 | scenario grammar: each `#### Scenario:` has a WHEN bullet and a THEN bullet | yes | yes |
  | SF26 | frontmatter values: `type` in the layout's vocabulary, `updated` a `YYYY-MM-DD` date, entrypoint `scope` known | yes | yes |
  | SF27 | `children:` (block and inline lists): inline children exist; every existing child's `parent:` names its entrypoint | yes | — |
  | SF28 | each `features/<f>/` folder appears as a cell in a root `plan.md` table row | yes | yes |
  | SF29 | `updated:` is not older than the file's last commit date | yes | yes |
  | SF30 | layout allowlist | — | yes |
  | SF31 | line budgets: hub ≤80, note ≤60 | — | yes |
  | SF32 | names: lowercase-kebab files and folders, note names unique across the tree | — | yes |
  | SF33 | typed notes: `type` matches the kind folder, `updated` present, `code:` on areas exists, `applies:` on tech set | — | yes |
  | SF34 | every kind note is reachable from a hub by links | — | yes |
  | SF35 | superpowers artifact names (`specs/YYYY-MM-DD-<topic>-design.md`, `plans/YYYY-MM-DD-<topic>.md`, nothing else) | yes | yes |
  | SF36 | each superpowers plan has a `**Spec:**` line whose first path resolves | yes | yes |

  SF22 (links) also covers every note of a layered tree. The flat layout keeps its legacy link scope. Cost: a flat tree gets no layout or budget checks until F20 migrates it, which is what D35's warn-first ratchet intends.
- **P3b-R12 — Bash stays.** The fixes land in Rust only. `spec/scripts/validate.sh` and its consumers stay as they are until F21s/F22s (P3a-R7). Cost: skill users keep the four bugs until the binary ships.
- **P3b-R13 — `trace`.** `vibe-spec trace [--since <ref>] [--json]`:
  - **Units:** canonical `### <f>/<n>` headings in fallback `features/<f>/plan.md`. Superpowers plans carry no unit ids and are not traced.
  - **Citations:** word-bounded `<name>/<n>` tokens in commit messages (`git log`, or `<ref>..HEAD` with `--since`) and in files under `<project>/tests/`. A token counts only when `<name>` is an existing feature folder.
  - **Orphan:** a citation of an existing feature whose unit does not exist.
  - **Exit:** 0 when everything is cited and nothing is orphaned, 1 otherwise, 2 for usage or a bad ref, 3 when no root is found.
  - **Without git history:** commits are skipped and stderr says so.
  - Cost: a citation of a deleted feature is never flagged.
- **P3b-R14 — Baseline.** `bash tests/run.sh` was green at `c8c326d`, except that the engine `doctor` perf budget fails intermittently on this machine (P3a-R10). Judge each task by the delta. Cost: none.
- **P3b-R15 — Gate honesty.** Row F17 stays `IN PROGRESS — local gate green; CI pending push` until the owner approves a push and CI is green. Cost: none.
- **P3b-R16 — Scope fix (bug c).**
  - The section starts at a line that is exactly `## Scope`, trailing blanks allowed. It ends at the next line starting with `# ` or `## `.
  - "Owns" is the word `owns`, case-insensitive and word-bounded, inside that section.
  - "Does not own" is the phrase `does not own`, case-insensitive and word-bounded, inside that section.
  - The messages stay the same.
  - Cost: `## Scopes` or `## Scope and limits` no longer counts as the section.
- **P3b-R17 — RFC 2119 fix (bug b).** `SHALL`, `MUST`, `SHOULD` and `MAY` count only in capitals and as whole words (bounded by non-`[A-Za-z0-9_]`). Cost: a lowercase "must" no longer satisfies SF10, as RFC 2119 intends.
- **P3b-R18 — WHEN/THEN.** A scenario block runs from `#### Scenario:` to the next line starting with `#`. Fenced lines are skipped. A WHEN (THEN) bullet is a line starting with `- ` or `* ` whose first word, after stripping leading `*`/`_`, is `when` (`then`), case-insensitive. Cost: a scenario written as prose fails SF25.
- **P3b-R19 — OpenSpec samples.** Hand-written to OpenSpec's published grammar and kept as test fixtures: `### Requirement:`, a `SHALL`/`MUST` sentence, `#### Scenario:`, and `- **WHEN** … / - **THEN** …` bullets. No network fetch. Cost: drift from upstream goes unnoticed until F18's export round-trip.
- **P3b-R20 — SF28 direction.** Folder → row only. A plan row with no folder is drift, owned by `check-drift` (F18). Cost: none.
- **P3b-R21 — SF29 date.**
  - The comparison uses the committer date (`%cs`) of the last commit touching that path, read in one `git log --name-only` pass.
  - It is silent when `git` is missing, the project has no commits, the file is uncommitted, or the root lies outside the repository.
  - Cost: a rebase that rewrites committer dates can flag files until `updated:` is bumped.

## F17 acceptance → tasks

| Acceptance (#34, plan.md row F17) | Where |
|---|---|
| one landed mutant per fixed bug (R-ID prefix, RFC-2119 boundary, Scope scoping, zero-requirement short-circuit) makes its test fail | Task 5 (three mutants), Task 6 (SF24 mutant) |
| warn/error/off strictness matrix, all three states tested per check | Task 3 harness; each check task adds its trigger; Task 13 asserts nothing is pending |
| OpenSpec sample specs pass grammar checks; mutated samples fail | Task 6 |
| synthetic repo fixture with cited and uncited units exercises `trace` | Task 12 |
| layered fixture tree passes, each mutated rule fails | Task 10 |
| `bash tests/run.sh` green, `cargo test --workspace` green | Task 13 gate (R14 baseline) |

## File Structure

```text
crates/vibe-spec/src/check.rs          # registry, Level, Levels, config loader          (Tasks 1, 2)
crates/vibe-spec/src/report.rs         # Report, Finding, text + JSON rendering, markers (Tasks 1–3)
crates/vibe-spec/src/validate.rs       # driver: legacy loop + F17 passes, flags        (Tasks 1–3, 6–11)
crates/vibe-spec/src/content.rs        # fixes (SF8, SF10, SF12), SF24, SF25            (Tasks 1, 5, 6)
crates/vibe-spec/src/pattern.rs        # rfc_keyword, word helpers, unit_citations      (Tasks 5, 12)
crates/vibe-spec/src/frontmatter.rs    # tiny YAML-subset frontmatter reader            (Task 7)
crates/vibe-spec/src/adherence.rs      # SF26, SF27, SF28, SF29                         (Tasks 7, 8)
crates/vibe-spec/src/git.rs            # last-commit dates, commit messages             (Tasks 8, 12)
crates/vibe-spec/src/layered.rs        # layout detection, SF22 tree links, SF30–SF34   (Tasks 9, 10)
crates/vibe-spec/src/superpowers.rs    # SF35, SF36                                     (Task 11)
crates/vibe-spec/src/trace.rs          # `trace`                                        (Task 12)
crates/vibe-spec/src/main.rs           # clap: validate flags, trace                    (Tasks 3, 12)
crates/vibe-spec/src/root.rs           # `root --json` layout                           (Task 9)
crates/vibe-spec/tests/common/mod.rs   # legacy config in parity, json/git helpers      (Tasks 3, 4, 8)
crates/vibe-spec/tests/{strictness,grammar,adherence,layered,superpowers,trace}.rs
crates/vibe-spec/tests/validate.rs     # parity tests keep passing                       (Tasks 3–5)
crates/vibe-parity/src/corpus_spec.rs  # .vibe/spec.json in every case; fix cases       (Tasks 4, 5)
tests/parity/legacy-strictness.json    # the F17-added checks, all off                  (Task 4)
tests/parity/spec.json                 # pins, floors                                    (Task 5)
tests/parity/run.sh                    # live validate retired                           (Task 4)
contracts/spec-json.schema.json        # validate, finding, trace                       (Tasks 3, 12)
contracts/run.mjs, contracts/fixtures/spec-json/{validate,trace}/{valid,invalid}/
spec/reference/product.md              # R-1 → R1                                        (Task 5)
docs/superpowers/plans/2026-10-13-spec-v04.md, .spec/{tech-spec,tech-contracts,tech-rust,plan,lessons}.md (Task 13)
```

---

### Task 1: Check registry and findings report (behaviour-neutral)

**Files:**
- Create: `crates/vibe-spec/src/check.rs`, `crates/vibe-spec/src/report.rs`
- Modify: `crates/vibe-spec/src/main.rs` (module list), `crates/vibe-spec/src/validate.rs` (the `Report` moves out; every `red`/`yellow` call names a check), `crates/vibe-spec/src/content.rs` (same)
- Test: unit tests in `check.rs` and `report.rs`; the existing `crates/vibe-spec/tests/validate.rs` and `bash tests/parity/run.sh` must stay green unchanged

**Interfaces:**
- Consumes: `text::echo_e`, the `cat!` macro (`main.rs`).
- Produces (later tasks rely on these exact names):
  - `check::{Origin, Check, CHECKS, Level, Levels, normalize}`
    - `Level::{Off, Warn, Error}`, `Level::parse(&str) -> Option<Level>`, `Level::as_str(self) -> &'static str`
    - `Levels::new(BTreeMap<&'static str, Level>, strict: bool) -> Levels`
    - `Levels::strict(&self) -> bool`
    - `Levels::resolve(&self, check: &str, default: Level) -> Level`
    - `Levels::describe(&self, check: &str) -> &'static str`
    - `Levels::all_off(&self) -> bool`
    - `normalize(key: &str) -> Option<&'static str>`
  - `report::{Severity, Finding, Report}`
    - `Report::new(project: &Path, levels: Levels) -> Report`
    - `at(&mut self, file: &str)`, `examine(&mut self, file: &str)`, `examined(&self) -> usize`, `levels(&self) -> &Levels`
    - `error`/`warn`/`warn_uncounted(&mut self, check: &'static str, msg: &[u8])`
    - `refuse(&mut self, msg: &[u8])`, `green`/`info`/`line(&mut self, msg: &[u8])`
    - Public fields: `out: Vec<u8>`, `errors: u32`, `warnings: u32`, `findings: Vec<Finding>`

`Finding.file` is the path relative to the project, `/`-joined: `.spec/features/auth/plan.md`, `CLAUDE.md`. It is absolute when the root lies outside the project, and `""` for tree-level findings. The driver builds it as `format!("{display}/{rel}")`.

- [ ] **Step 1: Write the failing unit tests**

Create `crates/vibe-spec/src/check.rs` with only its test module first (the types do not exist yet, so it fails to compile):

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn ids_are_unique_and_partitioned() {
        let mut seen = std::collections::BTreeSet::new();
        for c in CHECKS {
            assert!(seen.insert(c.id), "duplicate {}", c.id);
        }
        let legacy: Vec<&str> = CHECKS.iter().filter(|c| c.origin == Origin::Legacy).map(|c| c.id).collect();
        assert_eq!(
            legacy,
            ["SF3", "SF4", "SF8", "SF9", "SF10", "SF11", "SF12", "SF13", "SF14", "SF15", "SF16", "I1", "SF20", "SF21", "SF22", "SF23"]
        );
        let new: Vec<&str> = CHECKS.iter().filter(|c| c.origin == Origin::New).map(|c| c.id).collect();
        assert_eq!(
            new,
            ["SF24", "SF25", "SF26", "SF27", "SF28", "SF29", "SF30", "SF31", "SF32", "SF33", "SF34", "SF35", "SF36"]
        );
    }

    #[test]
    fn keys_normalize_to_registered_ids() {
        assert_eq!(normalize("SF-12"), Some("SF12"));
        assert_eq!(normalize("sf12"), Some("SF12"));
        assert_eq!(normalize("SF12"), Some("SF12"));
        assert_eq!(normalize("i1"), Some("I1"));
        assert_eq!(normalize("SF-1"), None);
        assert_eq!(normalize("SF99"), None);
        assert_eq!(normalize(""), None);
    }

    #[test]
    fn levels_resolve_config_then_default_then_strict() {
        let mut set = BTreeMap::new();
        set.insert("SF12", Level::Off);
        set.insert("SF13", Level::Error);
        set.insert("SF20", Level::Warn);
        let plain = Levels::new(set.clone(), false);
        assert_eq!(plain.resolve("SF12", Level::Warn), Level::Off);
        assert_eq!(plain.resolve("SF13", Level::Warn), Level::Error);
        assert_eq!(plain.resolve("SF20", Level::Error), Level::Warn);
        assert_eq!(plain.resolve("SF14", Level::Warn), Level::Warn);
        assert_eq!(plain.resolve("SF14", Level::Error), Level::Error);
        let strict = Levels::new(set, true);
        assert_eq!(strict.resolve("SF12", Level::Warn), Level::Off);
        assert_eq!(strict.resolve("SF14", Level::Warn), Level::Error);
        assert_eq!(strict.resolve("SF20", Level::Error), Level::Error);
        assert_eq!(plain.describe("SF12"), "off");
        assert_eq!(plain.describe("SF14"), "default");
        assert_eq!(strict.describe("SF14"), "error");
        assert_eq!(strict.describe("SF12"), "off");
        assert!(!plain.all_off());
        let all: BTreeMap<&'static str, Level> = CHECKS.iter().map(|c| (c.id, Level::Off)).collect();
        assert!(Levels::new(all, false).all_off());
    }

    #[test]
    fn level_names_round_trip() {
        for l in [Level::Off, Level::Warn, Level::Error] {
            assert_eq!(Level::parse(l.as_str()), Some(l));
        }
        assert_eq!(Level::parse("warning"), None);
    }
}
```

Create `crates/vibe-spec/src/report.rs` with only its tests:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::check::{Level, Levels};
    use std::collections::BTreeMap;
    use std::path::Path;

    fn report(set: &[(&'static str, Level)], strict: bool) -> Report {
        Report::new(Path::new("/nonexistent"), Levels::new(set.iter().copied().collect::<BTreeMap<_, _>>(), strict))
    }

    #[test]
    fn default_levels_render_like_f16() {
        let mut r = report(&[], false);
        r.warn("SF13", b"w");
        r.error("SF22", b"e");
        r.warn_uncounted("SF11", b"u");
        r.green(b"g");
        r.info(b"i");
        assert_eq!(
            r.out,
            b"\x1b[33m  WARN:  w\x1b[0m\n\x1b[31m  ERROR: e\x1b[0m\n\x1b[33m  WARN:  u\x1b[0m\n\x1b[32m  OK:    g\x1b[0m\n\x1b[36m  INFO:  i\x1b[0m\n"
        );
        assert_eq!((r.errors, r.warnings), (1, 1));
        assert_eq!(r.findings.len(), 3);
        assert_eq!(r.findings[1].severity, Severity::Error);
        assert_eq!(r.findings[1].check, "SF22");
    }

    #[test]
    fn off_drops_error_promotes_strict_counts_uncounted() {
        let mut r = report(&[("SF13", Level::Off), ("SF14", Level::Error)], false);
        r.warn("SF13", b"gone");
        r.warn("SF14", b"up");
        assert_eq!(r.out, b"\x1b[31m  ERROR: up\x1b[0m\n");
        assert_eq!((r.errors, r.warnings, r.findings.len()), (1, 0, 1));
        let mut s = report(&[], true);
        s.warn_uncounted("SF11", b"u");
        assert_eq!((s.errors, s.warnings), (1, 0));
    }

    #[test]
    fn examined_counts_distinct_files() {
        let mut r = report(&[], false);
        r.examine(".spec/a.md");
        r.examine(".spec/a.md");
        r.at(".spec/b.md");
        r.examine("");
        r.examine(".spec/c.md");
        assert_eq!(r.examined(), 2);
    }

    #[test]
    fn refuse_ignores_levels() {
        let all: Vec<(&'static str, Level)> = crate::check::CHECKS.iter().map(|c| (c.id, Level::Off)).collect();
        let mut r = report(&all, true);
        r.refuse(b"no");
        assert_eq!((r.errors, r.findings.len()), (1, 1));
        assert_eq!(r.findings[0].check, "SF23");
    }
}
```

Add `mod check;` and `mod report;` to `main.rs` (alphabetical, after `mod content;`/`mod pattern;`).

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test --locked -p vibe-spec --bins 2>&1 | tail -5`
Expected: compile errors such as `cannot find value \`CHECKS\` in this scope` and `cannot find type \`Report\``. Paste them in the report.

- [ ] **Step 3: Implement `check.rs`**

Put this above the test module:

```rust
//! The validate check registry (P3b-R4). Every finding names one check;
//! its level comes from `.vibe/spec.json` `strictness`, else the finding's
//! own default (P3b-R5).
use std::collections::BTreeMap;

/// Who introduced a check.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Origin {
    /// In the frozen oracle: numbered there, or a family it left unnumbered.
    Legacy,
    /// Added by F17; `off` in the parity corpus (P3b-R2).
    New,
}

/// One registered check.
#[derive(Debug)]
pub struct Check {
    /// `SF12`, `I1`.
    pub id: &'static str,
    /// Who introduced it.
    pub origin: Origin,
    /// What it checks.
    pub title: &'static str,
}

const fn legacy(id: &'static str, title: &'static str) -> Check {
    Check { id, origin: Origin::Legacy, title }
}

const fn new(id: &'static str, title: &'static str) -> Check {
    Check { id, origin: Origin::New, title }
}

/// Every check, legacy first, in id order within each origin.
pub const CHECKS: &[Check] = &[
    legacy("SF3", "empty design token group"),
    legacy("SF4", "design.md lint"),
    legacy("SF8", "feature Scope section"),
    legacy("SF9", "feature frontmatter fields"),
    legacy("SF10", "requirement keyword and scenario"),
    legacy("SF11", "plan structure"),
    legacy("SF12", "unit ids and R-ID trace"),
    legacy("SF13", "stale features/archive link"),
    legacy("SF14", "scope conflict"),
    legacy("SF15", "root spec length"),
    legacy("SF16", "lesson tags"),
    legacy("I1", "cross-feature unit dependency"),
    legacy("SF20", "frontmatter presence"),
    legacy("SF21", "file naming"),
    legacy("SF22", "link resolution"),
    legacy("SF23", "spec tree and required files"),
    new("SF24", "requirement population"),
    new("SF25", "scenario WHEN/THEN"),
    new("SF26", "frontmatter values"),
    new("SF27", "children and backlinks"),
    new("SF28", "feature folder has a plan row"),
    new("SF29", "updated: not older than the last commit"),
    new("SF30", "layout allowlist"),
    new("SF31", "line budget"),
    new("SF32", "kebab and unique names"),
    new("SF33", "typed notes"),
    new("SF34", "reachable from a hub"),
    new("SF35", "superpowers artifact name"),
    new("SF36", "superpowers plan Spec link"),
];

/// A check's level after configuration.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Level {
    /// Not reported, not counted.
    Off,
    /// A warning.
    Warn,
    /// An error (exit 1).
    Error,
}

impl Level {
    /// `off`, `warn`, `error`.
    pub fn parse(s: &str) -> Option<Level> {
        match s {
            "off" => Some(Level::Off),
            "warn" => Some(Level::Warn),
            "error" => Some(Level::Error),
            _ => None,
        }
    }

    /// The config spelling.
    pub fn as_str(self) -> &'static str {
        match self {
            Level::Off => "off",
            Level::Warn => "warn",
            Level::Error => "error",
        }
    }
}

/// `SF-12`, `sf12`, `SF12` → `SF12`; `i1` → `I1`; unregistered → `None`.
pub fn normalize(key: &str) -> Option<&'static str> {
    let up: String = key.chars().filter(|&c| c != '-').collect::<String>().to_ascii_uppercase();
    CHECKS.iter().find(|c| c.id == up).map(|c| c.id)
}

/// Configured levels plus `--strict`.
#[derive(Clone, Debug, Default)]
pub struct Levels {
    set: BTreeMap<&'static str, Level>,
    strict: bool,
}

impl Levels {
    /// Levels from a config map and the `--strict` flag.
    pub fn new(set: BTreeMap<&'static str, Level>, strict: bool) -> Levels {
        Levels { set, strict }
    }

    /// `--strict` was given.
    pub fn strict(&self) -> bool {
        self.strict
    }

    /// The level of one finding of `check` whose own default is `default`.
    pub fn resolve(&self, check: &str, default: Level) -> Level {
        let level = self.set.get(check).copied().unwrap_or(default);
        if self.strict && level == Level::Warn { Level::Error } else { level }
    }

    /// What `--format json` reports for `check`: `off`, `warn`, `error`, or
    /// `default` when unconfigured; under `--strict` every non-off is `error`.
    pub fn describe(&self, check: &str) -> &'static str {
        match (self.set.get(check), self.strict) {
            (Some(Level::Off), _) => "off",
            (_, true) => "error",
            (Some(l), false) => l.as_str(),
            (None, false) => "default",
        }
    }

    /// Every registered check is configured `off`.
    pub fn all_off(&self) -> bool {
        CHECKS.iter().all(|c| self.set.get(c.id) == Some(&Level::Off))
    }
}
```

- [ ] **Step 4: Implement `report.rs`**

Put this above its test module:

```rust
//! validate's findings and their text rendering. Text output is the F16
//! byte format; each finding also keeps its check, severity and file.
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use crate::check::{Level, Levels};
use crate::text::echo_e;

/// A finding's severity after strictness.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Severity {
    /// Counted in `Warnings:`.
    Warning,
    /// Counted in `Errors:`; exit 1.
    Error,
}

impl Severity {
    /// The Spec JSON spelling.
    pub fn as_str(self) -> &'static str {
        match self {
            Severity::Warning => "warning",
            Severity::Error => "error",
        }
    }
}

/// One reported finding.
#[derive(Debug)]
pub struct Finding {
    /// The registered check id.
    pub check: &'static str,
    /// After strictness.
    pub severity: Severity,
    /// Project-relative, `/`-joined; absolute outside the project; `""` for the tree.
    pub file: String,
    /// The text-mode message without colour or tag.
    pub message: Vec<u8>,
}

/// validate's stdout, counters and findings.
pub struct Report {
    /// Everything printed so far (text mode).
    pub out: Vec<u8>,
    /// `Errors:`.
    pub errors: u32,
    /// `Warnings:`.
    pub warnings: u32,
    /// Every finding not dropped by `off`.
    pub findings: Vec<Finding>,
    levels: Levels,
    project: PathBuf,
    file: String,
    examined: BTreeSet<String>,
}

impl Report {
    /// An empty report for a project.
    pub fn new(project: &Path, levels: Levels) -> Report {
        Report {
            out: Vec::new(),
            errors: 0,
            warnings: 0,
            findings: Vec::new(),
            levels,
            project: project.to_path_buf(),
            file: String::new(),
            examined: BTreeSet::new(),
        }
    }

    /// The levels in force.
    pub fn levels(&self) -> &Levels {
        &self.levels
    }

    /// Later findings belong to `file` (see [`Finding::file`]).
    pub fn at(&mut self, file: &str) {
        self.file = file.to_string();
    }

    /// Like [`Report::at`], and counts `file` as examined when non-empty.
    pub fn examine(&mut self, file: &str) {
        self.at(file);
        if !file.is_empty() {
            self.examined.insert(file.to_string());
        }
    }

    /// Distinct files examined.
    pub fn examined(&self) -> usize {
        self.examined.len()
    }

    /// A finding whose default is error.
    pub fn error(&mut self, check: &'static str, msg: &[u8]) {
        self.push(check, Level::Error, true, msg);
    }

    /// A finding whose default is warning.
    pub fn warn(&mut self, check: &'static str, msg: &[u8]) {
        self.push(check, Level::Warn, true, msg);
    }

    /// A warning the oracle prints from a pipeline subshell and so never
    /// counts; it counts once it resolves to error.
    pub fn warn_uncounted(&mut self, check: &'static str, msg: &[u8]) {
        self.push(check, Level::Warn, false, msg);
    }

    /// An error no configuration can silence (`--strict` refusal).
    pub fn refuse(&mut self, msg: &[u8]) {
        self.colour("31", "ERROR: ", msg);
        self.errors += 1;
        self.record("SF23", Severity::Error, msg);
    }

    fn push(&mut self, check: &'static str, default: Level, counted: bool, msg: &[u8]) {
        match self.levels.resolve(check, default) {
            Level::Off => {}
            Level::Warn => {
                self.colour("33", "WARN:  ", msg);
                if counted {
                    self.warnings += 1;
                }
                self.record(check, Severity::Warning, msg);
            }
            Level::Error => {
                self.colour("31", "ERROR: ", msg);
                self.errors += 1;
                self.record(check, Severity::Error, msg);
            }
        }
    }

    fn record(&mut self, check: &'static str, severity: Severity, msg: &[u8]) {
        self.findings.push(Finding {
            check,
            severity,
            file: self.file.clone(),
            message: msg.to_vec(),
        });
    }

    fn colour(&mut self, code: &str, tag: &str, msg: &[u8]) {
        self.out
            .extend(echo_e(&cat!("\\033[", code, "m  ", tag, msg, "\\033[0m")));
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
```

- [ ] **Step 5: Route every legacy finding through the registry**

In `validate.rs`:
- Delete `struct Report` and its `impl`. Import `use crate::check::Levels;` and `use crate::report::Report;`.
- Create the report with `let mut r = Report::new(&project, Levels::default());`.
- Pass `ctx` to `children` (signature `fn children(r: &mut Report, ctx: &Ctx, name: &str)`; use `ctx.dir`).

Replace each call exactly as follows. The message bytes stay the same.

| location (current text) | new call |
|---|---|
| `run`: `No spec root found…`, `No spec files found…` | `r.error("SF23", …)` |
| `root_file`: `missing YAML frontmatter`, `missing 'type:'`, `missing 'updated:'`, `entrypoint missing 'children:'`, `branch doc missing 'parent:'` | `r.error("SF20", …)` |
| `root_file`: `entrypoint has no 'children:' list`, `branch doc missing 'scope:'`/`'covers:'` | `r.warn("SF20", …)` |
| `root_file`: `must start with 'product-'…`, `topic must be lowercase…` | `r.error("SF21", …)` |
| `root_file`, `feature`, `claude_md`: `broken link to` | `r.error("SF22", …)` |
| `children`: `listed in frontmatter but file doesn't exist` | `r.error("SF22", …)` |
| `feature`: `missing required` | `r.error("SF23", …)` |
| `feature_doc`: `missing YAML frontmatter`, `missing 'type:'`, `missing 'updated:'` | `r.error("SF20", …)` |
| `feature_doc`: `expected 'type: feature-…'` | `r.warn("SF20", …)` |
| `sf13`, `sf15`, `sf16` | `r.warn("SF13"/"SF15"/"SF16", …)` |
| `lint_design` (every `yellow`) | `r.warn("SF4", …)` |

Set the file before each finding, with paths built from `ctx.display` (`dsp` below):
- **`run`**: `r.at("")` before the two SF23 errors.
- **`root_file`**: `r.examine(&format!("{dsp}/{name}"))` right after the `--- name ---` line. Lessons included.
- **`children`**: `r.at(&format!("{dsp}/{name}"))`.
- **`feature`**: for a missing required file, `r.at(&format!("{dsp}/{label}/{required}"))`. Before reading a present doc, `r.examine(&format!("{dsp}/{label}/{file}"))`.
- **`claude_md`**: `r.examine("CLAUDE.md")`.
- **`sf13`, `sf15`**: `r.at(&format!("{display}/{name}"))` per file.
- **`sf16`**: `r.at(&format!("{display}/lessons.md"))`.
- **`content::sf14`**: needs the display, so it becomes `pub fn sf14(r: &mut Report, dir: &Path, display: &str)`. It calls `r.at(&format!("{display}/features/{feature}/product.md"))` per feature before reading its items.

In `content.rs`, replace `use crate::validate::Report;` with `use crate::report::Report;` and map the calls:
- `design_tokens`: `r.warn("SF3", …)`
- `feature_scope`: `r.warn("SF8", …)`
- `feature_frontmatter`: `r.warn("SF9", …)`
- `requirements_scenarios`: `r.warn("SF10", …)`, the hint included
- `plan_structure`: `r.warn("SF11", …)`
- `root_plan_milestones`: `r.warn_uncounted("SF11", …)`
- `id_traceability`: `r.warn("SF12", …)`; the INFO line stays `r.info`
- `cross_deps`: `r.warn("I1", …)`
- `sf14`: `r.warn("SF14", …)`

- [ ] **Step 6: Run the unit tests, the validate tests and parity**

Run, one at a time:

```bash
cargo test --locked -p vibe-spec 2>&1 | grep -E 'test result|FAILED|panicked'
cargo clippy --workspace --all-targets --locked -- -D warnings 2>&1 | tail -1
cargo fmt --all --check && echo fmt-ok
bash tests/parity/run.sh 2>&1 | tail -4
```

Expected:
- Every `test result: ok`.
- clippy reports no issues, and `fmt-ok` prints.
- Parity: `0 failed, 0 error(s)` for the self-check and the candidate, then five `PASS  live` lines. This task changes no output.

- [ ] **Step 7: Commit**

```bash
git add crates/vibe-spec/src
git commit -m "refactor(spec): route findings through a registry"
```

### Task 2: Strictness config, `--strict`, allow markers, zero-input refusal

**Files:**
- Modify: `crates/vibe-spec/src/check.rs` (add `load`), `crates/vibe-spec/src/report.rs` (allow markers), `crates/vibe-spec/src/text.rs` (add `find`), `crates/vibe-spec/src/validate.rs` (`Options`, load levels, refusal), `crates/vibe-spec/src/main.rs` (clap flags for `validate`)
- Test: `crates/vibe-spec/tests/strictness.rs` (new); `crates/vibe-spec/tests/validate.rs` (replace `arguments_are_a_usage_error_until_f17`)

**Interfaces:**
- Consumes: Task 1's `check` and `report`.
- Produces:
  - `check::load(project: &Path, strict: bool) -> Result<Levels, String>`
  - `report::allow_markers(text: &[u8]) -> Vec<&'static str>`
  - `text::find(hay: &[u8], needle: &[u8]) -> Option<usize>`
  - `validate::Options { pub strict: bool }` (Task 3 adds `format`)
  - `validate::run(found: &Result<Root, Miss>, cwd: &Path, opts: &Options) -> (u8, Vec<u8>)`
  - clap `Validate { strict: bool }`

- [ ] **Step 1: Write the failing tests**

Create `crates/vibe-spec/tests/strictness.rs`:

```rust
#![cfg(unix)]
#![allow(clippy::unwrap_used, clippy::expect_used)]
//! `.vibe/spec.json` strictness, `--strict`, allow markers (P3b-R5–R8).
mod common;

use common::{project, spec, write};

const ENTRY: &str = "---\ntype: entrypoint\nscope: product\nchildren:\nupdated: 2026-01-15\n---\n\n# P\n\n[f](features/ghost/)\n";

/// A tree whose only finding is one SF13 warning (a stale `features/ghost/` link).
fn sf13_tree(name: &str) -> std::path::PathBuf {
    let p = project(name);
    write(&p, ".spec/product.md", ENTRY);
    p
}

#[test]
fn a_legacy_check_can_be_turned_off_or_up() {
    let p = sf13_tree("str-legacy");
    let base = spec(&p, &["validate"], &[]);
    assert_eq!(base.code, 0, "{}", base.text());
    assert!(base.text().contains("WARN:  SF13: stale link"), "{}", base.text());
    assert!(base.text().ends_with("Errors:   0\nWarnings: 1\n"));

    write(&p, ".vibe/spec.json", r#"{"strictness": {"SF-13": "off"}}"#);
    let off = spec(&p, &["validate"], &[]);
    assert_eq!(off.code, 0);
    let without: String = base.text().lines().filter(|l| !l.contains("SF13")).map(|l| format!("{l}\n")).collect();
    assert_eq!(off.text(), without.replace("Warnings: 1", "Warnings: 0"));

    write(&p, ".vibe/spec.json", r#"{"strictness": {"sf13": "error"}}"#);
    let up = spec(&p, &["validate"], &[]);
    assert_eq!(up.code, 1);
    assert!(up.text().contains("\x1b[31m  ERROR: SF13: stale link"), "{}", up.text());
    assert!(up.text().ends_with("Errors:   1\nWarnings: 0\n"));
}

#[test]
fn strict_promotes_warnings_and_keeps_off() {
    let p = sf13_tree("str-strict");
    let strict = spec(&p, &["validate", "--strict"], &[]);
    assert_eq!(strict.code, 1);
    assert!(strict.text().contains("ERROR: SF13"), "{}", strict.text());
    write(&p, ".vibe/spec.json", r#"{"strictness": {"SF13": "off"}}"#);
    let quiet = spec(&p, &["validate", "--strict"], &[]);
    assert_eq!(quiet.code, 0, "{}", quiet.text());
}

#[test]
fn an_allow_marker_beats_strict() {
    let p = sf13_tree("str-marker");
    write(&p, ".spec/product.md", &format!("{ENTRY}\n<!-- vibe-spec: allow SF-13 -->\n"));
    write(&p, ".vibe/spec.json", r#"{"strictness": {"SF13": "error"}}"#);
    let out = spec(&p, &["validate", "--strict"], &[]);
    assert_eq!(out.code, 0, "{}", out.text());
    assert!(!out.text().contains("SF13"), "{}", out.text());
}

#[test]
fn a_marker_only_covers_its_own_file() {
    let p = sf13_tree("str-marker-scope");
    write(&p, ".spec/tech.md", "---\ntype: entrypoint\nscope: technical\nchildren:\nupdated: 2026-01-15\n---\n\n<!-- vibe-spec: allow SF13 -->\n");
    let out = spec(&p, &["validate"], &[]);
    assert!(out.text().contains("SF13: stale link in .spec/product.md"), "{}", out.text());
}

#[test]
fn strict_refuses_when_every_check_is_off() {
    let p = sf13_tree("str-all-off");
    let all = r#"{"strictness": {"SF3":"off","SF4":"off","SF8":"off","SF9":"off","SF10":"off","SF11":"off","SF12":"off","SF13":"off","SF14":"off","SF15":"off","SF16":"off","I1":"off","SF20":"off","SF21":"off","SF22":"off","SF23":"off","SF24":"off","SF25":"off","SF26":"off","SF27":"off","SF28":"off","SF29":"off","SF30":"off","SF31":"off","SF32":"off","SF33":"off","SF34":"off","SF35":"off","SF36":"off"}}"#;
    write(&p, ".vibe/spec.json", all);
    let plain = spec(&p, &["validate"], &[]);
    assert_eq!(plain.code, 0);
    let strict = spec(&p, &["validate", "--strict"], &[]);
    assert_eq!(strict.code, 1, "{}", strict.text());
    assert!(strict.text().contains("ERROR: validate --strict examined nothing"), "{}", strict.text());
}

#[test]
fn bad_config_values_are_usage_errors() {
    let p = sf13_tree("str-bad");
    for bad in [
        r#"{"strictness": {"SF13": "loud"}}"#,
        r#"{"strictness": ["SF13"]}"#,
        r#"{"strictness": {"SF13": 1}}"#,
        "not json",
    ] {
        write(&p, ".vibe/spec.json", bad);
        let out = spec(&p, &["validate"], &[]);
        assert_eq!(out.code, 2, "{bad}: {}", out.text());
        assert!(out.stdout.is_empty(), "{bad}");
    }
}

#[test]
fn an_unknown_check_warns_and_is_ignored() {
    let p = sf13_tree("str-unknown");
    write(&p, ".vibe/spec.json", r#"{"strictness": {"SF99": "off", "SF13": "off"}}"#);
    let out = spec(&p, &["validate"], &[]);
    assert_eq!(out.code, 0);
    assert!(out.stderr.contains("unknown check 'SF99'"), "{}", out.stderr);
    assert!(!out.text().contains("SF13"));
}

#[test]
fn an_explicit_root_still_reads_the_project_config() {
    let p = sf13_tree("str-explicit");
    write(&p, ".vibe/spec.json", r#"{"strictness": {"SF13": "off"}}"#);
    let out = spec(&p, &["--root", ".spec", "validate"], &[]);
    assert!(!out.text().contains("SF13"), "{}", out.text());
}
```

Note: `"not json"` already exits 2 today, through discovery's `BadConfig`, because the walk reads the config at the project directory. The other three fail until `load` exists.

In `crates/vibe-spec/tests/validate.rs`, replace `arguments_are_a_usage_error_until_f17` with:

```rust
#[test]
fn unknown_arguments_are_a_usage_error() {
    let p = project("val-args");
    write(&p, ".spec/product.md", ENTRY);
    assert_eq!(common::spec(&p, &["validate", "--bogus"], &[]).code, 2);
    assert_eq!(common::spec(&p, &["validate", "extra"], &[]).code, 2);
    assert_ne!(common::spec(&p, &["validate", "--strict"], &[]).code, 2);
}
```

- [ ] **Step 2: Run the tests to see them fail**

Run: `cargo test --locked -p vibe-spec --test strictness --test validate 2>&1 | grep -E '^test |test result'`
Expected: every strictness test FAILS except possibly `bad_config_values_are_usage_errors` ("not json" passes, `"loud"` gives exit 0). `unknown_arguments_are_a_usage_error` FAILS on `--strict` (exit 2). Paste the output.

- [ ] **Step 3: Add `text::find`**

```rust
/// The index of the first `needle` in `hay`.
pub fn find(hay: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() {
        return Some(0);
    }
    hay.windows(needle.len()).position(|w| w == needle)
}
```

- [ ] **Step 4: Add `check::load`**

```rust
use std::path::Path;

use serde_json::Value;

/// `<project>/.vibe/spec.json` `strictness` (P3b-R6). No file or no key is
/// an empty set; an unknown check id warns on stderr and is ignored.
pub fn load(project: &Path, strict: bool) -> Result<Levels, String> {
    let path = project.join(".vibe").join("spec.json");
    if !path.is_file() {
        return Ok(Levels::new(BTreeMap::new(), strict));
    }
    let shown = path.display();
    let text = std::fs::read_to_string(&path).map_err(|e| format!("{shown}: {e}"))?;
    let value: Value = serde_json::from_str(&text).map_err(|e| format!("{shown}: not JSON: {e}"))?;
    let Value::Object(map) = value else {
        return Err(format!("{shown}: not a JSON object"));
    };
    let mut set = BTreeMap::new();
    match map.get("strictness") {
        None => {}
        Some(Value::Object(entries)) => {
            for (key, v) in entries {
                let level = v.as_str().and_then(Level::parse).ok_or_else(|| {
                    format!("{shown}: strictness.{key} must be \"warn\", \"error\" or \"off\"")
                })?;
                match normalize(key) {
                    Some(id) => {
                        set.insert(id, level);
                    }
                    None => eprintln!("vibe-spec: {shown}: unknown check '{key}' in strictness (ignored)"),
                }
            }
        }
        Some(_) => return Err(format!("{shown}: `strictness` must be an object")),
    }
    Ok(Levels::new(set, strict))
}
```

- [ ] **Step 5: Allow markers in `report.rs`**

Add `markers: BTreeMap<String, Vec<&'static str>>` to `Report` (initialized empty), and drop allowed findings at the top of `push`:

```rust
    fn push(&mut self, check: &'static str, default: Level, counted: bool, msg: &[u8]) {
        if self.allowed(check) {
            return;
        }
        // ... unchanged
    }

    /// The current file holds `<!-- vibe-spec: allow <check> -->` (P3b-R8).
    fn allowed(&mut self, check: &str) -> bool {
        if self.file.is_empty() {
            return false;
        }
        if !self.markers.contains_key(&self.file) {
            let path = if self.file.starts_with('/') {
                PathBuf::from(&self.file)
            } else {
                self.project.join(&self.file)
            };
            let ids = std::fs::read(path).map(|t| allow_markers(&t)).unwrap_or_default();
            self.markers.insert(self.file.clone(), ids);
        }
        self.markers.get(&self.file).is_some_and(|ids| ids.contains(&check))
    }
```

and the free function:

```rust
/// The checks named by `<!-- vibe-spec: allow SF-nn -->` markers in `text`.
pub fn allow_markers(text: &[u8]) -> Vec<&'static str> {
    const OPEN: &[u8] = b"<!-- vibe-spec: allow ";
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(i) = find(rest, OPEN) {
        let after = rest.get(i + OPEN.len()..).unwrap_or_default();
        let Some(end) = find(after, b"-->") else {
            break;
        };
        let key = trim_space(after.get(..end).unwrap_or_default());
        if let Some(id) = std::str::from_utf8(key).ok().and_then(normalize) {
            out.push(id);
        }
        rest = after.get(end + 3..).unwrap_or_default();
    }
    out
}
```

(imports: `use std::collections::BTreeMap;`, `use crate::check::normalize;`, `use crate::text::{find, trim_space};`). Add a unit test:

```rust
    #[test]
    fn markers_name_checks() {
        assert_eq!(
            allow_markers(b"a <!-- vibe-spec: allow SF-13 --> b <!-- vibe-spec: allow i1--> <!-- vibe-spec: allow SF99 --> <!-- vibe-spec: allow SF12"),
            vec!["SF13", "I1"]
        );
    }
```

- [ ] **Step 6: Wire `--strict`, the config and the refusal**

In `main.rs`, the `Validate` variant becomes:

```rust
    /// Validate the spec tree (port of validate.sh, plus F17's checks).
    Validate {
        /// Promote every non-off check to error; refuse a run that examined nothing.
        #[arg(long)]
        strict: bool,
    },
```

The dispatch becomes `Command::Validate { strict } => validate::run(&found, &cwd, &validate::Options { strict })`. Update the `raw_tail` doc comment: validate no longer reads the raw tail.

In `validate.rs`:

```rust
/// Parsed `validate` flags.
pub struct Options {
    /// `--strict`.
    pub strict: bool,
}

/// Runs the command.
pub fn run(found: &Result<Root, Miss>, cwd: &Path, opts: &Options) -> (u8, Vec<u8>) {
    let (dir, project, display) = /* unchanged match */;
    let levels = match check::load(&project, opts.strict) {
        Ok(levels) => levels,
        Err(why) => {
            eprintln!("vibe-spec: {why}");
            return (exit::USAGE, Vec::new());
        }
    };
    let mut r = Report::new(&project, levels);
    // ... unchanged up to and including sf16(...)
    if opts.strict && (r.examined() == 0 || r.levels().all_off()) {
        r.at("");
        r.refuse(b"validate --strict examined nothing (no spec file, or every check is off); refusing to report green");
    }
    // summary lines and exit as before
}
```

Delete the old `if let Some(arg) = args.first()` usage branch: clap now rejects unknown arguments with exit 2.

- [ ] **Step 7: Run the tests, the old validate tests and parity**

```bash
cargo test --locked -p vibe-spec 2>&1 | grep -E 'test result|FAILED|panicked'
cargo clippy --workspace --all-targets --locked -- -D warnings 2>&1 | tail -1
cargo fmt --all --check && echo fmt-ok
bash tests/parity/run.sh 2>&1 | tail -4
```

Expected:
- Everything `ok`, and the 8 strictness tests pass.
- Parity is unchanged and green: no corpus tree has a config yet, and no flags are passed.

- [ ] **Step 8: Commit**

```bash
git add crates/vibe-spec
git commit -m "feat(spec): add strictness config and --strict"
```

### Task 3: `validate --format json`, the contract, and the strictness matrix harness

**Files:**
- Modify: `crates/vibe-spec/src/report.rs` (`json`), `crates/vibe-spec/src/validate.rs` (`Format`, JSON paths), `crates/vibe-spec/src/main.rs` (`--format`, `--json`)
- Modify: `contracts/spec-json.schema.json` (`validate`, `finding`, `check-id`, `check-level`), `contracts/run.mjs` (`QUERIES`, `REQUIRED`)
- Create: `contracts/fixtures/spec-json/validate/valid/{clean,findings,unknown-field}.json`, `contracts/fixtures/spec-json/validate/invalid/{bad-severity,missing-checks}.json`
- Modify: `crates/vibe-spec/tests/strictness.rs` (matrix harness), `crates/vibe-spec/tests/common/mod.rs` (`validate_json`)

**Interfaces:**
- Consumes: Tasks 1–2.
- Produces:
  - `validate::Format::{Text, Json}` (clap `ValueEnum`)
  - `Options { strict, format }`
  - `Report::json(&self, root: &str, layout: &str) -> Vec<u8>`
  - The Spec JSON v1 `validate` answer:
    - `{v, root, layout, strict, examined, errors, warnings, checks[{id, level}], findings[{check, severity, file?, message}]}`
    - `layout` is `flat` until Task 9 detects `layered`.
  - The matrix harness (`tests/strictness.rs`):
    - `fn trigger(id: &str) -> Option<Trigger>`, with `struct Trigger { project: PathBuf, env: Vec<(String, String)>, args: Vec<String>, needle: &'static str }`
    - `const PENDING: &[&str]`: registered ids without a trigger yet. Later tasks move ids from `PENDING` into `trigger`.

- [ ] **Step 1: Write the failing tests**

Add to `tests/common/mod.rs`:

```rust
/// `vibe-spec validate --format json <extra>` in `cwd`, parsed.
pub fn validate_json(cwd: &Path, extra: &[&str], env: &[(&str, &str)]) -> (i32, serde_json::Value) {
    let mut args = vec!["validate", "--format", "json"];
    args.extend_from_slice(extra);
    let out = spec(cwd, &args, env);
    let v = serde_json::from_slice(&out.stdout)
        .unwrap_or_else(|e| panic!("not JSON ({e}): {}\nstderr: {}", out.text(), out.stderr));
    (out.code, v)
}
```

(`vibe-spec`'s `[dev-dependencies]` gains `serde_json = { workspace = true }` if it is not already reachable in tests. It is a normal dependency, so integration tests can already `use serde_json`.)

Append to `crates/vibe-spec/tests/strictness.rs`:

```rust
use common::{assert_spec_json, validate_json};
use std::path::PathBuf;

/// A project that makes check `id` report at least one finding whose
/// message contains `needle`, run with `args` and `env`.
struct Trigger {
    project: PathBuf,
    env: Vec<(String, String)>,
    args: Vec<String>,
    needle: &'static str,
}

fn plain(project: PathBuf, needle: &'static str) -> Option<Trigger> {
    Some(Trigger { project, env: Vec::new(), args: Vec::new(), needle })
}

const FEATURE_PRODUCT: &str = "---\ntype: feature-product\nfeature: auth\nparent: ../../product.md\nsibling: tech.md\nupdated: 2026-01-15\n---\n\n## Scope\n\n| Owns | Does not own |\n|---|---|\n| sessions | billing |\n\n### Requirement: Sign in (R1)\nUsers MUST sign in.\n\n#### Scenario: ok\n- **WHEN** they log in\n- **THEN** they are in\n";
const FEATURE_TECH: &str = "---\ntype: feature-tech\nfeature: auth\nparent: ../../tech.md\nsibling: product.md\nupdated: 2026-01-15\n---\n\n# auth — Tech\n";

/// A clean flat tree with one feature; triggers add the one defect they need.
fn base(name: &str) -> PathBuf {
    let p = project(name);
    write(&p, ".spec/product.md", "---\ntype: entrypoint\nscope: product\nchildren:\nupdated: 2026-01-15\n---\n\n# P\n");
    write(&p, ".spec/plan.md", "---\ntype: entrypoint\nscope: implementation\nchildren:\nupdated: 2026-01-15\n---\n\n| Feature | Status |\n|---|---|\n| auth | IN PROGRESS |\n");
    write(&p, ".spec/features/auth/product.md", FEATURE_PRODUCT);
    write(&p, ".spec/features/auth/tech.md", FEATURE_TECH);
    p
}

fn trigger(id: &str) -> Option<Trigger> {
    let p = base(&format!("mx-{}", id.to_ascii_lowercase()));
    match id {
        "SF3" => {
            write(&p, ".spec/design.md", "---\ntype: entrypoint\nscope: design\nchildren:\ncolors:\nupdated: 2026-01-15\n---\n");
            plain(p, "empty token group")
        }
        "SF4" => {
            write(&p, ".spec/design.md", "---\ntype: entrypoint\nscope: design\nchildren:\nupdated: 2026-01-15\n---\n");
            let empty = p.join("emptybin");
            std::fs::create_dir_all(&empty).unwrap();
            Some(Trigger {
                project: p,
                env: vec![("VIBE_DESIGN_LINT".into(), "1".into()), ("PATH".into(), empty.display().to_string())],
                args: Vec::new(),
                needle: "lint skipped",
            })
        }
        "SF8" => {
            write(&p, ".spec/features/auth/product.md", &FEATURE_PRODUCT.replace("## Scope", "## Notes"));
            plain(p, "missing '## Scope'")
        }
        "SF9" => {
            write(&p, ".spec/features/auth/tech.md", &FEATURE_TECH.replace("sibling: product.md\n", ""));
            plain(p, "missing 'sibling:'")
        }
        "SF10" => {
            write(&p, ".spec/features/auth/product.md", &FEATURE_PRODUCT.replace("Users MUST sign in.", "Users sign in."));
            plain(p, "missing RFC-2119 keyword")
        }
        "SF11" => {
            write(&p, ".spec/features/auth/plan.md", "---\ntype: feature-plan\nfeature: auth\nparent: ../../plan.md\nupdated: 2026-01-15\n---\n\nNo units. Covers R1.\n");
            plain(p, "plan missing '### <feature>/<n>'")
        }
        "SF12" => {
            write(&p, ".spec/features/auth/plan.md", "---\ntype: feature-plan\nfeature: auth\nparent: ../../plan.md\nupdated: 2026-01-15\n---\n\n## Requirements Trace\n\n### auth/1 — a\n\n### auth/1 — b\n\nCovers R1.\n");
            plain(p, "duplicate unit heading ID")
        }
        "SF13" => {
            write(&p, ".spec/product.md", "---\ntype: entrypoint\nscope: product\nchildren:\nupdated: 2026-01-15\n---\n\n[g](features/ghost/)\n");
            plain(p, "SF13: stale link")
        }
        "SF14" => {
            write(&p, ".spec/features/billing/product.md", &FEATURE_PRODUCT.replace("feature: auth", "feature: billing"));
            write(&p, ".spec/features/billing/tech.md", &FEATURE_TECH.replace("feature: auth", "feature: billing"));
            plain(p, "SF14: scope conflict")
        }
        "SF15" => {
            write(&p, ".spec/product.md", &format!("---\ntype: entrypoint\nscope: product\nchildren:\nupdated: 2026-01-15\n---\n{}", "x\n".repeat(301)));
            plain(p, "SF15:")
        }
        "SF16" => {
            write(&p, ".spec/lessons.md", "# Lessons\n\n### Untagged\n**Rule:** x\n");
            plain(p, "has no **Tags:** line")
        }
        "I1" => {
            write(&p, ".spec/features/auth/plan.md", "---\ntype: feature-plan\nfeature: auth\nparent: ../../plan.md\nupdated: 2026-01-15\n---\n\n## Requirements Trace\n\nCovers R1.\n\n### auth/1 — a\n\nDepends on billing/2.\n");
            plain(p, "cross-feature unit dependency")
        }
        "SF20" => {
            write(&p, ".spec/tech.md", "# no frontmatter\n");
            plain(p, "missing YAML frontmatter")
        }
        "SF21" => {
            write(&p, ".spec/notes.md", "---\ntype: entrypoint\nupdated: 2026-01-15\nchildren:\n---\n");
            plain(p, "must start with")
        }
        "SF22" => {
            write(&p, ".spec/product.md", "---\ntype: entrypoint\nscope: product\nchildren:\nupdated: 2026-01-15\n---\n\n[x](missing.md)\n");
            plain(p, "broken link to 'missing.md'")
        }
        "SF23" => {
            std::fs::remove_file(p.join(".spec/features/auth/tech.md")).unwrap();
            plain(p, "missing required 'tech.md'")
        }
        _ => None,
    }
}

/// Registered checks that have no trigger yet; each later task moves its ids
/// into `trigger`. Task 13 requires this to be empty.
const PENDING: &[&str] = &[
    "SF24", "SF25", "SF26", "SF27", "SF28", "SF29", "SF30", "SF31", "SF32", "SF33", "SF34", "SF35", "SF36",
];

fn registered() -> Vec<String> {
    let p = base("mx-ids");
    let (_, v) = validate_json(&p, &[], &[]);
    v["checks"].as_array().unwrap().iter().map(|c| c["id"].as_str().unwrap().to_string()).collect()
}

fn run_with(t: &Trigger, config: Option<(&str, &str)>) -> (i32, serde_json::Value) {
    match config {
        Some((id, level)) => write(&t.project, ".vibe/spec.json", &format!(r#"{{"strictness": {{"{id}": "{level}"}}}}"#)),
        None => {
            let _ = std::fs::remove_file(t.project.join(".vibe/spec.json"));
        }
    }
    let env: Vec<(&str, &str)> = t.env.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
    let args: Vec<&str> = t.args.iter().map(String::as_str).collect();
    validate_json(&t.project, &args, &env)
}

fn hits<'a>(v: &'a serde_json::Value, id: &str, needle: &str) -> Vec<&'a serde_json::Value> {
    v["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["check"] == id && f["message"].as_str().unwrap().contains(needle))
        .collect()
}

#[test]
fn every_check_is_either_triggered_or_pending() {
    for id in registered() {
        let pending = PENDING.contains(&id.as_str());
        assert!(trigger(&id).is_some() != pending, "{id}: needs exactly one of a trigger or a PENDING entry");
    }
}

#[test]
fn warn_error_off_matrix() {
    let mut covered = 0;
    for id in registered() {
        let Some(t) = trigger(&id) else { continue };
        let (_, warn) = run_with(&t, Some((&id, "warn")));
        let w = hits(&warn, &id, t.needle);
        assert!(!w.is_empty(), "{id} warn: no finding with {:?}\n{warn:#}", t.needle);
        assert!(w.iter().all(|f| f["severity"] == "warning"), "{id} warn: {w:?}");
        let (code, error) = run_with(&t, Some((&id, "error")));
        let e = hits(&error, &id, t.needle);
        assert!(!e.is_empty() && e.iter().all(|f| f["severity"] == "error"), "{id} error: {e:?}");
        assert_eq!(code, 1, "{id} error must exit 1");
        let (_, off) = run_with(&t, Some((&id, "off")));
        assert!(off["findings"].as_array().unwrap().iter().all(|f| f["check"] != id.as_str()), "{id} off: {off:#}");
        covered += 1;
    }
    assert!(covered >= 16, "matrix covered only {covered} checks");
}

#[test]
fn json_answers_validate_against_the_schema() {
    let t = trigger("SF13").unwrap();
    let (code, v) = run_with(&t, None);
    assert_eq!(code, 0);
    assert_spec_json("validate", &v.to_string());
    assert_eq!(v["layout"], "flat");
    assert!(v["examined"].as_u64().unwrap() >= 4);
    let (_, strict) = validate_json(&t.project, &["--strict"], &[]);
    assert_eq!(strict["strict"], true);
    assert_spec_json("validate", &strict.to_string());
}

#[test]
fn json_without_a_root_exits_3() {
    let p = project("mx-noroot");
    let out = spec(&p, &["validate", "--json"], &[]);
    assert_eq!(out.code, 3);
    assert!(out.stdout.is_empty());
    let text = spec(&p, &["validate"], &[]);
    assert_eq!(text.code, 1, "text mode keeps the oracle's exit");
}

#[test]
fn json_survives_hostile_bytes() {
    let p = base("mx-hostile");
    write(&p, ".spec/lessons.md", "# Lessons\n\n### Tab\there \u{1} \"q\" \\ x\n**Rule:** y\n");
    std::fs::write(p.join(".spec/features/auth/plan.md"), b"---\ntype: feature-plan\nupdated: 2026-01-15\n---\n\n### auth/1 \xff\xfe\n\n### auth/1 again\n").unwrap();
    let out = spec(&p, &["validate", "--json"], &[]);
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_spec_json("validate", &v.to_string());
    assert!(v["findings"].as_array().unwrap().iter().any(|f| f["check"] == "SF16"));
}
```

Create the goldens (one JSON object per file):
- `validate/valid/clean.json`: `{ "v": 1, "root": ".spec", "layout": "flat", "strict": false, "examined": 5, "errors": 0, "warnings": 0, "checks": [ { "id": "SF3", "level": "default" } ], "findings": [] }`
- `validate/valid/findings.json`: the same with `"strict": true, "errors": 1, "checks": [ { "id": "SF13", "level": "error" }, { "id": "SF24", "level": "off" } ], "findings": [ { "check": "SF13", "severity": "error", "file": ".spec/product.md", "message": "SF13: stale link in .spec/product.md → features/ghost/ (not found)" } ]`
- `validate/valid/unknown-field.json`: `clean.json` plus `"duration_ms": 12`
- `validate/invalid/bad-severity.json`: `findings.json` with `"severity": "fatal"`
- `validate/invalid/missing-checks.json`: `clean.json` without `checks`

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test --locked -p vibe-spec --test strictness 2>&1 | grep -E '^test |test result'` and `bash contracts/run.sh 2>&1 | tail -2`
Expected:
- The matrix tests fail, because `--format` is an unknown argument (exit 2) and the stdout is not JSON.
- contracts passes. The `validate` goldens are not read yet, because `QUERIES` does not list `validate`.

Paste the output.

- [ ] **Step 3: Add the schema defs and register the query**

In `contracts/spec-json.schema.json`, update the description's first sentence to "Answers of the read-only vibe-spec queries when called with --json: root, lessons-for, plan, feature, validate (and trace, Task 12)." Add to `$defs`:

```json
    "validate": {
      "type": "object",
      "required": ["v", "root", "layout", "strict", "examined", "errors", "warnings", "checks", "findings"],
      "properties": {
        "v": { "const": 1 },
        "root": { "type": "string", "minLength": 1, "description": "The spec root as validate prints it: project-relative, or absolute outside the project." },
        "layout": { "enum": ["flat", "layered"] },
        "strict": { "type": "boolean" },
        "examined": { "type": "integer", "minimum": 0, "description": "Distinct files the run examined." },
        "errors": { "type": "integer", "minimum": 0 },
        "warnings": { "type": "integer", "minimum": 0 },
        "checks": { "type": "array", "items": { "$ref": "#/$defs/check-level" } },
        "findings": { "type": "array", "items": { "$ref": "#/$defs/finding" } }
      }
    },
    "check-level": {
      "type": "object",
      "required": ["id", "level"],
      "properties": {
        "id": { "$ref": "#/$defs/check-id" },
        "level": { "enum": ["default", "warn", "error", "off"] }
      }
    },
    "finding": {
      "type": "object",
      "required": ["check", "severity", "message"],
      "properties": {
        "check": { "$ref": "#/$defs/check-id" },
        "severity": { "enum": ["warning", "error"] },
        "file": { "type": "string", "minLength": 1, "description": "Project-relative path with forward slashes; absolute outside the project." },
        "message": { "type": "string" }
      }
    },
    "check-id": { "type": "string", "pattern": "^(SF[0-9]+|I[0-9]+)$" }
```

In `contracts/run.mjs`, set `const QUERIES = ['root', 'lessons-for', 'plan', 'feature', 'validate'];` and add the following to `REQUIRED`:

```js
  validate: ['checks', 'errors', 'examined', 'findings', 'layout', 'root', 'strict', 'v', 'warnings'],
  'check-level': ['id', 'level'],
  finding: ['check', 'message', 'severity'],
```

- [ ] **Step 4: Render JSON and parse `--format`**

In `report.rs` (imports `serde_json::json`, `crate::check::CHECKS`, `crate::text::lossy`):

```rust
    /// Spec JSON v1 `validate`.
    pub fn json(&self, root: &str, layout: &str) -> Vec<u8> {
        let checks: Vec<serde_json::Value> = CHECKS
            .iter()
            .map(|c| json!({ "id": c.id, "level": self.levels.describe(c.id) }))
            .collect();
        let findings: Vec<serde_json::Value> = self
            .findings
            .iter()
            .map(|f| {
                let mut v = json!({
                    "check": f.check,
                    "severity": f.severity.as_str(),
                    "message": lossy(&f.message),
                });
                if !f.file.is_empty() {
                    v["file"] = json!(f.file);
                }
                v
            })
            .collect();
        let v = json!({
            "v": 1,
            "root": root,
            "layout": layout,
            "strict": self.levels.strict(),
            "examined": self.examined(),
            "errors": self.errors,
            "warnings": self.warnings,
            "checks": checks,
            "findings": findings,
        });
        format!("{v}\n").into_bytes()
    }
```

In `validate.rs`:

```rust
/// `--format`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
pub enum Format {
    /// The oracle's coloured report.
    Text,
    /// Spec JSON v1 `validate`.
    Json,
}

/// Parsed `validate` flags.
pub struct Options {
    /// `--strict`.
    pub strict: bool,
    /// `--format` (`--json` sets `Json`).
    pub format: Format,
}
```

`run` changes:
- `Err(Miss::NotFound)` with `opts.format == Format::Json`: `eprintln!("{}", Miss::NotFound.message())` and return `(exit::NO_ROOT, Vec::new())` (P3b-R9). Text mode keeps its current branch.
- The two early returns (`No spec root…`, `No spec files…`) and the final return pick the output by format. Text gives `r.out` plus the summary. JSON gives `r.json(&display, "flat")` with the same exit code (1 when `r.errors > 0`, else 0).
- The summary lines (`====…`, `Errors:`, `Warnings:`) are added only in text mode. Build them into `r.out` before returning, as today.

`main.rs`:

```rust
    Validate {
        /// Promote every non-off check to error; refuse a run that examined nothing.
        #[arg(long)]
        strict: bool,
        /// Output format.
        #[arg(long, value_enum, default_value = "text")]
        format: validate::Format,
        /// Same as `--format json`.
        #[arg(long)]
        json: bool,
    },
```

The dispatch becomes `Command::Validate { strict, format, json } => validate::run(&found, &cwd, &validate::Options { strict, format: if json { validate::Format::Json } else { format } })`.

- [ ] **Step 5: Run everything affected**

```bash
cargo test --locked -p vibe-spec 2>&1 | grep -E 'test result|FAILED|panicked'
cargo clippy --workspace --all-targets --locked -- -D warnings 2>&1 | tail -1
cargo fmt --all --check && echo fmt-ok
bash contracts/run.sh | tail -1
bash tests/parity/run.sh 2>&1 | tail -4
```

Expected:
- All `ok`, with the matrix covering 16 legacy checks.
- `contracts: 2 schemas, 12 valid + 10 invalid goldens — OK`.
- Parity is green and unchanged.

- [ ] **Step 6: Commit**

```bash
git add crates/vibe-spec contracts
git commit -m "feat(spec): add validate --format json"
```

### Task 4: Parity through the behaviour change

**Files:**
- Create: `tests/parity/legacy-strictness.json`
- Modify: `crates/vibe-parity/src/corpus_spec.rs` (write it into every case tree), `crates/vibe-parity/tests/spec_corpus.rs` (guard), `tests/parity/corpus/spec/**` (regenerated), `tests/parity/run.sh` (retire live validate), `crates/vibe-spec/tests/common/mod.rs` (`legacy`, `assert_parity`), `crates/vibe-spec/tests/validate.rs` (direct oracle comparisons call `legacy`), `crates/vibe-spec/src/check.rs` (sync test)

**Interfaces:**
- Consumes: `check::{CHECKS, Origin, normalize}`.
- Produces:
  - `tests/parity/legacy-strictness.json`: the single list of F17-added checks, all `off`.
  - `common::LEGACY_STRICTNESS: &str`, `common::legacy(project: &Path)`.
  - `assert_parity` writes the legacy config before comparing `validate`.

- [ ] **Step 1: Write the failing guards**

In `crates/vibe-spec/src/check.rs` tests:

```rust
    #[test]
    #[allow(clippy::unwrap_used)]
    fn the_corpus_config_turns_off_exactly_the_new_checks() {
        let text = include_str!("../../../tests/parity/legacy-strictness.json");
        let v: serde_json::Value = serde_json::from_str(text).unwrap();
        let mut got: Vec<&str> = v["strictness"]
            .as_object()
            .unwrap()
            .iter()
            .map(|(k, level)| {
                assert_eq!(level, "off", "{k}");
                normalize(k).unwrap()
            })
            .collect();
        got.sort_unstable();
        let mut want: Vec<&str> = CHECKS.iter().filter(|c| c.origin == Origin::New).map(|c| c.id).collect();
        want.sort_unstable();
        assert_eq!(got, want);
    }
```

In `crates/vibe-parity/tests/spec_corpus.rs`:

```rust
#[test]
fn every_case_tree_turns_the_f17_checks_off() {
    let want = fs::read_to_string(repo().join("tests/parity/legacy-strictness.json")).unwrap();
    let corpus = repo().join("tests/parity/corpus/spec");
    let mut cases = 0;
    for class in fs::read_dir(&corpus).unwrap().flatten().filter(|e| e.path().is_dir()) {
        for case in fs::read_dir(class.path()).unwrap().flatten().filter(|e| e.path().is_dir()) {
            let got = fs::read_to_string(case.path().join("tree/.vibe/spec.json"))
                .unwrap_or_else(|e| panic!("{}: {e}", case.path().display()));
            assert_eq!(got, want, "{}", case.path().display());
            cases += 1;
        }
    }
    assert!(cases >= 25, "only {cases} cases");
}
```

(Use the file's existing `repo()` helper and `use std::fs;` imports.)

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test --locked -p vibe-spec --bins the_corpus_config 2>&1 | tail -3; cargo test --locked -p vibe-parity --test spec_corpus every_case 2>&1 | tail -3`
Expected:
- vibe-spec fails to compile: `couldn't read .../tests/parity/legacy-strictness.json`.
- vibe-parity fails with `No such file or directory`.

- [ ] **Step 3: Create the config and write it into every case**

`tests/parity/legacy-strictness.json`:

```json
{
  "strictness": {
    "SF-24": "off",
    "SF-25": "off",
    "SF-26": "off",
    "SF-27": "off",
    "SF-28": "off",
    "SF-29": "off",
    "SF-30": "off",
    "SF-31": "off",
    "SF-32": "off",
    "SF-33": "off",
    "SF-34": "off",
    "SF-35": "off",
    "SF-36": "off"
  }
}
```

In `corpus_spec.rs`, add the following near the other constants:

```rust
/// The F17-added checks, all `off` (P3b-R2). The oracle ignores the file;
/// the candidate reads it, so the corpus keeps proving the legacy engine.
const LEGACY_STRICTNESS: &str = include_str!("../../../tests/parity/legacy-strictness.json");
```

and in `generate`, after the loop that writes each case's files:

```rust
        write(&dir.join("tree/.vibe/spec.json"), LEGACY_STRICTNESS)?;
```

Regenerate: `cargo run --locked -q -p vibe-parity -- gen-spec-corpus oracles/bash/spec/reference/templates tests/parity/corpus/spec`. Expect `gen-spec-corpus: 25 cases written to tests/parity/corpus/spec`.

- [ ] **Step 4: Rust parity tests write the same config**

In `crates/vibe-spec/tests/common/mod.rs`:

```rust
/// The corpus's F17-off strictness (P3b-R2).
pub const LEGACY_STRICTNESS: &str = include_str!("../../../../tests/parity/legacy-strictness.json");

/// Writes the legacy strictness into the project that holds `cwd` (nearest
/// ancestor with `.git`), unless a config is already there.
pub fn legacy(cwd: &Path) {
    let project = cwd.ancestors().find(|d| d.join(".git").exists()).unwrap_or(cwd);
    let path = project.join(".vibe/spec.json");
    if !path.exists() {
        write(project, ".vibe/spec.json", LEGACY_STRICTNESS);
    }
}
```

In `assert_parity`, call `legacy(cwd);` first when `command == "validate"`. In `tests/validate.rs`, call `common::legacy(&p);` at the start of every test that compares with `common::oracle`/`oracle_env` directly: `features_sf13_sf15_and_sf16_match` (macOS branch), `design_lint_degrades_like_the_oracle`, `sf15_threshold_follows_bash_arithmetic`.

- [ ] **Step 5: Retire live validate parity**

In `tests/parity/run.sh`, delete the line `live validate validate.sh`. Replace the header comment with:

```bash
# Spec parity: the frozen bash oracles against themselves, against the
# vibe-spec candidate over the generated corpus, and against the candidate on
# this repo's own spec tree ("live"). validate leaves live parity at F17: its
# F17 checks are off only in the corpus (P3b-R2). Needs cargo. Exit 0 only
# when all pass.
```

- [ ] **Step 6: Run the guards and parity**

```bash
cargo test --locked -p vibe-spec 2>&1 | grep -E 'test result|FAILED'
cargo test --locked -p vibe-parity 2>&1 | grep -E 'test result|FAILED'
bash tests/parity/run.sh 2>&1 | tail -8
```

Expected:
- Everything `ok`.
- Parity: the self-check and the candidate show `0 failed, 0 error(s)` with the same counts as before.
- Four `PASS  live` lines, then `parity: self-check, candidate and live (4) green`.

- [ ] **Step 7: Commit**

```bash
git add tests/parity crates/vibe-parity crates/vibe-spec
git commit -m "test(parity): keep f17 checks off in the corpus"
```

### Task 5: Fix the R-ID, RFC-2119 and Scope bugs; one R-ID matcher

**Files:**
- Modify: `crates/vibe-spec/src/pattern.rs` (`is_word`, `has_word`, `rfc_keyword`), `crates/vibe-spec/src/content.rs` (SF8, SF10, SF12)
- Modify: `crates/vibe-parity/src/corpus_spec.rs` (cases `features/rfc-boundary`, `features/scope-section`), `tests/parity/spec.json` (pins, floor), `tests/parity/corpus/spec/**` (regenerated)
- Modify: `spec/reference/product.md` (`R-1` → `R1`)
- Test: `crates/vibe-spec/tests/grammar.rs` (new), `crates/vibe-spec/tests/validate.rs` (`content_checks_match` cites both R-IDs)

**Interfaces:**
- Consumes: `pattern::r_ids`, `Report::warn`.
- Produces:
  - `pattern::is_word(c: u8) -> bool`
  - `pattern::has_word(s: &[u8], needle: &[u8]) -> bool`
  - `pattern::rfc_keyword(s: &[u8]) -> bool`

The SF8 and SF12 message text is unchanged.

- [ ] **Step 1: Write the failing tests**

Create `crates/vibe-spec/tests/grammar.rs`:

```rust
#![cfg(unix)]
#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Requirement grammar: the substring-bug fixes (P3b-R16, R17, R3), SF24,
//! SF25 and the OpenSpec samples (P3b-R19).
mod common;

use common::{project, validate_json, write};
use std::path::PathBuf;

const HEAD: &str = "---\ntype: feature-product\nfeature: auth\nparent: ../../product.md\nsibling: tech.md\nupdated: 2026-01-15\n---\n\n";
const SCOPE: &str = "## Scope\n\n| Owns | Does not own |\n|---|---|\n| sessions | billing |\n\n";
const TECH: &str = "---\ntype: feature-tech\nfeature: auth\nparent: ../../tech.md\nsibling: product.md\nupdated: 2026-01-15\n---\n";

/// A project whose only feature product body is `body` (after the frontmatter).
fn feature(name: &str, body: &str) -> PathBuf {
    let p = project(name);
    write(&p, ".spec/product.md", "---\ntype: entrypoint\nscope: product\nchildren:\nupdated: 2026-01-15\n---\n");
    write(&p, ".spec/features/auth/product.md", &format!("{HEAD}{body}"));
    write(&p, ".spec/features/auth/tech.md", TECH);
    p
}

fn messages(p: &std::path::Path, check: &str) -> Vec<String> {
    let (_, v) = validate_json(p, &[], &[]);
    v["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["check"] == check)
        .map(|f| f["message"].as_str().unwrap().to_string())
        .collect()
}

#[test]
fn r_id_citations_are_word_bounded() {
    let p = feature("gr-rid", &format!("{SCOPE}### Requirement: Sign in (R1)\nUsers MUST sign in.\n#### Scenario: a\n- WHEN x\n- THEN y\n\n### Requirement: R10 — Sign out\nUsers MAY sign out.\n#### Scenario: b\n- WHEN x\n- THEN y\n"));
    write(&p, ".spec/features/auth/plan.md", "---\ntype: feature-plan\nfeature: auth\nparent: ../../plan.md\nupdated: 2026-01-15\n---\n\n## Requirements Trace\n\nCovers R10 and xR1 and R1x.\n\n### auth/1 — a\n");
    let sf12 = messages(&p, "SF12");
    assert_eq!(sf12, ["features/auth: R1 in product.md not cited in plan.md (light traceability check)"], "{sf12:?}");
}

#[test]
fn the_openspec_id_prefix_is_an_r_id() {
    let p = feature("gr-rid-prefix", &format!("{SCOPE}### Requirement: R7 — Export\nUsers MUST export.\n#### Scenario: a\n- WHEN x\n- THEN y\n"));
    write(&p, ".spec/features/auth/plan.md", "---\ntype: feature-plan\nfeature: auth\nparent: ../../plan.md\nupdated: 2026-01-15\n---\n\n## Requirements Trace\n\nNothing cited.\n\n### auth/1 — a\n");
    assert_eq!(messages(&p, "SF12"), ["features/auth: R7 in product.md not cited in plan.md (light traceability check)"]);
}

#[test]
fn rfc_keywords_are_capital_whole_words() {
    let p = feature("gr-rfc", &format!("{SCOPE}### Requirement: Mayor sign in\nUsers mustard the shallow form; they MUSTN'T x.\n#### Scenario: a\n- WHEN x\n- THEN y\n\n### Requirement: Logout\nUsers MUST NOT stay.\n#### Scenario: b\n- WHEN x\n- THEN y\n"));
    let sf10 = messages(&p, "SF10");
    assert!(sf10.iter().any(|m| m.contains("missing RFC-2119 keyword (SHALL|MUST|SHOULD|MAY) -> Mayor sign in")), "{sf10:?}");
    assert!(!sf10.iter().any(|m| m.contains("-> Logout")), "{sf10:?}");
}

#[test]
fn scope_words_count_only_inside_the_scope_section() {
    let p = feature("gr-scope", "## Scope\n\nTBD.\n\n## Notes\n\nThe downstream team does not own billing. Owns nothing.\n\n### Requirement: A\nUsers MUST a.\n#### Scenario: a\n- WHEN x\n- THEN y\n");
    let sf8 = messages(&p, "SF8");
    assert_eq!(
        sf8,
        [
            "features/auth/product.md: Scope section missing 'Owns' column or row",
            "features/auth/product.md: Scope section missing 'Does not own' column or row",
        ],
        "{sf8:?}"
    );
    let ok = feature("gr-scope-ok", "## Scope\n\nOwns invoices; does not own auth.\n\n### Requirement: A\nUsers MUST a.\n#### Scenario: a\n- WHEN x\n- THEN y\n");
    assert!(messages(&ok, "SF8").is_empty());
    let scopes = feature("gr-scopes", "## Scopes\n\n| Owns | Does not own |\n\n### Requirement: A\nUsers MUST a.\n#### Scenario: a\n- WHEN x\n- THEN y\n");
    assert_eq!(messages(&scopes, "SF8"), ["features/auth/product.md: missing '## Scope' section (add Owns / Does not own table)"]);
}
```

Also add unit tests to `pattern.rs`:

```rust
    #[test]
    fn whole_words_only() {
        assert!(has_word(b"a owns b", b"owns"));
        assert!(has_word(b"**Owns**", b"Owns"));
        assert!(!has_word(b"downstream", b"owns"));
        assert!(!has_word(b"ownsx", b"owns"));
        assert!(has_word(b"x does not own", b"does not own"));
        assert!(rfc_keyword(b"Users MUST NOT x"));
        assert!(rfc_keyword(b"(SHALL)"));
        assert!(!rfc_keyword(b"Mayor of SHALLOW"));
        assert!(!rfc_keyword(b"users must x"));
        assert!(!rfc_keyword(b"MUSTN"));
    }
```

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test --locked -p vibe-spec --test grammar 2>&1 | grep -E '^test |test result'`
Expected:
- `the_openspec_id_prefix_is_an_r_id` passes. It already works, because extraction is word-bounded today.
- The other three FAIL:
  - SF12 is empty, since `R1` is found inside `R10`.
  - The `Mayor` requirement is not flagged.
  - SF8 is empty.

The `pattern` unit test does not compile (`has_word` is missing). Paste both.

- [ ] **Step 3: Implement the matchers**

In `pattern.rs`, make `word` public as `is_word` and update its callers. Then add:

```rust
/// `[A-Za-z0-9_]`, the `\b` word class.
pub fn is_word(c: u8) -> bool {
    c.is_ascii_alphanumeric() || c == b'_'
}

/// `needle` in `s` with a non-word byte (or an end) on both sides.
pub fn has_word(s: &[u8], needle: &[u8]) -> bool {
    if needle.is_empty() || s.len() < needle.len() {
        return false;
    }
    (0..=s.len() - needle.len()).any(|i| {
        s.get(i..).is_some_and(|rest| rest.starts_with(needle))
            && (i == 0 || s.get(i - 1).is_some_and(|&c| !is_word(c)))
            && s.get(i + needle.len()).is_none_or(|&c| !is_word(c))
    })
}

/// An RFC 2119 keyword in capitals as a whole word (P3b-R17).
pub fn rfc_keyword(s: &[u8]) -> bool {
    [&b"SHALL"[..], b"MUST", b"SHOULD", b"MAY"]
        .iter()
        .any(|k| has_word(s, k))
}
```

- [ ] **Step 4: Fix SF10, SF8 and SF12**

In `content.rs`:
- Delete `fn rfc` and use `rfc_keyword` in its two call sites. Change the module doc's last sentence to "F17 fixes their known bugs (R-ID prefix, RFC-2119 boundary, Scope scoping)."
- Replace `feature_scope` with:

```rust
/// The lines of the `## Scope` section: after the first line that is
/// exactly `## Scope` (surrounding blanks allowed) up to the next `# ` or
/// `## ` heading. `None` without one (P3b-R16).
fn scope_section(text: &[u8]) -> Option<Vec<&[u8]>> {
    let mut section: Option<Vec<&[u8]>> = None;
    for line in lines(text) {
        match section.as_mut() {
            Some(s) => {
                if line.starts_with(b"# ") || line.starts_with(b"## ") {
                    break;
                }
                s.push(line);
            }
            None if trim_space(line) == b"## Scope" => section = Some(Vec::new()),
            None => {}
        }
    }
    section
}

/// SF8: the feature product's Scope section names what it owns and what
/// it does not own, as words inside that section.
pub fn feature_scope(r: &mut Report, text: &[u8], label: &[u8]) {
    let Some(section) = scope_section(text) else {
        r.warn("SF8", &cat!(label, ": missing '## Scope' section (add Owns / Does not own table)"));
        return;
    };
    let lower: Vec<Vec<u8>> = section.iter().map(|l| l.to_ascii_lowercase()).collect();
    if !lower.iter().any(|l| has_word(l, b"owns")) {
        r.warn("SF8", &cat!(label, ": Scope section missing 'Owns' column or row"));
    }
    if !lower.iter().any(|l| has_word(l, b"does not own")) {
        r.warn("SF8", &cat!(label, ": Scope section missing 'Does not own' column or row"));
    }
}
```

- In `id_traceability`, replace the citation lookup:

```rust
    let cited: BTreeSet<&[u8]> = r_ids(plan).into_iter().collect();
    for id in ids {
        if !cited.contains(id.as_slice()) {
            r.warn("SF12", &cat!(label, ": ", id, " in product.md not cited in plan.md (light traceability check)"));
        }
    }
```

and change its doc comment to "SF12: duplicate unit IDs, and product R-IDs the plan does not cite as a whole word (one matcher for both sides, P3b-R3)."

- [ ] **Step 5: Keep the Rust parity fixture on the shared behaviour**

In `tests/validate.rs`, change `AUTH_PLAN`'s `Covers R10.` to `Covers R1 and R10.` so `content_checks_match` keeps comparing behaviour both versions share. The R1-vs-R10 divergence is now covered by `r_id_citations_are_word_bounded`.

- [ ] **Step 6: Run the new tests to see them pass**

Run: `cargo test --locked -p vibe-spec 2>&1 | grep -E 'test result|FAILED|panicked'`
Expected: everything `ok`.

- [ ] **Step 7: Corpus cases and pins**

In `corpus_spec.rs`, add two `features` cases next to `regex-feature`. Use the `rich-features` feature tech text (with `feature:`, `parent:` and `sibling:`) for `tech.md`. The products are:
- `rfc-boundary`: `features/auth/product.md` = the rich-features auth frontmatter, then `## Scope\n\n| Owns | Does not own |\n|---|---|\n| sessions | billing |\n\n### Requirement: Mayor sign in\nThey enter the shallow end.\n#### Scenario: ok\nWorks.\n`.
- `scope-section`: `features/auth/product.md` = the same frontmatter, then `## Scope\n\nTBD.\n\n## Notes\n\nThe downstream team does not own billing.\n`.

Raise `floors.features` to 12 in `tests/parity/spec.json`. Regenerate the corpus. Run `bash tests/parity/run.sh 2>&1 | grep -E '^FAIL|PIN|parity\['`.

Expected: exactly these validate candidate runs FAIL on `stdout`:
- `features/rich-features`: R1 is now uncited.
- `features/rfc-boundary`.
- `features/scope-section`.

Any other FAIL is unexplained and blocks the task. Report it and stop. Then add one pin per failure to `divergences` in `tests/parity/spec.json`:

```json
    { "command": "validate", "case": "features/rich-features", "args": [], "field": "stdout", "reason": "the oracle's grep -q counts R1 as cited by R10; the port compares whole R-IDs (P3b-R3)" },
    { "command": "validate", "case": "features/rfc-boundary", "args": [], "field": "stdout", "reason": "the oracle's awk matches MAY inside Mayor and SHALL inside shallow; the port needs a capital whole word (P3b-R17)" },
    { "command": "validate", "case": "features/scope-section", "args": [], "field": "stdout", "reason": "the oracle greps owns/does not own across the whole file (downstream counts); the port reads words inside the ## Scope section (P3b-R16)" }
```

Re-run parity. Expected: `0 failed, 0 error(s)`, 13 `PIN` lines on macOS (12 on Linux), 4 `PASS  live`.

- [ ] **Step 8: Reword the R-ID reference**

In `spec/reference/product.md`, change the frontmatter example ids `R-1` and `R-2` to `R1` and `R2`. They are the same IDs the `(R1)` heading tag uses.

- [ ] **Step 9: Commit, then land one mutant per fix**

```bash
git add crates tests/parity spec/reference/product.md
git commit -m "fix(spec): bound r-ids, rfc keywords and scope"
```

For each fix, plant the mutant and run its test. It must FAIL. Then `git diff --numstat` must show the one-line mutant, after which you restore the file with `git checkout -- <file>`. Paste all three FAIL lines:

1. **R-ID prefix.** In `id_traceability`, replace `!cited.contains(id.as_slice())` with `!crate::text::contains(plan, &id)`. Run `cargo test --locked -p vibe-spec --test grammar r_id_citations_are_word_bounded`.
2. **RFC boundary.** In `rfc_keyword`, replace `has_word(s, k)` with `crate::text::contains(s, k)`. Run `cargo test --locked -p vibe-spec --test grammar rfc_keywords_are_capital_whole_words`.
3. **Scope scoping.** In `feature_scope`, replace `let lower: Vec<Vec<u8>> = section.iter()…` with `let lower: Vec<Vec<u8>> = lines(text).iter().map(|l| l.to_ascii_lowercase()).collect();`. Run `cargo test --locked -p vibe-spec --test grammar scope_words_count_only_inside_the_scope_section`.

After the last restore, `git status --short crates` prints nothing.

### Task 6: Grammar: SF24 requirement population, SF25 WHEN/THEN, OpenSpec samples

**Files:**
- Modify: `crates/vibe-spec/src/text.rs` (move `fence_run` here from `query.rs`; add `Fences`), `crates/vibe-spec/src/query.rs` (import them), `crates/vibe-spec/src/content.rs` (`requirement_population`, `scenario_grammar`), `crates/vibe-spec/src/validate.rs` (call sites)
- Test: `crates/vibe-spec/tests/grammar.rs`, `crates/vibe-spec/tests/strictness.rs` (triggers SF24, SF25)

**Interfaces:**
- Consumes: Task 5's `grammar.rs` helpers (`feature`, `messages`, `HEAD`, `SCOPE`, `TECH`).
- Produces:
  - `text::fence_run(line: &[u8]) -> Option<(u8, usize, &[u8])>`
  - `text::Fences::new()`, `Fences::skip(&mut self, line: &[u8]) -> bool`
  - `content::requirement_population(r, text, label)` (SF24)
  - `content::scenario_grammar(r, text, label)` (SF25)

Call sites:
- `requirement_population` runs on every fallback `features/<f>/product.md`.
- `scenario_grammar` runs everywhere `requirements_scenarios` runs: root files and all four feature docs. Task 10 adds layered notes.

- [ ] **Step 1: Write the failing tests**

Append to `tests/grammar.rs`:

```rust
/// OpenSpec's requirement grammar (P3b-R19): `### Requirement:`, a SHALL or
/// MUST sentence, `#### Scenario:`, `- **WHEN**` / `- **THEN**` bullets.
const OPENSPEC_AUTH: &str = "### Requirement: User Authentication\nThe system SHALL issue a session token on successful login.\n\n#### Scenario: Valid credentials\n- **WHEN** a user submits valid credentials\n- **THEN** a session token is returned\n\n#### Scenario: Invalid credentials\n- **WHEN** a user submits a wrong password\n- **THEN** an error is returned\n- **AND** no token is issued\n";
const OPENSPEC_EXPORT: &str = "### Requirement: R2 — Data export\nUsers MUST be able to export their data as CSV.\n\n#### Scenario: Export with a fenced sample\n- **GIVEN** an account with data\n- **WHEN** the user requests an export\n- **THEN** a CSV file is produced\n\n```text\n#### Scenario: not a scenario\n- WHEN ignored\n```\n";

fn grammar(p: &std::path::Path) -> Vec<String> {
    let (_, v) = validate_json(p, &[], &[]);
    v["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| ["SF10", "SF24", "SF25"].contains(&f["check"].as_str().unwrap()))
        .map(|f| format!("{} {}", f["check"].as_str().unwrap(), f["message"].as_str().unwrap()))
        .collect()
}

#[test]
fn openspec_samples_pass() {
    let p = feature("gr-os", &format!("{SCOPE}## Requirements\n\n{OPENSPEC_AUTH}\n{OPENSPEC_EXPORT}"));
    assert_eq!(grammar(&p), Vec::<String>::new());
}

#[test]
fn mutated_openspec_samples_fail() {
    let cases: [(&str, String, &str); 5] = [
        ("gr-os-noshall", OPENSPEC_AUTH.replace("SHALL ", ""), "SF10"),
        ("gr-os-shallow", OPENSPEC_AUTH.replace("SHALL", "SHALLOW"), "SF10"),
        ("gr-os-nothen", OPENSPEC_AUTH.replace("- **THEN** a session token is returned\n", ""), "SF25"),
        ("gr-os-nowhen", OPENSPEC_AUTH.replace("- **WHEN** a user submits a wrong password\n", ""), "SF25"),
        ("gr-os-noscenario", "### Requirement: Bare\nThe system SHALL exist.\n".to_string(), "SF10"),
    ];
    for (name, body, check) in cases {
        let p = feature(name, &format!("{SCOPE}{body}"));
        let got = grammar(&p);
        assert!(got.iter().any(|m| m.starts_with(check)), "{name}: want {check}, got {got:?}");
    }
}

#[test]
fn zero_requirements_are_reported() {
    let p = feature("gr-zero", SCOPE);
    assert_eq!(
        grammar(&p),
        ["SF24 features/auth/product.md: no '### Requirement:' blocks — the requirement checks examined nothing"]
    );
    let empty = feature("gr-empty-title", &format!("{SCOPE}### Requirement:\nUsers MUST x.\n#### Scenario: a\n- WHEN x\n- THEN y\n"));
    assert!(grammar(&empty).contains(&"SF24 features/auth/product.md: requirement with an empty title".to_string()));
}

#[test]
fn fenced_examples_do_not_count() {
    let p = feature("gr-fenced", &format!("{SCOPE}```markdown\n### Requirement: Example only\nThe system SHALL x.\n#### Scenario: e\n- WHEN a\n```\n"));
    let got = grammar(&p);
    assert!(got.iter().any(|m| m.starts_with("SF24") && m.contains("no '### Requirement:' blocks")), "{got:?}");
    assert!(!got.iter().any(|m| m.starts_with("SF25")), "{got:?}");
}
```

In `tests/strictness.rs`, remove `"SF24"` and `"SF25"` from `PENDING`, and add these arms to `trigger`:

```rust
        "SF24" => {
            write(&p, ".spec/features/auth/product.md", FEATURE_PRODUCT.split("### Requirement").next().unwrap());
            plain(p, "no '### Requirement:' blocks")
        }
        "SF25" => {
            write(&p, ".spec/features/auth/product.md", &FEATURE_PRODUCT.replace("- **THEN** they are in\n", ""));
            plain(p, "has no THEN bullet")
        }
```

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test --locked -p vibe-spec --test grammar --test strictness 2>&1 | grep -E '^test |test result'`
Expected:
- `openspec_samples_pass` and `mutated_openspec_samples_fail` cases that rely on SF10 pass.
- `gr-os-nothen` fails, and so do `zero_requirements_are_reported`, `fenced_examples_do_not_count` and `warn_error_off_matrix` (SF24 and SF25 report nothing).

Paste the output.

- [ ] **Step 3: Shared fence tracking**

Move `fence_run` from `query.rs` to `text.rs` as `pub fn fence_run`, unchanged. Add:

```rust
/// Fenced-code tracking for the F17 line scanners.
#[derive(Default)]
pub struct Fences(Option<(u8, usize)>);

impl Fences {
    /// Outside any fence.
    pub fn new() -> Fences {
        Fences(None)
    }

    /// Feeds one line; true when it is a fence delimiter or inside a fence.
    pub fn skip(&mut self, line: &[u8]) -> bool {
        if let Some((ch, len)) = self.0 {
            if fence_run(line).is_some_and(|(c, n, rest)| c == ch && n >= len && trim_space(rest).is_empty()) {
                self.0 = None;
            }
            return true;
        }
        if let Some((ch, len, _)) = fence_run(line) {
            self.0 = Some((ch, len));
            return true;
        }
        false
    }
}
```

In `query.rs`, replace the open-coded fence state in `task_rows` with `let mut fences = Fences::new(); … if fences.skip(line) { continue; }`. Its query tests must still pass.

- [ ] **Step 4: Implement SF24 and SF25**

In `content.rs`:

```rust
/// SF24: a fallback feature product needs at least one `### Requirement:`
/// block, each with a title. Fenced lines do not count (bug d: SF10 passes
/// a product with zero requirements without a word).
pub fn requirement_population(r: &mut Report, text: &[u8], label: &[u8]) {
    let mut fences = Fences::new();
    let mut count = 0;
    for line in lines(text) {
        if fences.skip(line) {
            continue;
        }
        if let Some(rest) = line.strip_prefix(b"### Requirement:") {
            count += 1;
            if trim_space(rest).is_empty() {
                r.warn("SF24", &cat!(label, ": requirement with an empty title"));
            }
        }
    }
    if count == 0 {
        r.warn("SF24", &cat!(label, ": no '### Requirement:' blocks — the requirement checks examined nothing"));
    }
}

/// The lowercased first word of a `- ` or `* ` bullet, past leading `*`/`_`.
fn bullet_word(line: &[u8]) -> Option<Vec<u8>> {
    let t = trim_start_space(line);
    let rest = t.strip_prefix(b"- ").or_else(|| t.strip_prefix(b"* "))?;
    let rest = trim_start_space(rest);
    let start = rest.iter().position(|&c| c != b'*' && c != b'_')?;
    let word: Vec<u8> = rest
        .get(start..)?
        .iter()
        .take_while(|c| c.is_ascii_alphabetic())
        .map(u8::to_ascii_lowercase)
        .collect();
    (!word.is_empty()).then_some(word)
}

/// SF25: each `#### Scenario:` block (up to the next `#` line, fences
/// skipped) has a WHEN bullet and a THEN bullet (P3b-R18).
pub fn scenario_grammar(r: &mut Report, text: &[u8], label: &[u8]) {
    let finish = |r: &mut Report, open: Option<(Vec<u8>, bool, bool)>| {
        if let Some((title, when, then)) = open {
            if !when {
                r.warn("SF25", &cat!(label, ": scenario '", title, "' has no WHEN bullet"));
            }
            if !then {
                r.warn("SF25", &cat!(label, ": scenario '", title, "' has no THEN bullet"));
            }
        }
    };
    let mut fences = Fences::new();
    let mut open: Option<(Vec<u8>, bool, bool)> = None;
    for line in lines(text) {
        if fences.skip(line) {
            continue;
        }
        if let Some(rest) = line.strip_prefix(b"#### Scenario:") {
            finish(r, open.take());
            open = Some((trim_space(rest).to_vec(), false, false));
        } else if line.starts_with(b"#") {
            finish(r, open.take());
        } else if let Some((_, when, then)) = open.as_mut() {
            match bullet_word(line).as_deref() {
                Some(b"when") => *when = true,
                Some(b"then") => *then = true,
                _ => {}
            }
        }
    }
    finish(r, open.take());
}
```

In `validate.rs`:
- After every `content::requirements_scenarios(r, &text, X)` call, add `content::scenario_grammar(r, &text, X);`.
- In `feature`, after `content::feature_scope(...)` for `product.md`, add `content::requirement_population(r, &text, doc.as_bytes());`.

- [ ] **Step 5: Run the tests and parity**

```bash
cargo test --locked -p vibe-spec 2>&1 | grep -E 'test result|FAILED|panicked'
cargo clippy --workspace --all-targets --locked -- -D warnings 2>&1 | tail -1
cargo fmt --all --check && echo fmt-ok
bash tests/parity/run.sh 2>&1 | grep -E '^FAIL|parity\[|parity:'
```

Expected:
- Everything `ok`, and the matrix covers 18 checks.
- Parity unchanged: SF24 and SF25 are off in the corpus.

- [ ] **Step 6: Commit and land the bug-d mutant**

```bash
git add crates/vibe-spec
git commit -m "feat(spec): add requirement and scenario grammar"
```

Mutant: in `requirement_population`, change `if count == 0 {` to `if count == usize::MAX {`. Run `cargo test --locked -p vibe-spec --test grammar zero_requirements_are_reported`; it must FAIL. Confirm with `git diff --numstat`, restore with `git checkout -- crates/vibe-spec/src/content.rs`, and paste the FAIL line.

### Task 7: Frontmatter reader; SF26 values, SF27 children, SF28 feature rows

**Files:**
- Create: `crates/vibe-spec/src/frontmatter.rs`, `crates/vibe-spec/src/adherence.rs`
- Modify: `crates/vibe-spec/src/main.rs` (modules), `crates/vibe-spec/src/validate.rs` (`Ctx` becomes `pub` with `pub` fields; call `adherence::run`)
- Test: `crates/vibe-spec/tests/adherence.rs` (new), `crates/vibe-spec/tests/strictness.rs` (triggers SF26, SF27, SF28)

**Interfaces:**
- Consumes: `Report`, `text::{lines, lossy, trim_space, trim_start_space, is_space, is_date, md_files, sub_dirs, name_of, join_raw}`.
- Produces:
  - `frontmatter::{Value, Frontmatter, parse}`
    - `Value::{Scalar(String), Inline(Vec<String>), Block(Vec<String>)}`
    - `Frontmatter::get(&self, key: &str) -> Option<&Value>`
    - `Frontmatter::scalar(&self, key: &str) -> Option<&str>`
    - `Frontmatter::list(&self, key: &str) -> Vec<String>`
    - `parse(text: &[u8]) -> Option<Frontmatter>`
  - `adherence::{FLAT_TYPES, LAYERED_TYPES, FEATURE_TYPES}`
  - `adherence::flat_files(dir: &Path) -> Vec<PathBuf>`
  - `adherence::feature_files(dir: &Path) -> Vec<PathBuf>`
  - `adherence::rel_of(dir: &Path, file: &Path) -> String`
  - `adherence::run(r: &mut Report, ctx: &Ctx, files: &[PathBuf], layered: bool)`
  - `validate::Ctx<'a> { pub dir: &'a Path, pub project: &'a Path, pub display: &'a str }`

- [ ] **Step 1: Write the failing tests**

Unit tests at the bottom of `frontmatter.rs` (create the file with only these first):

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scalars_lists_and_blocks() {
        let fm = parse(b"---\r\ntype: \"entrypoint\"\r\nchildren: [a.md, 'b.md', ]\nscope: product\ncode:\n  - src/a.rs\n- src/b.rs\ntypography:\n  body: Inter\n# comment\nempty:\nupdated: 2026-01-15\n---\nbody: no\n").unwrap();
        assert_eq!(fm.scalar("type"), Some("entrypoint"));
        assert_eq!(fm.get("children"), Some(&Value::Inline(vec!["a.md".into(), "b.md".into()])));
        assert_eq!(fm.list("code"), ["src/a.rs", "src/b.rs"]);
        assert_eq!(fm.get("typography"), Some(&Value::Block(Vec::new())));
        assert_eq!(fm.list("empty"), Vec::<String>::new());
        assert_eq!(fm.list("scope"), ["product"]);
        assert_eq!(fm.scalar("body"), None);
        assert_eq!(fm.scalar("updated"), Some("2026-01-15"));
    }

    #[test]
    fn no_or_unclosed_frontmatter_is_none() {
        assert!(parse(b"# title\n").is_none());
        assert!(parse(b"---\ntype: x\n").is_none());
        assert!(parse(b"").is_none());
        assert!(parse(b"---\n---\n").is_some());
    }
}
```

`crates/vibe-spec/tests/adherence.rs`:

```rust
#![cfg(unix)]
#![allow(clippy::unwrap_used, clippy::expect_used)]
//! SF26–SF29: frontmatter values, children and backlinks, feature rows, freshness.
mod common;

use common::{project, validate_json, write};
use std::path::{Path, PathBuf};

const PRODUCT: &str = "---\ntype: entrypoint\nscope: product\nchildren:\nupdated: 2026-01-15\n---\n\n# P\n";
const PLAN: &str = "---\ntype: entrypoint\nscope: implementation\nchildren:\nupdated: 2026-01-15\n---\n\n| Feature | Status |\n|---|---|\n| [auth](features/auth/) | IN PROGRESS |\n| `billing` | NOT STARTED |\n";
const FPRODUCT: &str = "---\ntype: feature-product\nfeature: {f}\nparent: ../../product.md\nsibling: tech.md\nupdated: 2026-01-15\n---\n\n## Scope\n\n| Owns | Does not own |\n|---|---|\n| {f} things | other |\n\n### Requirement: A\nUsers MUST a.\n\n#### Scenario: a\n- WHEN x\n- THEN y\n";
const FTECH: &str = "---\ntype: feature-tech\nfeature: {f}\nparent: ../../tech.md\nsibling: product.md\nupdated: 2026-01-15\n---\n";

fn tree(name: &str, features: &[&str]) -> PathBuf {
    let p = project(name);
    write(&p, ".spec/product.md", PRODUCT);
    write(&p, ".spec/plan.md", PLAN);
    for f in features {
        write(&p, &format!(".spec/features/{f}/product.md"), &FPRODUCT.replace("{f}", f));
        write(&p, &format!(".spec/features/{f}/tech.md"), &FTECH.replace("{f}", f));
    }
    p
}

fn found(p: &Path, check: &str) -> Vec<String> {
    let (_, v) = validate_json(p, &[], &[]);
    v["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["check"] == check)
        .map(|f| f["message"].as_str().unwrap().to_string())
        .collect()
}

#[test]
fn a_clean_tree_has_no_adherence_findings() {
    let p = tree("ad-clean", &["auth", "billing"]);
    for check in ["SF26", "SF27", "SF28"] {
        assert_eq!(found(&p, check), Vec::<String>::new(), "{check}");
    }
}

#[test]
fn frontmatter_values_are_checked() {
    let p = tree("ad-values", &["auth"]);
    write(&p, ".spec/product.md", &PRODUCT.replace("scope: product", "scope: marketing").replace("2026-01-15", "{YYYY-MM-DD}"));
    write(&p, ".spec/features/auth/tech.md", &FTECH.replace("{f}", "auth").replace("feature-tech", "feature-notes"));
    let got = found(&p, "SF26");
    assert!(got.contains(&"product.md: scope 'marketing' is not one of product, technical, design, implementation".to_string()), "{got:?}");
    assert!(got.contains(&"product.md: updated '{YYYY-MM-DD}' is not a YYYY-MM-DD date".to_string()), "{got:?}");
    assert!(got.iter().any(|m| m.starts_with("features/auth/tech.md: type 'feature-notes' is not one of")), "{got:?}");
}

#[test]
fn inline_children_must_exist_and_backlink() {
    let p = tree("ad-children", &[]);
    write(&p, ".spec/tech.md", "---\ntype: entrypoint\nscope: technical\nchildren: [tech-api.md, tech-gone.md, tech-{topic}.md]\nupdated: 2026-01-15\n---\n");
    write(&p, ".spec/tech-api.md", "---\ntype: tech-topic\nparent: product.md\nscope: api\ncovers: x\nupdated: 2026-01-15\n---\n");
    let got = found(&p, "SF27");
    assert_eq!(
        got,
        [
            "tech-api.md: parent is 'product.md', but tech.md lists it as a child",
            "tech.md: child 'tech-gone.md' listed in frontmatter but file doesn't exist",
        ],
        "{got:?}"
    );
}

#[test]
fn a_missing_block_child_is_reported_once() {
    let p = tree("ad-block", &[]);
    write(&p, ".spec/tech.md", "---\ntype: entrypoint\nscope: technical\nchildren:\n  - tech-gone.md\nupdated: 2026-01-15\n---\n");
    assert_eq!(found(&p, "SF27"), Vec::<String>::new());
    assert_eq!(found(&p, "SF22").len(), 1);
}

#[test]
fn feature_folders_need_a_plan_row() {
    let p = tree("ad-rows", &["auth", "billing", "search"]);
    assert_eq!(found(&p, "SF28"), ["features/search/: no row in plan.md names 'search'"]);
    std::fs::remove_file(p.join(".spec/plan.md")).unwrap();
    assert_eq!(found(&p, "SF28").len(), 3);
}
```

In `tests/strictness.rs`, remove `"SF26"`, `"SF27"` and `"SF28"` from `PENDING`. Add the following arms:

```rust
        "SF26" => {
            write(&p, ".spec/product.md", "---\ntype: entry\nscope: product\nchildren:\nupdated: 2026-01-15\n---\n\n# P\n");
            plain(p, "type 'entry' is not one of")
        }
        "SF27" => {
            write(&p, ".spec/tech.md", "---\ntype: entrypoint\nscope: technical\nchildren: [tech-api.md]\nupdated: 2026-01-15\n---\n");
            plain(p, "child 'tech-api.md' listed in frontmatter")
        }
        "SF28" => {
            write(&p, ".spec/features/billing/product.md", &FEATURE_PRODUCT.replace("feature: auth", "feature: billing").replace("sessions", "invoices"));
            write(&p, ".spec/features/billing/tech.md", &FEATURE_TECH.replace("feature: auth", "feature: billing"));
            plain(p, "no row in plan.md names 'billing'")
        }
```

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test --locked -p vibe-spec 2>&1 | grep -E 'error\[|^test .*FAILED|test result'`
Expected: the build fails because `parse` is undefined. Once Step 3 adds stubs, the adherence tests fail on empty finding lists. Paste the output.

- [ ] **Step 3: Implement `frontmatter.rs`**

```rust
//! A YAML-subset frontmatter reader for the F17 checks: `key: value`
//! scalars, inline lists `[a, b]`, block lists (`key:` then `- item`
//! lines). Nested maps, comments and anything else are skipped. Values are
//! lossy UTF-8 with surrounding quotes removed.
use crate::text::{is_space, lines, lossy, trim_space, trim_start_space};

/// One frontmatter value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Value {
    /// `key: value`.
    Scalar(String),
    /// `key: [a, b]`.
    Inline(Vec<String>),
    /// `key:` followed by zero or more `- item` lines.
    Block(Vec<String>),
}

/// Parsed frontmatter, keys in file order.
#[derive(Debug, Default)]
pub struct Frontmatter {
    entries: Vec<(String, Value)>,
}

impl Frontmatter {
    /// The value of `key`.
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.entries.iter().find(|(k, _)| k == key).map(|(_, v)| v)
    }

    /// A scalar value.
    pub fn scalar(&self, key: &str) -> Option<&str> {
        match self.get(key)? {
            Value::Scalar(s) => Some(s),
            _ => None,
        }
    }

    /// A list's items; a non-empty scalar is a one-item list.
    pub fn list(&self, key: &str) -> Vec<String> {
        match self.get(key) {
            Some(Value::Inline(v) | Value::Block(v)) => v.clone(),
            Some(Value::Scalar(s)) if !s.is_empty() => vec![s.clone()],
            _ => Vec::new(),
        }
    }
}

fn unquote(s: &[u8]) -> String {
    let t = trim_space(s);
    let quoted = t.len() >= 2
        && matches!((t.first(), t.last()), (Some(b'"'), Some(b'"')) | (Some(b'\''), Some(b'\'')));
    lossy(if quoted { t.get(1..t.len() - 1).unwrap_or_default() } else { t })
}

/// The frontmatter between a first line `---` and the next `---` line
/// (`\r` and blanks around the dashes ignored). `None` without one, or when
/// it never closes.
pub fn parse(text: &[u8]) -> Option<Frontmatter> {
    let all = lines(text);
    let mut it = all.iter();
    if trim_space(it.next()?) != b"---" {
        return None;
    }
    let mut fm = Frontmatter::default();
    for raw in it {
        let line = raw.strip_suffix(b"\r").unwrap_or(raw);
        if trim_space(line) == b"---" {
            return Some(fm);
        }
        let t = trim_start_space(line);
        if let Some(rest) = t.strip_prefix(b"- ").or_else(|| (t == b"-").then_some(&b""[..])) {
            if let Some((_, Value::Block(items))) = fm.entries.last_mut() {
                items.push(unquote(rest));
            }
            continue;
        }
        if line.first().is_none_or(|&c| is_space(c) || c == b'#') {
            continue;
        }
        let Some(colon) = line.iter().position(|&c| c == b':') else {
            continue;
        };
        let key = lossy(trim_space(line.get(..colon).unwrap_or_default()));
        let rest = trim_space(line.get(colon + 1..).unwrap_or_default());
        let value = if rest.is_empty() {
            Value::Block(Vec::new())
        } else if rest.len() >= 2 && rest.first() == Some(&b'[') && rest.last() == Some(&b']') {
            let inner = rest.get(1..rest.len() - 1).unwrap_or_default();
            Value::Inline(inner.split(|&c| c == b',').map(unquote).filter(|s| !s.is_empty()).collect())
        } else {
            Value::Scalar(unquote(rest))
        };
        fm.entries.push((key, value));
    }
    None
}
```

Note that a block item belongs to the last key only when that key's value is a `Block`. `children:` followed by `  - a.md` gives `Block(["a.md"])`. A nested map such as `typography:\n  body: Inter` stays `Block([])`.

- [ ] **Step 4: Implement `adherence.rs`**

```rust
//! F17 adherence checks over the tree (P3b-R11): SF26 frontmatter values,
//! SF27 children and backlinks, SF28 feature folders in the plan. Task 8
//! adds SF29 freshness.
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use crate::frontmatter::{self, Frontmatter, Value};
use crate::report::Report;
use crate::text::{is_date, join_raw, lines, lossy, md_files, name_of, sub_dirs, trim_space, trim_start_space};
use crate::validate::Ctx;

/// Flat-layout `type`s outside features.
pub const FLAT_TYPES: [&str; 4] = ["entrypoint", "product-topic", "tech-topic", "plan-topic"];
/// Layered-layout `type`s (D44).
pub const LAYERED_TYPES: [&str; 8] = ["hub", "area", "flow", "concept", "platform", "tech", "decision", "question"];
/// Fallback feature `type`s (both layouts).
pub const FEATURE_TYPES: [&str; 5] = ["feature-product", "feature-tech", "feature-design", "feature-plan", "feature-research"];
const SCOPES: [&str; 4] = ["product", "technical", "design", "implementation"];
const ENTRYPOINTS: [&str; 4] = ["product.md", "tech.md", "design.md", "plan.md"];

/// `file` relative to `dir`, `/`-joined.
pub fn rel_of(dir: &Path, file: &Path) -> String {
    file.strip_prefix(dir)
        .map(|r| r.components().map(|c| c.as_os_str().to_string_lossy().into_owned()).collect::<Vec<_>>().join("/"))
        .unwrap_or_else(|_| file.to_string_lossy().into_owned())
}

/// Each `features/<f>/{product,tech,design,plan,research}.md` that exists.
pub fn feature_files(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for fdir in sub_dirs(&dir.join("features")) {
        for doc in ["product", "tech", "design", "plan", "research"] {
            let p = fdir.join(format!("{doc}.md"));
            if p.is_file() {
                out.push(p);
            }
        }
    }
    out
}

/// The flat files SF26 reads: root `*.md` but `lessons.md`, then the feature docs.
pub fn flat_files(dir: &Path) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = md_files(dir).into_iter().filter(|p| name_of(p) != "lessons.md").collect();
    out.extend(feature_files(dir));
    out
}

/// SF26 on `files`, SF27 (flat only), SF28.
pub fn run(r: &mut Report, ctx: &Ctx, files: &[PathBuf], layered: bool) {
    for file in files {
        let rel = rel_of(ctx.dir, file);
        r.examine(&format!("{}/{rel}", ctx.display));
        let text = std::fs::read(file).unwrap_or_default();
        if let Some(fm) = frontmatter::parse(&text) {
            values(r, &rel, &fm, layered);
        }
    }
    if !layered {
        children(r, ctx);
    }
    feature_rows(r, ctx);
}

fn values(r: &mut Report, rel: &str, fm: &Frontmatter, layered: bool) {
    if let Some(t) = fm.scalar("type") {
        let vocabulary: Vec<&str> = if layered { LAYERED_TYPES.to_vec() } else { FLAT_TYPES.to_vec() }
            .into_iter()
            .chain(FEATURE_TYPES)
            .collect();
        if !vocabulary.contains(&t) {
            r.warn("SF26", format!("{rel}: type '{t}' is not one of {}", vocabulary.join(", ")).as_bytes());
        }
        if t == "entrypoint"
            && let Some(s) = fm.scalar("scope")
            && !SCOPES.contains(&s)
        {
            r.warn("SF26", format!("{rel}: scope '{s}' is not one of {}", SCOPES.join(", ")).as_bytes());
        }
    }
    if let Some(u) = fm.scalar("updated")
        && !is_date(u.as_bytes())
    {
        r.warn("SF26", format!("{rel}: updated '{u}' is not a YYYY-MM-DD date").as_bytes());
    }
}

/// SF27. A missing child from a block list is legacy SF22's error, so only
/// inline lists report existence here.
fn children(r: &mut Report, ctx: &Ctx) {
    for name in ENTRYPOINTS {
        let Some(fm) = std::fs::read(ctx.dir.join(name)).ok().and_then(|t| frontmatter::parse(&t)) else {
            continue;
        };
        let inline = matches!(fm.get("children"), Some(Value::Inline(_)));
        for child in fm.list("children") {
            if child.contains('{') {
                continue;
            }
            let path = join_raw(ctx.dir, child.as_bytes());
            if !path.is_file() {
                if inline {
                    r.at(&format!("{}/{name}", ctx.display));
                    r.warn("SF27", format!("{name}: child '{child}' listed in frontmatter but file doesn't exist").as_bytes());
                }
                continue;
            }
            let parent = std::fs::read(&path)
                .ok()
                .and_then(|t| frontmatter::parse(&t))
                .and_then(|f| f.scalar("parent").map(str::to_string));
            if let Some(p) = parent
                && p != name
            {
                r.at(&format!("{}/{child}", ctx.display));
                r.warn("SF27", format!("{child}: parent is '{p}', but {name} lists it as a child").as_bytes());
            }
        }
    }
}

/// Every table cell of `text`: `` ` `` and `*` dropped, `[text](…)` → `text`, trimmed.
fn row_cells(text: &[u8]) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for line in lines(text) {
        let t = trim_start_space(line);
        if !t.starts_with(b"|") {
            continue;
        }
        for cell in t.split(|&c| c == b'|') {
            let kept: Vec<u8> = cell.iter().copied().filter(|&c| c != b'`' && c != b'*').collect();
            let mut s = trim_space(&kept).to_vec();
            if s.first() == Some(&b'[')
                && let Some(end) = s.iter().position(|&c| c == b']')
            {
                s = s.get(1..end).unwrap_or_default().to_vec();
            }
            out.insert(lossy(trim_space(&s)));
        }
    }
    out
}

/// SF28: each feature folder is named by a cell of a root plan table row (P3b-R20).
fn feature_rows(r: &mut Report, ctx: &Ctx) {
    let features = sub_dirs(&ctx.dir.join("features"));
    if features.is_empty() {
        return;
    }
    r.at(&format!("{}/plan.md", ctx.display));
    let plan = std::fs::read(ctx.dir.join("plan.md")).ok();
    let cells = plan.as_deref().map(row_cells).unwrap_or_default();
    for fdir in features {
        let f = name_of(&fdir);
        if plan.is_none() {
            r.warn("SF28", format!("features/{f}/: no root plan.md lists it").as_bytes());
        } else if !cells.contains(&f) {
            r.warn("SF28", format!("features/{f}/: no row in plan.md names '{f}'").as_bytes());
        }
    }
}
```

In `validate.rs`, make `Ctx` and its fields `pub`. After `sf16(...)`, add:

```rust
    let files = adherence::flat_files(&dir);
    adherence::run(&mut r, &ctx, &files, false);
```

Add `mod adherence;` and `mod frontmatter;` to `main.rs`.

- [ ] **Step 5: Run the tests and parity**

```bash
cargo test --locked -p vibe-spec 2>&1 | grep -E 'test result|FAILED|panicked'
cargo clippy --workspace --all-targets --locked -- -D warnings 2>&1 | tail -1
cargo fmt --all --check && echo fmt-ok
bash tests/parity/run.sh 2>&1 | grep -E '^FAIL|parity\[|parity:'
```

Expected:
- Everything `ok`, and the matrix covers 21 checks.
- Parity unchanged.

- [ ] **Step 6: Commit**

```bash
git add crates/vibe-spec
git commit -m "feat(spec): check frontmatter, children, plan rows"
```

### Task 8: SF29 freshness through git

**Files:**
- Create: `crates/vibe-spec/src/git.rs`
- Modify: `crates/vibe-spec/src/adherence.rs` (`freshness`, called from `run`), `crates/vibe-spec/src/main.rs` (module), `crates/vibe-spec/tests/common/mod.rs` (`git`, `git_out`)
- Test: `crates/vibe-spec/tests/adherence.rs`, `crates/vibe-spec/tests/strictness.rs` (trigger SF29)

**Interfaces:**
- Consumes: `frontmatter::parse`, `adherence::rel_of`.
- Produces:
  - `git::top(dir: &Path) -> Option<PathBuf>`
  - `git::has_commits(top: &Path) -> bool`
  - `git::last_dates(top: &Path, under: &str) -> BTreeMap<String, String>`
  - Test helpers `common::git(cwd: &Path, date: &str, args: &[&str])` and `common::git_out(cwd: &Path, args: &[&str]) -> String`. Both pass `-c commit.gpgsign=false -c tag.gpgsign=false -c core.hooksPath=/dev/null` plus a fixed identity, and panic on a non-zero exit (lesson "Sandbox git repos in tests inherit the user's signing config").

- [ ] **Step 1: Write the failing tests**

Add to `tests/common/mod.rs`:

```rust
fn git_cmd(cwd: &Path, args: &[&str]) -> Command {
    let mut cmd = Command::new("git");
    cmd.args([
        "-c", "commit.gpgsign=false",
        "-c", "tag.gpgsign=false",
        "-c", "core.hooksPath=/dev/null",
        "-c", "user.email=t@t",
        "-c", "user.name=t",
        "-c", "init.defaultBranch=main",
    ])
    .args(args)
    .current_dir(cwd)
    .env_remove("GIT_DIR")
    .env_remove("GIT_WORK_TREE");
    cmd
}

/// Runs `git args` in `cwd` with author and committer date `date`; panics on failure.
pub fn git(cwd: &Path, date: &str, args: &[&str]) {
    let out = git_cmd(cwd, args)
        .env("GIT_AUTHOR_DATE", date)
        .env("GIT_COMMITTER_DATE", date)
        .output()
        .unwrap();
    assert!(out.status.success(), "git {args:?}: {}", String::from_utf8_lossy(&out.stderr));
}

/// `git args` stdout, trimmed; panics on failure.
pub fn git_out(cwd: &Path, args: &[&str]) -> String {
    let out = git_cmd(cwd, args).output().unwrap();
    assert!(out.status.success(), "git {args:?}: {}", String::from_utf8_lossy(&out.stderr));
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}
```

Append to `tests/adherence.rs`:

```rust
use common::git;

fn committed(name: &str, date: &str) -> PathBuf {
    let p = tree(name, &["auth"]);
    git(&p, date, &["init", "-q"]);
    git(&p, date, &["add", "-A"]);
    git(&p, date, &["commit", "-q", "-m", "init"]);
    p
}

#[test]
fn stale_updated_is_reported() {
    let p = committed("ad-stale", "2026-03-01T12:00:00Z");
    let got = found(&p, "SF29");
    assert!(got.contains(&"product.md: updated 2026-01-15 is older than its last commit (2026-03-01)".to_string()), "{got:?}");
    assert!(got.contains(&"features/auth/product.md: updated 2026-01-15 is older than its last commit (2026-03-01)".to_string()), "{got:?}");
}

#[test]
fn fresh_updated_is_silent() {
    let p = committed("ad-fresh", "2026-01-15T12:00:00Z");
    assert_eq!(found(&p, "SF29"), Vec::<String>::new());
}

#[test]
fn freshness_is_silent_without_history() {
    let bare = tree("ad-nogit", &["auth"]);
    assert_eq!(found(&bare, "SF29"), Vec::<String>::new(), "empty .git dir");
    let fresh = tree("ad-nocommit", &["auth"]);
    git(&fresh, "2026-03-01T12:00:00Z", &["init", "-q"]);
    assert_eq!(found(&fresh, "SF29"), Vec::<String>::new(), "no commits");
    let p = committed("ad-untracked", "2026-01-15T12:00:00Z");
    write(&p, ".spec/tech.md", "---\ntype: entrypoint\nscope: technical\nchildren:\nupdated: 2020-01-01\n---\n");
    assert_eq!(found(&p, "SF29"), Vec::<String>::new(), "untracked file");
    let empty = p.join("nogit-bin");
    std::fs::create_dir_all(&empty).unwrap();
    let path = empty.display().to_string();
    let (code, v) = validate_json(&committed("ad-nobin", "2026-03-01T12:00:00Z"), &[], &[("PATH", path.as_str())]);
    assert_eq!(code, 0);
    assert!(v["findings"].as_array().unwrap().iter().all(|f| f["check"] != "SF29"), "no git binary");
}

#[test]
fn an_outside_root_is_silent() {
    let p = committed("ad-outside", "2026-03-01T12:00:00Z");
    let outside = tree("ad-outside-root", &["auth"]);
    let root = outside.join(".spec");
    let out = common::spec(&p, &["--root", root.to_str().unwrap(), "validate", "--json"], &[]);
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert!(v["findings"].as_array().unwrap().iter().all(|f| f["check"] != "SF29"), "{v:#}");
}
```

In `tests/strictness.rs`, remove `"SF29"` from `PENDING` and add:

```rust
        "SF29" => {
            common::git(&p, "2026-03-01T12:00:00Z", &["init", "-q"]);
            common::git(&p, "2026-03-01T12:00:00Z", &["add", "-A"]);
            common::git(&p, "2026-03-01T12:00:00Z", &["commit", "-q", "-m", "init"]);
            plain(p, "older than its last commit")
        }
```

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test --locked -p vibe-spec --test adherence --test strictness 2>&1 | grep -E '^test |test result'`
Expected: `stale_updated_is_reported` and `warn_error_off_matrix` FAIL (no SF29 findings). The silent cases pass. Paste the output.

- [ ] **Step 3: Implement `git.rs`**

```rust
//! `git` as a subprocess (no `git2`). Every function degrades to "nothing"
//! when git is missing, the directory is outside a work tree, or the
//! repository has no commits.
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::text::lossy;

/// `git -C dir -c core.quotepath=off <args>` stdout when it exits 0.
pub(crate) fn git(dir: &Path, args: &[&str]) -> Option<Vec<u8>> {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(["-c", "core.quotepath=off"])
        .args(args)
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    out.status.success().then_some(out.stdout)
}

/// The top directory of the work tree holding `dir`.
pub fn top(dir: &Path) -> Option<PathBuf> {
    let out = git(dir, &["rev-parse", "--show-toplevel"])?;
    let s = String::from_utf8(out).ok()?;
    let t = s.trim_end_matches('\n');
    (!t.is_empty()).then(|| PathBuf::from(t))
}

/// `HEAD` resolves.
pub fn has_commits(top: &Path) -> bool {
    git(top, &["rev-parse", "--verify", "-q", "HEAD"]).is_some()
}

/// For each file under `under` (top-relative, `/`-joined, `.` for the
/// top), the committer date `YYYY-MM-DD` of the last commit touching it.
pub fn last_dates(top: &Path, under: &str) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    if !has_commits(top) {
        return out;
    }
    let Some(log) = git(top, &["log", "--format=%x00%cs", "--name-only", "--", under]) else {
        return out;
    };
    for record in log.split(|&b| b == 0).skip(1) {
        let mut rows = record.split(|&b| b == b'\n');
        let date = lossy(rows.next().unwrap_or_default());
        for name in rows.filter(|l| !l.is_empty()) {
            out.entry(lossy(name)).or_insert_with(|| date.clone());
        }
    }
    out
}
```

- [ ] **Step 4: Implement `freshness` and call it**

In `adherence.rs`:

```rust
fn slash(p: &Path) -> String {
    p.components().map(|c| c.as_os_str().to_string_lossy().into_owned()).collect::<Vec<_>>().join("/")
}

/// SF29: `updated:` older than the committer date of the file's last commit
/// (P3b-R21). Silent without git, history, a commit for the file, or when
/// the root lies outside the work tree.
fn freshness(r: &mut Report, ctx: &Ctx, files: &[PathBuf]) {
    let Ok(dir) = std::fs::canonicalize(ctx.dir) else { return };
    let Some(top) = git::top(&dir) else { return };
    let Some(top) = std::fs::canonicalize(&top).ok() else { return };
    let Ok(under) = dir.strip_prefix(&top) else { return };
    let under = slash(under);
    let dates = git::last_dates(&top, if under.is_empty() { "." } else { &under });
    if dates.is_empty() {
        return;
    }
    for file in files {
        let Ok(abs) = std::fs::canonicalize(file) else { continue };
        let Ok(in_top) = abs.strip_prefix(&top) else { continue };
        let Some(date) = dates.get(&slash(in_top)) else { continue };
        let text = std::fs::read(file).unwrap_or_default();
        let Some(fm) = frontmatter::parse(&text) else { continue };
        let Some(updated) = fm.scalar("updated") else { continue };
        if is_date(updated.as_bytes()) && updated < date.as_str() {
            let rel = rel_of(ctx.dir, file);
            r.at(&format!("{}/{rel}", ctx.display));
            r.warn("SF29", format!("{rel}: updated {updated} is older than its last commit ({date})").as_bytes());
        }
    }
}
```

Call `freshness(r, ctx, files);` at the end of `adherence::run`. Add `use crate::git;`, add `mod git;` to `main.rs`, and update the module doc ("… Task 8 adds" → "SF29 freshness").

- [ ] **Step 5: Run the tests and parity**

```bash
cargo test --locked -p vibe-spec 2>&1 | grep -E 'test result|FAILED|panicked'
cargo clippy --workspace --all-targets --locked -- -D warnings 2>&1 | tail -1
cargo fmt --all --check && echo fmt-ok
bash tests/parity/run.sh 2>&1 | grep -E '^FAIL|parity\[|parity:'
```

Expected:
- Everything `ok`, and the matrix covers 22 checks.
- Parity unchanged. The sandboxes have no history, and SF29 is off there anyway.

- [ ] **Step 6: Commit**

```bash
git add crates/vibe-spec
git commit -m "feat(spec): flag updated dates older than commits"
```

### Task 9: Layered layout: detection, SF30 allowlist, SF31 budgets, SF32 names, tree links

**Files:**
- Create: `crates/vibe-spec/src/layered.rs`
- Modify: `crates/vibe-spec/src/validate.rs` (layout switch), `crates/vibe-spec/src/root.rs` (`root --json` reports `layout`), `crates/vibe-spec/src/main.rs` (module), `crates/vibe-spec/tests/common/mod.rs` (`layered_tree`)
- Test: `crates/vibe-spec/tests/layered.rs` (new), `crates/vibe-spec/tests/root.rs` (layout), `crates/vibe-spec/tests/strictness.rs` (triggers SF30–SF32)

**Interfaces:**
- Consumes: `frontmatter::parse`, `pattern::{md_links, md_link_target}`, `text::{Fences, join_raw, lines}`, `adherence::{feature_files, rel_of}`.
- Produces:
  - `layered::{Layout, detect, Place, place, KINDS, walk, spec_files, run}`
    - `Layout::{Flat, Layered}`, `Layout::as_str(self) -> &'static str` (`flat`/`layered`)
    - `detect(dir: &Path) -> Layout`
    - `Place::{Hub, Readme, Lessons, PartHub, Note(&'static str), Feature, Exempt, Other}`
    - `place(parts: &[&str]) -> Place`
    - `walk(dir: &Path) -> Vec<(PathBuf, Vec<String>)>` (non-hidden files, byte-sorted, root-relative components; symlinks are not followed)
    - `spec_files(dir: &Path) -> Vec<PathBuf>` (hubs, READMEs, part hubs, notes, feature docs)
    - `run(r: &mut Report, ctx: &Ctx)`
  - `common::layered_tree(name: &str) -> PathBuf`: a clean layered project under `docs/spec/`.

- [ ] **Step 1: Write the failing tests**

Add to `tests/common/mod.rs`:

```rust
/// A clean layered project (D44): hubs, README/AGENTS, every kind folder,
/// a nested topic folder with its own README hub, and a part.
pub fn layered_tree(name: &str) -> PathBuf {
    let p = project(name);
    let hub = |title: &str, body: &str| format!("---\ntype: hub\nupdated: 2026-01-15\n---\n\n# {title}\n\n{body}\n");
    let note = |t: &str, extra: &str, body: &str| format!("---\ntype: {t}\n{extra}updated: 2026-01-15\n---\n\n{body}\n");
    write(&p, "docs/spec/README.md", &hub("Spec", "[Product](product.md) · [Tech](tech.md) · [Design](design.md) · [Plan](plan.md) · [How to write](AGENTS.md)"));
    write(&p, "docs/spec/AGENTS.md", "# Writing the spec\n\nOne fact, one place. Link every note from a hub.\n");
    write(&p, "docs/spec/product.md", &hub("Product", "- [Billing](areas/billing/README.md)\n- [Checkout](flows/checkout.md)\n- [Invoice](concepts/invoice.md)\n- [Mobile](parts/mobile/product.md)"));
    write(&p, "docs/spec/tech.md", &hub("Tech", "- [Rust](tech/rust.md)\n- [Errors](platform/errors.md)\n- [Use Rust](decisions/d01-use-rust.md)\n- [Pricing](questions/pricing.md)"));
    write(&p, "docs/spec/design.md", &hub("Design", "Plain text."));
    write(&p, "docs/spec/plan.md", &hub("Plan", "| Feature | Status |\n|---|---|\n| checkout | NOT STARTED |"));
    write(&p, "docs/spec/lessons.md", "# Lessons\n");
    write(&p, "docs/spec/areas/billing/README.md", &hub("Billing", "- [Invoices](invoices.md)"));
    write(&p, "docs/spec/areas/billing/invoices.md", &note("area", "code: src/billing.rs\n", "Issued monthly. See [checkout](../../flows/checkout.md)."));
    write(&p, "docs/spec/flows/checkout.md", &note("flow", "", "Cart to payment."));
    write(&p, "docs/spec/concepts/invoice.md", &note("concept", "", "A bill."));
    write(&p, "docs/spec/platform/errors.md", &note("platform", "", "Errors are typed."));
    write(&p, "docs/spec/tech/rust.md", &note("tech", "applies: \"crates/**/*.rs\"\n", "Edition 2024."));
    write(&p, "docs/spec/decisions/d01-use-rust.md", &note("decision", "", "We use Rust."));
    write(&p, "docs/spec/questions/pricing.md", &note("question", "", "Monthly or yearly?"));
    write(&p, "docs/spec/parts/mobile/product.md", &hub("Mobile", "- [Screens](areas/screens.md)"));
    write(&p, "docs/spec/parts/mobile/areas/screens.md", &note("area", "code: [src/billing.rs]\n", "Two screens."));
    write(&p, "src/billing.rs", "// billing\n");
    p
}
```

Create `crates/vibe-spec/tests/layered.rs`:

```rust
#![cfg(unix)]
#![allow(clippy::unwrap_used, clippy::expect_used)]
//! The layered layout (D44): detection and SF22/SF30–SF34 (P3b-R10, R11).
mod common;

use common::{layered_tree, spec, validate_json, write};
use std::path::Path;

fn findings(p: &Path) -> Vec<(String, String)> {
    let (_, v) = validate_json(p, &[], &[]);
    v["findings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| (f["check"].as_str().unwrap().to_string(), f["message"].as_str().unwrap().to_string()))
        .collect()
}

fn append(p: &Path, rel: &str, text: &str) {
    let path = p.join(rel);
    let mut body = std::fs::read_to_string(&path).unwrap();
    body.push_str(text);
    std::fs::write(path, body).unwrap();
}

#[test]
fn a_hub_typed_root_is_layered() {
    let p = layered_tree("ly-detect");
    let (code, v) = validate_json(&p, &[], &[]);
    assert_eq!(v["layout"], "layered");
    assert_eq!(v["findings"], serde_json::json!([]), "{v:#}");
    assert_eq!(code, 0);
    assert!(v["examined"].as_u64().unwrap() >= 16, "{v:#}");
    let root = spec(&p, &["root", "--json"], &[]);
    assert!(root.text().contains("\"layout\":\"layered\""), "{}", root.text());
    let text = spec(&p, &["validate"], &[]).text();
    assert!(!text.contains("--- README.md ---") && !text.contains("--- AGENTS.md ---"), "{text}");
}

#[test]
fn layout_rules_fail_when_broken() {
    let cases: [(&str, &str, &str, fn(&Path)); 7] = [
        ("ly-other", "SF30", "notes.txt: not in the layered layout", |p| write(p, "docs/spec/notes.txt", "x\n")),
        ("ly-stray", "SF30", "random/x.md: not in the layered layout", |p| write(p, "docs/spec/random/x.md", "x\n")),
        ("ly-note-budget", "SF31", "flows/checkout.md: 68 lines, over the 60-line note budget", |p| append(p, "docs/spec/flows/checkout.md", &"x\n".repeat(62))),
        ("ly-hub-budget", "SF31", "over the 80-line hub budget", |p| append(p, "docs/spec/tech.md", &"x\n".repeat(80))),
        ("ly-kebab", "SF32", "concepts/Invoice.md: 'Invoice.md' is not lowercase-kebab", |p| {
            std::fs::rename(p.join("docs/spec/concepts/invoice.md"), p.join("docs/spec/concepts/Invoice.md")).unwrap();
            let hub = std::fs::read_to_string(p.join("docs/spec/product.md")).unwrap().replace("concepts/invoice.md", "concepts/Invoice.md");
            std::fs::write(p.join("docs/spec/product.md"), hub).unwrap();
        }),
        ("ly-unique", "SF32", "flows/invoice.md: name 'invoice' is also used by concepts/invoice.md", |p| {
            write(p, "docs/spec/flows/invoice.md", "---\ntype: flow\nupdated: 2026-01-15\n---\n\nPaying one.\n");
            append(p, "docs/spec/product.md", "- [Pay](flows/invoice.md)\n");
        }),
        ("ly-link", "SF22", "flows/checkout.md: broken link to 'gone.md'", |p| append(p, "docs/spec/flows/checkout.md", "[x](gone.md) [ok](#top) [web](https://x.io) [t]({x}.md)\n")),
    ];
    for (name, check, needle, mutate) in cases {
        let p = layered_tree(name);
        mutate(&p);
        let got = findings(&p);
        assert!(got.iter().any(|(c, m)| c == check && m.contains(needle)), "{name}: want {check} {needle:?}\n{got:#?}");
    }
}

#[test]
fn layered_edge_inputs() {
    let p = layered_tree("ly-edge");
    std::fs::write(p.join("docs/spec/flows/checkout.md"), "---\r\ntype: flow\r\nupdated: 2026-01-15\r\n---\r\n\r\nCart.\r\n").unwrap();
    write(&p, "docs/spec/areas/.DS_Store", "binary");
    let budget = format!("---\ntype: concept\nupdated: 2026-01-15\n---\n{}last line, no newline", "x\n".repeat(56));
    std::fs::write(p.join("docs/spec/concepts/invoice.md"), budget).unwrap();
    let got = findings(&p);
    assert!(got.iter().any(|(c, m)| c == "SF31" && m.starts_with("concepts/invoice.md: 61 lines")), "{got:#?}");
    assert!(!got.iter().any(|(c, _)| c == "SF30"), "dotfile must be ignored: {got:#?}");
    assert!(!got.iter().any(|(_, m)| m.contains("flows/checkout.md")), "CRLF note is fine: {got:#?}");
}

#[test]
fn a_flat_root_stays_flat() {
    let p = common::project("ly-flat");
    write(&p, ".spec/product.md", "---\ntype: entrypoint\nscope: product\nchildren:\nupdated: 2026-01-15\n---\n");
    write(&p, ".spec/notes.txt", "x\n");
    let (_, v) = validate_json(&p, &[], &[]);
    assert_eq!(v["layout"], "flat");
    assert!(v["findings"].as_array().unwrap().iter().all(|f| f["check"] != "SF30"));
}
```

In `tests/root.rs`, add an assertion to an existing `--json` test: a flat `.spec/` root reports `"layout":"flat"`.

In `tests/strictness.rs`, remove `"SF30"`, `"SF31"` and `"SF32"` from `PENDING`. Add these arms; their trees come from `common::layered_tree`, not from `base`:

```rust
        "SF30" => {
            let p = common::layered_tree("mx-sf30-l");
            write(&p, "docs/spec/notes.txt", "x\n");
            plain(p, "not in the layered layout")
        }
        "SF31" => {
            let p = common::layered_tree("mx-sf31-l");
            write(&p, "docs/spec/flows/checkout.md", &format!("---\ntype: flow\nupdated: 2026-01-15\n---\n{}", "x\n".repeat(70)));
            plain(p, "over the 60-line note budget")
        }
        "SF32" => {
            let p = common::layered_tree("mx-sf32-l");
            write(&p, "docs/spec/flows/invoice.md", "---\ntype: flow\nupdated: 2026-01-15\n---\n\nPay.\n");
            plain(p, "is also used by")
        }
```

(`base(&format!("mx-…"))` still runs first in `trigger`. These arms shadow `p`, and that is fine.)

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test --locked -p vibe-spec --test layered --test strictness --test root 2>&1 | grep -E '^test |test result'`
Expected:
- `a_hub_typed_root_is_layered` FAILS: `layout` is `flat`, and README.md is printed by the root loop with SF20 and SF21 findings.
- `layout_rules_fail_when_broken` and the matrix FAIL.
- `a_flat_root_stays_flat` passes.

Paste the output.

- [ ] **Step 3: Implement `layered.rs`**

```rust
//! The layered layout (D44, P3b-R10, R11): detection, the tree walk, and
//! the layered-only checks — SF22 links across notes, SF30 allowlist, SF31
//! budgets, SF32 names (Task 10 adds SF33, SF34).
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::adherence::feature_files;
use crate::frontmatter;
use crate::pattern::{md_link_target, md_links};
use crate::report::Report;
use crate::text::{Fences, join_raw, lines};
use crate::validate::Ctx;

/// A root's layout.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Layout {
    /// Legacy `.spec/`-style layout.
    Flat,
    /// D44 hubs and kind folders.
    Layered,
}

impl Layout {
    /// The Spec JSON spelling.
    pub fn as_str(self) -> &'static str {
        match self {
            Layout::Flat => "flat",
            Layout::Layered => "layered",
        }
    }
}

const HUBS: [&str; 4] = ["product.md", "tech.md", "design.md", "plan.md"];
const FEATURE_DOCS: [&str; 5] = ["product.md", "tech.md", "design.md", "plan.md", "research.md"];

/// Kind folder → the note `type` it holds.
pub const KINDS: [(&str, &str); 7] = [
    ("areas", "area"),
    ("flows", "flow"),
    ("concepts", "concept"),
    ("platform", "platform"),
    ("tech", "tech"),
    ("decisions", "decision"),
    ("questions", "question"),
];

/// Layered when a root hub or `README.md` declares `type: hub` (P3b-R10).
pub fn detect(dir: &Path) -> Layout {
    let hub = HUBS.iter().chain(["README.md"].iter()).any(|name| {
        std::fs::read(dir.join(name))
            .ok()
            .and_then(|t| frontmatter::parse(&t))
            .is_some_and(|f| f.scalar("type") == Some("hub"))
    });
    if hub { Layout::Layered } else { Layout::Flat }
}

/// Where a file sits in a layered tree.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Place {
    /// Root `product|tech|design|plan.md`.
    Hub,
    /// Root `README.md`/`AGENTS.md`, or a `README.md` hub inside a kind folder.
    Readme,
    /// Root `lessons.md` (unbounded).
    Lessons,
    /// `parts/<part>/*.md`.
    PartHub,
    /// A note under a kind folder (also inside `parts/<part>/`), with its type.
    Note(&'static str),
    /// `features/<f>/{product,tech,design,plan,research}.md`.
    Feature,
    /// `archive/<name>/**`, `quick/<slug>.md`, `templates/**`.
    Exempt,
    /// Anything else (SF30).
    Other,
}

fn kind_type(folder: &str) -> Option<&'static str> {
    KINDS.iter().find(|(k, _)| *k == folder).map(|(_, t)| *t)
}

fn in_kind(kind: Option<&'static str>, rest: &[&str]) -> Place {
    match (kind, rest.last()) {
        (Some(_), Some(&"README.md")) => Place::Readme,
        (Some(t), Some(last)) if last.ends_with(".md") => Place::Note(t),
        _ => Place::Other,
    }
}

/// Classifies a root-relative path given as components.
pub fn place(parts: &[&str]) -> Place {
    match parts {
        [name] if HUBS.contains(name) => Place::Hub,
        ["README.md"] | ["AGENTS.md"] => Place::Readme,
        ["lessons.md"] => Place::Lessons,
        ["features", _, doc] if FEATURE_DOCS.contains(doc) => Place::Feature,
        ["archive", _, _, ..] | ["templates", _, ..] => Place::Exempt,
        ["quick", slug] if slug.ends_with(".md") => Place::Exempt,
        ["parts", _, name] if name.ends_with(".md") => Place::PartHub,
        ["parts", _, kind, rest @ ..] if !rest.is_empty() => in_kind(kind_type(kind), rest),
        [kind, rest @ ..] if !rest.is_empty() => in_kind(kind_type(kind), rest),
        _ => Place::Other,
    }
}

/// Every non-hidden file under `dir` with its root-relative components,
/// byte-sorted. Symlinks are not followed.
pub fn walk(dir: &Path) -> Vec<(PathBuf, Vec<String>)> {
    fn go(dir: &Path, prefix: &[String], out: &mut Vec<(PathBuf, Vec<String>)>) {
        let Ok(read) = std::fs::read_dir(dir) else { return };
        let mut entries: Vec<_> = read.flatten().collect();
        entries.sort_by_key(std::fs::DirEntry::file_name);
        for e in entries {
            let name = e.file_name().to_string_lossy().into_owned();
            if name.starts_with('.') {
                continue;
            }
            let mut parts = prefix.to_vec();
            parts.push(name);
            if e.file_type().is_ok_and(|t| t.is_dir()) {
                go(&e.path(), &parts, out);
            } else {
                out.push((e.path(), parts));
            }
        }
    }
    let mut out = Vec::new();
    go(dir, &[], &mut out);
    out
}

/// The files SF26/SF29 read in a layered tree.
pub fn spec_files(dir: &Path) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = walk(dir)
        .into_iter()
        .filter(|(_, parts)| {
            let refs: Vec<&str> = parts.iter().map(String::as_str).collect();
            matches!(place(&refs), Place::Hub | Place::Readme | Place::PartHub | Place::Note(_))
        })
        .map(|(p, _)| p)
        .collect();
    out.extend(feature_files(dir));
    out
}

fn kebab(s: &str) -> bool {
    !s.is_empty()
        && s.split('-').all(|w| !w.is_empty() && w.bytes().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit()))
}

/// SF31: hubs ≤80 lines, notes ≤60 (`lines` counts a last line without `\n`).
fn budget(r: &mut Report, rel: &str, place: Place, text: &[u8]) {
    let hub_typed = frontmatter::parse(text).is_some_and(|f| f.scalar("type") == Some("hub"));
    let (limit, what) = match place {
        Place::Hub | Place::Readme | Place::PartHub => (80, "hub"),
        Place::Note(_) if hub_typed => (80, "hub"),
        Place::Note(_) => (60, "note"),
        _ => return,
    };
    let n = lines(text).len();
    if n > limit {
        r.warn("SF31", format!("{rel}: {n} lines, over the {limit}-line {what} budget — split it into a folder with its own hub").as_bytes());
    }
}

/// SF32: lowercase-kebab folders and files; note names unique across the tree.
fn names(r: &mut Report, rel: &str, parts: &[&str], place: Place, stems: &mut BTreeMap<String, String>) {
    for (i, part) in parts.iter().enumerate() {
        let last = i + 1 == parts.len();
        if last && (*part == "README.md" || *part == "AGENTS.md") {
            break;
        }
        let stem = if last { part.strip_suffix(".md").unwrap_or(part) } else { part };
        if !kebab(stem) {
            r.warn("SF32", format!("{rel}: '{part}' is not lowercase-kebab").as_bytes());
            return;
        }
    }
    if matches!(place, Place::Note(_) | Place::PartHub) {
        let stem = parts.last().and_then(|p| p.strip_suffix(".md")).unwrap_or_default().to_string();
        match stems.get(&stem) {
            Some(other) => r.warn("SF32", format!("{rel}: name '{stem}' is also used by {other}").as_bytes()),
            None => {
                stems.insert(stem, rel.to_string());
            }
        }
    }
}

/// A link target that is not a local path: empty (`#anchor`), a template, or a URL scheme.
fn skip_target(t: &[u8]) -> bool {
    let colon = t.iter().position(|&c| c == b':');
    let slash = t.iter().position(|&c| c == b'/');
    t.is_empty() || t.contains(&b'{') || matches!((colon, slash), (Some(c), s) if s.is_none_or(|s| c < s))
}

/// SF22 for a layered file: every local link resolves (relative to the file; `/` = the project).
fn links(r: &mut Report, ctx: &Ctx, path: &Path, rel: &str, text: &[u8]) {
    let base = path.parent().unwrap_or(ctx.dir);
    let mut fences = Fences::new();
    for line in lines(text) {
        if fences.skip(line) {
            continue;
        }
        for link in md_links(line) {
            let target = md_link_target(link);
            if skip_target(&target) {
                continue;
            }
            let resolved = match target.strip_prefix(b"/") {
                Some(abs) => join_raw(ctx.project, abs),
                None => join_raw(base, &target),
            };
            if !resolved.exists() {
                r.warn("SF22", &cat!(rel, ": broken link to '", target, "'"));
            }
        }
    }
}

/// The layered-only checks.
pub fn run(r: &mut Report, ctx: &Ctx) {
    let mut stems: BTreeMap<String, String> = BTreeMap::new();
    for (path, parts) in walk(ctx.dir) {
        let refs: Vec<&str> = parts.iter().map(String::as_str).collect();
        let rel = parts.join("/");
        let place = place(&refs);
        let shown = format!("{}/{rel}", ctx.display);
        if place == Place::Other {
            r.at(&shown);
            r.warn("SF30", format!("{rel}: not in the layered layout (hubs, README.md, AGENTS.md, lessons.md, kind folders, parts/<part>/, features/<f>/, archive/, quick/, templates/)").as_bytes());
            continue;
        }
        if matches!(place, Place::Exempt | Place::Feature | Place::Lessons) {
            continue;
        }
        r.examine(&shown);
        let text = std::fs::read(&path).unwrap_or_default();
        budget(r, &rel, place, &text);
        names(r, &rel, &refs, place, &mut stems);
        if place != Place::Hub {
            links(r, ctx, &path, &rel, &text);
        }
    }
}
```

(`cat!` is the crate macro from `main.rs`. `std::fs::DirEntry::file_name` used as a sort key returns an owned `OsString`. If the borrow checker rejects that, use `entries.sort_by_key(|e| e.file_name())`.)

- [ ] **Step 4: Switch the driver and `root --json`**

In `validate.rs`:
- After the `specs.is_empty()` check, add `let layout = layered::detect(&dir); let is_layered = layout == layered::Layout::Layered;`.
- In the root loop, `continue` without printing when `is_layered` and the name is `README.md` or `AGENTS.md`.
- Call `sf15` only when `!is_layered`.
- Replace the Task 7 adherence call with:

```rust
    let files = if is_layered { layered::spec_files(&dir) } else { adherence::flat_files(&dir) };
    adherence::run(&mut r, &ctx, &files, is_layered);
    if is_layered {
        layered::run(&mut r, &ctx);
    }
```

- Pass `layout.as_str()` to `r.json`. The two early returns keep `"flat"`.

In `root.rs` `command`, add `v["layout"] = json!(crate::layered::detect(&root.dir).as_str());` before printing. Add `mod layered;` to `main.rs`.

- [ ] **Step 5: Run the tests and parity**

```bash
cargo test --locked -p vibe-spec 2>&1 | grep -E 'test result|FAILED|panicked'
cargo clippy --workspace --all-targets --locked -- -D warnings 2>&1 | tail -1
cargo fmt --all --check && echo fmt-ok
bash contracts/run.sh | tail -1
bash tests/parity/run.sh 2>&1 | grep -E '^FAIL|parity\[|parity:'
```

Expected:
- Everything `ok`, and the matrix covers 25 checks.
- Contracts OK: `root.layout` already exists in the schema.
- Parity unchanged: every corpus root is flat.

- [ ] **Step 6: Commit**

```bash
git add crates/vibe-spec
git commit -m "feat(spec): detect and check the layered layout"
```

### Task 10: Layered SF33 typed notes, SF34 reachability, grammar on notes

**Files:**
- Modify: `crates/vibe-spec/src/layered.rs` (`typed`, `reach`, grammar calls in `run`)
- Test: `crates/vibe-spec/tests/layered.rs`, `crates/vibe-spec/tests/strictness.rs` (triggers SF33, SF34)

**Interfaces:**
- Consumes: Task 9's `place`, `walk`, `KINDS`, `skip_target`; `content::{requirements_scenarios, scenario_grammar}`; `frontmatter::parse`.
- Produces:
  - SF33 and SF34 findings.
  - `layered::run` also runs SF10 and SF25 on every `Note`, `PartHub` and non-root `Readme`.

- [ ] **Step 1: Write the failing tests**

Append to `tests/layered.rs`:

```rust
#[test]
fn typed_notes_and_reachability_fail_when_broken() {
    let cases: [(&str, &str, &str, fn(&Path)); 8] = [
        ("ly-type", "SF33", "platform/errors.md: type 'flow' does not match its folder (expected 'platform')", |p| {
            write(p, "docs/spec/platform/errors.md", "---\ntype: flow\nupdated: 2026-01-15\n---\n\nTyped.\n")
        }),
        ("ly-notype", "SF33", "questions/pricing.md: missing 'type:' (expected 'question')", |p| {
            write(p, "docs/spec/questions/pricing.md", "---\nupdated: 2026-01-15\n---\n\nMonthly?\n")
        }),
        ("ly-noupdated", "SF33", "decisions/d01-use-rust.md: missing 'updated:'", |p| {
            write(p, "docs/spec/decisions/d01-use-rust.md", "---\ntype: decision\n---\n\nRust.\n")
        }),
        ("ly-nofm", "SF33", "concepts/invoice.md: no frontmatter", |p| write(p, "docs/spec/concepts/invoice.md", "A bill.\n")),
        ("ly-code", "SF33", "areas/billing/invoices.md: code path 'src/gone.rs' does not exist", |p| {
            write(p, "docs/spec/areas/billing/invoices.md", "---\ntype: area\ncode: [src/billing.rs, src/gone.rs]\nupdated: 2026-01-15\n---\n\nMonthly.\n")
        }),
        ("ly-nocode", "SF33", "parts/mobile/areas/screens.md: an area needs 'code:'", |p| {
            write(p, "docs/spec/parts/mobile/areas/screens.md", "---\ntype: area\nupdated: 2026-01-15\n---\n\nTwo.\n")
        }),
        ("ly-applies", "SF33", "tech/rust.md: a tech note needs 'applies:'", |p| {
            write(p, "docs/spec/tech/rust.md", "---\ntype: tech\nupdated: 2026-01-15\n---\n\nEdition 2024.\n")
        }),
        ("ly-orphan", "SF34", "flows/refund.md: not reachable from a hub", |p| {
            write(p, "docs/spec/flows/refund.md", "---\ntype: flow\nupdated: 2026-01-15\n---\n\nMoney back.\n")
        }),
    ];
    for (name, check, needle, mutate) in cases {
        let p = layered_tree(name);
        mutate(&p);
        let got = findings(&p);
        assert!(got.iter().any(|(c, m)| c == check && m.contains(needle)), "{name}: want {check} {needle:?}\n{got:#?}");
    }
}

#[test]
fn a_root_hub_must_be_hub_typed() {
    let p = layered_tree("ly-hubtype");
    write(&p, "docs/spec/design.md", "---\ntype: entrypoint\nscope: design\nchildren:\nupdated: 2026-01-15\n---\n\n# Design\n");
    let got = findings(&p);
    assert!(got.iter().any(|(c, m)| c == "SF33" && m == "design.md: a hub declares 'type: hub'"), "{got:#?}");
}

#[test]
fn reachability_follows_links_through_folder_hubs() {
    let p = layered_tree("ly-reach");
    let hub = std::fs::read_to_string(p.join("docs/spec/product.md")).unwrap().replace("- [Billing](areas/billing/README.md)\n", "");
    std::fs::write(p.join("docs/spec/product.md"), hub).unwrap();
    let got: Vec<String> = findings(&p).into_iter().filter(|(c, _)| c == "SF34").map(|(_, m)| m).collect();
    assert_eq!(
        got,
        [
            "areas/billing/README.md: not reachable from a hub (link it from a hub or its folder README.md)",
            "areas/billing/invoices.md: not reachable from a hub (link it from a hub or its folder README.md)",
        ]
    );
}

#[test]
fn grammar_runs_on_notes() {
    let p = layered_tree("ly-grammar");
    write(&p, "docs/spec/flows/checkout.md", "---\ntype: flow\nupdated: 2026-01-15\n---\n\n### Requirement: Pay\nThe system SHALL charge.\n\n#### Scenario: ok\n- WHEN paying\n");
    let got = findings(&p);
    assert!(got.iter().any(|(c, m)| c == "SF25" && m == "flows/checkout.md: scenario 'ok' has no THEN bullet"), "{got:#?}");
}
```

Also extend `a_hub_typed_root_is_layered`. Its `findings == []` assertion now also covers SF33 and SF34 on the clean fixture.

In `tests/strictness.rs`, remove `"SF33"` and `"SF34"` from `PENDING`, and add:

```rust
        "SF33" => {
            let p = common::layered_tree("mx-sf33-l");
            write(&p, "docs/spec/tech/rust.md", "---\ntype: tech\nupdated: 2026-01-15\n---\n\nx\n");
            plain(p, "a tech note needs 'applies:'")
        }
        "SF34" => {
            let p = common::layered_tree("mx-sf34-l");
            write(&p, "docs/spec/flows/refund.md", "---\ntype: flow\nupdated: 2026-01-15\n---\n\nx\n");
            plain(p, "not reachable from a hub")
        }
```

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test --locked -p vibe-spec --test layered --test strictness 2>&1 | grep -E '^test |test result'`
Expected: the four new tests FAIL, and so does the matrix. Paste the output.

- [ ] **Step 3: Implement SF33, SF34 and the grammar calls**

Add to `layered.rs` (imports: `std::collections::{BTreeSet, VecDeque}`, `crate::content`, `crate::text::lossy`):

```rust
/// SF33: notes carry the type of their folder (or `hub`), `updated:`, and
/// `code:` (areas, existing paths from the project) or `applies:` (tech);
/// root and part hubs declare `type: hub`.
fn typed(r: &mut Report, ctx: &Ctx, rel: &str, place: Place, text: &[u8]) {
    match place {
        Place::Note(want) => {
            let Some(fm) = frontmatter::parse(text) else {
                r.warn("SF33", format!("{rel}: no frontmatter (needs 'type: {want}' and 'updated:')").as_bytes());
                return;
            };
            let t = fm.scalar("type");
            match t {
                None => r.warn("SF33", format!("{rel}: missing 'type:' (expected '{want}')").as_bytes()),
                Some(t) if t != want && t != "hub" => r.warn(
                    "SF33",
                    format!("{rel}: type '{t}' does not match its folder (expected '{want}')").as_bytes(),
                ),
                Some(_) => {}
            }
            if fm.scalar("updated").is_none() {
                r.warn("SF33", format!("{rel}: missing 'updated:'").as_bytes());
            }
            match t.unwrap_or(want) {
                "area" => {
                    let code = fm.list("code");
                    if code.is_empty() {
                        r.warn("SF33", format!("{rel}: an area needs 'code:' (a path or a list of paths)").as_bytes());
                    }
                    for c in code {
                        let p = join_raw(ctx.project, c.trim_start_matches('/').as_bytes());
                        if !p.exists() {
                            r.warn("SF33", format!("{rel}: code path '{c}' does not exist").as_bytes());
                        }
                    }
                }
                "tech" if fm.list("applies").is_empty() => {
                    r.warn("SF33", format!("{rel}: a tech note needs 'applies:' (a glob)").as_bytes());
                }
                _ => {}
            }
        }
        Place::Hub | Place::PartHub => {
            let hub = frontmatter::parse(text).is_some_and(|f| f.scalar("type") == Some("hub"));
            if !hub {
                r.warn("SF33", format!("{rel}: a hub declares 'type: hub'").as_bytes());
            }
        }
        _ => {}
    }
}

/// `dir_rel` joined with `target`, `.`/`..` resolved; `None` when it leaves the root.
fn join_rel(dir_rel: &[&str], target: &str) -> Option<String> {
    let mut parts: Vec<&str> = dir_rel.to_vec();
    for seg in target.split('/') {
        match seg {
            "" | "." => {}
            ".." => {
                parts.pop()?;
            }
            s => parts.push(s),
        }
    }
    Some(parts.join("/"))
}

/// SF34: every note, part hub and folder README is reachable from the root
/// hubs, README.md or AGENTS.md by local links (fences skipped).
fn reach(r: &mut Report, ctx: &Ctx, files: &[(PathBuf, Vec<String>)]) {
    let rels: BTreeSet<String> = files.iter().map(|(_, parts)| parts.join("/")).collect();
    let mut seen: BTreeSet<String> = HUBS
        .iter()
        .chain(["README.md", "AGENTS.md"].iter())
        .map(|s| s.to_string())
        .filter(|s| rels.contains(s))
        .collect();
    let mut queue: VecDeque<String> = seen.iter().cloned().collect();
    while let Some(rel) = queue.pop_front() {
        let text = std::fs::read(ctx.dir.join(&rel)).unwrap_or_default();
        let dir: Vec<&str> = {
            let mut d: Vec<&str> = rel.split('/').collect();
            d.pop();
            d
        };
        let mut fences = Fences::new();
        for line in lines(&text) {
            if fences.skip(line) {
                continue;
            }
            for link in md_links(line) {
                let target = md_link_target(link);
                if skip_target(&target) || target.first() == Some(&b'/') {
                    continue;
                }
                if let Some(next) = join_rel(&dir, &lossy(&target))
                    && rels.contains(&next)
                    && seen.insert(next.clone())
                {
                    queue.push_back(next);
                }
            }
        }
    }
    for (_, parts) in files {
        let refs: Vec<&str> = parts.iter().map(String::as_str).collect();
        let reachable_kind = match place(&refs) {
            Place::Note(_) | Place::PartHub => true,
            Place::Readme => refs.len() > 1,
            _ => false,
        };
        let rel = parts.join("/");
        if reachable_kind && !seen.contains(&rel) {
            r.at(&format!("{}/{rel}", ctx.display));
            r.warn("SF34", format!("{rel}: not reachable from a hub (link it from a hub or its folder README.md)").as_bytes());
        }
    }
}
```

Change `run` so it:
1. collects `let files = walk(ctx.dir);` once;
2. in the per-file loop, after `links(...)`, calls `typed(r, ctx, &rel, place, &text)`. For `Place::Note(_) | Place::PartHub`, and for `Place::Readme` when `refs.len() > 1`, it also calls `content::requirements_scenarios(r, &text, rel.as_bytes()); content::scenario_grammar(r, &text, rel.as_bytes());`. Root hubs get both from `root_file` already.
3. ends with `reach(r, ctx, &files);`.

Update the module doc to list SF33 and SF34.

- [ ] **Step 4: Run the tests and parity**

```bash
cargo test --locked -p vibe-spec 2>&1 | grep -E 'test result|FAILED|panicked'
cargo clippy --workspace --all-targets --locked -- -D warnings 2>&1 | tail -1
cargo fmt --all --check && echo fmt-ok
bash tests/parity/run.sh 2>&1 | grep -E '^FAIL|parity\[|parity:'
```

Expected:
- Everything `ok`. The layered fixture still has zero findings, and the matrix covers 27 checks.
- Parity unchanged.

- [ ] **Step 5: Commit**

```bash
git add crates/vibe-spec
git commit -m "feat(spec): check typed notes and hub reachability"
```

### Task 11: Superpowers artifacts: SF35 names, SF36 the plan's `**Spec:**` path

**Files:**
- Create: `crates/vibe-spec/src/superpowers.rs`
- Modify: `crates/vibe-spec/src/query.rs` (use `superpowers::{SPECS, PLANS}`), `crates/vibe-spec/src/validate.rs` (call `superpowers::run`), `crates/vibe-spec/src/main.rs` (module)
- Test: `crates/vibe-spec/tests/superpowers.rs` (new), `crates/vibe-spec/tests/strictness.rs` (triggers SF35, SF36)

**Interfaces:**
- Consumes: `text::{is_date, join_raw, lines}`, `pattern::{md_links, md_link_target}`, `Report`.
- Produces:
  - `superpowers::{SPECS, PLANS}: &str`
  - `superpowers::run(r: &mut Report, project: &Path)`, called from `validate::run` in both layouts, after `adherence::run`.

Findings name project-relative files (`docs/superpowers/plans/<name>`).

- [ ] **Step 1: Write the failing tests**

Create `crates/vibe-spec/tests/superpowers.rs`:

```rust
#![cfg(unix)]
#![allow(clippy::unwrap_used, clippy::expect_used)]
//! SF35 artifact names and SF36 plan `**Spec:**` paths (P3b-R11).
mod common;

use common::{project, validate_json, write};
use std::path::{Path, PathBuf};

fn sp(name: &str) -> PathBuf {
    let p = project(name);
    write(&p, ".spec/product.md", "---\ntype: entrypoint\nscope: product\nchildren:\nupdated: 2026-01-15\n---\n");
    write(&p, "docs/superpowers/specs/2026-01-02-auth-design.md", "# Auth design\n");
    write(&p, "docs/superpowers/plans/2026-01-03-auth.md", "# Auth plan\n\n**Spec:** `docs/superpowers/specs/2026-01-02-auth-design.md` (§2). Index: `docs/x.md`.\n");
    write(&p, "docs/superpowers/plans/2026-01-04-auth-p2.md", "# P2\n\n**Spec:** [design](../specs/2026-01-02-auth-design.md#goal)\n");
    p
}

fn found(p: &Path) -> Vec<(String, String)> {
    let (_, v) = validate_json(p, &[], &[]);
    v["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| matches!(f["check"].as_str(), Some("SF35" | "SF36")))
        .map(|f| (f["check"].as_str().unwrap().to_string(), f["message"].as_str().unwrap().to_string()))
        .collect()
}

#[test]
fn good_artifacts_pass() {
    assert_eq!(found(&sp("sp-ok")), Vec::<(String, String)>::new());
}

#[test]
fn bad_names_fail() {
    let p = sp("sp-names");
    write(&p, "docs/superpowers/specs/2026-01-02-auth.md", "# no -design\n");
    write(&p, "docs/superpowers/plans/notes.md", "**Spec:** `docs/superpowers/specs/2026-01-02-auth-design.md`\n");
    write(&p, "docs/superpowers/plans/2026-13-40-x.md", "**Spec:** `docs/superpowers/specs/2026-01-02-auth-design.md`\n");
    write(&p, "docs/superpowers/plans/2026-01-05-Bad_Topic.md", "**Spec:** `docs/superpowers/specs/2026-01-02-auth-design.md`\n");
    write(&p, "docs/superpowers/plans/drafts/x.md", "x\n");
    write(&p, "docs/superpowers/plans/.DS_Store", "x");
    let got = found(&p);
    let sf35: Vec<&str> = got.iter().filter(|(c, _)| c == "SF35").map(|(_, m)| m.as_str()).collect();
    assert_eq!(
        sf35,
        [
            "docs/superpowers/specs/2026-01-02-auth.md: not named YYYY-MM-DD-<topic>-design.md",
            "docs/superpowers/plans/2026-01-05-Bad_Topic.md: not named YYYY-MM-DD-<topic>.md",
            "docs/superpowers/plans/2026-13-40-x.md: not named YYYY-MM-DD-<topic>.md",
            "docs/superpowers/plans/drafts: not named YYYY-MM-DD-<topic>.md",
            "docs/superpowers/plans/notes.md: not named YYYY-MM-DD-<topic>.md",
        ],
        "{got:#?}"
    );
}

#[test]
fn plan_spec_lines_must_resolve() {
    let p = sp("sp-links");
    write(&p, "docs/superpowers/plans/2026-01-06-nospec.md", "# No spec line\n");
    write(&p, "docs/superpowers/plans/2026-01-07-gone.md", "**Spec:** `docs/superpowers/specs/2026-01-01-gone-design.md`\n");
    write(&p, "docs/superpowers/plans/2026-01-08-nopath.md", "**Spec:** the auth design\n");
    let sf36: Vec<String> = found(&p).into_iter().filter(|(c, _)| c == "SF36").map(|(_, m)| m).collect();
    assert_eq!(
        sf36,
        [
            "docs/superpowers/plans/2026-01-06-nospec.md: no '**Spec:**' line naming the design it implements",
            "docs/superpowers/plans/2026-01-07-gone.md: **Spec:** path 'docs/superpowers/specs/2026-01-01-gone-design.md' does not exist",
            "docs/superpowers/plans/2026-01-08-nopath.md: the '**Spec:**' line names no path",
        ]
    );
}

#[test]
fn this_repo_s_artifacts_pass() {
    let (_, v) = validate_json(&common::repo(), &[], &[]);
    let bad: Vec<&serde_json::Value> = v["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| matches!(f["check"].as_str(), Some("SF35" | "SF36")))
        .collect();
    assert!(bad.is_empty(), "{bad:#?}");
}
```

In `tests/strictness.rs`, remove `"SF35"` and `"SF36"` from `PENDING`. `PENDING` is now `&[]`. Add these arms:

```rust
        "SF35" => {
            write(&p, "docs/superpowers/plans/notes.md", "x\n");
            plain(p, "not named YYYY-MM-DD-<topic>.md")
        }
        "SF36" => {
            write(&p, "docs/superpowers/plans/2026-01-03-auth.md", "# Plan\n");
            plain(p, "no '**Spec:**' line")
        }
```

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test --locked -p vibe-spec --test superpowers --test strictness 2>&1 | grep -E '^test |test result'`
Expected: `bad_names_fail`, `plan_spec_lines_must_resolve` and the matrix FAIL. `good_artifacts_pass` and `this_repo_s_artifacts_pass` pass vacuously. Paste the output.

- [ ] **Step 3: Implement `superpowers.rs`**

```rust
//! Superpowers artifacts (D45, P3b-R11): SF35 names under
//! `docs/superpowers/{specs,plans}/`, SF36 each plan's `**Spec:**` path.
//! Content format is not linted.
use std::path::Path;

use crate::pattern::{md_link_target, md_links};
use crate::report::Report;
use crate::text::{is_date, join_raw, lines, lossy};

/// Design specs, `YYYY-MM-DD-<topic>-design.md`.
pub const SPECS: &str = "docs/superpowers/specs";
/// Plans, `YYYY-MM-DD-<topic>.md`.
pub const PLANS: &str = "docs/superpowers/plans";

fn kebab(s: &str) -> bool {
    !s.is_empty()
        && s.split('-').all(|w| !w.is_empty() && w.bytes().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit()))
}

/// A real calendar-shaped date: `is_date` plus month 01–12 and day 01–31.
fn plausible(date: &[u8]) -> bool {
    let num = |r: std::ops::Range<usize>| {
        date.get(r).and_then(|b| std::str::from_utf8(b).ok()).and_then(|s| s.parse::<u32>().ok())
    };
    is_date(date) && num(5..7).is_some_and(|m| (1..=12).contains(&m)) && num(8..10).is_some_and(|d| (1..=31).contains(&d))
}

/// `YYYY-MM-DD-<topic><suffix>`.
fn dated(name: &str, suffix: &str) -> bool {
    let Some(stem) = name.strip_suffix(suffix) else { return false };
    let b = stem.as_bytes();
    b.len() > 11 && plausible(b.get(..10).unwrap_or_default()) && b.get(10) == Some(&b'-') && kebab(stem.get(11..).unwrap_or_default())
}

/// The first `` `…` `` span or markdown link target on a line, whichever comes first.
fn first_path(line: &[u8]) -> Option<Vec<u8>> {
    let tick = line.iter().position(|&c| c == b'`').and_then(|open| {
        let rest = line.get(open + 1..)?;
        let close = rest.iter().position(|&c| c == b'`')?;
        Some((open, rest.get(..close)?.to_vec()))
    });
    let link = md_links(line).first().map(|l| {
        let at = line.windows(l.len()).position(|w| w == *l).unwrap_or(usize::MAX);
        (at, md_link_target(l))
    });
    match (tick, link) {
        (Some(t), Some(l)) => Some(if t.0 <= l.0 { t.1 } else { l.1 }),
        (Some(t), None) => Some(t.1),
        (None, Some(l)) => Some(l.1),
        (None, None) => None,
    }
    .filter(|p| !p.is_empty())
}

fn spec_link(r: &mut Report, project: &Path, rel: &str, path: &Path) {
    let text = std::fs::read(path).unwrap_or_default();
    let Some(line) = lines(&text).into_iter().find(|l| l.starts_with(b"**Spec:**")) else {
        r.warn("SF36", format!("{rel}: no '**Spec:**' line naming the design it implements").as_bytes());
        return;
    };
    let Some(target) = first_path(line.get(9..).unwrap_or_default()) else {
        r.warn("SF36", format!("{rel}: the '**Spec:**' line names no path").as_bytes());
        return;
    };
    let target: Vec<u8> = target.split(|&c| c == b'#').next().unwrap_or_default().to_vec();
    let here = path.parent().unwrap_or(project);
    if !(join_raw(project, &target).exists() || join_raw(here, &target).exists()) {
        r.warn("SF36", format!("{rel}: **Spec:** path '{}' does not exist", lossy(&target)).as_bytes());
    }
}

/// SF35 and SF36 when the artifact directories exist.
pub fn run(r: &mut Report, project: &Path) {
    for (dir, suffix, shape) in [
        (SPECS, "-design.md", "YYYY-MM-DD-<topic>-design.md"),
        (PLANS, ".md", "YYYY-MM-DD-<topic>.md"),
    ] {
        let Ok(read) = std::fs::read_dir(project.join(dir)) else { continue };
        let mut entries: Vec<_> = read.flatten().collect();
        entries.sort_by_key(|e| e.file_name());
        for e in entries {
            let name = e.file_name().to_string_lossy().into_owned();
            if name.starts_with('.') {
                continue;
            }
            let rel = format!("{dir}/{name}");
            let file = e.file_type().is_ok_and(|t| t.is_file());
            if file {
                r.examine(&rel);
            } else {
                r.at(&rel);
            }
            if !file || !dated(&name, suffix) {
                r.warn("SF35", format!("{rel}: not named {shape}").as_bytes());
                continue;
            }
            if dir == PLANS {
                spec_link(r, project, &rel, &e.path());
            }
        }
    }
}
```

Note: `entries` sort byte-wise, so `2026-01-05-Bad_Topic.md` sorts before `2026-13-40-x.md`, then `drafts`, then `notes.md`. That is the order the test expects.

Wire it up:
- In `validate.rs`, call `superpowers::run(&mut r, &project);` after the adherence/layered calls.
- In `query.rs`, delete `SP_SPECS`/`SP_PLANS` and use `crate::superpowers::{SPECS, PLANS}`.
- Add `mod superpowers;` to `main.rs`.

- [ ] **Step 4: Run the tests and parity**

```bash
cargo test --locked -p vibe-spec 2>&1 | grep -E 'test result|FAILED|panicked'
cargo clippy --workspace --all-targets --locked -- -D warnings 2>&1 | tail -1
cargo fmt --all --check && echo fmt-ok
bash tests/parity/run.sh 2>&1 | grep -E '^FAIL|parity\[|parity:'
```

Expected:
- Everything `ok`, and the matrix covers all 29 checks.
- Parity unchanged. Corpus sandboxes hold no `docs/superpowers/`, and SF35/SF36 are off there.

- [ ] **Step 5: Commit**

```bash
git add crates/vibe-spec
git commit -m "feat(spec): check superpowers artifact names"
```

### Task 12: `vibe-spec trace`

**Files:**
- Create: `crates/vibe-spec/src/trace.rs`
- Modify: `crates/vibe-spec/src/git.rs` (`commits`), `crates/vibe-spec/src/pattern.rs` (`unit_citations`), `crates/vibe-spec/src/main.rs` (`Trace`), `contracts/spec-json.schema.json` (`trace`, `orphan`), `contracts/run.mjs`
- Create: `contracts/fixtures/spec-json/trace/valid/{clean,findings,unknown-field}.json`, `contracts/fixtures/spec-json/trace/invalid/{zero-units,bad-history}.json`
- Test: `crates/vibe-spec/tests/trace.rs` (new); unit test for `unit_citations`

**Interfaces:**
- Consumes: `git::{git, top, has_commits}`, `text::{Fences, lines, lossy, sub_dirs, name_of}`, `common::{git, git_out}`.
- Produces:
  - `pattern::unit_citations(s: &[u8]) -> Vec<&[u8]>`
  - `git::commits(dir: &Path, since: Option<&str>) -> Result<Option<Vec<(String, Vec<u8>)>>, String>`
  - `trace::run(found: &Result<Root, Miss>, since: Option<&str>, json_out: bool) -> (u8, Vec<u8>)`
  - The Spec JSON v1 `trace` answer:
    - `{v, units, cited, history, since?, uncited[], orphans[{unit, source}]}`
    - `history` is one of `all`, `since`, `none`.
  - Exit codes per P3b-R13.

- [ ] **Step 1: Write the failing tests**

Unit test in `pattern.rs`:

```rust
    #[test]
    fn unit_citations_are_word_bounded() {
        assert_eq!(
            unit_citations(b"(auth/1) x-auth/2 tests/auth/3 auth/4x _auth/5 9lives/6"),
            vec![&b"auth/1"[..], b"x-auth/2", b"auth/3", b"9lives/6"]
        );
    }
```

`crates/vibe-spec/tests/trace.rs`:

```rust
#![cfg(unix)]
#![allow(clippy::unwrap_used, clippy::expect_used)]
//! `vibe-spec trace` over a synthetic repo (P3b-R13).
mod common;

use common::{assert_spec_json, git, git_out, project, spec, write};
use std::path::PathBuf;

const ENTRY: &str = "---\ntype: entrypoint\nscope: product\nchildren:\nupdated: 2026-01-15\n---\n";
const AUTH_PLAN: &str = "---\ntype: feature-plan\n---\n\n### auth/1 — Login\n\n### auth/2 — Logout\n\n### auth/3 — Reset\n\n```\n### auth/4 — fenced, not a unit\n```\n";

/// Units auth/1–3 and billing/1. Commit 1 cites auth/1; commit 2 cites
/// auth/9 and auth/4 (no such units); tests/ cites auth/2, billing/1 and
/// unknown/4 (not a feature).
fn repo(name: &str) -> (PathBuf, String) {
    let p = project(name);
    write(&p, ".spec/product.md", ENTRY);
    write(&p, ".spec/features/auth/plan.md", AUTH_PLAN);
    write(&p, ".spec/features/billing/plan.md", "---\ntype: feature-plan\n---\n\n### billing/1 — Invoice\n");
    write(&p, "tests/auth_test.sh", "# covers auth/2 and billing/1; unknown/4 is not a feature\n");
    let d = "2026-02-01T12:00:00Z";
    git(&p, d, &["init", "-q"]);
    git(&p, d, &["add", "-A"]);
    git(&p, d, &["commit", "-q", "-m", "feat: add login (auth/1)"]);
    let first = git_out(&p, &["rev-parse", "HEAD"]);
    write(&p, "notes.txt", "x\n");
    git(&p, d, &["add", "-A"]);
    git(&p, d, &["commit", "-q", "-m", "fix: touch auth/9 and auth/4"]);
    (p, first)
}

#[test]
fn trace_reports_uncited_units_and_orphans() {
    let (p, _) = repo("tr-all");
    let out = spec(&p, &["trace"], &[]);
    assert_eq!(out.code, 1, "{}{}", out.text(), out.stderr);
    let text = out.text();
    assert!(text.starts_with("trace: 4 units, 3 cited; commits: all\n"), "{text}");
    assert!(text.contains("UNCITED  auth/3\n"), "{text}");
    assert!(text.contains("ORPHAN   auth/4  commit "), "{text}");
    assert!(text.contains("ORPHAN   auth/9  commit "), "{text}");
    assert!(!text.contains("unknown/4"), "{text}");
    let json = spec(&p, &["trace", "--json"], &[]);
    let v: serde_json::Value = serde_json::from_slice(&json.stdout).unwrap();
    assert_spec_json("trace", &v.to_string());
    assert_eq!(v["uncited"], serde_json::json!(["auth/3"]));
    assert_eq!(v["orphans"].as_array().unwrap().len(), 2);
    assert_eq!((v["units"].as_u64(), v["cited"].as_u64(), v["history"].as_str()), (Some(4), Some(3), Some("all")));
}

#[test]
fn since_limits_the_commits() {
    let (p, first) = repo("tr-since");
    let out = spec(&p, &["trace", "--since", &first, "--json"], &[]);
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["uncited"], serde_json::json!(["auth/1", "auth/3"]));
    assert_eq!(v["history"], "since");
    assert_eq!(v["since"], first);
    assert_spec_json("trace", &v.to_string());
}

#[test]
fn a_clean_trace_exits_0() {
    let (p, _) = repo("tr-clean");
    write(&p, "tests/more_test.sh", "# auth/3\n");
    git(&p, "2026-02-02T12:00:00Z", &["add", "-A"]);
    git(&p, "2026-02-02T12:00:00Z", &["commit", "-q", "-m", "test: cover reset"]);
    let mut plan = AUTH_PLAN.to_string();
    plan.push_str("\n### auth/4 — Now real\n\n### auth/9 — Also real\n");
    write(&p, ".spec/features/auth/plan.md", &plan);
    let out = spec(&p, &["trace"], &[]);
    assert_eq!(out.code, 0, "{}", out.text());
}

#[test]
fn trace_without_git_uses_tests_only() {
    let p = project("tr-nogit");
    write(&p, ".spec/product.md", ENTRY);
    write(&p, ".spec/features/auth/plan.md", AUTH_PLAN);
    write(&p, "tests/a.sh", "# auth/1 auth/2 auth/3\n");
    let out = spec(&p, &["trace", "--json"], &[]);
    assert_eq!(out.code, 0, "{}", out.stderr);
    assert!(out.stderr.contains("no git history"), "{}", out.stderr);
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["history"], "none");
}

#[test]
fn trace_usage_and_root_errors() {
    let (p, _) = repo("tr-bad");
    assert_eq!(spec(&p, &["trace", "--since", "no-such-ref"], &[]).code, 2);
    assert_eq!(spec(&p, &["trace", "--since", "--all"], &[]).code, 2);
    let none = project("tr-noroot");
    assert_eq!(spec(&none, &["trace"], &[]).code, 3);
    let empty = project("tr-nounits");
    write(&empty, ".spec/product.md", ENTRY);
    let out = spec(&empty, &["trace"], &[]);
    assert_eq!(out.code, 1);
    assert!(out.stderr.contains("nothing to trace"), "{}", out.stderr);
}
```

Goldens:
- `trace/valid/clean.json`: `{ "v": 1, "units": 3, "cited": 3, "history": "all", "uncited": [], "orphans": [] }`
- `trace/valid/findings.json`: `{ "v": 1, "units": 4, "cited": 2, "history": "since", "since": "abc123", "uncited": ["auth/3"], "orphans": [ { "unit": "auth/9", "source": "commit 1a2b3c4" } ] }`
- `trace/valid/unknown-field.json`: `clean.json` plus `"took_ms": 3`
- `trace/invalid/zero-units.json`: `clean.json` with `"units": 0`
- `trace/invalid/bad-history.json`: `clean.json` with `"history": "partial"`

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test --locked -p vibe-spec --test trace 2>&1 | grep -E '^test |test result|error'`
Expected: every test FAILS with `unrecognized subcommand 'trace'` (exit 2). The pattern test does not compile. Paste the output.

- [ ] **Step 3: Implement the pieces**

`pattern.rs`:

```rust
/// `\b[a-z0-9][a-z0-9-]*/[0-9]+\b`: unit citations in free text (trace).
pub fn unit_citations(s: &[u8]) -> Vec<&[u8]> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < s.len() {
        let bounded = i == 0 || s.get(i - 1).is_some_and(|&c| !is_word(c));
        if bounded
            && let Some(n) = unit_slash_at(s, i)
            && s.get(i + n).is_none_or(|&c| !is_word(c))
        {
            out.push(&s[i..i + n]);
            i += n;
            continue;
        }
        i += 1;
    }
    out
}
```

`git.rs`:

```rust
/// Commit hashes and messages, newest first: all of `HEAD`, or
/// `<since>..HEAD`. `Ok(None)` without git, a work tree or commits (an
/// error when `since` was given); `Err` for a `since` git cannot resolve.
pub fn commits(dir: &Path, since: Option<&str>) -> Result<Option<Vec<(String, Vec<u8>)>>, String> {
    if let Some(s) = since
        && (s.is_empty() || s.starts_with('-'))
    {
        return Err(format!("--since '{s}' is not a revision"));
    }
    let Some(top) = top(dir).filter(|t| has_commits(t)) else {
        return match since {
            Some(_) => Err("--since needs git history".to_string()),
            None => Ok(None),
        };
    };
    let range = match since {
        Some(s) => {
            if git(&top, &["rev-parse", "--verify", "-q", &format!("{s}^{{commit}}")]).is_none() {
                return Err(format!("unknown revision '{s}'"));
            }
            format!("{s}..HEAD")
        }
        None => "HEAD".to_string(),
    };
    let log = git(&top, &["log", "--format=%H%x1f%B%x1e", &range]).ok_or_else(|| format!("git log {range} failed"))?;
    let mut out = Vec::new();
    for record in log.split(|&b| b == 0x1e) {
        let record = record.strip_prefix(b"\n").unwrap_or(record);
        let Some(sep) = record.iter().position(|&b| b == 0x1f) else { continue };
        let sha = lossy(record.get(..sep).unwrap_or_default());
        out.push((sha, record.get(sep + 1..).unwrap_or_default().to_vec()));
    }
    Ok(Some(out))
}
```

`trace.rs`:

```rust
//! `vibe-spec trace [--since <ref>] [--json]` (P3b-R13): units are the
//! canonical `### <f>/<n>` headings of fallback feature plans; citations are
//! `<name>/<n>` tokens in commit messages and under `<project>/tests/`.
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use serde_json::json;
use vibe_core::exit;

use crate::git;
use crate::pattern::unit_citations;
use crate::root::{Miss, Root};
use crate::text::{Fences, lines, lossy, name_of, sub_dirs};

/// Feature names and their units.
fn units(dir: &Path) -> (BTreeSet<String>, BTreeSet<String>) {
    let mut features = BTreeSet::new();
    let mut units = BTreeSet::new();
    for fdir in sub_dirs(&dir.join("features")) {
        let f = name_of(&fdir);
        let text = std::fs::read(fdir.join("plan.md")).unwrap_or_default();
        let own = format!("{f}/");
        let mut fences = Fences::new();
        for line in lines(&text) {
            if fences.skip(line) {
                continue;
            }
            let Some(rest) = line.strip_prefix(b"### ") else { continue };
            if let Some(tok) = unit_citations(rest).first()
                && rest.starts_with(tok)
                && tok.starts_with(own.as_bytes())
            {
                units.insert(lossy(tok));
            }
        }
        features.insert(f);
    }
    (features, units)
}

/// Every file under `<project>/tests/` (hidden entries skipped, symlinks not followed).
fn test_files(project: &Path) -> Vec<(String, Vec<u8>)> {
    fn go(dir: &Path, rel: &str, out: &mut Vec<(String, Vec<u8>)>) {
        let Ok(read) = std::fs::read_dir(dir) else { return };
        let mut entries: Vec<_> = read.flatten().collect();
        entries.sort_by_key(|e| e.file_name());
        for e in entries {
            let name = e.file_name().to_string_lossy().into_owned();
            if name.starts_with('.') {
                continue;
            }
            let here = format!("{rel}/{name}");
            match e.file_type() {
                Ok(t) if t.is_dir() => go(&e.path(), &here, out),
                Ok(t) if t.is_file() => out.push((here, std::fs::read(e.path()).unwrap_or_default())),
                _ => {}
            }
        }
    }
    let mut out = Vec::new();
    go(&project.join("tests"), "tests", &mut out);
    out
}

/// Runs the command.
pub fn run(found: &Result<Root, Miss>, since: Option<&str>, json_out: bool) -> (u8, Vec<u8>) {
    let root = match found {
        Ok(root) => root,
        Err(miss) => {
            eprintln!("{}", miss.message());
            return (miss.code(), Vec::new());
        }
    };
    let (features, units) = units(&root.dir);
    if units.is_empty() {
        eprintln!(
            "vibe-spec trace: no units under {}/features/*/plan.md; nothing to trace (superpowers plans carry no unit ids)",
            root.display()
        );
        return (exit::FINDINGS, Vec::new());
    }
    let history = match git::commits(&root.project, since) {
        Ok(h) => h,
        Err(why) => {
            eprintln!("vibe-spec trace: {why}");
            return (exit::USAGE, Vec::new());
        }
    };
    if history.is_none() {
        eprintln!("vibe-spec trace: no git history; citations from tests/ only");
    }
    let mut cited: BTreeMap<String, usize> = units.iter().map(|u| (u.clone(), 0)).collect();
    let mut orphans: BTreeSet<(String, String)> = BTreeSet::new();
    let mut see = |token: &[u8], source: &str| {
        let t = lossy(token);
        let Some((name, _)) = t.split_once('/') else { return };
        if !features.contains(name) {
            return;
        }
        match cited.get_mut(&t) {
            Some(n) => *n += 1,
            None => {
                orphans.insert((t, source.to_string()));
            }
        }
    };
    for (sha, body) in history.iter().flatten() {
        let source = format!("commit {}", sha.get(..7).unwrap_or(sha));
        for tok in unit_citations(body) {
            see(tok, &source);
        }
    }
    for (rel, text) in test_files(&root.project) {
        for tok in unit_citations(&text) {
            see(tok, &rel);
        }
    }
    let uncited: Vec<&String> = cited.iter().filter(|(_, n)| **n == 0).map(|(u, _)| u).collect();
    let n_cited = cited.len() - uncited.len();
    let label = match (&history, since) {
        (None, _) => "none",
        (Some(_), Some(_)) => "since",
        (Some(_), None) => "all",
    };
    let code = if uncited.is_empty() && orphans.is_empty() { exit::OK } else { exit::FINDINGS };
    if json_out {
        let mut v = json!({
            "v": 1,
            "units": units.len(),
            "cited": n_cited,
            "history": label,
            "uncited": uncited,
            "orphans": orphans.iter().map(|(u, s)| json!({ "unit": u, "source": s })).collect::<Vec<_>>(),
        });
        if let (Some(s), "since") = (since, label) {
            v["since"] = json!(s);
        }
        return (code, format!("{v}\n").into_bytes());
    }
    let commits = match (label, since) {
        ("since", Some(s)) => format!("{s}..HEAD"),
        ("all", _) => "all".to_string(),
        _ => "unavailable".to_string(),
    };
    let mut out = format!("trace: {} units, {n_cited} cited; commits: {commits}\n", units.len());
    for u in &uncited {
        out.push_str(&format!("UNCITED  {u}\n"));
    }
    for (u, s) in &orphans {
        out.push_str(&format!("ORPHAN   {u}  {s}\n"));
    }
    (code, out.into_bytes())
}
```

`main.rs`: add `mod trace;` and the variant:

```rust
    /// Units cited by commits and tests/, uncited units, orphan citations.
    Trace {
        /// Only commits after this revision (`<ref>..HEAD`).
        #[arg(long, value_name = "REF", allow_hyphen_values = true)]
        since: Option<String>,
        /// Print Spec JSON v1.
        #[arg(long)]
        json: bool,
    },
```

Dispatch: `Command::Trace { since, json } => trace::run(&found, since.as_deref(), json)`.

Schema: add the following to `$defs`, and add `'trace'` to `QUERIES` with `trace: ['cited', 'history', 'orphans', 'v', 'uncited', 'units']` and `orphan: ['source', 'unit']` in `REQUIRED`:

```json
    "trace": {
      "type": "object",
      "required": ["v", "units", "cited", "history", "uncited", "orphans"],
      "properties": {
        "v": { "const": 1 },
        "units": { "type": "integer", "minimum": 1 },
        "cited": { "type": "integer", "minimum": 0 },
        "history": { "enum": ["all", "since", "none"] },
        "since": { "type": "string", "minLength": 1 },
        "uncited": { "type": "array", "items": { "type": "string", "minLength": 1 } },
        "orphans": { "type": "array", "items": { "$ref": "#/$defs/orphan" } }
      }
    },
    "orphan": {
      "type": "object",
      "required": ["unit", "source"],
      "properties": {
        "unit": { "type": "string", "minLength": 1 },
        "source": { "type": "string", "minLength": 1 }
      }
    }
```

- [ ] **Step 4: Run the tests and contracts**

```bash
cargo test --locked -p vibe-spec 2>&1 | grep -E 'test result|FAILED|panicked'
cargo clippy --workspace --all-targets --locked -- -D warnings 2>&1 | tail -1
cargo fmt --all --check && echo fmt-ok
bash contracts/run.sh | tail -1
```

Expected:
- Everything `ok`.
- `contracts: 2 schemas, 15 valid + 12 invalid goldens — OK`.

- [ ] **Step 5: Commit**

```bash
git add crates/vibe-spec contracts
git commit -m "feat(spec): add trace for unit citations"
```

### Task 13: Docs, program index and phase gate

**Files:**
- Modify inside `strategy.spec`: `.spec/tech-spec.md`, `.spec/tech-contracts.md`, `.spec/tech-rust.md`, `.spec/plan.md`, `.spec/lessons.md`; `AGENTS.md` (generated markers only, via `regen-active-rules.sh`)
- Modify: `docs/superpowers/plans/2026-10-13-spec-v04.md` (row 3b)
- Modify: `crates/vibe-spec/tests/strictness.rs` (no pending check; matrix covers every registered check), `crates/vibe-spec/src/main.rs` (crate doc)

**Interfaces:**
- Consumes: everything above.
- Produces: the F18 baseline. `p3c` is written against the landed `check`, `report`, `frontmatter`, `layered` and `git` modules.

- [ ] **Step 1: Close the matrix**

In `tests/strictness.rs`:
- Set `const PENDING: &[&str] = &[];`.
- Add `assert!(PENDING.is_empty(), "F17 leaves no check untriggered");` to `every_check_is_either_triggered_or_pending`.
- In `warn_error_off_matrix`, change the floor to `assert_eq!(covered, registered().len(), "every registered check is tested warn/error/off");`.

In `main.rs`, change the crate doc to: "`vibe-spec`: the spec tool. F16 ports the read-only spec scripts and adds root discovery and the Spec JSON v1 queries; F17 adds the check registry, strictness, the F17 validators and `trace`."

Run `cargo test --locked -p vibe-spec --test strictness`. Expected: `ok`, and the matrix covers 29 checks.

- [ ] **Step 2: Run every affected check and keep the output**

Run each alone, in this order:

```bash
cargo test --workspace --locked 2>&1 | grep -E 'test result|FAILED'
cargo clippy --workspace --all-targets --locked -- -D warnings 2>&1 | tail -1
cargo fmt --all --check && echo fmt-ok
cargo tree --workspace -d 2>&1 | head -2
bash tests/parity/run.sh 2>&1 | tail -22
bash oracles/check.sh
bash contracts/run.sh | tail -1
git ls-files '*.sh' | grep -Ev '^oracles/(bash|js)/' | xargs shellcheck; echo "shellcheck rc=$?"
bash spec/scripts/validate.sh | tail -2
bash spec/scripts/check-drift.sh; echo "drift rc=$?"
target/debug/vibe-spec validate --json | head -c 400; echo
target/debug/vibe-spec trace; echo "trace rc=$?"
bash tests/run.sh 2>&1 | tail -12
```

Expected:
- Every cargo result is `ok`, and `nothing to print`.
- Parity: the self-check passes. The candidate shows 13 `PIN` lines on macOS (12 on Linux), `0 failed, 0 error(s)`, `3 skipped`. Four `PASS  live` lines.
- `oracles: 43 files match MANIFEST.sha256`; contracts OK; `shellcheck rc=0`; validate shows `Errors: 0`; `drift rc=0`.
- `vibe-spec validate --json` on this repo prints `"layout":"flat"`. Its findings are warnings only: SF29 for stale `updated:` dates, if any. List them in the report. Do not fix them here; the owner decides during compound.
- `vibe-spec trace` exits 1 with `nothing to trace`, because this repo has no fallback feature plans. Record this in the report; it is not a failure.
- `tests/run.sh`: every suite PASS. The only tolerated failure is the engine `doctor` perf budget (P3b-R14). Any other failure blocks the gate.

- [ ] **Step 3: Program index**

In `docs/superpowers/plans/2026-10-13-spec-v04.md`, replace the 3b row's plan cell `…-p3b-spec-validators.md` (written after 3a) with `` `2026-10-15-spec-v04-p3b-spec-validators.md` ``.

- [ ] **Step 4: Update the spec docs**

```bash
bash .agents/skills/vibe/scripts/set-state.sh strategy.brainstorm
bash .agents/skills/vibe/scripts/set-state.sh strategy.spec
```

`.spec/tech-spec.md`:
- **"Strictness"**: append:

  ```markdown
  Shipped (F17): every finding names one check in `vibe-spec`'s registry —
  SF3, SF4, SF8–SF16 and I1 from the oracle, SF20–SF23 for the families it
  left unnumbered (frontmatter, naming, links, tree), SF24–SF36 added by
  F17. Keys accept `SF-nn` or `SFnn`; an unknown id warns and is ignored.
  A configured level replaces a finding's default (legacy errors stay
  errors, everything else warns); `--strict` turns warn into error; an
  allow marker `<!-- vibe-spec: allow SF-nn -->` silences that check for
  its file before either applies.
  ```

- **"Adherence checks"**: append one paragraph:

  ```markdown
  Shipped (F17): SF24 requirement population, SF25 scenario WHEN/THEN,
  SF26 frontmatter values, SF27 children and backlinks, SF28 feature
  folder ↔ plan row, SF29 freshness against git, and on layered roots
  (a root hub declares `type: hub`) SF30 allowlist, SF31 budgets, SF32
  names, SF33 typed notes (`code:`, `applies:`), SF34 hub reachability;
  in both layouts SF35/SF36 check `docs/superpowers/` names and each
  plan's `**Spec:**` path. `trace` reads units from fallback feature plans
  only; superpowers plans carry no unit ids.
  ```

- **"Shipped today"**: append "F17 adds `validate --strict`, `--format text|json` (`--json`), the check registry and strictness config, and `trace`. `--changed`, `--file` and `--format hook` arrive with F19's hooks. The fixes (R-ID prefix, RFC 2119 boundary, Scope scoping, zero-requirement short-circuit) land in Rust only; `spec/scripts/validate.sh` keeps the oracle's behaviour until the binary ships."

`.spec/tech-contracts.md`:
- **"Spec JSON v1"**: add `validate` and `trace` to the list of query shapes, with their fields as in `contracts/spec-json.schema.json`. In `validate`, `checks` lists `{id, level}` and `findings` lists `{check, severity, file?, message}`. In `trace`, `history` is one of `all|since|none`.
- **Exit-code prose**: add "`validate --format json` and `trace` exit 3 without a root; text-mode `validate` keeps the oracle's exit 1."

`.spec/tech-rust.md`:
- **"Shipped today"** paragraph: change "`list-specs`, `lessons-for`, `scan-merges` and `validate` are parity-green over … and live on this repo" to "`list-specs`, `lessons-for` and `scan-merges` are parity-green over `tests/parity/corpus/spec/` and live on this repo; `validate` is parity-green over the corpus with F17's checks off (`tests/parity/legacy-strictness.json`, P3b-R2)".
- **Pin paragraph**: change "F16's ten pins" to "Thirteen pins (ten from F16, three from F17)". Append: "F17's pins are the three bug fixes whose output differs from the oracle: whole-word R-IDs, capital whole-word RFC 2119 keywords, and Scope words inside the `## Scope` section."

`.spec/plan.md`: set the F17 row's Status to `IN PROGRESS — local gate green; CI pending push`.

`.spec/lessons.md`: add one lesson per failed command, reverted change or late-found defect in this phase (Pattern / Rule / Tags / Date 2026-10-15).

Bump `updated:` to 2026-10-15 on every edited spec doc that has frontmatter. Then run:

```bash
bash .agents/skills/vibe/scripts/regen-active-rules.sh
bash spec/scripts/validate.sh | tail -2
bash spec/scripts/check-drift.sh
bash tests/parity/run.sh 2>&1 | tail -3
bash .agents/skills/vibe/scripts/set-state.sh idle
```

- [ ] **Step 5: Commit**

```bash
git add .spec AGENTS.md docs/superpowers/plans/2026-10-13-spec-v04.md crates/vibe-spec
git commit -m "docs(spec): record f17 spec validators"
```

- [ ] **Step 6: Owner gate**

Report to the owner:
- the Step 2 outputs;
- the pin list;
- the repo's own `validate --json` findings;
- any open items.

Ask for push approval. After CI is green on every job, set F17 to `DONE` in a `strategy.spec` commit `docs(spec): close f17 row`. Then write `p3c` (F18) against the landed modules.
