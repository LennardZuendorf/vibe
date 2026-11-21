---
type: tech
applies: ["crates/vibe-spec/**", "spec/**"]
updated: 2026-11-21
---

# Spec feature backend contract

The shared Spec application layer owns feature discovery and lifecycle queries.
Flow consumes that interface. It does not parse native artifact files.

## Contracts

- A resolved feature exposes project and branch context, stable feature identity,
  selected backend, authoritative artifact paths, task IDs and readiness.
  Repeated reads of unchanged artifacts return equivalent values.
- The selected backend resolves current state from its native artifacts. Backend
  change or ambiguous matches require an explicit decision before writes.
- Preserve repo-first and global-first resolution and reconciliation. A selected
  backend cannot silently switch code refs, change storage mode or start another
  authoritative copy of the same feature. Keep the private spec history and
  index separate from code Git state.
- Repository validation, backend artifact validation and Flow verification are
  separate results. A successful native validation cannot approve a gated edge.
- Promote durable requirements and decisions through one owner. Re-read the
  candidate and feature identity before closeout. If promotion would revise the
  reviewed candidate, require fresh verification under the existing gates.
- Skill evaluations exercise initial routing, resume, missing prerequisites,
  native edits and promotion. Query tests assert real task IDs and statuses.

## Existing and open decisions

Superpowers remains the current default. Native OpenSpec feature support and its
supported schema/version set require design. The durable root stays Vibe-owned
until the authority decision in [#104](https://github.com/LennardZuendorf/vibe/issues/104).
The eventual design must define native archive and promotion ordering.
[Current project storage](spec/spec-projects-mcp.md) and
[OpenSpec fit](spec/spec-openspec-durable-fit.md) provide the constraints.

[Milestones](../concepts/product-milestones.md) · [Product requirements](../concepts/milestone-spec-workflow.md)
