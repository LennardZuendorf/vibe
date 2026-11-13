---
type: tech
applies:
  - "crates/vibe-flow/**"
  - "flow/state-machine.json"
  - ".vibe/run/flow/**"
updated: 2026-11-10
---

# Machine teeth: the transition check and the ledger

F7 adds legal-`next` enforcement and `--confirm` on gated edges, the shared
transition checker and the ledger append. Both halves are in: `state set`
asks the one transition check (`Machine::check_transition`) before it writes —
the target must name a state, must be in the current state's `next`, and a
`gates` edge needs `--confirm`; `idle` is legal from every state, a corrupt
cursor degrades to `idle`, and a refusal writes nothing. Every accepted write
appends one `transition` event to `.vibe/run/flow/ledger.jsonl`. The callers do
not re-implement the check: `/flow` is a thin wrapper over `vibe-flow state set`,
and `set-state.sh` delegates to the same binary when it is on `PATH` (an install
without the Rust build keeps the plain writer and says on stderr that the gate
was not enforced). Ledger rotation is still pending.

## Pending F7 remainder

Still pending in F7: ledger rotation, and executable GSD aliases. The current
alias table is data, not executable command support.

## Recorded divergence between the two writers

One difference between them is deliberate and recorded rather than fixed: when
the cursor leaf is an in-project symlink to a *directory*, the fallback writer
refuses it (`unsafe cursor path`) while the binary permits it and replaces the
link with a regular file, never following it. Both fail safe — the target
directory and its contents are untouched, and neither writer writes through a
link out of the project. The engine suite records it as a KNOWN DIVERGENCE on
the shell-refusal fixture in `flow/engine/tests/cursor.test.mjs`.

## Pending ledger contract

`.vibe/run/flow/ledger.jsonl`, `O_APPEND`, rotate at 5 MB retaining one
generation; kinds `transition | hook | signal`.

## Pending signals

Pending `Signal`: `{ts, source, kind: progress|blocked|done|drift,
confidence, evidence, state}`; no verdict. `SignalProvider::observe(&Observation)
-> Vec<Signal>`; `RulesProvider` reads Stop/SubagentStop transcript tails.
`ModelProvider` is later opt-in behind feature `model`; core never depends on
signals.
