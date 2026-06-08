---
type: entrypoint
scope: implementation
covers: milestones, build sequence, validation criteria, open decisions
children: []
updated: 2026-06-06
---

# vibe — Implementation Plan

**Parent specs:** [product.md](product.md), [tech.md](tech.md), [design.md](design.md)

**Feature plans (unit-level detail lives here, not duplicated below):**

| Feature | Product | Plan | Status |
|---|---|---|---|
| spec-framework | `.spec/` + `spec` skill | [`.agents/skills/spec/`](../.agents/skills/spec/SKILL.md) · [`tests/spec/run.sh`](../tests/spec/run.sh) | DONE (SF0–SF17, wrapped up) |
| [vibe-flow](features/vibe-flow/product.md) | `.agents/flow` + `vibe-*` skills | [plan.md](features/vibe-flow/plan.md) | DONE (VF1→U8, VF2–VF3 open) |
| [agent-instructions](features/agent-instructions/product.md) | `AGENTS.md` template + symlinks | [plan.md](features/agent-instructions/plan.md) | PARTIAL (Stage 1 done; AI0–AI5 open) |
| [platform-adapters](features/platform-adapters/product.md) | plugin + hooks + installer | [plan.md](features/platform-adapters/plan.md) | PARTIAL (Stage 1 done; U8–U7 open) |

---

## Validation Summary

Stage 1 is built: bundled `spec` skill, root `.spec/` docs, `.agents/flow` state
machine + scripts, seven `vibe-*` skill shims, and canonical `AGENTS.md` with
`CLAUDE.md` symlinked. Superseded prior art removed from `.spec/archive/`.

**Spec-vs-repo gap (audit 2026-06-06, updated post-SF16):** D12 orders-in-skills and
`vibe:instructions` markers still ahead of repo. **Partially closed:** spec-framework
SF16 aligned `feature.design`/`feature.plan` inject to spec authoring flow;
`regen-active-rules.sh` now symlink-safe with path dedupe (partial U8 prereq).
Legacy feature plans (vibe-flow, agent-instructions, platform-adapters) not yet
migrated to new `### U1.` + Requirements Trace shape — 8 validate warnings remain.

Core design decision:

> `spec` owns durable planning in `.spec/`; `vibe-*` skills own workflow
> orchestration; `.agents/flow` owns platform-neutral runtime state; agent
> instruction files are provisioned once via [agent-instructions](features/agent-instructions/product.md);
> platform runtimes wire in via [platform-adapters](features/platform-adapters/product.md).

---

## Feature Boundaries

Each feature owns one layer. Cross-feature work cites the owning plan's unit IDs.

```text
┌─────────────────────────────────────────────────────────────┐
│  spec-framework     .spec/ tree, spec skill, validate       │
├─────────────────────────────────────────────────────────────┤
│  vibe-flow          .agents/flow, vibe-* skills, D9–D12     │
├─────────────────────────────────────────────────────────────┤
│  agent-instructions AGENTS.md template, adapter symlinks    │
├─────────────────────────────────────────────────────────────┤
│  platform-adapters  plugin, hooks, /flow, install.sh        │
└─────────────────────────────────────────────────────────────┘
```

| Layer | Owns | Does not own |
|---|---|---|
| **spec-framework** | `.spec/` docs, templates, `validate.sh`, lesson format (D8) | Flow state, adapters, runtime lesson read (→ vibe-flow) |
| **vibe-flow** | State machine, `vibe-*` shims, transitions, D8 read-on-entry | `AGENTS.md` content, hooks |
| **agent-instructions** | `AGENTS.md` merge, `CLAUDE.md`/`WARP.md` symlinks | Plugin, `.claude/hooks/` |
| **platform-adapters** | Plugin manifest, three hooks, `install.sh`, regen dedupe | Instruction file templates |

---

## Critical Architecture Decisions

### Decided

