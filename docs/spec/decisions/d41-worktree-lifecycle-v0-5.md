---
type: decision
updated: 2026-10-19
---

- **D41 — Worktree lifecycle (v0.5).** `vibe worktree create/list/remove/archive` provisions git worktrees with isolated runtime state and a bootstrapped harness; the only hard refusal is `remove` on a mid-flow cursor; everything else advises.
