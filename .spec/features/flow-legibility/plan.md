---
type: feature-plan
feature: flow-legibility
sibling: tech.md
parent: ../../plan.md
updated: 2026-07-18
---

# Feature: flow-legibility — Implementation Plan

Six units, smallest-and-safest first: loop edges (data only), then imperative
orders, tier pins, the doctrine source + resolver, the SessionStart hook wiring,
and finally drift inference. Each unit is TDD'd against the existing bash suites
and preserves the warn-first / byte-stable / jq-optional invariants.

**Parent:** [../../plan.md](../../plan.md)
**Requirements:** [product.md](product.md)
**Architecture:** [tech.md](tech.md)

**Feature gate:** First 2026-07 rework feature (root [plan.md](../../plan.md) row 10). Unblocks delegation-redirect and vibe-plugin; depends on no other feature's units.

---

## Requirements Trace

| ID | Requirement | Units |
|---|---|---|
| R1 | [Self-carrying orders](product.md#requirement-self-carrying-orders-r1) | flow-legibility/2 |
| R2 | [SessionStart doctrine hook](product.md#requirement-sessionstart-doctrine-hook-r2) | flow-legibility/4, flow-legibility/5 |
| R3 | [Loop edges](product.md#requirement-loop-edges-r3) | flow-legibility/1 |
| R4 | [Drift-first nudges](product.md#requirement-drift-first-nudges-r4) | flow-legibility/6 |
| R5 | [Model-tier pins](product.md#requirement-model-tier-pins-r5) | flow-legibility/3 |
| R6 | [Enforcement pattern preserved](product.md#requirement-enforcement-pattern-preserved-r6) | all |

---

## Key Technical Decisions

1. **Data before behavior.** Loop edges (machine JSON) land first — pure data, unblocks the orders rewrite that references `next`.
2. **Resolver before hook.** `doctrine.sh` (unit 4) ships and is tested before the SessionStart hook (unit 5) that shells over it — mirrors the orders.sh/inject split.
3. **Single-source by parity test.** The `<!-- vibe:doctrine -->` block is canonical; a test asserts the AGENTS.md template carries the same invariant lines, so no divergent copy survives.
4. **Drift stays warn-only.** `infer` never blocks and only fires on clear cursor/activity contradiction; a false positive costs one advisory line.

---

## Global Constraints

<!-- Executors: read before starting any unit. -->

- Test command: `bash flow/tests/run.sh` (core) and `bash flow/tests/adapters/run.sh` (hooks/install); `bash tests/run.sh` runs both. Keep every existing assertion green.
- Preserve invariants (R6): orders blocks ≤400 bytes, byte-stable, jq/no-jq byte-identical; only the three `detect-context.sh` hard blocks deny; every hook exits 0 on a missing keystone; machine `delegates` ⊆ phase-file prose; gate↔orders consistency.
- TDD: extend the suite first (red), implement (green), re-run. Cite the unit ID in the commit (`feat(flow): flow-legibility/n ...`).
- Edit `flow/` (canonical), never the `.agents/skills/vibe` symlink target directly.
- Build on `claude/vibe-framework-rework-hy1xp5` (designated branch), not a new `feature/` branch.

---

### flow-legibility/1 — Loop edges + research artifact

**Goal:** machine gains `strategy.spec→strategy.brainstorm` and `feature.plan→feature.design`; `research.md` is a first-class `feature.design` write. No new state/phase.

**Requirements:** R3, R6

**Dependencies:** —

**Files:**

```
flow/state-machine.json    # +2 next edges; research.md in feature.design.writes
flow/tests/run.sh          # update strategy.spec.next test; assert both new edges
```

**Test scenarios:**

- `strategy.spec.next == [strategy.brainstorm, idle]` (updated from `[idle]`)
- `feature.plan.next` contains `feature.design`
- `research.md` present in `feature.design.writes`
- gates↔next, idle-abort sweep, every-next-target-known all stay green

**Verification:** `bash flow/tests/run.sh` green.

---

### flow-legibility/2 — Self-carrying imperative orders

**Goal:** every orders block (and the `idle` inline inject) states the transition command imperatively — `set-state.sh <next>` (non-gated) / `/flow <next> confirm` (gated) — within the 400-byte budget, keeping `gate:` markers.

**Requirements:** R1, R6

**Dependencies:** flow-legibility/1

**Files:**

```
flow/SKILL.md              # rewrite each vibe:orders:* block tail
flow/state-machine.json    # idle inline inject → imperative
flow/tests/run.sh          # imperative-form + byte-budget assertions
```

**Test scenarios:**

- Every non-gated state's orders block contains `set-state.sh <next>`
- Every gated-source block (`feature.plan`, `feature.verify`) contains `/flow <next> confirm` and keeps `gate:`
- Every orders block ≤400 bytes; jq/no-jq byte-identical (existing parity test)

**Verification:** `bash flow/tests/run.sh` green.

---

### flow-legibility/3 — Model-tier pins

**Goal:** every delegation contract block that dispatches a subagent names a tier — `code-explorer`→sonnet, `code-architect`/`code-reviewer`→opus — plus a one-line policy in SKILL.md.

**Requirements:** R5, R6

**Dependencies:** —

**Files:**

```
flow/feature.md            # code-explorer/code-architect block → +model line
flow/verify.md             # code-reviewer block → +model line
flow/SKILL.md              # one-line subagent model-tier policy
flow/tests/run.sh          # assert tiers present; machine⊆prose still holds
```

**Test scenarios:**

- `code-explorer` dispatch names sonnet; `code-architect` and `code-reviewer` name opus
- SKILL.md carries the tier policy line
- Delegate-parity (machine `delegates` ⊆ phase file) unchanged

**Verification:** `bash flow/tests/run.sh` green.

---

### flow-legibility/4 — Doctrine block + resolver

**Goal:** a `<!-- vibe:doctrine -->` block in SKILL.md and `flow/scripts/doctrine.sh` that emits it plus a cursor summary; the AGENTS.md template mirrors the same invariant lines (parity-tested).

**Requirements:** R2, R6

**Dependencies:** —

**Files:**

```
flow/SKILL.md                       # <!-- vibe:doctrine --> block
flow/scripts/doctrine.sh            # NEW resolver (self-location + jq-optional, mirrors orders.sh)
flow/reference/templates/AGENTS.md  # session-start/working-model lines aligned to the block
flow/tests/run.sh                   # doctrine.sh output + parity + no-jq
```

**Test scenarios:**

- `doctrine.sh` emits a non-empty block containing the write-invariant + two-gate + session-start-read lines
- Cursor summary reflects `state.json` (feature.design/flow-legibility vs idle)
- jq and no-jq output byte-identical
- AGENTS.md template contains the same invariant lines (single-source parity)

**Verification:** `bash flow/tests/run.sh` green.

---

### flow-legibility/5 — SessionStart hook + wiring + doctor coverage

**Goal:** `session-start-doctrine.sh` (thin shell over `doctrine.sh`); `merge-settings.sh` adds an idempotent `SessionStart` block (`startup|resume|compact`); repo `.claude/settings.json` wired; `doctor.sh` gains an instruction-coverage check.

**Requirements:** R2, R6

**Dependencies:** flow-legibility/4

**Files:**

```
.claude/hooks/session-start-doctrine.sh   # NEW thin shell
flow/scripts/merge-settings.sh            # +SessionStart block builder
.claude/settings.json                     # +SessionStart wiring
flow/scripts/doctor.sh                    # +instruction-coverage check
flow/tests/adapters/run.sh                # wiring + hook output + doctor assertions
```

**Test scenarios:**

- `merge-settings.sh` on a fresh + existing settings produces a `SessionStart` array; re-run is a no-op; user keys survive
- `session-start-doctrine.sh` emits the doctrine (graceful exit 0 when doctrine.sh absent)
- `doctor.sh` reports coverage ok when the block is present OR SessionStart wired; warns (exit 0) when neither

**Verification:** `bash flow/tests/adapters/run.sh` + `bash flow/tests/run.sh` green.

---

### flow-legibility/6 — Drift-first nudges

**Goal:** `detect-context.sh infer` maps working-tree activity to a likely state; the inject hook prepends the nudge as line 1 only on drift, byte-stable otherwise.

**Requirements:** R4, R6

**Dependencies:** —

**Files:**

```
flow/scripts/detect-context.sh              # +infer subcommand (read-only)
.claude/hooks/user-prompt-submit-inject.sh  # prepend drift nudge line 1 on drift
flow/tests/run.sh                           # infer rule table + no-jq
flow/tests/adapters/run.sh                  # inject order: drift-first vs stable
```

**Test scenarios:**

- `infer` with cursor idle + `src/` change → `drift:feature.impl:...`; consistent activity → no output
- `decide` behavior unchanged (three hard blocks, warn classes)
- inject prepends `vibe-drift:` as line 1 on drift; no drift → orders block first, byte-stable
- no-jq degrade path clean

**Verification:** `bash flow/tests/run.sh` + `bash flow/tests/adapters/run.sh` green.

---

## Progress

| Unit | Status |
|---|---|
| flow-legibility/1 | NOT STARTED |
| flow-legibility/2 | NOT STARTED |
| flow-legibility/3 | NOT STARTED |
| flow-legibility/4 | NOT STARTED |
| flow-legibility/5 | NOT STARTED |
| flow-legibility/6 | NOT STARTED |
