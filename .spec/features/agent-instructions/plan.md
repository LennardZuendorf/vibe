---
type: feature-plan
feature: agent-instructions
sibling: tech.md
parent: ../../plan.md
covers: vibe-setup AGENTS.md template merge + optional adapter symlinks
updated: 2026-06-18
---

# Feature: Agent Instructions — Implementation Plan

Template-driven `AGENTS.md` provisioning and optional adapter symlinks as part of
`vibe-setup` `setup.apply`. A closed, deliverable, testable box. Supersedes the
constitution-block merge path.

**Parent:** [../../plan.md](../../plan.md)
**Requirements:** [product.md](product.md)
**Architecture:** [tech.md](tech.md)

**Feature gate:** Starts when `vibe-flow` is `DONE` (root [plan.md](../../plan.md)
Feature Sequence) — it builds on the `vibe-setup` skill. `platform-adapters` starts
only when this feature is `DONE` (its installer consumes `merge-agents.sh`). No
cross-feature unit edges.

Unit IDs are `agent-instructions/n` — assigned once, never renumbered.

---

## Requirements Trace

| ID | Requirement area | Units |
|---|---|---|
| R1, R3, R8 | Canonical template + managed markers | agent-instructions/2 |
| R2, R9 | Marker-only merge; deprecate constitution path | agent-instructions/3 |
| R7 | `setup.detect` audit surface | agent-instructions/4 |
| R4, R5, R6, R10 | Optional user-driven symlink adapters | agent-instructions/5 |

---

## Validation Summary

**Already exists:**
- Canonical repo-root `AGENTS.md` engineering guide.
- `CLAUDE.md` → `AGENTS.md` symlink (dogfooded in this repo).
- `vibe-setup` skill with detect/apply and constitution-block template (legacy).
- `regen-active-rules.sh` — symlink-safe writes + resolved-path dedupe (landed).

**Delivered (2026-06-18):**
- Dogfood `AGENTS.md` wrapped in `vibe:instructions` markers; re-merge is a byte no-op.
- `vibe-setup/reference/templates/AGENTS.md` + `reference/adapters.json` (CLAUDE.md, WARP.md).
- `vibe-setup/scripts/merge-agents.sh` — create / replace-in-markers / migrate
  constitution / wrap-equivalent / append-divergent, plus a `link` mode for adapter
  symlinks (skip-correct, relink-wrong, refuse-real-file). Idempotent.
- `vibe-setup` skill `setup.detect` audit surface + `setup.apply` flow rewritten
  off the constitution path onto `merge-agents.sh` + adapter symlinks + regen.
- Five merge/symlink scenarios covered by `tests/adapters/run.sh`.

---

## Units

| ID | Seq | Summary | Depends | Status |
|---|---:|---|---|---|
| agent-instructions/1 | 1 | Wrap-migrate dogfood `AGENTS.md` (unmarked guide → `vibe:instructions`) | — | DONE |
| agent-instructions/2 | 2 | `reference/templates/AGENTS.md` + `reference/adapters.json` | agent-instructions/1 | DONE |
| agent-instructions/3 | 3 | `vibe-setup/scripts/merge-agents.sh` (marker merge + migration) | agent-instructions/2 | DONE |
| agent-instructions/4 | 4 | `setup.detect` audit surface in `vibe-setup` skill | agent-instructions/2 | DONE |
| agent-instructions/5 | 5 | `setup.apply`: merge template, prompt symlinks, conditional regen | agent-instructions/3, agent-instructions/4 | DONE |
| agent-instructions/6 | 6 | Dogfood five scenarios | agent-instructions/5 | DONE |

---

### agent-instructions/1 — Dogfood wrap migration

**Goal:** content-equivalent wrap of this repo's Stage-1 guide — inject
`vibe:instructions` markers around existing content without duplication.

**Requirements:** R2

**Dependencies:** —

**Done when:** live `AGENTS.md` has `vibe:instructions` markers; byte content inside
unchanged; `spec validate.sh` clean.

---

### agent-instructions/2 — Template and manifest

**Goal:** copy the repo `AGENTS.md` into `vibe-setup/reference/templates/AGENTS.md`
(vibe-owned body wrapped in markers; `vibe:active-rules` empty digest below) and add
`reference/adapters.json`.

**Requirements:** R1, R3, R8

**Dependencies:** agent-instructions/1

**Done when:** template validates structurally; markers parseable; `adapters.json`
lists `CLAUDE.md` and `WARP.md`.

---

### agent-instructions/3 — Merge script

**Goal:** `merge-agents.sh [target-repo-root]` — create-if-missing,
replace-inside-markers, wrap-if-unmarked-equivalent, append-if-no-markers (warn),
migrate `vibe:constitution` → `vibe:instructions`. Idempotent.

**Requirements:** R2, R9

**Dependencies:** agent-instructions/2

**Done when:** scripted tests cover fresh file, merge update, user preamble
preserved, legacy constitution migration. (This script is the artifact
`platform-adapters` consumes for its installer.)

---

### agent-instructions/4 — Detect audit

**Goal:** extend `setup.detect` checklist in `vibe-setup/SKILL.md`; report adapter
states from `adapters.json`.

**Requirements:** R7

**Dependencies:** agent-instructions/2

**Done when:** detect output lists `AGENTS.md` health and each adapter row without
writing anything.

---

### agent-instructions/5 — Apply flow

**Goal:** replace the constitution-block step — run `merge-agents.sh`, prompt for
adapter symlinks, `link_adapter` per selection (skip correct symlinks; confirm
before clobbering real files), then `regen-active-rules.sh` (dedupe landed).

**Requirements:** R4, R5, R6, R10

**Dependencies:** agent-instructions/3, agent-instructions/4

**Done when:** `setup.apply` is a no-op on this repo after the wrap; on a fresh
sandbox creates `AGENTS.md` and optional symlinks; never overwrites user preamble;
`CLAUDE.md` symlink survives regen.

---

### agent-instructions/6 — Dogfood

| # | Scenario | Verifies |
|---|---|---|
| 1 | Fresh repo, no `AGENTS.md` | create + symlink opt-in |
| 2 | Existing repo with user preamble above markers | R2 preserve outside markers |
| 3 | Legacy `vibe:constitution` block | migration to `vibe:instructions` |
| 4 | Unmarked dogfood guide (this repo) | wrap, no duplicate body |
| 5 | Adapter target is real file / stale block diff declined | R5, R10, product UX |

**Requirements:** R2, R5, R10

**Dependencies:** agent-instructions/5

**Done when:** five scenarios documented; `spec validate.sh` clean.

---

## Progress

| Unit | Status |
|---|---|
| agent-instructions/1 | DONE |
| agent-instructions/2 | DONE |
| agent-instructions/3 | DONE |
| agent-instructions/4 | DONE |
| agent-instructions/5 | DONE |
| agent-instructions/6 | DONE |

---

## Legacy aliases

One-time map for git grep (old `{PREFIX}{N}` → `feature/n`); do not use for new work:
`AI0` = `agent-instructions/1`, `AI1` = `agent-instructions/2`, `AI2` =
`agent-instructions/3`, `AI3` = `agent-instructions/4`, `AI4` =
`agent-instructions/5`, `AI5` = `agent-instructions/6`.
