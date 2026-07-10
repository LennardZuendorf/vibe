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
| `spec` skill bundle | root `.spec/` + [`.agents/skills/spec/`](../.agents/skills/spec/SKILL.md) | [`spec/tests/run.sh`](../spec/tests/run.sh) |
| [vibe-flow](features/vibe-flow/product.md) | `.agents/skills/vibe/` + `vibe` skill | [plan.md](features/vibe-flow/plan.md) |
| [agent-instructions](features/agent-instructions/product.md) | `AGENTS.md` template + symlinks | [plan.md](features/agent-instructions/plan.md) |
| [platform-adapters](features/platform-adapters/product.md) | hooks + `/flow` + installer (settings.json wiring) | [plan.md](features/platform-adapters/plan.md) |

---

## Feature Boundaries

Each feature is a closed, deliverable, testable box. Cross-feature coupling is a
whole-feature gate (Feature Sequence), never a unit-to-unit edge.

```text
┌─────────────────────────────────────────────────────────────┐
│  spec skill bundle  .spec/ tree, spec skill, validate, tests │
├─────────────────────────────────────────────────────────────┤
│  vibe-flow          .agents/skills/vibe/, vibe skill, D9–D12      │
├─────────────────────────────────────────────────────────────┤
│  agent-instructions AGENTS.md template, adapter symlinks     │
├─────────────────────────────────────────────────────────────┤
│  platform-adapters  hooks, /flow, install.sh (settings.json) │
└─────────────────────────────────────────────────────────────┘
```

| Layer | Owns | Does not own |
|---|---|---|
| **`spec` skill bundle** | `.spec/` docs, templates, `validate.sh`, lesson format, `spec/tests/run.sh` | Flow state, adapters, runtime lesson read |
| **vibe-flow** | State machine, `vibe` skill phases, transitions, D8 read-on-entry, **D12 orders-in-skills** | `AGENTS.md` content, hooks |
| **agent-instructions** | `AGENTS.md` merge, `CLAUDE.md`/`WARP.md` symlinks | Hook wiring, `.claude/hooks/` |
| **platform-adapters** | `settings.json` wiring, three hooks, `install.sh` | Instruction file templates, skill bodies |

---

## Feature Sequence

Whole-feature delivery order with **binary** gates — a feature starts only when
its upstream is `DONE`. (D12 now lives inside vibe-flow, so platform-adapters'
hooks consume frozen skills rather than reaching across a boundary.)

| Order | Feature | Deliverable | Test | Status | Starts when |
|---:|---|---|---|---|---|
| 1 | spec | `.spec/` docs + templates + `validate.sh` | `spec/tests/run.sh` | DONE | — |
| 2 | vibe-flow | state machine + `vibe` skill + D12 orders | `flow/tests/run.sh` | DONE | spec DONE |
| 3 | agent-instructions | `AGENTS.md` template + merge + symlinks | `flow/tests/adapters/run.sh` | DONE | vibe-flow DONE |
| 4 | platform-adapters | three hooks + `/flow` + `install.sh` (settings.json wiring) | `flow/tests/adapters/run.sh` | DONE | agent-instructions DONE |
| 5 | dogfood | hook/merge/install behaviours + earn-the-teeth | scripted + lessons | DONE | platform-adapters DONE |
| 6 | monorepo-split | `spec/`+`flow/` split + symlinks + truth sweep + orphan compound | suites + validate + grep evidence | DONE | — |
| 7 | [install-tooling](features/install-tooling/plan.md) | `--only`/`--dry-run`/`--uninstall`, `doctor.sh`, `deps.json` | `flow/tests/adapters/run.sh` + `flow/tests/run.sh` | DONE | monorepo-split DONE |
| 8 | [release-docs](features/release-docs/plan.md) | READMEs + rails + logo + examples + stranger eval + PR | CI + eval report | DONE | install-tooling DONE |
| 9 | [flow-mvp](archive/flow-mvp/plan.md) | precedence + contract blocks, hybrid plan grammar, auto-advance + two gates, quick.compound, evidence-receipt verify tooth, caveman demotion | `flow/tests/run.sh` (hermetic sandbox, machine⊆prose, gate↔orders, evidence-gate block/pass) | DONE | — |

