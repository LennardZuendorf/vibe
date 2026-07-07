---
type: feature-plan
feature: vibe-flow
sibling: tech.md
parent: ../../plan.md
covers: flow state machine, vibe-* skills, inject-source restructure
updated: 2026-06-18
---

# Feature: Vibe Flow — Implementation Plan

Platform-neutral workflow harness: `.agents/skills/vibe/` state machine + `vibe`
skill shims. A closed, deliverable, testable box. Owns D12 (orders-in-skills);
does not own adapter files or the `spec` document format.

**Parent:** [../../plan.md](../../plan.md)
**Requirements:** [product.md](product.md)
**Architecture:** [tech.md](tech.md)
**Design:** [design.md](design.md)

**Feature gate:** Starts when `spec` is `DONE` (root [plan.md](../../plan.md) Feature
Sequence). Downstream features (`agent-instructions`, then `platform-adapters`)
start only when this feature is `DONE` — they consume frozen artifacts, not units.

Unit IDs are `vibe-flow/n` — assigned once, never renumbered.

---

## Requirements Trace

| ID | Requirement area | Units |
|---|---|---|
| R1, R2, R4, R5, R6, R7, R8, R10, R11 | Flow core — delivered Stage 1 | — |
| R3 | Skill delegation robustness | vibe-flow/3 |
| R9 | Single inject owner; orders sourced from linked skill (D12) | vibe-flow/1 |

---

## Validation Summary

**Already exists (Stage 1 complete):**
- `.agents/skills/vibe/state-machine.json` (15 states), `state.example.json`.
- Scripts: `set-state.sh`, `validate-state.sh`, `detect-context.sh`, `regen-active-rules.sh`.
- Seven `vibe-*` skills: setup, strategy, feature, quick, verify, compound, amend.
- Per-state `caveman`, `skill`, `reads`/`writes`, `next`, `exit` in machine.
- D8 lessons retrieval, D9 stable plan IDs, D10/D12 inject model documented.

**Delivered (Stage 2, 2026-06-18):**
- `vibe-flow/1` — D12 implemented. Every skill-owning state carries `inject: null`;
  orders live in each `vibe-*` skill's `## Orders (D12)` `<!-- vibe:orders:<state> -->`
  block; `.agents/skills/vibe/scripts/orders.sh` resolves the cursor → skill → block and
  interpolates `<feature>`. Verified by `tests/flow/run.sh`.
- `vibe-flow/3` — `.agents/skills/vibe/scripts/check-skills.sh` warns on unverifiable
  delegated skills and prints the caveman fallback; never hard-fails.

**Decided:**
- `vibe-flow/2` — OPEN-2 skill count: **keep all seven** `vibe-*` shims. `vibe-verify`
  and `vibe-compound` stay separate (distinct write surfaces and caveman levels:
  verify writes no specs at `full`; compound writes lessons/root specs at `lite`/`ultra`
  receipts). `vibe-setup` owns flow/skill bootstrap; adapter-file provisioning is
  delegated to `agent-instructions`. No merge.

**Deferred:**
- `vibe-flow/4` — D7 `feature.deepen` (spec-only until dogfood proves the need).

---

## Units

| ID | Seq | Summary | Depends | Status |
|---|---:|---|---|---|
| vibe-flow/1 | 1 | D12 orders blocks in each `vibe-*` skill + machine `inject: null` on skill states + `orders.sh` | — | DONE |
| vibe-flow/2 | 2 | OPEN-2 skill-count review (document decision) | — | DONE |
| vibe-flow/3 | 3 | OPEN-6 skill-availability check in flow scripts (`check-skills.sh`) | — | DONE |
| vibe-flow/4 | — | D7 `feature.deepen` (deferred — spec only until dogfood) | — | DEFERRED |

---

### vibe-flow/1 — Skill-as-inject-source (D12)

**Goal:** per-state orders blocks in each `vibe-*` skill; `inject: null` for
skill-owning states; inline fallback for `idle`/`amend` only.

**Requirements:** R9

**Dependencies:** —

**Done when:** every skill-owning state has its orders in the linked skill, the
machine carries `inject: null` for those states, and a turn injects the linked
skill's orders verbatim (prompt-cache safe). This is the artifact `platform-adapters`
hooks later consume.

---

### vibe-flow/2 — Skill count (OPEN-2)

**Goal:** decide whether `vibe-verify` + `vibe-compound` merge, and clarify
`vibe-setup` scope (flow/skill bootstrap vs adapter provisioning — adapter file
work belongs to `agent-instructions`).

**Requirements:** —

**Dependencies:** —

**Done when:** decision recorded in root plan OPEN-2; no contradictory skill
descriptions remain.

---

### vibe-flow/3 — Graceful degradation (OPEN-6)

**Goal:** detect-and-warn skill-availability check in `.agents/skills/vibe/scripts/`,
with a 1-line caveman fallback when the caveman plugin is absent.

**Requirements:** R3

**Dependencies:** —

**Done when:** a missing `superpowers:*` or subagent warns without hard-failing
the session.

---

### vibe-flow/4 — feature.deepen (D7, deferred)

**Goal:** optional confidence-gated pass between `plan` and `impl`.

**Requirements:** —

**Dependencies:** —

**Done when:** revisited after dogfood and accepted or rejected with rationale.

---

## Progress

| Unit | Status |
|---|---|
| vibe-flow/1 | DONE |
| vibe-flow/2 | DONE |
| vibe-flow/3 | DONE |
| vibe-flow/4 | DEFERRED |

---

## Legacy aliases

One-time map for git grep (old `{PREFIX}{N}` → `feature/n`); do not use for new work:
`VF1` = `vibe-flow/1` (also tracked as legacy platform-adapters `U8`), `VF2` =
`vibe-flow/2`, `VF3` = `vibe-flow/3`, `VF4` = `vibe-flow/4`.
