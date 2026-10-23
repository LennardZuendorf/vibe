# Spec v0.4 Phase 3c — spec-delta (F18) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Deliver row F18 (#35) in `vibe-spec`:
- `check-drift` ported byte-for-byte, plus the superpowers artifact gate `--pre-merge`;
- `promote` ported byte-for-byte, plus the header-keyed `ADDED|MODIFIED|REMOVED Requirements` delta engine, the superpowers promotion checklist and `promote <feature> --done`;
- `export|import --openspec`, with OpenSpec's own validator passing on an export.

**Architecture:** One pure module, `delta.rs`, owns the requirement grammar: section headers, requirement blocks keyed by their full normalized name (OpenSpec's rule), and the apply engine. `promote`, `export`, `import` and the validators all read requirements through it. `check-drift` and `promote` keep the oracle's text output, proven by the parity corpus (stdout, exit, files) and by Rust tests that also compare stderr. The new behaviour is additive: it only fires on inputs the oracles never see (delta sections, `docs/superpowers/`, new flags).

**Tech Stack:** Rust 1.88 MSRV / edition 2024 (`clap`, `serde`, `serde_json` only), frozen bash oracles under `oracles/bash/spec/`, `git` as a subprocess, Node ≥20.19 and `@fission-ai/openspec@1.14.2` (CI gate only, through `npx`).

**Spec:** `docs/superpowers/specs/2026-10-13-spec-v04-design.md`, §5 (compound, D46), §7 (command surface) and §9 (testing). Supporting docs:
- Program index: `docs/superpowers/plans/2026-10-13-spec-v04.md` (row 3c).
- Row detail and acceptance: `docs/roadmap/issues/18-F18-spec-delta.md`.
- OpenSpec interchange: `.spec/tech-spec.md` ("OpenSpec interchange").
- Contracts: `.spec/tech-contracts.md` ("Spec JSON v1", "Exit-code contract").
- Predecessor: `docs/superpowers/plans/2026-10-15-spec-v04-p3b-spec-validators.md` (landed; CI green at `8021896`).

## Global Constraints

- Commits: Conventional Commits, imperative, lowercase, ≤50-char subject, no body, no footer.
- MSRV 1.88, edition 2024. The dependency allowlist is `clap` (derive), `serde` and `serde_json`. No `toml`, `regex`, `tokio`, `tempfile`, `chrono` or `git2`. Pattern matching is hand-written. Git runs as a subprocess through `crate::git::git`.
- `vibe-core` holds no spec vocabulary. Everything here lives in `crates/vibe-spec`, except the CI job and the gate script.
- No `unwrap`/`expect` outside tests. No panics on input: slice only with offsets you computed inside the buffer, otherwise use `get`.
- Exit codes: 0 ok · 1 findings/refused · 2 usage · 3 no spec tree. Ports keep the oracle's codes: `promote` argument errors exit 1, and `check-drift` without a root exits 0 in text mode. `--json` without a root exits 3 with empty stdout.
- The oracles under `oracles/` never change. The live bash scripts under `spec/scripts/` do not change. Skills, CI and flow keep calling the bash scripts until F21–F22 ship the binary (P3c-R2).
- Parity: with no delta sections, no `docs/superpowers/` files and no new flags, `check-drift` and `promote` stdout, exit code and written files equal the oracle's on every corpus case. The parity harness cannot see stderr, so the Rust tests in Tasks 3 and 5 compare stderr against a staged oracle.
- Every write goes through `vibe_core::fs::write_atomic`. Every multi-file command computes everything in memory and refuses before the first write.
- Printed paths are `/`-joined strings (`Root::display()`, `rel`), never `Path::display()`, so Windows output matches. Test files that spawn `bash`, `git` or `node` start with `#![cfg(unix)]`. An ungated test builds expected paths one component per `join`.
- New line scanners skip fenced code (`text::Fences`).
- Test git sandboxes use `common::git`/`common::git_out` (signing off) and check every setup command.
- Run every affected suite and show its output before claiming a task done. Never run `bash tests/run.sh`, `node flow/engine/tests/run.mjs`, `bash tests/parity/run.sh` or `bash tests/openspec/run.sh` concurrently with each other or with `cargo` builds.
- Never Edit or Write under `tests/parity/corpus/`. Corpus changes go through `crates/vibe-parity/src/corpus_spec.rs` and `cargo run --locked -q -p vibe-parity -- gen-spec-corpus oracles/bash/spec/reference/templates tests/parity/corpus/spec`. Tasks 2, 4, 6 and 7 add corpus cases and pins this way (P3c-R19).
- Tasks run with the flow cursor at `idle`. Every `.spec/` edit happens inside `bash .agents/skills/vibe/scripts/set-state.sh strategy.spec`, then `set-state.sh idle`.
- Outward-facing actions (push, PR, issue edits) need an explicit owner yes at that moment.
- Scratch files go in `.superpowers/p3c/` (gitignored), never `/tmp`.

## Review Focus

1. **Stderr the harness cannot see.** Nearly all `check-drift` output and every `promote` error goes to stderr, and `vibe-parity` compares only exit, stdout and files. Expected: candidate stderr equals the oracle's byte for byte on every Task 3 and Task 5 fixture. Tests: Task 3 `check_drift_matches_the_oracle_on_every_stream`; Task 5 `promote_matches_the_oracle_on_every_stream`.
2. **Requirement block edges.** Inputs:
   - prose after the last block inside `## Requirements` (the feature-product template has one);
   - a `---` rule before the next `##`;
   - CRLF line endings;
   - a `### Requirement:` inside a code fence;
   - a last line with no newline;
   - a target whose `## Requirements` holds a numbered list (the root product template).

   Expected:
   - A block ends at its last line that is not blank and not a thematic break.
   - Fenced headers are text.
   - ADDED lands after the section's last content line.
   - CRLF targets get CRLF separators.

   Tests: Task 1 `block_edges`, `added_lands_after_trailing_prose_and_lists`, `crlf_target_keeps_crlf`.
3. **`--done` deletes files.** Inputs:
   - a feature name like `../x`;
   - an unchanged spec root;
   - no git;
   - a shallow clone;
   - no base branch.

   Expected: nothing is deleted and the command exits 1, every time. Only a changed spec root and a valid name delete. Tests: Task 7 `done_refuses_when_the_spec_root_is_unchanged`, `done_refuses_without_a_base`, `done_rejects_unsafe_names`.
4. **A gate that checks nothing.** `openspec validate` exits 0 with `No items found to validate.` Expected: the gate asserts `summary.totals.items == 3` and `passed == 3` for the fixture, and a broken fixture must fail. Test: Task 10 `tests/openspec/run.sh`, steps 1 and 2.
5. **Requirements edited on the OpenSpec side.** An exported change whose delta spec was edited, then imported. Expected: import refuses and names the change, and writes nothing. It never silently restores the stale carried copy. Test: Task 9 `import_refuses_edited_requirements`.

## Rulings (owner review)

The owner set scope by precedent ("one plan, all of F17" for P3b). Each ruling lists its cost if wrong.

- **P3c-R1 — Scope.** One plan for all of F18. Cost: a long plan. Tasks are cut so a reviewer can reject one without its neighbours.
- **P3c-R2 — Callers stay on bash.** Design §7 says skills, CI and flow switch "in the same commit as each port". `.spec/tech-spec.md` "Shipped today" says the scripts stay until the binary ships (F21–F22), and F16 and F17 followed that. F18 adds the commands, parity and the CI gate only. Cost: `--pre-merge` is not yet wired into flow compound or `finishing-a-development-branch`; F19/F21 wire it.
- **P3c-R3 — `check-drift` port.**
  - Root by discovery. Paths print from `Root::display()`, so a `.spec` root matches the oracle byte for byte, and a `docs/spec` root prints `docs/spec/...`.
  - Check (a)'s substring match (`grep -qF "$feat"`) is ported as-is: port first.
  - Text mode without a root: the oracle's stderr line and exit 0, naming the nearest ancestor of the cwd that holds `.spec` or `.git`.
  - The oracle ignores all arguments. The port parses `--pre-merge` and `--json` with clap, so an unknown flag exits 2. The corpus passes no arguments.

  Cost: a feature named `auth` still counts as recorded when the plan only says `oauth`.
- **P3c-R4 — Superpowers artifact gate.**
  - Scope: every regular file under `<project>/docs/superpowers/`, recursive, dot names skipped, symlinks not followed.
  - Plain `check-drift` warns once per file. `--pre-merge` makes each an error.
  - The lines come after check (c), before the summary.

  Cost: this repo's plain `check-drift` warns until the program's own artifacts are deleted (D46 dogfood). That is intended.
- **P3c-R5 — JSON.** `check-drift`, `promote`, `export` and `import` take `--json`. New Spec JSON v1 defs: `check-drift`, `promote`, `openspec` (both directions). Each def has ≥2 valid goldens, ≥2 invalid goldens and `valid/unknown-field.json`. In `--json` mode stdout carries only the JSON. Errors and refusals still go to stderr.
- **P3c-R6 — `promote` port.**
  - Byte parity for merge blocks: `--dry-run`, `--target <path>` (cwd-relative, printed as given), every message, abort-on-nested, the read-loop's dropped unterminated last line, and empty blocks always counting as present.
  - Argument errors exit 1, as the oracle does: a missing feature, `--target` without a path, an unknown option.
  - The feature name stays unsanitized on the legacy path, as in the oracle.
- **P3c-R7 — Requirement keys.**
  - A block's key is its full header name, normalized the way OpenSpec does: the text after `### Requirement:` (case-insensitive keyword, `###` exactly), a trailing ` ###`-style run dropped, spaces and tabs trimmed.
  - Comparison is case-sensitive.
  - The R-ID is part of the name, not the key. Renaming a requirement is REMOVED plus ADDED. `RENAMED Requirements` is refused everywhere.

  Cost: a MODIFIED cannot change a requirement's title. Write REMOVED + ADDED instead.
- **P3c-R8 — Requirement containers.**
  - Requirements live under an H2 that matches OpenSpec's `^##\s+Requirements\s*$` (case-insensitive): the root and topic templates' "Requirements" section. Delta sections match `^##\s+(ADDED|MODIFIED|REMOVED|RENAMED)\s+Requirements\s*$`.
  - A section ends at the next non-fenced heading of level 1–2.
  - A block runs from its header to the line before the next non-fenced heading of level 1–3, minus trailing blank lines and thematic breaks (`---`, `***`, `___`).
  - `promote`, `export`, `import` and the validators share these rules.
- **P3c-R9 — Applying a delta.**
  - Order: REMOVED, then MODIFIED, then ADDED, in document order within each.
  - ADDED: block absent → append after the last content line of the target's last `## Requirements` section. If the target has no such section, append one at EOF. Present and byte-equal → skipped. Present with other text → refused.
  - MODIFIED: absent → refused. Byte-equal → skipped. Otherwise the block is replaced in place.
  - REMOVED: needs a `**Reason**:` line (the why-line), else refused. Absent from the target → skipped. Present → the block and the blank lines after it are removed; at EOF, the blank lines before it go instead, so no blank tail stays.
  - A name twice in the delta is refused. A target with a duplicate name is refused.
  - Every refusal is collected before any byte changes, and one refusal means no write.
  - A plain `## Requirements` section in a feature product is not a delta. The agent promotes it, as before.
- **P3c-R10 — Delta target.** The feature `product.md` frontmatter `parent:`, resolved from the feature folder and normalized lexically. Without one, the target is `<root>/product.md`. A target outside the spec root is refused. Cost: a feature can feed only one target document.
- **P3c-R11 — Validators and REMOVED.** Lines of `## REMOVED Requirements` sections are blanked (newlines kept) before SF10, SF12 (R-IDs), SF24 and SF25 read a feature product. No new check id; the matrix stays at 29. Cost: a product holding only REMOVED blocks gets SF24's "no requirement blocks" warning.
- **P3c-R12 — Superpowers binding.**
  - Artifacts are regular files `docs/superpowers/specs/YYYY-MM-DD-<f>-design.md` and `docs/superpowers/plans/YYYY-MM-DD-<f>.md`, every date, exact slug.
  - The lookup runs only for a valid feature name (`^[a-z0-9][a-z0-9-]*$`).
  - `promote <f>` prints the artifact list and the promotion checklist, with targets per layout (D44 kind folders when layered; root docs when flat) and no writes.
  - With a fallback folder too, the legacy promotion runs first, then the checklist.

  Cost: phase plans such as `…-spec-v04-p3b-…` are not bound to feature `spec-v04`. `check-drift --pre-merge` catches them.
- **P3c-R13 — `promote --done`.**
  - It deletes only the bound artifacts, by filesystem remove. The git index is not touched.
  - It refuses (exit 1, nothing deleted) unless the spec root changed since the branch base. The base is `--base <ref>`, else the merge-base of HEAD with the first of `origin/HEAD`, `origin/main`, `main`, `master` that names a commit.
  - "Changed" means `git diff --name-only <base> -- .` or `git ls-files --others --exclude-standard -- .`, run inside the spec root, prints anything.
  - No git, no history, a shallow clone without a reachable base, or no base at all → refused (fail closed).
  - `--done --dry-run` prints `WOULD DELETE` lines and deletes nothing.
- **P3c-R14 — `export --openspec <dir>`.**
  - Writes `<dir>/openspec/`. Refuses when that already exists.
  - Capabilities: every spec document outside `features/` and `archive/` holding ≥1 block in a `## Requirements` section. The capability path is the document path without `.md`, so `product-auth` and `concepts/invoice` are both valid (OpenSpec allows nested capability paths).
  - `## Purpose` is a generated sentence (≥50 chars): root documents are vibe's own and are never imported.
  - Every fallback feature with a `product.md` becomes `changes/<f>/`:
    - `proposal.md`;
    - one delta spec for its target's capability (plain `## Requirements` blocks export as ADDED);
    - `tasks.md` from the plan's units, with `[x]` for DONE in Progress;
    - `design.md` carrying `product.md`, `tech.md`, `design.md`, `plan.md` and `research.md` byte-for-byte in length-prefixed `## vibe:<file>` blocks after a `<!-- vibe:export v1 -->` marker.
  - A feature with no requirement blocks gets `.openspec.yaml` with `skip_specs: true`.
  - A MODIFIED that drops a scenario the target still has is refused, because OpenSpec errors on it.
  - A duplicate requirement name in one capability is refused.
  - Other files in a feature folder are not exported; a stderr note names each.
- **P3c-R15 — `import --openspec <dir> [--feature <name>]`.**
  - Imports changes only, into `<root>/features/<id>/`. `openspec/specs/` is never written into vibe's root documents.
  - Refuses: an existing folder, an invalid id, `RENAMED`, more than one capability.
  - With the vibe marker, the carried files are restored byte-for-byte, but only if the change's delta blocks equal the carried `product.md`'s blocks (op by op, trailing newline normalized). Otherwise import refuses (Review Focus 5).
  - Without the marker, it builds `product.md`, `tech.md`, `plan.md` (units from `tasks.md`) and `design.md` (copied) from the change, with `updated:` as today's UTC date.
- **P3c-R16 — Round trip.** export → import into an empty root reproduces every carried file byte-for-byte, and the requirement blocks are byte-identical. This is the primary correctness signal (issue note).
- **P3c-R17 — OpenSpec gate.**
  - `tests/openspec/run.sh` exports `tests/openspec/fixture/` and runs `OPENSPEC_TELEMETRY=0 npx -y @fission-ai/openspec@1.14.2 validate --all --strict --no-interactive --json`.
  - It asserts 3 items and 3 passed (one is the `skip_specs` change), then proves that a broken fixture fails.
  - It runs as its own CI job (network). It is not part of `tests/run.sh`.

  Cost: CI depends on the npm registry for that job.
- **P3c-R18 — `today`.** Foreign imports date their docs with a hand-written civil-from-days conversion of the system clock (UTC). Tests pin the conversion, not the clock.

- **P3c-R19 — Pinned divergences.** Every change the oracle can see gets a corpus case and a pin in `tests/parity/spec.json`, as P3b-R2 did:
  - `features/delta-removed`: a feature product whose only requirement section is REMOVED, with a `parent:` topic target.
    - Task 2 pins `validate` stdout (the SF10/SF12 exemption).
    - Task 6 pins `promote auth --dry-run` stdout, and `promote auth` stdout and files (the deltas are applied).
  - `features/superpowers-artifact`: one file under `docs/superpowers/plans/`.
    - Task 4 pins `check-drift` stdout (one more warning).
    - Task 7 pins `promote auth` exit and stdout (a checklist instead of `not found`).

  The `features` floor rises from 13 to 15. A pin that no run hits is reported stale, so each pin must keep earning its place.

## File Structure

| File | Responsibility |
|---|---|
| `crates/vibe-spec/src/delta.rs` (new) | Requirement grammar: section kinds, blocks, names, scenarios, `apply`, `without_removed`, `target` |
| `crates/vibe-spec/src/drift.rs` (new) | `check-drift` port, superpowers artifact gate, `--json` |
| `crates/vibe-spec/src/promote.rs` (new) | `promote` port, deltas, checklist, `--done`, `--json` |
| `crates/vibe-spec/src/openspec.rs` (new) | `export` / `import --openspec`, carried-file encoding, tasks mapping |
| `crates/vibe-spec/src/main.rs` | New subcommands `check-drift`, `promote`, `export`, `import` |
| `crates/vibe-spec/src/git.rs` | `branch_base`, `changed_since` |
| `crates/vibe-spec/src/superpowers.rs` | `artifacts(project, feature)` |
| `crates/vibe-spec/src/query.rs` | `valid_name` made `pub(crate)` |
| `crates/vibe-spec/src/layered.rs` | `lexical` made `pub(crate)` |
| `crates/vibe-spec/src/text.rs` | `walk_files`, `today_utc`, `civil` |
| `crates/vibe-spec/src/content.rs`, `validate.rs`, `layered.rs` | REMOVED exemption (P3c-R11) |
| `crates/vibe-spec/tests/{drift,promote,openspec}.rs` (new) | Integration tests |
| `crates/vibe-spec/tests/common/mod.rs` | `staged_oracle` |
| `tests/parity/spec.json` | Candidates for `check-drift` and `promote`; pins; `features` floor |
| `crates/vibe-parity/src/corpus_spec.rs`, `tests/parity/corpus/spec/` (regenerated) | Cases `features/delta-removed`, `features/superpowers-artifact` |
| `contracts/spec-json.schema.json`, `contracts/run.mjs`, `contracts/fixtures/spec-json/{check-drift,promote,openspec}/` | JSON contracts |
| `tests/openspec/run.sh`, `tests/openspec/check.mjs`, `tests/openspec/fixture/` (new) | The OpenSpec validate gate |
| `.github/workflows/ci.yml` | `openspec` job |
| `.spec/{tech-spec,tech-contracts,tech-rust,plan,lessons}.md`, `CHANGELOG.md`, `README.md`, `docs/superpowers/plans/2026-10-13-spec-v04.md` | Docs (Task 11) |

---

### Task 1: Requirement grammar and the delta engine

**Files:**
- Create: `crates/vibe-spec/src/delta.rs`
- Modify: `crates/vibe-spec/src/main.rs` (declare the module)
- Modify: `crates/vibe-spec/src/layered.rs:321` (`fn lexical` → `pub(crate) fn lexical`)

**Interfaces:**
- Consumes: `crate::text::{Fences, lossy}`, `crate::layered::lexical(&Path) -> PathBuf`, `crate::frontmatter::parse(&[u8]) -> Option<Frontmatter>` with `Frontmatter::scalar(&str) -> Option<&str>`.
- Produces (every later task uses these names):
  - `pub enum Kind { Plain, Added, Modified, Removed, Renamed }`, `Kind::as_str(self) -> &'static str` (`"REQUIREMENTS"`, `"ADDED"`, …).
  - `pub struct Block { pub name: Vec<u8>, pub start: usize, pub end: usize }`: the block is `text[start..end]`.
  - `pub struct Section { pub kind: Kind, pub header: usize, pub end: usize, pub blocks: Vec<Block> }`.
  - `pub struct Change { pub kind: Kind, pub name: Vec<u8>, pub applied: bool }`.
  - `pub fn requirement_name(line: &[u8]) -> Option<Vec<u8>>`
  - `pub fn section_kind(line: &[u8]) -> Option<Kind>`
  - `pub fn sections(text: &[u8]) -> Vec<Section>`
  - `pub fn requirements(text: &[u8]) -> Vec<Block>`: blocks of the plain `## Requirements` sections.
  - `pub fn has_delta(text: &[u8]) -> bool`
  - `pub fn without_removed(text: &[u8]) -> Vec<u8>`
  - `pub fn scenarios(block: &[u8]) -> Vec<Vec<u8>>`
  - `pub fn apply(target: &[u8], delta: &[u8]) -> Result<(Vec<u8>, Vec<Change>), Vec<String>>`
  - `pub fn target(root: &Path, feature_dir: &Path, product: &[u8]) -> Result<PathBuf, String>`

- [ ] **Step 1: Write the failing tests**

Create `crates/vibe-spec/src/delta.rs` with only the module doc line and the test module below. Declare it in `main.rs` after `mod content;`:

```rust
#[allow(dead_code)] // Task 8 removes this once every item has a caller.
mod delta;
```

```rust
//! Requirement grammar and the delta engine (F18, D34, P3c-R7–R10).

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    const TARGET: &str = "# Auth\n\n## Requirements\n\n### Requirement: Sign in (R1)\nUsers SHALL sign in.\n\n#### Scenario: ok\n- **WHEN** valid\n- **THEN** in\n\n### Requirement: Sign out (R2)\nUsers SHALL sign out.\n\n#### Scenario: out\n- **WHEN** asked\n- **THEN** out\n\n---\n\n## Non-Goals\n\n- SSO\n";
    const AUDIT: &str = "## ADDED Requirements\n\n### Requirement: Audit\nThe system SHALL log.\n\n#### Scenario: log\n- **WHEN** x\n- **THEN** y\n";

    fn applied(target: &str, delta: &str) -> (String, Vec<(Kind, String, bool)>) {
        let (doc, changes) = apply(target.as_bytes(), delta.as_bytes()).unwrap();
        (
            String::from_utf8(doc).unwrap(),
            changes
                .into_iter()
                .map(|c| (c.kind, String::from_utf8(c.name).unwrap(), c.applied))
                .collect(),
        )
    }

    #[test]
    fn names_follow_openspec() {
        assert_eq!(requirement_name(b"### Requirement: Sign in (R1)\n"), Some(b"Sign in (R1)".to_vec()));
        assert_eq!(requirement_name(b"###requirement:  Export  ##  \r\n"), Some(b"Export".to_vec()));
        assert_eq!(requirement_name(b"### Requirement: C# basics\n"), Some(b"C# basics".to_vec()));
        assert_eq!(requirement_name(b"#### Requirement: x\n"), None);
        assert_eq!(requirement_name(b"### Requirement:   \n"), None);
        assert_eq!(requirement_name(b"## Requirement: x\n"), None);
    }

    #[test]
    fn section_kinds() {
        assert_eq!(section_kind(b"## Requirements\n"), Some(Kind::Plain));
        assert_eq!(section_kind(b"## added   requirements \r\n"), Some(Kind::Added));
        assert_eq!(section_kind(b"## RENAMED Requirements\n"), Some(Kind::Renamed));
        assert_eq!(section_kind(b"## Requirements Trace\n"), None);
        assert_eq!(section_kind(b"### Requirements\n"), None);
        assert_eq!(section_kind(b"##Requirements\n"), None);
    }

    #[test]
    fn block_edges() {
        let text = "## Requirements\n\n### Requirement: A\nThe system SHALL a.\n\n```text\n### Requirement: Fenced\n## Not a section\n```\n\nTrailing prose.\n\n---\n\n## Next\n\n### Requirement: Outside\nx\n";
        let secs = sections(text.as_bytes());
        assert_eq!(secs.len(), 1);
        let b = &secs[0].blocks;
        assert_eq!(b.len(), 1);
        assert_eq!(
            &text.as_bytes()[b[0].start..b[0].end],
            "### Requirement: A\nThe system SHALL a.\n\n```text\n### Requirement: Fenced\n## Not a section\n```\n\nTrailing prose.\n".as_bytes()
        );
        let eof = "## Requirements\n### Requirement: B\nx";
        let s = sections(eof.as_bytes());
        assert_eq!(&eof.as_bytes()[s[0].blocks[0].start..s[0].blocks[0].end], b"### Requirement: B\nx");
    }

    #[test]
    fn modified_replaces_in_place_byte_exact() {
        let delta = "## MODIFIED Requirements\n\n### Requirement: Sign in (R1)\nUsers SHALL sign in with a passkey.\n\n#### Scenario: ok\n- **WHEN** valid\n- **THEN** in\n";
        let (doc, ch) = applied(TARGET, delta);
        assert_eq!(doc, TARGET.replace("Users SHALL sign in.\n", "Users SHALL sign in with a passkey.\n"));
        assert_eq!(ch, [(Kind::Modified, "Sign in (R1)".to_string(), true)]);
        let last = "## MODIFIED Requirements\n### Requirement: Sign out (R2)\nUsers MUST sign out.\n#### Scenario: out\n- **WHEN** asked\n- **THEN** out\n";
        let (doc, _) = applied(TARGET, last);
        assert_eq!(doc, TARGET.replace("Users SHALL sign out.\n\n#### Scenario: out", "Users MUST sign out.\n#### Scenario: out"));
    }

    #[test]
    fn added_lands_after_trailing_prose_and_lists() {
        let block = "### Requirement: Audit\nThe system SHALL log.\n\n#### Scenario: log\n- **WHEN** x\n- **THEN** y\n";
        let (doc, ch) = applied(TARGET, AUDIT);
        assert_eq!(doc, TARGET.replace("- **THEN** out\n\n---", &format!("- **THEN** out\n\n{block}\n---")));
        assert_eq!(ch, [(Kind::Added, "Audit".to_string(), true)]);
        let list = "# P\n\n## Requirements\n\nAt a project level, P must:\n\n1. **One.** a\n2. **Two.** b\n\n---\n\n## Design Principles\n";
        let (doc, _) = applied(list, AUDIT);
        assert_eq!(doc, list.replace("2. **Two.** b\n", &format!("2. **Two.** b\n\n{block}")));
        let (doc, _) = applied("# P\n\nNo section.", AUDIT);
        assert_eq!(doc, format!("# P\n\nNo section.\n\n## Requirements\n\n{block}"));
    }

    #[test]
    fn removed_needs_a_reason_and_removes_the_block() {
        let delta = "## REMOVED Requirements\n\n### Requirement: Sign out (R2)\n**Reason**: sessions never end.\n";
        let (doc, ch) = applied(TARGET, delta);
        assert_eq!(
            doc,
            TARGET.replace("### Requirement: Sign out (R2)\nUsers SHALL sign out.\n\n#### Scenario: out\n- **WHEN** asked\n- **THEN** out\n\n", "")
        );
        assert_eq!(ch, [(Kind::Removed, "Sign out (R2)".to_string(), true)]);
        let err = apply(TARGET.as_bytes(), b"## REMOVED Requirements\n### Requirement: Sign out (R2)\nGone.\n").unwrap_err();
        assert_eq!(err, ["REMOVED 'Sign out (R2)' has no **Reason**: line"]);
    }

    #[test]
    fn removing_the_last_block_at_eof_drops_the_gap() {
        let t = "## Requirements\n\n### Requirement: A\nx\n\n### Requirement: B\ny\n";
        let (doc, _) = applied(t, "## REMOVED Requirements\n### Requirement: B\n**Reason**: z\n");
        assert_eq!(doc, "## Requirements\n\n### Requirement: A\nx\n");
    }

    #[test]
    fn reapplying_is_idempotent() {
        let delta = "## REMOVED Requirements\n### Requirement: Sign out (R2)\n**Reason**: x\n\n## MODIFIED Requirements\n### Requirement: Sign in (R1)\nUsers SHALL sign in fast.\n#### Scenario: ok\n- **WHEN** a\n- **THEN** b\n\n## ADDED Requirements\n### Requirement: Audit\nThe system SHALL log.\n#### Scenario: log\n- **WHEN** x\n- **THEN** y\n";
        let (once, _) = applied(TARGET, delta);
        let (twice, ch) = applied(&once, delta);
        assert_eq!(twice, once);
        assert!(ch.iter().all(|(_, _, applied)| !applied), "{ch:?}");
    }

    #[test]
    fn refusals_are_collected_and_nothing_applies() {
        let delta = "## MODIFIED Requirements\n### Requirement: Ghost\nx\n\n## ADDED Requirements\n### Requirement: Sign in (R1)\nOther text.\n\n### Requirement: Twice\na\n\n### Requirement: Twice\nb\n\n## RENAMED Requirements\n- FROM: `### Requirement: A`\n- TO: `### Requirement: B`\n";
        let err = apply(TARGET.as_bytes(), delta.as_bytes()).unwrap_err();
        assert_eq!(
            err,
            [
                "RENAMED Requirements is not supported; write REMOVED + ADDED",
                "duplicate requirement 'Twice' in the delta",
                "MODIFIED 'Ghost' is not in the target",
                "ADDED 'Sign in (R1)' already exists in the target with other text",
            ]
        );
        let dup = "## Requirements\n### Requirement: A\nx\n### Requirement: A\ny\n";
        assert_eq!(apply(dup.as_bytes(), AUDIT.as_bytes()).unwrap_err(), ["the target has duplicate requirement 'A'"]);
    }

    #[test]
    fn crlf_target_keeps_crlf() {
        let target = TARGET.replace('\n', "\r\n");
        let delta = "## ADDED Requirements\n### Requirement: Audit\nThe system SHALL log.\n#### Scenario: log\n- **WHEN** x\n- **THEN** y";
        let (doc, _) = applied(&target, delta);
        assert!(
            doc.contains("- **THEN** out\r\n\r\n### Requirement: Audit\nThe system SHALL log.\n#### Scenario: log\n- **WHEN** x\n- **THEN** y\r\n\r\n---"),
            "{doc:?}"
        );
        let (again, ch) = applied(&doc, delta);
        assert_eq!(again, doc);
        assert!(!ch[0].2);
    }

    #[test]
    fn without_removed_blanks_only_removed_sections() {
        let text = "## REMOVED Requirements\n### Requirement: X (R9)\n**Reason**: y\n\n## Requirements\n### Requirement: Y\n";
        assert_eq!(without_removed(text.as_bytes()), b"\n\n\n\n## Requirements\n### Requirement: Y\n");
        assert!(has_delta(text.as_bytes()));
        assert!(!has_delta(b"## Requirements\n### Requirement: Y\n"));
    }

    #[test]
    fn scenario_names() {
        assert_eq!(
            scenarios(b"### Requirement: A\nx\n#### Scenario: One\n- a\n####  scenario:two  ##\n```\n#### Scenario: fenced\n```\n"),
            [b"One".to_vec(), b"two".to_vec()]
        );
    }

    #[test]
    fn targets_resolve_from_the_feature_folder() {
        let root = Path::new("/p/.spec");
        let fd = root.join("features").join("auth");
        assert_eq!(target(root, &fd, b"---\nparent: ../../product-auth.md\n---\n").unwrap(), root.join("product-auth.md"));
        assert_eq!(target(root, &fd, b"# no frontmatter\n").unwrap(), root.join("product.md"));
        assert_eq!(
            target(root, &fd, b"---\nparent: ../../../x.md\n---\n").unwrap_err(),
            "parent '../../../x.md' leaves the spec root"
        );
    }
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p vibe-spec --bin vibe-spec delta:: 2>&1 | tail -5`
Expected: compile errors (`cannot find function apply`, `cannot find type Kind`, …).

- [ ] **Step 3: Write the implementation**

Insert above the test module in `delta.rs`, and make `lexical` in `layered.rs` `pub(crate)`:

```rust
//! Requirement grammar and the delta engine (F18, D34, P3c-R7–R10):
//! OpenSpec's `### Requirement:` blocks and `## ADDED|MODIFIED|REMOVED
//! Requirements` sections inside vibe's own documents. Headers match the way
//! OpenSpec matches them; a requirement's key is its full normalized name.
use std::path::{Path, PathBuf};

