---
type: feature-plan
feature: vibe-flow
sibling: tech.md
parent: ../../plan.md
covers: flow state machine, vibe-* skills, inject-source restructure
updated: 2026-06-08
---

# Feature: Vibe Flow — Implementation Plan

Platform-neutral workflow harness: `.agents/flow` state machine + seven `vibe-*`
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
- `.agents/flow/state-machine.json` (15 states), `state.example.json`.
- Scripts: `set-state.sh`, `validate-state.sh`, `detect-context.sh`, `regen-active-rules.sh`.
- Seven `vibe-*` skills: setup, strategy, feature, quick, verify, compound, amend.
- Per-state `caveman`, `skill`, `reads`/`writes`, `next`, `exit` in machine.
- D8 lessons retrieval, D9 stable plan IDs, D10/D12 inject model documented.

**Spec ahead of repo (audit 2026-06-08):**
- D12 documented in tech.md but **not implemented** — all states still carry frozen
  `inject` strings; no orders blocks in skills. `vibe-flow/1` closes this gap.

**Must build / decide:**
- `vibe-flow/1` — D12 skill-as-inject-source restructure.
- `vibe-flow/2` — OPEN-2 skill count (verify vs compound, setup scope).
- `vibe-flow/3` — OPEN-6 graceful degradation for missing delegated skills.
- `vibe-flow/4` — D7 `feature.deepen` (deferred).

---

## Units

| ID | Seq | Summary | Depends | Status |
|---|---:|---|---|---|
| vibe-flow/1 | 1 | D12 orders blocks in each `vibe-*` skill + machine `inject: null` on skill states | — | NOT STARTED |
| vibe-flow/2 | 2 | OPEN-2 skill-count review (document decision) | — | NOT STARTED |
| vibe-flow/3 | 3 | OPEN-6 skill-availability check in flow scripts | — | NOT STARTED |
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

**Goal:** detect-and-warn skill-availability check in `.agents/flow/scripts/`,
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
| vibe-flow/1 | NOT STARTED |
| vibe-flow/2 | NOT STARTED |
| vibe-flow/3 | NOT STARTED |
| vibe-flow/4 | DEFERRED |

---

## Legacy aliases

One-time map for git grep (old `{PREFIX}{N}` → `feature/n`); do not use for new work:
`VF1` = `vibe-flow/1` (also tracked as legacy platform-adapters `U8`), `VF2` =
`vibe-flow/2`, `VF3` = `vibe-flow/3`, `VF4` = `vibe-flow/4`.
