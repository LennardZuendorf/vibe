---
type: tech
applies: ["crates/vibe-flow/**", "flow/**"]
updated: 2026-11-21
---

# Trusted Flow runner contract

Build on the existing [v2 runtime](rust/rust-flow-runtime.md). The Flow writer
owns legal edges and gated transitions. The runner and all UI entry points use
that owner. Reference-host execution capability must be proven before enabling it.

## Contracts

- Persist run ID, revision, approved plan/scope, task and attempt identity,
  candidate identity, outcome and pending next action. Check revision on writes.
- Distinguish worker reports and advisory signals from trusted evidence.
  Verification binds the actual trusted producer, check result, run and exact
  candidate. A worker cannot attest its own readiness merely by writing a result.
- Bind human decisions to the approved scope and relevant revision or candidate.
  Candidate mutation invalidates evidence tied to the old candidate. The writer
  checks expected evidence and approval immediately before the gated transition.
- Dispatch from the approved task set using the proven host scope boundary.
  Keep retries bounded, report exhausted attempts and reconcile interrupted
  dispatches before replay. Missing authority or uncertain effects stop progress.
- Surface paused reason, pending task and evidence references for CLI consumers.
  Advisory signal or instruction payloads confer no permission.
- Run negative tests for forged results, stale revision, changed scope, duplicate
  resume, verification failure and declined human approval.

## Open design

[#101](https://github.com/LennardZuendorf/vibe/issues/101) decides trusted
collection and scoped actions, in [trusted verification receipts](flow-trusted-verification.md).
[#106](https://github.com/LennardZuendorf/vibe/issues/106)
chooses the reference host and checkpoint/retry policy. Avoid promising
universal host confinement. The TUI uses the same writer and outcomes.

[Milestones](../concepts/product-milestones.md) · [Product requirements](../concepts/milestone-autonomous-flow.md)
