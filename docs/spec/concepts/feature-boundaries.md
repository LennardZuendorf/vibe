---
type: concept
updated: 2026-10-19
---

# Feature Boundaries

Each tool is a closed, deliverable, testable box. Cross-tool coupling is a whole-feature gate (Feature Sequence), never a unit-to-unit edge.

```text
┌─────────────────────────────────────────────────────────────┐
│  memory      spec root tree, AGENTS.md per-owner blocks      │  repo
├─────────────────────────────────────────────────────────────┤
│  contracts   schemas + golden fixtures, run.sh                │  shared
├─────────────────────────────────────────────────────────────┤
│  spec        spec-root tooling, validators, 2 agents          │  ┐
├─────────────────────────────────────────────────────────────┤  │
│  flow        state machine, cursor, guard, TUI, ledger        │  ├ tools
├─────────────────────────────────────────────────────────────┤  │
│  instruct    tiers, blocks, channels, providers, sync         │  ┘
├─────────────────────────────────────────────────────────────┤
│  installer   install.sh: subsets, platforms, uninstall        │  repo
└─────────────────────────────────────────────────────────────┘
```

`contracts/` and `install.sh` are shared, not owned by any one tool — `contracts/run.sh` (per-tool CI) and the installer's `--only` flag are the only things that touch all three.
