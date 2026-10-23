---
type: area
code:
  - "spec"
  - "crates/vibe-spec"
updated: 2026-10-19
---

# The spec tool

## Product piece

| Piece | What It Owns | Lives In |
|---|---|---|
| `spec` | The spec tree (`docs/spec/`; legacy `.spec/` still resolves), templates, validators, OpenSpec interchange, two subagents | `spec/` | <!-- literal -->

## Features

| Tool | Covers | Lives in |
|---|---|---|
| **spec** | Durable planning memory: spec tree, templates, validators, OpenSpec interchange, two subagents. | [`spec/`](../../../../../spec/README.md) |

## Feature boundaries

| Tool | Owns | Does not own | Reads from peers | Writes for peers |
|---|---|---|---|---|
| spec | Spec tree, templates, validate/drift/promote/scan/trace, OpenSpec interchange, 2 agents, its own hooks | Cursor, injection, block composition | Nothing | `--json` answers (`root`, `lessons-for`, `plan`, `feature`); its AGENTS.md block |
