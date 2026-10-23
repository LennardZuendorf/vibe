---
type: decision
updated: 2026-10-19
---

- **D40 — The vibe CLI is the front door (v0.5).** One `vibe` binary dispatches to the three tool binaries and owns the cross-tool capabilities (worktree, space, global blocks). The three tools stay independent and alone-installable; "no umbrella ever" retires, "no mandatory bundle" replaces it. Supersedes D18's umbrella clause.
