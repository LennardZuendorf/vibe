---
type: entrypoint
scope: implementation
covers: feature sequence, binary gates, validation criteria, open decisions
children: []
updated: 2026-07-03
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
| 1 | spec | `.spec/` docs + templates + `validate.sh` | `tests/spec/run.sh` (44) | DONE | — |
| 2 | vibe-flow | state machine + `vibe-*` skills + D12 orders | `tests/flow/run.sh` (26) | DONE | spec DONE |
| 3 | agent-instructions | `AGENTS.md` template + merge + symlinks | `tests/adapters/run.sh` | DONE | vibe-flow DONE |
| 4 | platform-adapters | plugin + three hooks + `install.sh` | `tests/adapters/run.sh` (39) | DONE | agent-instructions DONE |
| 5 | dogfood | hook/merge/install behaviours + earn-the-teeth | scripted + lessons | DONE | platform-adapters DONE |
| 6 | [monorepo-split](features/monorepo-split/plan.md) | `spec/`+`flow/` split + symlinks + truth sweep + orphan compound | suites + validate + grep evidence | NOT STARTED | — |
| 7 | [install-tooling](features/install-tooling/plan.md) | `--only`/`--dry-run`/`--uninstall`, `doctor.sh`, `deps.json` | `tests/adapters/run.sh` + `tests/flow/run.sh` | NOT STARTED | monorepo-split DONE |
| 8 | [release-docs](features/release-docs/plan.md) | READMEs + rails + logo + examples + stranger eval + PR | CI + eval report | NOT STARTED | install-tooling DONE |

**Active focus:** release-polish branch (2026-07-03 overnight) — features 6–8
finalize the repo for public shareability. Prior five features remain DONE and
self-hosting. Still deferred: real-world earn-the-teeth promotions and
`vibe-flow/4` `feature.deepen`.

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

### Resolved

- [x] **OPEN-2:** Skill count — keep all seven `vibe-*` shims; `vibe-verify` and
  `vibe-compound` stay separate (distinct write surfaces + caveman). `vibe-flow/2`.
- [x] **OPEN-3:** Install mode — `install.sh` **copies** core `.agents/**` + Claude
  adapter, seeds+gitignores the cursor, merges `AGENTS.md` via `merge-agents.sh`,
  symlinks adapters opt-in (`--adapters`), idempotent. `platform-adapters/6`.
- [x] **OPEN-4 / OPEN-7:** Hook strictness — shipped warn-first; only the three
  pre-existing `detect-context.sh` hard blocks deny; every `Stop` predicate is
  warn-only with a `TODO(earn-the-teeth)`. `platform-adapters/3`.
- [x] **OPEN-6:** Skill degradation — `.agents/flow/scripts/check-skills.sh` warns on
  unverifiable delegates + prints the caveman fallback; never hard-fails. `vibe-flow/3`.

---

## Spec vs Implementation

No open drift. The two former gaps are closed:

| Former gap | Owning unit | Resolution |
|---|---|---|
| D12 orders-in-skills documented, not implemented | `vibe-flow/1` | orders live in each skill (`## Orders`); machine `inject: null`; `orders.sh` resolves; tested |
| `vibe:instructions` markers ahead of repo | `agent-instructions/1` | repo `AGENTS.md` wrapped + driven from the template via `merge-agents.sh` |

Honest drift inventory; shrink as features complete. No backlog beyond work-ready units.

---

## Delivered (history)

Cleansed notes for shipped work — detail lives in live surfaces, not this plan.

- **spec skill bundle — DONE.** Four-layer model, warn-first `validate.sh`, strict
  templates, feature-authoring flow, skill discovery/routing; `tests/spec/run.sh`
  green. `.spec/features/spec-framework/` deleted (truth = skill bundle + tests).
- **vibe-flow — DONE.** 15-state machine, six scripts (`set-state`, `validate-state`,
  `detect-context`, `regen-active-rules`, `orders`, `check-skills`), seven `vibe-*`
  skills. D12 orders sourced from each skill via `orders.sh`; D8–D11 in place.
  `tests/flow/run.sh` green. Specs kept as living architecture docs (root entrypoints
  link them as children) rather than archived.
- **agent-instructions — DONE.** Canonical `AGENTS.md` template + `adapters.json` under
  `vibe-setup/reference/`; `merge-agents.sh` (marker merge + constitution migration +
  adapter symlinks); `vibe-setup` detect/apply rewritten off the constitution path.
  Repo `AGENTS.md` wrapped in `vibe:instructions`; `CLAUDE.md` → `AGENTS.md`.
- **platform-adapters — DONE.** Three hooks (inject/guard/gate) as thin shells over
  `.agents/flow/scripts/`; `hooks.json`; `.claude-plugin/plugin.json`; `install.sh`.
  Warn-first; graceful-degrade (exit 0). `tests/adapters/run.sh` green.
- **dogfood — DONE (scripted).** Hook block/warn/allow/graceful, merge scenarios, and a
  full `install.sh` into a sandbox are exercised in `tests/adapters/run.sh`; the build
  itself was the end-to-end arc. Real-session earn-the-teeth promotions stay future work.
