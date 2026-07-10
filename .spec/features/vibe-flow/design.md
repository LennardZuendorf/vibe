---
type: feature-design
feature: vibe-flow
sibling: product.md
parent: ../../design.md
updated: 2026-06-06
---

# Feature: Vibe Flow — Design

The vibe flow should make the next right action feel obvious. The user should be
able to say "I need X" and not carry the planning load — the flow decides which
phase they are in, which skill is mandatory, which files are in scope, and how
terse to be, then guides them through spec → plan → build → TDD-validate.

This is the KISS-personal middle ground: it borrows ideas from Compound
Engineering (persistent lessons, stable plan unit IDs) without copying its
machinery. The agent's instinct is encoded as constraints and injected resources,
not as a large second toolchain.

**Parent:** [../../design.md](../../design.md)
**Requirements:** [product.md](product.md)
**Architecture:** [tech.md](tech.md)
**Plan:** [plan.md](plan.md)

---

## Interaction Rules

- Prefer named skills (`vibe-feature`) over slash-command-only workflows.
- Keep phase receipts short: state, changed files, verification, next transition.
- Put exact output paths in every delegation prompt.
- When scope grows, transition to a bigger flow rather than stretching quick mode.
- On entering a `*.design` or `*.triage` state, read `lessons.md` first so past
  mistakes shape the new work (retrieval, not just recording).

---

## Injection Design

The per-turn inject is the flow's main lever. One inject owner emits one static
set of orders per state, naming the mandatory skill, the write surface, the output
path, and the next state. The orders are **sourced from the
state's linked skill** (D12), not a separately authored string. Principles:

- **Constrain, then resource.** Say what is in scope first, then which skill and
  paths to use. The agent should not have to remember conventions — the inject
  re-states them.
- **One source per state.** The orders live in the linked skill, so behaviour is
  edited in one place; the inject hook just delivers them (D12).
- **Static per phase.** The same state always injects the same orders, so prompt
  caching holds (see tech.md *Prompt Cache Discipline*).
- **One owner.** Do not run competing injectors; output density is fixed by the
  machine's one top-level `style` note, not a separate hook.
- **Safety overrides density.** Security warnings and irreversible-action
  confirmations are always normal prose, regardless of the `style` note.

---

## Output Density

Density is communication style, not reasoning depth. It shapes how the agent
*writes back*, never how hard it thinks. Density is governed by one top-level
`style` note in `state-machine.json` — no filler; compress receipts and subagent
summaries; keep security warnings and irreversible-action confirmations in full
prose; never trade reasoning depth for brevity. The per-state caveman levels
(`lite`/`full`/`ultra`) this section once described were **retired 2026-07-09** in
favour of that single note.
