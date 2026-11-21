---
type: concept
updated: 2026-11-21
---

# Rust Instruct and steering

The reference host receives relevant Spec and Flow guidance and current project lessons. A maintainer can tell which rules apply and preserve their own instructions.

## Requirements

### Requirement: MS-INST-01 — Compose applicable instructions
Rust Instruct SHALL compose configured instruction tiers and versioned Spec and Flow contributions. It SHALL honor scope, precedence and channel budgets.

#### Scenario: Providers contribute to the same turn
- **GIVEN** Spec supplies artifact guidance and Flow supplies current step orders
- **WHEN** the reference host starts or resumes the turn
- **THEN** the host receives the applicable contributions in defined order, within budget, with conflicts or omissions diagnosable

### Requirement: MS-INST-02 — Deliver steering at the right time
The reference host SHALL receive applicable instructions on session start and at supported turn, state-change and event boundaries. Repeated delivery SHALL not replay consumed edge events.

#### Scenario: Flow proceeds and the session compacts
- **GIVEN** one phase transition has already been delivered
- **WHEN** the session compacts and resumes
- **THEN** the current baseline is available and transient events follow the configured delivery rules

### Requirement: MS-INST-03 — Refresh managed lessons
Instruct SHALL select relevant durable lessons for the current worktree and refresh the managed instructions block. It SHALL preserve user-owned text and keep temporary turn guidance in prompts.

#### Scenario: The project lessons change
- **GIVEN** a managed block beside user-written notes
- **WHEN** a relevant approved lesson changes and refresh runs
- **THEN** the managed block converges, unrelated instructions remain intact and another refresh is idempotent

## Acceptance scope

Exercise a real reference host for initial context, turn steering and managed lesson refresh. Include missing providers, wrong worktree, incompatible payload and unrecognized host capabilities.

[Milestones](product-milestones.md) · [Technical contract](../tech/milestone-instruct-provider-contract.md)