use crate::layered::lexical;
use crate::text::{Fences, lossy};

/// What a requirement-bearing `##` section holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// `## Requirements`.
    Plain,
    /// `## ADDED Requirements`.
    Added,
    /// `## MODIFIED Requirements`.
    Modified,
    /// `## REMOVED Requirements`.
    Removed,
    /// `## RENAMED Requirements`, read only to refuse it.
    Renamed,
}

impl Kind {
    /// The OpenSpec section word.
    pub fn as_str(self) -> &'static str {
        match self {
            Kind::Plain => "REQUIREMENTS",
            Kind::Added => "ADDED",
            Kind::Modified => "MODIFIED",
            Kind::Removed => "REMOVED",
            Kind::Renamed => "RENAMED",
        }
    }
}

/// One requirement block: `text[start..end]`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Block {
    pub name: Vec<u8>,
    pub start: usize,
    pub end: usize,
}

/// One requirement-bearing section: the header line starts at `header`; the
/// section ends at `end` (exclusive).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Section {
    pub kind: Kind,
    pub header: usize,
    pub end: usize,
    pub blocks: Vec<Block>,
}

/// One delta entry and whether it changed the target.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Change {
    pub kind: Kind,
    pub name: Vec<u8>,
    pub applied: bool,
}

/// `(start, end)` of every line, `\n` included; a last line without `\n` counts.
fn spans(text: &[u8]) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let mut start = 0;
    while start < text.len() {
        let end = text
            .get(start..)
            .and_then(|rest| rest.iter().position(|&b| b == b'\n'))
            .map_or(text.len(), |i| start + i + 1);
        out.push((start, end));
        start = end;
    }
    out
}

/// A line without its `\n` and `\r`.
fn bare(line: &[u8]) -> &[u8] {
    let l = line.strip_suffix(b"\n").unwrap_or(line);
    l.strip_suffix(b"\r").unwrap_or(l)
}

fn is_ws(c: u8) -> bool {
    c == b' ' || c == b'\t'
}

fn trim_ws(s: &[u8]) -> &[u8] {
    let start = s.iter().position(|&c| !is_ws(c)).unwrap_or(s.len());
    let end = s.iter().rposition(|&c| !is_ws(c)).map_or(start, |i| i + 1);
    s.get(start..end).unwrap_or_default()
}

/// OpenSpec's `normalizeRequirementName`: drop a closing `#` run that
/// follows a space or tab, then trim.
fn normalize(name: &[u8]) -> &[u8] {
    let t = trim_ws(name);
    let hashes = t.iter().rev().take_while(|&&c| c == b'#').count();
    let before = t.get(..t.len() - hashes).unwrap_or_default();
    if hashes > 0 && before.last().is_some_and(|&c| is_ws(c)) {
        trim_ws(before)
    } else {
        t
    }
}

/// The key of a `### Requirement: <name>` line (OpenSpec's
/// `^###\s*Requirement:\s*(.+)\s*$`, case-insensitive keyword); None for
/// any other line or an empty name.
pub fn requirement_name(line: &[u8]) -> Option<Vec<u8>> {
    let rest = bare(line).strip_prefix(b"###")?;
    if rest.first() == Some(&b'#') {
        return None;
    }
    let rest = rest.get(rest.iter().position(|&c| !is_ws(c))?..)?;
    if !rest.get(..12)?.eq_ignore_ascii_case(b"requirement:") {
        return None;
    }
    let name = normalize(rest.get(12..).unwrap_or_default());
    (!name.is_empty()).then(|| name.to_vec())
}

/// The kind of a `##` requirement section header.
pub fn section_kind(line: &[u8]) -> Option<Kind> {
    let rest = bare(line).strip_prefix(b"##")?;
    if !rest.first().is_some_and(|&c| is_ws(c)) {
        return None;
    }
    let words: Vec<&[u8]> = rest.split(|&c| is_ws(c)).filter(|w| !w.is_empty()).collect();
    let req = |w: &[u8]| w.eq_ignore_ascii_case(b"requirements");
    match words.as_slice() {
        [w] if req(w) => Some(Kind::Plain),
        [op, w] if req(w) => match op.to_ascii_uppercase().as_slice() {
            b"ADDED" => Some(Kind::Added),
            b"MODIFIED" => Some(Kind::Modified),
            b"REMOVED" => Some(Kind::Removed),
            b"RENAMED" => Some(Kind::Renamed),
            _ => None,
        },
        _ => None,
    }
}

/// The ATX heading level (1–6) of a line; a requirement header is level 3.
fn level(line: &[u8]) -> Option<usize> {
    if requirement_name(line).is_some() {
        return Some(3);
    }
    let l = bare(line);
    let n = l.iter().take_while(|&&c| c == b'#').count();
    ((1..=6).contains(&n) && l.get(n).is_none_or(|&c| is_ws(c))).then_some(n)
}

fn is_blank(line: &[u8]) -> bool {
    trim_ws(bare(line)).is_empty()
}

/// `---`, `***` or `___` (three or more, spaces allowed).
fn is_break(line: &[u8]) -> bool {
    let t: Vec<u8> = bare(line).iter().copied().filter(|&c| !is_ws(c)).collect();
    t.len() >= 3 && [b'-', b'*', b'_'].iter().any(|&m| t.iter().all(|&c| c == m))
}

/// Every requirement-bearing section, fences skipped (P3c-R8). A section
/// ends at the next heading of level 1–2; a block at the next heading of
/// level 1–3, minus trailing blank lines and thematic breaks.
pub fn sections(text: &[u8]) -> Vec<Section> {
    let mut out: Vec<Section> = Vec::new();
    let mut open: Option<Section> = None;
    let mut block: Option<(Vec<u8>, usize)> = None;
    let mut last = 0;
    let mut fences = Fences::new();
    for (s, e) in spans(text) {
        let line = text.get(s..e).unwrap_or_default();
        if fences.skip(line) {
            if block.is_some() {
                last = e;
            }
            continue;
        }
        let lvl = level(line);
        if lvl.is_some_and(|l| l <= 3) {
            if let (Some(sec), Some((name, start))) = (open.as_mut(), block.take()) {
                sec.blocks.push(Block { name, start, end: last });
            }
        }
        if lvl.is_some_and(|l| l <= 2) {
            if let Some(mut sec) = open.take() {
                sec.end = s;
                out.push(sec);
            }
            if let Some(kind) = section_kind(line) {
                open = Some(Section { kind, header: s, end: text.len(), blocks: Vec::new() });
            }
            continue;
        }
        if open.is_none() {
            continue;
        }
        if let Some(name) = requirement_name(line) {
            block = Some((name, s));
            last = e;
        } else if block.is_some() && !is_blank(line) && !is_break(line) {
            last = e;
        }
    }
    if let (Some(sec), Some((name, start))) = (open.as_mut(), block.take()) {
        sec.blocks.push(Block { name, start, end: last });
    }
    if let Some(sec) = open.take() {
        out.push(sec);
    }
    out
}

/// The blocks of every plain `## Requirements` section.
pub fn requirements(text: &[u8]) -> Vec<Block> {
    sections(text)
        .into_iter()
        .filter(|s| s.kind == Kind::Plain)
        .flat_map(|s| s.blocks)
        .collect()
}

/// Whether `text` has an ADDED, MODIFIED, REMOVED or RENAMED section.
pub fn has_delta(text: &[u8]) -> bool {
    sections(text).iter().any(|s| s.kind != Kind::Plain)
}

/// `text` with every byte of every `## REMOVED Requirements` section
/// dropped except its newlines, so line numbers hold (P3c-R11).
pub fn without_removed(text: &[u8]) -> Vec<u8> {
    let removed: Vec<(usize, usize)> = sections(text)
        .iter()
        .filter(|s| s.kind == Kind::Removed)
        .map(|s| (s.header, s.end))
        .collect();
    text.iter()
        .enumerate()
        .filter(|&(i, &b)| b == b'\n' || !removed.iter().any(|&(s, e)| (s..e).contains(&i)))
        .map(|(_, &b)| b)
        .collect()
}

/// The scenario names of a block: every non-fenced `####` header with a
/// leading `Scenario:` (any case) dropped, normalized like a name.
pub fn scenarios(block: &[u8]) -> Vec<Vec<u8>> {
    let mut fences = Fences::new();
    let mut out = Vec::new();
    for (s, e) in spans(block) {
        let line = block.get(s..e).unwrap_or_default();
        if fences.skip(line) || level(line) != Some(4) {
            continue;
        }
        let t = trim_ws(bare(line).get(4..).unwrap_or_default());
        let t = match t.get(..9) {
            Some(p) if p.eq_ignore_ascii_case(b"scenario:") => t.get(9..).unwrap_or_default(),
            _ => t,
        };
        out.push(normalize(t).to_vec());
    }
    out
}

/// `\r\n` when the first line of `text` ends with it, else `\n`.
fn eol(text: &[u8]) -> &'static [u8] {
    match text.iter().position(|&b| b == b'\n') {
        Some(i) if i > 0 && text.get(i - 1) == Some(&b'\r') => b"\r\n",
        _ => b"\n",
    }
}

/// A block without its final line ending, for comparisons.
fn core(b: &[u8]) -> &[u8] {
    let b = b.strip_suffix(b"\n").unwrap_or(b);
    b.strip_suffix(b"\r").unwrap_or(b)
}

fn with_eol(b: &[u8], nl: &[u8]) -> Vec<u8> {
    let mut v = b.to_vec();
    if !v.ends_with(b"\n") {
        v.extend_from_slice(nl);
    }
    v
}

/// A `**Reason**:` line after the header (the REMOVED why-line).
fn has_reason(block: &[u8]) -> bool {
    spans(block)
        .iter()
        .skip(1)
        .any(|&(s, e)| trim_ws(bare(block.get(s..e).unwrap_or_default())).starts_with(b"**Reason**:"))
}

fn splice(doc: &[u8], start: usize, end: usize, with: &[u8]) -> Vec<u8> {
    let mut out = doc.get(..start).unwrap_or_default().to_vec();
    out.extend_from_slice(with);
    out.extend_from_slice(doc.get(end..).unwrap_or_default());
    out
}

fn find_block(doc: &[u8], name: &[u8]) -> Option<Block> {
    requirements(doc).into_iter().find(|b| b.name == name)
}

fn replace(doc: &[u8], name: &[u8], bytes: &[u8]) -> Vec<u8> {
    let Some(b) = find_block(doc, name) else {
        return doc.to_vec();
    };
    let ends_nl = doc.get(b.start..b.end).is_some_and(|o| o.ends_with(b"\n"));
    splice(doc, b.start, b.end, if ends_nl { bytes } else { core(bytes) })
}

fn remove(doc: &[u8], name: &[u8]) -> Vec<u8> {
    let Some(b) = find_block(doc, name) else {
        return doc.to_vec();
    };
    let lines = spans(doc);
    let mut end = b.end;
    for &(s, e) in &lines {
        if s < b.end {
            continue;
        }
        if !is_blank(doc.get(s..e).unwrap_or_default()) {
            break;
        }
        end = e;
    }
    // At EOF, drop the gap before the block instead, so no blank tail stays.
    let mut start = b.start;
    if end == doc.len() {
        for &(s, e) in lines.iter().rev() {
            if e > b.start {
                continue;
            }
            if !is_blank(doc.get(s..e).unwrap_or_default()) {
                break;
            }
            start = s;
        }
    }
    splice(doc, start, end, b"")
}

/// Appends `bytes` after the last content line of the last plain
/// `## Requirements` section; without one, appends that section at EOF.
fn add(doc: &[u8], bytes: &[u8]) -> Vec<u8> {
    let nl = eol(doc);
    let Some(sec) = sections(doc).into_iter().rfind(|s| s.kind == Kind::Plain) else {
        let mut out = doc.to_vec();
        if !out.is_empty() {
            if !out.ends_with(b"\n") {
                out.extend_from_slice(nl);
            }
            out.extend_from_slice(nl);
        }
        out.extend_from_slice(b"## Requirements");
        out.extend_from_slice(nl);
        out.extend_from_slice(nl);
        out.extend_from_slice(bytes);
        return out;
    };
    let mut at = sec.header;
    let mut fences = Fences::new();
    for (s, e) in spans(doc) {
        if s < sec.header || s >= sec.end {
            continue;
        }
        if s == sec.header {
            at = e;
            continue;
        }
        let line = doc.get(s..e).unwrap_or_default();
        if fences.skip(line) || (!is_blank(line) && !is_break(line)) {
            at = e;
        }
    }
    let mut with = Vec::new();
    if !doc.get(..at).unwrap_or_default().ends_with(b"\n") {
        with.extend_from_slice(nl);
    }
    with.extend_from_slice(nl);
    with.extend_from_slice(bytes);
    splice(doc, at, at, &with)
}

/// Applies `delta`'s sections to `target` (P3c-R9): REMOVED, then MODIFIED,
/// then ADDED. Err holds every refusal, and then nothing is applied.
pub fn apply(target: &[u8], delta: &[u8]) -> Result<(Vec<u8>, Vec<Change>), Vec<String>> {
    let nl = eol(target);
    let mut errs: Vec<String> = Vec::new();
    let secs = sections(delta);
    if secs.iter().any(|s| s.kind == Kind::Renamed) {
        errs.push("RENAMED Requirements is not supported; write REMOVED + ADDED".to_string());
    }
    let mut entries: Vec<(Kind, Vec<u8>, Vec<u8>)> = Vec::new();
    for kind in [Kind::Removed, Kind::Modified, Kind::Added] {
        for sec in secs.iter().filter(|s| s.kind == kind) {
            for b in &sec.blocks {
                let bytes = with_eol(delta.get(b.start..b.end).unwrap_or_default(), nl);
                entries.push((kind, b.name.clone(), bytes));
            }
        }
    }
    let mut seen: Vec<&[u8]> = Vec::new();
    for (_, name, _) in &entries {
        if seen.contains(&name.as_slice()) {
            errs.push(format!("duplicate requirement '{}' in the delta", lossy(name)));
        } else {
            seen.push(name);
        }
    }
    for (kind, name, bytes) in &entries {
        if *kind == Kind::Removed && !has_reason(bytes) {
            errs.push(format!("REMOVED '{}' has no **Reason**: line", lossy(name)));
        }
    }
    let index = requirements(target);
    let mut names: Vec<&[u8]> = Vec::new();
    for b in &index {
        if names.contains(&b.name.as_slice()) {
            errs.push(format!("the target has duplicate requirement '{}'", lossy(&b.name)));
        } else {
            names.push(&b.name);
        }
    }
    let current = |name: &[u8]| {
        index
            .iter()
            .find(|b| b.name == name)
            .and_then(|b| target.get(b.start..b.end))
    };
    let mut changes = Vec::new();
    for (kind, name, bytes) in &entries {
        let applied = match (kind, current(name)) {
            (Kind::Removed, now) => now.is_some(),
            (Kind::Modified, None) => {
                errs.push(format!("MODIFIED '{}' is not in the target", lossy(name)));
                false
            }
            (Kind::Modified, Some(old)) => core(old) != core(bytes),
            (Kind::Added, None) => true,
            (Kind::Added, Some(old)) if core(old) == core(bytes) => false,
            (Kind::Added, Some(_)) => {
                errs.push(format!("ADDED '{}' already exists in the target with other text", lossy(name)));
                false
            }
            _ => false,
        };
        changes.push(Change { kind: *kind, name: name.clone(), applied });
    }
    if !errs.is_empty() {
        return Err(errs);
    }
    let mut doc = target.to_vec();
    for ((kind, name, bytes), change) in entries.iter().zip(&changes) {
        if change.applied {
            doc = match kind {
                Kind::Removed => remove(&doc, name),
                Kind::Modified => replace(&doc, name, bytes),
                _ => add(&doc, bytes),
            };
        }
    }
    Ok((doc, changes))
}

