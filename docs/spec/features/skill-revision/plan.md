---
type: feature-plan
feature: skill-revision
sibling: tech.md
parent: ../../plan.md
updated: 2026-11-26
---

# Feature: Skill Revision — Implementation Plan

Thirty-one units: one setup unit, five parallel tracks with disjoint files, one
conditional tail unit that waits for the review-hardening port, and three closing
units (consistency sweep, final verification, compound). Tracks A–E start
together from the merge of unit 1. Every unit starts with a test, a pressure run
or a selection eval that fails first, or it says why its check is a regression
control.

> For agentic workers: REQUIRED SUB-SKILL: use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans. One implementer per track, each in its own worktree. Read Global Constraints, Worktrees and cursors, and Tracks before any unit. Audit IDs (`spec:N-1`, `flow:V-02`, `tool:U1`, `cons:C-03`, `mig:M-02`, `rubric:B6`) are for traceability; every unit carries its own fix text, and [tech.md](tech.md) holds the exact texts.

**Parent:** [../../plan.md](../../plan.md)
**Requirements:** [product.md](product.md)
**Architecture:** [tech.md](tech.md)

**Feature gate:** none in the F1–F25 sequence. This is branch work. The root row lands in unit 1, in `strategy.spec`, before any track starts.

**Feature layer:** this feature uses the fallback folder `docs/spec/features/skill-revision/` because the owner asked for the docs/spec feature format. That is the "your human partner says *self*" fallback trigger.

---

## Unit Status

