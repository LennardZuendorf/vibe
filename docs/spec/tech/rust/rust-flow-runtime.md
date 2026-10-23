---
type: tech
applies:
  - "crates/vibe-flow/**"
  - "crates/vibe-signals/**"
  - "flow/state-machine.json"
  - ".vibe/run/flow/**"
updated: 2026-10-19
---

# machine-teeth

`vibe-flow state set <flow.phase> [feature] [--confirm]`: the target state
must exist and be in the current state's `next` (`idle` is always legal);
an edge listed in `gates` requires `--confirm` or the call refuses (exit 1,
listing the legal `next` set). No `--force` — repair is `state set idle`
then walking forward. Every write appends a ledger event. `/flow` becomes a
thin wrapper (`confirm` token → `--confirm`); the guard hook and the TUI
call the same `check_transition` function core exposes — one enforcement
point, not three.

## Ledger and signals

`.vibe/run/flow/ledger.jsonl`, `O_APPEND`, rotated at 5 MB, kinds
`transition | hook | signal`. `vibe-signals` defines `Signal{ts, source,
kind: progress|blocked|done|drift, confidence, evidence, state}` — no
verdict field — and `trait SignalProvider { fn observe(&Observation) ->
Vec<Signal> }`. `vibe-core` cannot depend on `vibe-signals` (cycle-free,
asserted by a `cargo metadata` test), so the guard and the gate are
structurally unable to read a signal. Ships now: `RulesProvider`
(fixed-confidence keyword rules) built from the `Stop`/`SubagentStop`
transcript tail. Later, opt-in: `ModelProvider` behind feature `model`.
Surfaced in the TUI's signals pane and as one advisory line in the inject
relay.

## TUI v1

`vibe-flow tui` (ratatui, feature `tui`): a live pane (cursor, legal `next`
with gate marks, orders, last 20 ledger events, one doctor line; 500 ms
mtime poll, no `notify` crate); transitions (pick `next`, gated edges
prompt `y/N`, `a` aborts — same `check_transition`, same teeth); recording
(`r` starts, `n` adds a note, `s` saves to
`~/.config/vibe/flows/<name>.json`, `%APPDATA%` on Windows). A recorded
flow is a linear subset of `state-machine.json`; a repo selects one via
`.vibe/flow.json` `{"machine": "<path>"}`, falling back to the repo's own
machine.
