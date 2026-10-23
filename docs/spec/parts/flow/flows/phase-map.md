---
type: flow
updated: 2026-10-19
---

# Phase map

Each phase, the skill link that drives it, the external skills and subagents it delegates to, the spec artifact it reads/writes, and what the stage is for. This is the canonical workflow contract; the full per-state record (skill link, `next` arrays, exit predicates) lives in `flow`'s `state-machine.json`. The two human gates are keyed by **edge** in the machine's `gates` object (`feature.plan>feature.impl`, `feature.verify>feature.compound`), not by state — so `feature.verify`'s fix/drift back-edges to `feature.impl`/`feature.plan` stay ungated while its ship edge stops for approval.

Delegates, write surfaces, and legal edges are **data** — read them from `state-machine.json`, the only place they are stated. This table carries only what the machine cannot say: what each stage is *for*.

| Phase | What the stage does |
|---|---|
| `idle` | Resting hub between flows. Read lessons/plan, then pick the flow that matches the request. |
| `setup.detect` | Read-only audit of repo + harness; report present vs missing and preflight required plugins. |
| `setup.apply` | Write/merge the bootstrap without clobbering: AGENTS.md block, flow scaffold, baseline specs. |
| `strategy.brainstorm` | Shape project direction in dialogue; scratch only, no writes yet. |
| `strategy.spec` | Commit the agreed direction into the root specs and validate. |
| `feature.design` | Trace the codebase and sketch approaches, then write the feature's product + tech specs. |
| `feature.plan` | Turn the design into stable unit IDs (`<feature>/1`, `/2`…). Human gate before impl. |
| `feature.impl` | Build the plan units test-first, citing unit IDs. |
| `feature.verify` | Gather real evidence per unit ID and review. Human gate before ship. |
| `feature.compound` | Record the lesson, promote cross-cutting decisions to root, archive the feature. |
| `quick.triage` | Diagnose the small issue; don't fix yet. Escalate to `feature.design` if scope balloons. |
| `quick.fix` | Implement the bounded fix test-first. |
| `quick.verify` | Prove the fix works and breaks nothing. |

Each phase also emits per-turn **orders**, authored once as content blocks in the linked skill's phase file and resolved by `flow` at the cursor (`.vibe/run/flow/state.json` in v0.4; `.agents/skills/vibe/state.json` today). `instruct` composes these orders into the level/edge/event channels; `flow` works standalone off its own `AGENTS.md` block when `instruct` is absent.
