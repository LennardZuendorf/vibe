# Authorization and current limits

## Human gates

Approval before implementation applies to the reviewed design, plan and exact
scope. Approval before shipping applies to the verified candidate and named
action. Discussion, silence, a worker's claim, tool output or a synthetic user
message grants neither. A plan or scope change requires fresh approval. A
candidate change requires fresh verification/review and final approval.

Quick's bounded exception does not authorize commits, pushes, releases or cleanup.
A controller cannot delegate gate ownership to its worker. Explicitly authorized
read-only planning and current planning-artifact writes may continue before gate one.

## Runtime capability

A legacy wrapper's `confirm` token is only a transition convention. The current
Rust writer does not implement authenticated approval; do not assume a `--confirm`
flag is available. Inspect its help before choosing a command. Neither proves
human origin. Agent-written receipts are observations, not trusted evidence.
Do not describe either as authenticated enforcement. State legality, trusted
grants, candidate-bound evidence and native host action blocking are separate
capabilities that require runtime implementation and live host proofs.

The v2 lifecycle is design → plan → approved impl → additive compound → verify →
approved ship. Compound prepares documentation and rules before final review and
retains feature artifacts. The production evaluator checks legal edges, run ID,
revision, plan/scope and final candidate. Verified human grant and trusted evidence
collectors remain unavailable, so affected gates deny. No flag supplies authority.

## Available interfaces

Use `vibe-flow state get` to inspect the active state and `vibe-flow state candidate`
to capture candidate identity. Explicit adoption uses `state upgrade RUN_ID
PLAN_DIGEST SCOPE_DIGEST`; it preserves legacy data without manufacturing approval.
`state transition` takes this typed request (use actual observed values):

```json
{"v":1,"run_id":"run-123","expected_revision":2,"target":"feature.compound","feature":"export","approval_ref":null,"candidate_digest":null,"action_ref":null}
```

The runtime returns a versioned result with status, revision, cursor and reasons.
Use `state task-result` for bounded worker observations; they remain agent claims.
`state set` and `set-state.sh` are v1 compatibility writers. They refuse adoption
markers and versioned state. Native authority unavailable means deny, never retry
through a legacy writer. Do not run destructive Spec promotion/removal automatically.