/// Where a fallback feature's deltas land (P3c-R10): its product's
/// `parent:` resolved from `feature_dir` and normalized, else
/// `<root>/product.md`. Err when it leaves the spec root.
pub fn target(root: &Path, feature_dir: &Path, product: &[u8]) -> Result<PathBuf, String> {
    let parent = crate::frontmatter::parse(product)
        .and_then(|fm| fm.scalar("parent").map(|p| p.trim().to_string()))
        .filter(|p| !p.is_empty());
    let Some(parent) = parent else {
        return Ok(root.join("product.md"));
    };
    let path = lexical(&feature_dir.join(&parent));
    if path.starts_with(lexical(root)) {
        Ok(path)
    } else {
        Err(format!("parent '{parent}' leaves the spec root"))
    }
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p vibe-spec --bin vibe-spec delta:: 2>&1 | tail -5`
Expected: `test result: ok. 13 passed; 0 failed`. A failure in `added_lands_after_trailing_prose_and_lists` or `crlf_target_keeps_crlf` means `add` or `sections` disagrees with P3c-R8. Fix the code, never the expected bytes.

- [ ] **Step 5: Lint and commit**

Run: `cargo fmt --all && cargo clippy --workspace --all-targets -q -- -D warnings && cargo test -p vibe-spec -q 2>&1 | grep -E 'test result|FAILED'`
Expected: no clippy output, every `test result: ok`.

```bash
git add crates/vibe-spec/src/delta.rs crates/vibe-spec/src/main.rs crates/vibe-spec/src/layered.rs
git commit -m "feat(spec): add requirement delta engine"
```

### Task 2: REMOVED requirements in the validators

**Files:**
- Modify: `crates/vibe-spec/src/validate.rs` (the fallback feature loop, the `required == "product.md"` branch near line 360)
- Modify: `crates/vibe-spec/src/content.rs:350` (`id_traceability`)
- Modify: `crates/vibe-parity/src/corpus_spec.rs`, `tests/parity/spec.json` (case `features/delta-removed`, pin, floor 14); regenerate `tests/parity/corpus/spec/`
- Test: `crates/vibe-spec/tests/grammar.rs`

**Interfaces:**
- Consumes: `crate::delta::without_removed(&[u8]) -> Vec<u8>` (Task 1).
- Produces: nothing new. P3c-R11 becomes true: a REMOVED block needs no scenario, no RFC keyword and no plan citation.

- [ ] **Step 1: Write the failing test**

Append to `crates/vibe-spec/tests/grammar.rs`. It reuses that file's `feature`, `SCOPE`, `OPENSPEC_AUTH`, `grammar` and `messages`:

```rust
#[test]
fn removed_requirements_need_no_scenario_or_citation() {
    let p = feature(
        "gr-removed",
        &format!(
            "{SCOPE}## Requirements\n\n{OPENSPEC_AUTH}\n## REMOVED Requirements\n\n### Requirement: Legacy login (R9)\n**Reason**: replaced by passkeys.\n"
        ),
    );
    write(
        &p,
        ".spec/features/auth/plan.md",
        "---\ntype: feature-plan\nfeature: auth\nparent: ../../plan.md\nupdated: 2026-01-15\n---\n\n## Requirements Trace\n\nNone.\n\n### auth/1 — a\n",
    );
    assert_eq!(grammar(&p), Vec::<String>::new());
    assert_eq!(messages(&p, "SF12"), Vec::<String>::new());
}
```

- [ ] **Step 2: Run it to verify it fails**

Run: `cargo test -p vibe-spec --test grammar removed_requirements 2>&1 | tail -15`
Expected: FAIL. `grammar(&p)` lists `SF10 features/auth/product.md: requirement missing RFC-2119 keyword … -> Legacy login (R9)` and `… zero #### Scenario: blocks …`.

- [ ] **Step 3: Implement**

In `validate.rs`, inside the required-docs loop, run the requirement checks on the stripped text of a product. Replace:

```rust
        if required == "product.md" {
            content::feature_scope(r, &text, doc.as_bytes());
            content::requirement_population(r, &text, doc.as_bytes());
        }
        content::requirements_scenarios(r, &text, doc.as_bytes());
        content::scenario_grammar(r, &text, doc.as_bytes());
```

with:

```rust
        // P3c-R11: REMOVED blocks are deletions, not requirements.
        let reqs = if required == "product.md" {
            crate::delta::without_removed(&text)
        } else {
            text.clone()
        };
        if required == "product.md" {
            content::feature_scope(r, &text, doc.as_bytes());
            content::requirement_population(r, &reqs, doc.as_bytes());
        }
        content::requirements_scenarios(r, &reqs, doc.as_bytes());
        content::scenario_grammar(r, &reqs, doc.as_bytes());
```

In `content.rs` `id_traceability`, replace:

```rust
    let ids = product_r_ids(&std::fs::read(product).unwrap_or_default());
```

with:

```rust
    let ids = product_r_ids(&crate::delta::without_removed(
        &std::fs::read(product).unwrap_or_default(),
    ));
```

- [ ] **Step 4: Run the tests and parity**

Run: `cargo test -p vibe-spec -q 2>&1 | grep -E 'test result|FAILED'`
Expected: every `test result: ok`.

Pin the divergence (P3c-R19). In `crates/vibe-parity/src/corpus_spec.rs`, add two constants beside `AUTH_PRODUCT`:

```rust
/// A fallback product whose only requirement section is REMOVED (P3c-R11).
const DELTA_PRODUCT: &str = "---\ntype: feature-product\nfeature: auth\nparent: ../../product-auth.md\nsibling: tech.md\nupdated: 2026-01-15\n---\n\n# auth — Product\n\n## REMOVED Requirements\n\n### Requirement: Legacy login (R9)\n**Reason**: replaced by passkeys.\n";
/// Its `parent:` target, holding the requirement the delta removes.
const DELTA_TOPIC: &str = "---\ntype: product-topic\nparent: product.md\nscope: auth\ncovers: sign-in\nupdated: 2026-01-15\n---\n\n# auth — Product\n\n## Requirements\n\n### Requirement: Legacy login (R9)\nThe system SHALL accept passwords.\n\n#### Scenario: password\n- **WHEN** a password is sent\n- **THEN** a session starts\n";
```

Add a `features` case after `closed-blocks`:

```rust
        CaseDef {
            class: "features",
            name: "delta-removed",
            cwd: "",
            runs: &[],
            files: feature(
                FEATURE_TECH,
                &[
                    (".spec/features/auth/product.md", DELTA_PRODUCT),
                    (".spec/product-auth.md", DELTA_TOPIC),
                ],
            ),
        },
```

In `tests/parity/spec.json`, raise `"features"` in `floors` to `14` and append this pin:

```json
    { "command": "validate", "case": "features/delta-removed", "args": [], "field": "stdout", "reason": "REMOVED blocks are deletions: the port exempts them from SF10 and SF12 (P3c-R11); the oracle reads them as requirements" }
```

Regenerate: `cargo run --locked -q -p vibe-parity -- gen-spec-corpus oracles/bash/spec/reference/templates tests/parity/corpus/spec`

Run: `grep -rlE '^## +(ADDED|MODIFIED|REMOVED|RENAMED) +Requirements' tests/parity/corpus`
Expected: exactly `tests/parity/corpus/spec/features/delta-removed/tree/.spec/features/auth/product.md`.

Run (alone): `bash tests/parity/run.sh 2>&1 | tail -4`
Expected: `0 failed`, one more `pinned` than before, no `stale pin`, and `live (4) green`. A stale pin means the oracle and the port agree on this case: read both outputs before you change anything.

- [ ] **Step 5: Commit**

```bash
git add crates/vibe-spec/src/validate.rs crates/vibe-spec/src/content.rs crates/vibe-spec/tests/grammar.rs crates/vibe-parity/src/corpus_spec.rs tests/parity/
git commit -m "fix(spec): exempt removed requirements from checks"
```

### Task 3: Port `check-drift`

**Files:**
- Create: `crates/vibe-spec/src/drift.rs`
- Modify: `crates/vibe-spec/src/main.rs` (subcommand `CheckDrift`)
- Modify: `crates/vibe-spec/tests/common/mod.rs` (add `staged_oracle`)
- Modify: `tests/parity/spec.json` (check-drift candidate)
- Test: `crates/vibe-spec/tests/drift.rs` (new)

**Interfaces:**
- Consumes: `Root::display()`, `text::{find, lines, name_of, sub_dirs}`, the `cat!` macro from `main.rs`.
- Produces:
  - `pub fn drift::run(found: &Result<Root, Miss>, cwd: &Path) -> (u8, Vec<u8>)`. Task 4 changes the signature to `run(found, cwd, &Options)`.
  - `pub fn common::staged_oracle(project: &Path, script: &str, args: &[&str]) -> Out`.

- [ ] **Step 1: Write the failing tests**

Add to `crates/vibe-spec/tests/common/mod.rs`:

```rust
/// Runs a frozen oracle copied into `<project>/.oracle/scripts/`, with
/// `project` as cwd. check-drift.sh finds its repo from its own path, so a
/// staged copy treats `project` as the repo.
pub fn staged_oracle(project: &Path, script: &str, args: &[&str]) -> Out {
    let dir = project.join(".oracle").join("scripts");
    fs::create_dir_all(&dir).unwrap();
    fs::copy(
        repo().join("oracles/bash/spec/scripts").join(script),
        dir.join(script),
    )
    .unwrap();
    let mut cmd = Command::new("bash");
    cmd.arg(dir.join(script)).args(args);
    run(cmd, project, &[])
}
```

Create `crates/vibe-spec/tests/drift.rs`:

```rust
#![cfg(unix)]
#![allow(clippy::unwrap_used, clippy::expect_used)]
//! `vibe-spec check-drift` (F18): the oracle's three streams, then the
//! superpowers artifact gate and JSON (Task 4).
mod common;

use std::path::Path;

use common::{project, spec, staged_oracle, write};

/// Exit code, stdout and stderr all equal the staged oracle's.
fn same(p: &Path) {
    let want = staged_oracle(p, "check-drift.sh", &[]);
    let got = spec(p, &["check-drift"], &[]);
    assert_eq!(
        (got.code, &got.stdout, &got.stderr),
        (want.code, &want.stdout, &want.stderr),
        "{}",
        p.display()
    );
}

#[test]
fn check_drift_matches_the_oracle_on_every_stream() {
    let cases: Vec<(&str, Vec<(&str, &str)>)> = vec![
        ("dr-clean", vec![(".spec/plan.md", "| auth | IN PROGRESS |\n"), (".spec/features/auth/plan.md", "### auth/1 — a\n")]),
        ("dr-norow", vec![(".spec/plan.md", "# Plan\n"), (".spec/features/billing/tech.md", "x\n")]),
        (
            "dr-done",
            vec![
                (".spec/plan.md", "| auth | **DONE** | js-core DONE |\n| oauth | DONE ✅ |\n"),
                (".spec/features/auth/plan.md", "x\n"),
                (".spec/features/oauth/plan.md", "x\n"),
            ],
        ),
        (
            "dr-bold",
            vec![(".spec/plan.md", "- **auth** — IN PROGRESS | x\n"), (".spec/features/auth/plan.md", "- M1: NOT STARTED\n- M2: NOT STARTED")],
        ),
        ("dr-substr", vec![(".spec/plan.md", "| oauth | DONE |\n"), (".spec/features/auth/tech.md", "x\n")]),
        (
            "dr-counts",
            vec![
                (".spec/plan.md", "x\n"),
                ("README.md", "All 12 passing.\n3 green\n"),
                (".spec/sub/deep.md", "7 assertions\nv2 passed\n"),
                (".spec/archive/old.md", "9 passing\n"),
                (".spec/.hidden.md", "4 passed\n"),
            ],
        ),
        ("dr-noplan", vec![(".spec/features/auth/plan.md", "NOT STARTED\n")]),
    ];
    for (name, files) in cases {
        let p = project(name);
        for (rel, text) in &files {
            write(&p, rel, text);
        }
        same(&p);
    }
    same(&project("dr-bare"));
}
```

- [ ] **Step 2: Run it to verify it fails**

Run: `cargo test -p vibe-spec --test drift 2>&1 | tail -8`
Expected: FAIL. The candidate exits 2 and clap prints `unrecognized subcommand 'check-drift'`.

- [ ] **Step 3: Implement**

Create `crates/vibe-spec/src/drift.rs`. Its count walk includes dot files, as the oracle's `find` does:

```rust
//! `vibe-spec check-drift` — port of `spec/scripts/check-drift.sh` (F18,
//! P3c-R3). For a `.spec` root the output is the oracle's, byte for byte:
//! stdout carries only the final OK line, everything else goes to stderr.
//! Paths print relative to the project through `Root::display()`.
use std::io::Write;
use std::path::{Path, PathBuf};

use vibe_core::exit;

use crate::root::{Miss, Root};
use crate::text::{find, lines, name_of, sub_dirs};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Sev {
    Error,
    Warning,
    Note,
}

struct Event {
    sev: Sev,
    msg: Vec<u8>,
    file: Option<String>,
}

fn event(sev: Sev, msg: Vec<u8>, file: Option<String>) -> Event {
    Event { sev, msg, file }
}

/// The oracle's `find_repo_root`, from the cwd: the nearest ancestor (not
/// `/`) holding `.spec/` or `.git`, else the cwd.
fn marker_root(cwd: &Path) -> PathBuf {
    let mut d = Some(cwd);
    while let Some(dir) = d {
        if dir.parent().is_none() {
            break;
        }
        if dir.join(".spec").is_dir() || dir.join(".git").exists() {
            return dir.to_path_buf();
        }
        d = dir.parent();
    }
    cwd.to_path_buf()
}

/// The first line holding `needle` (`grep -F … | head -n1`).
fn first_line_with<'a>(text: &'a [u8], needle: &[u8]) -> Option<&'a [u8]> {
    lines(text).into_iter().find(|l| find(l, needle).is_some())
}

/// The oracle's status-cell scan: split on `|`, keep each cell's ASCII
/// letters, and match the whole cell. The text after the last `|` is never
/// read.
fn row_status(row: &[u8]) -> &'static str {
    let mut status = "NOT STARTED";
    let mut rest = row;
    while let Some(i) = rest.iter().position(|&c| c == b'|') {
        let cell = rest.get(..i).unwrap_or_default();
        rest = rest.get(i + 1..).unwrap_or_default();
        let letters: Vec<u8> = cell.iter().copied().filter(u8::is_ascii_alphabetic).collect();
        match letters.as_slice() {
            b"DONE" => return "DONE",
            b"INPROGRESS" => status = "IN PROGRESS",
            _ => {}
        }
    }
    status
}

/// `grep -E '[0-9]+ (passing|passed|assertions|green)'` on one line.
fn hand_count(line: &[u8]) -> bool {
    line.windows(2).enumerate().any(|(i, w)| {
        w.first().is_some_and(u8::is_ascii_digit)
            && w.get(1) == Some(&b' ')
            && line.get(i + 2..).is_some_and(|rest| {
                [&b"passing"[..], b"passed", b"assertions", b"green"]
                    .iter()
                    .any(|word| rest.starts_with(word))
            })
    })
}

/// `find "$SPEC_DIR" -type f -name '*.md' -not -path "$SPEC_DIR/archive/*"`,
/// as paths relative to the root, sorted. Dot files count; symlinks do not.
fn count_targets(dir: &Path) -> Vec<String> {
    fn walk(dir: &Path, prefix: &str, out: &mut Vec<String>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            let rel = format!("{prefix}{name}");
            let Ok(meta) = std::fs::symlink_metadata(entry.path()) else {
                continue;
            };
            if meta.is_dir() {
                if rel != "archive" {
                    walk(&entry.path(), &format!("{rel}/"), out);
                }
            } else if meta.is_file() && name.ends_with(".md") {
                out.push(rel);
            }
        }
    }
    let mut out = Vec::new();
    walk(dir, "", &mut out);
    out.sort();
    out
}

/// Runs the command.
pub fn run(found: &Result<Root, Miss>, cwd: &Path) -> (u8, Vec<u8>) {
    let root = match found {
        Ok(root) => root,
        Err(Miss::NotFound) => {
            eprintln!(
                "check-drift: no .spec/ under {}; nothing to check.",
                marker_root(cwd).to_string_lossy()
            );
            return (exit::OK, Vec::new());
        }
        Err(miss) => {
            eprintln!("{}", miss.message());
            return (miss.code(), Vec::new());
        }
    };
    let d = root.display();
    let plan_path = root.dir.join("plan.md");
    let plan = plan_path
        .is_file()
        .then(|| std::fs::read(&plan_path).unwrap_or_default());
    let features: Vec<PathBuf> = sub_dirs(&root.dir.join("features"));
    let mut ev: Vec<Event> = Vec::new();

    // (a) every live feature folder must be named in the root plan.
    match &plan {
        Some(text) => {
            for dir in &features {
                let feat = name_of(dir);
                if find(text, feat.as_bytes()).is_none() {
                    ev.push(event(Sev::Error, cat!("feature '", feat, "' has no row in ", d, "/plan.md — a merged feature must be compounded into the root plan"), None));
                }
            }
        }
        None => ev.push(event(Sev::Warning, cat!("root plan ", d, "/plan.md not found; skipping plan-row check."), None)),
    }

    // (d) status of each live feature's row in the root plan.
    if let Some(text) = &plan {
        for dir in &features {
            let feat = name_of(dir);
            let row = first_line_with(text, format!("| {feat} |").as_bytes())
                .or_else(|| first_line_with(text, format!("**{feat}**").as_bytes()));
            let Some(row) = row else {
                ev.push(event(Sev::Note, cat!("check-drift: note: feature '", feat, "' has no Feature Sequence row in ", d, "/plan.md."), None));
                continue;
            };
            let status = row_status(row);
            ev.push(event(Sev::Note, cat!("check-drift: live feature '", feat, "' — root plan says: ", status), None));
            if status == "DONE" {
                ev.push(event(Sev::Error, cat!("feature '", feat, "' is DONE in ", d, "/plan.md but ", d, "/features/", feat, "/ still exists — compound promotes AND archives; move it to ", d, "/archive/"), None));
            }
        }
    }

    // (b) NOT STARTED units left in a live feature plan.
    for dir in &features {
        let path = dir.join("plan.md");
        if !path.is_file() {
            continue;
        }
        let rel = format!("{d}/features/{}/plan.md", name_of(dir));
        let text = std::fs::read(&path).unwrap_or_default();
        for (i, line) in lines(&text).into_iter().enumerate() {
            if find(line, b"NOT STARTED").is_some() {
                ev.push(event(Sev::Warning, cat!("NOT STARTED unit in ", rel, ":", (i + 1).to_string(), ":", line), Some(rel.clone())));
            }
        }
    }

    // (c) hand-written test counts in README.md or the tree (archive excluded).
    let mut targets: Vec<(String, PathBuf)> = Vec::new();
    if root.project.join("README.md").is_file() {
        targets.push(("README.md".to_string(), root.project.join("README.md")));
    }
    for rel in count_targets(&root.dir) {
        let path = root.dir.join(&rel);
        targets.push((format!("{d}/{rel}"), path));
    }
    for (shown, path) in &targets {
        let text = std::fs::read(path).unwrap_or_default();
        for (i, line) in lines(&text).into_iter().enumerate() {
            if hand_count(line) {
                ev.push(event(Sev::Error, cat!("hand-written test count in ", shown, ":", (i + 1).to_string(), ":", line), Some(shown.clone())));
            }
        }
    }

    let errors = ev.iter().filter(|e| e.sev == Sev::Error).count();
    let warnings = ev.iter().filter(|e| e.sev == Sev::Warning).count();
    let mut err = std::io::stderr().lock();
    for e in &ev {
        let line = match e.sev {
            Sev::Error => cat!("check-drift: ERROR: ", e.msg, "\n"),
            Sev::Warning => cat!("check-drift: WARN: ", e.msg, "\n"),
            Sev::Note => cat!(e.msg, "\n"),
        };
        let _ = err.write_all(&line);
    }
    if errors > 0 {
        let _ = writeln!(err, "check-drift: FAIL — {errors} error(s), {warnings} warning(s).");
        return (exit::FINDINGS, Vec::new());
    }
    (
        exit::OK,
        format!("check-drift: OK — 0 errors, {warnings} warning(s).\n").into_bytes(),
    )
}
```

`Event::file` is unused until Task 4. Mark the field `#[allow(dead_code)] // Task 4 reads it for --json.` so clippy stays clean.

In `main.rs`, add `mod drift;` after `mod delta;`, a subcommand and its dispatch:

```rust
    /// Compound drift gate (port of check-drift.sh).
    CheckDrift,
```

```rust
        Command::CheckDrift => drift::run(&found, &cwd),
```

In `tests/parity/spec.json`, give check-drift its candidate:

```json
    { "name": "check-drift", "oracle": ["bash", "{oracle}/scripts/check-drift.sh"], "candidate": ["{candidate}", "check-drift"], "classes": ["no-root", "seeded", "filled", "features", "malformed"] },
```

- [ ] **Step 4: Run the tests and parity**

Run: `cargo test -p vibe-spec --test drift 2>&1 | tail -4`
Expected: `test result: ok. 1 passed`. If a stream differs, the assertion prints both. Fix the port; the oracle is the truth.

Run (alone): `bash tests/parity/run.sh 2>&1 | tail -4`
Expected: the candidate line has `0 failed`, and `skipped` drops by the check-drift runs.

- [ ] **Step 5: Lint and commit**

Run: `cargo fmt --all && cargo clippy --workspace --all-targets -q -- -D warnings && cargo test -p vibe-spec -q 2>&1 | grep -E 'test result|FAILED'`

```bash
git add crates/vibe-spec/src/drift.rs crates/vibe-spec/src/main.rs crates/vibe-spec/tests/common/mod.rs crates/vibe-spec/tests/drift.rs tests/parity/spec.json
git commit -m "feat(spec): port check-drift to vibe-spec"
```

### Task 4: Superpowers artifact gate and `check-drift --json`

**Files:**
- Modify: `crates/vibe-spec/src/drift.rs`
- Modify: `crates/vibe-spec/src/text.rs` (add `walk_files`)
- Modify: `crates/vibe-spec/src/main.rs` (`CheckDrift { pre_merge, json }`)
- Modify: `crates/vibe-parity/src/corpus_spec.rs`, `tests/parity/spec.json` (case `features/superpowers-artifact`, pin, floor 15); regenerate `tests/parity/corpus/spec/`
- Modify: `contracts/spec-json.schema.json`, `contracts/run.mjs`
- Create: `contracts/fixtures/spec-json/check-drift/valid/{clean,findings,unknown-field}.json`, `contracts/fixtures/spec-json/check-drift/invalid/{missing-pre-merge,bad-severity}.json`
- Test: `crates/vibe-spec/tests/drift.rs`

**Interfaces:**
- Consumes: `common::assert_spec_json(def, json)`.
- Produces:
  - `pub fn text::walk_files(dir: &Path) -> Vec<String>`: regular files under `dir`, recursive, `/`-joined relative paths, dot names skipped, symlinks neither followed nor listed, sorted by bytes. Tasks 8 and 9 use it too.
  - `pub struct drift::Options { pub pre_merge: bool, pub json: bool }`.
  - `pub fn drift::run(found: &Result<Root, Miss>, cwd: &Path, opts: &Options) -> (u8, Vec<u8>)`.
  - Spec JSON def `check-drift`: required `v`, `root`, `pre_merge`, `errors`, `warnings`, `findings`; optional `features: [{name, status}]` with status ∈ `NOT STARTED|IN PROGRESS|DONE|NO ROW`; findings `{severity: warning|error, message, file?}`.

- [ ] **Step 1: Write the failing tests**

Append to `crates/vibe-spec/tests/drift.rs` (also import `common::assert_spec_json`):

```rust
const NEXT: &str = " — promote its facts, then run vibe-spec promote <feature> --done";

#[test]
fn superpowers_artifacts_warn_and_fail_pre_merge() {
    let p = project("dr-sp");
    write(&p, ".spec/plan.md", "x\n");
    write(&p, "docs/superpowers/plans/2026-01-01-auth.md", "x\n");
    write(&p, "docs/superpowers/specs/deep/2026-01-01-auth-design.md", "x\n");
    write(&p, "docs/superpowers/.DS_Store", "x");
    let plain = spec(&p, &["check-drift"], &[]);
    assert_eq!(plain.code, 0, "{}", plain.stderr);
    assert_eq!(plain.text(), "check-drift: OK — 0 errors, 2 warning(s).\n");
    assert_eq!(
        plain.stderr,
        format!(
            "check-drift: WARN: superpowers artifact remains: docs/superpowers/plans/2026-01-01-auth.md{NEXT}\n\
             check-drift: WARN: superpowers artifact remains: docs/superpowers/specs/deep/2026-01-01-auth-design.md{NEXT}\n"
        )
    );
    let gate = spec(&p, &["check-drift", "--pre-merge"], &[]);
    assert_eq!(gate.code, 1);
    assert!(gate.text().is_empty());
    assert!(
        gate.stderr.ends_with("check-drift: FAIL — 2 error(s), 0 warning(s).\n"),
        "{}",
        gate.stderr
    );
}

#[test]
fn json_validates_and_exits_like_text() {
    let p = project("dr-json");
    write(&p, ".spec/plan.md", "| auth | DONE |\n");
    write(&p, ".spec/features/auth/plan.md", "NOT STARTED\n");
    let out = spec(&p, &["check-drift", "--json"], &[]);
    assert_eq!(out.code, 1);
    assert!(out.stderr.is_empty(), "{}", out.stderr);
    assert_spec_json("check-drift", &out.text());
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!((v["errors"].as_u64(), v["warnings"].as_u64()), (Some(1), Some(1)));
    assert_eq!(v["pre_merge"], false);
    assert_eq!(v["features"], serde_json::json!([{ "name": "auth", "status": "DONE" }]));
    assert_eq!(v["findings"][1]["file"], ".spec/features/auth/plan.md");
    let none = project("dr-json-none");
    let miss = spec(&none, &["check-drift", "--json"], &[]);
    assert_eq!((miss.code, miss.stdout.is_empty()), (3, true));
}
```

- [ ] **Step 2: Run them to verify they fail**

Run: `cargo test -p vibe-spec --test drift 2>&1 | tail -8`
Expected: the two new tests FAIL. clap rejects `--pre-merge` and `--json` (exit 2), and the plain run reports `0 warning(s)`.

- [ ] **Step 3: Implement**

Add to `crates/vibe-spec/src/text.rs`:

```rust
/// Regular files under `dir`, recursive, as `/`-joined paths relative to
/// `dir`, sorted by bytes. Dot names are skipped; symlinks are not followed
/// and not listed.
pub fn walk_files(dir: &Path) -> Vec<String> {
    fn walk(dir: &Path, prefix: &str, out: &mut Vec<String>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.starts_with('.') {
                continue;
            }
            let Ok(meta) = std::fs::symlink_metadata(entry.path()) else {
                continue;
            };
            let rel = format!("{prefix}{name}");
            if meta.is_dir() {
                walk(&entry.path(), &format!("{rel}/"), out);
            } else if meta.is_file() {
                out.push(rel);
            }
        }
    }
    let mut out = Vec::new();
    walk(dir, "", &mut out);
    out.sort();
    out
}
```

In `drift.rs`, remove the `#[allow(dead_code)]` on `Event::file` and add:

```rust
use serde_json::{Value, json};

use crate::text::{lossy, walk_files};

/// Command options.
pub struct Options {
    /// Superpowers artifacts are errors, not warnings (D46).
    pub pre_merge: bool,
    /// Print Spec JSON v1 `check-drift` instead of text.
    pub json: bool,
}
```

Change the signature to `pub fn run(found: &Result<Root, Miss>, cwd: &Path, opts: &Options) -> (u8, Vec<u8>)`. With `--json` and no root, exit 3 (P3c-R5). Replace the first match arm with:

```rust
        Err(Miss::NotFound) if !opts.json => {
```

so that `NotFound` under `--json` falls through to `Err(miss)`, which prints `miss.message()` and returns `miss.code()` (3).

Record each feature's status while running check (d). Declare `let mut statuses: Vec<(String, &'static str)> = Vec::new();` beside `ev`. Push `(feat.clone(), "NO ROW")` in the no-row branch and `(feat.clone(), status)` after computing `status`.

After check (c), add the artifact gate:

```rust
    // D46 (P3c-R4): superpowers artifacts left in the tree.
    for rel in walk_files(&root.project.join("docs").join("superpowers")) {
        let shown = format!("docs/superpowers/{rel}");
        let sev = if opts.pre_merge { Sev::Error } else { Sev::Warning };
        ev.push(event(sev, cat!("superpowers artifact remains: ", shown, " — promote its facts, then run vibe-spec promote <feature> --done"), Some(shown.clone())));
    }
