# Feature procedure

## Inputs

Named requirement, discovered spec root, relevant lessons, design/plan artifacts,
current cursor/orders, authorization and task ledger.

## Prerequisites

Use read-only exploration before design. Delegate cross-module tracing to the
[explorer](../agents/code-explorer.md) and architecture decisions to the
[architect](../agents/code-architect.md) when authorized and useful. Use bounded
brainstorming and planning methods. Spec chooses artifact locations. Ask for
missing requirements; do not invent interfaces or expand scope.

## Artifacts

Create design and plan in their Spec-owned feature layer. Include task IDs,
dependencies, exact scopes and executable checks. Present the concrete plan at
gate one. After explicit approval, freeze it and keep progress in the runtime
ledger. Dispatch [TaskBriefs](../reference/task-contract.md) with bounded TDD,
debugging and review methods; never a whole-lifecycle executor.

The controller checks each returned result and dispatches independent review.
Serialize shared files; parallelize disjoint work only under controlling rules.
A worker cannot advance state or approve changes. Prepare additive docs, lessons
and generated rules for the candidate. Retain feature artifacts; do not run
`vibe-spec promote --done` or destructive removal as an automatic step.

## Checks

Run each task's real affected checks. Record commands, exit codes and output,
then final candidate checks and independent whole-candidate review after docs.
Inspect tracked and untracked changes. Worker claims and fresh timestamps are
insufficient evidence. Report baseline failures separately. Present candidate
identity, unresolved limitations and one exact proposed ship action at gate two.
No automatic commit, push, merge, release, branch finishing or cleanup.

## Recovery

Follow [recovery](../reference/recovery.md). Resume valid completed tasks instead
of redispatching them. Failed checks return to bounded implementation. Scope or
plan drift returns to planning and invalidates affected approval. Candidate
changes invalidate verification and final approval. Missing independent review
blocks readiness. Use existing machine edges until upgraded states are verified;
see [authorization](../reference/authorization.md).

## Example

For CSV export, approve a design and three scoped tasks: encoder, command wiring,
and docs. Reuse a checked encoder after a restart. Review the integrated candidate
after documentation preparation. “Tests pass” does not authorize committing it.
