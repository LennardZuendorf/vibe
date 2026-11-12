---
type: area
code:
  - "spec"
  - "crates/vibe-spec"
updated: 2026-11-12
---

# The spec tool

Back to [spec](../spec.md).

## Current product

| Piece | What It Owns | Lives In |
|---|---|---|
| `spec` | The spec tree (`docs/spec/`; deprecated `.spec/` still resolves), templates, validators, OpenSpec interchange, two subagents | `spec/`, `crates/vibe-spec/` | <!-- literal -->

Central `vibe spec`, compatibility `vibe-spec` and local stdio MCP share one
Rust application layer. Registered project storage and reconciliation are
implemented, locally verified and independently reviewed on this branch.
Release distribution and installer delivery remain separate.

## Storage and access

- Project registration selects repo-first or global-first storage.
- Repo-first reads/writes follow selected or matching ambient checkout files.
  Otherwise project-only access uses the main checkout's actual branch.
- Global-first keeps complete durable and feature specs outside code repositories.
  The same project/branch shares specs across clones/worktrees; branches stay separate.
- Delayed reconciliation merges proven clean snapshot-bound changes automatically;
  uncertain histories need explicit reconciliation. Conflicts have a CLI/MCP
  resolution lifecycle and never block a code merge.
- Central `vibe spec` and Rust MCP expose equivalent operations through one API.

## Feature boundaries

| Tool | Owns | Does not own | Reads from peers | Writes for peers |
|---|---|---|---|---|
| spec | Spec tree, templates, validate/drift/promote/scan/trace, OpenSpec interchange, 2 agents, its own hooks | Cursor, injection, block composition | Nothing | `--json` answers (`root`, `lessons-for`, `plan`, `feature`); its AGENTS.md block |

This surface owns spec identity, storage and reconciliation only. Flow,
Instruct, general Spaces/config and MCP-server projection remain outside scope.
See [contract](../concepts/spec-framework-contract.md),
[technology](../../../tech/spec/spec-projects-mcp.md) and
[plan](../concepts/spec-feature-sequence.md).
