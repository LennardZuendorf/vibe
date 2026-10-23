---
type: hub
scope: design
design_format: google-labs-code/design.md-inspired
children: [concepts/design-principles.md, concepts/interaction-conventions.md, concepts/information-hierarchy.md, parts/flow/concepts/tui-conventions.md, parts/spec/concepts/spec-document-ergonomics.md]
updated: 2026-10-19
---

# vibe — Design

Cross-cutting design language for the personal coding-agent toolkit. This is not a visual product; design here means interaction shape, information hierarchy, agent-facing tone, and the ergonomics of moving through work across three independent tools.

For UI-heavy projects, `docs/spec/design.md` should follow or reference the
google-labs-code [`DESIGN.md`](https://github.com/google-labs-code/design.md)
pattern: design tokens in YAML frontmatter plus markdown rationale. vibe
uses the same idea, but this repo's current design doc is prose-first because
the product is an agent workflow rather than a visual interface.

**Product:** [product.md](product.md)
**Architecture:** [tech.md](tech.md)

## Concepts

- [Design Principles](concepts/design-principles.md) — seven interaction principles
- [Interaction Conventions](concepts/interaction-conventions.md) — tools, state locations, phase receipts
- [Information Hierarchy](concepts/information-hierarchy.md) — which surface answers which question
- [TUI Conventions](parts/flow/concepts/tui-conventions.md) — keys, prompts, the live pane
- [Spec Document Ergonomics](parts/spec/concepts/spec-document-ergonomics.md) — how the spec tree reads

## Feature Design Index

No feature designs are open. Feature `design.md` fragments are branch-scoped and were removed when their features compounded; the cross-cutting design language lives in this document.
