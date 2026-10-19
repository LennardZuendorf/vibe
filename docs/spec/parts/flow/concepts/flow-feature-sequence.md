---
type: concept
updated: 2026-10-19
---

# Flow feature sequence

Rows of the Feature Sequence owned by the flow tool; delivery order and binary gates are in [the feature sequence](../../../concepts/feature-sequence.md).

| Order | Feature | Tool | Deliverable | Test | Status | Starts when |
|---:|---|---|---|---|---|---|
| F5 | flow-core | flow | Machine/policy in core; cursor at `.vibe/run/flow/state.json`; `state get`/`policy check` | Parity on `state get`/`policy decide` incl. mutated policies | NOT STARTED | F3, F4 |
| F6 | flow-teeth-hooks | flow | Guard + Stop gate in Rust; `hook.sh` + `hooks.json` + frozen fallback; five-tier discovery | Parity on guard corpus + receipt×git corpus; tier matrix with deleted env | NOT STARTED | F5 |
| F7 | machine-teeth | flow | `state set` enforces `next` + `--confirm`; ledger append; `/flow` a thin wrapper | Non-`next` refused, unconfirmed gate refused; parity on legal edges | NOT STARTED | F6 |
| F8 | flow-orders-payload | flow | `orders`/`doctrine`/`doctor` ported; `.vibe/run/inject/flow.json` payload writer; standalone AGENTS.md block | Parity per command; golden payload after transition; PATH-less spec probe degrades | NOT STARTED | F7 |
| F9 | flow-signals-hooks | flow | `vibe-signals` + `RulesProvider`; `PostToolUse(Task)`/`SubagentStop`/`SessionEnd` ledger events | `cargo metadata` proves core has no signals dependency; transcript fixtures → expected signals | NOT STARTED (held) | F8 |
| F10 | flow-tui | flow | `vibe-flow tui` v1 (live pane, transitions, recording); `flows list\|use` | `TestBackend` snapshots; recorded flow round-trips; gated prompts | NOT STARTED (held) | F7, F9 |
