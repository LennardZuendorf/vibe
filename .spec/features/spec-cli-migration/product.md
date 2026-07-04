---
type: feature-product
feature: spec-cli-migration
sibling: tech.md
parent: ../../product.md
updated: 2026-07-04
---

# Feature: spec-CLI migration — Product

Restructure the `vibe` CLI into a **3-app monorepo** and, as its core payload,
migrate the six spec-framework shell scripts (`validate`, `setup`, `list-specs`,
`lessons-for`, `promote`, `scan-merges`) into native Python. The split is by
**consumer + import tier**: two stdlib-only agent tools (`vibe-flow` for the
hooks + flow-management commands, `vibe-spec` for the former `.sh` scripts) and
one rich human tool (`vibe`) for setup, management, and pretty flow/spec output —
all sharing a zero-dependency `vibe-core` library. This finishes the flow port
(vibe-cli feature) and the spec port in one coherent package structure, keeping
each agent entry point dependency-free.

**Parent:** [../../product.md](../../product.md)
**Architecture:** [tech.md](tech.md)
**Plan:** [plan.md](plan.md)

---

## Scope

| | |
|---|---|
| **Owns** | The workspace restructure into `vibe-core` / `vibe-flow` / `vibe-spec` / `vibe-cli` packages; the three console_scripts (`vibe-flow`, `vibe-spec`, `vibe`); native Python for all six spec commands; the byte-parity suites vs the bash originals; the assets/source-of-truth reconciliation (packages become canonical, root dirs + `.agents` symlinks repoint); the dependency-minimization (stdlib agent tools, `pydantic` dropped) |
| **Does not own** | The `.spec/` document *format* and validation *rules* (owned by the `spec` skill — behavior is reproduced, not changed); `state-machine.json` / cursor *schema* (canonical data owned by vibe-flow; loaded, never hardcoded); the flow *behavior* already shipped by vibe-cli (this re-homes it into `vibe_flow`, no logic change) |

**Supersedes.** Re-homes the flow CLI shipped by [vibe-cli](../vibe-cli/product.md)
into the `vibe_flow` package, and moves ownership of `spec/scripts/*.sh` into
`vibe_spec` once R4 retirement fires. Until then the `.sh` scripts remain the
skill-context fallback and the parity oracle.

---

## Requirements

### Requirement: Three apps, split by consumer and import tier (R1)

The CLI SHALL ship as three console_scripts: `vibe-flow` (agent + hooks) and
`vibe-spec` (agent) MUST import stdlib only; `vibe` (human) MAY use `typer`/
`rich`. Each agent tool installed alone MUST pull zero third-party dependencies.

#### Scenario: Agent tool installs dependency-free

- **Given** a fresh environment
- **When** `vibe-spec` (or `vibe-flow`) is installed
- **Then** only it and `vibe-core` are pulled — no `typer`, `rich`, or `pydantic`

#### Scenario: Hot path stays stdlib

- **Given** the per-Edit guard, now `vibe-flow hook guard`
- **When** it fires during an implementation turn
- **Then** its import cost matches today's stdlib `vibe-hook`, not the rich `vibe`

### Requirement: Native parity for all six spec commands (R2)

`vibe-spec {validate,setup,list,lessons-for,promote,scan-merges}` SHALL each
produce output and exit codes byte-identical to the current bash script across a
fixture matrix, reimplemented in Python rather than shelling out.

#### Scenario: `vibe-spec validate` matches the script byte-for-byte

- **Given** a `.spec/` tree with a known mix of errors and warnings
- **When** `vibe-spec validate` runs and `bash spec/scripts/validate.sh` runs
- **Then** their stdout, stderr, and exit codes are identical, no `bash` spawned

### Requirement: One behavior source, two renderers (R3)

Command *behavior* SHALL live once in the stdlib packages; the human `vibe`
SHALL import that logic and apply a `rich` renderer, never fork the logic. The
byte-parity target is the stdlib app's plain output; `vibe`'s pretty output is
not parity-bound.

