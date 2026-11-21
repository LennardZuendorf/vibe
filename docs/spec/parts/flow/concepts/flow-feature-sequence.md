---
type: concept
updated: 2026-11-21
---

# Flow feature sequence

Rows of the Feature Sequence owned by the flow tool; delivery order and binary gates are in [the feature sequence](../../../concepts/feature-sequence.md).

| Order | Feature | Tool | Deliverable | Test | Status | Starts when |
|---:|---|---|---|---|---|---|
| F5 | flow-core | flow | Machine/policy in core; cursor at `.vibe/run/flow/state.json`; `state get`/`policy check` | Parity on `state get`/`policy decide` incl. mutated policies | DONE (#57) | F3, F4 |
| F6 | flow-teeth-hooks | flow | Guard + Stop gate in Rust; `hook.sh` + `hooks.json` + frozen fallback; five-tier discovery | Parity on guard corpus + receipt×git corpus; tier matrix with deleted env | DONE (#57) | F5 |
| F7 | machine-teeth | flow | `state set` enforces `next` + `--confirm`; ledger append; `/flow` a thin wrapper | Non-`next` refused, unconfirmed gate refused; parity on legal edges | DONE (#80/#81/#82, 2026-11-13) | F6 |
| F8 | flow-orders-payload | flow | `orders`/`doctrine`/`doctor` ported; `.vibe/run/inject/flow.json` payload writer; standalone AGENTS.md block | Parity per command; golden payload after transition; PATH-less spec probe degrades | DONE (#93, 2026-11-13) | F7 |
| F9 | flow-signals-hooks | flow | `vibe-signals` + `RulesProvider`; `PostToolUse(Task)`/`SubagentStop`/`SessionEnd` ledger events | `cargo metadata` proves core has no signals dependency; transcript fixtures → expected signals | NOT STARTED ([#100](https://github.com/LennardZuendorf/vibe/issues/100); advisory) | F8 |
| F10 | flow-tui | flow | `vibe-flow tui` v1 (live pane, transitions, recording); `flows list\|use` | `TestBackend` snapshots; recorded flow round-trips; gated prompts | BLOCKED ([#102](https://github.com/LennardZuendorf/vibe/issues/102)) | F7, F9 |

F5/F6 merged in [#57](https://github.com/LennardZuendorf/vibe/pull/57). F7 merged as [#80](https://github.com/LennardZuendorf/vibe/pull/80) → [#81](https://github.com/LennardZuendorf/vibe/pull/81) → [#82](https://github.com/LennardZuendorf/vibe/pull/82). F8 merged in [#93](https://github.com/LennardZuendorf/vibe/pull/93). The hard-parity gate is present and F9's source dependency is satisfied. F10 still depends on F9.

The [opt-in v2 runtime core](../../../tech/rust/rust-flow-runtime.md#adopted-v2-runs) merged separately in [#96](https://github.com/LennardZuendorf/vibe/pull/96). [#101](https://github.com/LennardZuendorf/vibe/issues/101) tracks the decision on trusted verification and scoped action authority. F9 signals remain advisory. The approved product sequence puts trusted verification and the [agent loop (#106)](https://github.com/LennardZuendorf/vibe/issues/106) ahead of TUI delivery. F10 also depends on F9.
