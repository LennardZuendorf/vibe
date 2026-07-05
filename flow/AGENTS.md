# AGENTS.md — `flow/` (Vibe Flow half)

Scoped guide for working **inside `flow/`**, the workflow state machine + skill.
The repo-root [`AGENTS.md`](../AGENTS.md) is canonical; this file only adds what
is local to this half. Read the root guide first.

## What lives here

The `vibe` flow skill. Ships to targets as `.agents/skills/vibe/` (a `-L`
dereferenced copy of this dir); the Claude adapter in `../.claude/` drives it.

| Path | Role |
|---|---|
| `SKILL.md` | Router + `## Orders` (D12 per-turn orders) |
| `setup.md`, `strategy.md`, `feature.md`, `quick.md`, `verify.md`, `compound.md`, `amend.md` | Phase guides |
| `state-machine.json` | **Data** — states, skills, legal `next`. Edit this, not prose |
| `state.example.json` | Template for the runtime cursor |
| `state.json` | Gitignored runtime cursor — seed only when testing transitions |
| `scripts/` | `set-state.sh`, `detect-context.sh`, `orders.sh`, `doctor.sh`, `merge-*.sh` |
| `reference/` | `deps.json`, `adapters.json`, templates |
| `tests/` | Behaviour suites (`flow/tests/run.sh` + `tests/adapters/`) — **source-only, never ships** |

## Working here

- Bash MUST use `set -euo pipefail`; MUST be shellcheck-clean.
- The state machine is **data** — change `state-machine.json`, never duplicate
  its transitions in prose.
- Never edit `state.json` by hand — only via
  `bash flow/scripts/set-state.sh <flow.phase> [feature]`.
- Self-locate by upward marker search (`.spec` / `.git`), not `../` hop counts.
- `tests/` and this `AGENTS.md` are pruned by `install.sh` on copy — contributor
  artifacts, not part of the shipped skill.

## Verify

```bash
bash flow/tests/run.sh              # flow core behaviour suite
bash flow/tests/adapters/run.sh     # adapters (hooks, install.sh, settings.json)
bash flow/scripts/doctor.sh         # install health
```
