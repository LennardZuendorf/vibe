---
type: hub
scope: product
children: [concepts/milestone-spec-workflow.md, concepts/milestone-autonomous-flow.md, concepts/milestone-rust-instruct.md, concepts/milestone-feature-workspaces.md, concepts/milestone-cli-installation.md, concepts/story.md, concepts/carrier-model.md, concepts/requirements.md, concepts/product-principles.md, concepts/target-user.md, concepts/non-goals.md, parts/spec/spec.md, parts/flow/flow.md, parts/instruct/instruct.md, parts/cli/cli.md, areas/contracts-schemas.md, areas/installer.md, questions/resolved-questions.md]
updated: 2026-11-21
---

# vibe — Product

Vibe is a CLI for spec-driven agent work. One install sets up supported agent
hosts. A feature workspace can contain worktrees from several repositories.
Agents work from an approved plan and stop at human gates.

Spec keeps durable project knowledge and supports selectable feature
workflows. Flow runs and verifies approved work. Rust Instruct supplies
context, steering and relevant lessons to each host. The
[product milestones](concepts/product-milestones.md) define delivery proof.

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

Build order, delivery status and earlier implementation rows live in [plan.md](plan.md).

Milestone [product requirements](concepts/product-milestones.md#milestone-specifications) define observable outcomes.
