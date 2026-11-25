---
type: concept
updated: 2026-11-25
---

# Trusted Spec workflow

A maintainer can choose a feature workflow and recover its scope, current tasks and readiness from the artifacts. Durable product knowledge remains available after the feature closes.

## Requirements

### Requirement: MS-SPEC-01 — Reliable skill routing
The Spec skill SHALL locate the canonical root and follow the selected feature workflow for new work, resumed work and closeout. It SHALL explain missing prerequisites or ambiguous artifacts before making changes.

#### Scenario: Resume a real feature after a fresh session
- **GIVEN** a project with feature artifacts and completed tasks
- **WHEN** an agent resumes without prior conversation context
- **THEN** it finds the same feature, identifies the remaining tasks and respects the current approval gate

### Requirement: MS-SPEC-02 — Select a native feature backend
A feature SHALL have one artifact owner. Vibe SHALL expose create, validate, task query, resume, verification and closeout for the approved Superpowers and OpenSpec paths.

#### Scenario: Change tasks using the native backend
- **GIVEN** a native feature with an implementation task
- **WHEN** a maintainer edits the task using that backend's normal tools
- **THEN** Vibe reports the edited task and current status without creating a second authoring copy

### Requirement: MS-SPEC-03 — Preserve durable ownership
Closeout SHALL distinguish long-lived project knowledge from feature artifacts, apply approved updates through the chosen owner and retain the verification and promotion requirements.

#### Scenario: Close a verified change
- **GIVEN** an approved capability change and the associated technical decision
- **WHEN** the feature closes
- **THEN** the current capability and durable technical decision are reachable in their owning stores, and the original feature is archived or retired per its backend policy

## Acceptance scope

The [backend contract](../tech/milestone-spec-feature-contract.md) now fixes
selection, identity, task queries, resume, verification and closeout for both
backends, plus the mixed-state refusals, and records where the tree still falls
short so [#107](https://github.com/LennardZuendorf/vibe/issues/107) has a
definition to build against. Run real-feature drills for both backends. Include
ambiguous selection, missing artifacts, invalid task state and conflict at closeout. The durable-root decision remains open in [#104](https://github.com/LennardZuendorf/vibe/issues/104).

[Milestones](product-milestones.md) · [Technical contract](../tech/milestone-spec-feature-contract.md)