```

Before the stderr loop, add the JSON branch:

```rust
    if opts.json {
        let findings: Vec<Value> = ev
            .iter()
            .filter(|e| e.sev != Sev::Note)
            .map(|e| {
                let mut f = json!({
                    "severity": if e.sev == Sev::Error { "error" } else { "warning" },
                    "message": lossy(&e.msg),
                });
                if let Some(file) = &e.file {
                    f["file"] = json!(file);
                }
                f
            })
            .collect();
        let v = json!({
            "v": 1,
            "root": d,
            "pre_merge": opts.pre_merge,
            "errors": errors,
            "warnings": warnings,
            "features": statuses.iter().map(|(n, s)| json!({ "name": n, "status": s })).collect::<Vec<_>>(),
            "findings": findings,
        });
        let code = if errors > 0 { exit::FINDINGS } else { exit::OK };
        return (code, format!("{v}\n").into_bytes());
    }
```

In `main.rs`:

```rust
    /// Compound drift gate (port of check-drift.sh); `--pre-merge` also fails
    /// on superpowers artifacts left under docs/superpowers/.
    CheckDrift {
        /// Superpowers artifacts are errors (run before merging).
        #[arg(long)]
        pre_merge: bool,
        /// Print Spec JSON v1.
        #[arg(long)]
        json: bool,
    },
```

```rust
        Command::CheckDrift { pre_merge, json } => {
            drift::run(&found, &cwd, &drift::Options { pre_merge, json })
        }
```

Contract. Add to `$defs` in `contracts/spec-json.schema.json`:

```json
    "check-drift": {
      "type": "object",
      "required": ["v", "root", "pre_merge", "errors", "warnings", "findings"],
      "properties": {
        "v": { "const": 1 },
        "root": { "type": "string", "minLength": 1 },
        "pre_merge": { "type": "boolean" },
        "errors": { "type": "integer", "minimum": 0 },
        "warnings": { "type": "integer", "minimum": 0 },
        "features": { "type": "array", "items": { "$ref": "#/$defs/drift-feature" } },
        "findings": { "type": "array", "items": { "$ref": "#/$defs/drift-finding" } }
      }
    },
    "drift-feature": {
      "type": "object",
      "required": ["name", "status"],
      "properties": {
        "name": { "type": "string", "minLength": 1 },
        "status": { "enum": ["NOT STARTED", "IN PROGRESS", "DONE", "NO ROW"] }
      }
    },
    "drift-finding": {
      "type": "object",
      "required": ["severity", "message"],
      "properties": {
        "severity": { "enum": ["warning", "error"] },
        "message": { "type": "string" },
        "file": { "type": "string", "minLength": 1 }
      }
    },
```

In `contracts/run.mjs`, add `'check-drift'` to `QUERIES`, and add to `REQUIRED`:

```js
  'check-drift': ['errors', 'findings', 'pre_merge', 'root', 'v', 'warnings'],
  'drift-feature': ['name', 'status'],
  'drift-finding': ['message', 'severity'],
```

Goldens:
- `valid/clean.json`: `{"v":1,"root":".spec","pre_merge":false,"errors":0,"warnings":0,"features":[],"findings":[]}`
- `valid/findings.json`: `{"v":1,"root":"docs/spec","pre_merge":true,"errors":1,"warnings":1,"features":[{"name":"auth","status":"NO ROW"}],"findings":[{"severity":"error","message":"superpowers artifact remains: docs/superpowers/plans/2026-01-01-auth.md","file":"docs/superpowers/plans/2026-01-01-auth.md"},{"severity":"warning","message":"NOT STARTED unit in docs/spec/features/auth/plan.md:3:x"}]}`
- `valid/unknown-field.json`: `clean.json` plus `"took_ms":3`
- `invalid/missing-pre-merge.json`: `clean.json` without `pre_merge`
- `invalid/bad-severity.json`: `clean.json` with `"findings":[{"severity":"info","message":"x"}]`

Also update the schema's top-level `description` to list `check-drift`. The JSON queries are no longer all read-only: name `check-drift` here, and Tasks 7 and 8 add `promote` and `openspec`.

- [ ] **Step 4: Run the tests, contracts and parity**

Run: `cargo test -p vibe-spec --test drift 2>&1 | tail -4 && bash contracts/run.sh 2>&1 | tail -2`
Expected: `3 passed`; contracts `OK`, with the query count up by one.

Pin the divergence (P3c-R19). In `crates/vibe-parity/src/corpus_spec.rs`, add a `features` case after `closed-blocks`:

```rust
        CaseDef {
            class: "features",
            name: "superpowers-artifact",
            cwd: "",
            runs: &[],
            files: with(filled(), &[("docs/superpowers/plans/2026-01-14-auth.md", "# auth plan\n")]),
        },
```

In `tests/parity/spec.json`, raise `"features"` in `floors` to `15` and append this pin:

```json
    { "command": "check-drift", "case": "features/superpowers-artifact", "args": [], "field": "stdout", "reason": "the port warns on superpowers artifacts left under docs/superpowers/ (D46, P3c-R4); the oracle does not look there" }
```

Regenerate: `cargo run --locked -q -p vibe-parity -- gen-spec-corpus oracles/bash/spec/reference/templates tests/parity/corpus/spec`

Run: `find tests/parity/corpus -path '*docs/superpowers*' -type f`
Expected: exactly `tests/parity/corpus/spec/features/superpowers-artifact/tree/docs/superpowers/plans/2026-01-14-auth.md`.

Run (alone): `bash tests/parity/run.sh 2>&1 | tail -4`
Expected: `0 failed`, one more `pinned` than before, and no `stale pin`.

- [ ] **Step 5: Lint and commit**

Run: `cargo fmt --all && cargo clippy --workspace --all-targets -q -- -D warnings`

```bash
git add crates/vibe-spec/src/drift.rs crates/vibe-spec/src/text.rs crates/vibe-spec/src/main.rs crates/vibe-spec/tests/drift.rs contracts/ crates/vibe-parity/src/corpus_spec.rs tests/parity/
git commit -m "feat(spec): gate superpowers artifacts in drift"
```

### Task 5: Port `promote` (merge blocks)

**Files:**
- Create: `crates/vibe-spec/src/promote.rs`
- Modify: `crates/vibe-spec/src/main.rs` (subcommand `Promote`)
- Modify: `tests/parity/spec.json` (promote candidate)
- Test: `crates/vibe-spec/tests/promote.rs` (new)

**Interfaces:**
- Consumes: `text::{find, join_raw, read_lines}`, `vibe_core::fs::write_atomic(&Path, &[u8]) -> vibe_core::Result<()>`, `common::staged_oracle` (Task 3), `main.rs`'s `raw_tail`.
- Produces: `pub fn promote::run(found: &Result<Root, Miss>, cwd: &Path, args: &[String]) -> (u8, Vec<u8>)`. Tasks 6 and 7 extend it in place.

- [ ] **Step 1: Write the failing test**

Create `crates/vibe-spec/tests/promote.rs`:

```rust
#![cfg(unix)]
#![allow(clippy::unwrap_used, clippy::expect_used)]
//! `vibe-spec promote` (F18): the oracle's streams and files for merge
//! blocks, then requirement deltas (Task 6) and superpowers (Task 7).
mod common;

use std::fs;
use std::path::PathBuf;

use common::{project, spec, staged_oracle, write};

const TECH: &str = "---\ntype: feature-tech\n---\n\n<!-- merge -->\n## Auth\n\nSessions live in signed cookies.\n<!-- /merge -->\n\n<!-- merge -->\nTokens expire after 1 hour.\n<!-- /merge -->\n";
const PRESENT: &str = "# Tech\n\n## Auth\n\nSessions live in signed cookies.\n\nTokens expire after 1 hour.\n";

fn tree(name: &str, tech: &str, target: &str) -> PathBuf {
    let p = project(name);
    write(&p, ".spec/features/auth/tech.md", tech);
    write(&p, ".spec/tech.md", target);
    p
}

/// Twin trees: candidate and staged oracle give the same exit code, stdout,
/// stderr (when `stderr_too`) and resulting target bytes.
fn same(name: &str, tech: &str, target: &str, args: &[&str], stderr_too: bool) {
    let a = tree(&format!("{name}-o"), tech, target);
    let b = tree(&format!("{name}-c"), tech, target);
    let want = staged_oracle(&a, "promote.sh", args);
    let got = spec(&b, &[&["promote"], args].concat(), &[]);
    assert_eq!((got.code, &got.stdout), (want.code, &want.stdout), "{name} {args:?}");
    if stderr_too {
        assert_eq!(got.stderr, want.stderr, "{name} {args:?}");
    }
    assert_eq!(
        fs::read(b.join(".spec/tech.md")).unwrap(),
        fs::read(a.join(".spec/tech.md")).unwrap(),
        "{name} {args:?}"
    );
}

#[test]
fn promote_matches_the_oracle_on_every_stream() {
    let cases: Vec<(&str, &str, &str, Vec<&str>, bool)> = vec![
        ("pr-dry", TECH, "# Tech\n", vec!["auth", "--dry-run"], true),
        ("pr-real", TECH, "# Tech\n", vec!["auth"], true),
        ("pr-again", TECH, PRESENT, vec!["auth"], true),
        ("pr-again-dry", TECH, PRESENT, vec!["auth", "--dry-run"], true),
        ("pr-missing", TECH, "x\n", vec!["missing"], true),
        ("pr-flag-first", TECH, "x\n", vec!["--dry-run"], true),
        ("pr-target-missing", TECH, "x\n", vec!["auth", "--target", "nope.md"], true),
        ("pr-target", TECH, "x\n", vec!["auth", "--target", ".spec/tech.md", "--dry-run"], true),
        ("pr-nested", "<!-- merge -->\na\n<!-- merge -->\n", "x\n", vec!["auth"], true),
        ("pr-orphan", "<!-- /merge -->\n## bad\n<!-- merge -->\n", "x\n", vec!["auth"], true),
        ("pr-unclosed-eof", "<!-- merge -->\na\n<!-- /merge -->", "x\n", vec!["auth"], true),
        ("pr-none", "no blocks\n", "x\n", vec!["auth"], true),
        ("pr-empty-block", "<!-- merge -->\n<!-- /merge -->\n", "x\n", vec!["auth"], true),
        ("pr-unknown", TECH, "x\n", vec!["auth", "--bogus"], true),
        // bash's own `${2:?}` / `${1:?}` messages name the script: code only.
        ("pr-target-nopath", TECH, "x\n", vec!["auth", "--target"], false),
        ("pr-noargs", TECH, "x\n", vec![], false),
    ];
    for (name, tech, target, args, stderr_too) in cases {
        same(name, tech, target, &args, stderr_too);
    }
}
```

- [ ] **Step 2: Run it to verify it fails**

Run: `cargo test -p vibe-spec --test promote 2>&1 | tail -6`
Expected: FAIL. `unrecognized subcommand 'promote'` (exit 2 against the oracle's 0).

- [ ] **Step 3: Implement**

Create `crates/vibe-spec/src/promote.rs`:

```rust
//! `vibe-spec promote <feature> [--dry-run] [--target <file>]` — port of
//! `spec/scripts/promote.sh` (F18, P3c-R6): `<!-- merge -->` blocks of a
//! fallback feature's tech.md are appended to the root tech doc. Byte
//! parity: messages, the `while read` loop (an unterminated last line is
//! dropped), whole-line markers, abort on the first marker error, empty
//! blocks always present, argument errors exit 1.
use std::path::{Path, PathBuf};

use vibe_core::exit;

use crate::root::{Miss, Root};
use crate::text::{find, join_raw, read_lines};

const USAGE: &str = "usage: vibe-spec promote <feature-name> [--dry-run] [--target <file>]";

struct Args {
    feature: String,
    dry_run: bool,
    target: Option<String>,
}

fn parse(args: &[String]) -> Result<Args, String> {
    let feature = args
        .first()
        .filter(|f| !f.is_empty())
        .ok_or_else(|| USAGE.to_string())?
        .clone();
    let mut a = Args { feature, dry_run: false, target: None };
    let mut i = 1;
    while let Some(arg) = args.get(i) {
        match arg.as_str() {
            "--dry-run" => a.dry_run = true,
            "--target" => {
                let t = args
                    .get(i + 1)
                    .filter(|t| !t.is_empty())
                    .ok_or_else(|| "promote: --target requires a path".to_string())?;
                a.target = Some(t.clone());
                i += 1;
            }
            other => return Err(format!("Unknown option: {other}")),
        }
        i += 1;
    }
    Ok(a)
}

/// The merge blocks of a feature tech doc, read the oracle's way.
fn merge_blocks(text: &[u8], show: &str) -> Result<Vec<Vec<u8>>, String> {
    let mut blocks = Vec::new();
    let mut open: Option<Vec<u8>> = None;
    for (i, line) in read_lines(text).into_iter().enumerate() {
        let n = i + 1;
        if line == b"<!-- merge -->" {
            if open.is_some() {
                return Err(format!("ERROR: nested <!-- merge --> at line {n} in {show}"));
            }
            open = Some(Vec::new());
        } else if line == b"<!-- /merge -->" {
            match open.take() {
                Some(b) => blocks.push(b),
                None => {
                    return Err(format!("ERROR: <!-- /merge --> without opener at line {n} in {show}"));
                }
            }
        } else if let Some(b) = open.as_mut() {
            b.extend_from_slice(line);
            b.push(b'\n');
        }
    }
    if open.is_some() {
        return Err(format!("ERROR: unclosed <!-- merge --> block in {show}"));
    }
    Ok(blocks)
}

struct MergePlan {
    append: Vec<u8>,
    appended: usize,
    skipped: usize,
}

/// The oracle's idempotency: a block already in the target (or empty) is
/// skipped; each new one is appended after a newline.
fn merge_plan(target: &[u8], blocks: &[Vec<u8>]) -> MergePlan {
    let mut content = target.to_vec();
    let mut p = MergePlan { append: Vec::new(), appended: 0, skipped: 0 };
    for b in blocks {
        if b.is_empty() || find(&content, b).is_some() {
            p.skipped += 1;
            continue;
        }
        p.append.push(b'\n');
        p.append.extend_from_slice(b);
        content.push(b'\n');
        content.extend_from_slice(b);
        p.appended += 1;
    }
    p
}

fn fail(msg: &str) -> (u8, Vec<u8>) {
    eprintln!("{msg}");
    (exit::FINDINGS, Vec::new())
}

