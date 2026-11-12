---
type: concept
updated: 2026-11-12
---

# Flow feature sequence

Rows of the Feature Sequence owned by the flow tool; delivery order and binary gates are in [the feature sequence](../../../concepts/feature-sequence.md).

| Order | Feature | Tool | Deliverable | Test | Status | Starts when |
|---:|---|---|---|---|---|---|
| F5 | flow-core | flow | Machine/policy in core; cursor at `.vibe/run/flow/state.json`; `state get`/`policy check` | Parity on `state get`/`policy decide` incl. mutated policies | DONE (#57) | F3, F4 |
| F6 | flow-teeth-hooks | flow | Guard + Stop gate in Rust; `hook.sh` + `hooks.json` + frozen fallback; five-tier discovery | Parity on guard corpus + receipt×git corpus; tier matrix with deleted env | DONE (#57) | F5 |
| F7 | machine-teeth | flow | `state set` enforces `next` + `--confirm`; ledger append; `/flow` a thin wrapper | Non-`next` refused, unconfirmed gate refused; parity on legal edges | DELIVERED — PR #80/#81/#82, merge pending | F6 |
| F8 | flow-orders-payload | flow | `orders`/`doctrine`/`doctor` ported; `.vibe/run/inject/flow.json` payload writer; standalone AGENTS.md block | Parity per command; golden payload after transition; PATH-less spec probe degrades | NOT STARTED | F7 |
| F9 | flow-signals-hooks | flow | `vibe-signals` + `RulesProvider`; `PostToolUse(Task)`/`SubagentStop`/`SessionEnd` ledger events | `cargo metadata` proves core has no signals dependency; transcript fixtures → expected signals | NOT STARTED (held) | F8 |
| F10 | flow-tui | flow | `vibe-flow tui` v1 (live pane, transitions, recording); `flows list\|use` | `TestBackend` snapshots; recorded flow round-trips; gated prompts | NOT STARTED (held) | F7, F9 |

F5/F6 source: [Rust flow](../../../../../crates/vibe-flow/) and [hook checks](../../../../../flow/tests/hooks.sh), merged in [#57](https://github.com/LennardZuendorf/vibe/pull/57) (2026-11-03). F7 is delivered on the linear stack [#80](https://github.com/LennardZuendorf/vibe/pull/80) → [#81](https://github.com/LennardZuendorf/vibe/pull/81) → [#82](https://github.com/LennardZuendorf/vibe/pull/82), stacked on `main` at `c2881a7`; all three are `CLEAN` with 30 green checks and their merge is pending — delivered, not done. F8 is carded as [#28](https://github.com/LennardZuendorf/vibe/issues/28) against that stack top; F9/F10 stay held until the stack merges. Source completion does not establish release or installation.
