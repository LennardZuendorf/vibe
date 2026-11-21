---
type: hub
scope: technical
children: [tech/milestone-spec-feature-contract.md, tech/milestone-flow-runner-contract.md, tech/milestone-instruct-provider-contract.md, tech/milestone-workspace-contract.md, tech/milestone-install-contract.md, tech/contracts.md, tech/rust.md, tech/instruct.md, tech/spec.md, tech/cli.md]
updated: 2026-11-21
---

# vibe — Technical Architecture

Spec owns documents, feature artifact backends, and validation. Flow owns
execution state, scoped authority, checkpoints and human approval edges.
Instruct owns layered, budgeted instruction composition and managed lesson
projection. The planned provider boundary is versioned and supports the
reference host before other host adapters.

The central `vibe` binary already embeds Spec. Its approved direct
`vibe -> vibe-spec` dependency stays in place. Global-first Spec history
requires Git. Rust Flow F5–F8 and the opt-in v2 core are merged. Trusted v2
verification remains pending. Shell and JS fallback surfaces remain
available until their replacement and migration are proven.

The product target is one installation entry point. The current independent
tool and release-train decisions remain documented. The reference installer
and worktree APIs can be designed in their own slices. See
[product milestones](concepts/product-milestones.md).

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

Milestone [technical contracts](concepts/product-milestones.md#milestone-specifications) define ownership and verification boundaries.
