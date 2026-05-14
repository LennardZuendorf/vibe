---
type: feature-design
feature: code-flow
sibling: product.md
parent: ../../design.md
updated: 2026-05-14
---

# Feature: Code Flow — Design

The code flow should make the next right action feel obvious. It should not ask
the agent to remember hidden conventions; the active skill and state machine
should name the expected files, tools, evidence, and transition.

**Parent:** [../../design.md](../../design.md)
**Requirements:** [product.md](product.md)
**Architecture:** [tech.md](tech.md)

---

## Interaction Rules

- Prefer named skills (`code-feature`) over slash-command-only workflows.
- Keep phase receipts short: state, changed files, verification, next transition.
- Put exact output paths in every delegation prompt.
- When scope grows, transition to a bigger flow rather than stretching quick mode.
