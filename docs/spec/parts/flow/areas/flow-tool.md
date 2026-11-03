---
type: area
code:
  - "flow"
  - "crates/vibe-flow"
  - "flow/bin/hook.sh"
updated: 2026-11-03
---

# The flow tool

## Product piece

| Piece | What It Owns | Lives In |
|---|---|---|
| `flow` | State machine, cursor, guard/stop hooks, ledger, advisory signals, the TUI | `flow/` |

## Features

| Tool | Covers | Lives in |
|---|---|---|
| **flow** | State-machine workflow with real teeth: cursor, guard/stop hooks, ledger, signals, TUI. | [`flow/`](../../../../../flow/README.md) |

## Feature boundaries

| Tool | Owns | Does not own | Reads from peers | Writes for peers |
|---|---|---|---|---|
| flow | `state-machine.json`, cursor, orders, `policy.json`, guard, sniffer, Stop tooth, evidence, ledger, signals, TUI | Spec content, prompt injection, block composition | spec via `vibe-spec … --json` (soft) | Payload `.vibe/run/inject/flow.json`; provider manifest; its AGENTS.md block |

Current Rust source covers F5/F6 cursor, policy and guard/Stop behavior. Ledger, signals, TUI, payload emission and provider output remain pending ownership targets.
