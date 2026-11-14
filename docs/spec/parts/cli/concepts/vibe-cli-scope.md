---
type: concept
updated: 2026-11-12
---

# The vibe CLI (v0.5)

Current source delivers shared configuration through `vibe config`, the stable
`vibe hook session-start` runtime entry and embedded `vibe spec`.
See [configuration tool](../areas/config-tool.md) and [binary contract](../../../tech/cli/cli-binary.md).
The C1–C6 front-door capabilities below remain targets.

## Product piece

| Piece | What It Owns | Lives In |
|---|---|---|
| `vibe` CLI (v0.5) | Front door: tool dispatch, worktree lifecycle, global spec space, global blocks | `crates/vibe`, plugin `vibe/` |

## Features

| Tool | Covers | Lives in |
|---|---|---|
| **vibe CLI** (v0.5) | Front door + cross-tool capabilities: worktree lifecycle, global lessons, cross-repo visibility, global blocks. | `crates/vibe`, plugin `vibe/` |
