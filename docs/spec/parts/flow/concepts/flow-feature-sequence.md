---
type: concept
updated: 2026-11-14
---

# Flow feature sequence

Rows of the Feature Sequence owned by the flow tool; delivery order and binary gates are in [the feature sequence](../../../concepts/feature-sequence.md).

| Order | Feature | Tool | Deliverable | Test | Status | Starts when |
|---:|---|---|---|---|---|---|
| F5 | flow-core | flow | Machine/policy in core; cursor at `.vibe/run/flow/state.json`; `state get`/`policy check` | Parity on `state get`/`policy decide` incl. mutated policies | DONE (#57) | F3, F4 |
| F6 | flow-teeth-hooks | flow | Guard + Stop gate in Rust; `hook.sh` + `hooks.json` + frozen fallback; five-tier discovery | Parity on guard corpus + receipt×git corpus; tier matrix with deleted env | DONE (#57) | F5 |
| F7 | machine-teeth | flow | `state set` enforces `next` + `--confirm`; ledger append; `/flow` a thin wrapper | Non-`next` refused, unconfirmed gate refused; parity on legal edges | DONE (#80/#81/#82, 2026-11-13) | F6 |
| F8 | flow-orders-payload | flow | `orders`/`doctrine`/`doctor` ported; `.vibe/run/inject/flow.json` payload writer; standalone AGENTS.md block | Parity per command; golden payload after transition; PATH-less spec probe degrades | DONE (#93, 2026-11-13) | F7 |
| F9 | flow-signals-hooks | flow | `vibe-signals` + `RulesProvider`; `PostToolUse(Task)`/`SubagentStop`/`SessionEnd` ledger events | `cargo metadata` proves core has no signals dependency; transcript fixtures → expected signals | NOT STARTED (held) | F8 |
| F10 | flow-tui | flow | `vibe-flow tui` v1 (live pane, transitions, recording); `flows list\|use` | `TestBackend` snapshots; recorded flow round-trips; gated prompts | NOT STARTED (held) | F7, F9 |

F5/F6 source: [Rust flow](../../../../../crates/vibe-flow/) and [hook checks](../../../../../flow/tests/hooks.sh), merged in [#57](https://github.com/LennardZuendorf/vibe/pull/57) (2026-11-03). F7 merged on 2026-11-13 as the linear stack [#80](https://github.com/LennardZuendorf/vibe/pull/80) (`fd70635`) → [#81](https://github.com/LennardZuendorf/vibe/pull/81) (`0ad85ad`) → [#82](https://github.com/LennardZuendorf/vibe/pull/82) (`7d25b9e`). F8 shipped as [#93](https://github.com/LennardZuendorf/vibe/pull/93) (`7c868f1`) on 2026-11-13, which closed [#28](https://github.com/LennardZuendorf/vibe/issues/28). The unblock condition the F9/F10 rows named — this stack merging — is satisfied, and `tests/parity/GREEN.hard` is present on `main`, so the `port-order` CI job does not hold them either; what remains is an owner decision about when F9 starts, not a dependency. Source completion does not establish release or installation.
