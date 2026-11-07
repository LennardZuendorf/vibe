---
type: tech
applies:
  - "crates/vibe-flow/**"
  - "crates/vibe-signals/**"
  - "flow/state-machine.json"
  - ".vibe/run/flow/**"
updated: 2026-11-03
---

# Flow runtime: current source and pending targets

F5/F6 implement `state get|set`, `policy check|decide|list|render`, and
`hook pre-tool-use-guard|stop-gate` in `crates/vibe-flow/`.
F7's first half is in: `state set` asks the one transition check
(`Machine::check_transition`) before it writes — the target must name a state,
must be in the current state's `next`, and a `gates` edge needs `--confirm`;
`idle` is legal from every state, a corrupt cursor degrades to `idle`, and a
refusal writes nothing. Every accepted write appends one `transition` event to
`.vibe/run/flow/ledger.jsonl`. Ledger rotation is still pending, and `/flow`
still runs its own prose gate over `set-state.sh` rather than wrapping the
binary.
`policy check` refuses with exit 1; `decide` preserves exit 2.

## Repo-local layout

`FlowPaths` owns discovery: `--root` → `CLAUDE_PROJECT_DIR` → `VIBE_ROOT`
→ nearest `.vibe/` or `.git` ancestor → cwd. Data discovery is
`--vibe-dir` → `.agents/flow` when present → `.agents/skills/vibe`.
Runtime remains under `.vibe/run/`; flow does not use a per-user directory.
The [flow contract](../../parts/flow/concepts/flow-contract.md) specifies
absence-only cursor/evidence fallback, physical confinement and diagnostics.

## Current skill contracts

The flow skill has one `SKILL.md`; phase files are removed.
Doctrine is ≤700 bytes; each order block is ≤400 bytes and their average
is ≤250 bytes. The style summary is ≤220 bytes and the AGENTS template
is ≤110 lines. Executable checks live in `flow/tests/run.sh`.

## Pending targets

F7 adds legal-`next` enforcement and `--confirm` on gated edges, the shared
transition checker and the ledger append. Still pending in F7: the `/flow`
command becomes a thin wrapper over `vibe-flow state set`, ledger rotation, and
executable GSD aliases. The current alias table is data, not executable command
support.
`.planning/` is read-only context and never authoritative state.
Receipt-verdict enforcement is pending; current receipts use freshness only.

F8 adds Rust orders/doctrine/doctor, payload emission, provider manifest
and standalone managed AGENTS output. F11/F12 add instruct composition
and the hook ownership handoff. JS remains active until its retirement row.
F9 signals and F10 TUI/playback remain pending; advisory signals cannot
become blocking evidence. The writer-gate and ledger target decisions remain
[D27](../../decisions/d27-the-writer-is-the-gate.md) and
[D28](../../decisions/d28-ledger-advisory-signals.md).

Pending ledger contract: `.vibe/run/flow/ledger.jsonl`, `O_APPEND`, rotate
at 5 MB retaining one generation; kinds `transition | hook | signal`.
Pending `Signal`: `{ts, source, kind: progress|blocked|done|drift,
confidence, evidence, state}`; no verdict. `SignalProvider::observe(&Observation)
-> Vec<Signal>`; `RulesProvider` reads Stop/SubagentStop transcript tails.
`ModelProvider` is later opt-in behind feature `model`; core never depends on signals.
Pending recorded flows save to `~/.config/vibe/flows/<name>.json` (`%APPDATA%`
on Windows); global storage remains a deferred target. A project selects a
linear machine subset through `.vibe/flow.json` `{"machine":"<path>"}`,
falling back to its own machine. These targets do not describe current runtime.
