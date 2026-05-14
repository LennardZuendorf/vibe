---
type: feature-product
feature: platform-adapters
sibling: tech.md
parent: ../../product.md
updated: 2026-05-14
---

# Feature: Platform Adapters — Product

Platform adapters expose the same `code` flow to Codex, Claude Code, and future
agent runtimes without making any one platform canonical.

**Parent:** [../../product.md](../../product.md)
**Architecture:** [tech.md](tech.md)
**Design:** [design.md](design.md)

---

## Why this feature exists

Codex and Claude Code read different instruction files and support different
integration points. The workflow should still behave the same because both
adapters point at `.agents/flow` and `.agents/skills/code-*`.

---

## Requirements

| # | Requirement |
|---|---|
| R1 | `AGENTS.md` mirrors the core workflow for Codex-style agents. |
| R2 | `CLAUDE.md` mirrors the core workflow for Claude Code. |
| R3 | Claude slash commands and hooks read `.agents/flow`, not `.claude/state.json`. |
| R4 | Adapter files do not define a separate spec layout or state model. |
| R5 | Installation preserves existing project instructions and offers diffs when merging. |

---

## Outputs

- `AGENTS.md`
- `CLAUDE.md`
- `.claude/commands/*`
- `.claude/hooks/*`
- installer/setup behavior

---

## Non-Goals

- Duplicating the state machine per platform
- Making `.claude/` canonical
- Making Codex-specific desktop behavior part of the core flow
