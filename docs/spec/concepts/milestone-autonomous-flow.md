---
type: concept
updated: 2026-11-21
---

# Autonomous Flow execution

A maintainer can approve a scoped plan, let agents work unattended within that scope and return to an intelligible verification result.

## Requirements

### Requirement: MS-FLOW-01 — Run bounded approved work
Flow SHALL dispatch eligible tasks from an approved plan on the reference host. It SHALL track results, bound retries and report progress, blockers and remaining work.

#### Scenario: Retry a fixable failure
- **GIVEN** a task fails a configured check without exhausting its retry budget
- **WHEN** Flow classifies the failure as retryable within the approved scope
- **THEN** it records the failure and retries the task while retaining its attempt history

### Requirement: MS-FLOW-02 — Resume safely
Flow SHALL persist enough state to resume interrupted runs. It SHALL reconcile unresolved work or effects before dispatching a duplicate attempt.

#### Scenario: Recovery after an interrupted worker
- **GIVEN** a worker was interrupted after writing a result or starting an effect
- **WHEN** the run resumes
- **THEN** Flow checks the recorded attempt and candidate, reuses a valid result, and escalates an effect whose status remains uncertain

### Requirement: MS-FLOW-03 — Enforce verification and approval
Flow SHALL hold the two human gates. Ship readiness SHALL depend on trusted checks of the exact candidate. Flow SHALL stop on changed scope, exhausted retries, stale evidence or missing approval.

#### Scenario: Candidate changes after verification
- **GIVEN** a trusted check passed on candidate A
- **WHEN** the worktree changes to candidate B
- **THEN** Flow withholds ship readiness until the new candidate has its own evidence and applicable approval

## Acceptance scope

Show start, dispatch, retry, recovery, verification and stop for human ship approval on one host and one repo. TUI and advisory signals do not supply enforcement authority.

[Milestones](product-milestones.md) · [Technical contract](../tech/milestone-flow-runner-contract.md)
