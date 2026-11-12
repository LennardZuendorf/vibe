# Bounded task contract

## TaskBrief

The controller supplies these required fields before dispatch. Task and run IDs
contain 1–128 ASCII letters, digits, `-`, `_` or `.`; slashes are invalid.

```json
{"task_id":"export-2","run_id":"run-123","plan_digest":"sha256:…","requirements":["approved export task 2"],"read_paths":["src/export.rs"],"write_scope":["src/export.rs","tests/export.rs"],"dependencies":["export-1"],"checks":["export"],"role":"implementer"}
```

Use exact relative write paths. Runtime task-result validation accepts no globs. Include the active orders,
frozen design/plan paths, context, baseline failures, permission limits and
expected return format. Dependency results must be checked before dispatch.
Choose a capability first, then an available model under controlling preferences.
Serialize tasks that share files. Parallelize disjoint tasks only when permitted.

## TaskResult

```json
{"task_id":"export-2","status":"done","changed_paths":["src/export.rs","tests/export.rs"],"check_results":[{"id":"export","argv":["cargo","test","export"],"cwd":"/project","exit_code":0,"output_digest":"sha256:…","output_path":"runtime/export-test.log","started_at":"2026-11-11T10:00:00Z","finished_at":"2026-11-11T10:00:01Z"}],"findings":[],"artifacts":[],"candidate_digest":"git-object:<from state candidate>"}
```

Status is `done`, `needs_context`, `blocked` or `failed`. Return actual commands,
exit codes, outputs/artifact paths, baseline deltas, red/green observations and
remaining risks. Do not fabricate a digest; report unavailable when uncaptured.
Results are worker claims until the controller checks changed bytes and evidence.

## Controller ledger

Keep task IDs, dependency results, accepted candidate identity and review findings
in `.vibe/run/flow/work/<feature>/`. Never check off tasks in a frozen plan.
On resume, reuse valid checked results. Redispatch only incomplete, stale or
rejected work with the precise remaining scope. A changed dependency or plan
invalidates affected results and approval. Workers cannot approve themselves,
advance state, expand scope, commit, delete artifacts or finish branches.

## Consumers

Use [explorer](../agents/code-explorer.md), [architect](../agents/code-architect.md),
[implementer](../agents/implementer.md) and [reviewer](../agents/code-reviewer.md)
for the corresponding capability. Registration is optional; pass the same brief
to a generic agent when no matching native role exists.
