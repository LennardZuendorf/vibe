---
type: feature-product
feature: vibe-flow
sibling: tech.md
parent: ../../product.md
updated: 2026-06-04
---

# Feature: Vibe Flow — Product

The vibe flow is the strict workflow harness. It turns recurring work into
first-class agent skills that route through planning, implementation,
verification, and compounding.

**Parent:** [../../product.md](../../product.md)
**Architecture:** [tech.md](tech.md)
**Design:** [design.md](design.md)

---

## Why this feature exists

The spec framework records decisions, but it does not make agents follow a
workflow. The vibe flow provides the active harness so the user can say "I need
X" and let the flow carry the planning load: it picks the phase, names the
mandatory skill and output path, sets the communication density, and walks the
work through spec → plan → build → TDD-validate. Intent and instinct are encoded
as constraints and injected resources, not left to the agent to remember.

---

## Requirements

| # | Requirement |
|---|---|
| R1 | Canonical flow state lives under `.agents/flow/`. |
| R2 | Workflow shims are agent skills under `.agents/skills/vibe-*`. |
| R3 | Each `vibe-*` skill delegates to existing skills instead of reimplementing them. |
| R4 | Delegation always injects canonical `.spec/` output paths. |
| R5 | States are compound `<flow>.<phase>` keys; the state machine defines transitions, required skills, write surfaces, and exit predicates per state. |
| R6 | Mutable runtime files do not live under `.spec/`; the cursor carries no turn-varying fields. |
| R7 | `vibe-setup` bootstraps or repairs `.agents/flow`, `.agents/skills/vibe-*`, adapters, and baseline specs. |
| R8 | Each state declares a caveman level (`lite`, `full`, `ultra`) for communication density; level definitions are canonical in root `product.md`. |
| R9 | A single inject owner emits one static per-turn inject (skill, write surface, output path, caveman level, next), with safety carve-outs that override density. The orders are sourced from the state's linked `vibe-*` skill, not a hand-written string; skill-less states (`idle`, `amend`) keep an inline fallback. (D10, D12) |
| R10 | `*.design` and `*.triage` states read `.spec/lessons.md` on entry so lessons are retrieved, not just recorded. (D8) |
| R11 | `feature.plan` assigns stable unit IDs (`U1`, `U2`, …); `impl` and `verify` reference them so state survives re-planning. (D9) |

---

## User Experience

The user asks for setup, strategy, a feature, a quick fix, verification, or
compounding. The active agent invokes the matching `vibe-*` skill, which reads
flow state, loads the right specs, delegates to specialized skills, and ends
with evidence and the next legal transition.

---

## Outputs

- `.agents/flow/state-machine.json`
- `.agents/flow/state.example.json`
- `.agents/flow/scripts/*`
- `.agents/skills/vibe-*`

---

## Non-Goals

- Owning the `.spec/` document format
- Owning platform-specific hook syntax
- Replacing Superpowers, `spec`, or review subagents
