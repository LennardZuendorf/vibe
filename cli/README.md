# vibe-flow — the vibe CLI

`vibe` is the self-hosting flow harness as a single installable command. It *is* the
15-state flow machine, and it makes project setup — skills, hooks, `AGENTS.md`, and the
dependency plugins — a one-command operation. Built with [Typer](https://typer.tiangolo.com)
+ [Rich](https://rich.readthedocs.io); `state-machine.json` stays the canonical data.

It supersedes the bash flow scripts (`flow/scripts/*.sh`) and `install.sh` — see
[Migration](#migration-from-the-bash-scripts).

## Install

```bash
uv tool install vibe-flow      # persistent PATH — required so the hooks resolve
vibe init                      # provision the current project
```

`uvx vibe-flow init` works for a one-off trial, but the settings.json hooks invoke the bare
`vibe-hook` binary; an ephemeral install makes them silently no-op, so `init` warns when
`vibe`/`vibe-hook` are not resolvable on `PATH`.

## Quickstart

```bash
vibe init [PATH]        # copy skills, register hooks, merge AGENTS.md, seed the cursor,
                        #   offer the dependency plugins — no manual /plugin step
vibe status            # where am I: flow / phase / feature + legal next states
vibe go feature.design # transition (refused unless it is a legal next state)
vibe doctor            # health report; --exit-code for CI
```

Two console scripts ship from this one package:

- **`vibe`** — the rich human app (below).
- **`vibe-hook`** — a stdlib-only fast entry (`inject` / `guard` / `gate`) that the
  `.claude/settings.json` hooks call on every turn. It imports no Typer/Rich/pydantic, so the
  per-edit guard stays near bash latency (~24 ms vs ~66 ms for the full app).

## Commands

| Command | Purpose |
|---|---|
| `vibe init [PATH]` | one-command project provisioning (`--yes`, `--only spec`, `--dry-run`) |
| `vibe status` / `vibe next` | current cursor + legal next states |
| `vibe go <state> [--feature F]` | legality-checked flow transition |
| `vibe doctor` | install health, per-check fix hints; `--exit-code` for CI |
| `vibe update [PATH]` | re-provision managed files, preserving the live cursor + prose |
| `vibe uninstall [PATH]` | surgical removal (`--yes` also drops the cursor) |
| `vibe check <path>` | write-policy verdict for a path (allow / warn / block) |
| `vibe orders` | print the current state's per-turn orders |
| `vibe rules` | regenerate the active-rules digest |
| `vibe spec validate` / `setup` | front the standalone spec framework (no flow runtime needed) |
| `vibe plugins list` / `install` / `add` | orchestrate `claude plugin …` for the dependencies |

Hooks fire straight from `.claude/settings.json` — no plugin registration required:

```json
"UserPromptSubmit":[{"hooks":[{"type":"command","command":"vibe-hook inject","timeout":10}]}],
"PreToolUse":[{"matcher":"Edit|Write|NotebookEdit","hooks":[{"type":"command","command":"vibe-hook guard","timeout":10}]}],
"Stop":[{"hooks":[{"type":"command","command":"vibe-hook gate","timeout":10}]}]
```

## Migration from the bash scripts

The CLI replaces the earlier bash provisioning and flow tooling:

| Was | Now |
|---|---|
| `./install.sh <target>` | `vibe init <target>` |
| `./install.sh <target>` (re-run) | `vibe update <target>` |
| `./install.sh <target> --uninstall` | `vibe uninstall <target>` |
| manual `/plugin` registration | none — hooks fire from `.claude/settings.json` |
| `flow/scripts/set-state.sh <s>` | `vibe go <s>` |
| `flow/scripts/detect-context.sh decide <p>` | `vibe check <p>` |
| `flow/scripts/orders.sh` | `vibe orders` (or the `vibe-hook inject` hook) |
| `flow/scripts/doctor.sh` | `vibe doctor` |
| `flow/scripts/regen-active-rules.sh` | `vibe rules` |

The `flow/scripts/*.sh` and `install.sh` are **deprecated** — kept only until the CLI is the
established path. The **spec framework** (`spec/` + `validate.sh`/`setup.sh`) is *not* replaced;
the CLI wraps it and it stays a standalone, zero-runtime, any-agent half.

## Development

```bash
cd cli
uv sync                    # editable install + dev deps
uv run --no-sync pytest -q # full suite
```

The three parity suites (`test_parity_{policy,orders,rules}.py`) assert byte-for-byte
equivalence with the bash origins and require `jq` + `bash`; they skip with a message if either
is absent.