/// Runs the command.
pub fn run(found: &Result<Root, Miss>, cwd: &Path, args: &[String]) -> (u8, Vec<u8>) {
    let a = match parse(args) {
        Ok(a) => a,
        Err(msg) => return fail(&msg),
    };
    // The oracle's paths are cwd-relative under `.spec/`; the port uses the
    // discovered root and prints it relative to the project.
    let (dir, display): (PathBuf, String) = match found {
        Ok(root) => (root.dir.clone(), root.display()),
        Err(Miss::NotFound) => (cwd.join(".spec"), ".spec".to_string()),
        Err(miss) => {
            eprintln!("{}", miss.message());
            return (miss.code(), Vec::new());
        }
    };
    let tech = join_raw(&dir.join("features"), a.feature.as_bytes()).join("tech.md");
    let tech_show = format!("{display}/features/{}/tech.md", a.feature);
    let (target, target_show) = match &a.target {
        Some(t) => (cwd.join(t), t.clone()),
        None => (dir.join("tech.md"), format!("{display}/tech.md")),
    };
    if !tech.is_file() {
        return fail(&format!("ERROR: {tech_show} not found"));
    }
    if !target.is_file() {
        return fail(&format!("ERROR: {target_show} not found"));
    }
    let blocks = match merge_blocks(&std::fs::read(&tech).unwrap_or_default(), &tech_show) {
        Ok(b) => b,
        Err(msg) => return fail(&msg),
    };
    if blocks.is_empty() {
        eprintln!("WARN: no <!-- merge --> blocks found in {tech_show}");
        return (exit::OK, Vec::new());
    }
    let current = std::fs::read(&target).unwrap_or_default();
    let plan = merge_plan(&current, &blocks);
    if a.dry_run {
        if plan.appended == 0 {
            let line = format!("--- nothing to promote: all {} block(s) already present in {target_show} ---\n", blocks.len());
            return (exit::OK, line.into_bytes());
        }
        let mut out = format!("--- would promote {} block(s) to {target_show} ({} already present) ---\n", plan.appended, plan.skipped).into_bytes();
        out.extend_from_slice(&plan.append);
        return (exit::OK, out);
    }
    if plan.appended == 0 {
        let line = format!("No new blocks to promote — all {} already present in {target_show}\n", blocks.len());
        return (exit::OK, line.into_bytes());
    }
    let mut bytes = current;
    bytes.extend_from_slice(&plan.append);
    if let Err(e) = vibe_core::fs::write_atomic(&target, &bytes) {
        return fail(&format!("ERROR: {target_show}: {e}"));
    }
    let line = format!("PROMOTED {} block(s) from {tech_show} → {target_show} ({} already present)\n", plan.appended, plan.skipped);
    (exit::OK, line.into_bytes())
}
```

In `main.rs`, add `mod promote;` and:

```rust
    /// Promote a feature (port of promote.sh; requirement deltas; superpowers).
    #[command(disable_help_flag = true)]
    Promote {
        /// `<feature> [--dry-run] [--target <file>]`.
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
```

```rust
        Command::Promote { args: _ } => promote::run(&found, &cwd, &raw_tail(std::env::args_os())),
```

In `tests/parity/spec.json`, set the promote candidate to `["{candidate}", "promote"]`.

- [ ] **Step 4: Run the tests and parity**

Run: `cargo test -p vibe-spec --test promote 2>&1 | tail -4`
Expected: `test result: ok. 1 passed`.

Run (alone): `bash tests/parity/run.sh 2>&1 | tail -4`
Expected: `0 failed`. The corpus `features/closed-blocks` and `features/unclosed-block` promote runs now compare files too.

- [ ] **Step 5: Lint and commit**

Run: `cargo fmt --all && cargo clippy --workspace --all-targets -q -- -D warnings`

```bash
git add crates/vibe-spec/src/promote.rs crates/vibe-spec/src/main.rs crates/vibe-spec/tests/promote.rs tests/parity/spec.json
git commit -m "feat(spec): port promote to vibe-spec"
```

### Task 6: Requirement deltas in `promote`

**Files:**
- Modify: `crates/vibe-spec/src/promote.rs`
- Modify: `crates/vibe-parity/src/corpus_spec.rs`, `tests/parity/spec.json` (promote runs and pins on `features/delta-removed`); regenerate `tests/parity/corpus/spec/`
- Test: `crates/vibe-spec/tests/promote.rs`

**Interfaces:**
- Consumes: `delta::{has_delta, apply, target, Kind, Change}` (Task 1), `layered::lexical`.
- Produces: `promote <f>` also applies the feature product's delta sections to its target document (P3c-R9, R10). The legacy output is unchanged when the product has no delta section.

- [ ] **Step 1: Write the failing tests**

Append to `crates/vibe-spec/tests/promote.rs`:

```rust
const TOPIC: &str = "# auth — Product\n\n## Requirements\n\n### Requirement: Sign in (R1)\nThe system SHALL sign users in.\n\n#### Scenario: ok\n- **WHEN** valid\n- **THEN** in\n\n### Requirement: Sign out (R2)\nThe system SHALL sign users out.\n\n#### Scenario: out\n- **WHEN** asked\n- **THEN** out\n";
const DELTA: &str = "---\ntype: feature-product\nfeature: auth\nparent: ../../product-auth.md\n---\n\n# Feature: Auth — Product\n\n## MODIFIED Requirements\n\n### Requirement: Sign in (R1)\nThe system SHALL sign users in with a passkey.\n\n#### Scenario: ok\n- **WHEN** valid\n- **THEN** in\n\n## REMOVED Requirements\n\n### Requirement: Sign out (R2)\n**Reason**: sessions expire.\n\n## ADDED Requirements\n\n### Requirement: Enrol (R3)\nThe system MUST enrol passkeys.\n\n#### Scenario: enrol\n- **WHEN** added\n- **THEN** stored\n";
const AFTER: &str = "# auth — Product\n\n## Requirements\n\n### Requirement: Sign in (R1)\nThe system SHALL sign users in with a passkey.\n\n#### Scenario: ok\n- **WHEN** valid\n- **THEN** in\n\n### Requirement: Enrol (R3)\nThe system MUST enrol passkeys.\n\n#### Scenario: enrol\n- **WHEN** added\n- **THEN** stored\n";

fn delta_tree(name: &str, product: &str) -> PathBuf {
    let p = tree(name, TECH, "# Tech\n");
    write(&p, ".spec/product-auth.md", TOPIC);
    write(&p, ".spec/features/auth/product.md", product);
    p
}

#[test]
fn deltas_apply_after_merge_blocks() {
    let p = delta_tree("pr-delta", DELTA);
    let out = spec(&p, &["promote", "auth"], &[]);
    assert_eq!(out.code, 0, "{}", out.stderr);
    assert_eq!(
        out.text(),
        "PROMOTED 2 block(s) from .spec/features/auth/tech.md → .spec/tech.md (0 already present)\n\
         APPLIED 3 requirement change(s) from .spec/features/auth/product.md → .spec/product-auth.md (0 already applied)\n"
    );
    assert_eq!(fs::read_to_string(p.join(".spec/product-auth.md")).unwrap(), AFTER);
    let again = spec(&p, &["promote", "auth"], &[]);
    assert_eq!(
        again.text(),
        "No new blocks to promote — all 2 already present in .spec/tech.md\n\
         No requirement changes to apply — all 3 already in .spec/product-auth.md\n"
    );
    assert_eq!(fs::read_to_string(p.join(".spec/product-auth.md")).unwrap(), AFTER);
}

#[test]
fn delta_dry_run_writes_nothing() {
    let p = delta_tree("pr-delta-dry", DELTA);
    let out = spec(&p, &["promote", "auth", "--dry-run"], &[]);
    assert_eq!(out.code, 0, "{}", out.stderr);
    assert!(
        out.text().ends_with(
            "\n--- would apply 3 requirement change(s) to .spec/product-auth.md (0 already applied) ---\n\
             REMOVED Sign out (R2)\nMODIFIED Sign in (R1)\nADDED Enrol (R3)\n"
        ),
        "{}",
        out.text()
    );
    assert_eq!(fs::read_to_string(p.join(".spec/product-auth.md")).unwrap(), TOPIC);
    assert_eq!(fs::read_to_string(p.join(".spec/tech.md")).unwrap(), "# Tech\n");
}

#[test]
fn a_refused_delta_writes_nothing_at_all() {
    let p = delta_tree("pr-delta-bad", &DELTA.replace("Sign in (R1)\nThe system SHALL sign users in with", "Ghost\nThe system SHALL haunt"));
    let out = spec(&p, &["promote", "auth"], &[]);
    assert_eq!(out.code, 1);
    assert!(out.text().is_empty());
    assert_eq!(out.stderr, "ERROR: MODIFIED 'Ghost' is not in the target (.spec/features/auth/product.md)\n");
    assert_eq!(fs::read_to_string(p.join(".spec/product-auth.md")).unwrap(), TOPIC);
    assert_eq!(fs::read_to_string(p.join(".spec/tech.md")).unwrap(), "# Tech\n");
}

#[test]
fn a_parent_outside_the_root_is_refused() {
    let p = delta_tree("pr-delta-out", &DELTA.replace("../../product-auth.md", "../../../outside.md"));
    let out = spec(&p, &["promote", "auth"], &[]);
    assert_eq!(out.code, 1);
    assert_eq!(out.stderr, "ERROR: parent '../../../outside.md' leaves the spec root (.spec/features/auth/product.md)\n");
}

#[test]
fn deltas_run_without_merge_blocks_and_share_a_target() {
    let p = delta_tree("pr-delta-only", DELTA);
    write(&p, ".spec/features/auth/tech.md", "no blocks\n");
    let out = spec(&p, &["promote", "auth"], &[]);
    assert_eq!(out.code, 0, "{}", out.stderr);
    assert_eq!(out.stderr, "WARN: no <!-- merge --> blocks found in .spec/features/auth/tech.md\n");
    assert_eq!(fs::read_to_string(p.join(".spec/product-auth.md")).unwrap(), AFTER);
    let shared = delta_tree("pr-delta-shared", DELTA);
    let out = spec(&shared, &["promote", "auth", "--target", ".spec/product-auth.md"], &[]);
    assert_eq!(out.code, 0, "{}", out.stderr);
    let text = fs::read_to_string(shared.join(".spec/product-auth.md")).unwrap();
    assert!(text.starts_with(AFTER), "{text}");
    assert!(text.ends_with("\nTokens expire after 1 hour.\n"), "{text}");
}
```

- [ ] **Step 2: Run them to verify they fail**

Run: `cargo test -p vibe-spec --test promote delta 2>&1 | tail -8`
Expected: 5 FAIL. Nothing applies the product's delta sections yet, so `product-auth.md` is unchanged and the `APPLIED` line is missing.

- [ ] **Step 3: Implement**

In `promote.rs`, add imports and a delta planner:

```rust
use crate::delta::{self, Change};
use crate::layered::lexical;

/// The planned requirement changes for one feature.
struct DeltaPlan {
    dest: PathBuf,
    dest_show: String,
    product_show: String,
    bytes: Vec<u8>,
    changes: Vec<Change>,
}

/// Plans the feature product's delta sections against their target, reading
/// `pending` instead of the disk when a merge write to the same file is
/// queued. Err holds the stderr lines of a refusal.
fn plan_delta(
    dir: &Path,
    display: &str,
    feature: &str,
    product: &[u8],
    pending: Option<(&Path, &[u8])>,
) -> Result<DeltaPlan, Vec<String>> {
    let product_show = format!("{display}/features/{feature}/product.md");
    let fdir = join_raw(&dir.join("features"), feature.as_bytes());
    let dest = delta::target(dir, &fdir, product).map_err(|e| vec![format!("ERROR: {e} ({product_show})")])?;
    let rel = dest
        .strip_prefix(lexical(dir))
        .map(|r| r.components().map(|c| c.as_os_str().to_string_lossy().into_owned()).collect::<Vec<_>>().join("/"))
        .unwrap_or_default();
    let dest_show = format!("{display}/{rel}");
    let base = match pending {
        Some((path, bytes)) if lexical(path) == dest => bytes.to_vec(),
        _ if dest.is_file() => std::fs::read(&dest).unwrap_or_default(),
        _ => return Err(vec![format!("ERROR: {dest_show} not found")]),
    };
    let (bytes, changes) = delta::apply(&base, product)
        .map_err(|errs| errs.into_iter().map(|e| format!("ERROR: {e} ({product_show})")).collect::<Vec<_>>())?;
    Ok(DeltaPlan { dest, dest_show, product_show, bytes, changes })
}

/// The stdout lines for a delta plan (P3c-R9).
fn delta_lines(p: &DeltaPlan, dry_run: bool) -> Vec<u8> {
    let applied = p.changes.iter().filter(|c| c.applied).count();
    let skipped = p.changes.len() - applied;
    let mut out = match (dry_run, applied) {
        (true, 0) => format!("--- nothing to apply: all {} requirement change(s) already in {} ---\n", p.changes.len(), p.dest_show),
        (true, _) => format!("--- would apply {applied} requirement change(s) to {} ({skipped} already applied) ---\n", p.dest_show),
        (false, 0) => format!("No requirement changes to apply — all {} already in {}\n", p.changes.len(), p.dest_show),
        (false, _) => format!("APPLIED {applied} requirement change(s) from {} → {} ({skipped} already applied)\n", p.product_show, p.dest_show),
    }
    .into_bytes();
    if dry_run {
        for c in p.changes.iter().filter(|c| c.applied) {
            out.extend_from_slice(c.kind.as_str().as_bytes());
            out.push(b' ');
            out.extend_from_slice(&c.name);
            out.push(b'\n');
        }
    }
    out
}
```

Restructure the tail of `run`, from `let blocks = …` onward. The legacy text stays byte-identical when there is no delta. Every write waits until both plans succeed:

```rust
    let blocks = match merge_blocks(&std::fs::read(&tech).unwrap_or_default(), &tech_show) {
        Ok(b) => b,
        Err(msg) => return fail(&msg),
    };
    let product = std::fs::read(tech.with_file_name("product.md"))
        .ok()
        .filter(|t| delta::has_delta(t));
    let mut out: Vec<u8> = Vec::new();
    let mut merged: Option<Vec<u8>> = None;
    if blocks.is_empty() {
        eprintln!("WARN: no <!-- merge --> blocks found in {tech_show}");
        if product.is_none() {
            return (exit::OK, Vec::new());
        }
    } else {
        let current = std::fs::read(&target).unwrap_or_default();
        let plan = merge_plan(&current, &blocks);
        out = match (a.dry_run, plan.appended) {
            (true, 0) => format!("--- nothing to promote: all {} block(s) already present in {target_show} ---\n", blocks.len()).into_bytes(),
            (true, _) => {
                let mut o = format!("--- would promote {} block(s) to {target_show} ({} already present) ---\n", plan.appended, plan.skipped).into_bytes();
                o.extend_from_slice(&plan.append);
                o
            }
            (false, 0) => format!("No new blocks to promote — all {} already present in {target_show}\n", blocks.len()).into_bytes(),
            (false, _) => format!("PROMOTED {} block(s) from {tech_show} → {target_show} ({} already present)\n", plan.appended, plan.skipped).into_bytes(),
        };
        if !a.dry_run && plan.appended > 0 {
            let mut bytes = current;
            bytes.extend_from_slice(&plan.append);
            merged = Some(bytes);
        }
    }
    let delta_plan = match &product {
        Some(doc) => {
            let pending = merged.as_deref().map(|b| (target.as_path(), b));
            match plan_delta(&dir, &display, &a.feature, doc, pending) {
                Ok(p) => Some(p),
                Err(lines) => {
                    for l in lines {
                        eprintln!("{l}");
                    }
                    return (exit::FINDINGS, Vec::new());
                }
            }
        }
        None => None,
    };
    if let Some(p) = &delta_plan {
        if !out.is_empty() && !out.ends_with(b"\n") {
            out.push(b'\n');
        }
        out.extend_from_slice(&delta_lines(p, a.dry_run));
    }
    if !a.dry_run {
        if let Some(bytes) = &merged {
            if let Err(e) = vibe_core::fs::write_atomic(&target, bytes) {
                return fail(&format!("ERROR: {target_show}: {e}"));
            }
        }
        if let Some(p) = delta_plan.as_ref().filter(|p| p.changes.iter().any(|c| c.applied)) {
            if let Err(e) = vibe_core::fs::write_atomic(&p.dest, &p.bytes) {
                return fail(&format!("ERROR: {}: {e}", p.dest_show));
            }
        }
    }
    (exit::OK, out)
```

A merge target shared with the delta target is planned from the merged bytes, so the second write carries both. `target` must be compared lexically: `cwd.join(t)` can hold `.` or `..`, which is why `plan_delta` compares `lexical(path) == dest`.

- [ ] **Step 4: Run the tests and parity**

Run: `cargo test -p vibe-spec --test promote 2>&1 | tail -4`
Expected: `6 passed`. The Task 5 oracle test still passes, which proves the legacy output unchanged.

Pin the divergence (P3c-R19). In `corpus_spec.rs`, give `features/delta-removed` promote runs: `runs: &[("promote", &[&["auth", "--dry-run"], &["auth"]])]`. Append to `tests/parity/spec.json`:

```json
    { "command": "promote", "case": "features/delta-removed", "args": ["auth", "--dry-run"], "field": "stdout", "reason": "the port also plans the product's requirement deltas (P3c-R9); the oracle promotes merge blocks only" },
    { "command": "promote", "case": "features/delta-removed", "args": ["auth"], "field": "stdout", "reason": "the port also applies the product's requirement deltas (P3c-R9); the oracle promotes merge blocks only" },
    { "command": "promote", "case": "features/delta-removed", "args": ["auth"], "field": "files", "reason": "the port removes Legacy login (R9) from product-auth.md (P3c-R9); the oracle never reads product.md" }
```

Regenerate: `cargo run --locked -q -p vibe-parity -- gen-spec-corpus oracles/bash/spec/reference/templates tests/parity/corpus/spec`

Run (alone): `bash tests/parity/run.sh 2>&1 | tail -4`
Expected: `0 failed`, three more `pinned`, no `stale pin`.

- [ ] **Step 5: Lint and commit**

Run: `cargo fmt --all && cargo clippy --workspace --all-targets -q -- -D warnings`

```bash
git add crates/vibe-spec/src/promote.rs crates/vibe-spec/tests/promote.rs crates/vibe-parity/src/corpus_spec.rs tests/parity/
git commit -m "feat(spec): apply requirement deltas on promote"
```

### Task 7: Superpowers checklist, `--done` and `promote --json`

**Files:**
- Modify: `crates/vibe-spec/src/promote.rs`, `crates/vibe-spec/src/git.rs`, `crates/vibe-spec/src/superpowers.rs`, `crates/vibe-spec/src/query.rs` (`valid_name` and `rel` → `pub(crate)`)
- Modify: `contracts/spec-json.schema.json`, `contracts/run.mjs`
- Create: `contracts/fixtures/spec-json/promote/valid/{checklist,done,unknown-field}.json`, `contracts/fixtures/spec-json/promote/invalid/{missing-deleted,bad-op}.json`
- Modify: `crates/vibe-parity/src/corpus_spec.rs`, `tests/parity/spec.json` (promote run and pins on `features/superpowers-artifact`); regenerate `tests/parity/corpus/spec/`
- Test: `crates/vibe-spec/tests/promote.rs`

**Interfaces:**
- Consumes: `query::valid_name(&str) -> bool`, `query::rel(&Path, &Path) -> String`, `layered::{detect, Layout}`, `git::git`.
- Produces:
  - `pub fn superpowers::artifacts(project: &Path, feature: &str) -> Vec<PathBuf>`: sorted by project-relative path.
  - `pub fn git::branch_base(dir: &Path, explicit: Option<&str>) -> Option<String>`.
  - `pub fn git::changed_since(dir: &Path, base: &str) -> Option<bool>`.
  - New flags `--done`, `--base <ref>`, `--json`.
  - Spec JSON def `promote`: required `v`, `feature`, `dry_run`, `artifacts`, `deleted`; optional `merge {source, target, appended, skipped}`, `delta {source, target, changes: [{op, name, applied}]}`, `checklist: [{what, target}]`.

- [ ] **Step 1: Write the failing tests**

Append to `crates/vibe-spec/tests/promote.rs`, and import `common::{assert_spec_json, git, layered_tree}`:

```rust
const STAMP: &str = "2026-01-01T00:00:00Z";

/// A committed repo on branch `feat` off `main`, with two artifacts bound
/// to `auth` and one that is not (`auth-v2`).
fn sp_repo(name: &str) -> PathBuf {
    let p = project(name);
    write(&p, ".spec/plan.md", "# Plan\n");
    write(&p, "docs/superpowers/specs/2026-01-02-auth-design.md", "# Auth design\n");
    write(&p, "docs/superpowers/plans/2026-01-14-auth.md", "x\n");
    write(&p, "docs/superpowers/plans/2026-01-14-auth-v2.md", "x\n");
    git(&p, STAMP, &["init", "-q", "-b", "main"]);
    git(&p, STAMP, &["add", "-A"]);
    git(&p, STAMP, &["commit", "-q", "-m", "base"]);
    git(&p, STAMP, &["checkout", "-q", "-b", "feat"]);
    p
}

fn bound_exist(p: &std::path::Path) -> (bool, bool) {
    (
        p.join("docs/superpowers/specs/2026-01-02-auth-design.md").is_file(),
        p.join("docs/superpowers/plans/2026-01-14-auth.md").is_file(),
    )
}

#[test]
fn checklist_names_targets_per_layout() {
    let p = sp_repo("pr-sp-flat");
    let out = spec(&p, &["promote", "auth"], &[]);
    assert_eq!(out.code, 0, "{}", out.stderr);
    assert_eq!(
        out.text(),
        "superpowers artifacts for 'auth':\n\
         \x20 docs/superpowers/plans/2026-01-14-auth.md\n\
         \x20 docs/superpowers/specs/2026-01-02-auth-design.md\n\
         Promote durable facts, then run: vibe-spec promote auth --done\n\
         \x20 [ ] chosen approach → .spec/plan.md\n\
         \x20 [ ] new nouns → .spec/product.md\n\
         \x20 [ ] built places (+code:) → .spec/tech.md\n\
         \x20 [ ] rules → .spec/tech.md\n\
         \x20 [ ] failures → .spec/lessons.md\n"
    );
    let l = layered_tree("pr-sp-layered");
    write(&l, "docs/superpowers/plans/2026-01-14-auth.md", "x\n");
    let out = spec(&l, &["promote", "auth"], &[]);
    assert!(out.text().contains("  [ ] chosen approach → docs/spec/decisions/\n  [ ] new nouns → docs/spec/concepts/\n  [ ] built places (+code:) → docs/spec/areas/\n  [ ] rules → docs/spec/tech/\n  [ ] failures → docs/spec/lessons.md\n"), "{}", out.text());
}

#[test]
fn done_refuses_when_the_spec_root_is_unchanged() {
    let p = sp_repo("pr-done-same");
    let out = spec(&p, &["promote", "auth", "--done"], &[]);
    assert_eq!(out.code, 1);
    assert!(out.text().is_empty());
    assert!(out.stderr.starts_with("promote: refused — .spec/ is unchanged since "), "{}", out.stderr);
    assert_eq!(bound_exist(&p), (true, true));
}

#[test]
fn done_deletes_after_promotion() {
    let p = sp_repo("pr-done-ok");
    write(&p, ".spec/plan.md", "# Plan\n\nD47 promoted.\n");
    let dry = spec(&p, &["promote", "auth", "--done", "--dry-run"], &[]);
    assert_eq!(dry.text(), "WOULD DELETE docs/superpowers/plans/2026-01-14-auth.md\nWOULD DELETE docs/superpowers/specs/2026-01-02-auth-design.md\n");
    assert_eq!(bound_exist(&p), (true, true));
    let out = spec(&p, &["promote", "auth", "--done"], &[]);
    assert_eq!(out.code, 0, "{}", out.stderr);
    assert_eq!(out.text(), "DELETED docs/superpowers/plans/2026-01-14-auth.md\nDELETED docs/superpowers/specs/2026-01-02-auth-design.md\n");
    assert_eq!(bound_exist(&p), (false, false));
    assert!(p.join("docs/superpowers/plans/2026-01-14-auth-v2.md").is_file());
}

#[test]
fn done_counts_untracked_files() {
    let p = sp_repo("pr-done-new");
    write(&p, ".spec/decisions.md", "# new\n");
    assert_eq!(spec(&p, &["promote", "auth", "--done"], &[]).code, 0);
    assert_eq!(bound_exist(&p), (false, false));
}

#[test]
fn done_refuses_without_a_base() {
    let bare = project("pr-done-nogit");
    write(&bare, ".spec/plan.md", "x\n");
    write(&bare, "docs/superpowers/plans/2026-01-14-auth.md", "x\n");
    let out = spec(&bare, &["promote", "auth", "--done"], &[]);
    assert_eq!(out.code, 1);
    assert_eq!(out.stderr, "promote: refused — no branch base found (pass --base <ref>); nothing deleted\n");
    assert!(bare.join("docs/superpowers/plans/2026-01-14-auth.md").is_file());
    let p = sp_repo("pr-done-badbase");
    write(&p, ".spec/plan.md", "changed\n");
    assert_eq!(spec(&p, &["promote", "auth", "--done", "--base", "nope"], &[]).code, 1);
    assert_eq!(bound_exist(&p), (true, true));
}

#[test]
fn done_rejects_unsafe_names() {
    let p = sp_repo("pr-done-name");
    write(&p, ".spec/plan.md", "changed\n");
    let out = spec(&p, &["promote", "../auth", "--done"], &[]);
    assert_eq!(out.code, 1);
    assert_eq!(out.stderr, "promote: refused — invalid feature name \"../auth\" (lowercase-kebab); nothing deleted\n");
    assert_eq!(bound_exist(&p), (true, true));
    let none = spec(&p, &["promote", "billing", "--done"], &[]);
    assert_eq!((none.code, none.stderr.as_str()), (1, "promote: no superpowers artifacts for 'billing'\n"));
}

#[test]
fn json_validates() {
    let p = sp_repo("pr-json");
    let check = spec(&p, &["promote", "auth", "--json"], &[]);
    assert_eq!(check.code, 0, "{}", check.stderr);
    assert_spec_json("promote", &check.text());
    write(&p, ".spec/plan.md", "changed\n");
    let done = spec(&p, &["promote", "auth", "--done", "--json"], &[]);
    assert_eq!(done.code, 0, "{}", done.stderr);
    assert_spec_json("promote", &done.text());
    let v: serde_json::Value = serde_json::from_slice(&done.stdout).unwrap();
    assert_eq!(v["deleted"].as_array().unwrap().len(), 2);
    let d = delta_tree("pr-json-delta", DELTA);
    let both = spec(&d, &["promote", "auth", "--json", "--dry-run"], &[]);
    assert_spec_json("promote", &both.text());
    let v: serde_json::Value = serde_json::from_slice(&both.stdout).unwrap();
    assert_eq!(v["delta"]["changes"][0], serde_json::json!({ "op": "REMOVED", "name": "Sign out (R2)", "applied": true }));
    assert_eq!(v["merge"]["appended"], 2);
}
```

- [ ] **Step 2: Run them to verify they fail**

Run: `cargo test -p vibe-spec --test promote 2>&1 | grep -E 'test .* (ok|FAILED)'`
Expected: the 7 new tests FAIL. For example, `promote auth` with no fallback folder prints `ERROR: .spec/features/auth/tech.md not found` and exits 1.

- [ ] **Step 3: Implement**

`query.rs`: change `fn valid_name` and `fn rel` to `pub(crate) fn`.

`superpowers.rs`:

```rust
/// Every artifact bound to `feature` (D45 binding, P3c-R12): regular files
/// `specs/YYYY-MM-DD-<f>-design.md` and `plans/YYYY-MM-DD-<f>.md`, every
/// date, sorted by project-relative path. Callers pass a valid name.
pub fn artifacts(project: &Path, feature: &str) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for (dir, suffix) in [(PLANS, ".md"), (SPECS, "-design.md")] {
        let want = format!("-{feature}{suffix}");
        let Ok(entries) = std::fs::read_dir(project.join(dir)) else {
            continue;
        };
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            let b = name.as_bytes();
            let bound = b.len() == 10 + want.len()
                && crate::text::is_date(b.get(..10).unwrap_or_default())
                && b.get(10..) == Some(want.as_bytes());
            if bound && std::fs::symlink_metadata(entry.path()).is_ok_and(|m| m.is_file()) {
                out.push(entry.path());
            }
        }
    }
    out.sort_by_key(|p| crate::query::rel(project, p));
    out
}
```

`git.rs`:

```rust
/// The commit this branch forked from (P3c-R13): `merge-base HEAD <base>`,
/// where `<base>` is `explicit` or the first of `origin/HEAD`,
/// `origin/main`, `main`, `master` that names a commit. None without git,
/// history or a base.
pub fn branch_base(dir: &Path, explicit: Option<&str>) -> Option<String> {
    let candidates: Vec<&str> = match explicit {
        Some(b) => vec![b],
        None => vec!["origin/HEAD", "origin/main", "main", "master"],
    };
    let base = candidates
        .into_iter()
        .find(|c| git(dir, &["rev-parse", "--verify", "-q", &format!("{c}^{{commit}}")]).is_some())?;
    let out = git(dir, &["merge-base", "HEAD", base])?;
    let hash = String::from_utf8_lossy(&out).trim().to_string();
    (!hash.is_empty()).then_some(hash)
}

/// Whether the tree under `dir` differs from `base`: tracked changes,
/// committed or not, or untracked files. None when git fails.
pub fn changed_since(dir: &Path, base: &str) -> Option<bool> {
    let diff = git(dir, &["diff", "--name-only", base, "--", "."])?;
    if !diff.is_empty() {
        return Some(true);
    }
    let untracked = git(dir, &["ls-files", "--others", "--exclude-standard", "--", "."])?;
    Some(!untracked.is_empty())
}
```

`promote.rs`. Extend `Args` with `done: bool`, `base: Option<String>`, `json: bool`, and add three arms to `parse`:

```rust
            "--done" => a.done = true,
            "--json" => a.json = true,
            "--base" => {
                let b = args
                    .get(i + 1)
                    .filter(|b| !b.is_empty() && !b.starts_with('-'))
                    .ok_or_else(|| "promote: --base requires a ref".to_string())?;
                a.base = Some(b.clone());
                i += 1;
            }
