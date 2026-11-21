---
type: hub
scope: implementation
covers: feature sequence, binary gates, validation criteria, open decisions
children: [concepts/product-milestones.md, concepts/feature-sequence.md, concepts/shared-feature-sequence.md, parts/spec/concepts/spec-feature-sequence.md, parts/flow/concepts/flow-feature-sequence.md, parts/instruct/concepts/instruct-feature-sequence.md, parts/cli/concepts/cli-feature-sequence.md, concepts/issue-catalog.md, concepts/feature-boundaries.md, concepts/delivered-features.md, concepts/spec-vs-implementation.md, decisions/README.md, questions/resolved-open-items.md]
updated: 2026-11-21
---

# vibe — Implementation Plan

Vibe delivers one installed workflow for approved agent work across repositories. The [product milestones](concepts/product-milestones.md) own current delivery order and acceptance. F1–F25 and C1–C6 retain implementation history and row-level scope.

Rust Instruct is active planning again. The trusted Flow runner precedes TUI work. A reference-host install can be proven early. Each feature still needs design and implementation approval. The OpenSpec durable-root decision remains open.

**Parent specs:** [product.md](product.md), [tech.md](tech.md), [design.md](design.md). Architecture detail, including the v0.4 branch docs and the v0.5 CLI doc, lives in tech.md → `tech-contracts.md`, `tech-rust.md`, `tech-instruct.md`, `tech-spec.md`, `tech-cli.md`.

## Sequence

- [Product milestones](concepts/product-milestones.md) — current outcomes, gates and GitHub tracking

- [Feature Sequence](concepts/feature-sequence.md) — order, binary gates, tracks, active focus
- [Shared rows](concepts/shared-feature-sequence.md) — F3, F4, F21 to F24
- [Spec rows](parts/spec/concepts/spec-feature-sequence.md) — F1, F2, F16 to F20, F25
- [Flow rows](parts/flow/concepts/flow-feature-sequence.md) — F5 to F10
- [Instruct rows](parts/instruct/concepts/instruct-feature-sequence.md) — F11 to F15
- [v0.5 CLI arc](parts/cli/concepts/cli-feature-sequence.md) — delivered C0 and queued C1 to C6
- [Issue catalog](concepts/issue-catalog.md) — GitHub issues per row

## Boundaries and status

- [Feature Boundaries](concepts/feature-boundaries.md) — the tool boxes and shared pieces
- [Delivered features](concepts/delivered-features.md) — what shipped, with history
- [Spec vs Implementation](concepts/spec-vs-implementation.md) — what is spec-ahead

## Decisions and questions

- [Decisions](decisions/README.md) — D8 to D47, one note each
- [Resolved open items](questions/resolved-open-items.md) — OPEN-2, OPEN-3, OPEN-4/OPEN-7, OPEN-6

## Branch work

| Feature | Scope | Status |
|---|---|---|
| review-hardening | Port of the pre-v0.4 review-hardening fixes onto the live JS/bash harness; pins in [rust-parity-pins](tech/rust/rust-parity-pins.md) | DONE |
| skill-revision | Tracks A/B/C/E delivered; reconcile Flow track D, the conditional tail and closeout in [#103](https://github.com/LennardZuendorf/vibe/issues/103) | IN PROGRESS |
| config-namespace | [Seven shared-configuration operations under `vibe config`](tech/cli/cli-binary.md); old commands removed without aliases; merged and independently reviewed | DONE |
| spec-projects-mcp | [Embedded spec, project storage and local Rust MCP](tech/spec/spec-projects-mcp.md); merged and independently reviewed | DONE |
