---
type: flow
updated: 2026-10-24
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
| `feature.design` | Trace the codebase and sketch approaches, then write a superpowers design doc. |
| `feature.plan` | Turn the design into a verified task plan. Human gate before impl. |
| `feature.impl` | Execute the plan test-first. |
| `feature.verify` | Gather evidence per plan task and review. Human gate before ship. |
| `feature.compound` | Record the lesson, promote cross-cutting decisions to root, finish the branch. |
| `quick.triage` | Diagnose the small issue; don't fix yet. Escalate to `feature.design` if scope balloons. |
| `quick.fix` | Implement the bounded fix test-first. |
| `quick.verify` | Prove the fix works and breaks nothing. |

Each state also emits per-turn **orders**, authored as `<!-- vibe:orders:<state> -->` blocks in the linked skill's `SKILL.md` and resolved by `flow` at the cursor (`.vibe/run/flow/state.json`). `instruct` composes these orders into the level/edge/event channels; `flow` works standalone off its own `AGENTS.md` block when `instruct` is absent. The six former phase files are removed.
