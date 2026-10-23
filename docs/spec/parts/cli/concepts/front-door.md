---
type: concept
updated: 2026-10-19
---

# Front Door (v0.5)

The queued v0.5 arc adds `vibe` — one CLI that dispatches to the three tool
binaries and owns the cross-tool capabilities: worktree lifecycle, global
lessons, cross-repo visibility, and global blocks. The three tools stay
independently installable; the CLI is a convenience front door, never a
requirement. Dispatcher discovery, the repo registry, worktree contracts,
global-spec merge semantics, and degrade rules: [tech-cli.md](../../../tech/cli.md).
