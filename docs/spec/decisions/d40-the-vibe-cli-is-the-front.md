---
type: decision
updated: 2026-11-12
---

- **D40 — The vibe CLI is the front door (v0.5).** Target architecture: one `vibe` binary dispatches to the three tool binaries and owns the cross-tool capabilities (worktree, space, global blocks). The three tools stay independent and alone-installable; "no umbrella ever" retires, "no mandatory bundle" replaces it. Supersedes D18's umbrella clause. Current source delivers shared configuration under `vibe config`, stable `vibe hook session-start`, and embedded `vibe spec`; general peer dispatch, worktrees and Spaces remain C1–C6 targets. Live contract: [the vibe binary](../tech/cli/cli-binary.md).