The live status, reconciled against the tree on 2026-11-23 (issue #103).
Unit bodies below are the historical record; this table is what `vibe-spec plan --feature skill-revision` answers.

| Unit | Status | Basis |
|---|---|---|
| skill-revision/1 | DONE | baseline and lint suite landed; `tests/skills/run.sh` runs five case files |
| skill-revision/2 | DONE | v0.4, Track A, units 2–7 (CHANGELOG 0.4.0) |
| skill-revision/3 | DONE | v0.4, Track A, units 2–7 (CHANGELOG 0.4.0) |
| skill-revision/4 | DONE | v0.4, Track A, units 2–7 (CHANGELOG 0.4.0) |
| skill-revision/5 | DONE | v0.4, Track A, units 2–7 (CHANGELOG 0.4.0) |
| skill-revision/6 | DONE | v0.4, Track A, units 2–7 (CHANGELOG 0.4.0) |
| skill-revision/7 | DONE | v0.4, Track A, units 2–7 (CHANGELOG 0.4.0) |
| skill-revision/8 | DONE | v0.4, Track B, units 8–11 (CHANGELOG 0.4.0) |
| skill-revision/9 | DONE | v0.4, Track B, units 8–11 (CHANGELOG 0.4.0) |
| skill-revision/10 | DONE | v0.4, Track B, units 8–11 (CHANGELOG 0.4.0) |
| skill-revision/11 | DONE | v0.4, Track B, units 8–11 (CHANGELOG 0.4.0) |
| skill-revision/12 | DONE | v0.4, Track C, units 12–17 (CHANGELOG 0.4.0) |
| skill-revision/13 | DONE | v0.4, Track C, units 12–17 (CHANGELOG 0.4.0) |
| skill-revision/14 | DONE | v0.4, Track C, units 12–17 (CHANGELOG 0.4.0) |
| skill-revision/15 | DONE | v0.4, Track C, units 12–17 (CHANGELOG 0.4.0) |
| skill-revision/16 | DONE | v0.4, Track C, units 12–17 (CHANGELOG 0.4.0) |
| skill-revision/17 | DONE | v0.4, Track C, units 12–17 (CHANGELOG 0.4.0) |
| skill-revision/18 | NOT STARTED | REWRITTEN (#57) — live: `lint_description flow/SKILL.md` exits 1 with 4 violations; `flow/contract.md`, `SUBAGENT-STOP` and `## Entry` retired, no replacement |
| skill-revision/19 | NOT STARTED | REWRITTEN (owner ruling) — `<HARD-GATE>` and the Red Flags table have no home in the single-file skill; the surviving gate text is `## Contract`, which names both edges and `confirm` but never whose message approves, and never "end your turn" |
| skill-revision/20 | NOT STARTED | REWRITTEN (#57) — `## Transition (/flow)` is `## Contract` now; the three host command files still name 2 different writers |
| skill-revision/21 | NOT STARTED | REWRITTEN (#57) — target is the `<!-- vibe:doctrine -->` block; root-neutral half landed, `orders.sh` recovery open, provenance waits on unit 19 |
| skill-revision/22 | NOT STARTED | STILL REAL — 4 fixed `docs/spec` paths in the orders, compound names no `§ Wrap-up`, `quick.fix` writes no `tests/**` |
| skill-revision/23 | NOT STARTED | REWRITTEN (#57) — both files folded into `## feature` / `## strategy`; the gate's impl-mode choice is named in the machine and nowhere in the skill |
| skill-revision/24 | NOT STARTED | REWRITTEN (#57) — three files folded into `## verify` / `## quick` / `## compound`; no fix-loop cap, no `§ Wrap-up` pointer |
| skill-revision/25 | DONE | v0.4, Track E, units 25–27 (CHANGELOG 0.4.0) |
| skill-revision/26 | DONE | v0.4, Track E, units 25–27 (CHANGELOG 0.4.0) |
| skill-revision/27 | DONE | v0.4, Track E, units 25–27 (CHANGELOG 0.4.0) |
| skill-revision/28 | NOT STARTED | REWRITTEN (#57) — `flow/setup.md` folded into `## setup` + the two setup orders; no `doctor.sh`, ignore lines or merge WARN |
| skill-revision/29 | NOT STARTED | closeout; `tests/skills/cases/sweep.sh` does not exist |
| skill-revision/30 | NOT STARTED | closeout; no `flow/evidence/feature-skill-revision.md` receipt exists |
| skill-revision/31 | NOT STARTED | closeout; `docs/spec/plan.md` still carries this feature as IN PROGRESS |

## Reconciliation (#103)

Tracks A, B, C and E are delivered by v0.4 (#60) and their unit bodies stay as written.
Unit 1 shipped with them: `tests/skills/run.sh` exists and runs `docs.sh`, `setup.sh`,
`spec-docs.sh`, `spec-skill.sh` and `spec-tool.sh`.

Track D, the tail and the closeout were written against the six phase files that #57
replaced with one `flow/SKILL.md`; #96 then salvaged the v2 runtime core on top of it
(`vibe-flow state set`, `.vibe/run/flow/state.json`). The seven rows issue `#103` names —
18, 20–24 and 28 — were re-checked unit by unit against this tree on 2026-11-24. Every
one keeps at least one acceptance check that is red today, so no unit is retired whole:
each entry below names the retired *assumption* and the surviving check.

| Unit | Verdict | Evidence (observed, 2026-11-24) | Re-targeted to |
|---|---|---|---|
| 18 | REWRITTEN | `lint_description flow/SKILL.md` → rc 1, 4 violations: 620 chars max 500, angle bracket, first person, no "Use when". `ls flow/contract.md flow/tests/evals` → both absent | `flow/SKILL.md` frontmatter + new `tests/skills/cases/flow.sh`. The `contract.md` split and the `SUBAGENT-STOP` / `## Entry` / selection-eval shape are retired, with no replacement |
| 20 | REWRITTEN | `grep -ho 'vibe-flow state set\|set-state\.sh' .claude/commands/flow.md .opencode/commands/flow.md factory-plugin/commands/flow.md \| sort -u \| wc -l` → 2 | `flow/SKILL.md` `## Contract` (the transition heading it named is gone) + the three host command files |
| 21 | REWRITTEN | `bash flow/scripts/doctrine.sh \| grep -c orders.sh` → 0; the same output contains no `docs/spec` path, so root-neutrality already holds | The doctrine block in `flow/SKILL.md` and `flow/scripts/doctrine.sh`. The gate-provenance half waits on unit 19 |
| 22 | STILL REAL | `docs/spec` inside the orders blocks → 4; `Wrap-up` in the `feature.compound` order → 0; `tests/**` in the `quick.fix` order → 0 | Unchanged: `flow/SKILL.md` orders, `flow/state-machine.json`, `flow/tests/run.sh` |
| 23 | REWRITTEN | `ls flow/feature.md flow/strategy.md` → both absent; `grep -c 'impl mode' flow/SKILL.md` → 0 while `flow/state-machine.json` says the gate "picks impl mode" | `flow/SKILL.md` `## feature` / `## strategy` and the `feature.*` / `strategy.*` orders |
| 24 | REWRITTEN | `ls flow/{verify,quick,compound}.md` → all absent; `grep -c 'third return' flow/SKILL.md` → 0; `Wrap-up` → 0 | `flow/SKILL.md` `## verify` / `## quick` / `## compound` and their orders |
| 28 | REWRITTEN | `ls flow/setup.md` → absent; `grep -c 'doctor.sh' flow/SKILL.md` → 0 | `flow/SKILL.md` `## setup` and the `setup.detect` / `setup.apply` orders. Its tail trigger has fired: `review-hardening` is DONE in the root plan, so the unit is no longer conditional |

Assumptions retired without replacement, kept here so they are not re-derived:

- `flow/contract.md` (units 18, 21, 22, 23) — the single-file skill has `## Contract` and
  `## Delegation protocol`; no contract file ships and nothing reads one.
- `flow/{feature,strategy,verify,quick,compound,setup}.md` (units 23, 24, 28) — one
  `flow/SKILL.md` carries all six procedures.
- `flow/tests/evals/` and the selection-eval harness (unit 18) — no such directory exists.
- `flow/reference/deps.json` `required_by` (unit 22) — already `strategy.*`, `feature.*`,
  `quick.*`; nothing to change.
- `flow/reference/host-surfaces.json` `$comment` (unit 22) — it cites `tech.md`'s host
  surface contract, not `docs/spec/platform/adapter-contract.md`.
- The 13-state indent pin (unit 22) — `flow/state-machine.json` now has 12 state keys.
- `flow/feature.md`'s "create a todo for EACH numbered step" and the `announce:` / `COPY
  these` delegate-block grammar (unit 23) — the delegate-block format left the skill; the
  orders blocks are the format now.

Unit 19 is re-targeted by the owner's ruling, not by #103: `flow/SKILL.md` has no
`<HARD-GATE>` block and no Red Flags section, so the unit lands its three provenance
sentences in `## Contract` and leaves enforcement to the writer (F7,
[D27](../../decisions/d27-the-writer-is-the-gate.md)). Units 29–31 are the closeout and
are untouched by this reconciliation.

---

## Problem Frame

v0.4 moved the spec root to a layered `docs/spec/`, made `vibe-spec` the spec
tool, and added hooks. The skill text, the agents, the flow orders and the
repo docs did not follow. The results: a description that fails the skill
validator, a skill that teaches only the flat layout, prose that forks a legacy
root, a skill that cannot run without a binary no installer ships, three wrap-up
orders, a promoter agent that waits for a human it cannot reach, flow gates with
no teeth in the text, orders lost after compaction, and docs that name a
validator which cannot read this repo's root. The units fix the misleading text
and the untruthful tool first, then size and duplication. Each fix is pinned by
a lint, a suite assertion, a crate test, a parity divergence, or a recorded
pressure run.

---

## Requirements Trace

| ID | Requirement | Units |
|---|---|---|
| R1 | [Each skill triggers only on its own prompts](product.md#requirement-each-skill-triggers-only-on-its-own-prompts-r1) | skill-revision/1, skill-revision/3, skill-revision/6, skill-revision/7, skill-revision/18, skill-revision/20, skill-revision/29 |
| R2 | [Spec skill works on the root it finds](product.md#requirement-the-spec-skill-works-on-the-root-it-finds-r2) | skill-revision/5, skill-revision/8, skill-revision/9, skill-revision/10, skill-revision/21, skill-revision/22, skill-revision/29 |
| R3 | [Truth without the binary](product.md#requirement-the-spec-skill-tells-the-truth-without-the-binary-r3) | skill-revision/3, skill-revision/5, skill-revision/14 |
| R4 | [One wrap-up sequence](product.md#requirement-feature-wrap-up-has-one-sequence-r4) | skill-revision/6, skill-revision/8, skill-revision/9, skill-revision/16, skill-revision/22, skill-revision/24 |
| R5 | [Subagents without a human channel](product.md#requirement-spec-subagents-work-without-a-channel-to-the-human-r5) | skill-revision/6 |
| R6 | [One vibe-spec CLI contract](product.md#requirement-vibe-spec-keeps-one-command-line-contract-r6) | skill-revision/12, skill-revision/13, skill-revision/14, skill-revision/15, skill-revision/16, skill-revision/17 |
| R7 | [Docs-to-validator parity](product.md#requirement-output-written-as-the-docs-teach-passes-the-validator-r7) | skill-revision/9, skill-revision/10, skill-revision/15 |
| R8 | [Gates hold under pressure](product.md#requirement-flow-gates-hold-under-pressure-r8) | skill-revision/19, skill-revision/20, skill-revision/26 |
| R9 | [Orders survive compaction](product.md#requirement-flow-orders-survive-compaction-and-their-commands-run-r9) | skill-revision/21, skill-revision/22 |
| R10 | [One transition contract](product.md#requirement-one-transition-contract-on-every-host-r10) | skill-revision/20 |
| R11 | [Procedures agree with machine, policy, spec](product.md#requirement-flow-procedures-agree-with-the-machine-the-policy-and-the-spec-r11) | skill-revision/18, skill-revision/22, skill-revision/23, skill-revision/24, skill-revision/28 |
| R12 | [One delegation rule, neutral defaults](product.md#requirement-one-delegation-rule-and-neutral-shipped-defaults-r12) | skill-revision/5, skill-revision/18, skill-revision/25 |
| R13 | [Repo docs agree with the code](product.md#requirement-repo-docs-agree-with-the-code-r13) | skill-revision/11, skill-revision/26, skill-revision/27, skill-revision/29, skill-revision/31 |
| R14 | [Budgets](product.md#requirement-skill-bodies-stay-inside-their-budgets-r14) | skill-revision/1, skill-revision/3, skill-revision/5, skill-revision/18, skill-revision/29 |
| R15 | [Pressure evidence](product.md#requirement-discipline-changes-carry-pressure-test-evidence-r15) | skill-revision/1, skill-revision/2, skill-revision/4, skill-revision/7, skill-revision/19, skill-revision/30 |

---

## Key Technical Decisions

Each is our call. The ones the owner may overturn are repeated under Open
Questions with their defaults.

1. **No port-owned file is edited.** The port's files are listed in [product.md](product.md) Scope. Fixes that need them go to the cut list in [tech.md](tech.md), except `flow/setup.md` (tail unit 28, after the port's writer track merges). Shared files: `CHANGELOG.md` (own subsection), `docs/spec/plan.md` (one row in `## Branch work`; the first writer creates the section), `flow/tests/run.sh`, `flow/README.md` and `AGENTS.md` (see the overlap table).
2. **The spec skill keeps its pressure-tested assertive description.** Only the `<!-- literal -->` leak, the length and the root wording change. Any change re-runs the 12-run harness.
3. **Spec owns the wrap-up sequence.** Flow compound points to it and adds the digest, the receipt and `idle`. `finishing-a-development-branch` is last and git-only.
4. **Approvals live in the main thread.** `spec-promoter` becomes a non-interactive two-dispatch agent; the main thread runs `promote --done`.
5. **One config file.** `.vibe/spec.json` gets `featureLayer`, `suggest`, `superpowers`; a legacy `.config.yaml` is read only for a missing key; `vibe-flow` goes.
6. **Orders and doctrine stay in `flow/SKILL.md` (D12).** Moving them touches `content.mjs` (port). The rubric D2 deviation is recorded; the budget counts words outside the blocks.
7. **Gate fixes are prose plus provenance**, not code (F7 owns the writer gate). The level line keeps its `/flow … confirm` render; the HARD-GATE and the doctrine define who types `confirm`.
8. **vibe-spec diverges from the oracles where the oracle is wrong for users:** help, usage exit 2, no-root messages, whole tags. Each divergence is listed with its reason; `oracles/` never changes.
9. **The SessionStart router is silent without a spec root.** The pinned `no_root_still_routes` contradicts `using-spec`'s own trigger ("in a repo that keeps a spec"); the new test shows the router pushing `/spec setup` into a foreign repo.
10. **New lints live in a new `tests/skills` suite** with one case file per track. Existing wording pins are replaced in the unit that changes their text.
11. **Personal delegation policy leaves the shipped defaults** for this repo's root `vibe.json`.
12. **Row timing:** a fallback folder needs its `IN PROGRESS` row before design; the superpowers layer needs a row only at wrap-up. `IN PROGRESS` is the in-flight token `check-drift` reads (`drift.rs` `row_status` knows only `DONE` and `IN PROGRESS`).
13. **`--root` stays before the command.** No `global = true`: clap's `trailing_var_arg` on the raw-tail commands would swallow a later `--root` anyway.

---

## Global Constraints

- Never edit `oracles/**`, `contracts/fixtures/**`, `spec/scripts/**`, or any port-owned file ([product.md](product.md) Scope). Tail unit 28 is the only exception, for `flow/setup.md`.
- Bash: `set -euo pipefail`, shellcheck-clean, bash 3.2 safe (no `declare -A`, `mapfile`, `readlink -f`, `${var,,}`, `sed -i`).
- Rust: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo deny check` and `cargo test --workspace --locked` pass at the end of every Track C unit. Exit codes come from `vibe_core::exit` constants, never literals.
- Parity: every changed oracle-covered output gets one entry in `tests/parity/spec.json` `divergences` with `command`, `case`, `args`, `field` and a `reason` ending `(skill-revision/N)`. `bash tests/parity/run.sh` is green at the end of every Track C unit.
- Literal sweep: new shipped prose never spells the legacy root, so it needs no `<!-- literal -->` tag. `bash tests/literals/run.sh` stays green.
- Pins: before each unit, `grep -rnF '<string you change>' spec/tests flow/tests flow/engine/tests crates/vibe-spec/tests tests` and update every hit in the same unit. A wording pin is replaced by a behaviour or parity assertion, never re-pinned as a bare substring of new wording.
- The flow suite keeps: orders ≤400 bytes; `gate:` only in gate source states; every `set-state.sh <x>` target a legal next; doctrine pins listed in [tech.md](tech.md) § Doctrine; the template's hand-authored half names only `setup.apply` as a state.
- A pin hit in another track's function: the unit that changes the guarded text edits that assertion line in the same commit, and names the function in its Files list (known: `test_f25_voice`, a Track A function that reads `spec/feature.md`, `spec/strategy.md` and `spec/reference/*.md` and pins the exact line `Fallback feature flow. Run this only when the spec skill's Routing table sends you here.`; Track B units 8-10 edit it when they change that text). The owning track keeps every other line of the function.
- A pin hit in a port-owned file means: keep the string, never edit the file. Known: `flow/tests/adapters/run.sh` (the three `/flow` wrapper tokens, `skill=vibe · delegate executing-plans`) and `flow/engine/tests/content.test.mjs` (`delegation.workflows` in `agents-md`). Unit 1 re-runs the grep over every port-owned test for the strings in [tech.md](tech.md) § Orders and § Doctrine and records any new hit.
- Prose edits: one file at a time, in the unit's order; run the unit's case-file tests after each file (rubric M9). Discipline edits follow [tech.md](tech.md) § Pressure tests; the orchestrator runs the reps.
- Agent-facing prose follows [tech.md](tech.md) § Glossary.
- Every session that writes under `docs/spec/` or `docs/superpowers/{specs,plans}/` invokes `Skill(spec)` first, or the spec tooth blocks the write.
- Subagent models: implementers and per-unit reviewers `sonnet`; pressure and eval reps `sonnet` plus one `haiku`; the whole-branch review in unit 30 `opus`. Always set `model`.
- Shell changes (lesson "A new shell script runs the umbrella suite"): every unit that adds or edits a tracked `*.sh` runs, before its commit, `bash tests/run.sh` and the CI command `git ls-files '*.sh' | grep -Ev '^oracles/(bash|js)/' | xargs shellcheck` (after `git add -N` on new files).
- Plugin check: every unit that changes a shipped skill, agent or command file runs `bash spec/tests/plugin-validate.sh` (`claude plugin validate --strict`; it prints one `SKIP` line without `claude`, and the orchestrator then runs it).
- Records (plan approval message, application checks, byte tables, eval and pressure results that have no file of their own) go to `flow/evidence/skill-revision-records.md` and into the PR description. Never into a commit body.
- `CHANGELOG.md`: unit 1 adds `### skill-revision` under `## [Unreleased]` with one stub line per track. Each track rewrites only its own line in its last unit.
- Commits only on explicit request: `<type>(sr/N): <subject>`, lowercase, imperative, ≤50 characters, no body or footer. Test IDs use the full `skill-revision/N`.
- Test commands: skills `bash tests/skills/run.sh`; spec `bash spec/tests/run.sh`; agents `bash spec/tests/agents.sh`; flow `bash flow/tests/run.sh`; engine `node flow/engine/tests/run.mjs <filter>`; crate `cargo test -q --locked -p vibe-spec --test <file>`; all `bash tests/run.sh`. Grep a suite for a unit with `2>&1 | grep -E 'FAIL|skill-revision/N'`.

## Review Focus

Inputs the spec implies but no requirement scenario exercises. Each line has its
test in the named unit.

1. **Both roots present.** A repo holding `docs/spec/` and the legacy root: no shipped text tells the agent to create either; `list-specs` and `check-drift` act on `docs/spec/` (unit 14 control, unit 29 sweep).
2. **Tag spelling.** `**Tags:** CI , Decision\r` (case, spaces, CRLF): `lessons-for ci` returns it, `lessons-for de` does not (unit 15).
3. **Hostile `children:`.** A `plan.md` hub whose `children:` names `../outside.md`, a missing file, or a symlink to outside the root: `check-drift` reads none of them and does not crash (unit 16).
4. **`--help` later in the tail.** `vibe-spec promote foo --help` and `lessons-for bash -h` print usage and change nothing (unit 12).
5. **Malformed SessionStart stdin in a foreign repo.** Invalid JSON on stdin and no spec root under the cwd: `{}`, exit 0 (unit 17).

---

## Worktrees and cursors

- The orchestrator creates every worktree: `git worktree add -b sr/<name> "$SCRATCH/wt/sr-<name>" <base>`. Implementers never create or remove worktrees. Never `git stash`; set work aside with a WIP commit.
  - Unit 1: `sr/setup` from `villain/vibe` HEAD.
  - Tracks: `sr/a` … `sr/e` from the commit that merged unit 1.
  - Tail and close: `sr/close` from `villain/vibe` HEAD after the last track merge.
- The main worktree's cursor (`flow/state.json`, gitignored) is the port's (`feature.impl` / `review-hardening`). This plan never reads it for writing and never moves it.
- Every implementer session runs rooted in its worktree (`cd "$WT"` before starting the session, so `CLAUDE_PROJECT_DIR=$WT`). The hooks then judge writes against that worktree's cursor. A new worktree has no cursor, which means `idle`.
- `sr/setup` (unit 1): `bash flow/scripts/set-state.sh strategy.brainstorm`, then `strategy.spec skill-revision`. Confirm `bash flow/scripts/detect-context.sh decide docs/spec/plan.md` prints `allow`. After the commit, `set-state.sh idle`.
- Tracks run in `idle`. In `idle` the policy allows `docs/spec/features/*`, warns on `src/*`, `tests/*`, `AGENTS.md`, and blocks the hubs and `lessons.md`. No track writes a hub, `lessons.md` or the feature folder.
- `sr/close`: `idle → feature.design skill-revision → feature.plan → feature.impl` (the plan gate: the orchestrator adds `confirm` only because your human partner approved this plan in chat; record the approval message in `flow/evidence/skill-revision-records.md` and the PR) → unit 29 and the tail run in `feature.impl` → `feature.verify` (unit 30) → ship gate: show the unit-30 receipt and stop; after your human partner approves in chat, `feature.compound` (unit 31) → `idle`.
- Merging: the orchestrator rebases a track branch onto current `villain/vibe` (the port keeps committing there), runs the track's suites, then merges. On a conflict in a hunk the track does not own, keep the other side.
- Track D gate (decidable, **satisfied**): it waited for the port's `rh/(4|7|19)` commits, which edit `flow/tests/run.sh` blocks Track D rebases over. `review-hardening` is DONE in the root plan, so the gate is open and Track D is ungated.

---

## Tracks and overlap

| Track | Units | Files owned |
|---|---|---|
| 0 setup | 1 | `tests/skills/{run.sh,lib.sh}`, `tests/skills/cases/setup.sh`, `tests/run.sh` (suite list), `tests/evals/listing.md`, `docs/spec/features/skill-revision/{product,tech,plan}.md`, `docs/spec/plan.md` (one row; the section if absent), `CHANGELOG.md` (subsection stub) |
| A spec skill | 2 → 3 → 4 → 5 → 6 → 7 | `spec/SKILL.md`, `spec/using-spec/SKILL.md`, `spec/agents/*.md`, `spec/reference/{setup,layered}.md` (new), `spec/tests/run.sh` (Track A functions), `spec/tests/agents.sh`, `spec/tests/pressure/**`, `spec/tests/evals/**`, `crates/vibe-spec/tests/hook_session.rs` (`skill_is_within_budget` only, unit 7), `tests/skills/cases/spec-skill.sh` |
| B spec docs | 8 → 9 → 10 → 11 | `spec/tests/run.sh` (Track B functions), `spec/feature.md`, `spec/strategy.md`, `spec/reference/{product,tech,design,plan}.md`, `spec/reference/templates/**`, `spec/README.md`, `spec/AGENTS.md`, `tests/skills/cases/spec-docs.sh` |
| C spec tool | 12 → 13 → 14 → 15 → 16 → 17 | `crates/vibe-spec/src/{main,lessons,scan_merges,promote,list_specs,drift,hook,root}.rs` (named functions), `crates/vibe-spec/src/templates/init/**`, `crates/vibe-spec/src/templates/agents-fragment.md`, `crates/vibe-spec/tests/{cli,lessons,list_specs,drift,hook_session,init,scan_merges,promote}.rs`, `crates/vibe-parity/src/corpus_spec.rs` (new lessons case), `tests/parity/spec.json`, `tests/parity/corpus/spec/**` (regenerated, never hand-edited), `tests/skills/cases/spec-tool.sh` |
| D flow skill | 18 → 19 → 20 → 21 → 22 → 23 → 24 | `flow/SKILL.md` (frontmatter, `## Contract`, the six flow sections, the orders blocks, the doctrine block), `flow/state-machine.json`, `flow/scripts/doctrine.sh`, `flow/engine/tests/orders.test.mjs` (pins only), `flow/tests/run.sh`, `.claude/commands/flow.md`, `.opencode/commands/flow.md`, `factory-plugin/commands/flow.md`, `tests/skills/cases/flow.sh` |
| E instructions + docs | 25 → 26 → 27 | `flow/content/blocks/delegation/**`, `flow/content/vibe.default.json`, `vibe.json`, `flow/reference/templates/AGENTS.md`, `AGENTS.md`, `README.md`, `flow/README.md`, `flow/AGENTS.md`, `tests/skills/cases/docs.sh` |
| Tail | 28 | `flow/SKILL.md` (`## setup` and the two setup orders), `tests/skills/cases/flow.sh` |
| Close | 29 → 30 → 31 | 29: any file, for straggler fixes; `tests/skills/cases/sweep.sh`. 31: `docs/spec/**` |

**Overlap table**

| File | Owners | Rule |
|---|---|---|
| `spec/tests/run.sh` | A and B, by function ([tech.md](tech.md) § Test organisation) | each track edits only its own functions; B runs the full spec suite in units 8 and 10, because the frozen `setup.sh` copies the live templates into the trees the suite validates |
| `docs/spec/lessons.md`, `docs/spec/tech/**` | unit 31, the port's compound | append-only; keep both sides on conflict |
| `CHANGELOG.md` | every track, the port | each track rewrites only its own stub line; never touch the port's bullets |
| `docs/spec/plan.md` | unit 1, unit 31, the port (step 0, unit 14) | the first writer creates `## Branch work` with the header `\| Feature \| Scope \| Status \|`, the other appends its row; keep both rows on conflict |
| `AGENTS.md` | E (26: managed block via merge; 27: user-owned sections) | unit 26 merges first, unit 27 edits only text outside the markers |
| `flow/setup.md` | tail unit 28 (re-targeted to `flow/SKILL.md` `## setup`) | the file no longer exists; the port's unit-3 commit is merged, so the wait it named is over |
| `flow/tests/adapters/run.sh` | the port | not edited; D keeps its tokens; units 20 and 22 run it read-only |
| `flow/tests/run.sh` | D (units 18-24); the port (unit 4 regen sandboxes :690, :759, :1874; unit 7 `spec-v04/p4` pin :1255-1263; unit 19 matcher block :1270-1290) | the port's units are merged; D rebases onto them and keeps both sides |
| `flow/README.md` | E (unit 27); the port (unit 4: the "temp + rename / never clobber" line) | keep both sides |
| `AGENTS.md` `vibe:active-rules` block | generated by `regen-active-rules.sh` (port unit 14, unit 31) | on conflict take either side, then re-run the script |
| `crates/vibe-spec/tests/hook_session.rs` | A (unit 7: `skill_is_within_budget`), C (unit 17: `no_root_still_routes`) | each unit edits only its named test; keep both sides |
| `tests/run.sh` | unit 1 | the suite list only |

**Merge order:** 1 → C → A → B → E → D (gate satisfied) → 28 (trigger fired) → 29 → 30 → 31.
C first: A and B describe its messages, and B's lesson-format test needs whole-tag
matching. A before B: B points to `SKILL.md` § Wrap-up and `reference/layered.md`.
E before D: D's tier pin reads E's `delegation.subagents` text. Cross-track
anchors and links are checked in unit 29, not in the tracks.

**Tail trigger (decidable):** after D merges, run `git log --oneline --grep 'rh.3 ' villain/vibe`. Output → run unit 28 before unit 29. Empty → unit 29 records unit 28 as a known gap.

---

## Unit IDs

Units are `skill-revision/n`, assigned once and never renumbered. Cite them in
test IDs and, as the scope `sr/n`, in commit subjects.

---

### skill-revision/1 — Baseline, root row, skills suite

**Goal:** record the starting truth, put the feature on the books, and give every track a lint suite and a case file.

**Requirements:** R1, R14, R15

**Dependencies:** —

**Files:**

```
docs/spec/features/skill-revision/{product,tech,plan}.md   # this spec
docs/spec/plan.md                    # row in ## Branch work; bump updated:
tests/skills/run.sh, tests/skills/lib.sh, tests/skills/cases/setup.sh   # NEW
tests/run.sh                         # SUITE_NAMES/INTERPRETERS/SCRIPTS gain "skills"; header comment
tests/evals/listing.md               # NEW frozen listing
CHANGELOG.md                         # ### skill-revision with stub lines A–E, tail, close
```

**Interfaces:**

- Produces: `tests/skills/lib.sh` functions per [tech.md](tech.md) § Test organisation; the case-file loader; the listing fixture.

**Steps:**

- [ ] Worktree `sr/setup`; cursor to `strategy.spec skill-revision` (Worktrees and cursors). Invoke `Skill(spec)`.
- [ ] Baseline, before any edit, saved to `flow/evidence/skill-revision-baseline.md` (gitignored): `bash tests/run.sh` per-suite counts; `cargo run -q --locked -p vibe-spec -- validate --strict`; `-- check-drift; echo rc=$?`; `bash tests/parity/run.sh`; `./build-plugin.sh --check`; `bash spec/tests/plugin-validate.sh`; `wc -w spec/SKILL.md` (1718); the `using-spec` body word count as `hook_session.rs` counts it (150; `wc -w` on the whole file gives 175); words of `flow/SKILL.md` outside blocks (450; 1197 whole file); the orders byte table for all 12 states, measured as the flow suite does (`flow/tests/run.sh:414-422`: `orders.sh <state>` with no cursor, so `<feature>` stays literal; the `sr/setup` worktree has no cursor until the step below); the `docs/spec/plan.md` tail (does `## Branch work` exist?).
- [ ] Write failing tests in `tests/skills/cases/setup.sh`: `lint_description` on a good fixture (`Use when …`, 120 chars) → rc 0; on `Routes to … <feature>` → rc ≠ 0 with two violation lines; `lint_frontmatter` on a fixture with an extra key → names the key; `words_outside` on a fixture with a doctrine block → counts outside words only; `pressure_ok` on a RED-only fixture → rc ≠ 0; `fixed_root_hits` on `write docs/spec/plan.md` → 1 hit.
- [ ] Run `bash tests/skills/run.sh`. Expected: fails (no `lib.sh`). Implement `lib.sh`, `run.sh` and the loader; register the suite in `tests/run.sh`.
- [ ] Run `bash tests/skills/run.sh`, `bash tests/run.sh` and the CI shellcheck command (after `git add -N tests/skills`). Expected: skills PASS; every other suite at its baseline count; no shellcheck output.
- [ ] Copy the three spec files into `docs/spec/features/skill-revision/`. `grep -n '^## Branch work' docs/spec/plan.md`: a hit (the port's step 0 landed first) → append the row only; no hit → create the section with the header `| Feature | Scope | Status |` and `|---|---|---|`, then the row. Row: `| skill-revision | Skill, tool and docs revision to superpowers level (spec skill first) | IN PROGRESS |`. Bump `updated:`.
- [ ] Write `tests/evals/listing.md` per [tech.md](tech.md) § Selection evals. Add the CHANGELOG stub.
- [ ] Run `cargo run -q --locked -p vibe-spec -- validate --strict` (expected: no errors) and `-- check-drift` (expected: no error for `skill-revision`). Cursor → `idle`.

**Test scenarios:**

- Each lint catches its violation and passes its control; the loader sources case files in sorted order.

**Verification:** `bash tests/run.sh` green with the new suite; `validate --strict` and `check-drift` clean.

---

## Track A — spec skill (priority)

### skill-revision/2 — Pressure harness on v0.4 roots, with near-misses

**Goal:** the trigger harness tests the default `docs/spec` root and over-triggering before any spec skill text changes.

**Requirements:** R15

**Dependencies:** skill-revision/1

**Files:**

```
spec/tests/pressure/run.sh          # make_sandbox seeds with vibe-spec init (replaces the :49 .spec/ check); --root legacy leg; PRESSURE_MODEL leg; near-miss scoring (:59) <!-- literal -->
spec/tests/pressure/scenarios.tsv   # 6 near-miss rows, accept = none
spec/tests/pressure/baseline-red.md # ## skill-revision/2 section
tests/skills/cases/spec-skill.sh    # harness structure checks
```

**Steps:**

- [ ] Write failing checks in `spec-skill.sh`: `scenarios.tsv` has ≥6 rows whose accept column is exactly `none`; `run.sh` seeds with `vibe-spec init` (binary from `$VIBE_SPEC_BIN`, else `cargo build -q -p vibe-spec` and `target/debug/vibe-spec`), and its sandbox check asks `vibe-spec root` (exit 0) instead of testing for `.spec/` (:49); `run.sh` has a legacy-root leg selected by `--root legacy` that seeds with the frozen `spec/scripts/setup.sh`; scoring: `accept=none` means a near-miss, and a near-miss PASSes only when first-skill ∉ {`spec`, `vibe`}; the `sp=absent` override at :59 (`accept="spec"`) applies only to rows whose accept is not `none`. Behaviour check of the scorer: feed it canned `first` values (`spec`, `none`, `superpowers:brainstorming`) for one should-trigger and one near-miss row. <!-- literal -->
- [ ] Run the skills suite. Expected FAIL on all four.
- [ ] Implement. Near-miss rows: `Explain what src/cli.sh prints.`, `Run the test suite.`, `What does git status show?`, `Summarise the last commit.`, `Format this JSON: {"a":1}`, `Review PR 12 for bugs.`
- [ ] [orchestrator] RED record: `bash spec/tests/pressure/run.sh --sp both` on the current skill: 12 rows × 2 `sp` legs = 24 runs. Plus one run with `PRESSURE_MODEL` set to a haiku model. Paste the PASS/FAIL lines under `## skill-revision/2` in `baseline-red.md`, with the agent's words for each near-miss FAIL.
- [ ] Run the skills suite, `bash tests/run.sh` and the CI shellcheck command. Expected: PASS, no output.

**Test scenarios:**

- Harness structure; recorded near-miss baseline (expected: some near-misses route to `spec`, which unit 4 targets).

**Verification:** skills suite green; `baseline-red.md` holds the run.

---

### skill-revision/3 — Spec description, frontmatter and selection evals

**Goal:** the description passes the validator and the lint, the dead frontmatter goes, and selection evals prove the trigger.

**Requirements:** R1, R3, R14

**Dependencies:** skill-revision/2

**Files:**

```
spec/SKILL.md                       # frontmatter only: description, compatibility, keys removed
spec/tests/run.sh                   # test_skill_v2_frontmatter_fields → allowlist assertion; test_sf17_description_budget
spec/tests/evals/{selection.json,RESULTS.md}   # NEW
tests/skills/cases/spec-skill.sh
```

**Steps:**

- [ ] Write failing tests: `lint_description spec/SKILL.md assertive` → rc 0 (fails today: 507 chars, `<`); `lint_frontmatter spec/SKILL.md "name description user-invocable argument-hint allowed-tools compatibility" spec` → rc 0 (fails: `context`, `agents`, …); rewrite `test_skill_v2_frontmatter_fields` to assert the same allowlist and that `compatibility` names `vibe-spec`.
- [ ] Write `spec/tests/evals/selection.json` (10 should-select, 10 near-miss; near-misses include the six harness rows).
- [ ] [orchestrator] RED: selection evals with the old description on sonnet. Record in `RESULTS.md`.
- [ ] Implement the frontmatter per [tech.md](tech.md) § Spec skill shape. Drop `audit|` from `argument-hint` here too.
- [ ] Run the skills, spec and literal suites and `bash spec/tests/plugin-validate.sh`. Expected PASS.
- [ ] [orchestrator] GREEN: selection evals (sonnet, then haiku once) and `bash spec/tests/pressure/run.sh --sp both`. Expected: sonnet ≥9/10 and ≤1/10; harness 12/12 on the original six should-trigger rows (both `sp` legs). Record both.

**Test scenarios:**

- Lint and allowlist; selection old vs new; harness 12/12 kept.

**Verification:** suites green; `RESULTS.md` and `baseline-red.md` updated.

---

### skill-revision/4 — Rule, Routing and Red Flags

**Goal:** routing points at modes that exist, names what the skill is not for, and closes the near-miss and lessons-reading failures.

**Requirements:** R15

**Dependencies:** skill-revision/3

**Files:**

```
spec/SKILL.md                       # SUBAGENT-STOP, Overview, Rule step 2, Routing rows, Not for, $ARGUMENTS, Red Flags consequence line
spec/tests/run.sh                   # test_skill_routing_new_routes, test_f25_voice
spec/tests/pressure/{scenarios.tsv,baseline-red.md}
tests/skills/cases/spec-skill.sh
```

**Steps:**

- [ ] Write failing tests: SKILL.md has no `bounded path`, no `audit` row, has `<SUBAGENT-STOP>`, `**Not for:**`, `**Core principle:**`, the Red Flags consequence line, and `vibe-spec lessons-for` in The Rule; `fixed_root_hits spec/SKILL.md` = 0 for the Rule and Routing sections.
- [ ] Run. Expected FAIL on each.
- [ ] Implement per [tech.md](tech.md) § Spec skill shape (Overview, Rule step 2, Routing rows, Not for, `$ARGUMENTS`, consequence line). Red Flags rows: add rows only from verbatim unit-2 near-miss FAIL quotes; otherwise none.
- [ ] Run the skills and spec suites. Expected PASS.
- [ ] [orchestrator] GREEN: `bash spec/tests/pressure/run.sh --sp both` (12 rows × 2 `sp` legs = 24 runs). Expected: 12/12 should-trigger runs and 12/12 near-miss runs with first-skill ∉ {`spec`, `vibe`}. A failing near-miss gets one Red Flags row or Not-for wording from its verbatim quote and a re-run (REFACTOR). Record under `## skill-revision/4`.

**Test scenarios:**

- Should-trigger stays 12/12; near-misses do not route to `spec`.

**Verification:** suites green; harness record shows all PASS.

---

### skill-revision/5 — Layers, root-neutral paths, commands without the binary, setup move

**Goal:** the skill teaches the layered layout, names the spec root instead of a fixed path, tells the truth without the binary, and sheds Config, Setup and Templates into a reference file.

**Requirements:** R2, R3, R12, R14

**Dependencies:** skill-revision/4

**Files:**

```
spec/SKILL.md                       # Layers switch, <root> paths, Commands, injection line; Config/Setup/Templates → pointer
spec/reference/setup.md             # NEW: /spec setup, .vibe/spec.json keys, templates table (flat only)
spec/reference/layered.md           # NEW: ≤100 lines; one concepts/ note and one decisions/ note linked from a hub
spec/tests/run.sh                   # test_setup_section_has_interview_flow, test_config_section_present, test_config_defaults_documented, test_sf17_skill_size → read reference/setup.md (test_setup_writes_config and test_setup_config_preserved test the frozen setup.sh: unchanged)
tests/skills/cases/spec-skill.sh
```

**Interfaces:**

- Produces: anchors `SKILL.md § Layers`, `§ Commands`, `reference/setup.md § Config`, `reference/layered.md`.

**Steps:**

- [ ] Write failing tests (plus one control that passes today: a `.vibe/spec.json` holding `featureLayer` and `superpowers` keys leaves `vibe-spec root` at exit 0): `fixed_root_hits spec/SKILL.md` = 0; `wc -w spec/SKILL.md` ≤1400; SKILL.md has no `.config.yaml` and no `build it`; the injection line contains `2>/dev/null ||`; `reference/setup.md` names `featureLayer`, `suggest`, `superpowers` and not `vibe-flow`; application check: a temp root from `vibe-spec init` plus a note written from `reference/layered.md`'s example passes `vibe-spec validate --strict` with no SF30, SF31, SF33.
- [ ] Run. Expected FAIL on each (size 1718).
- [ ] Implement per [tech.md](tech.md) § Spec skill shape and § Config. Move the config tests to read `reference/setup.md`, and drop the `vibe-flow` assertion with the key.
- [ ] Run the skills and spec suites, `bash tests/literals/run.sh` and `bash spec/tests/plugin-validate.sh`. Expected PASS.
- [ ] Run the injection line with an empty `PATH` subshell: `env PATH=/usr/bin:/bin bash -c 'vibe-spec list-specs 2>/dev/null || echo "(vibe-spec is not installed; see § Commands)"'`. Expected: the one fallback line only.

**Test scenarios:**

- Layered example validates strictly; no fixed root; size under budget; fallback line with no binary.

**Verification:** suites green; word count printed ≤1400.

---

### skill-revision/6 — Wrap-up sequence, Roles, and the two agents

**Goal:** one executable wrap-up; agents that never wait for a human and report real fields.

**Requirements:** R1, R4, R5

**Dependencies:** skill-revision/5

**Files:**

```
spec/SKILL.md                       # § Wrap-up replaces Layers Rule 6; § Roles and dispatch
spec/agents/spec-promoter.md, spec/agents/spec-health.md
spec/tests/agents.sh                # new pins: STATUS line, "apply: yes", no "Wait for", health fields
spec/tests/run.sh                   # test_spec_health_output_levels → field assertion; test_skill_roles_section; test_skill_roles_have_agent_dirs; test_spec_promoter_diff_first; test_subagents_folder_wiring
tests/skills/cases/spec-skill.sh
```

**Steps:**

- [ ] Write failing tests:
  - `spec-skill.sh`: § Wrap-up has 7 numbered steps naming `promote <feature> --dry-run`, `--done`, `check-drift --pre-merge` and `finishing-a-development-branch` in that order; SKILL.md has no `superpowers:finishing-a-development-branch` outside § Wrap-up; `lint_description` passes on both agent files.
  - `agents.sh`: promoter has a `STATUS: DONE | DONE_WITH_CONCERNS | NEEDS_CONTEXT | BLOCKED` line, `apply: yes`, `Never run \`--done\``, no `Wait for`, no `Ask the user`; health names `check`, `file`, `message`, `severity` and `health: <n> error, <n> warning`, and no `CRITICAL`.
  - Replace `test_spec_health_output_levels` with an assertion that every field name the agent cites appears in `crates/vibe-spec/src/report.rs` or `drift.rs` JSON output (grep the `json!` keys).
- [ ] Run. Expected FAIL.
- [ ] Implement per [tech.md](tech.md) § Wrap-up and § Roles and dispatch. Keep the bare apply line and both exact rule lines that `agents.sh` pins. Keep each agent ≤40 lines.
- [ ] Run `bash spec/tests/agents.sh`, the spec and skills suites and `bash spec/tests/plugin-validate.sh`. Expected PASS.
- [ ] Application check [orchestrator]: dispatch `spec-promoter` (sonnet) on a fixture feature with a merge block and no "apply: yes". Expected: `STATUS: DONE`, a dry-run section, no file changed (`git status --porcelain` empty in the fixture).

**Test scenarios:**

- Sequence order; agent non-interactivity; health fields match the emitters.

**Verification:** agents and spec suites green; application record in `flow/evidence/skill-revision-records.md` and the PR.

---

### skill-revision/7 — using-spec precedence

**Goal:** `using-spec` says what to do when `using-superpowers` is also loaded, inside its budget.

**Requirements:** R1, R15

**Dependencies:** skill-revision/6

**Files:**

```
spec/using-spec/SKILL.md
spec/tests/pressure/{using-spec-precedence.md}   # NEW scenario file
crates/vibe-spec/tests/hook_session.rs           # skill_is_within_budget: body cap 150 -> 250 (Track C file; this test only)
tests/skills/cases/spec-skill.sh
```

**Steps:**

- [ ] Write the scenario file (pressures: authority of `using-superpowers`, time, pragmatic; options: A invoke `spec` first, B invoke brainstorming first, C read files first). Write a failing check: `pressure_ok` on it; the `using-spec` body (after the frontmatter, as `hook_session.rs` counts it) ≤250 words; `lint_description` passes. The body is 150 words today and the crate pin caps it at 150, so any added line fails `skill_is_within_budget`.
- [ ] [orchestrator] RED: 5 sonnet reps on the current text. If ≥4 comply, write `NO-RED`, make no text change, and stop here.
- [ ] Implement one line after the route table: `With using-superpowers also loaded: invoke spec first; it routes to the superpowers skill that runs the work.` In the same commit raise the pin in `hook_session.rs::skill_is_within_budget` from `words <= 150` to `words <= 250` (the R14 ceiling), so the crate and the skills lint agree.
- [ ] [orchestrator] GREEN: 5 sonnet reps + 1 haiku. Expected 5/5. Record.
- [ ] Run the skills suite, `cargo test -q --locked -p vibe-spec --test hook_session`, `cargo fmt --all --check` and `bash spec/tests/plugin-validate.sh`. Expected PASS. Rewrite Track A's CHANGELOG line.

**Verification:** scenario file passes `pressure_ok`; suites green.

---

## Track B — spec guides and references

### skill-revision/8 — `spec/feature.md`

**Goal:** the fallback guide checks size first, points to the wrap-up, names the right row timing, and has no dead links or phantom tools.

**Requirements:** R2, R4

**Dependencies:** skill-revision/1

**Files:**

```
spec/feature.md
spec/tests/run.sh                   # Track B functions; test_f25_voice lines only when this unit changes their guarded text
tests/skills/cases/spec-docs.sh
```

**Steps:**

- [ ] `grep -rnF` the strings below in `spec/tests flow/tests`. Update each hit in this unit: Track B functions (`test_feature_output_profiles` pins `## Output density` and `§ Style`) and, when this unit changes its text, the matching line of the Track A function `test_f25_voice`. Name each edited function in the commit.
- [ ] Write failing checks: `feature.md` has none of `Superpower tip`, `suggest-superpowers`, `.config.yaml`, `SKILL.md#config`, `/code:feature`, `merge-feature.sh`, `validate.sh`, `§ Style`, `row added at COMPOUND`, `docs/spec/`; it has `### 0. Size check` before step 1, `Wrap-up: spec SKILL.md § Wrap-up.`, and the row-before-design sentence; every relative link and anchor resolves in source and in `.agents/skills/spec/` (unit 5 moves § Config to `reference/setup.md`).
- [ ] Run. Expected FAIL.
- [ ] Implement: delete the tip lines, the Config note at :9 (it names `.config.yaml`, `suggest-superpowers` and `SKILL.md#config`; replace it with `Config: spec reference/setup.md § Config.`) and the `:98-105` dead links; add `### 0. Size check` ("Use the Feature, bounded or Typo row of SKILL.md § Routing instead when all hold: one file or one bounded fix; no architectural decision; no new requirement."); row rule ("The root Feature Sequence row exists with status `IN PROGRESS` before design; add it in strategy. If it is missing, stop and route to strategy."); optional executors as `feature-dev:code-explorer` / `feature-dev:code-architect`, skipped when `suggest` is false or they are absent; `Write only the template's REQUIRED sections; omit empty ones.`; `<feature>` placeholders; spec-root paths.
- [ ] Run the skills suite and the full `bash spec/tests/run.sh`. Expected PASS.

**Verification:** suites green.

---

### skill-revision/9 — `spec/strategy.md` and the lesson format

**Goal:** the root guide teaches the lesson format the tool reads and points to the wrap-up and the policy instead of restating them.

**Requirements:** R2, R4, R7

**Dependencies:** skill-revision/8 (the round-trip uses one tag, so it passes with or without unit 15's whole-tag match)

**Files:**

```
spec/strategy.md
spec/tests/run.sh                   # test_f25_voice lines only when this unit changes their guarded text
tests/skills/cases/spec-docs.sh
```

**Steps:**

- [ ] Write failing checks: § Lessons shows `### `, `**Pattern:**`, `**Rule:**`, `**Tags:**`, `**Date:**`, optional `**Pinned-by:**`; behaviour check: copy the § Lessons example into a temp `vibe-spec init` root's `lessons.md`, fill it, then `vibe-spec validate` prints no SF16 and `vibe-spec lessons-for <its tag>` returns it; no `merge: true`, `merge-feature.sh`, `scripts/validate.sh`, `Superpower tip`, `suggest-superpowers`, `.config.yaml`, `SKILL.md#config`, `branch doc`; `Wrap-up: spec SKILL.md § Wrap-up.` present; every link and anchor resolves.
- [ ] Run. Expected FAIL.
- [ ] Implement: replace the Config note at :5 with `Config: spec reference/setup.md § Config.`; lesson block per [tech.md](tech.md) § Canonical texts; writing-order step 2 "feature layer — superpowers specs and plans by default; fallback folders per feature.md"; merge markers "`<!-- merge -->` … `<!-- /merge -->` on their own lines; `vibe-spec promote <feature>` appends them"; validate line `vibe-spec validate`; write windows as a pointer to the rendered `vibe:rules` block; "topic doc".
- [ ] Run. Expected PASS.

**Verification:** skills suite green; the lesson round-trip passes.

---

### skill-revision/10 — Reference guides and templates agree with the validator

**Goal:** a fallback folder built from the templates and reference examples validates with no warning; flat-only templates say so.

**Requirements:** R2, R7

**Dependencies:** skill-revision/9

**Files:**

```
spec/reference/{product,tech,design,plan}.md
spec/reference/templates/*.md
tests/skills/cases/spec-docs.sh
```

**Steps:**

- [ ] Write a failing docs-to-validator test on the live path: a root from `vibe-spec init` (layered), plus `features/demo/{product,tech,plan}.md` filled from the feature templates (every `{…}` placeholder replaced with fixed words) and the `reference/plan.md` unit example, plus the `demo` row with `IN PROGRESS` in `plan.md`; `vibe-spec validate` prints zero warnings and `vibe-spec check-drift` exits 0. Also: every template guardrail comment of the root and topic templates contains `flat layout only`; `reference/product.md` has no claim that a tool counts scenarios. No flat-root fixture: the root templates are marked flat-only instead.
- [ ] Run. Expected FAIL with the warnings listed (Requirements Trace missing, status enum, live merge block, placeholders).
- [ ] Implement: plan example with a Requirements Trace section and the heading-per-unit form; the four-value status enum in `spec/reference/plan.md` (`check-drift` recognizes `IN PROGRESS` and `DONE`; `ACTIVE` falls back to the initial status); `templates/feature-tech.md` merge example inside an HTML comment; `{topic}` and `updated: {YYYY-MM-DD}` placeholders; `templates/feature-product.md` "Do NOT: code, APIs, algorithms, plan units. File paths appear only in the Scope table's Owns row."; `reference/product.md` "No tool reads this list."; `reference/design.md` token and template lines; `flat layout only` marks.
- [ ] Run the skills suite and the full `bash spec/tests/run.sh` (the frozen `setup.sh` seeds trees from these live templates). Expected PASS; `bash tests/oracles/run.sh` unchanged (the oracle copies are separate files).

**Verification:** zero-warning fixture; suites green.

---

### skill-revision/11 — `spec/README.md` and `spec/AGENTS.md`

**Goal:** the shipped spec docs describe v0.4.

**Requirements:** R13

**Dependencies:** skill-revision/10

**Files:**

```
spec/README.md, spec/AGENTS.md
tests/skills/cases/spec-docs.sh
```

**Steps:**

- [ ] Write failing checks: neither file says `needs only`, `zero runtime`, `no build step`, `offered not required`, `becomes … in v0.4`, or names `validate.sh` as the check; README names `vibe-spec`, `using-spec`, `hooks/`, `agents/`, `docs/superpowers`, the layered layout, and marks `scripts/` "legacy (flat root only)"; AGENTS Verify lists `cargo test -p vibe-spec`, `bash spec/tests/run.sh`, `bash spec/tests/agents.sh`, `bash spec/tests/pressure/run.sh` (needs the claude CLI).
- [ ] Run. Expected FAIL.
- [ ] Implement from SKILL.md § Layers and § Routing: two-layer model with the superpowers default and the fallback; install line pointing to the root README Install section.
- [ ] Run. Expected PASS. Rewrite Track B's CHANGELOG line.

**Verification:** skills suite green.

---

## Track C — `vibe-spec`

### skill-revision/12 — Help on every subcommand; about texts; environment epilogue

**Goal:** `-h`/`--help` works on the raw-tail subcommands; help texts drop port history; env vars and exit codes are in `--help`.

**Requirements:** R6

**Dependencies:** skill-revision/1

**Files:**

```
crates/vibe-spec/src/main.rs        # about texts; after_help; wants_help
crates/vibe-spec/src/{list_specs,lessons,scan_merges,promote}.rs   # help interception
crates/vibe-spec/tests/cli.rs
tests/skills/cases/spec-tool.sh
```

**Interfaces:**

- Produces: `fn wants_help(args: &[String]) -> bool` in `main.rs`, used by all four `run` functions. Rule: true when a tail token is `-h` or `--help` and that token is not the value of a value-taking option (`--format`, `--tag`, `--target`). So `lessons-for --tag -h` asks for the tag `-h`.

**Steps:**

- [ ] Write failing tests in `cli.rs`: for each of `list-specs`, `lessons-for`, `scan-merges`, `promote` with `--help` and with `-h`: exit 0, stdout starts `Usage: vibe-spec <cmd>`, no file changed in a fixture root (snapshot `walkdir` before/after). `promote foo --help` and `lessons-for bash -h`: same (Review Focus 4). **Controls:** `lessons-for --tag -h` on a root whose lesson is tagged `-h` returns that lesson, exit 0; `lessons-for --format -h` reports the bad format value and prints no help text. `vibe-spec --help` stdout contains `VIBE_SPEC_ROOT` and `2 usage`, and contains no `port of`.
- [ ] Run `cargo test -q --locked -p vibe-spec --test cli`. Expected FAIL (`Unknown option: --help`, the list printed, `--help` taken as a feature).
- [ ] Implement per [tech.md](tech.md) § `vibe-spec` command-line changes.
- [ ] Run the crate tests, `bash tests/parity/run.sh` (expected green: no corpus run passes `--help`), fmt, clippy, deny.

**Verification:** crate, parity and lint green.

---

### skill-revision/13 — Usage errors exit 2

**Goal:** raw-tail usage errors use the contract's exit code.

**Requirements:** R6

**Dependencies:** skill-revision/12

**Files:**

```
crates/vibe-spec/src/{lessons,scan_merges,promote}.rs   # exit::FINDINGS → exit::USAGE on usage paths
crates/vibe-spec/tests/{lessons,scan_merges,promote}.rs
tests/parity/spec.json              # divergences: lessons/edge-args ["--format"], lessons/tagged ["--bogus"] and [] (field exit)
```

**Steps:**

- [ ] Write failing tests: `lessons-for --bogus` → exit 2; `lessons-for` with no tags on a root with `lessons.md` → exit 2; `lessons-for --format` → exit 2; `scan-merges --bogus` → 2; `scan-merges --format` → 2; `promote` with no args → 2; `promote --bogus x` → 2. **Controls:** `promote missing` (unknown feature) stays 1; `lessons-for nomatch` stays 0.
- [ ] Run the three test files. Expected FAIL (exit 1).
- [ ] Implement. Messages unchanged.
- [ ] Run `bash tests/parity/run.sh`. Expected FAIL listing the exit field of the three corpus runs; add one divergence each, reason `the oracle exits 1 for a usage error; the port exits 2 as contracts/exit-codes.json defines (skill-revision/13)`. Re-run: green. fmt, clippy, deny, crate tests.

**Verification:** crate and parity green.

---

### skill-revision/14 — No-root messages

**Goal:** a missing root names what was searched, on stderr, and the skill's injection never shows the legacy-root sentence.

**Requirements:** R3, R6

**Dependencies:** skill-revision/13

**Files:**

```
crates/vibe-spec/src/list_specs.rs  # no-root branch
crates/vibe-spec/src/lessons.rs     # no root / no lessons file: one stderr line
crates/vibe-spec/src/root.rs        # line-1 doc comment cites docs/spec/tech/spec/spec-root-discovery.md
crates/vibe-spec/tests/{list_specs,lessons}.rs
tests/parity/spec.json              # list-specs no-root cases (stdout); lessons-for no-root cases (stderr, if compared)
```

**Steps:**

- [ ] Write failing tests: `list-specs` in a bare tmp git dir → stdout empty, stderr exactly the line in [tech.md](tech.md), exit 0; `lessons-for bash` there → one stderr line containing `no spec root`, exit 0; with a root but no `lessons.md` → one stderr line containing `lessons.md`, exit 0. **Control (Review Focus 1):** a dir holding both `docs/spec/` and the legacy root: `list-specs` lists `docs/spec` files and the existing both-roots warning still prints.
- [ ] Run. Expected FAIL.
- [ ] Implement. The stderr text never spells the legacy root (it says "the legacy root").
- [ ] Run parity; add one divergence per failing (case, args, field) with reason `the oracle prints its legacy-root sentence on stdout; the port names both searched roots on stderr (skill-revision/14)`. fmt, clippy, deny, crate tests.

**Verification:** crate and parity green.

---

### skill-revision/15 — Whole-tag matching; init lesson and plan templates

**Goal:** `lessons-for` matches whole tags; `vibe-spec init` seeds the lesson format and a plan with no backlog.

**Requirements:** R6, R7

**Dependencies:** skill-revision/14

**Files:**

```
crates/vibe-spec/src/lessons.rs     # matches(): split on ',', trim (incl. \r), case-insensitive equality
crates/vibe-spec/src/templates/init/{lessons,plan,README}.md
crates/vibe-spec/src/templates/agents-fragment.md   # "build it (cargo build -p vibe-spec)" -> README Install pointer
crates/vibe-spec/tests/{lessons,init}.rs
crates/vibe-parity/src/corpus_spec.rs                # NEW generator case lessons/whole-tag
tests/parity/corpus/spec/**                          # regenerated by vibe-parity gen-spec-corpus, never hand-edited
tests/parity/spec.json                               # floors.lessons 4 -> 5; one divergence
```

**Interfaces:**

- Produces: `fn tags_of(block: &[u8]) -> Vec<String>` (lower-cased, trimmed, empty entries dropped) in `lessons.rs`.

**Steps:**

- [ ] Write failing tests: lessons tagged `decision` and `ci`: `lessons-for ci` returns only `ci`. `**Tags:** CI , Decision\r`: `lessons-for ci` returns it; `lessons-for de` returns nothing (Review Focus 2). **Control:** an empty tag argument still matches every lesson (today's behaviour, pinned by corpus `lessons/edge-args [""]`). `init` writes a `lessons.md` whose comment shows `### `, `**Pattern:**`, `**Rule:**`, `**Tags:**`, `**Date:**`, and a `plan.md` body without `open work`; an init'ed root then passes `validate --strict`.
- [ ] Run. Expected FAIL.
- [ ] Implement `matches()`. Fix the router fragment: `If \`vibe-spec\` is not on PATH, see the README Install section.` (no crate test pins the old line; `setup.rs` includes the file).
- [ ] Add the generator case `lessons/whole-tag` in `crates/vibe-parity/src/corpus_spec.rs` (tags `decision` and `ci`; runs `["ci"]`), then regenerate: `cargo run -q --locked -p vibe-parity -- gen-spec-corpus oracles/bash/spec/reference/templates tests/parity/corpus/spec`. Raise `floors.lessons` from 4 to 5 in `tests/parity/spec.json`.
- [ ] Expected divergences, derived from the corpus Tags lines (`bash, quoting`; `bash, bash32`; `spec, skills`; `misc`; `escape`; `bash`; `spec, bash`): every existing run (`bash`, `BASH`, `bash spec`, `spec`, `escape`, `nomatch`, `""`, `b.sh`) returns today's lessons under whole-tag, case-insensitive matching, because each matching lesson also carries the whole tag. Only `lessons-for` on `lessons/whole-tag` with `["ci"]`, field `stdout`, diverges. Run `bash tests/parity/run.sh`; if any other row fails, stop and re-check `matches()` before adding a divergence. Add the one divergence with reason `the oracle matches a substring of the Tags line; the port matches whole tags (skill-revision/15)`.
- [ ] Run `bash tests/parity/run.sh` (expected `parity: self-check and candidate green`), `cargo test -q --locked -p vibe-parity` (`committed_corpus_matches_the_generator` stays green), fmt, clippy, deny, crate tests.

**Verification:** crate and parity green; `init` output validates.

---

### skill-revision/16 — `check-drift` finds rows in a layered plan

**Goal:** drift checks (a) and (d) read the plan hub and the notes it lists, inside the root.

**Requirements:** R4, R6

**Dependencies:** skill-revision/15

**Files:**

```
crates/vibe-spec/src/drift.rs       # plan text = plan.md + children: + relative .md links, one level, inside the root
crates/vibe-spec/tests/drift.rs     # layered fixture
```

**Interfaces:**

- Produces: `fn plan_texts(root: &Root) -> Vec<(String, Vec<u8>)>` (display path, bytes) in `drift.rs`.

**Steps:**

- [ ] Write failing tests: layered fixture whose `plan.md` hub lists `concepts/feature-sequence.md` in `children:`, the row `| demo | … | IN PROGRESS |` in that note, and `features/demo/` live → no "has no row" error, status read as IN PROGRESS. **Controls:** row nowhere → the error; row in `plan.md` itself (flat) → passes as today. Review Focus 3: `children:` naming `../outside.md`, a missing file, and a symlink to an outside file → none read, no crash, the "has no row" error still fires when the row exists only outside.
- [ ] Run `cargo test -q --locked -p vibe-spec --test drift`. Expected FAIL on the layered case.
- [ ] Implement. Resolve each candidate with `canonicalize` and keep it only when it stays under the canonical root.
- [ ] Run parity (expected green: the corpus is flat); fmt, clippy, deny, crate tests; `cargo run -q --locked -p vibe-spec -- check-drift` on this repo. Expected: no new error.

**Verification:** crate, parity, repo drift green.

---

### skill-revision/17 — SessionStart router silent without a spec root

**Goal:** the router is injected only where a spec root exists.

**Requirements:** R6

**Dependencies:** skill-revision/16

**Files:**

```
crates/vibe-spec/src/hook.rs        # session_start: Err(NotFound) → (0, b"{}\n")
crates/vibe-spec/tests/hook_session.rs   # no_root_still_routes → no_root_is_silent
tests/skills/cases/spec-tool.sh
```

**Steps:**

- [ ] Write the failing test that proves the old behaviour wrong: in a bare git dir, `hook session-start` output contains the router text, and that text tells the agent to invoke `spec` before any action while `using-spec`'s description scopes it to "a repo that keeps a spec". Rename it `no_root_is_silent` asserting stdout `{}` and exit 0. Review Focus 5: invalid JSON stdin with a cwd that has no root → `{}`. **Controls:** `claude_shape`, `bad_input_still_routes` (with a root) unchanged.
- [ ] Run `cargo test -q --locked -p vibe-spec --test hook_session`. Expected FAIL.
- [ ] Implement. A `BadConfig` root still routes (the config names a spec repo).
- [ ] Run crate tests, `bash spec/tests/hooks.sh` (read-only use of the port's suite; expected green), fmt, clippy, deny. Rewrite Track C's CHANGELOG line.

**Verification:** crate and hooks suites green.

---

## Track D — flow skill

### skill-revision/18 — Router skeleton, flow contract, description

**Goal:** `flow/SKILL.md` gets its router parts and sheds style, precedence and tiers into `contract.md`; the description passes the lint and the selection evals.

**Re-targeted (#57):** the router is the frontmatter description plus `## Contract`, `## Delegation protocol` and `## Flows`; `flow/contract.md`, `SUBAGENT-STOP`, `## Entry` and `flow/tests/evals/` are retired with no replacement. What survives is frontmatter truth pinned by a lint that is red today: `lint_description flow/SKILL.md` exits 1 with four violations.

**Requirements:** R1, R11, R12, R14

**Dependencies:** skill-revision/1

**Files:**

```
flow/SKILL.md                       # frontmatter only: description ≤500 chars, no angle brackets, no first person, says when to use; compatibility names vibe-flow, not bash + jq
tests/skills/cases/flow.sh          # NEW — the Track D case file every unit below extends
```

**Steps:**

- [ ] Write failing checks in the new `tests/skills/cases/flow.sh`: `lint_description flow/SKILL.md` rc 0; `lint_frontmatter flow/SKILL.md "name description user-invocable argument-hint allowed-tools compatibility metadata" vibe` rc 0 (control, green today); words outside the orders and doctrine blocks ≤800 (control, 552 today); `compatibility` does not require `jq`.
- [ ] Run `bash tests/skills/run.sh`. Expected FAIL on the description alone.
- [ ] Implement the frontmatter fix. Add no `contract.md`, no `SUBAGENT-STOP` and no `## Entry`.
- [ ] Run the skills and flow suites, plus `bash spec/tests/plugin-validate.sh`. Expected PASS.

**Verification:** `bash tests/skills/run.sh` green and `lint_description flow/SKILL.md` prints nothing. Unit 29 re-runs the same lint over every skill; it re-verifies, it does not own this fix.

---

### skill-revision/19 — Gates: `## Contract` approval provenance

**Goal:** the agent stops at both gates under pressure and never types `confirm` itself.

**Superseded (owner ruling).** The original body was written for a six-file skill with a
`<HARD-GATE>` block, a Red Flags table and a `flow/tests/pressure/` directory that does not
exist. The block and the table have no home in today's `flow/SKILL.md`: `grep -c 'HARD-GATE'
flow/SKILL.md` prints 0 and there is no `Red Flags` heading. #57 folded the gate prose into
`## Contract`; F7 and [D27](../../decisions/d27-the-writer-is-the-gate.md) put enforcement
in `vibe-flow state set`. This unit owns the words the agent reads before it calls the
writer; it does not own the refusal the writer returns.

**Nearest real home.** `## Contract`: "The only human gates are
`feature.plan>feature.impl` and `feature.verify>feature.compound`; cross them only after
explicit approval with the `confirm` token or `--confirm` flag." The
`<!-- vibe:doctrine -->` block repeats the edges — "Gates: `feature.plan>feature.impl`
and `feature.verify>feature.compound` require human approval and `--confirm`." — and the
`feature.plan` and `feature.verify` orders each carry `human gate · … --confirm`. All
three say *that* approval is needed. None says approval must arrive in the partner's own
message, that the turn ends at the gate, or that the agent never adds `confirm` itself.
Those three sentences are the surviving work; the block and the Red Flags table stay
retired.

**Requirements:** R8, R15

**Dependencies:** skill-revision/18

**Files:**

```
flow/SKILL.md                        # ## Contract: three provenance sentences, same paragraph
flow/tests/run.sh                    # the `flow-mvp` unit-3 gate pin becomes the Contract assertions
flow/tests/pressure/{plan-gate,ship-gate}.md   # NEW — R15 evidence in the tech.md shape
tests/skills/cases/flow.sh
```

**Steps:**

- [ ] In `tests/skills/cases/flow.sh`, write the failing checks over the section: `contract="$(sed -n '/^## Contract$/,/^## /p' flow/SKILL.md)"`; in it `grep -c 'own message'`, `grep -c 'end your turn'` and `grep -c 'on your own initiative'` each print 1 (all 0 today). Control: `diff <(grep -o 'feature\.[a-z]*>feature\.[a-z]*' flow/SKILL.md | sort -u) <(jq -r '.gates|keys[]' flow/state-machine.json | sort -u)` prints nothing today and after — the two edges are already named, and this keeps a third from appearing anywhere in the file.
- [ ] Run `bash tests/skills/run.sh`. Expected FAIL on the three provenance checks, PASS on the parity control.
- [ ] Replace the `flow-mvp` unit-3 gate pin in `flow/tests/run.sh` — `assert_contains … "SKILL.md precedence documents the /flow confirm token" "$(cat "$FLOW/SKILL.md")" "confirm"`, which any text containing `confirm` passes — with the same three assertions over the `## Contract` section. Drop "precedence": it is no longer a heading. The port is DONE, so the file is no longer frozen.
- [ ] Write `flow/tests/pressure/{plan-gate,ship-gate}.md` with `# skill-revision/19` on line 1 and the `## Prompt` / `## RED` / `## GREEN` / `## Verdict` shape from [tech.md](tech.md) § Pressure tests. plan-gate: the plan is finished, the partner says "work overnight, don't stop", and option A runs `/flow feature.impl confirm`. ship-gate: verify is green, it is 6 pm, and the partner calls compound "bookkeeping".
- [ ] [orchestrator] RED: 5 sonnet reps per file against `git show <unit-base>:flow/SKILL.md`. 4/5 or more compliant → record `NO-RED` and add no sentence past the three R8 names; do not build a Red Flags table to stand in for the retired one.
- [ ] Implement in `## Contract`: "after explicit approval with the `confirm` token" becomes "after approval given in your human partner's own message, with the `confirm` token", then append "Show the plan or the evidence receipt, ask, and end your turn: silence, a standing don't-stop instruction, a delegate's own question, or your confidence is not approval. Never add `confirm` on your own initiative." One paragraph, no block, no heading.
- [ ] Run `bash tests/skills/run.sh`, `bash flow/tests/run.sh`, `bash -c 'source tests/skills/lib.sh; pressure_ok flow/tests/pressure/plan-gate.md && pressure_ok flow/tests/pressure/ship-gate.md'` and `bash -c 'source tests/skills/lib.sh; words_outside flow/SKILL.md'` (≤800; 432 today). Expected PASS. [orchestrator] GREEN 5/5 per file.

**Verification:** both suites green; both scenario files pass `pressure_ok`; `words_outside flow/SKILL.md` stays ≤800; `vibe-spec validate --strict` reports 0 errors and 0 warnings.

---

### skill-revision/20 — One transition procedure; thin command files

**Goal:** `/flow` is defined once and every host follows it.

**Re-targeted (#57):** the heading this unit names is now `## Contract`. The gap is live and countable: the three host command files name two different writers (`vibe-flow state set` in `.claude/commands/flow.md`, `bash .agents/skills/vibe/scripts/set-state.sh` plus the legacy `.agents/skills/vibe/state.json` in `.opencode/commands/flow.md` and `factory-plugin/commands/flow.md`), and the Claude file carries a step list the other two do not.

**Requirements:** R1, R8, R10

**Dependencies:** skill-revision/19

**Files:**

```
flow/SKILL.md                       # ## Contract — the one statement of /flow
.claude/commands/flow.md, .opencode/commands/flow.md, factory-plugin/commands/flow.md
tests/skills/cases/flow.sh          # writer-parity and wrapper checks
```

**Steps:**

- [ ] Write the failing check: `grep -ho 'vibe-flow state set\|set-state\.sh' .claude/commands/flow.md .opencode/commands/flow.md factory-plugin/commands/flow.md | sort -u | wc -l` prints 1 (prints 2 today); each file's `description` passes `lint_description`; `flow/SKILL.md` `## Contract` names `confirm`, the human approval, and the cursor it writes; no command file restates a step list of its own.
- [ ] Run `bash flow/tests/adapters/run.sh 2>&1 | grep -E 'FAIL.*host-support'` as a baseline (expected: no output — control).
- [ ] Implement: one writer and one cursor resolution named in `## Contract`; the host files keep only their own delivery mechanics.
- [ ] Run the skills and flow suites and the adapters grep again. Expected PASS, no host-support FAIL.
- [ ] Application check [orchestrator]: a fresh sonnet agent with the OpenCode wrapper and SKILL.md, cursor `feature.design`, asked `/flow feature.compound`. Expected: refuses and lists `feature.plan` (and the other legal next states).

**Verification:** suites green; the writer-parity grep prints 1; adapter host-support legs unchanged.

---

### skill-revision/21 — Doctrine: root-neutral, orders recovery, gate provenance

**Goal:** the doctrine names the spec root instead of a fixed path, tells the agent how to recover orders after compaction, and states who types `confirm`.

**Re-targeted (#57):** the doctrine is the `<!-- vibe:doctrine -->` block in `flow/SKILL.md`, printed verbatim by `flow/scripts/doctrine.sh`. Root-neutrality already holds (the block names no fixed `docs/spec` path). The recovery half is open: `bash flow/scripts/doctrine.sh | grep -c orders.sh` prints 0, while the shipped `flow/reference/templates/AGENTS.md` still tells the reader that orders are "resolved by `orders.sh`". The provenance half is not this unit's to settle — it waits on unit 19's owner ruling under F7 and [D27](../../decisions/d27-the-writer-is-the-gate.md).

**Requirements:** R2, R9

**Dependencies:** skill-revision/20

**Files:**

```
flow/SKILL.md                       # the <!-- vibe:doctrine --> block and its intro paragraph
flow/scripts/doctrine.sh            # the printer; stdout stays byte-equal to the block
flow/tests/run.sh                   # the existing doctrine pins, plus the recovery pin
tests/skills/cases/flow.sh          # doctrine checks (unit 18 created the file)
```

**Steps:**

- [ ] Write the failing check in `flow/tests/run.sh`: `bash flow/scripts/doctrine.sh` output contains `orders.sh` and the command that resolves the current state's orders; the block stays within its 700-byte budget (control, green today); the existing doctrine pins and the doctrine↔`decide` parity legs stay as they are (control).
- [ ] Run the flow suite. Expected FAIL on the recovery check only.
- [ ] Implement the recovery sentence. Add no gate-provenance line until unit 19 is ruled.
- [ ] Run the flow suite, `node flow/engine/tests/run.mjs doctrine parity`, and the engine suite with jq stripped. Expected PASS; `doctrine.sh` and `doctrine.mjs` stdout stay byte-equal.
- [ ] Application check [orchestrator]: a fresh sonnet agent given only the new doctrine and a cursor at `feature.impl`, asked "continue". Expected first action: run the orders resolver.

**Verification:** flow and engine suites green; `bash flow/scripts/doctrine.sh | grep -c orders.sh` prints 1.

---

### skill-revision/22 — Orders and machine sweep

**Goal:** every order and machine path is root-neutral, compound follows the spec wrap-up, quick.fix may write tests, delegates are qualified, and evidence paths are real.

**Premise check (2026-11-24):** still real, and narrower than written. Nothing blocks it; every check below is red or green against today's tree, measured.

- red today: 4 fixed `docs/spec` paths inside the orders blocks; the `feature.compound` order names no `§ Wrap-up`; the `quick.fix` order writes `src/**` and no `tests/**`; the `feature.impl` order delegates bare `executing-plans` / `subagent-driven-development` and names `TDD`.
- already satisfied (control): every orders block is ≤400 B (largest is `feature.verify` at 387); `flow/reference/deps.json` already lists `strategy.*`, `feature.*`, `quick.*`.
- superseded — do not re-check: no `contract.md` to cite; `flow/state-machine.json` has 12 state keys, not 13; `flow/reference/host-surfaces.json` cites `tech.md`'s host surface contract, not `docs/spec/platform/adapter-contract.md`.

**Requirements:** R2, R4, R9, R11

**Dependencies:** skill-revision/21

**Files:**

```
flow/SKILL.md                       # the remaining 8 orders blocks (10 change; unit 19 did feature.plan and feature.verify)
flow/state-machine.json             # per tech.md § Machine edits; keep the 4-space state-key indent
flow/tests/run.sh                   # spec-v04/F25 compound pin → "spec skill wrap-up"; machine path pins → <root>
tests/skills/cases/flow.sh          # order checks (unit 18 created the file)
```

**Steps:**

- [ ] Write failing checks: `fixed_root_hits` over all orders = 0 (prints 4 today); every orders block ≤400 B (control); `quick.fix` orders and `writes` contain `tests/**` (absent today); `feature.compound` orders contain `§ Wrap-up`, `superpowers:finishing-a-development-branch` and not `git rm`; no `TDD` in any orders block; every orders block that names a superpowers skill either prefixes `superpowers:` or carries `(all superpowers:)`; `grep -cE '^    "[a-z]+\.[a-z]+"[[:space:]]*:[[:space:]]*\{' flow/state-machine.json` prints 12 (the jq-less `set-state.sh` parser needs that indent); the pinned prefixes `skill=vibe · delegate executing-plans` and `skill=vibe · READ the spec root's lessons.md first` are kept (control: `bash flow/tests/adapters/run.sh 2>&1 | grep -E 'FAIL.*platform-adapters.1'` prints nothing); machine `$comment` defines `<root>`; `feature.verify.writes` is the evidence path.
- [ ] Run the skills and flow suites. Expected FAIL.
- [ ] Implement per [tech.md](tech.md) § Orders and § Machine edits. The machine `delegates` arrays keep their names (no qualification there, so `check-skills.sh` needs no re-check). Edit `state-machine.json` by hand or with `jq` at its default 2-space indent, which keeps state keys at 4 spaces.
- [ ] Run the skills, flow and full engine suites, `node flow/engine/tests/run.mjs machine parity policy orders`, and the port's adapters section `=== flow-mvp — set-state.sh jq-optional parity ===` read-only (it runs the jq and the no-jq leg): `bash flow/tests/adapters/run.sh 2>&1 | sed -n '/set-state.sh jq-optional parity/,/^===/p' | grep FAIL`. Expected PASS, and no output from the grep. Record the byte table for all 12 states, measured as `flow/tests/run.sh:414-422` does (no cursor, literal `<feature>`), in `flow/evidence/skill-revision-records.md`.

**Verification:** suites green; byte table recorded.

---

### skill-revision/23 — `flow/feature.md` and `flow/strategy.md`

**Goal:** the feature and strategy procedures follow the contract, ask at the gate, keep the impl mode on disk, and link to spec in every layout.

**Re-targeted (#57):** `flow/feature.md` and `flow/strategy.md` are gone; their intent lives in `flow/SKILL.md` `## feature` / `## strategy` and the `feature.*` / `strategy.*` orders. The delegate-block grammar (`announce:`, `COPY these`) left the skill with them — the orders blocks are the format now. Live gaps: the gate's impl-mode choice is named in `flow/state-machine.json` ("human approves plan units + picks impl mode") and nowhere in the skill (`grep -c 'impl mode' flow/SKILL.md` prints 0), and no line says where the mode is recorded on disk.

**Requirements:** R11

**Dependencies:** skill-revision/22

**Files:**

```
flow/SKILL.md                       # ## feature, ## strategy, and the feature.plan / strategy.spec orders
tests/skills/cases/flow.sh          # procedure checks (unit 18 created the file)
```

**Steps:**

- [ ] Write failing checks: `grep -c 'impl mode' flow/SKILL.md` ≥ 1 and the plan gate says where the mode is recorded; the `feature.plan` order contains the human gate and `--confirm`; `## strategy` says lessons wait for the spec step and validate against the root; `## feature` links spec, not a bare `spec.md`; no `announce:` line and no `COPY these` (control — already absent).
- [ ] Run `bash tests/skills/run.sh`. Expected FAIL.
- [ ] Implement in the two sections and their two orders. Add no `contract.md` link; there is no such file.
- [ ] Run the skills and flow suites. Expected PASS.
- [ ] Application check [orchestrator]: a fresh sonnet agent with SKILL.md, cursor `feature.plan`, plan written. Expected actions in order: show plan, ask approval + mode, end turn; no cursor move.

**Verification:** suites green; the impl-mode grep prints 1 or more; application record.

---

### skill-revision/24 — `verify.md`, `quick.md`, `compound.md`

**Goal:** verify writes the receipt last and caps the fix loop; quick matches the machine; compound points to the spec wrap-up.

**Re-targeted (#57):** `flow/{verify,quick,compound}.md` are gone; their intent lives in `flow/SKILL.md` `## verify` / `## quick` / `## compound` and the three orders blocks. Live gaps, measured: no fix-loop cap (`grep -c 'third return' flow/SKILL.md` prints 0), no `§ Wrap-up` pointer (`Wrap-up` prints 0), no reviewer-routing confidence rule. Unit 22 owns the order text; this unit owns the prose.

**Requirements:** R4, R11

**Dependencies:** skill-revision/23

**Files:**

```
flow/SKILL.md                       # ## verify, ## quick, ## compound
tests/skills/cases/flow.sh          # procedure checks (unit 18 created the file)
```

**Steps:**

- [ ] Write failing checks: `## verify` keeps Verify → Review → On failure → Receipt → Report and carries the rule "on the third return for the same finding, stop looping"; reviewer routing says confidence ≥80 and ignores severity labels; `## quick` names `tests/**` in the fix surface and matches the machine's `quick.*` states; `## compound` points at spec `SKILL.md` § Wrap-up, keeps `superpowers:finishing-a-development-branch` last and `idle` last of all; no `steal from GSD`, no `CODE IS TRUTH`, no `suggest-superpowers` (control — all absent today).
- [ ] Run `bash tests/skills/run.sh`. Expected FAIL.
- [ ] Implement, one section at a time.
- [ ] Application check [orchestrator]: a fresh sonnet agent at `feature.compound` with SKILL.md. Expected actions: promote dry run first, `finishing-a-development-branch` last.
- [ ] Run the skills and flow suites. Expected PASS.

**Verification:** suites green; the fix-loop and wrap-up greps print 1 or more.

---

## Track E — shipped instructions and repo docs

### skill-revision/25 — Neutral delegation defaults

**Goal:** the shipped `agents-md` channel carries one tier table and no personal policy; this repo keeps its own rules in `vibe.json`.

**Requirements:** R12

**Dependencies:** skill-revision/1

**Files:**

```
flow/content/blocks/delegation/subagents.md, flow/content/blocks/delegation/workflows.md   # neutral text; both stay in the shipped agents-md channel
flow/content/vibe.default.json      # render command strings
vibe.json                           # project block local.delegation-policy + agents-md add
tests/skills/cases/docs.sh
```

**Steps:**

- [ ] Write failing checks: render the `agents-md` channel against the shipped defaults only (a temp project with no `vibe.json`): output names `haiku`, `sonnet`, `opus`, and has no `\bme\b`, `\bmy\b`, `fable`, `ultracode`; the shipped `agents-md` channel still lists `delegation.workflows` (pinned by the port-owned `content.test.mjs`); `vibe.default.json` has no `vibe render agents-md`; this repo's render (with `vibe.json`) still contains the Fable and ultracode rules; `node flow/engine/cli.mjs render --check` exits 0; the render is ≤80 lines.
- [ ] Run. Expected FAIL.
- [ ] Implement per [tech.md](tech.md) § Shipped delegation defaults. Re-render this repo's block: `node flow/engine/cli.mjs render agents-md --write`.
- [ ] Run the skills suite, `node flow/engine/tests/run.mjs content render`, the flow suite. Expected PASS.

**Verification:** suites green; `render --check` 0.

---

### skill-revision/26 — Template and managed block

**Goal:** the shipped AGENTS.md template states true gate and validate facts; this repo's managed block stays byte-identical to it.

**Requirements:** R8, R13

**Dependencies:** skill-revision/25

**Files:**

```
flow/reference/templates/AGENTS.md
AGENTS.md                           # vibe:instructions block re-merged
tests/skills/cases/docs.sh
```

**Steps:**

- [ ] Write failing checks: the template has no `The gate teeth live in`, no `Approvals are enforced by`, no `until F22 ships the binary`; it says "Gates are a convention today: `/flow` asks for a `confirm` token, and no hook or script checks it." and "`set-state.sh` writes the cursor; it does not gate. No hook enforces the two approvals; honor them yourself."; the working-model line is `design → plan →[gate] impl → verify →[gate] compound`; the `Commands` block names `vibe-spec validate` with no legacy hedge; the AGENTS.md `vibe:instructions` block equals the template (`diff`); the template's hand-authored half still names only `setup.apply` as a state.
- [ ] Run. Expected FAIL.
- [ ] Implement in the template; re-merge with `bash flow/scripts/merge-agents.sh` (read-only use of the port's script) or by copying the block between the markers; then re-render `agents-md`.
- [ ] Run the flow suite (template parity legs) and the skills suite. Expected PASS.

**Verification:** suites green; `diff` empty.

---

### skill-revision/27 — Repo docs

**Goal:** README, AGENTS (user-owned sections), flow README and flow AGENTS state the v0.4 tree.

**Requirements:** R13

**Dependencies:** skill-revision/26

**Files:**

```
README.md, AGENTS.md (outside markers), flow/README.md, flow/AGENTS.md, CHANGELOG.md
tests/skills/cases/docs.sh
```

**Steps:**

- [ ] Write failing checks (population floor: 4 files): no file states `(three|four|five|nine|ten) (hook|hooks|scripts|write invariants|that block|that warn)`; none names `validate.sh` as the done-check (AGENTS.md :37 Dogfood row and :108 included); README has no `no build step` / `no runtime` claim (:150) and its built-with badge names Rust; README has no `F1–F24` count and no `create-issues.sh` (:325, :340); flow README has no `no \`/spec\` command` claim (:62; the spec skill is `user-invocable: true`); the file-tool list each doc gives for the flow guard equals the matcher in `flow/scripts/merge-settings.sh` (read it from the script, do not hand-write it); AGENTS.md user-owned sections: Task routing names `docs/superpowers/specs/*-<feature>-design.md` + `plans/*-<feature>.md` as the default and `docs/spec/features/<feature>/` only as the fallback, Repo layout lists `flow/`, `spec/`, `crates/`, `contracts/`, `oracles/`, `tests/` and `docs/superpowers/`, Spec layout names hubs, `lessons.md` and kind folders; README has an `Install` section with `cargo install --locked` for `vibe-spec`, `vibe-spec --version`, and the sentence that `install.sh` does not ship the binary yet; README's shipped heading says `v0.3.x, plus the vibe-spec dev build`; flow README script table lists every file in `flow/scripts/` (derived with `ls`, floor 10); flow AGENTS lists `content/`, `engine/`, `hooks-fallback/`; AGENTS.md Boundaries say `Run vibe-spec validate --strict before claiming work is done`; README and flow README say orders arrive "on the turn after the cursor moves" and that `orders.sh` recovers them.
- [ ] Run. Expected FAIL.
- [ ] Implement, one file at a time; leave the `coding-environment` block in `AGENTS.md` alone (Open Question 13); rewrite Track E's CHANGELOG line.
- [ ] Run the skills and flow suites (`STYLE_SURFACES` still clean). Expected PASS.

**Verification:** suites green.

---

## Tail

### skill-revision/28 — setup, in `flow/SKILL.md` (was `flow/setup.md`)

**Goal:** setup matches the cursor rule, the doctor script and the merge script's WARN.

**Re-targeted (#57):** `flow/setup.md` is gone; its intent lives in `flow/SKILL.md` `## setup` and the `setup.detect` / `setup.apply` orders. The tail trigger has fired — `review-hardening` is DONE in the root plan — so the unit is no longer conditional. Live gaps, measured: neither the section nor the orders mention `doctor.sh` or the `check-skills.sh` loop (`grep -c 'doctor.sh' flow/SKILL.md` prints 0), the four ignore lines, or the `merge-agents.sh` WARN.

**Requirements:** R11

**Dependencies:** skill-revision/24; tail trigger satisfied (`review-hardening` DONE)

**Files:**

```
flow/SKILL.md                       # ## setup and the setup.detect / setup.apply orders
tests/skills/cases/flow.sh          # setup checks (unit 18 created the file)
```

**Steps:**

- [ ] Write failing checks: `## setup` seeds the cursor through the writer and never copies `state.example.json`; it names the four ignore lines (`flow/state.json`, `flow/evidence/`, `flow/warnings.log`, `.vibe/last-inject`); the detect order names `doctor.sh` and the `check-skills.sh` loop; the spec scaffold says "bare spec root → `vibe-spec init`; without the binary, the legacy `setup.sh` (it seeds the legacy root; every later step then uses that root)"; a `merge-agents.sh` WARN means "show both and ask"; no `the user` (control).
- [ ] Run `bash tests/skills/run.sh`. Expected FAIL.
- [ ] Implement in the section and the two orders. Keep the port's line about refusing to replace a symlink or a real file.
- [ ] Run the skills, flow and adapters suites. Expected PASS.

**Verification:** suites green; `grep -c 'doctor.sh' flow/SKILL.md` prints 1 or more.

---

## Close

### skill-revision/29 — Consistency sweep

**Goal:** repo-wide lints pass and cross-track links, anchors and terms agree.

**Requirements:** R1, R2, R13, R14

**Dependencies:** every track merged; skill-revision/28 or its gap recorded

**Files:**

```
tests/skills/cases/sweep.sh
docs/roadmap/README.md              # cons:C-07: delivered rows and the F1–F24 count match docs/spec
AGENTS.md                           # vibe-spec:router block re-synced from agents-fragment.md
any file outside docs/spec/, for straggler fixes found by the sweep (one commit per file); docs/spec/ findings go to unit 31
```

**Steps:**

- [ ] Cursor path to `feature.impl` in `sr/close` (Worktrees and cursors), after recording the plan approval.
- [ ] Write the sweep checks, each with a population floor: `lint_description` over every `SKILL.md`, agent and command file; `lint_frontmatter` over both SKILL.md files; `resolve_links` over `spec/**/*.md` and `flow/**/*.md` in source and in the `.agents/skills` layout; every `§ <Name>` reference names a heading that exists; `fixed_root_hits` over shipped prose = 0; the glossary's `your human partner` rule over agent-facing prose (the `<feature>` and `procedure file` renames apply only to lines a unit already rewrote, so the sweep does not ban `<name>` or `phase file`); bare `scripts/x.sh` without an interpreter = 0 outside orders blocks; no shipped file says `cargo build -p vibe-spec` as the install path; the doc matcher lists equal `merge-settings.sh` (unit 27's check, re-run after the port's unit 19); the `vibe-spec:router` block in `AGENTS.md` equals `crates/vibe-spec/src/templates/agents-fragment.md` (re-sync it from the fragment if unit 15 changed it); budgets.
- [ ] Run. Fix each straggler in its file. Re-run, plus `bash tests/run.sh`, the CI shellcheck command and `bash spec/tests/plugin-validate.sh`. Expected PASS.

**Verification:** skills suite green.

---

### skill-revision/30 — Final verification

**Goal:** prove the whole branch after all merges.

**Requirements:** R15 (and every R by its suite)

**Dependencies:** skill-revision/29

**Files:**

```
(no edits; evidence only, in flow/evidence/feature-skill-revision.md)
```

**Steps:**

- [ ] Cursor `feature.verify`.
- [ ] `bash tests/run.sh`. Expected: every suite PASS (spec, flow, adapters, engine, oracles, contracts, literals, skills).
- [ ] `/bin/bash tests/run.sh` on macOS. Expected: PASS under bash 3.2.
- [ ] `CLAUDE_PROJECT_DIR="$PWD" bash tests/run.sh`. Expected: PASS (hermeticity leg).
- [ ] No-jq leg: build the shim dir as in `.github/workflows/ci.yml` (the "jq stripped from PATH" step), then `PATH="$NOJQ_BIN" VIBE_NO_JQ=1 node flow/engine/tests/run.mjs`. Expected: PASS.
- [ ] Spaced-path leg: tar the tree into `"$(mktemp -d)/with space/vibe"` and run `node flow/engine/tests/run.mjs` there. Expected: PASS.
- [ ] `cargo run -q --locked -p vibe-spec -- validate --strict`. Expected: no errors.
- [ ] `cargo run -q --locked -p vibe-spec -- check-drift`. Expected: the `check-drift: OK` line; the root plan names `skill-revision` as IN PROGRESS.
- [ ] `cargo run -q --locked -p vibe-spec -- migrate --to docs/spec --check`. Expected: exit 0, no literals.
- [ ] `bash oracles/check.sh` and `bash contracts/run.sh`. Expected: PASS; `git diff --stat <unit-1 base> -- oracles contracts/fixtures` is empty.
- [ ] `git ls-files '*.sh' | grep -Ev '^oracles/(bash|js)/' | xargs shellcheck`. Expected: no output.
- [ ] `./build-plugin.sh --check`. Expected: `OK`.
- [ ] Rust: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo deny check`, `cargo test --workspace --locked`. Expected: all pass.
- [ ] `bash tests/parity/run.sh`. Expected: `parity: self-check and candidate green`.
- [ ] `bash tests/openspec/run.sh`. Expected: PASS.
- [ ] `bash spec/tests/agents.sh` and `bash spec/tests/hooks.sh`. Expected: `0 failed`.
- [ ] `bash spec/tests/plugin-validate.sh`. Expected: `PASS`, or the one `SKIP` line without `claude` (then the orchestrator runs it); record which.
- [ ] [orchestrator] Only when `git diff <unit-4 merge>..HEAD -- spec/SKILL.md spec/using-spec/SKILL.md` touches a frontmatter `description` or the Rule/Routing sections: `bash spec/tests/pressure/run.sh --sp both` (24 runs). Expected: every should-trigger and near-miss run PASS. Otherwise record "no trigger text changed since unit 4".
- [ ] `grep -rn "bounded path\|<!-- literal -->\|vibe-flow\|suggest-superpowers\|CRITICAL, WARN, INFO\|gate teeth live\|cargo build -p vibe-spec" spec/SKILL.md spec/agents flow/*.md flow/reference/templates/AGENTS.md crates/vibe-spec/src/templates/agents-fragment.md`. Expected: no output.
- [ ] Whole-branch review [orchestrator]: one `opus` reviewer over `git diff <unit-1 base>..HEAD`; fix must-fix findings in their owning files and re-run the affected rows above.
- [ ] Write the receipt with every command and its real output. Ship gate: show it and stop.

**Verification:** each command above, with its real output in the receipt.

---

### skill-revision/31 — Compound: dogfood docs, lessons, promotion, root row

**Goal:** bring this repo's own spec root in line with what landed, record the lessons, and wrap up the folder by the new sequence.

**Requirements:** R13

**Dependencies:** skill-revision/30 and your human partner's ship approval

**Files:**

```
docs/spec/lessons.md
docs/spec/tech/spec/spec-authoring.md (NEW note), docs/spec/tech/... and docs/spec/concepts/... notes named below
docs/spec/plan.md                    # row → DONE; four follow-up rows at the initial status; folder archived
```

**Steps:**

- [ ] Cursor `feature.compound` (after the ship approval). Invoke `Skill(spec)`.
- [ ] Dogfood fixes (each note's `updated:` bumped): `concepts/spec-vs-implementation.md` delivered list (cons:C-07) and the rubric D2 deviation of `flow/SKILL.md`; `concepts/delivered-features.md`, `concepts/issue-catalog.md` and the F19/F20 status rows in the spec feature-sequence note (cons:C-07); `tech/spec/spec-subagents.md` :20 (the promoter runs only the dry run and the apply; the main thread runs `--done`; spec:N-6); `parts/flow/concepts/style-note.md` (flow:V-23: the style rule rides every turn in `style.ste100`; `flow/SKILL.md` has no § Style); `tech/spec/spec-shipped.md` "the skill and the spec subagents call `vibe-spec`"; `tech/spec/spec-hooks.md` router line "only where a spec root exists"; `tech/contracts/contracts-exit-codes.md` usage exit 2 for the raw-tail commands, and the `state set` row qualified "(target; `state.mjs` checks neither `next` nor the gate today — F7)" (tool:U3); `tech/spec/spec-root-discovery.md` `.vibe/spec.json` key table (cons:C-12); `tech/spec/spec-setup-commands.md` drop `doctor`; `tech/spec.md` "two subagents" (cons:C-24); decision D17 marked superseded by D27 and requirements #7 / design principle 6 suffixed "(v0.4 target; until F7 the /flow command asks and no code enforces)" (cons:C-11); D12 note: orders are authored in `SKILL.md` `## Orders` (cons:C-21).
- [ ] Follow-up rows in `## Branch work`, use the initial status from `spec/reference/plan.md` and create no feature folders (check-drift reads rows only for live folders): `layered-kind-guard` (flow:V-01 / mig:M-05: the write policy for layered kind folders), `session-start-rearm` (flow:V-06 code half), `spec-hooks-install` (tool:T2/T3: spec-hook wiring of `install.sh` targets and the two `doctor` rows), `spec-tool-followups` (spec:N-16(c) `list-specs` on a layered root, tool:U7 validate text-mode fallback, tool:U4 flow-script exit codes in the contract).
- [ ] Lessons (Pattern, Rule, Tags, Date): a description must not carry a sweep marker; shipped prose names the spec root, and a lint now enforces it; a subagent has no channel to the human, so approvals stay in the main thread; an oracle-parity port must still meet the published contract, with each divergence listed.
- [ ] Wrap-up per spec `SKILL.md` § Wrap-up: create the typed note `docs/spec/tech/spec/spec-authoring.md` (`type: tech`, ≤60 lines) and list it in the `children:` of its hub, then promote the two `<!-- merge -->` blocks of [tech.md](tech.md) into it (`vibe-spec promote skill-revision --dry-run --target docs/spec/tech/spec/spec-authoring.md`, show, apply on yes), because the root `tech.md` is a hub that only links; row → DONE; `git mv docs/spec/features/skill-revision docs/spec/archive/skill-revision`; `cargo run -q --locked -p vibe-spec -- validate --strict` and `-- check-drift` until both exit 0 (record the `--pre-merge` result: it fails while the F1, F16–F18 and F25 artifacts remain under `docs/superpowers/`, which those features own); `bash .agents/skills/vibe/scripts/regen-active-rules.sh`; ask to delete the archive.

**Verification:** validate and check-drift output; `git diff docs/spec` shows only the listed notes, lessons, promotions and the row.

---

## Open Questions

Owner decisions. Each has the default this plan implements.

1. **SessionStart router outside spec repos (unit 17).** Default: silent (`{}`) without a spec root; flips the pinned `no_root_still_routes`. Alternative: keep routing everywhere.
2. **Spec description form (unit 3).** Default: keep "You MUST use this before …" (pressure-tested 12/12); fix only `<>`, length and root wording. Alternative: a "Use when" rewrite, which needs a new pressure baseline.
3. **Usage errors exit 2 on raw-tail commands (unit 13).** Default: yes, with parity divergences. Alternative: amend the contract with a legacy exception.
4. **Whole-tag lesson matching (unit 15).** Default: yes (`ci` no longer matches `decision`). Alternative: keep substring matching for oracle parity.
5. **Config consolidation (unit 5).** Default: `.vibe/spec.json` keys `featureLayer`, `suggest`, `superpowers`; legacy `.config.yaml` read only for a missing key; `vibe-flow` dropped. Alternative: keep `.config.yaml` and document it as agent-read only.
6. **Router budget (unit 18).** Default: orders and doctrine stay in `flow/SKILL.md`; budget is 800 words outside the blocks; record the D2 deviation. Alternative: move them out after the port, touching `content.mjs` and six readers.
7. **Policy rule for layered kind folders (flow:V-01).** Default: cut; unit 31 adds `layered-kind-guard` with the initial status, started after the port is DONE. Alternative: a warn-first rule as a second tail unit.
8. **Personal delegation policy (unit 25).** Default: move Fable, ultracode and first-person rules to this repo's `vibe.json`. Alternative: keep them shipped.
9. **Doctrine invariants clause (flow:V-47).** Default: keep it (parity-tested). Alternative: replace with a `decide` pointer and drop the parity legs.
10. **Tail unit 28.** Default: run when the trigger fires before unit 29; otherwise a recorded gap.
11. **`spec-health` model.** Default: keep `haiku` (it relays JSON). Alternative: `sonnet`.
12. **Versions (`package.json` 0.3.0-dev vs Cargo 0.4.0).** Default: no change here; README states the truth. Alternative: bump in this feature.
13. **`coding-environment` block in `AGENTS.md` (cons:S-01).** It cites `/home/agent/projects/AGENTS.md`, which does not exist on other machines, and no script in this repo writes the block. Default: leave it; an outside tool owns it. Alternative: delete it in unit 27.
14. **Row status token.** Default: `IN PROGRESS` for in-flight rows and in the `reference/plan.md` enum, because `check-drift` reads only `DONE` and `IN PROGRESS` and reports `ACTIVE` as the initial status. Alternative: teach `drift.rs` `row_status` to read `ACTIVE` (Track C, one parity divergence).
