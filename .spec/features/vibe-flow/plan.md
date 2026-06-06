---
type: feature-plan
feature: vibe-flow
sibling: tech.md
parent: ../../plan.md
covers: flow state machine, vibe-* skills, inject-source restructure
updated: 2026-06-06
---

# Feature: Vibe Flow — Implementation Plan

Platform-neutral workflow harness: `.agents/flow` state machine + seven `vibe-*`
skill shims. Does not own adapter files or the `spec` document format.

**Parent:** [../../plan.md](../../plan.md)
**Requirements:** [product.md](product.md)
**Architecture:** [tech.md](tech.md)
**Design:** [design.md](design.md)
**Related:** [../platform-adapters/plan.md](../platform-adapters/plan.md) (D12 inject hooks),
[../agent-instructions/plan.md](../agent-instructions/plan.md) (`vibe-setup` adapter steps)

Unit IDs (`VF1`…) are stable (D9). D12 restructure units live in platform-adapters
as `U8` — cited here, not duplicated.

---

## Validation Summary

**Already exists (Stage 1 complete):**
- `.agents/flow/state-machine.json` (15 states), `state.example.json`.
- Scripts: `set-state.sh`, `validate-state.sh`, `detect-context.sh`,
  `regen-active-rules.sh`.
- Seven `vibe-*` skills: setup, strategy, feature, quick, verify, compound, amend.
- Per-state `caveman`, `skill`, `reads`/`writes`, `next`, `exit` in machine.
- D8 lessons retrieval, D9 stable plan IDs, D10/D12 inject model documented.

**Spec ahead of repo (audit 2026-06-06):**
- D12 documented in tech.md but **not implemented** — all states still have frozen
  `inject` strings; no orders blocks in skills. U8/VF1 closes this gap.

**Must build / decide:**
- D12 skill-as-inject-source restructure → platform-adapters U8 (prerequisite for hooks).
- OPEN-2: validate skill count (verify vs compound, setup scope).
- OPEN-6: graceful-degradation script for missing delegated skills.
- D7 (deferred): `feature.deepen` phase.
- Narrow `setup.apply` adapter writes once agent-instructions AI4 lands.

---

## Implementation Roadmap

| Unit | Description | Status | Owner |
|---|---|---|---|
| VF1 | D12 orders blocks in each `vibe-*` skill + machine relink | NOT STARTED | platform-adapters U8 |
| VF2 | OPEN-2 skill-count review (document decision) | NOT STARTED | vibe-flow |
| VF3 | OPEN-6 skill-availability check in flow scripts | NOT STARTED | vibe-flow |
| VF4 | D7 `feature.deepen` (deferred — spec only until dogfood) | DEFERRED | vibe-flow |

---

## VF1 — Skill-as-inject-source (D12)

Implemented as [platform-adapters U8](../platform-adapters/plan.md) — per-state
orders blocks in each `vibe-*` skill; `inject: null` for skill-owning states;
inline fallback for `idle`/`amend` only.

**Done when:** U8 acceptance criteria met (see platform-adapters plan).

---

## VF2 — Skill count (OPEN-2)

- Review whether `vibe-verify` + `vibe-compound` should merge.
- Clarify `vibe-setup` scope: flow/skill bootstrap vs adapter provisioning
  (adapter file work → [agent-instructions](../agent-instructions/plan.md)).
- **Interim:** `setup.apply` machine still writes adapter active-rules until AI4;
  document dual-write window in root plan.
- Record decision in root plan OPEN-2.

**Done when:** decision documented; no contradictory skill descriptions remain.

---

## VF3 — Graceful degradation (OPEN-6)

- Port detect-and-warn skill-availability check into `.agents/flow/scripts/`.
- Include 1-line caveman fallback when caveman plugin absent.

**Done when:** missing `superpowers:*` or subagent warns without hard-failing session.

---

## VF4 — feature.deepen (D7, deferred)

- Optional confidence-gated pass between `plan` and `impl`.
- Do not implement until M5 dogfood completes.

**Done when:** revisited and accepted or rejected with rationale.

---

## Progress

| Unit | Status |
|---|---|
| VF1 | NOT STARTED (→ platform-adapters U8) |
| VF2 | NOT STARTED |
| VF3 | NOT STARTED |
| VF4 | DEFERRED |
