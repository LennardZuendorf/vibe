---
type: tech
applies:
  - "crates/vibe-spec/**"
  - "tests/openspec/**"
updated: 2026-11-21
---

# OpenSpec durable-spec fit

Back to [Spec technology](../spec.md) and [product milestones](../../concepts/product-milestones.md).

**Finding.** OpenSpec keeps current, durable capability requirements and
scenarios in `openspec/specs/`. Native change folders carry proposals, designs,
tasks and delta specs. Archive applies the deltas to current capability specs and
retains the finished change as history. [Upstream lifecycle](https://github.com/Fission-AI/OpenSpec/blob/main/docs/concepts.md).

**Executable proof.** The repo-pinned OpenSpec 1.14.2 CLI validated one native
capability and one change under `validate --all --strict`.
Archiving applied one MODIFIED requirement and moved the proposal to archive.
Separate Vibe technology and lessons fixtures retained their original hashes.

| Vibe requirement | OpenSpec fit |
|---|---|
| Current requirements and scenarios | Native durable capability specs and delta updates. |
| Product, architecture, design, and roadmap hubs | Project config and custom change schemas can supply context and artifact templates. Vibe's durable hub and typed-note structure requires additional policy. |
| Lessons, links, budgets, and Git-aware checks | Vibe owns these checks. OpenSpec's native validators and archive do not implement them. |
| Project/branch storage and change ownership | OpenSpec has beta stores and references. Equivalence with Vibe's branch-bound snapshots, paired code/spec reconciliation and cross-repo ownership remains unproven. |
| Approval and promotion | Vibe must retain its gates and choose a single writer for any durable update. |

Sources: [OpenSpec configuration and schemas](https://github.com/Fission-AI/OpenSpec/blob/main/docs/customization.md),
[CLI and beta stores](https://github.com/Fission-AI/OpenSpec/blob/main/docs/cli.md),
[Vibe adherence](spec-adherence.md) and [project storage](spec-projects-mcp.md).

**Recommendation.** Keep Vibe's durable root as the current authority. Plan
native OpenSpec changes as a selectable feature backend alongside Superpowers.
Approve the mapping for OpenSpec capability updates and Vibe promotion before
enabling either writer on the same facts. A full durable-root replacement needs
a separate equivalence decision. [#104](https://github.com/LennardZuendorf/vibe/issues/104)
holds that decision. The [current interchange](spec-openspec.md) does not provide
the planned native feature lifecycle.
