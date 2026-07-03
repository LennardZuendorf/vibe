---
type: feature-tech
feature: vibe-cli
sibling: product.md
parent: ../../tech.md
updated: 2026-07-03
---

# Feature: vibe-cli — Architecture

Port the bash flow scripts + `install.sh` + manual `/plugin` dance into one
installable Python CLI. `state-machine.json` stays canonical data; a `typer`+`rich`
app (`vibe`) fronts every human command, and a stdlib-only second entry (`vibe-hook`)
carries the per-turn hooks. The spec half stays bash and standalone; the CLI wraps it.

**Parent:** [../../tech.md](../../tech.md)
**Design (approved):** [research.md](research.md) — Approach A, decisions D1–D5.

---

## Architecture

### Dual entry — the load-bearing decision

The `PreToolUse` guard fires per-Edit (dozens of invocations in one impl turn), so its
startup cost is the whole perf budget. Measured (M-series, Python 3.14, warm, avg of 8):

| path | ms/run |
|---|---|
| bash + jq (current guard) | ~10 |
| python + **stdlib only** (json/argparse) | ~24 |
| python + typer/rich/pydantic | ~66 |

Routing the guard through the full `typer` app (~66 ms, worse on cold cache / CI / slow
disks) adds real lag exactly during implementation. So the package ships **two
console_scripts**: `vibe-hook` is stdlib-only `argparse` and imports only the cheap
`policy`/`orders`/`cursor` modules — the rich `typer` app never loads on the hot path.
Both entries read the same `policy.py`, so verdicts can't drift. Mirrors `indexed`'s
`indexed` + `indexed-mcp` split.

### Package (src-layout, hatchling)

Three names, kept distinct: PyPI distribution `vibe-flow`; import package `vibe`;
console_scripts `vibe` and `vibe-hook`.

```
cli/
├── pyproject.toml          # name = vibe-flow; hatchling; src-layout
│                           # [project.scripts] vibe = "vibe.app:main"
│                           #                    vibe-hook = "vibe.hook:main"
└── src/vibe/
    ├── app.py              # rich typer app + @callback + main(); registers sub-apps
    │                       #   (incl. `spec` → shells validate.sh/setup.sh, no flow runtime)
    ├── hook.py             # SECOND entry (vibe-hook): stdlib argparse; inject|guard|gate
    ├── machine.py          # pydantic load of state-machine.json (StateMachine, State)
    ├── cursor.py           # read + atomic-write state.json (ports set-state semantics)
    ├── policy.py           # allow/warn/block oracle + the 3 hard blocks (cheap import)
    ├── orders.py           # D12 resolution: SKILL.md marker → machine.inject → fallback
    ├── provision/          # init / uninstall / update
    │   ├── settings.py     #   read/merge .claude/settings.json hook entries (idempotent)
    │   ├── agents_md.py    #   port merge-agents.sh markers (5 cases + reversal + unmerge)
    │   └── plugins.py      #   orchestrate `claude plugin install/add` from deps.json
    ├── rules.py            # regen active-rules digest (ports regen-active-rules.sh)
    ├── doctor.py           # rich health report (ports doctor.sh) + fix hints; --exit-code for CI
    ├── ui/                 # console singleton, theme, alerts/cards/panels (indexed)
    └── errors.py           # VibeError base; typer.Exit(1) at call sites
tests/                      # CliRunner + Path.home sandbox; parity vs current bash
```

---

## State machine & cursor

`state-machine.json` (`version:"1"`, 15 states) **stays the canonical data contract** —
repo convention "state machine is data". `machine.py` **loads and validates** that file at
runtime with **stdlib `json`** (not pydantic) so it is safe to import on the per-Edit guard
hot path (R6); the 15 states, their `next`/`writes`/`inject`/`caveman` fields are never
hardcoded in Python. `pydantic` is reserved for the rich path (config / display models) and
never appears in a hot-path module. Editing the flow means editing the JSON, exactly as today.

The cursor `state.json` `{flow, phase, feature, updated}` stays plain JSON. It is
**writer-only through the CLI** (`vibe go` → `cursor.py`), written atomically
(temp-file + `os.replace`), never hand-edited — porting `set-state.sh` semantics.

---

## Data contracts

### `.claude/settings.json` hook entries (written by `vibe init`)

`vibe init` merges three entries into the target's `.claude/settings.json` (idempotent,
keyed by event + command). Unlike the plugin's `hooks.json`, settings.json hooks receive
`CLAUDE_PROJECT_DIR` but **not** `CLAUDE_PLUGIN_ROOT`, so they invoke the bare binary on
`PATH` and self-locate the project:

