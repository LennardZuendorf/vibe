---
type: feature-tech
feature: cli-restructure
sibling: product.md
parent: ../../tech.md
updated: 2026-07-04
---

# Feature: CLI restructure — Architecture

Restructure `cli/` into a `uv` workspace of four packages — a lean `vibe-core`
library, two stdlib-argparse agent apps (`vibe-flow`, `vibe-spec`), and the rich
human app (`vibe`) — with three console_scripts. Flow logic re-homes into
`vibe_flow`; the six spec scripts port into `vibe_spec` behind a byte-parity gate;
then a hard cutoff makes the packages canonical and removes **all** runtime bash
(flow + spec) plus the plugin bash-shim hooks. Deps point only downward and are
kept as few as possible, so each agent tool stays lean and the hot path stays
stdlib-fast.

**Parent:** [../../tech.md](../../tech.md)
**Requirements:** [product.md](product.md)
**Plan:** [plan.md](plan.md)

---

## Package graph

```
vibe-core   (stdlib)  ── markers (cli-provisioning), errors, paths, asset load
   ▲   ▲   ▲
   │   │   └───────────── vibe-spec (stdlib)  ── app: vibe-spec
   │   └───────────────── vibe-flow (stdlib)  ── app: vibe-flow  (subsumes vibe-hook)
   └── vibe-cli (typer+rich, deps: core+flow+spec) ── app: vibe
```

| Dist | Import pkg | Third-party deps | console_script | Consumer |
|---|---|---|---|---|
| `vibe-core` | `vibe_core` | — | — (library) | shared |
| `vibe-flow` | `vibe_flow` | — (stdlib argparse) | `vibe-flow` | agent + hooks |
| `vibe-spec` | `vibe_spec` | — (stdlib argparse) | `vibe-spec` | agent |
| `vibe` | `vibe_cli` | `typer`, `rich` | `vibe` | human |

No dependency cycle: `vibe-cli → {vibe-flow, vibe-spec} → vibe-core`, strictly
downward; `vibe_cli` importing `vibe_flow`/`vibe_spec` logic for re-render is
sound. Deps are kept as few as possible (R1/D5), not dogmatically zero — but the
`vibe-flow hook` hot path stays stdlib-only for latency. `pydantic` is dropped
(D5): it is a **phantom dep** — no `src/`/`tests/` file imports it; removal is
deleting an unused direct dependency, not a model migration. One lockstep version
across all four `pyproject`s + `plugin.json` (D7).

---

## Asset ownership (the complete map)

`test_assets_sync.py` is the current source-of-truth map; every class it tracks
gets a package owner. Post-cutover these package `_assets/` are canonical:

| Asset | New owner | Consumed by |
|---|---|---|
| `state-machine.json`, `state.example.json` | `vibe-flow` | `machine.py`, provisioning |
| flow skill tree (`skills/vibe/**`: SKILL.md, phase files, `merge-agents.sh`→retired) | `vibe-flow` | `vibe init` provisioning |
| `deps.json`, `adapters.json` | `vibe-flow` | `plugins.py`, `doctor.py` |
| `templates/AGENTS.md` (a **flow** asset) | `vibe-flow` | `provision/agents_md.py` |
| spec skill tree (`skills/spec/**`) + `reference/templates/**` + validate fixtures | `vibe-spec` | `vibe init`, `setup` |

`vibe_cli.provision` locates the two sibling packages' `_assets/` via
`importlib`/`__file__`, not a single local tree (today `skills.py` reads one
package's `_assets` with `SKILL_NAMES=("spec","vibe")`; post-split those trees
live in two installed packages). Unit /10's retargeted asset-sync test asserts
package↔root-symlink integrity, not a byte mirror.

---

## Workspace layout

```
cli/
├── pyproject.toml                     # [tool.uv.workspace] members = ["packages/*"]; one lockstep version
├── uv.lock                            # single lock across the workspace
├── conftest.py / tests/               # shared fixtures (bash_ref, home_sandbox, target_project, golden)
└── packages/
    ├── vibe-core/  src/vibe_core/     assets.py · markers.py (provisioning guard) · errors.py · paths.py
    ├── vibe-flow/  src/vibe_flow/     machine cursor policy orders · hook.py · verbs.py+render.py · app.py(argparse)
    │                                  _assets/: state-machine.json, state.example.json, skills/vibe/**, deps.json, adapters.json, templates/AGENTS.md
    ├── vibe-spec/  src/vibe_spec/     model validate setup listing lessons merges · render.py · app.py(argparse)
    │                                  _assets/: skills/spec/**, reference/templates/**
    └── vibe-cli/   src/vibe_cli/      app.py(typer) · provision/{settings,agents_md,plugins,skills}.py · doctor.py rules.py · ui/
```

---

## Contract / API

Behavior is a pure function per command returning a structured result; a renderer
formats it. The stdlib app calls the plain renderer, `vibe_cli` a rich one — R3.

```python
# vibe_spec/validate.py   → ValidationReport(errors, warnings, exit_code)
# vibe_spec/render.py     → render_plain(report) -> str   # byte-identical to validate.sh
# vibe_flow/verbs.py      → go(root, target, feature) -> TransitionResult(old, new, feature, warnings[])
#                           status/next/check/orders likewise return result objects (no printing in logic)
# vibe_cli/app.py         → imports the above, applies rich views
```

