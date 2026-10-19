---
type: hub
scope: implementation
covers: feature sequence, binary gates, validation criteria, open decisions
children: []
updated: 2026-10-19

# vibe — Implementation Plan

Single-purpose repo: build the self-hosting vibe workflow harness. This plan is current-only — delivered work collapses to notes, no long-horizon backlog. The active arc is v0.4: three tools, one language, no bundle — `spec`, `flow`, and `instruct` rewritten as independent static Rust binaries, sequenced as binary-gated features F1–F25 below. The queued arc is v0.5: the `vibe` CLI front door — worktree lifecycle, global spec management, global blocks (C1–C6, own section below).

**Parent specs:** [product.md](product.md), [tech.md](tech.md), [design.md](design.md). Architecture detail, including the v0.4 branch docs and the v0.5 CLI doc, lives in tech.md → `tech-contracts.md`, `tech-rust.md`, `tech-instruct.md`, `tech-spec.md`, `tech-cli.md`.

## Sequence

- [Feature Sequence](concepts/feature-sequence.md) — order, binary gates, tracks, active focus
- [Shared rows](concepts/shared-feature-sequence.md) — F3, F4, F21 to F24
- [Spec rows](parts/spec/concepts/spec-feature-sequence.md) — F1, F2, F16 to F20, F25
- [Flow rows](parts/flow/concepts/flow-feature-sequence.md) — F5 to F10
- [Instruct rows](parts/instruct/concepts/instruct-feature-sequence.md) — F11 to F15
- [v0.5 CLI arc](parts/cli/concepts/cli-feature-sequence.md) — C1 to C6
- [Issue catalog](concepts/issue-catalog.md) — GitHub issues per row

## Boundaries and status

- [Feature Boundaries](concepts/feature-boundaries.md) — the tool boxes and shared pieces
- [Delivered features](concepts/delivered-features.md) — what shipped, with history
- [Spec vs Implementation](concepts/spec-vs-implementation.md) — what is spec-ahead

## Decisions and questions

- [Decisions](decisions/README.md) — D8 to D47, one note each
- [Resolved open items](questions/resolved-open-items.md) — OPEN-2, OPEN-3, OPEN-4/OPEN-7, OPEN-6
