---
type: concept
updated: 2026-11-12
---

# Front Door (v0.5)

Current source implements C0 shared configuration under `vibe config` and
embedded `vibe spec`; `vibe hook session-start` remains the runtime entry.
The old configuration commands are removed without aliases.
See [binary contract](../../../tech/cli/cli-binary.md).

The queued C1–C6 arc extends `vibe` into one CLI that dispatches to the three tool
binaries and owns the cross-tool capabilities: worktree lifecycle, global
lessons, cross-repo visibility, and global blocks. The three tools stay
independently installable; the CLI is a convenience front door, never a
requirement. Dispatcher discovery, the repo registry, worktree contracts,
global-spec merge semantics, and degrade rules: [tech-cli.md](../../../tech/cli.md).
