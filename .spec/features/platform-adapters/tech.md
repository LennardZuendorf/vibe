---
type: feature-tech
feature: platform-adapters
sibling: product.md
parent: ../../tech.md
updated: 2026-05-14
---

# Feature: Platform Adapters — Architecture

Adapters are thin files that translate runtime-specific affordances into the
platform-neutral `.agents/flow` and `.agents/skills/code-*` core.

**Parent:** [../../tech.md](../../tech.md)
**Requirements:** [product.md](product.md)
**Design:** [design.md](design.md)

---

## Files

```text
AGENTS.md
CLAUDE.md
.claude/
├── commands/
│   └── flow.md
└── hooks/
    ├── user-prompt-submit-inject.sh
    ├── pre-tool-use-forbid.sh
    └── stop-exit-predicate.sh
```

---

## Adapter Rules

- Read canonical state from `.agents/flow/state.json`.
- Read canonical transitions from `.agents/flow/state-machine.json`.
- Invoke `.agents/skills/code-*` skills for workflow behavior.
- Do not write a platform-specific state cursor.
- Do not introduce platform-specific `.spec/` paths.

---

## Install Behavior

The installer should copy or symlink core `.agents` files, then install adapter
files per selected runtime. Existing `AGENTS.md` and `CLAUDE.md` should be
merged or presented as diffs, never blindly overwritten.