- **Spec framework remains independent.** Usable without the vibe flow.
- **Root spec model:** `product`, `tech`, `design`, `plan`, `lessons`.
- **Feature spec model:** `product` + `tech` required; `design`, `plan`, `research` optional.
- **Vibe workflow shims** live under `.agents/skills/vibe-*`.
- **Canonical flow state** is platform-neutral (`.agents/flow`).
- **Agent instructions:** `AGENTS.md` is canonical; runtime adapters are symlinks.
- **D8 split:** lesson format → spec-framework; read-on-entry + tag scan → vibe-flow.
- **Adapters are thin.** Plugin/hooks read `.agents/flow`; they do not own state or spec layout.
- **Transitions:** `set-state.sh` is writer not gate; agent refuses illegal `next` before calling.
- **D12 target:** orders in skills, `inject: null` on skill states, `idle` inline fallback only.
- **`amend`:** modifier only; cursor never `amend`; inject uses stored cursor state.

### To Resolve

- [x] **OPEN-1:** Mutable cursor gitignored; `state-machine.json` versioned.
- [ ] **OPEN-2:** Skill count — [vibe-flow/plan.md](features/vibe-flow/plan.md) VF2.
- [ ] **OPEN-3:** Install mode — copy `.agents/**`; merge via agent-instructions; symlinks opt-in. U6.
- [ ] **OPEN-4 / OPEN-7:** Hook strictness — warn-first. U7 after M5.
- [x] **OPEN-5:** DESIGN.md validation — local token checks (SF3, always-on) + opt-in `VIBE_DESIGN_LINT=1` → `npx @google/design.md lint` (SF4, network-gated, graceful-degrade). Bash-only contract preserved when unset.
- [ ] **OPEN-6:** Skill degradation — vibe-flow VF3.
- [ ] **D7 (deferred):** `feature.deepen` — VF4 after M5.

---

## Implementation Roadmap

| Milestone | Feature | Goal | Status |
|---|---|---|---|
| M0 | spec-framework | Specs + skill aligned to four-feature model | DONE |
| M1 | vibe-flow | Flow core (state machine + scripts) | DONE |
| M2 | vibe-flow | `vibe-*` skill shims (setup→amend) | DONE |
| M3 | vibe-flow | Verify, compound, amend + lessons loop | DONE |
| M4a | agent-instructions | `AGENTS.md` engineering guide + template init | PARTIAL |
| M4b | platform-adapters | Plugin + hooks + installer (Stage 2) | PARTIAL |
| M5 | all | Dogfood on sandbox project | NOT STARTED |

---

## M0: Spec Framework

**Live surface:** [`.agents/skills/spec/SKILL.md`](../.agents/skills/spec/SKILL.md), root `.spec/` entrypoints, [`tests/spec/run.sh`](../tests/spec/run.sh) (SF0–SF17) — **arc complete, feature folder removed**

- [x] Four-feature spec model, validation, bundled skill (Stage 1).
- [x] **SF0** — lessons `**Tags:**` bootstrap + validate warn.
- [x] **SF1** — `reference/design.md` writing guide + `SKILL.md` link.
- [x] **SF2** — `list-specs.sh` design area + feature folder surfacing.
- [x] **SF3** — local design token checks (offline floor).
- [x] **SF4** — opt-in `VIBE_DESIGN_LINT=1` → `npx @google/design.md lint` (OPEN-5 resolved).
- [x] **SF5** — strict root templates (product/tech/plan).
- [x] **SF6** — strict feature templates + `feature-design.md`.
- [x] **SF7** — guardrail headers on all templates.
- [x] **SF8–SF12** — validate.sh warn-first teeth (Scope, frontmatter, Requirement+Scenario, plan structure, ID traceability).
- [x] **SF13** — README two-layer model.
- [x] **SF14** — SKILL/strategy dedup.
- [x] **SF15** — anti-slop in reference guides.
- [x] **SF16** — feature authoring interview flow (spec-heavy); `vibe-feature` delegates.
- [x] **Tests** — `tests/spec/run.sh` (17/17).
- [x] **Compound** — lesson, OPEN-5, regen digest, regen symlink fix.
- [ ] **Deferred** — migrate legacy feature plans; promote warn→error.

