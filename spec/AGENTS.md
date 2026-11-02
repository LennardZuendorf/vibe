# AGENTS.md — `spec/` (Vibe Spec half)

Scoped guide for working **inside `spec/`**, the bundled spec framework. The
repo-root [`AGENTS.md`](../AGENTS.md) is canonical; this file only adds what is
local to this half. Read the root guide first.

## What lives here

Markdown skills and agents, a hook shim, legacy bash scripts, and the tests. The
`vibe-spec` binary (`crates/vibe-spec/`) does the work the skills call. Ships to
targets as `.agents/skills/spec/` and `.agents/skills/using-spec/` (a `-L`
dereferenced copy).

| Path | Role |
|---|---|
| `SKILL.md` | Skill router — entry point the agent loads |
| `using-spec/` | Session-start router skill |
| `strategy.md`, `feature.md` | Guides (root strategy, fallback feature authoring) |
| `reference/` | Templates + per-doc authoring references |
| `scripts/` | Legacy bash scripts, flat root only — deterministic, shellcheck-clean |
| `agents/` | Claude Code agents (`spec-promoter.md`, `spec-health.md`) |
| `hooks/` | `run-hook.sh`, the fail-open shim for `vibe-spec hook` |
| `tests/` | Behaviour suites (`spec/tests/`) — **source-only, never ships** |

## Working here

- Bash MUST use `set -euo pipefail`; MUST be shellcheck-clean.
- Scripts stay deterministic, idempotent, graceful-degrade (warn, never hard-fail).
- Do not hardcode the repo root by counting `../` hops — self-locate by upward
  marker search (`docs/spec` / `.git`). See the `tests/` suites for the pattern.
- `tests/` and this `AGENTS.md` are pruned by `install.sh` on copy — they are
  contributor artifacts, not part of the shipped skill.
- Skill prose follows the authoring standard in the repo's tech spec: a rule lives
  in one file, a description says when to use the skill, and `tests/skills` lints it.

## Verify

```bash
cargo test -p vibe-spec                  # the vibe-spec binary
bash spec/tests/run.sh                   # this half's behaviour suite
bash spec/tests/agents.sh                # the two agents
bash spec/tests/pressure/run.sh          # pressure harness; needs the claude CLI
bash tests/skills/run.sh                 # skill, agent and doc lints
```