```

Update `USAGE` to `usage: vibe-spec promote <feature-name> [--dry-run] [--target <file>] [--done] [--base <ref>] [--json]`.

Add the checklist and `--done`:

```rust
use serde_json::{Value, json};

use crate::layered::{Layout, detect};
use crate::query::{rel, valid_name};
use crate::{git, superpowers};

/// The promotion checklist (spec §5): what goes where, per layout.
fn checklist(root: &Root) -> Vec<(&'static str, String)> {
    let d = root.display();
    let to = |flat: &str, layered: &str| {
        if detect(&root.dir) == Layout::Layered { format!("{d}/{layered}") } else { format!("{d}/{flat}") }
    };
    vec![
        ("chosen approach", to("plan.md", "decisions/")),
        ("new nouns", to("product.md", "concepts/")),
        ("built places (+code:)", to("tech.md", "areas/")),
        ("rules", to("tech.md", "tech/")),
        ("failures", to("lessons.md", "lessons.md")),
    ]
}

fn checklist_text(feature: &str, shown: &[String], items: &[(&str, String)]) -> Vec<u8> {
    let mut out = format!("superpowers artifacts for '{feature}':\n");
    for s in shown {
        out += &format!("  {s}\n");
    }
    out += &format!("Promote durable facts, then run: vibe-spec promote {feature} --done\n");
    for (what, target) in items {
        out += &format!("  [ ] {what} → {target}\n");
    }
    out.into_bytes()
}

fn refuse(msg: &str) -> (u8, Vec<u8>) {
    eprintln!("promote: refused — {msg}; nothing deleted");
    (exit::FINDINGS, Vec::new())
}

/// `promote <f> --done` (D46, P3c-R13): delete the bound artifacts once the
/// spec root changed since the branch base; fail closed otherwise.
fn done(found: &Result<Root, Miss>, a: &Args) -> (u8, Vec<u8>) {
    let root = match found {
        Ok(root) => root,
        Err(miss) => {
            eprintln!("{}", miss.message());
            return (miss.code(), Vec::new());
        }
    };
    if !valid_name(&a.feature) {
        return refuse(&format!("invalid feature name {:?} (lowercase-kebab)", a.feature));
    }
    let found_artifacts = superpowers::artifacts(&root.project, &a.feature);
    if found_artifacts.is_empty() {
        return fail(&format!("promote: no superpowers artifacts for '{}'", a.feature));
    }
    let Some(base) = git::branch_base(&root.project, a.base.as_deref()) else {
        return refuse("no branch base found (pass --base <ref>)");
    };
    let short = base.get(..7).unwrap_or(&base);
    match git::changed_since(&root.dir, &base) {
        Some(true) => {}
        Some(false) => {
            return refuse(&format!("{}/ is unchanged since {short}; promote the artifacts' durable facts first", root.display()));
        }
        None => return refuse(&format!("cannot compare {}/ with {short}", root.display())),
    }
    let shown: Vec<String> = found_artifacts.iter().map(|p| rel(&root.project, p)).collect();
    if !a.dry_run {
        for (path, s) in found_artifacts.iter().zip(&shown) {
            if let Err(e) = std::fs::remove_file(path) {
                return fail(&format!("promote: {s}: {e}"));
            }
        }
    }
    if a.json {
        let v = json!({
            "v": 1, "feature": a.feature, "dry_run": a.dry_run,
            "artifacts": shown, "deleted": if a.dry_run { Vec::new() } else { shown.clone() },
        });
        return (exit::OK, format!("{v}\n").into_bytes());
    }
    let word = if a.dry_run { "WOULD DELETE" } else { "DELETED" };
    let out: String = shown.iter().map(|s| format!("{word} {s}\n")).collect();
    (exit::OK, out.into_bytes())
}
```

Wire it into `run`, right after the `(dir, display)` match:

```rust
    if a.done {
        return done(found, &a);
    }
    let sp: Vec<PathBuf> = match found {
        Ok(root) if valid_name(&a.feature) => superpowers::artifacts(&root.project, &a.feature),
        _ => Vec::new(),
    };
    let sp_out = match found {
        Ok(root) if !sp.is_empty() => {
            let shown: Vec<String> = sp.iter().map(|p| rel(&root.project, p)).collect();
            Some((shown, checklist(root)))
        }
        _ => None,
    };
```

Then:
- If `!tech.is_file()` and `sp_out.is_some()`: skip the legacy and delta paths; output is only the checklist (text) or JSON. Exit 0.
- If `blocks.is_empty() && product.is_none()`: return `(exit::OK, Vec::new())` only when `sp_out` is None. Otherwise fall through, so the checklist prints after the WARN line.
- After the delta lines: when `sp_out` is `Some((shown, items))`, append `checklist_text(&a.feature, shown, items)`. Add a `\n` first if `out` does not end with one.

`--json` for the non-`--done` path: build the value from what `run` planned and print `format!("{v}\n")` instead of `out`. Every write still happens exactly as in text mode.

```rust
    let mut v = json!({ "v": 1, "feature": a.feature, "dry_run": a.dry_run, "artifacts": [], "deleted": [] });
    if let Some((shown, items)) = &sp_out {
        v["artifacts"] = json!(shown);
        v["checklist"] = Value::Array(items.iter().map(|(w, t)| json!({ "what": w, "target": t })).collect());
    }
    // when blocks were planned:
    v["merge"] = json!({ "source": tech_show, "target": target_show, "appended": appended, "skipped": skipped });
    // when a delta plan exists:
    v["delta"] = json!({
        "source": p.product_show, "target": p.dest_show,
        "changes": p.changes.iter().map(|c| json!({ "op": c.kind.as_str(), "name": crate::text::lossy(&c.name), "applied": c.applied })).collect::<Vec<_>>(),
    });
```

Keep `appended` and `skipped` from the merge plan in locals so the JSON branch can read them.

Contract. Add to `$defs` in `contracts/spec-json.schema.json`:

```json
    "promote": {
      "type": "object",
      "required": ["v", "feature", "dry_run", "artifacts", "deleted"],
      "properties": {
        "v": { "const": 1 },
        "feature": { "type": "string", "minLength": 1 },
        "dry_run": { "type": "boolean" },
        "merge": {
          "type": "object",
          "required": ["source", "target", "appended", "skipped"],
          "properties": {
            "source": { "type": "string", "minLength": 1 },
            "target": { "type": "string", "minLength": 1 },
            "appended": { "type": "integer", "minimum": 0 },
            "skipped": { "type": "integer", "minimum": 0 }
          }
        },
        "delta": {
          "type": "object",
          "required": ["source", "target", "changes"],
          "properties": {
            "source": { "type": "string", "minLength": 1 },
            "target": { "type": "string", "minLength": 1 },
            "changes": { "type": "array", "items": { "$ref": "#/$defs/promote-change" } }
          }
        },
        "artifacts": { "type": "array", "items": { "type": "string", "minLength": 1 } },
        "checklist": { "type": "array", "items": { "$ref": "#/$defs/promote-item" } },
        "deleted": { "type": "array", "items": { "type": "string", "minLength": 1 } }
      }
    },
    "promote-change": {
      "type": "object",
      "required": ["op", "name", "applied"],
      "properties": {
        "op": { "enum": ["ADDED", "MODIFIED", "REMOVED"] },
        "name": { "type": "string", "minLength": 1 },
        "applied": { "type": "boolean" }
      }
    },
    "promote-item": {
      "type": "object",
      "required": ["what", "target"],
      "properties": {
        "what": { "type": "string", "minLength": 1 },
        "target": { "type": "string", "minLength": 1 }
      }
    },
```

In `run.mjs`, add `'promote'` to `QUERIES`, and add to `REQUIRED`:

```js
  promote: ['artifacts', 'deleted', 'dry_run', 'feature', 'v'],
  'promote-change': ['applied', 'name', 'op'],
  'promote-item': ['target', 'what'],
```

Goldens:
- `valid/checklist.json`: `{"v":1,"feature":"auth","dry_run":false,"artifacts":["docs/superpowers/plans/2026-01-14-auth.md"],"checklist":[{"what":"rules","target":".spec/tech.md"}],"deleted":[]}`
- `valid/done.json`: `{"v":1,"feature":"auth","dry_run":false,"artifacts":["docs/superpowers/plans/2026-01-14-auth.md"],"deleted":["docs/superpowers/plans/2026-01-14-auth.md"],"merge":{"source":".spec/features/auth/tech.md","target":".spec/tech.md","appended":1,"skipped":0},"delta":{"source":".spec/features/auth/product.md","target":".spec/product.md","changes":[{"op":"ADDED","name":"Audit","applied":true}]}}`
- `valid/unknown-field.json`: `checklist.json` plus `"took_ms":1`
- `invalid/missing-deleted.json`: `checklist.json` without `deleted`
- `invalid/bad-op.json`: `done.json` with `"op":"RENAMED"`

- [ ] **Step 4: Run the tests, contracts and parity**

Run: `cargo test -p vibe-spec --test promote 2>&1 | tail -4 && bash contracts/run.sh 2>&1 | tail -2`
Expected: `13 passed`; contracts `OK`.

Discriminating check for spec §9 ("`promote --done` refusal vs naive delete"):
1. Temporarily make `done` skip the `changed_since` match: replace the match with `let _ = &base;`.
2. Run `cargo test -p vibe-spec --test promote done_refuses_when 2>&1 | tail -3`. Expected: FAIL, because `bound_exist` is `(false, false)`.
3. Restore the match and run the test again. Expected: PASS.

Record both outputs in the task report.

Pin the divergence (P3c-R19). In `corpus_spec.rs`, give `features/superpowers-artifact` a promote run: `runs: &[("promote", &[&["auth"]])]`. Append to `tests/parity/spec.json`:

```json
    { "command": "promote", "case": "features/superpowers-artifact", "args": ["auth"], "field": "exit", "reason": "the port prints the superpowers promotion checklist (P3c-R12); the oracle fails on the missing fallback folder" },
    { "command": "promote", "case": "features/superpowers-artifact", "args": ["auth"], "field": "stdout", "reason": "the port prints the superpowers promotion checklist (P3c-R12); the oracle prints nothing" }
```

Regenerate: `cargo run --locked -q -p vibe-parity -- gen-spec-corpus oracles/bash/spec/reference/templates tests/parity/corpus/spec`

Run (alone): `bash tests/parity/run.sh 2>&1 | tail -4`
Expected: `0 failed`, two more `pinned`, no `stale pin`.

- [ ] **Step 5: Lint and commit**

Run: `cargo fmt --all && cargo clippy --workspace --all-targets -q -- -D warnings && cargo test -p vibe-spec -q 2>&1 | grep -E 'test result|FAILED'`

```bash
git add crates/vibe-spec/src/ crates/vibe-spec/tests/promote.rs contracts/ crates/vibe-parity/src/corpus_spec.rs tests/parity/
git commit -m "feat(spec): add promote checklist and --done"
```

### Task 8: `export --openspec`

**Files:**
- Create: `crates/vibe-spec/src/openspec.rs`
- Modify: `crates/vibe-spec/src/main.rs` (subcommand `Export`; drop the `#[allow(dead_code)]` on `mod delta`)
- Modify: `contracts/spec-json.schema.json`, `contracts/run.mjs`
- Create: `contracts/fixtures/spec-json/openspec/valid/{export,import,unknown-field}.json`, `contracts/fixtures/spec-json/openspec/invalid/{bad-direction,missing-written}.json`
- Create: `tests/openspec/fixture/.spec/{product.md,product-auth.md,plan.md,tech.md,lessons.md}`, `tests/openspec/fixture/.spec/features/passkeys/{product.md,tech.md,plan.md}`, `tests/openspec/fixture/.spec/features/notes/product.md`
- Test: `crates/vibe-spec/tests/openspec.rs` (new)

**Interfaces:**
- Consumes: `delta::{sections, requirements, scenarios, target, Kind, Block}`, `query::valid_name`, `text::{lines, lossy, name_of, sub_dirs, walk_files}`, `vibe_core::fs::write_atomic`.
- Produces:
  - `pub fn openspec::export(found: &Result<Root, Miss>, cwd: &Path, dir: &Path, json: bool) -> (u8, Vec<u8>)`.
  - `const MARK: &str = "<!-- vibe:export v1 -->"`, `const CARRIED: [&str; 5]`, `fn carry(name: &str, bytes: &[u8]) -> Vec<u8>`. Task 9 adds `uncarry` beside them.
  - Spec JSON def `openspec`: required `v`, `direction` (`export|import`), `dir`, `written` (array of non-empty strings).

- [ ] **Step 1: Create the fixture and write the failing tests**

The fixture is the gate's input in Task 10, so it must pass `openspec validate --strict`:
- every requirement body has an uppercase SHALL or MUST;
- every scenario has a body;
- the MODIFIED block keeps every scenario of its target.

Create these files under `tests/openspec/fixture/.spec/`:

`product.md`:

```markdown
---
type: entrypoint
scope: product
children: product-auth.md
updated: 2026-01-15
---

# Fixture — Product

## Requirements

At a project level, Fixture must:

1. **Sign users in.** Every user can sign in.
```

`product-auth.md`:

```markdown
---
type: product-topic
parent: product.md
scope: auth
covers: sign-in
updated: 2026-01-15
---

# auth — Product

Sign-in for every user.

## Requirements

### Requirement: Sign in (R1)

The system SHALL issue a session on valid credentials.

#### Scenario: Valid credentials

- **WHEN** a user submits valid credentials
- **THEN** a session is issued

### Requirement: Sign out (R2)

The system SHALL end the session on request.

#### Scenario: Sign out

- **WHEN** the user signs out
- **THEN** the session ends
```

`plan.md`: `---\ntype: entrypoint\nscope: plan\nupdated: 2026-01-15\n---\n\n# Fixture — Plan\n\n| passkeys | IN PROGRESS |\n`
`tech.md`: `---\ntype: entrypoint\nscope: tech\nupdated: 2026-01-15\n---\n\n# Fixture — Tech\n`
`lessons.md`: `# Lessons\n`

`features/passkeys/product.md`:

```markdown
---
type: feature-product
feature: passkeys
sibling: tech.md
parent: ../../product-auth.md
updated: 2026-01-15
---

# Feature: Passkeys — Product

Passkeys replace passwords for sign-in.

## Scope

| | |
|---|---|
| **Owns** | passkey sign-in |
| **Does not own** | billing |

## MODIFIED Requirements

### Requirement: Sign in (R1)

The system SHALL issue a session on a valid passkey.

#### Scenario: Valid credentials

- **WHEN** a user presents a valid passkey
- **THEN** a session is issued

## REMOVED Requirements

### Requirement: Sign out (R2)
**Reason**: sessions expire on their own.

## ADDED Requirements

### Requirement: Passkey enrolment (R3)

The system MUST let a signed-in user enrol a passkey.

#### Scenario: Enrol

- **WHEN** a signed-in user adds a passkey
- **THEN** the passkey is stored
```

`features/passkeys/tech.md` holds a nested fence, to prove the carried fence is longer than any run inside:

````markdown
---
type: feature-tech
feature: passkeys
sibling: product.md
parent: ../../tech.md
updated: 2026-01-15
---

# Feature: Passkeys — Tech

```rust
let fence = "```";
```
````

Create `features/passkeys/plan.md` with no final newline. Use `printf '%s'` or the Write tool, and check it with `tail -c1`. Its content:

```markdown
---
type: feature-plan
feature: passkeys
parent: ../../plan.md
updated: 2026-01-15
---

# Feature: Passkeys — Plan

## Requirements Trace

| ID | Requirement | Units |
|---|---|---|
| R1 | Sign in | passkeys/1 |
| R3 | Enrol | passkeys/2 |

### passkeys/1 — WebAuthn ceremony

### passkeys/2 — Enrolment screen

## Progress

| Unit | Status |
|---|---|
| passkeys/1 | DONE |
| passkeys/2 | NOT STARTED |
```

`features/notes/product.md` is a feature with no requirement blocks. It exercises the `skip_specs` path through the real CLI in Task 10:

```markdown
---
type: feature-product
feature: notes
updated: 2026-01-15
---

# Feature: Notes — Product

Release notes for every change.
```

Create `crates/vibe-spec/tests/openspec.rs`:

```rust
#![cfg(unix)]
#![allow(clippy::unwrap_used, clippy::expect_used)]
//! `vibe-spec export|import --openspec` (F18, P3c-R14–R16).
mod common;

use std::fs;
use std::path::{Path, PathBuf};

use common::{assert_spec_json, project, repo, spec, write};

/// Copies `tests/openspec/fixture/` into a fresh project.
fn fixture(name: &str) -> PathBuf {
    fn copy(from: &Path, to: &Path) {
        fs::create_dir_all(to).unwrap();
        for e in fs::read_dir(from).unwrap() {
            let e = e.unwrap();
            if e.file_type().unwrap().is_dir() {
                copy(&e.path(), &to.join(e.file_name()));
            } else {
                fs::copy(e.path(), to.join(e.file_name())).unwrap();
            }
        }
    }
    let p = project(name);
    copy(&repo().join("tests/openspec/fixture"), &p);
    p
}

fn read(p: &Path, rel: &str) -> String {
    fs::read_to_string(p.join(rel)).unwrap()
}

const SPEC: &str = "# product-auth Specification\n\n## Purpose\nRequirements of .spec/product-auth.md in the vibe spec, exported by vibe-spec.\n\n## Requirements\n\n### Requirement: Sign in (R1)\n\nThe system SHALL issue a session on valid credentials.\n\n#### Scenario: Valid credentials\n\n- **WHEN** a user submits valid credentials\n- **THEN** a session is issued\n\n### Requirement: Sign out (R2)\n\nThe system SHALL end the session on request.\n\n#### Scenario: Sign out\n\n- **WHEN** the user signs out\n- **THEN** the session ends\n";
const DELTA: &str = "## ADDED Requirements\n\n### Requirement: Passkey enrolment (R3)\n\nThe system MUST let a signed-in user enrol a passkey.\n\n#### Scenario: Enrol\n\n- **WHEN** a signed-in user adds a passkey\n- **THEN** the passkey is stored\n\n## MODIFIED Requirements\n\n### Requirement: Sign in (R1)\n\nThe system SHALL issue a session on a valid passkey.\n\n#### Scenario: Valid credentials\n\n- **WHEN** a user presents a valid passkey\n- **THEN** a session is issued\n\n## REMOVED Requirements\n\n### Requirement: Sign out (R2)\n**Reason**: sessions expire on their own.\n";

#[test]
fn export_writes_the_openspec_layout() {
    let p = fixture("os-export");
    let out = spec(&p, &["export", "--openspec", "out"], &[]);
    assert_eq!(out.code, 0, "{}", out.stderr);
    assert_eq!(
        out.text(),
        "EXPORTED 8 file(s) to out/openspec/\n\
         \x20 openspec/specs/product-auth/spec.md\n\
         \x20 openspec/changes/notes/.openspec.yaml\n\
         \x20 openspec/changes/notes/proposal.md\n\
         \x20 openspec/changes/notes/design.md\n\
         \x20 openspec/changes/passkeys/specs/product-auth/spec.md\n\
         \x20 openspec/changes/passkeys/proposal.md\n\
         \x20 openspec/changes/passkeys/tasks.md\n\
         \x20 openspec/changes/passkeys/design.md\n"
    );
    assert_eq!(read(&p, "out/openspec/specs/product-auth/spec.md"), SPEC);
    assert_eq!(read(&p, "out/openspec/changes/passkeys/specs/product-auth/spec.md"), DELTA);
    assert_eq!(
        read(&p, "out/openspec/changes/passkeys/tasks.md"),
        "# Tasks\n\n## 1. passkeys\n\n- [x] 1.1 WebAuthn ceremony\n- [ ] 1.2 Enrolment screen\n"
    );
    assert_eq!(
        read(&p, "out/openspec/changes/passkeys/proposal.md"),
        "# Proposal: passkeys\n\n## Why\nPasskeys replace passwords for sign-in.\n\n## What Changes\n- Exported from `.spec/features/passkeys/` by vibe-spec.\n"
    );
    let design = read(&p, "out/openspec/changes/passkeys/design.md");
    assert!(design.contains("<!-- vibe:export v1 -->\n"), "{design}");
    let tech = read(&p, ".spec/features/passkeys/tech.md");
    assert!(design.contains(&format!("\n## vibe:tech.md {}\n\n````text\n{tech}````\n", tech.len())), "{design}");
    assert!(!p.join("out/openspec/project.md").exists());
    assert_eq!(read(&p, "out/openspec/changes/notes/.openspec.yaml"), "schema: spec-driven\nskip_specs: true\n");
}

#[test]
fn export_json_validates_and_refuses_an_existing_dir() {
    let p = fixture("os-export-json");
    let out = spec(&p, &["export", "--openspec", "out", "--json"], &[]);
    assert_eq!(out.code, 0, "{}", out.stderr);
    assert_spec_json("openspec", &out.text());
    let again = spec(&p, &["export", "--openspec", "out"], &[]);
    assert_eq!(again.code, 1);
    assert_eq!(again.stderr, "export: out/openspec already exists\n");
}

#[test]
fn export_refuses_a_modified_that_drops_a_scenario() {
    let p = fixture("os-export-drop");
    let product = read(&p, ".spec/features/passkeys/product.md").replace("#### Scenario: Valid credentials", "#### Scenario: Passkey");
    write(&p, ".spec/features/passkeys/product.md", &product);
    let out = spec(&p, &["export", "--openspec", "out"], &[]);
    assert_eq!(out.code, 1);
    assert_eq!(
        out.stderr,
        "export: .spec/features/passkeys/product.md: MODIFIED 'Sign in (R1)' drops scenario(s) the target still has: Valid credentials\n"
    );
    assert!(!p.join("out").exists());
}

#[test]
fn export_refuses_duplicate_names() {
    let p = fixture("os-export-dup");
    let topic = read(&p, ".spec/product-auth.md").replace("Sign out (R2)", "Sign in (R1)");
    write(&p, ".spec/product-auth.md", &topic);
    let out = spec(&p, &["export", "--openspec", "out"], &[]);
    assert_eq!(out.code, 1);
    assert!(out.stderr.contains("export: .spec/product-auth.md: duplicate requirement 'Sign in (R1)'\n"), "{}", out.stderr);
    assert!(!p.join("out").exists());
}
```

`tech.md` and every other fixture file end with a newline. The one exception is `features/passkeys/plan.md`.

- [ ] **Step 2: Run them to verify they fail**

Run: `cargo test -p vibe-spec --test openspec 2>&1 | tail -6`
Expected: FAIL. `unrecognized subcommand 'export'`.

- [ ] **Step 3: Implement**

Create `crates/vibe-spec/src/openspec.rs`:

```rust
//! `vibe-spec export --openspec <dir>` and `import --openspec <dir>
//! [--feature <name>]` (F18, D34, P3c-R14–R16): OpenSpec's grammar and
//! layout as an interchange format. vibe's own layout never changes, and
//! `openspec/specs/` is never written back into vibe's root documents.
use std::path::{Path, PathBuf};

use serde_json::json;
use vibe_core::exit;

use crate::delta::{self, Block, Kind};
use crate::query::valid_name;
use crate::root::{Miss, Root};
use crate::text::{Fences, lines, lossy, name_of, sub_dirs, walk_files};

/// Starts the carried vibe files in an exported design.md.
const MARK: &str = "<!-- vibe:export v1 -->";
/// The fallback feature files an export carries byte for byte.
const CARRIED: [&str; 5] = ["product.md", "tech.md", "design.md", "plan.md", "research.md"];

/// Files to write: path components under the output dir, and bytes.
type Files = Vec<(Vec<String>, Vec<u8>)>;

fn under(parts: &[&str], tail: &str) -> Vec<String> {
    let mut v: Vec<String> = parts.iter().map(|s| (*s).to_string()).collect();
    v.extend(tail.split('/').map(str::to_string));
    v
}