#### Scenario: Rich and plain share one implementation

- **Given** `vibe-spec validate` (plain) and `vibe spec validate` (rich)
- **When** both run on the same tree
- **Then** they call the same `vibe_spec.validate` logic; only rendering differs, and the plain output is the one pinned against `validate.sh`

### Requirement: Skill context stays runnable; retirement is gated (R4)

The migration MUST NOT break any context where the spec skill runs today. Python
is an accepted dependency; a `vibe doctor` Python/`PATH` preflight establishes
the guarantee. The `.sh` originals SHALL remain until that preflight is wired and
retirement is separately approved.

#### Scenario: Bash retained until preflight guarantee

- **Given** the native `vibe-spec` commands are landed and parity-green
- **When** no preflight guarantee + retirement approval exists yet
- **Then** `spec/scripts/*.sh` and the skill's bash invocations remain canonical

---

## Decisions

- **D1 — Monorepo, 4 packages, 3 apps (uv workspace).** `vibe-core` (stdlib
  lib) ← `vibe-flow` (stdlib app) ← / `vibe-spec` (stdlib app) ← / `vibe`
  (typer+rich app). Deps point only downward. Chosen over one package with
  groups because it makes "install only what you need" structural, not
  conventional, and keeps each agent tool provably dependency-free.
- **D2 — `vibe-flow` subsumes `vibe-hook`.** The hooks become `vibe-flow hook
  inject|guard|gate` alongside the agent flow verbs (`status`/`next`/`go`/
  `check`/`orders`) in one stdlib argparse app. `vibe-hook` is retired/aliased.
- **D3 — Stdlib apps use argparse; only `vibe` uses typer.** `typer` pulls
  `click`; to keep `vibe-flow`/`vibe-spec` dependency-free they dispatch with
  stdlib `argparse` (the current `hook.py` pattern). `rich` never imports on an
  agent path.
- **D4 — Logic + plain renderer in the stdlib packages; `vibe` re-renders (R3).**
  One behavior source; two presentations. Parity pins the plain output.
- **D5 — Minimal deps: drop `pydantic`.** `machine.py` already loads JSON with
  stdlib; config/display models move to stdlib `dataclasses`. `vibe`'s only
  third-party deps become `typer` + `rich`.
- **D6 — Packages become the asset source of truth.** Each package vendors the
  assets it owns (`vibe-flow`: `state-machine.json`; `vibe-spec`: templates);
  root `spec/`/`flow/` + `.agents/skills/*` symlinks repoint into the packages;
  the asset-sync test retargets. This is its own unit, not a side effect.

---

## Non-Goals

- Changing any validation rule, output wording, or `.spec/` format — behavior-
  preserving port.
- Rewriting `spec/SKILL.md` routing to call the CLI (gated by R4).
- Deleting the `.sh` scripts in this feature (gated by R4).
- Porting the spec skill's subagents (prompt assets, not scripts).

---

## Open Questions

1. **Assets source-of-truth cutover (OPEN).** Do the packages become canonical
   with root `spec/`/`flow/` + `.agents` symlinks pointing in, or stay mirrored?
   Recommendation: packages canonical (D6). This is the largest structural risk
   — the current `test_assets_sync.py` invariant must retarget, not break.
2. **Package/dist naming (OPEN).** Today's published dist is `vibe-flow`
  (the umbrella). Repurposing that name for the flow *sub-package* needs a PyPI
  plan; propose dist names `vibe-core`/`vibe-flow`/`vibe-spec`/`vibe`, with a
  deprecation note on the old umbrella. Confirm.
3. **Feature rename (OPEN, ask-first).** Scope has grown from "migrate spec
   scripts" to "restructure the CLI." Consider renaming this feature to
   `cli-restructure`; kept as `spec-cli-migration` for now to avoid churn.
4. **`validate.sh` fidelity (OPEN).** ~640 lines of `awk`/`grep`/`sed`
   (SF3–SF16); byte-parity is the bar; SF4 network lint stays a bash shell-out.
