---
type: area
code:
  - "flow/content"
updated: 2026-10-19
---

# The instruct tool

## Product piece

| Piece | What It Owns | Lives In |
|---|---|---|
| `instruct` | Instruction tiers, blocks, channels, budgets, providers, sync | `instruct/` |

## Features

| Tool | Covers | Lives in |
|---|---|---|
| **instruct** | Layered, budgeted instruction injection: tiers, blocks, channels, providers, sync. | `instruct/` |

## Feature boundaries

| Tool | Owns | Does not own | Reads from peers | Writes for peers |
|---|---|---|---|---|
| instruct | Tiers, blocks, channels, budgets, lints, providers, sources/sync, managed-block composition, `SessionStart`/`UserPromptSubmit` | Any provider's content, the cursor, spec docs | `.vibe/run/inject/*.json`, `.vibe/providers/*.json` | Nothing |