/// Blocks joined by one blank line, each ending in a newline.
fn join_blocks(text: &[u8], blocks: &[Block]) -> Vec<u8> {
    let mut out = Vec::new();
    for (i, b) in blocks.iter().enumerate() {
        if i > 0 {
            out.push(b'\n');
        }
        out.extend_from_slice(text.get(b.start..b.end).unwrap_or_default());
        if !out.ends_with(b"\n") {
            out.push(b'\n');
        }
    }
    out
}

fn duplicate(blocks: &[Block]) -> Option<Vec<u8>> {
    blocks
        .iter()
        .enumerate()
        .find(|(i, b)| blocks.iter().take(*i).any(|o| o.name == b.name))
        .map(|(_, b)| b.name.clone())
}

/// One carried file: `## vibe:<name> <len>`, a fence longer than any
/// backtick run inside, the bytes, a newline when they lack one, the fence.
fn carry(name: &str, bytes: &[u8]) -> Vec<u8> {
    let longest = bytes.split(|&c| c != b'`').map(<[u8]>::len).max().unwrap_or(0);
    let fence = "`".repeat(longest.max(2) + 1);
    let mut out = format!("\n## vibe:{name} {}\n\n{fence}text\n", bytes.len()).into_bytes();
    out.extend_from_slice(bytes);
    if !bytes.ends_with(b"\n") {
        out.push(b'\n');
    }
    out.extend_from_slice(fence.as_bytes());
    out.push(b'\n');
    out
}

/// The paragraph after the H1: consecutive non-blank lines that are not
/// headings, comments, rules or `**Label:**` lines, joined by spaces.
fn summary(product: &[u8]) -> Option<String> {
    let mut seen_h1 = false;
    let mut para: Vec<String> = Vec::new();
    for line in lines(product) {
        let t = lossy(line).trim().to_string();
        if !seen_h1 {
            seen_h1 = t.starts_with("# ");
            continue;
        }
        let skip = t.is_empty() || t.starts_with('#') || t.starts_with("<!--") || t.starts_with("**") || t == "---";
        if skip {
            if para.is_empty() {
                continue;
            }
            break;
        }
        para.push(t);
    }
    (!para.is_empty()).then(|| para.join(" "))
}

/// Whether the plan's Progress row for `id` says DONE (a whole cell).
fn done(plan: &[u8], id: &str) -> bool {
    lines(plan).into_iter().any(|l| {
        let cells: Vec<String> = lossy(l).split('|').map(|c| c.trim().to_string()).collect();
        cells.get(1).is_some_and(|c| c == id)
            && cells.iter().any(|c| c.chars().filter(char::is_ascii_alphabetic).collect::<String>() == "DONE")
    })
}

/// tasks.md for a fallback plan's `### <f>/<n>` units: `- [ ] 1.<n> <title>`,
/// `[x]` when Progress says DONE. None without units.
fn tasks(plan: &[u8], f: &str) -> Option<Vec<u8>> {
    let mut out = format!("# Tasks\n\n## 1. {f}\n\n");
    let mut any = false;
    let mut fences = Fences::new();
    let prefix = format!("### {f}/");
    for line in lines(plan) {
        if fences.skip(line) {
            continue;
        }
        let Some(after) = line.strip_prefix(prefix.as_bytes()) else {
            continue;
        };
        let n: String = lossy(after).chars().take_while(char::is_ascii_alphanumeric).collect();
        if !n.starts_with(|c: char| c.is_ascii_digit()) {
            continue;
        }
        let title = lossy(after.get(n.len()..).unwrap_or_default());
        let title = title.trim().trim_start_matches(['—', '-', ':']).trim();
        let title = if title.is_empty() { format!("{f}/{n}") } else { title.to_string() };
        let mark = if done(plan, &format!("{f}/{n}")) { "x" } else { " " };
        out += &format!("- [{mark}] 1.{n} {title}\n");
        any = true;
    }
    any.then(|| out.into_bytes())
}

/// Spec documents outside `features/` and `archive/` (relative paths).
fn docs(dir: &Path) -> Vec<String> {
    walk_files(dir)
        .into_iter()
        .filter(|r| r.ends_with(".md") && !r.starts_with("features/") && !r.starts_with("archive/"))
        .collect()
}

/// `path` under `root` as a capability path: `/`-joined, `.md` dropped.
fn capability(root: &Path, path: &Path) -> String {
    let rel = path
        .strip_prefix(crate::layered::lexical(root))
        .map(|r| r.components().map(|c| c.as_os_str().to_string_lossy().into_owned()).collect::<Vec<_>>().join("/"))
        .unwrap_or_default();
    rel.trim_end_matches(".md").to_string()
}

/// One fallback feature folder as an OpenSpec change (P3c-R14).
fn change(root: &Root, fdir: &Path, f: &str) -> Result<Files, Vec<String>> {
    let show = format!("{}/features/{f}", root.display());
    let Ok(product) = std::fs::read(fdir.join("product.md")) else {
        eprintln!("export: {show}/: no product.md, not exported");
        return Ok(Vec::new());
    };
    if !valid_name(f) {
        return Err(vec![format!("{show}/: '{f}' is not a valid change id (lowercase-kebab)")]);
    }
    let secs = delta::sections(&product);
    if secs.iter().any(|s| s.kind == Kind::Renamed) {
        return Err(vec![format!("{show}/product.md: RENAMED Requirements is not supported; write REMOVED + ADDED")]);
    }
    let target = delta::target(&root.dir, fdir, &product).map_err(|e| vec![format!("{show}/product.md: {e}")])?;
    let cap = capability(&root.dir, &target);
    let main = std::fs::read(&target).unwrap_or_default();
    let main_blocks = delta::requirements(&main);
    let mut errs = Vec::new();
    let mut groups: Vec<Vec<u8>> = Vec::new();
    let mut all: Vec<Block> = Vec::new();
    for kind in [Kind::Added, Kind::Modified, Kind::Removed] {
        let blocks: Vec<Block> = secs
            .iter()
            .filter(|s| s.kind == kind || (kind == Kind::Added && s.kind == Kind::Plain))
            .flat_map(|s| s.blocks.clone())
            .collect();
        if blocks.is_empty() {
            continue;
        }
        if kind == Kind::Modified {
            for b in &blocks {
                let Some(old) = main_blocks.iter().find(|m| m.name == b.name) else {
                    continue;
                };
                let keep = delta::scenarios(product.get(b.start..b.end).unwrap_or_default());
                let lost: Vec<String> = delta::scenarios(main.get(old.start..old.end).unwrap_or_default())
                    .into_iter()
                    .filter(|s| !keep.contains(s))
                    .map(|s| lossy(&s))
                    .collect();
                if !lost.is_empty() {
                    errs.push(format!("{show}/product.md: MODIFIED '{}' drops scenario(s) the target still has: {}", lossy(&b.name), lost.join(", ")));
                }
            }
        }
        let mut text = format!("## {} Requirements\n\n", kind.as_str()).into_bytes();
        text.extend(join_blocks(&product, &blocks));
        groups.push(text);
        all.extend(blocks);
    }
    if let Some(dup) = duplicate(&all) {
        errs.push(format!("{show}/product.md: duplicate requirement '{}'", lossy(&dup)));
    }
    if !errs.is_empty() {
        return Err(errs);
    }
    let base = ["openspec", "changes", f];
    let mut files: Files = Vec::new();
    if groups.is_empty() {
        files.push((under(&base, ".openspec.yaml"), b"schema: spec-driven\nskip_specs: true\n".to_vec()));
    } else {
        files.push((under(&base, &format!("specs/{cap}/spec.md")), groups.join(&b'\n')));
    }
    let why = summary(&product).unwrap_or_else(|| format!("Change exported by vibe-spec from {show}/."));
    let proposal = format!("# Proposal: {f}\n\n## Why\n{why}\n\n## What Changes\n- Exported from `{show}/` by vibe-spec.\n");
    files.push((under(&base, "proposal.md"), proposal.into_bytes()));
    if let Some(t) = tasks(&std::fs::read(fdir.join("plan.md")).unwrap_or_default(), f) {
        files.push((under(&base, "tasks.md"), t));
    }
    let mut design = format!(
        "# Design: {f}\n\nExported from `{show}/` by vibe-spec. The blocks below carry the vibe files byte for byte; `vibe-spec import --openspec` restores them.\n\n{MARK}\n"
    )
    .into_bytes();
    for name in CARRIED {
        let path = fdir.join(name);
        if path.is_file() {
            design.extend(carry(name, &std::fs::read(&path).unwrap_or_default()));
        }
    }
    files.push((under(&base, "design.md"), design));
    for other in walk_files(fdir).into_iter().filter(|r| !CARRIED.contains(&r.as_str())) {
        eprintln!("export: {show}/{other}: not exported");
    }
    Ok(files)
}

/// Every capability and change of the tree, in memory (P3c-R14).
fn plan_export(root: &Root) -> Result<Files, Vec<String>> {
    let d = root.display();
    let mut files: Files = Vec::new();
    let mut errs: Vec<String> = Vec::new();
    for rel in docs(&root.dir) {
        let text = std::fs::read(root.dir.join(&rel)).unwrap_or_default();
        let blocks = delta::requirements(&text);
        if blocks.is_empty() {
            continue;
        }
        if let Some(dup) = duplicate(&blocks) {
            errs.push(format!("{d}/{rel}: duplicate requirement '{}'", lossy(&dup)));
            continue;
        }
        let cap = rel.trim_end_matches(".md");
        let mut spec = format!(
            "# {cap} Specification\n\n## Purpose\nRequirements of {d}/{rel} in the vibe spec, exported by vibe-spec.\n\n## Requirements\n\n"
        )
        .into_bytes();
        spec.extend(join_blocks(&text, &blocks));
        files.push((under(&["openspec", "specs"], &format!("{cap}/spec.md")), spec));
    }
    for fdir in sub_dirs(&root.dir.join("features")) {
        match change(root, &fdir, &name_of(&fdir)) {
            Ok(mut f) => files.append(&mut f),
            Err(mut e) => errs.append(&mut e),
        }
    }
    if errs.is_empty() { Ok(files) } else { Err(errs) }
}

/// Writes `files` under `dir`; Err names the first failure.
fn write_files(dir: &Path, files: &[(Vec<String>, Vec<u8>)]) -> Result<(), String> {
    for (parts, bytes) in files {
        let path = parts.iter().fold(dir.to_path_buf(), |p, c| p.join(c));
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parts.join("/")))?;
        }
        vibe_core::fs::write_atomic(&path, bytes).map_err(|e| format!("{}: {e}", parts.join("/")))?;
    }
    Ok(())
}

/// `vibe-spec export --openspec <dir> [--json]`.
pub fn export(found: &Result<Root, Miss>, cwd: &Path, dir: &Path, json_out: bool) -> (u8, Vec<u8>) {
    let root = match found {
        Ok(root) => root,
        Err(miss) => {
            eprintln!("{}", miss.message());
            return (miss.code(), Vec::new());
        }
    };
    let shown = dir.to_string_lossy().trim_end_matches('/').to_string();
    let out_dir = cwd.join(dir);
    if out_dir.join("openspec").exists() {
        eprintln!("export: {shown}/openspec already exists");
        return (exit::FINDINGS, Vec::new());
    }
    let files = match plan_export(root) {
        Ok(f) => f,
        Err(errs) => {
            for e in errs {
                eprintln!("export: {e}");
            }
            return (exit::FINDINGS, Vec::new());
        }
    };
    if let Err(e) = write_files(&out_dir, &files) {
        eprintln!("export: {e}");
        return (exit::FINDINGS, Vec::new());
    }
    let written: Vec<String> = files.iter().map(|(p, _)| p.join("/")).collect();
    if json_out {
        let v = json!({ "v": 1, "direction": "export", "dir": shown, "written": written });
        return (exit::OK, format!("{v}\n").into_bytes());
    }
    let mut out = format!("EXPORTED {} file(s) to {shown}/openspec/\n", written.len());
    for w in &written {
        out += &format!("  {w}\n");
    }
    (exit::OK, out.into_bytes())
}
```

The export order is: capabilities first, then each change in byte order of its folder name (`notes` before `passkeys`). Within a change: delta spec or `.openspec.yaml`, then `proposal.md`, `tasks.md`, `design.md`. The first test's listing follows this order.

In `main.rs`, add `mod openspec;`, and remove the `#[allow(dead_code)]` above `mod delta;`. If clippy then reports an unused item, delete that item.

```rust
    /// Write the spec tree as an OpenSpec directory (`<DIR>/openspec/`).
    Export {
        /// Output directory; `openspec/` is created inside it.
        #[arg(long, value_name = "DIR")]
        openspec: PathBuf,
        /// Print Spec JSON v1.
        #[arg(long)]
        json: bool,
    },
```

```rust
        Command::Export { openspec: dir, json } => openspec::export(&found, &cwd, &dir, json),
```

Contract. Add to `$defs` in `contracts/spec-json.schema.json`:

```json
    "openspec": {
      "type": "object",
      "required": ["v", "direction", "dir", "written"],
      "properties": {
        "v": { "const": 1 },
        "direction": { "enum": ["export", "import"] },
        "dir": { "type": "string", "minLength": 1 },
        "written": { "type": "array", "items": { "type": "string", "minLength": 1 } }
      }
    },
```

In `run.mjs`, add `'openspec'` to `QUERIES` and `openspec: ['dir', 'direction', 'v', 'written']` to `REQUIRED`.

Goldens:
- `valid/export.json`: `{"v":1,"direction":"export","dir":"out","written":["openspec/specs/product-auth/spec.md"]}`
- `valid/import.json`: `{"v":1,"direction":"import","dir":"out","written":[".spec/features/passkeys/product.md"]}`
- `valid/unknown-field.json`: `export.json` plus `"took_ms":2`
- `invalid/bad-direction.json`: `export.json` with `"direction":"both"`
- `invalid/missing-written.json`: `export.json` without `written`

- [ ] **Step 4: Run the tests and contracts**

Run: `cargo test -p vibe-spec --test openspec 2>&1 | tail -4 && bash contracts/run.sh 2>&1 | tail -2`
Expected: `4 passed`; contracts `OK`.

- [ ] **Step 5: Lint and commit**

Run: `cargo fmt --all && cargo clippy --workspace --all-targets -q -- -D warnings && cargo test -p vibe-spec -q 2>&1 | grep -E 'test result|FAILED'`

```bash
git add crates/vibe-spec/src/openspec.rs crates/vibe-spec/src/main.rs crates/vibe-spec/tests/openspec.rs contracts/ tests/openspec/fixture
git commit -m "feat(spec): export the spec tree as openspec"
```

### Task 9: `import --openspec` and the round trip

**Files:**
- Modify: `crates/vibe-spec/src/openspec.rs`, `crates/vibe-spec/src/text.rs` (`today_utc`, `civil`), `crates/vibe-spec/src/main.rs` (subcommand `Import`)
- Test: `crates/vibe-spec/tests/openspec.rs`, `text.rs` unit tests

**Interfaces:**
- Consumes: Task 8's `MARK`, `CARRIED` and the `delta` grammar.
- Produces:
  - `pub fn openspec::import(found: &Result<Root, Miss>, cwd: &Path, dir: &Path, only: Option<&str>, json: bool) -> (u8, Vec<u8>)`.
  - `pub fn text::today_utc() -> String`, `pub fn text::civil(days: i64) -> String`.

- [ ] **Step 1: Write the failing tests**

Add to `text.rs` tests:

```rust
    #[test]
    fn civil_dates() {
        assert_eq!(civil(0), "1970-01-01");
        assert_eq!(civil(10_957), "2000-01-01");
        assert_eq!(civil(19_723), "2024-01-01");
        assert_eq!(civil(19_782), "2024-02-29");
        assert!(is_date(today_utc().as_bytes()));
    }
```

Append to `tests/openspec.rs`:

```rust
const FILES: [&str; 3] = ["product.md", "tech.md", "plan.md"];

/// An empty project with a `.spec` root and no features.
fn empty(name: &str) -> PathBuf {
    let p = project(name);
    write(&p, ".spec/product.md", "# P\n");
    p
}

#[test]
fn round_trip_is_byte_identical() {
    let a = fixture("os-rt-a");
    assert_eq!(spec(&a, &["export", "--openspec", "out"], &[]).code, 0);
    let b = empty("os-rt-b");
    let out_dir = a.join("out");
    let out = spec(&b, &["import", "--openspec", out_dir.to_str().unwrap()], &[]);
    assert_eq!(out.code, 0, "{}", out.stderr);
    for f in FILES {
        let rel = format!(".spec/features/passkeys/{f}");
        assert_eq!(fs::read(b.join(&rel)).unwrap(), fs::read(a.join(&rel)).unwrap(), "{rel}");
    }
    assert!(!b.join(".spec/features/passkeys/design.md").exists());
    let json = spec(&empty("os-rt-json"), &["import", "--openspec", out_dir.to_str().unwrap(), "--json"], &[]);
    assert_spec_json("openspec", &json.text());
}

#[test]
fn import_refuses_edited_requirements() {
    let a = fixture("os-edit-a");
    assert_eq!(spec(&a, &["export", "--openspec", "out"], &[]).code, 0);
    let delta = a.join("out/openspec/changes/passkeys/specs/product-auth/spec.md");
    let text = fs::read_to_string(&delta).unwrap().replace("valid passkey", "valid key");
    fs::write(&delta, text).unwrap();
    let b = empty("os-edit-b");
    let out = spec(&b, &["import", "--openspec", a.join("out").to_str().unwrap()], &[]);
    assert_eq!(out.code, 1);
    assert_eq!(
        out.stderr,
        "import: passkeys: requirements were edited in OpenSpec; delete design.md's vibe blocks to import it as a new change\n"
    );
    assert!(!b.join(".spec/features/passkeys").exists());
}

#[test]
fn foreign_change_imports() {
    let src = project("os-foreign-src");
    write(&src, "openspec/changes/add-export/proposal.md", "# Proposal\n\n## Why\nUsers need their data.\n\n## What Changes\n- export\n");
    write(
        &src,
        "openspec/changes/add-export/specs/product-auth/spec.md",
        "## ADDED Requirements\n\n### Requirement: Export\nThe system SHALL export CSV.\n\n#### Scenario: csv\n- **WHEN** asked\n- **THEN** csv\n\n## REMOVED Requirements\n\n- `### Requirement: Legacy dump`\n",
    );
    write(&src, "openspec/changes/add-export/tasks.md", "# Tasks\n\n## 1. Export\n\n- [x] 1.1 Writer\n- [ ] 1.2 Button\n");
    let b = empty("os-foreign");
    write(&b, ".spec/product-auth.md", "# auth\n");
    let out = spec(&b, &["import", "--openspec", src.to_str().unwrap()], &[]);
    assert_eq!(out.code, 0, "{}", out.stderr);
    let product = read(&b, ".spec/features/add-export/product.md");
    let (head, body) = product.split_once("---\n\n").unwrap();
    assert!(head.starts_with("---\ntype: feature-product\nfeature: add-export\nsibling: tech.md\nparent: ../../product-auth.md\nupdated: "), "{head}");
    assert_eq!(
        body,
        "# Feature: add-export — Product\n\nUsers need their data.\n\n## ADDED Requirements\n\n### Requirement: Export\nThe system SHALL export CSV.\n\n#### Scenario: csv\n- **WHEN** asked\n- **THEN** csv\n\n## REMOVED Requirements\n\n### Requirement: Legacy dump\n"
    );
    let plan = read(&b, ".spec/features/add-export/plan.md");
    assert!(plan.contains("\n### add-export/1 — Writer\n\n### add-export/2 — Button\n"), "{plan}");
    assert!(plan.ends_with("| add-export/1 | DONE |\n| add-export/2 | NOT STARTED |\n"), "{plan}");
    assert!(b.join(".spec/features/add-export/tech.md").is_file());
}

#[test]
fn import_refuses() {
    let a = fixture("os-refuse-a");
    assert_eq!(spec(&a, &["export", "--openspec", "out"], &[]).code, 0);
    let out_dir = a.join("out");
    let exists = spec(&a, &["import", "--openspec", out_dir.to_str().unwrap()], &[]);
    assert_eq!(exists.code, 1);
    assert_eq!(
        exists.stderr,
        "import: notes: .spec/features/notes/ already exists\nimport: passkeys: .spec/features/passkeys/ already exists\n"
    );
    let src = project("os-refuse-src");
    write(&src, "openspec/changes/two-caps/specs/a/spec.md", "## ADDED Requirements\n### Requirement: A\nThe system SHALL a.\n#### Scenario: a\n- x\n");
    write(&src, "openspec/changes/two-caps/specs/b/spec.md", "## ADDED Requirements\n### Requirement: B\nThe system SHALL b.\n#### Scenario: b\n- x\n");
    write(&src, "openspec/changes/renamed/specs/a/spec.md", "## RENAMED Requirements\n- FROM: `### Requirement: A`\n- TO: `### Requirement: B`\n");
    write(&src, "openspec/changes/Bad_Id/proposal.md", "# P\n");
    let out = spec(&empty("os-refuse-b"), &["import", "--openspec", src.to_str().unwrap()], &[]);
    assert_eq!(out.code, 1);
    assert_eq!(
        out.stderr,
        "import: Bad_Id: not a valid feature name (lowercase-kebab)\n\
         import: renamed: RENAMED Requirements is not supported\n\
         import: two-caps: touches 2 capabilities; a vibe feature targets one document\n"
    );
    let one = spec(&empty("os-refuse-one"), &["import", "--openspec", src.to_str().unwrap(), "--feature", "nope"], &[]);
    assert_eq!((one.code, one.stderr.as_str()), (1, "import: no change 'nope' to import\n"));
}
```

- [ ] **Step 2: Run them to verify they fail**

Run: `cargo test -p vibe-spec --test openspec 2>&1 | tail -6; cargo test -p vibe-spec --bin vibe-spec civil 2>&1 | tail -3`
Expected: FAIL. `unrecognized subcommand 'import'`, and `civil` is not found.

- [ ] **Step 3: Implement**

`text.rs`:

```rust
/// Today's UTC date as `YYYY-MM-DD`, from the system clock (P3c-R18).
pub fn today_utc() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    civil(i64::try_from(secs / 86_400).unwrap_or(0))
}

/// Days since 1970-01-01 as `YYYY-MM-DD` (Howard Hinnant's civil_from_days).
pub fn civil(days: i64) -> String {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02}")
}
```

`openspec.rs`. Add the import side:

```rust
/// The carried files of an exported design.md; None without the marker.
fn uncarry(design: &[u8]) -> Option<Result<Vec<(String, Vec<u8>)>, String>> {
    let at = crate::text::find(design, MARK.as_bytes())?;
    let mut rest = design.get(at + MARK.len()..).unwrap_or_default();
    let mut files = Vec::new();
    while let Some(h) = crate::text::find(rest, b"\n## vibe:") {
        let parsed = (|| {
            let after = rest.get(h + 9..)?;
            let eol = after.iter().position(|&c| c == b'\n')?;
            let header = lossy(after.get(..eol)?);
            let (name, len) = header.rsplit_once(' ')?;
            let len: usize = len.parse().ok()?;
            let body = after.get(eol + 1..)?.strip_prefix(b"\n")?;
            let fence_end = body.iter().position(|&c| c == b'\n')?;
            let content = body.get(fence_end + 1..fence_end + 1 + len)?;
            Some((name.to_string(), content.to_vec(), body.get(fence_end + 1 + len..)?))
        })();
        let Some((name, content, next)) = parsed else {
            return Some(Err("design.md has a malformed vibe block".to_string()));
        };
        if !CARRIED.contains(&name.as_str()) {
            return Some(Err(format!("design.md carries '{name}', which is not a vibe feature file")));
        }
        files.push((name, content));
        rest = next;
    }
    Some(Ok(files))
}

