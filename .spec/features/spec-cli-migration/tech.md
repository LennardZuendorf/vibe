---
type: feature-tech
feature: spec-cli-migration
sibling: product.md
parent: ../../tech.md
updated: 2026-07-04
---

# Feature: spec-CLI migration — Architecture

Restructure `cli/` into a `uv` workspace of four packages — a zero-dep
`vibe-core` library, two stdlib-only agent apps (`vibe-flow`, `vibe-spec`), and
the rich human app (`vibe`) — with three console_scripts. Flow logic re-homes
from the current flat package into `vibe_flow` unchanged; the six spec scripts
port into `vibe_spec` behind a byte-parity gate. Deps point only downward, so
each agent tool installs dependency-free.

**Parent:** [../../tech.md](../../tech.md)
**Requirements:** [product.md](product.md)
**Plan:** [plan.md](plan.md)

---

## Package graph

```
vibe-core   (stdlib)  ── markers, errors, paths, asset load
   ▲   ▲   ▲
   │   │   └───────────── vibe-spec (stdlib)  ── app: vibe-spec
   │   └───────────────── vibe-flow (stdlib)  ── app: vibe-flow  (subsumes vibe-hook)
   └── vibe-cli (typer+rich, deps: core+flow+spec) ── app: vibe
```

| Dist | Import pkg | Third-party deps | console_script | Consumer |
|---|---|---|---|---|
| `vibe-core` | `vibe_core` | — | — (library) | shared |
| `vibe-flow` | `vibe_flow` | — | `vibe-flow` | agent + hooks |
| `vibe-spec` | `vibe_spec` | — | `vibe-spec` | agent |
| `vibe` | `vibe_cli` | `typer`, `rich` | `vibe` | human |

`vibe-flow`/`vibe-spec` installed alone pull only `vibe-core` (R1). `pydantic` is
dropped everywhere (D5) — `machine.py` already parses JSON with stdlib; models
become `dataclasses`.

---

## Workspace layout

```
cli/
├── pyproject.toml                     # [tool.uv.workspace] members = ["packages/*"]
├── uv.lock                            # single lock across the workspace
├── conftest.py / tests/               # shared fixtures (bash_ref, home_sandbox, target_project)
└── packages/
    ├── vibe-core/
    │   ├── pyproject.toml             # name=vibe-core; no deps
    │   └── src/vibe_core/
    │       ├── assets.py              # locate vendored data; load json (stdlib)
    │       ├── markers.py             # strict marker pairing/reversal (shared by flow+spec)
    │       ├── errors.py              # VibeError(Exception)
    │       └── paths.py               # root-find (.spec/.git), atomic write
    ├── vibe-flow/
    │   ├── pyproject.toml             # name=vibe-flow; deps=[vibe-core]
    │   │                              # [project.scripts] vibe-flow = "vibe_flow.app:main"
    │   └── src/vibe_flow/
    │       ├── machine.py cursor.py policy.py orders.py   # re-homed from current cli, unchanged
    │       ├── hook.py                # inject|guard|gate  (was vibe.hook)
    │       ├── render.py              # plain/ANSI renderer for the flow verbs
    │       ├── app.py                 # argparse: vibe-flow {hook,status,next,go,check,orders}
    │       └── _assets/               # state-machine.json, state.example.json
    ├── vibe-spec/
    │   ├── pyproject.toml             # name=vibe-spec; deps=[vibe-core]
    │   │                              # [project.scripts] vibe-spec = "vibe_spec.app:main"
    │   └── src/vibe_spec/
    │       ├── model.py               # frontmatter/area/type parse
    │       ├── validate.py setup.py listing.py lessons.py merges.py   # the six, ported
    │       ├── render.py              # plain/ANSI renderer (byte-parity target)
    │       ├── app.py                 # argparse: vibe-spec {validate,setup,list,lessons-for,promote,scan-merges}
    │       └── _assets/               # reference/templates/**
    └── vibe-cli/
        ├── pyproject.toml             # name=vibe; deps=[vibe-core,vibe-flow,vibe-spec,typer,rich]
        │                              # [project.scripts] vibe = "vibe_cli.app:main"
        └── src/vibe_cli/
            ├── app.py                 # typer; top-level init/doctor/update/uninstall/plugins/setup
            │                          #   + rich-rendered status/next/go/check/orders + `spec` group
            ├── provision/             # settings.py agents_md.py plugins.py  (init/uninstall/update)
            ├── doctor.py rules.py     # + D5 python/PATH preflight check
            └── ui/                    # console, theme, banner
```

