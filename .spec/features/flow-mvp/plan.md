---
type: feature-plan
feature: flow-mvp
sibling: tech.md
parent: ../../plan.md
updated: 2026-07-07
---

# Feature: flow-mvp — Implementation Plan

Rework the flow half into the personal operating layer: precedence + contract
blocks make delegation collision-free, the hybrid template makes `.spec/` the
executor's native input, auto-advance removes stage-prompting, and the verify
tooth is the first promoted Stop predicate. Machine/data first, prose second,
teeth third — each unit leaves the suite green.

**Parent:** [../../plan.md](../../plan.md)
**Requirements:** [product.md](product.md)
**Architecture:** [tech.md](tech.md)

**Feature gate:** none upstream — starts on approval of these units. Root
`plan.md` Feature Sequence row is added at `feature.compound` (see product.md
Open Question 1).

---

## Problem Frame

Ten bounded units over data (state machine, deps), prose (SKILL.md, five phase
files, two templates), one hook (stop-gate), and tests. Order: hermetic tests
first (they protect everything after), then machine data, then the grammar and
shims that depend on it, then modes/teeth, cleanup last.

---

## Requirements Trace

| ID | Requirement | Units |
|---|---|---|
| R1 | Precedence contract | flow-mvp/2 |
| R2 | Delegation contract blocks | flow-mvp/5, flow-mvp/6 |
| R3 | Hybrid plan grammar | flow-mvp/4 |
| R4 | Auto-advance, two gates | flow-mvp/3, flow-mvp/7 |
| R5 | Two impl modes | flow-mvp/8 |
| R6 | Verify tooth | flow-mvp/9 |
| R7 | Quick compound | flow-mvp/3, flow-mvp/6 |
| R8 | Caveman demotion | flow-mvp/10 |
| R9 | Single router at idle | flow-mvp/3 |
| R10 | Tests protect the rework | flow-mvp/1, and per-unit tests throughout |

---

## Key Technical Decisions

1. **Tests-first sequencing.** Hermeticity (flow-mvp/1) lands before any
   behavior change so every later unit is verified on a suite that cannot race
   the live cursor. (tech.md D8)
2. **Gates as machine data, advance as prose.** `set-state.sh` stays
   writer-not-gate; auto-advance and the two stops live in orders + phase rules,
   with a consistency test tying them to `"gates"`. (tech.md D2, D5)
3. **Teeth only where state is unambiguous.** The one promoted Stop predicate
   fires only in `*.verify` with a readable cursor; all degrade paths keep
   exit 0. (tech.md D6)

---

## Unit IDs

Units are `flow-mvp/n`, assigned once, never renumbered. Cite in commits/tests
(`feat(flow): flow-mvp/3 …`).

---

### flow-mvp/1 — Hermetic flow test suite

**Goal:** `flow/tests/run.sh` runs entirely in a sandbox copy of `flow/`; the
live cursor is never touched.

**Requirements:** R10

**Dependencies:** —

**Files:**

```
flow/tests/run.sh        # sandbox setup (cp -RL), path rewiring, drop live-cursor backup/restore
```

**Test scenarios:**
- Two concurrent `run.sh` invocations both pass (the part-1 race is dead).
- A live cursor set before the suite is byte-identical after, and never read.

**Verification:** `bash flow/tests/run.sh & bash flow/tests/run.sh & wait` — both
exit 0; `cat flow/state.json` unchanged.

---

### flow-mvp/2 — Precedence section

**Goal:** SKILL.md and the AGENTS.md template carry the when/where-vs-how rule.

**Requirements:** R1

**Dependencies:** —

**Files:**

```
flow/SKILL.md                        # ## Precedence (4 lines, above Orders)
flow/reference/templates/AGENTS.md   # same lines in managed block
```

**Test scenarios:**
- Precedence section present in both; managed-block merge still idempotent.

**Verification:** `grep -c "cursor owns sequencing" flow/SKILL.md
flow/reference/templates/AGENTS.md` → 1 each; adapters suite green.

---

### flow-mvp/3 — Machine data: gates, quick.compound, router hygiene

**Goal:** `"gates"` field; `quick.compound` state + `quick.verify` back-edge;
idle and setup.apply delegate cleanup; policy allows lessons in quick.compound.

**Requirements:** R4, R7, R9

