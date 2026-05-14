---
type: feature-design
feature: platform-adapters
sibling: product.md
parent: ../../design.md
updated: 2026-05-14
---

# Feature: Platform Adapters — Design

Adapter docs should be calm, direct, and boring. They are not the product
architecture; they are reminders that point the runtime back to `.agents/flow`
and `.agents/skills/code-*`.

**Parent:** [../../design.md](../../design.md)
**Requirements:** [product.md](product.md)
**Architecture:** [tech.md](tech.md)

---

## Design Rules

- Put canonical paths near the top.
- Avoid platform-specific reinventions of the workflow.
- State what the adapter reads, what it writes, and what it must never own.
