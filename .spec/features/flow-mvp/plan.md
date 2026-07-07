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

**Goal:** `flow/tests/run.sh` runs entirely in a sandbox; the live cursor is
never touched. Sandbox preserves the path-parity subject: `sandbox/flow/`
(copied) + `sandbox/.agents/skills/vibe → ../../flow` (symlink recreated) +
`sandbox/.spec` marker — no bare `cp -RL` deref (it would kill the
real-vs-symlink duality the parity tests exercise).

**Requirements:** R10

**Dependencies:** —

**Files:**

```
flow/tests/run.sh        # sandbox layout per tech.md D8, path rewiring, drop live-cursor backup/restore
```

**Test scenarios:**
- Two concurrent `run.sh` invocations both pass (the part-1 race is dead).
- A live cursor set before the suite is byte-identical after, and never read.

**Verification:** `bash flow/tests/run.sh & bash flow/tests/run.sh & wait` — both
exit 0; `cat flow/state.json` unchanged.

---

### flow-mvp/2 — Precedence section + ambient alignment

**Goal:** SKILL.md and the AGENTS.md template carry the when/where-vs-how rule
(incl. abort legality); the ambient stack stops contradicting R4 — template
"Ask first" transition rows become gated-edges-only, Prime-Directive CONFIRM
points at the two gates, `/flow` command drops stop-per-transition.

**Requirements:** R1, R4

**Dependencies:** —

**Files:**

```
flow/SKILL.md                        # ## Precedence (5 lines, above Orders)
flow/reference/templates/AGENTS.md   # precedence lines; Ask-first rewrite; CONFIRM alignment
.claude/commands/flow.md             # drop "do NOT start the new state's work" line
```

**Test scenarios:**
- Precedence section present in both; managed-block merge still idempotent.
- Template no longer says "Ask first: State transitions" unqualified.

**Verification:** `grep -c "cursor owns sequencing" flow/SKILL.md
flow/reference/templates/AGENTS.md` → 1 each; adapters suite green.

---

### flow-mvp/3 — Machine data: gates, quick.compound, router hygiene

**Goal:** `"gates"` field keyed by **edge** (`feature.plan>feature.impl`,
`feature.verify>feature.compound`); abort edges (`idle` in `next` of
feature.design/impl/verify + strategy.brainstorm); `quick.compound` state +
`quick.verify` back-edge; idle and setup.apply delegate cleanup; policy allows
lessons in quick.compound.

**Requirements:** R4, R7, R9

**Dependencies:** flow-mvp/1

**Files:**

```
flow/state-machine.json          # gates (edges), abort edges, quick.compound, delegate removals
flow/scripts/detect-context.sh   # quick.compound in lessons allow list
flow/tests/run.sh                # machine-consistency + policy assertions
```

**Test scenarios:**
- `gates` keys parse as `<known-state>><known-state>` edges present in `next`;
  `quick.compound` reachable from `quick.verify`; `quick.verify.next` includes
  `quick.fix`; `idle` ∈ `next` of the four mid-arc states.
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
contract-block shape (announce / inject / redirect / skip — non-blocking,
`suggest-superpowers: false` = standing decline, redirect/skip copied into
subagent prompts).

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
fix + last-in-sequence, `quick.compound` procedure (conditional — default exit
is `idle`), escalation announce-and-confirm in quick.triage, and fix
`compound.md`'s "only state where lessons.md and root specs are writable"
sentence (false once quick.compound exists).

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
edges; orders for the two gated states carry `gate:` markers (plan gate
includes mode pick).

**Requirements:** R4

**Dependencies:** flow-mvp/3, flow-mvp/5, flow-mvp/6

**Files:**

```
flow/{strategy,feature,quick,verify}.md   # rule text swap (+ escalation confirm exception)
flow/SKILL.md                             # gate: plan-approval+mode / ship-approval in 2 blocks
flow/tests/run.sh                         # gate edges ↔ orders markers; byte budget
```

**Test scenarios:**
- Exactly the source states of the two `gates` edges carry `gate:` in orders;
  no other block does; markers match the machine's edge keys.
- All orders blocks ≤ 400 bytes.

**Verification:** `bash flow/tests/run.sh` green.

---

### flow-mvp/8 — Impl modes (interactive | handover)

**Goal:** `feature.md` impl section offers executing-plans vs SDD, chosen at
the plan gate (default interactive); both contract blocks inject the
**current-branch, no-worktree** stance so verify and the receipt run against
the same tree; SDD block adds runtime-stays-`.superpowers/**` + stop before
finishing + exit to verify; machine delegates updated.

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

**Goal:** receipt convention (fixed names `evidence/feature-<feature>.md`,
`evidence/quick.md`) + stop-gate blocks in `*.verify` on missing/stale receipt;
git-derived staleness (changed files per `git status --porcelain` newer than
receipt; existence-only without git); `stop_hook_active` pass-through; block
message names the abort hatch `set-state.sh idle`; gitignore covers evidence in
installer **and** this repo's root `.gitignore` (`flow/evidence/`).

**Requirements:** R6

**Dependencies:** flow-mvp/1, flow-mvp/6

**Files:**

```
.claude/hooks/stop-gate.sh   # promoted predicate per tech.md D6 (as amended)
flow/verify.md               # step: write the receipt (path + shape)
install.sh                   # gitignore line for .agents/skills/vibe/evidence/
.gitignore                   # flow/evidence/ (dogfood physical path)
flow/tests/run.sh            # block-missing / pass-fresh / block-stale / stop_hook_active
flow/tests/adapters/run.sh   # hook-level: exit 2 vs 0 against a real install
```

**Test scenarios:**
- In `feature.verify` sandbox (git repo): no receipt → exit 2; fresh receipt →
  exit 0; modify a tracked file after receipt → exit 2; `stop_hook_active` set
  → exit 0 regardless. In `idle`: always exit 0. No-git sandbox: existence-only.

**Verification:** `bash flow/tests/run.sh && bash flow/tests/adapters/run.sh`
green; manual hook probe with a crafted cursor.

---

### flow-mvp/10 — Caveman demotion + docs sync

**Goal:** caveman out of deps.json/doctor; attribution note; flow README rows
(dependency table, script count, adapters default wording) synced with this
feature's reality; OQ1 fix — `spec/feature.md` step 5 moves the root
Feature-Sequence row to the compound promote step (write-policy-legal).

**Requirements:** R8

**Dependencies:** flow-mvp/3

**Files:**

```
flow/reference/deps.json   # remove caveman entry
flow/state-machine.json    # $comment attribution line
flow/README.md             # dep table, "nine scripts", defaults wording
README.md                  # dependency table row
spec/feature.md            # step 5: root plan row at compound, not at plan (OQ1)
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

*(Resolved: gates are keyed by edge — `feature.verify` has three exits and only
the compound edge is gated; the state-level design's "single gated exit"
premise was false. Adversarial review 2026-07-07.)*