**Active focus:** hardening pass on `claude/vibe-repo-review-oszwb4`
(2026-07-09) — a full audit found verified bugs and doc drift; fixes in flight
across the flow engine, installer, spec skill, and docs, plus a compound of the
flow-mvp feature whose `.spec/` memory this pass reconciles. flow-mvp itself
landed 2026-07-08 (merged as PR #14): it reworked the flow half into the
personal operating layer (precedence contract, delegation contract blocks,
auto-advance with two edge-keyed gates, `quick.compound`, and the first promoted
Stop predicate — the evidence-receipt verify tooth). Prior features remain DONE
and self-hosting. Still deferred: real-world earn-the-teeth promotions beyond the
verify tooth, `vibe-flow/4` `feature.deepen`, the deferred flow-mvp methodology
follow-ups (multi-lens review, `/spec research` wiring), and the manual gh repo
metadata + social-preview upload.

---

## Critical Architecture Decisions

### Decided

- **Spec framework remains independent.** Usable without the vibe flow.
- **Root spec model:** `product`, `tech`, `design`, `plan`, `lessons`.
- **Feature spec model:** `product` + `tech` required; `design`, `plan`, `research` optional.
- **Vibe workflow shims** live under `.agents/skills/vibe/`.
- **Canonical flow state** is platform-neutral (`.agents/skills/vibe`).
- **Agent instructions:** `AGENTS.md` is canonical; runtime adapters are symlinks.
- **D8 split:** lesson format → `spec` skill bundle; read-on-entry + tag scan → vibe-flow.
- **Adapters are thin.** Hooks and the `/flow` command read `.agents/skills/vibe`; they do not own state or spec layout.
- **D12 owned by vibe-flow.** Orders live in the `vibe` skill phase files; `inject: null` on skill states; platform-adapters consumes the frozen skill.
- **Repo stores canonical halves at `spec/` + `flow/`.** `.agents/skills/{spec,vibe}` are compat symlinks — the portable runtime interface; installs materialize real dirs (`cp -RL`). `monorepo-split/1`.
- **Binary feature gates.** Features couple as whole boxes; no cross-feature unit edges.

### Resolved

- [x] **OPEN-2:** Skill count — originally seven `vibe-*` shims (`vibe-flow/2`);
  consolidated 2026-06-29 into one `vibe` skill with seven phase files, distinct
  write surfaces + caveman levels preserved per phase (`vibe-skill-consolidation`).
- [x] **OPEN-3:** Install mode — `install.sh` **copies** core `.agents/**` + Claude
  adapter, seeds+gitignores the cursor, merges `AGENTS.md` via `merge-agents.sh`,
  symlinks adapters opt-in (`--adapters`), idempotent. `platform-adapters/6`.
- [x] **OPEN-4 / OPEN-7:** Hook strictness — shipped warn-first; only the three
  pre-existing `detect-context.sh` hard blocks deny; every `Stop` predicate is
  warn-only with a `TODO(earn-the-teeth)`. `platform-adapters/3`.
- [x] **OPEN-6:** Skill degradation — `.agents/skills/vibe/scripts/check-skills.sh` warns on
  unverifiable delegates + prints the caveman fallback; never hard-fails. `vibe-flow/3`.

---

## Spec vs Implementation

The 2026-07-09 hardening truth sweep reconciled the drift the audit found: the
plugin-era claims (`.claude-plugin/plugin.json` + `hooks.json`) were retired
across the root and feature specs — the live Claude wiring is
`.claude/settings.json` (see the platform-adapters lesson); stale
`tests/{spec,flow,adapters}/run.sh` paths were rewritten to the split layout
(`spec/tests/`, `flow/tests/`, `flow/tests/adapters/`); the never-shipped
`examples/todo-api` sample was corrected to deferred; and the flow-mvp feature
(merged PR #14) was compounded — Delivered note added, root Feature Sequence row
recorded, feature folder archived. Hand-written assertion counts were removed
from README/specs and a `spec/scripts/check-drift.sh` gate (CI-wired) now fails
on merged features missing a plan row and on hand-written counts.

The earlier 2026-07-03 truth sweep (`monorepo-split/4`–`/5`) retired stale
`.agents/flow` references and compounded three shipped-but-uncompounded features
(spec-skill-improvements, vibe-skill-consolidation, vibe-flow-collapse).

Honest drift inventory; shrink as features complete. No backlog beyond work-ready units.

---

## Delivered (history)

Cleansed notes for shipped work — detail lives in live surfaces, not this plan.

- **spec skill bundle — DONE.** Four-layer model, warn-first `validate.sh`, strict
  templates, feature-authoring flow, skill discovery/routing; `spec/tests/run.sh`
  green. `.spec/features/spec-framework/` deleted (truth = skill bundle + tests).
- **vibe-flow — DONE.** 15-state machine, six scripts (`set-state`, `validate-state`,
  `detect-context`, `regen-active-rules`, `orders`, `check-skills`), originally seven
  `vibe-*` skills. D12 orders sourced from each skill via `orders.sh`; D8–D11 in place.
  `flow/tests/run.sh` green. Specs kept as living architecture docs (root entrypoints
  link them as children) rather than archived.
- **spec-skill-improvements — DONE (2026-06-29).** Spec skill v2.0: four subagent
  SKILL.md roles, `promote.sh`/`lessons-for.sh`/`scan-merges.sh`, SF13–SF16 validators,
  branch-doc templates, config-driven setup. Truth = `spec/` bundle + `spec/tests/run.sh`.
- **vibe-skill-consolidation — DONE (2026-06-29).** Seven `vibe-*` shim dirs collapsed
  into one `vibe` skill (router `SKILL.md` + seven phase files); machine `skill` links
  repointed; orders blocks preserved byte-stable. Truth = `flow/` + `flow/tests/run.sh`.
- **vibe-flow-collapse — DONE (2026-06/07, compounded 2026-07-03).** `.agents/flow/`
  engine merged into the vibe skill dir (now `flow/` post-split): scripts, machine,
  cursor all one bundle; hooks/installer/gitignore repointed; `flow/tests/adapters/run.sh`
  asserts `.agents/flow` absent. Plan was never status-updated pre-compound — caught
  by the 2026-07-03 audit.
- **agent-instructions — DONE.** Canonical `AGENTS.md` template + `adapters.json` under
  `vibe-setup/reference/`; `merge-agents.sh` (marker merge + constitution migration +
  adapter symlinks); `vibe-setup` detect/apply rewritten off the constitution path.
  Repo `AGENTS.md` wrapped in `vibe:instructions`; `CLAUDE.md` → `AGENTS.md`.
- **platform-adapters — DONE.** Three hooks (inject/guard/gate) as thin shells over
  `.agents/skills/vibe/scripts/`; `install.sh`. The live Claude wiring is
  `.claude/settings.json` (the earlier `.claude-plugin/plugin.json` + `hooks.json`
  plugin approach was retired — no plugin dir ships; see the platform-adapters
  lesson). Warn-first; graceful-degrade (exit 0). `flow/tests/adapters/run.sh` green.
- **monorepo-split — DONE (2026-07-03).** Canonical halves moved to `spec/` + `flow/`;
  `.agents/skills/{spec,vibe}` + `.claude/skills/spec` compat symlinks; installer
  materializes real dirs (`cp -RL`); script self-location fixed to marker search with
  path-parity tests; truth sweep retired stale `.agents/flow` refs; three orphan
  features compounded. Suites green, validate 0 errors.
- **release-docs — DONE (2026-07-03).** Public-facing polish: umbrella README
  rewrite (banner, spec/flow split, per-half install, dep table from deps.json,
  platform-honesty table) + standalone `spec/README.md` / `flow/README.md`; trust
  rails (MIT LICENSE, CHANGELOG 0.1.0, `tests/run.sh`, GitHub CI, issue templates);
  four rainbow SVG logo candidates; a worked `examples/todo-api/.spec/` sample was
  **deferred** (never shipped — the tree does not exist); and a README-only
  stranger eval (`docs/evals/stranger-2026-07-03.md`) that gated the docs. Eval
  found + fixed the orders.sh fresh-install bug and five doc frictions. gh repo
  metadata deferred to a documented manual step (sandbox boundary). Suites green.
- **install-tooling — DONE (2026-07-03).** `install.sh` refactored to per-half,
  dry-run-gated actions: `--dry-run` (byte-identical preview), `--only spec|flow`
  (partial install), `--uninstall` (surgical inverse — `remove_shipped` deletes
  only shipped files from shared adapter dirs, preserves co-located user files,
  `.spec/` and the cursor unless `--yes`, AGENTS.md block via merge-agents.sh
  `unmerge`). Added `flow/scripts/doctor.sh` (warn-only health report, marker
  self-location) and `flow/reference/deps.json` (single dep manifest). Adversarial
  review pass applied (7 findings). Suites green.
- **dogfood — DONE (scripted).** Hook block/warn/allow/graceful, merge scenarios, and a
  full `install.sh` into a sandbox are exercised in `flow/tests/adapters/run.sh`; the build
  itself was the end-to-end arc. Real-session earn-the-teeth promotions stay future work.
- **flow-mvp — DONE (2026-07-08, merged PR #14; compounded 2026-07-09).** Reworked the
  flow half into the personal operating layer over eleven units: a precedence contract
  (`flow/SKILL.md` + AGENTS.md template), delegation contract blocks at every seam,
  the hybrid plan template (`## Global Constraints` + per-unit `**Steps:**`),
  auto-advance with two edge-keyed `gates` (`feature.plan>feature.impl`,
  `feature.verify>feature.compound`), the new `quick.compound` state + lessons path,
  two impl modes (interactive / handover), caveman demoted from a dependency to
  frozen vocabulary, and the first promoted Stop predicate — the evidence-receipt
  verify tooth (`evidence/feature-<feature>.md`, `evidence/quick.md`, git-derived
  staleness). Tests made hermetic (sandboxed cursor) with machine⊆prose,
  gate↔orders, and evidence-gate block/pass assertions. Its `.spec/` memory said
  NOT STARTED at merge time — the missing compound the 2026-07-09 pass repaired,
  now guarded by `check-drift.sh`. Truth = `flow/` + `flow/tests/run.sh`.
