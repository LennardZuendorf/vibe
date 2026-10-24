# AGENTS.md — `flow/` (Vibe Flow half)

Scoped guide for working **inside `flow/`**, the workflow state machine + skill.
The repo-root [`AGENTS.md`](../AGENTS.md) is canonical; this file only adds what
is local to this half. Read the root guide first.

## What lives here

The `vibe` flow skill. Ships to targets at `.agents/flow/`; this repo links that
path to `flow/`. The Claude adapter in `../.claude/` drives it.

| Path | Role |
|---|---|
| `SKILL.md` | Flow procedures, doctrine, and D12 per-turn orders |
| `state-machine.json` | **Data** — states, skills, legal `next`. Edit this, not prose |
| `state.example.json` | Template for the runtime cursor |
| `.vibe/run/flow/state.json` | Runtime cursor — write through `vibe-flow state set` |
| `scripts/` | `set-state.sh`, `detect-context.sh`, `orders.sh`, `doctor.sh`, `merge-*.sh` |
| `reference/` | `deps.json`, `adapters.json`, templates |
| `tests/` | Behaviour suites (`flow/tests/run.sh` + `tests/adapters/`) — **source-only, never ships** |

## Working here

- Bash MUST use `set -euo pipefail`; MUST be shellcheck-clean.
- The state machine is **data** — change `state-machine.json`, never duplicate
  its transitions in prose.
- Never edit the runtime cursor by hand — use `vibe-flow state set <state> [feature] [--confirm]`.
- Self-locate by upward marker search (`docs/spec` / `.git`), not `../` hop counts.
- `tests/` and this `AGENTS.md` are pruned by `install.sh` on copy — contributor
  artifacts, not part of the shipped skill.

## Verify

```bash
bash flow/tests/run.sh              # flow core behaviour suite
bash flow/tests/adapters/run.sh     # adapters (hooks, install.sh, settings.json)
bash flow/scripts/doctor.sh         # install health
```
