# AGENTS.md — `spec/` (Vibe Spec half)

Scoped guide for working **inside `spec/`**, the bundled spec framework. The
repo-root [`AGENTS.md`](../AGENTS.md) is canonical; this file only adds what is
local to this half. Read the root guide first.

## What lives here

Pure `bash` + Markdown, **zero runtime**. Ships to targets as
`.agents/skills/spec/` (a `-L` dereferenced copy of this dir).

| Path | Role |
|---|---|
| `SKILL.md` | Skill router — entry point the agent loads |
| `strategy.md`, `feature.md` | Phase guides (root strategy, feature authoring) |
| `reference/` | Templates + per-doc authoring references |
| `scripts/` | `setup.sh`, `validate.sh` — deterministic, shellcheck-clean |
| `agents/` | Subagent prompts |
| `tests/` | Behaviour suite (`spec/tests/run.sh`) — **source-only, never ships** |

## Working here

- Bash MUST use `set -euo pipefail`; MUST be shellcheck-clean.
- Scripts stay deterministic, idempotent, graceful-degrade (warn, never hard-fail).
- Do not hardcode the repo root by counting `../` hops — self-locate by upward
  marker search (`.spec` / `.git`). See the `tests/` suites for the pattern.
- `tests/` and this `AGENTS.md` are pruned by `install.sh` on copy — they are
  contributor artifacts, not part of the shipped skill.

## Verify

```bash
bash spec/tests/run.sh          # this half's behaviour suite
bash spec/scripts/validate.sh   # structural spec validation
```
