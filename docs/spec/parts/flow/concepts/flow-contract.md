---
type: concept
updated: 2026-11-11
---

# Vibe Flow Contract

`vibe` orients the toolkit; `flow` owns the lifecycle; `spec` performs bounded
artifact operations. Flow resources live in `flow/`, discovered as
`.agents/skills/flow/`. Legacy discovery remains compatible during migration.
The primary cursor is `.vibe/run/flow/state.json`.

## Legacy v1

The cursor remains `{flow, phase, feature, updated}`. `state set` validates a
target name and writes atomically; it does not authenticate approvals or enforce
legal edges. JS/shell writers share the runtime lock and refuse adopted v2 state.
Migration preserves legacy cursor and receipt bytes before changing discovery.
Only established absence selects legacy fallback; invalid primary state cannot.
Legacy Stop uses receipt freshness, not a trusted verdict or candidate identity.
These compatibility guarantees do not establish strong enforcement.

## Explicit v2 adoption

`state upgrade` explicitly binds a run, plan and scope and returns mutation
phases to planning. No installation or read silently adopts v2 or grants access.
`state transition` validates legal edges and expected revision under a lock;
immutable revision records precede cursor publication. Idempotent retry accepts
identical records and refuses conflicting records. Adoption markers prevent
missing, versionless or malformed state from downgrading to v1.
`state recover` requires the observed corrupt digest and starts safe planning.

Progression is design → plan → impl → compound → verify → ship. Every entry
into feature implementation and shipping requires trusted approval. Production
approval and evidence verifiers are currently unavailable, so these entries
deny, including re-entry. A caller flag or model-authored record is not approval.
Explicit abort returns to idle without implying completion or permission to ship.

TaskBrief/TaskResult records validate declared paths, dependencies and successful
required checks. Accepted dependency identity survives legitimate downstream
changes; the controller must recheck affected work. Records remain agent claims.
Final readiness binds the integrated candidate, not a previous task snapshot.

`state candidate` hashes framed path/content/mode/deletion/symlink data for tracked
and all untracked paths, including ignored paths. Only `.vibe/run/flow` and
`target` are excluded. Git worktree roots are required; unsupported entries fail.
This identifies bytes; it does not prove protected storage or race-free execution.
See [runtime](../../../tech/rust/rust-flow-runtime.md) and
[authorization](../../../../../flow/reference/authorization.md).

Native hooks cannot prove complete action coverage or human origin across Claude
Code, OpenCode and Codex. Strong host enforcement remains unavailable/deny;
configured hooks and instruction evaluations are not native verification.