```json
"UserPromptSubmit":[{"hooks":[{"type":"command","command":"vibe-hook inject","timeout":10}]}],
"PreToolUse":[{"matcher":"Edit|Write|NotebookEdit",
  "hooks":[{"type":"command","command":"vibe-hook guard","timeout":10}]}],
"Stop":[{"hooks":[{"type":"command","command":"vibe-hook gate","timeout":10}]}]
```

- `guard` **blocks via exit 2** on a hard-block verdict (unchanged from the bash guard).
- Each `vibe-hook` invocation finds the project via `CLAUDE_PROJECT_DIR`, falling back to
  upward marker search (`.spec`/`.git`) — no `CLAUDE_PLUGIN_ROOT` reliance.
- Hooks fire on the next session with no `/plugin` step. `plugin.json` stays only as an
  optional secondary channel.

### Skills install dir (resolves research §8)

`.agents/skills/{spec,vibe}/` remains the portable canonical source. `vibe init` copies
those trees into `.claude/skills/{spec,vibe}/` for Claude Code auto-discovery; the canonical
`.agents/` copy is untouched.

### Reused markers

`vibe:instructions` (managed `AGENTS.md` block, via `agents_md.py`) and `vibe:active-rules`
(top-5 lessons digest, via `rules.py`, regenerated during `init`/`update`) keep their exact
marker pairs and reversal guard.

### The 3 hard write-blocks (port verbatim into `policy.py`)

1. `.spec/lessons.md` — writable only in a `*.compound` state.
2. Root `.spec/{product,tech,design,plan}.md` — only in `strategy.spec`, `feature.compound`, or `setup.apply`.
3. `.agents/skills/vibe/state.json` — never by direct edit; only via the CLI.

Everything else is allow/warn. `guard` returns exit 2 on any of these; `vibe check <path>`
exposes the same verdict for humans.

---

## Distribution

Publish `vibe-flow` to PyPI. Primary install: `uv tool install vibe-flow` — persistent
`PATH`, then `vibe init`. `uvx vibe-flow init` works for a trial run, but `init`'s prereq
check **verifies `vibe` and `vibe-hook` are resolvable on `PATH`** and warns loudly if they
are only ephemeral — otherwise the settings.json hooks reference binaries that vanish and
**silently no-op** (the bootstrapping trap). No `git clone` required for either path.

---

## Invariants preserved (research §5d — must not regress)

- `state-machine.json` is canonical; a **stdlib `json`** layer loads+validates it (hot-path
  safe), never hardcodes the 15 states; pydantic is reserved for the rich path.
- The 3 hard write-blocks port verbatim; `guard` still exits 2 on block.
- Cursor is writer-only through the CLI, written atomically; never hand-edited.
- Re-install/update **preserves the live cursor and user prose** (standing lesson).
- Uninstall is **surgical** — per-file inverse, preserving shared dirs / `.spec/` / prose.
- Hooks self-locate by `CLAUDE_PROJECT_DIR` + marker search (no `CLAUDE_PLUGIN_ROOT`).
- The spec half stays bash, zero-runtime, and standalone; the CLI only wraps it.

---

## Testing

- **Unit / command:** `pytest` + `typer.testing.CliRunner`, each test in a `Path.home`
  sandbox (the `indexed` pattern) so provisioning writes into a temp `HOME`/target, never
  the dev tree.
- **Parity (the load-bearing suite):** for `vibe-hook inject|guard|gate` and `vibe check`,
  assert the Python verdict/orders are **byte-identical** to the current bash scripts
  (`detect-context.sh decide`, `orders.sh`) across a state × path fixture matrix — this is
  what lets the bash scripts retire without behaviour drift.
- **Regression pins (port the standing lessons):** a live cursor (`feature.impl <feature>`)
  survives `vibe update`; a **discriminating** uninstall test — drop a user file into each
  shared dir, run `vibe uninstall`, assert the user file survives *and* the shipped file is
  gone (fails if surgical removal is replaced by a blanket `rm`).

---

## Risks

| Risk | Mitigation |
|---|---|
| Ephemeral `uvx` install → hooks reference missing binary, silent no-op | `init` prereq check verifies `vibe`/`vibe-hook` on persistent `PATH`; warn loudly |
| Guard latency regresses toward the 66 ms full-app path | `vibe-hook` is stdlib-only; import-cost parity test guards against pulling in `typer`/`rich`/`pydantic` on the hot path |
| Python port diverges from bash verdicts during migration | Byte-for-byte parity suite is a merge gate; keep bash scripts until parity is green |
| Runtime shift: flow now needs Python (D3) | Spec half stays pure bash/standalone; only the flow half takes the `uv` dependency |
| settings.json merge clobbers user hooks | Keyed, idempotent merge in `settings.py`; never rewrite unrelated entries |