---

## Contract / API

Behavior is a pure function per command; a renderer turns it into text. The
stdlib app calls the plain renderer, `vibe_cli` calls a rich one — R3.

```python
# vibe_spec/validate.py
def run(root: Path) -> ValidationReport: ...          # dataclass: errors, warnings, exit_code
# vibe_spec/render.py
def render_plain(report: ValidationReport) -> str: ...  # byte-identical to validate.sh
# vibe_cli/app.py
report = vibe_spec.validate.run(root); console.print(rich_view(report))

# vibe_flow/app.py  (argparse)
#   vibe-flow hook {inject|guard|gate}   ← settings.json hooks target this
#   vibe-flow status|next|go|check|orders
```

`settings.json` hook entries change from `vibe-hook <event>` to `vibe-flow hook
<event>`; `provision/settings.py` writes the new command (idempotent, keyed).

---

## Implementation Detail

- **Flow re-home is a move, not a rewrite.** `machine`/`cursor`/`policy`/
  `orders`/`hook` already exist and are stdlib-only in the current `cli/src/vibe`.
  Unit work is relocating them into `vibe_flow` (+ `vibe_core` for the shared
  bits) and adding the argparse `app.py` + plain flow-verb renderer. The existing
  flow parity tests (`test_parity_policy`, `test_parity_orders`) move with them.
- **Spec port is the real work.** `vibe_spec` reimplements the six scripts
  stdlib-only. Byte-parity is the bar — ANSI codes, ` ERROR:`/` WARN:`/` OK:`
  prefixes, the `Errors:`/`Warnings:` footer, setup's writing-order block, and
  the `--format` json/plain/inject/table variants all pinned against the `.sh`.
  Env knobs (`VIBE_DESIGN_LINT`, `SPEC_DIR`, `SPEC_ROOT_MAX_LINES`) honored
  identically; SF4 network lint stays a bash shell-out.
- **`validate.sh` is the hard unit** — ~640 lines of `awk`, ported check-by-check
  against the `tests/spec/run.sh` fixtures, diffed continuously.
- **Parity harness reuse.** The `bash_ref` conftest fixture (locates repo `*.sh`,
  skips without `bash`/`jq`) moves to the workspace root `conftest.py`; each
  `vibe-spec` command gets a parity test vs its origin, mirroring the flow parity
  tests.
- **Argparse, not typer, on agent paths (D3).** `vibe_flow.app`/`vibe_spec.app`
  dispatch subcommands with `argparse`; an import-cost test (fresh interpreter,
  `sys.modules` assertion — the standing lesson) pins that neither imports
  `typer`/`rich`/`pydantic`.

<!-- merge -->
CLI packaging pattern for this repo: a `uv` workspace with a zero-dep `vibe-core`
library and per-consumer apps — stdlib-only `argparse` entry points for anything
an agent or hook invokes, a single `typer`/`rich` app for humans. Split packages
by *who calls it and what it may import*, never by domain tidiness; a new app
earns its keep only when it's a distinct import tier. Behavior lives once in the
stdlib packages; the rich app re-renders. Freeze all package `pyproject` deps and
the workspace lock in the skeleton unit so downstream builders never touch them.
<!-- /merge -->

## Open Questions

1. **Asset cutover order (OPEN).** Sequence the D6 source-of-truth move so
   `test_assets_sync.py` retargets in the same unit that repoints the symlinks —
   never a window where bundled ≠ source silently.
2. **`validate.sh` glob/color fidelity (OPEN).** Reproduce the exact per-file
   header/finding interleaving and `.spec/*.md` glob order for byte-parity, or
   normalize both sides.
