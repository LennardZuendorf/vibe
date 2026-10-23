---
type: hub
scope: product
children: [concepts/story.md, concepts/carrier-model.md, concepts/requirements.md, concepts/product-principles.md, concepts/target-user.md, concepts/non-goals.md, parts/spec/spec.md, parts/flow/flow.md, parts/instruct/instruct.md, parts/cli/cli.md, areas/contracts-schemas.md, areas/installer.md, questions/resolved-questions.md]
updated: 2026-10-19
---

# vibe — Product

vibe is three small, independent tools for coding with agents — **spec**
(durable planning memory), **flow** (a state-machine workflow with real teeth),
and **instruct** (layered, budgeted instruction injection) — each a single
static Rust binary with no runtime dependency, each a Claude Code plugin and an
OpenCode plugin, each installable and useful alone, composing through
file-based contracts when co-installed. One repo, one installer, three release
trains. The repo's own memory stays committed Markdown, readable by any agent
or none.

**One-liner:** durable specs, a flow with teeth, and budgeted instructions —
three binaries, no bundle.

## Concepts

- [Story](concepts/story.md) — why vibe exists and what v0.4 and v0.5 change
- [Carrier model and front door](concepts/carrier-model.md) — v0.4 carrier model, v0.5 front door, shipped today
- [Requirements](concepts/requirements.md) — the 18 project-level requirements
- [Design Principles](concepts/product-principles.md) — the 12 product principles
- [Target User](concepts/target-user.md) — who the system is for
- [Non-Goals](concepts/non-goals.md) — what vibe will not be

## Parts

- [spec](parts/spec/spec.md) — durable planning memory
- [flow](parts/flow/flow.md) — the workflow with teeth, its phases and style
- [instruct](parts/instruct/instruct.md) — budgeted instruction injection
- [cli](parts/cli/cli.md) — the v0.5 front door

## Areas

- [Contracts](areas/contracts-schemas.md) — versioned schemas and golden fixtures
- [Installer](areas/installer.md) — one POSIX installer script
- Each tool's area note lives in its part hub.

## Questions

- [Resolved Questions](questions/resolved-questions.md) — nine settled product questions

Build order and status: see the F1–F25 feature sequence and the queued v0.5 C1–C6 arc in [plan.md](plan.md).
