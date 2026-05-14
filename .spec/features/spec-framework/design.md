---
type: feature-design
feature: spec-framework
sibling: product.md
parent: ../../design.md
updated: 2026-05-14
---

# Feature: Spec Framework — Design

The framework should feel like a small map, not a documentation maze. It uses
consistent filenames and short entrypoints so agents can load the right context
without reading everything.

**Parent:** [../../design.md](../../design.md)
**Requirements:** [product.md](product.md)
**Architecture:** [tech.md](tech.md)

---

## Design Rules

- Root docs answer project-level questions.
- Feature docs answer one buildable unit of work.
- Branch docs are rare and only for concerns that span multiple features.
- `design.md` is first-class when UX, interaction, language, or workflow
  ergonomics matter.
