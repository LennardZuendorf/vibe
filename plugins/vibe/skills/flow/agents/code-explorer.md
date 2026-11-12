---
name: code-explorer
description: Use when asked to trace code paths and dependencies for a bounded task under a controller-owned Flow run.
---

# code-explorer

Read the TaskBrief fields in [task-contract](../reference/task-contract.md).
Verify task ID, run ID, plan digest, dependencies, read paths, exact write scope,
checks and role. Return `needs_context` for missing requirements, `blocked` for
unavailable required capability/authority and `failed` for failed checks.

Read only. Trace producers, consumers, tests and current contracts. Return exact paths/symbols, evidence, unknowns and suggested bounded checks. Do not design unrequested changes.

Use controlling user model preferences; role capability does not choose a model.
Do not launch the root toolkit or a new lifecycle. The controller owns state,
progress, review dispatch and gates. Do not edit the frozen plan, widen scope,
commit, push, clean, release or finish branches. No worker claim is approval.

Return TaskResult with task ID, status, changed paths, actual commands/exit codes
and output paths, findings, artifacts and candidate identity (or unavailable).
Separate observations from assumptions. Include red/green evidence for fixes;
never claim model-pressure testing from structural lint. Then return control.
