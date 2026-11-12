---
type: flow
updated: 2026-11-11
---

# Phase map

`flow/state-machine.json` owns legal edges, write surfaces and delegate metadata.
D12 orders live once in `flow/SKILL.md`; procedure files explain bounded methods.
The feature gates are plan → impl and verify → ship. Compound prepares docs
before final candidate checks. Any v2 implementation re-entry requires approval;
production trusted verifiers are unavailable, so those entries currently deny.

| Phase | Purpose |
|---|---|
| `idle` | Select a flow from user intent; abort does not imply completion. |
| `setup.detect` | Read-only inventory and missing-resource diagnosis. |
| `setup.apply` | Legacy compatibility; no blanket repair authorization. |
| `strategy.brainstorm` | Discuss project direction without product writes. |
| `strategy.spec` | Request a bounded root-artifact update and validate it. |
| `feature.design` | Explore and write the design in the selected feature layer. |
| `feature.plan` | Define exact task scopes, dependencies and checks for approval. |
| `feature.impl` | Dispatch bounded workers; freeze approved design and plan. |
| `feature.compound` | Prepare additive docs, lessons and regenerated rules. |
| `feature.verify` | Check and independently review the final integrated candidate. |
| `feature.ship` | Execute only the approved candidate's named action. |
| `quick.triage` | Diagnose and mechanically confirm Quick scope. |
| `quick.fix` | Implement the bounded fix; escalate contract changes. |
| `quick.verify` | Verify the fix and record durable lessons. |

Runtime progress lives outside frozen plans. Worker results are claims until
checked. Resume preserves accepted dependencies and redispatches incomplete or
invalidated work. Candidate changes invalidate final review/readiness.
No phase implies commit, push, merge, release, destructive promotion or cleanup.

JS still composes injection; Rust orders/payload and Instruct composition remain
separate work. See [workflow surface](workflow-surface.md) and
[runtime](../../../tech/rust/rust-flow-runtime.md).
