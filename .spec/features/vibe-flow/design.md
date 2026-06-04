---
type: feature-design
feature: vibe-flow
sibling: product.md
parent: ../../design.md
updated: 2026-06-02
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

The per-turn inject is the flow's main lever. One inject owner emits one frozen
string per state, naming the mandatory skill, the write surface, the output path,
the caveman level, and the next state. Principles:

- **Constrain, then resource.** Say what is in scope first, then which skill and
  paths to use. The agent should not have to remember conventions — the inject
  re-states them.
- **Frozen per phase.** The same state always injects the same string, so prompt
  caching holds (see tech.md *Prompt Cache Discipline*).
- **One owner.** Do not run competing injectors; vibe's inject also sets
  the caveman level rather than delegating that to a separate hook.
- **Safety overrides density.** Security warnings and irreversible-action
  confirmations are always normal prose, at any caveman level.

---

## Caveman Behaviour

Caveman is communication density, not reasoning depth. The level a state requests
shapes how the agent *writes back*, never how hard it thinks. Level meanings are
canonical in the root [product.md](../../product.md) Communication Levels; the
inject names the level and trusts the agent (and, when installed, the upstream
caveman skill) to apply it.