**Hooks — one path only (D2).** `settings.json` entries change from `vibe-hook
<event>` to `vibe-flow hook <event>`. `provision/settings.py` gains a **migration
step**: on `update`/`init` it first `unmerge`s any legacy `vibe-hook inject|guard|
gate` commands, then merges the three `vibe-flow hook …` — so an already-
provisioned target never ends with both (today `merge()` only adds, which would
double-fire the guard). The plugin bash-shim hooks (`.claude-plugin/plugin.json`
→ `.claude/hooks/hooks.json` → `pre-tool-use-guard.sh` → `detect-context.sh`)
are **retired**. `doctor.py`'s "three `vibe-hook` entries" check updates in
lockstep.

**Spec skill invocations.** `spec/SKILL.md`: `bash …/validate.sh` → `vibe-spec
validate`; the auto-executing embed `` !`bash …/list-specs.sh` `` → a
preflight-guarded `` !`vibe-spec list || echo "install vibe-spec: see README"` ``
so a missing binary degrades to a documented line, not a raw error (R4). The
`compatibility:` field changes from `Requires bash` to `Requires vibe-spec`.

---

## Implementation Detail

- **Flow re-home is a move for the *oracles*, a rewrite for the *verbs*.**
  `machine`/`cursor`/`policy`/`orders`/`hook` relocate unchanged (+ their parity
  tests). But `status`/`next`/`go`/`check`/`orders` today are **typer command
  bodies with rich printing inline** (`commands/flow_cmds.py`; `check`/`orders`
  live in `commands/spec_cmd.py` and move out). Re-homing them means extracting
  logic into `vibe_flow/verbs.py` (returning result objects), an argparse
  `app.py`, and a plain `render.py` — real decomposition, scoped in /3.
- **Spec port — the parity boundary (pin before /7).** Reproduce bash semantics,
  not a cleaned-up version: `validate.sh` **ignores `$SPEC_DIR`** (hardcodes
  `.spec`) while `scan-merges`/`lessons-for` honor `${SPEC_DIR:-.spec}` — match
  per-script. Hand-rolled JSON in `lessons-for`/`scan-merges` is **unescaped**
  (`printf %s`) — reproduce the literal (broken) output, not `json.dumps`.
  Regex-as-data: SF12 `grep "$r_id"` is unanchored substring (`R1`⊂`R10`); SF14
  `grep "^${item}\t"` treats scope text as a regex; tag match is case-insensitive
  regex; the markdown-link ERE `\[.*?\]\(…\)` is **greedy** in grep but lazy in
  Python `re` — port the *behavior*. Byte details: the three prefixes
  `  ERROR: `/`  WARN:  `/`  OK:    ` are load-bearing; `wc -l` counts newlines
  (differs from `len(splitlines())` on no-final-newline files); `${line:0:60}`
  and setup blocks are **byte** slices. `promote` writes `mktemp` in the
  *target's* dir (`.promote.XXXXXX`) with `printf '\n%s\n'` framing and strict
  `<!-- merge -->` equality — `vibe_spec/merges.py` reproduces this exactly.
  `setup`'s trailing path/`bash validate.sh` hint lines are rewritten for the new
  layout and excluded from parity (R2). **SF4 dropped** (D9).
- **`merges.py` is net-new (D8).** Multi-block iteration + nesting detection over
  bare `<!-- merge -->` markers — a different grammar from `vibe_core/markers.py`
  (`{name}:start`/`:end`, single region). Do not credit it as guard reuse.
- **argparse by default on agent paths (D3).** A fresh-interpreter `sys.modules`
  import-cost test pins the `vibe-flow hook` path to stdlib-only (no `typer`/
  `rich`/`pydantic`); a whole-app import-purity test also pins `vibe-spec` lean.
- **Distribution (D7).** `install.sh` becomes a GH bootstrap: remove any prior
  combined `vibe-flow` install, then `uv tool install` the three tools from the
  checkout — **validated**, since `vibe` depends on `vibe-flow`/`vibe-spec` as
  workspace path-deps whose resolution from a git clone is non-trivial.
- **Golden fixtures (D6).** Before /10 deletes bash, capture per-command golden
  stdout/stderr/exit so a post-cutover regression test keeps a reference.

<!-- merge -->
CLI packaging pattern for this repo: a `uv` workspace with a lean `vibe-core`
library and per-consumer apps — `argparse` entry points (deps minimal; the
hook/hot path stdlib for latency) for anything an agent or hook invokes, one
`typer`/`rich` app for humans. Split by *who calls it and what it may import*,
never by domain tidiness. Behavior lives once (result objects); the rich app
re-renders. Distribute from GitHub via an install script (no PyPI); freeze all
package `pyproject` deps + one lockstep version + the workspace lock in the
skeleton unit. A bash→Python cutover must inventory EVERY live `.sh` consumer
(skills, the merged-into-targets AGENTS.md template, `tests/*/run.sh`, `ci.yml`,
`install.sh`, auto-executing `!` skill embeds) and freeze golden fixtures before
deleting the oracle.
<!-- /merge -->

## Open Questions

1. **Locale pin for parity.** Confirm the CI `LC_COLLATE` used to freeze golden
   glob-order fixtures, so `validate`/`list`/`scan` row order is stable.
2. **Symlink vs delete for root `spec/`/`flow/`** (product Open Q2) — resolve in
   /10; either way the multi-hop `cp -RL` materialization gets a test.
