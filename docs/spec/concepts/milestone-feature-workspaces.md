---
type: concept
updated: 2026-11-21
---

# Feature workspaces across repos

A maintainer can start one feature spanning several repositories without losing repository identity, branch ownership or project context.

## Requirements

### Requirement: MS-WS-01 — Group independent worktrees
Vibe SHALL create and list a workspace containing independent worktrees tied to one feature. The workspace SHALL report each repository, base, branch, local status and selected feature backend.

#### Scenario: Open an Indexed and web feature
- **GIVEN** the Indexed and web repositories are registered and the user selected their bases
- **WHEN** the workspace is created
- **THEN** both worktrees appear beneath one workspace and retain independent Git branches and histories

### Requirement: MS-WS-02 — Bootstrap and recover safely
Vibe SHALL apply the correct configuration in each checkout and set up the selected feature backend. It SHALL report partial creation and provide a safe retry or cleanup path.

#### Scenario: The second checkout cannot be created
- **GIVEN** one member is ready and the other member fails to bootstrap
- **WHEN** the user retries or requests cleanup
- **THEN** Vibe reports the state of each member and preserves unrelated worktrees and uncommitted user changes

### Requirement: MS-WS-03 — Verify the feature as a group
Flow SHALL report per-repo results and require current verification evidence for every scoped repository before the feature can reach ship approval.

#### Scenario: One repository changes after checking both
- **GIVEN** both candidates have verification evidence
- **WHEN** the web candidate changes
- **THEN** the workspace loses ship readiness and identifies the repository that needs fresh evidence

## Acceptance scope

Start, interrupt and resume one real two-repo feature. Prove worktree isolation, feature lookup, configuration idempotence, a per-repo failure, and the final human gate.

[Milestones](product-milestones.md) · [Technical contract](../tech/milestone-workspace-contract.md)
