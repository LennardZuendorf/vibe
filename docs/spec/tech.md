---
type: hub
scope: technical
children: [tech/contracts.md, tech/rust.md, tech/instruct.md, tech/spec.md, tech/cli.md]
updated: 2026-10-19
---

# vibe — Technical Architecture

vibe is three independent tools — spec, flow, instruct — each a single static
Rust binary with no runtime dependency, each a Claude Code plugin and an
OpenCode plugin, each installable and useful alone. They compose through
file-based contracts, never through imports. One repo, one installer, three
release trains. **Shipped today:** one Node engine (`flow/engine/`) plus bash
oracles drive all machinery, hooks run through `.claude/settings.json`, and
the cursor lives at `.agents/skills/vibe/state.json`. This document describes
the v0.4 target unless a callout says otherwise; the queued v0.5 arc adds one
front door — the `vibe` CLI — plus worktree lifecycle and a global spec space
over the same three tools ([tech-cli.md](tech/cli.md)).

## Concepts

- [Design Philosophy](concepts/design-philosophy.md) — nine architecture principles
- [Architecture Overview](concepts/architecture-overview.md) — the component graph
- [Cross-Tool Contracts](concepts/cross-tool-contracts.md) — four versioned contracts
- [Engine Module Boundaries](concepts/engine-module-boundaries.md) — the core crate and its limits
- [Risks & Mitigations](concepts/risks-and-mitigations.md) — risk table

## Platform

- [Layers](platform/layers.md) — what each layer owns
- [File Layout](platform/file-layout.md) — repo tree and what a target repo receives
- [Adapter Contract](platform/adapter-contract.md) — hooks per platform and shim discovery

## Contracts per part

- [Spec Framework Contract](parts/spec/concepts/spec-framework-contract.md)
- [Vibe Flow Contract](parts/flow/concepts/flow-contract.md)
- [Instruct Contract](parts/instruct/concepts/instruct-contract.md)
- [Front Door (v0.5)](parts/cli/concepts/front-door.md)

## Branch Documents

| Document | Covers |
|---|---|
| [tech-contracts.md](tech/contracts.md) | Payload, marker grammar, provider manifest, spec JSON — schemas, examples, exit codes, versioning. |
| [tech-rust.md](tech/rust.md) | Workspace, crates, shim discovery, oracles and parity, TUI, signals, distribution, CI. |
| [tech-instruct.md](tech/instruct.md) | Tiers, budgets, block format, lints, sources and sync, platform file targets. |
| [tech-spec.md](tech/spec.md) | Root discovery, migration, OpenSpec interchange, strictness, subagents, hooks. |
| [tech-cli.md](tech/cli.md) | The `vibe` front door (v0.5): dispatcher, repo registry, worktree lifecycle, global spec space, global blocks. |

## Features

Feature inventory and delivery status live in [plan.md](plan.md)'s Feature
Sequence — the single place cross-feature order is stated. This document
describes the architecture those features build, not their status.