---

## M1–M3: Vibe Flow

**Plan:** [features/vibe-flow/plan.md](features/vibe-flow/plan.md) (VF1–VF4)

- [x] State machine (15 states), four scripts, seven skills, D8–D11 documented.
- [ ] **VF1 (= U8):** D12 orders blocks — spec ahead of repo.
- [ ] **VF2–VF3:** OPEN-2 skill count, OPEN-6 degradation script.
- **Interim:** `setup.apply` still writes adapter active-rules until AI4.

---

## M4a: Agent Instructions

**Plan:** [features/agent-instructions/plan.md](features/agent-instructions/plan.md) (AI0–AI5)

**Stage 1 (done):**

- [x] `AGENTS.md` engineering guide (spec-first, flow routing).
- [x] `CLAUDE.md` → symlink `AGENTS.md`.

**Stage 2:**

- [ ] **AI0:** Wrap dogfood `AGENTS.md` in `vibe:instructions` markers.
- [ ] **AI1:** Template + `adapters.json`.
- [ ] **AI2:** `merge-agents.sh` (wrap, migrate, idempotent).
- [ ] **AI3:** `setup.detect` audit surface.
- [ ] **AI4:** `setup.apply` merge + opt-in symlinks + conditional regen.
- [ ] **AI5:** Five dogfood scenarios.

**Prerequisite:** regen symlink dedupe (U8 scope) — **partially landed** in spec-framework compound (`regen-active-rules.sh` symlink-safe + dedupe); confirm in AI4/U8 acceptance.

---

## M4b: Platform Adapters

**Plan:** [features/platform-adapters/plan.md](features/platform-adapters/plan.md) (U8–U7)

**Stage 1 (done):**

- [x] `.claude/commands/flow.md`.
- [x] `detect-context.sh`, canonical `AGENTS.md`.

**Stage 2:**

- [ ] **U8:** D12 + regen dedupe confirm (partial dedupe landed SF16 compound; blocks U1, unblocks safe AI4/AI5).
- [ ] **U1–U4:** Hooks + `hooks.json`.
- [ ] **U5:** `.claude-plugin/plugin.json`.
- [ ] **U6:** `install.sh` (delegates merge to AI2).
- [ ] **U7:** Dogfood hooks.

---

## M5: Dogfood

- [ ] Strategy, quick, and feature arcs on sandbox project.
- [ ] Record lesson from friction; close OPEN-2, OPEN-4, OPEN-7.

---

## Critical Path

```text
M0 [DONE]
  → M1–M3 [DONE]
  → U8/regen dedupe + AI0 wrap     (parallel entry)
  → AI1 → AI2 → AI4               (agent-instructions)
  → U8 → U1–U4 → U5 → U6 → U7     (platform-adapters; U8 blocks U1)
  → M5
```

**Hard dependencies:**

| Prerequisite | Blocks |
|---|---|
| U8 (D12 + regen dedupe) | U1 inject hook; safe `*.compound` with symlinks |
| AI0 wrap | AI4 no-op on dogfood repo |
| AI2 `merge-agents.sh` | U6 `install.sh` |
| U8 | VF1 acceptance |

---

## Unit ID prefixes (D9)

| Prefix | Feature | Units |
|---|---|---|
| `SF` | spec-framework | SF0–SF16 (arc complete) |
| `VF` | vibe-flow | VF1–VF4 (`VF1` = `U8`) |
| `AI` | agent-instructions | AI0–AI5 |
| `U` | platform-adapters | U8, U1–U7 |

---

## Progress

| Milestone | Status | Sessions |
|---|---|---:|
| M0 | DONE (SF0–SF16) | 2 |
| M1–M3 | DONE | 3 |
| M4a | PARTIAL (Stage 1) | 0.5 |
| M4b | PARTIAL (Stage 1) | 1 |
| M5 | NOT STARTED | 0 |

**Stage 1 complete:** flow runs on guidance alone. **Stage 2 remainder:** D12/regs,
template merge, hooks, installer — then M5 dogfood.