/// Requirement blocks per kind: ADDED (plain sections count as ADDED),
/// MODIFIED, REMOVED. Each block ends in a newline; REMOVED bullets
/// (`- \`### Requirement: X\``) become headers.
fn by_kind(text: &[u8]) -> [Vec<Vec<u8>>; 3] {
    let mut out: [Vec<Vec<u8>>; 3] = Default::default();
    for sec in delta::sections(text) {
        let slot = match sec.kind {
            Kind::Plain | Kind::Added => 0,
            Kind::Modified => 1,
            Kind::Removed => 2,
            Kind::Renamed => continue,
        };
        for b in &sec.blocks {
            let mut bytes = text.get(b.start..b.end).unwrap_or_default().to_vec();
            if !bytes.ends_with(b"\n") {
                bytes.push(b'\n');
            }
            out[slot].push(bytes);
        }
        if sec.kind == Kind::Removed {
            for line in lines(text.get(sec.header..sec.end).unwrap_or_default()) {
                let t = lossy(line);
                let t = t.trim().trim_start_matches(['-', '*', '+']).trim().trim_matches('`');
                if let Some(name) = delta::requirement_name(t.as_bytes()).filter(|_| !line.starts_with(b"###")) {
                    out[slot].push(cat!("### Requirement: ", name, "\n"));
                }
            }
        }
    }
    out
}

/// The body of `## <title>` (case-insensitive) up to the next heading.
fn section_text(md: &[u8], title: &str) -> Option<String> {
    let mut inside = false;
    let mut body: Vec<String> = Vec::new();
    for line in lines(md) {
        let t = lossy(line);
        if let Some(h) = t.strip_prefix("## ") {
            if inside {
                break;
            }
            inside = h.trim().eq_ignore_ascii_case(title);
            continue;
        }
        if inside && t.starts_with('#') {
            break;
        }
        if inside {
            body.push(t);
        }
    }
    let text = body.join("\n").trim().to_string();
    (!text.is_empty()).then_some(text)
}

/// plan.md units from tasks.md checkbox lines (`- [x] 1.1 Title`).
fn plan_from_tasks(tasks: &[u8], id: &str, today: &str) -> Option<Vec<u8>> {
    let mut units: Vec<(String, bool)> = Vec::new();
    for line in lines(tasks) {
        let t = lossy(line);
        let t = t.trim_start();
        let Some(rest) = ["- [", "* [", "+ ["].iter().find_map(|p| t.strip_prefix(p)) else {
            continue;
        };
        let Some((mark, title)) = rest.split_once(']') else {
            continue;
        };
        let title = title.trim();
        let title = match title.split_once(' ') {
            Some((num, rest)) if num.starts_with(|c: char| c.is_ascii_digit()) && num.contains('.') => rest.trim(),
            _ => title,
        };
        units.push((title.to_string(), mark.trim().eq_ignore_ascii_case("x")));
    }
    if units.is_empty() {
        return None;
    }
    let mut out = format!(
        "---\ntype: feature-plan\nfeature: {id}\nparent: ../../plan.md\nupdated: {today}\n---\n\n# Feature: {id} — Plan\n\n## Requirements Trace\n\n| ID | Requirement | Units |\n|---|---|---|\n\n## Units\n"
    );
    for (i, (title, _)) in units.iter().enumerate() {
        out += &format!("\n### {id}/{} — {title}\n", i + 1);
    }
    out += "\n## Progress\n\n| Unit | Status |\n|---|---|\n";
    for (i, (_, d)) in units.iter().enumerate() {
        out += &format!("| {id}/{} | {} |\n", i + 1, if *d { "DONE" } else { "NOT STARTED" });
    }
    Some(out.into_bytes())
}

/// One OpenSpec change as files under `<root>/features/<id>/` (P3c-R15).
fn import_change(root: &Root, cdir: &Path, id: &str) -> Result<Vec<(PathBuf, Vec<u8>)>, String> {
    if !valid_name(id) {
        return Err("not a valid feature name (lowercase-kebab)".to_string());
    }
    let fdir = root.dir.join("features").join(id);
    if fdir.exists() {
        return Err(format!("{}/features/{id}/ already exists", root.display()));
    }
    let specs: Vec<String> = walk_files(&cdir.join("specs"))
        .into_iter()
        .filter(|r| r.ends_with("/spec.md"))
        .collect();
    if specs.len() > 1 {
        return Err(format!("touches {} capabilities; a vibe feature targets one document", specs.len()));
    }
    let cap = specs.first().map(|r| r.trim_end_matches("/spec.md").to_string());
    let delta_text = specs
        .first()
        .map(|r| std::fs::read(cdir.join("specs").join(r)).unwrap_or_default())
        .unwrap_or_default();
    if delta::sections(&delta_text).iter().any(|s| s.kind == Kind::Renamed) {
        return Err("RENAMED Requirements is not supported".to_string());
    }
    let kinds = by_kind(&delta_text);
    let design = std::fs::read(cdir.join("design.md")).unwrap_or_default();
    match uncarry(&design) {
        Some(Err(e)) => Err(e),
        Some(Ok(carried)) => {
            let product = carried
                .iter()
                .find(|(n, _)| n == "product.md")
                .map(|(_, b)| b.clone())
                .ok_or_else(|| "the carried files have no product.md".to_string())?;
            if by_kind(&product) != kinds {
                return Err("requirements were edited in OpenSpec; delete design.md's vibe blocks to import it as a new change".to_string());
            }
            Ok(carried.into_iter().map(|(n, b)| (fdir.join(n), b)).collect())
        }
        None => Ok(foreign(root, &fdir, id, cap, &kinds, cdir)),
    }
}

/// A change without vibe blocks: synthesized feature docs (P3c-R15).
fn foreign(root: &Root, fdir: &Path, id: &str, cap: Option<String>, kinds: &[Vec<Vec<u8>>; 3], cdir: &Path) -> Vec<(PathBuf, Vec<u8>)> {
    let today = crate::text::today_utc();
    let parent = cap
        .filter(|c| root.dir.join(format!("{c}.md")).is_file())
        .unwrap_or_else(|| "product".to_string());
    let why = section_text(&std::fs::read(cdir.join("proposal.md")).unwrap_or_default(), "why")
        .unwrap_or_else(|| format!("Imported from OpenSpec change {id}."));
    let mut product = format!(
        "---\ntype: feature-product\nfeature: {id}\nsibling: tech.md\nparent: ../../{parent}.md\nupdated: {today}\n---\n\n# Feature: {id} — Product\n\n{why}\n"
    )
    .into_bytes();
    for (kind, blocks) in [Kind::Added, Kind::Modified, Kind::Removed].iter().zip(kinds) {
        if blocks.is_empty() {
            continue;
        }
        product.extend(format!("\n## {} Requirements\n\n", kind.as_str()).into_bytes());
        product.extend(blocks.join(&b'\n'));
    }
    let tech = format!(
        "---\ntype: feature-tech\nfeature: {id}\nsibling: product.md\nparent: ../../tech.md\nupdated: {today}\n---\n\n# Feature: {id} — Tech\n\nImported from OpenSpec change {id}; the design is in design.md.\n"
    );
    let mut files = vec![(fdir.join("product.md"), product), (fdir.join("tech.md"), tech.into_bytes())];
    if let Ok(d) = std::fs::read(cdir.join("design.md")) {
        files.push((fdir.join("design.md"), d));
    }
    if let Some(plan) = plan_from_tasks(&std::fs::read(cdir.join("tasks.md")).unwrap_or_default(), id, &today) {
        files.push((fdir.join("plan.md"), plan));
    }
    files
}

/// `vibe-spec import --openspec <dir> [--feature <name>] [--json]`.
pub fn import(found: &Result<Root, Miss>, cwd: &Path, dir: &Path, only: Option<&str>, json_out: bool) -> (u8, Vec<u8>) {
    let root = match found {
        Ok(root) => root,
        Err(miss) => {
            eprintln!("{}", miss.message());
            return (miss.code(), Vec::new());
        }
    };
    let shown = dir.to_string_lossy().trim_end_matches('/').to_string();
    let os = cwd.join(dir).join("openspec");
    if !os.is_dir() {
        eprintln!("import: no openspec/ under {shown}");
        return (exit::FINDINGS, Vec::new());
    }
    let ids: Vec<String> = sub_dirs(&os.join("changes"))
        .iter()
        .map(|d| name_of(d))
        .filter(|n| n.as_str() != "archive" && only.is_none_or(|o| n.as_str() == o))
        .collect();
    if ids.is_empty() {
        let which = only.map(|o| format!(" '{o}'")).unwrap_or_default();
        eprintln!("import: no change{which} to import");
        return (exit::FINDINGS, Vec::new());
    }
    let mut files: Vec<(PathBuf, Vec<u8>)> = Vec::new();
    let mut errs: Vec<String> = Vec::new();
    for id in &ids {
        match import_change(root, &os.join("changes").join(id), id) {
            Ok(mut f) => files.append(&mut f),
            Err(e) => errs.push(format!("import: {id}: {e}")),
        }
    }
    if !errs.is_empty() {
        for e in errs {
            eprintln!("{e}");
        }
        return (exit::FINDINGS, Vec::new());
    }
    let d = root.display();
    let mut written = Vec::new();
    for (path, bytes) in &files {
        let rel = format!("{d}/features/{}/{}", name_of(path.parent().unwrap_or(path)), name_of(path));
        if let Some(parent) = path.parent() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                eprintln!("import: {rel}: {e}");
                return (exit::FINDINGS, Vec::new());
            }
        }
        if let Err(e) = vibe_core::fs::write_atomic(path, bytes) {
            eprintln!("import: {rel}: {e}");
            return (exit::FINDINGS, Vec::new());
        }
        written.push(rel);
    }
    if json_out {
        let v = json!({ "v": 1, "direction": "import", "dir": shown, "written": written });
        return (exit::OK, format!("{v}\n").into_bytes());
    }
    let mut out = format!("IMPORTED {} file(s) from {shown}/openspec/\n", written.len());
    for w in &written {
        out += &format!("  {w}\n");
    }
    (exit::OK, out.into_bytes())
}
```

`sub_dirs` returns entries in byte order, so `Bad_Id` (uppercase B) sorts before `renamed` and `two-caps`, matching the test's stderr order.

In `main.rs`:

```rust
    /// Import OpenSpec changes as fallback feature folders.
    Import {
        /// Directory holding `openspec/`.
        #[arg(long, value_name = "DIR")]
        openspec: PathBuf,
        /// Import only this change.
        #[arg(long)]
        feature: Option<String>,
        /// Print Spec JSON v1.
        #[arg(long)]
        json: bool,
    },
```

```rust
        Command::Import { openspec: dir, feature, json } => {
            openspec::import(&found, &cwd, &dir, feature.as_deref(), json)
        }
```

- [ ] **Step 4: Run the tests**

Run: `cargo test -p vibe-spec --test openspec 2>&1 | tail -4 && cargo test -p vibe-spec --bin vibe-spec civil 2>&1 | tail -2`
Expected: `8 passed`; `1 passed`. `round_trip_is_byte_identical` is the primary signal (P3c-R16). If it fails, diff the two files with `cmp -l` before changing either side.

- [ ] **Step 5: Lint and commit**

Run: `cargo fmt --all && cargo clippy --workspace --all-targets -q -- -D warnings && cargo test --workspace -q 2>&1 | grep -E 'test result|FAILED'`

```bash
git add crates/vibe-spec/src/openspec.rs crates/vibe-spec/src/text.rs crates/vibe-spec/src/main.rs crates/vibe-spec/tests/openspec.rs
git commit -m "feat(spec): import openspec changes"
```

### Task 10: The OpenSpec validate gate

**Files:**
- Create: `tests/openspec/run.sh`, `tests/openspec/check.mjs`
- Modify: `.github/workflows/ci.yml` (new job `openspec`)

**Interfaces:**
- Consumes: `vibe-spec export --openspec` (Task 8), `tests/openspec/fixture/` (Task 8).
- Produces: the F18 acceptance evidence "`openspec validate` (external tool) passes on an exported dir", plus a mutant that must fail.

- [ ] **Step 1: Write the gate script**

The JavaScript lives in its own file. CI's shellcheck reports `${…}` inside single quotes (SC2016, info), and today an info finding fails the job.

Create `tests/openspec/check.mjs`:

```js
// tests/openspec/check.mjs — helpers for tests/openspec/run.sh (F18).
//   node check.mjs count <report.json> <n>   fail unless n items ran and n passed
//   node check.mjs mutate <file> <from> <to> replace <from> once; fail if absent
import { readFileSync, writeFileSync } from 'node:fs';

const [cmd, file, a, b] = process.argv.slice(2);
if (cmd === 'count') {
  const t = JSON.parse(readFileSync(file, 'utf8')).summary.totals;
  const want = Number(a);
  if (t.items !== want || t.passed !== want) {
    console.error(`openspec: expected ${want} items passed, got ${t.items} items, ${t.passed} passed`);
    process.exit(1);
  }
} else if (cmd === 'mutate') {
  const text = readFileSync(file, 'utf8');
  if (!text.includes(a)) {
    console.error(`openspec: ${file} does not contain ${JSON.stringify(a)}`);
    process.exit(1);
  }
  writeFileSync(file, text.replace(a, b));
} else {
  console.error('usage: node check.mjs count <report> <n> | mutate <file> <from> <to>');
  process.exit(2);
}
```

Create `tests/openspec/run.sh` (mode 755):

```bash
#!/usr/bin/env bash
# tests/openspec/run.sh — F18 gate (P3c-R17): OpenSpec's own validator
# accepts vibe-spec's export of the fixture and rejects a broken export.
# Needs node >= 20.19 and network: npx fetches the pinned CLI.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
OPENSPEC="@fission-ai/openspec@1.14.2"
# 1 capability (product-auth) + 2 changes (notes: skip_specs; passkeys).
# A run that examined fewer items is not evidence: validate prints
# "No items found" and exits 0.
EXPECTED_ITEMS=3

cargo build --locked -q -p vibe-spec --manifest-path "$REPO_ROOT/Cargo.toml"
BIN="$REPO_ROOT/target/debug/vibe-spec"
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

# validate <case>: openspec's JSON report on stdout; returns its exit code.
validate() {
  (cd "$work/$1/out" && OPENSPEC_TELEMETRY=0 npx -y "$OPENSPEC" validate --all --strict --no-interactive --json)
}

# 1. The fixture's export validates, and the run examined every item.
cp -R "$REPO_ROOT/tests/openspec/fixture" "$work/good"
(cd "$work/good" && "$BIN" export --openspec out >/dev/null)
if ! validate good >"$work/good.json"; then
  cat "$work/good.json"
  echo "openspec: the fixture's export failed validate --strict" >&2
  exit 1
fi
node "$REPO_ROOT/tests/openspec/check.mjs" count "$work/good.json" "$EXPECTED_ITEMS"

# 2. A requirement without SHALL/MUST must fail: the gate discriminates.
cp -R "$REPO_ROOT/tests/openspec/fixture" "$work/bad"
node "$REPO_ROOT/tests/openspec/check.mjs" mutate "$work/bad/.spec/product-auth.md" "The system SHALL end" "The system ends"
(cd "$work/bad" && "$BIN" export --openspec out >/dev/null)
if validate bad >"$work/bad.json"; then
  echo "openspec: a requirement without SHALL/MUST passed validate --strict" >&2
  exit 1
fi

echo "openspec: the export validates ($EXPECTED_ITEMS items); a broken export fails"
```

- [ ] **Step 2: Run it**

Run (alone): `bash tests/openspec/run.sh; echo "rc=$?"`
Expected: `openspec: the export validates (3 items); a broken export fails` and `rc=0`. If step 1 fails, the printed JSON names the rule and its exact message. Fix the exporter or the fixture so the fixture states a valid spec. Never loosen `--strict` or the item count.

Run: `shellcheck tests/openspec/run.sh`
Expected: no output.

- [ ] **Step 3: Add the CI job**

Append a job to `.github/workflows/ci.yml`, after `parity`:

```yaml
  openspec:
    # F18 (P3c-R17): OpenSpec's own validator accepts vibe-spec's export of
    # tests/openspec/fixture and rejects a broken one. Pinned CLI via npx.
    name: openspec — validate an export
    runs-on: ubuntu-latest
    steps:
      - name: Checkout
        uses: actions/checkout@v4
        with:
          persist-credentials: false

      - name: Setup Node
        uses: actions/setup-node@v4
        with:
          node-version: '20'

      - name: Toolchain (pinned by rust-toolchain.toml)
        run: rustup toolchain install

      - name: OpenSpec gate
        run: bash tests/openspec/run.sh
```

`node-version: '20'` resolves to the newest 20.x, which is ≥ 20.19. No `package.json` is added, so the zero-dependency check stays green.

- [ ] **Step 4: Verify the workflow parses and the suites stay green**

Run: `node -e 'require("fs").readFileSync(".github/workflows/ci.yml","utf8")' && git ls-files '*.sh' | grep -Ev '^oracles/(bash|js)/' | xargs shellcheck && echo SC-OK`
Expected: `SC-OK`.

- [ ] **Step 5: Commit**

```bash
git add tests/openspec/run.sh tests/openspec/check.mjs .github/workflows/ci.yml
git commit -m "test(spec): gate exports on openspec validate"
```

### Task 11: Docs, lessons and the phase gate

**Files:**
- Modify: `.spec/tech-spec.md` ("OpenSpec interchange", "Shipped today", the command list)
- Modify: `.spec/tech-contracts.md` (the Spec JSON v1 query list)
- Modify: `.spec/tech-rust.md:24`
- Modify: `.spec/plan.md` (row F18 status)
- Modify: `.spec/lessons.md`, `AGENTS.md` (regenerated active rules)
- Modify: `CHANGELOG.md`, `README.md` (the `export|import --openspec` lines near 228)

**Interfaces:**
- Consumes: everything above.
- Produces: docs that state today's truth, and the F18 row at `IN PROGRESS — local gate green; CI pending push`.

- [ ] **Step 1: Enter the strategy state**

Run: `bash .agents/skills/vibe/scripts/set-state.sh strategy.spec`
Expected: `-> strategy.spec`.

- [ ] **Step 2: Update the root specs**

`.spec/tech-spec.md`, "OpenSpec interchange". Keep the grammar paragraph. Replace the mapping table and the paragraph after it with:

```markdown
| vibe | OpenSpec |
|---|---|
| a root document with blocks under `## Requirements` (`product-auth.md`, `concepts/invoice.md`) | `openspec/specs/<path without .md>/spec.md`; `## Purpose` is generated |
| fallback feature `product.md` | `changes/<f>/proposal.md` + one delta spec for its target's capability (plain `## Requirements` blocks export as ADDED) |
| feature `plan.md` units | `changes/<f>/tasks.md` (`- [ ] 1.<n> <title>`, `[x]` when DONE) |
| every feature file | carried byte for byte in `changes/<f>/design.md` as `## vibe:<file> <len>` blocks after `<!-- vibe:export v1 -->` |

Requirements are keyed by their full header name, normalized as OpenSpec
does (closing `#` run dropped, trimmed, case-sensitive). `RENAMED` is not
supported: write REMOVED + ADDED. A REMOVED block carries a `**Reason**:`
line. `promote <f>` applies a fallback feature's delta sections to its
`parent:` document (REMOVED, then MODIFIED, then ADDED; all or nothing).

`vibe-spec export --openspec <dir>` refuses an existing `<dir>/openspec/`
and a MODIFIED that drops a scenario its target still has. `import
--openspec <dir> [--feature <name>]` writes fallback feature folders only,
never root documents. It restores carried files byte for byte, but refuses
when the change's requirements were edited on the OpenSpec side. It refuses
`RENAMED` and changes that touch more than one capability. A change without
carried files is imported as a new feature. Round trip: export → import is
byte-identical. `openspec validate --strict` runs on the exported directory
in CI (`tests/openspec/run.sh`), never on the tree itself.
```

`.spec/tech-spec.md`, "Shipped today". State that `check-drift`, `promote`, `export` and `import` exist in `vibe-spec` with byte parity for the two ports. State that skills, CI and flow still call `spec/scripts/*.sh` until F21–F22 (P3c-R2). In the command list, write `check-drift [--pre-merge] [--json]`, `promote <feature> [--dry-run] [--target <file>] [--done] [--base <ref>] [--json]` and `export|import --openspec <dir>`.

`.spec/tech-contracts.md`: the query list becomes `root`, `lessons-for`, `plan`, `feature`, `validate`, `trace`, `check-drift`, `promote`, `openspec`. Note that `promote` and `import` write files and that their JSON reports what was written.

`.spec/tech-rust.md:24`: "`init` stays self-check only until F20."

`.spec/plan.md`: set the F18 row's status cell to `IN PROGRESS — local gate green; CI pending push`.

- [ ] **Step 3: Lessons and active rules**

Add one lesson to `.spec/lessons.md` for each failed command, reverted change or late-found defect during this plan, in the file's format (`### title`, `**Pattern:**`, `**Rule:**`, `**Tags:**`, `**Date:** 2026-10-16` or the day it happened). If none happened, add none. Then run `bash flow/scripts/regen-active-rules.sh`.

- [ ] **Step 4: Changelog and README**

`CHANGELOG.md`, under the unreleased heading:

```markdown
- `vibe-spec check-drift [--pre-merge]` and `vibe-spec promote` ported with byte parity (F18). `promote` applies `## ADDED|MODIFIED|REMOVED Requirements` deltas, prints the superpowers promotion checklist, and `promote <feature> --done` deletes the artifacts once the spec changed since the branch base.
- `vibe-spec export|import --openspec <dir>`: OpenSpec interchange; exports pass `openspec validate --strict` (CI job `openspec`).
```

`README.md`: check the `export|import --openspec` lines (near line 228) against Step 2. Fix any claim that is no longer true.

- [ ] **Step 5: Leave the strategy state and run the phase gate**

Run: `bash .agents/skills/vibe/scripts/set-state.sh idle`

Run each command alone and show the output:

```bash
cargo fmt --all --check && cargo clippy --workspace --all-targets -q -- -D warnings
cargo test --workspace --locked 2>&1 | grep -E 'test result|FAILED'
bash contracts/run.sh
bash tests/parity/run.sh
bash tests/openspec/run.sh
bash tests/run.sh
bash .agents/skills/spec/scripts/validate.sh
bash spec/scripts/check-drift.sh
git ls-files '*.sh' | grep -Ev '^oracles/(bash|js)/' | xargs shellcheck
```

Expected:
- clippy and fmt print nothing; every `test result: ok`;
- contracts `OK`;
- parity `0 failed` with `live (4) green`;
- the openspec line from Task 10;
- `tests/run.sh` suites PASS. The doctor perf budget may fail only in a working copy with large ignored dirs (P3b-R14); confirm it with a clean-checkout timing before you report it;
- `validate.sh` `Errors: 0`;
- `check-drift: OK`.

- [ ] **Step 6: Commit**

```bash
git add .spec/ AGENTS.md CHANGELOG.md README.md
git commit -m "docs(spec): record f18 spec delta"
```

---

## Self-Review

1. **Spec coverage.**

   | Requirement (spec §5, §7, §9; issue #35) | Tasks |
   |---|---|
   | ADDED appends | 1, 6 |
   | MODIFIED replaces in place, byte-exact | 1, 6 |
   | REMOVED needs a why-line | 1, 2, 6 |
   | Golden trees | 1, 6 |
   | `promote.sh` port | 5 |
   | Superpowers checklist | 7 |
   | `--done` refuses on unchanged spec | 7 (discriminating mutant check) |
   | `check-drift` port | 3 |
   | `--pre-merge` | 4 |
   | `export\|import --openspec` | 8, 9 |
   | Round-trip byte identity | 9 |
   | `openspec validate` on an export | 10 |
   | `--json` on every new command | 4, 7, 8, 9 |
   | `bash tests/run.sh` and `cargo test --workspace` green | 11 |

   Wiring `--pre-merge` into flow compound and `finishing-a-development-branch` is deferred by P3c-R2.
2. **Placeholder scan.** No TBD or TODO. Task 7's JSON assembly is described field by field, because it threads values already computed in `run`. Every field name matches the schema in the same task.
3. **Type consistency.** `delta::{Kind, Block, Section, Change}` are used with the same fields in Tasks 1, 6, 8 and 9. `drift::run` gains `&Options` in Task 4, and `main.rs` is updated in the same task. `promote::run(found, cwd, args)` keeps one signature from Task 5 through Task 7. `openspec::{export, import}` match their `main.rs` dispatch lines.
4. **Review Focus.** Each of the five lines names its pinning test in its owning task. Items 1, 2 and 4 are inputs no other task exercises. Item 3 is spec §9's discriminating test.
