---
type: tech
applies:
  - "crates/vibe-flow/**"
  - "flow/state-machine.json"
  - ".vibe/run/flow/**"
updated: 2026-11-08
---

# Machine teeth: the transition check and the ledger

F7 adds legal-`next` enforcement and `--confirm` on gated edges, the shared
transition checker and the ledger append. The first half is in: `state set`
asks the one transition check (`Machine::check_transition`) before it writes —
the target must name a state, must be in the current state's `next`, and a
`gates` edge needs `--confirm`; `idle` is legal from every state, a corrupt
cursor degrades to `idle`, and a refusal writes nothing. Every accepted write
appends one `transition` event to `.vibe/run/flow/ledger.jsonl`. Ledger
rotation is still pending, and `/flow` still runs its own prose gate over
`set-state.sh` rather than wrapping the binary.

## Pending F7 remainder

Still pending in F7: the `/flow` command becomes a thin wrapper over
`vibe-flow state set`, ledger rotation, and executable GSD aliases. The current
alias table is data, not executable command support.

## Pending ledger contract

`.vibe/run/flow/ledger.jsonl`, `O_APPEND`, rotate at 5 MB retaining one
generation; kinds `transition | hook | signal`.

## Pending signals

Pending `Signal`: `{ts, source, kind: progress|blocked|done|drift,
confidence, evidence, state}`; no verdict. `SignalProvider::observe(&Observation)
-> Vec<Signal>`; `RulesProvider` reads Stop/SubagentStop transcript tails.
`ModelProvider` is later opt-in behind feature `model`; core never depends on
signals.
