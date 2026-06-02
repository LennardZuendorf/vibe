---
type: feature-product
feature: spec-framework
sibling: tech.md
parent: ../../product.md
updated: 2026-05-14
---

# Feature: Spec Framework — Product

The spec framework is the durable planning and memory layer. It gives every
project a predictable `.spec/` tree for product, tech, design, plan, lessons,
feature specs, and archives.

**Parent:** [../../product.md](../../product.md)
**Architecture:** [tech.md](tech.md)
**Design:** [design.md](design.md)

---

## Why this feature exists

Agent sessions drift when project intent only lives in conversation. The spec
framework makes decisions persistent, navigable, and separated by concern so
future work can resume without rediscovering the project.

---

## Requirements

| # | Requirement |
|---|---|
| R1 | Root specs include `product.md`, `tech.md`, `design.md`, `plan.md`, and `lessons.md`. |
| R2 | Feature specs live under `.spec/features/<name>/` with required `product.md` and `tech.md`, plus optional `design.md`, `plan.md`, and `research.md`. |
| R3 | Product, tech, design, and plan concerns remain distinct enough that agents can load only the docs they need. |
| R4 | Feature specs are ephemeral: created during design, consumed during implementation, merged when cross-cutting, then archived. |
| R5 | Validation catches missing frontmatter, broken links, missing feature files, and invalid root/branch naming. |
| R6 | The skill works both vendored in a repo and installed globally. |
| R7 | Design docs can follow the google-labs-code `DESIGN.md` token-plus-prose pattern when visual identity matters. |
| R8 | Lessons are tagged for retrieval, not just appended: each lesson carries a `tags:` line and a category so design/triage phases can find the relevant ones on entry. (D8) |

---

## User Experience

Users invoke the `spec` skill when they need strategy docs, feature specs,
architecture review, validation, or lessons. The skill tells agents which docs
to load first and where new decisions belong.

---

## Outputs

- `.spec/` root docs and feature folders
- Templates under `.agents/skills/spec/reference/templates/`
- Validation and setup scripts

---

## Non-Goals

- Runtime flow state
- Agent hook enforcement
- Codex or Claude Code adapter behavior