**Dependencies:** flow-mvp/1

**Files:**

```
flow/state-machine.json          # gates, quick.compound, next edits, delegate removals
flow/scripts/detect-context.sh   # quick.compound in lessons allow list
flow/tests/run.sh                # machine-consistency + policy assertions
```

**Test scenarios:**
- `gates` keys are known states; `quick.compound` reachable from `quick.verify`;
  `quick.verify.next` includes `quick.fix`.
- `detect-context.sh decide .spec/lessons.md quick.compound` → allow;
  `… idle` → block.
- Machine delegates no longer include using-superpowers (idle) or
  writing-skills (setup.apply).

**Verification:** `bash flow/tests/run.sh` green with new assertions;
`bash flow/scripts/validate-state.sh` on a `quick.compound` cursor passes.

---

### flow-mvp/4 — Hybrid plan template

**Goal:** feature-plan template gains the agentic-workers header,
`## Global Constraints`, and per-unit `**Steps:**` checkboxes; units stay
canonical.

**Requirements:** R3

**Dependencies:** —

**Files:**

```
spec/reference/templates/feature-plan.md   # additions per tech.md D4
spec/tests/run.sh                          # template structure assertions
```

**Test scenarios:**
- Template contains header line, Global Constraints, `**Steps:**` with the
  5-step TDD checkbox shape; `{name}/n` unit sections unchanged.

**Verification:** `bash spec/tests/run.sh` green; `validate.sh` 0 errors.

---

### flow-mvp/5 — Contract blocks: strategy + feature shims

**Goal:** rewrite delegation sites in `strategy.md` and `feature.md` to the
contract-block shape (offer / inject / redirect / skip).

**Requirements:** R2

**Dependencies:** flow-mvp/2, flow-mvp/4

**Files:**

```
flow/strategy.md   # brainstorm seam: dialogue-only scope, skip doc-write+commit+handoff
flow/feature.md    # design seam (redirect into feature docs; parallel explorers/architects),
                   # plan seam (hybrid template via storage-location override)
flow/tests/run.sh  # machine-delegates ⊆ phase-file-mentions assertion
```

**Test scenarios:**
- Every delegate in machine states strategy.*/feature.design/feature.plan is
  named in the corresponding phase file.
- Brainstorm block contains the skip of upstream write+commit.

**Verification:** `bash flow/tests/run.sh` green; manual read of both files
against tech.md D3 shape.

---

### flow-mvp/6 — Contract blocks: quick, verify, compound shims

**Goal:** same rework for `quick.md`, `verify.md`, `compound.md`: unified review
protocol, findings-route-to-impl rule, finishing-a-development-branch boundary
fix + last-in-sequence, `quick.compound` procedure.

**Requirements:** R2, R7

**Dependencies:** flow-mvp/3

**Files:**

```
flow/quick.md      # quick.compound step; contract blocks
flow/verify.md     # one review dispatch protocol; no fixes in verify
flow/compound.md   # boundary note matching spec/feature.md; finishing sequenced last
flow/SKILL.md      # orders block for quick.compound
flow/tests/run.sh  # delegates ⊆ prose for quick.*/…verify/…compound; orders block exists
```

**Test scenarios:**
- `orders.sh` on a `quick.compound` cursor returns a non-empty block ≤ 400 B.
- compound.md no longer implies the upstream skill performs the archive move.

**Verification:** `bash flow/tests/run.sh` green; `bash flow/scripts/orders.sh`
under a quick.compound cursor prints the new block.

---

### flow-mvp/7 — Auto-advance rule

**Goal:** replace confirm-every-transition prose with auto-advance at non-gated
exits; orders for the two gated states carry `gate:` markers.

**Requirements:** R4

**Dependencies:** flow-mvp/3, flow-mvp/5, flow-mvp/6

**Files:**

```
flow/{strategy,feature,quick,verify}.md   # rule text swap
flow/SKILL.md                             # gate: plan-approval / ship-approval in 2 blocks
flow/tests/run.sh                         # gates ↔ orders consistency; byte budget
```

**Test scenarios:**
- Exactly the two `gates` states' orders contain `gate:`; no other block does.
- All orders blocks ≤ 400 bytes.

**Verification:** `bash flow/tests/run.sh` green.

---

