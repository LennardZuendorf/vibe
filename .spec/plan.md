---
type: entrypoint
scope: implementation
covers: feature sequence, binary gates, validation criteria, open decisions
children: []
updated: 2026-06-08
---

# vibe — Implementation Plan

Single-purpose repo: build the self-hosting vibe workflow harness. This plan is
current-only — delivered work collapses to notes; no long-horizon backlog (that
lives outside the repo). Cross-feature order is the **Feature Sequence** below,
with binary whole-feature gates. Unit detail (`<feature>/n`) lives in each
feature plan, never here.

**Parent specs:** [product.md](product.md), [tech.md](tech.md), [design.md](design.md)

**Feature plans:**

| Feature | Owns | Plan |
|---|---|---|
| `spec` skill bundle | root `.spec/` + [`.agents/skills/spec/`](../.agents/skills/spec/SKILL.md) | [`tests/spec/run.sh`](../tests/spec/run.sh) |
| [vibe-flow](features/vibe-flow/product.md) | `.agents/flow` + `vibe-*` skills | [plan.md](features/vibe-flow/plan.md) |
| [agent-instructions](features/agent-instructions/product.md) | `AGENTS.md` template + symlinks | [plan.md](features/agent-instructions/plan.md) |
| [platform-adapters](features/platform-adapters/product.md) | plugin + hooks + installer | [plan.md](features/platform-adapters/plan.md) |

---

## Feature Boundaries

Each feature is a closed, deliverable, testable box. Cross-feature coupling is a
whole-feature gate (Feature Sequence), never a unit-to-unit edge.

```text
┌─────────────────────────────────────────────────────────────┐
│  spec skill bundle  .spec/ tree, spec skill, validate, tests │
├─────────────────────────────────────────────────────────────┤
│  vibe-flow          .agents/flow, vibe-* skills, D9–D12      │
├─────────────────────────────────────────────────────────────┤
│  agent-instructions AGENTS.md template, adapter symlinks     │
├─────────────────────────────────────────────────────────────┤
│  platform-adapters  plugin, hooks, /flow, install.sh         │
└─────────────────────────────────────────────────────────────┘
```

| Layer | Owns | Does not own |
|---|---|---|
| **`spec` skill bundle** | `.spec/` docs, templates, `validate.sh`, lesson format, `tests/spec/run.sh` | Flow state, adapters, runtime lesson read |
| **vibe-flow** | State machine, `vibe-*` shims, transitions, D8 read-on-entry, **D12 orders-in-skills** | `AGENTS.md` content, hooks |
| **agent-instructions** | `AGENTS.md` merge, `CLAUDE.md`/`WARP.md` symlinks | Plugin, `.claude/hooks/` |
| **platform-adapters** | Plugin manifest, three hooks, `install.sh` | Instruction file templates, skill bodies |

---

## Feature Sequence

Whole-feature delivery order with **binary** gates — a feature starts only when
its upstream is `DONE`. (D12 now lives inside vibe-flow, so platform-adapters'
hooks consume frozen skills rather than reaching across a boundary.)

| Order | Feature | Deliverable | Test | Status | Starts when |
|---:|---|---|---|---|---|
| 1 | spec | `.spec/` docs + templates + `validate.sh` | `tests/spec/run.sh` | DONE | — |
| 2 | vibe-flow | state machine + `vibe-*` skills + D12 orders | flow scenarios | ACTIVE | spec DONE |
| 3 | agent-instructions | `AGENTS.md` template + merge + symlinks | merge scenarios | BLOCKED | vibe-flow DONE |
| 4 | platform-adapters | plugin + three hooks + `install.sh` | hook dogfood | BLOCKED | agent-instructions DONE |
| 5 | dogfood | strategy/quick/feature arcs on sandbox | manual + lessons | BLOCKED | platform-adapters DONE |

**Active focus:** vibe-flow — close `vibe-flow/1` (D12 orders blocks + machine
`inject: null` on skill states). See [features/vibe-flow/plan.md](features/vibe-flow/plan.md).

---

## Critical Architecture Decisions

### Decided

- **Spec framework remains independent.** Usable without the vibe flow.
- **Root spec model:** `product`, `tech`, `design`, `plan`, `lessons`.
- **Feature spec model:** `product` + `tech` required; `design`, `plan`, `research` optional.
- **Vibe workflow shims** live under `.agents/skills/vibe-*`.
- **Canonical flow state** is platform-neutral (`.agents/flow`).
- **Agent instructions:** `AGENTS.md` is canonical; runtime adapters are symlinks.
- **D8 split:** lesson format → `spec` skill bundle; read-on-entry + tag scan → vibe-flow.
- **Adapters are thin.** Plugin/hooks read `.agents/flow`; they do not own state or spec layout.
- **D12 owned by vibe-flow.** Orders live in `vibe-*` skills; `inject: null` on skill states; platform-adapters consumes the frozen skills.
- **Binary feature gates.** Features couple as whole boxes; no cross-feature unit edges.

### To Resolve

- [ ] **OPEN-2:** Skill count — [vibe-flow/plan.md](features/vibe-flow/plan.md) `vibe-flow/2`.
- [ ] **OPEN-3:** Install mode — copy `.agents/**`; merge via agent-instructions; symlinks opt-in.
- [ ] **OPEN-4 / OPEN-7:** Hook strictness — warn-first; revisit during dogfood.
- [ ] **OPEN-6:** Skill degradation — [vibe-flow/plan.md](features/vibe-flow/plan.md) `vibe-flow/3`.

---

## Spec vs Implementation

| Gap | Owning unit | Notes |
|---|---|---|
| D12 orders-in-skills documented, not implemented | `vibe-flow/1` | all states still carry frozen `inject` strings |
| `vibe:instructions` markers ahead of repo | `agent-instructions/1` | gated on vibe-flow DONE |

Honest drift inventory; shrink as features complete. No backlog beyond work-ready units.

---

## Delivered (history)

Cleansed notes for shipped work — detail lives in live surfaces, not this plan.

- **spec skill bundle — DONE.** Four-layer model, warn-first `validate.sh`, strict
  templates, feature-authoring flow, skill discovery/routing; `tests/spec/run.sh`
  green. `.spec/features/spec-framework/` deleted (truth = skill bundle + tests).
- **vibe-flow Stage 1 — landed.** 15-state machine, four scripts, seven `vibe-*`
  skills, D8–D11 documented. Remaining: `vibe-flow/1` (D12).
- **agent-instructions Stage 1 — landed.** Canonical `AGENTS.md` engineering guide;
  `CLAUDE.md` → `AGENTS.md` symlink. Remaining work gated on vibe-flow DONE.
- **platform-adapters Stage 1 — landed.** `.claude/commands/flow.md`,
  `detect-context.sh`. Remaining work gated on agent-instructions DONE.
