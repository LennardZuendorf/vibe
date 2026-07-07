---
type: feature-design
feature: platform-adapters
sibling: product.md
parent: ../../design.md
updated: 2026-06-06
---

# Feature: Platform Adapters — Design

Adapter docs should be calm, direct, and boring. They are not the product
architecture; they are reminders that point the runtime back to `.agents/skills/vibe`.

**Parent:** [../../design.md](../../design.md)
**Requirements:** [product.md](product.md)
**Architecture:** [tech.md](tech.md)
**Plan:** [plan.md](plan.md)

---

## Design Rules

- Put canonical paths near the top.
- Avoid platform-specific reinventions of the workflow.
- State what the adapter reads, what it writes, and what it must never own.

## Hooks & plugin

- The Claude Code plugin is the *packaging*, not a second core. It carries the
  command, skills, and hooks; it owns no state.
- Hooks stay thin and boring: read state or call `detect-context.sh`, translate
  the verdict to Claude Code's exit-code convention, get out of the way.
- Earn the teeth: ship hooks warn-first; a hook should never end a session.
- The inject hook delivers the linked skill's per-state orders verbatim (D12) — it
  shapes nothing, it just delivers the current orders every turn.