### flow-mvp/8 — Impl modes (interactive | handover)

**Goal:** `feature.md` impl section offers executing-plans vs SDD; SDD contract
block (runtime stays `.superpowers/**`, stop before finishing, exit to verify);
machine delegates updated.

**Requirements:** R5

**Dependencies:** flow-mvp/5, flow-mvp/7

**Files:**

```
flow/feature.md            # mode fork + SDD contract block
flow/state-machine.json    # feature.impl.delegates + subagent-driven-development
flow/SKILL.md              # feature.impl orders mention both modes (within budget)
```

**Test scenarios:**
- Machine ⊆ prose assertion still green with the new delegate.
- feature.impl orders ≤ 400 B and name both modes.

**Verification:** `bash flow/tests/run.sh` green.

---

### flow-mvp/9 — Evidence receipt + verify tooth

**Goal:** receipt convention + stop-gate blocks in `*.verify` on missing/stale
receipt; gitignore stanza covers `evidence/`.

**Requirements:** R6

**Dependencies:** flow-mvp/1, flow-mvp/6

**Files:**

```
.claude/hooks/stop-gate.sh   # promoted predicate per tech.md D6
flow/verify.md               # step: write the receipt (path + shape)
install.sh                   # gitignore line for .agents/skills/vibe/evidence/
flow/tests/run.sh            # block-missing / pass-fresh / block-stale
flow/tests/adapters/run.sh   # hook-level: exit 2 vs 0 against a real install
```

**Test scenarios:**
- In `feature.verify` sandbox: no receipt → exit 2; fresh receipt → exit 0;
  touch `src/x` after receipt → exit 2. In `idle`: always exit 0.

**Verification:** `bash flow/tests/run.sh && bash flow/tests/adapters/run.sh`
green; manual hook probe with a crafted cursor.

---

### flow-mvp/10 — Caveman demotion + docs sync

**Goal:** caveman out of deps.json/doctor; attribution note; flow README rows
(dependency table, script count, adapters default wording) synced with this
feature's reality.

**Requirements:** R8

**Dependencies:** flow-mvp/3

**Files:**

```
flow/reference/deps.json   # remove caveman entry
flow/state-machine.json    # $comment attribution line
flow/README.md             # dep table, "nine scripts", defaults wording
README.md                  # dependency table row
flow/tests/run.sh          # doctor output lacks dep.caveman; manifest valid
```

**Test scenarios:**
- `doctor.sh` prints superpowers + feature-dev dep rows only, still exit 0.
- `check-skills.sh caveman full` still prints the frozen definition.

**Verification:** `bash flow/tests/run.sh` green; `bash flow/scripts/doctor.sh`
inspected.

---

## Dependencies

| Unit | Blocks | Blocked by |
|---|---|---|
| flow-mvp/1 | 3, 9 | — |
| flow-mvp/2 | 5 | — |
| flow-mvp/3 | 6, 7, 10 | 1 |
| flow-mvp/4 | 5 | — |
| flow-mvp/5 | 7, 8 | 2, 4 |
| flow-mvp/6 | 7, 9 | 3 |
| flow-mvp/7 | 8 | 3, 5, 6 |
| flow-mvp/8 | — | 5, 7 |
| flow-mvp/9 | — | 1, 6 |
| flow-mvp/10 | — | 3 |

---

## Progress

| Unit | Status |
|---|---|
| flow-mvp/1 | NOT STARTED |
| flow-mvp/2 | NOT STARTED |
| flow-mvp/3 | NOT STARTED |
| flow-mvp/4 | NOT STARTED |
| flow-mvp/5 | NOT STARTED |
| flow-mvp/6 | NOT STARTED |
| flow-mvp/7 | NOT STARTED |
| flow-mvp/8 | NOT STARTED |
| flow-mvp/9 | NOT STARTED |
| flow-mvp/10 | NOT STARTED |

---

## Open Questions

1. **Receipt shape** — minimal (commands + exit codes) vs per-unit verdict
   table? Recommendation: per-unit table for `feature.verify`, minimal for
   `quick.verify`; decide at flow-mvp/9.
2. **`gates` granularity** — marker on source state (current design) vs on the
   edge. State-level is simpler and both gated states have a single gated exit;
   revisit only if a state ever needs mixed gated/ungated exits.
