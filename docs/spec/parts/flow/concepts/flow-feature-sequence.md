---
type: concept
updated: 2026-11-11
---

# Flow feature sequence

Rows of the Feature Sequence owned by the flow tool; delivery order and binary gates are in [the feature sequence](../../../concepts/feature-sequence.md).

| Order | Feature | Tool | Deliverable | Test | Status | Starts when |
|---:|---|---|---|---|---|---|
| F5 | flow-core | flow | Machine/policy in core; cursor at `.vibe/run/flow/state.json`; `state get`/`policy check` | Parity on `state get`/`policy decide` incl. mutated policies | DONE (source) | F3, F4 |
| F6 | flow-teeth-hooks | flow | Guard + Stop gate in Rust; `hook.sh` + `hooks.json` + frozen fallback; five-tier discovery | Parity on guard corpus + receipt×git corpus; tier matrix with deleted env | DONE (source) | F5 |
| F7 | machine-teeth | flow | Explicit v2 transitions/recovery, immutable task/revision records; trusted authority and audit ledger pending | Legal-edge/revision checks, corruption/retry/tamper tests; unavailable trusted gates deny | PARTIAL (source; strong gates unavailable) | F6 |
| F8 | flow-orders-payload | flow | `orders`/`doctrine`/`doctor` ported; `.vibe/run/inject/flow.json` payload writer; standalone AGENTS.md block | Parity per command; golden payload after transition; PATH-less spec probe degrades | NOT STARTED | F7 |
| F9 | flow-signals-hooks | flow | `vibe-signals` + `RulesProvider`; `PostToolUse(Task)`/`SubagentStop`/`SessionEnd` ledger events | `cargo metadata` proves core has no signals dependency; transcript fixtures → expected signals | NOT STARTED (held) | F8 |
| F10 | flow-tui | flow | `vibe-flow tui` v1 (live pane, transitions, recording); `flows list\|use` | `TestBackend` snapshots; recorded flow round-trips; gated prompts | NOT STARTED (held) | F7, F9 |

F5/F6 source: [Rust flow](../../../../../crates/vibe-flow/) and [hook checks](../../../../../flow/tests/hooks.sh). F7 progression and task records are implemented; trusted authority and native full blocking remain deferred. F8 and later remain pending. Source completion does not establish release or installation.

Flow workflow source now provides distinct `vibe`, `flow`, `spec` entrypoints, preserving migration and bounded agent methods. Its [live contract](flow-contract.md) replaces bootstrap duplication; publication is not authorized.
