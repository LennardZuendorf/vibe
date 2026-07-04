---
type: feature-product
feature: cli-restructure
sibling: tech.md
parent: ../../product.md
updated: 2026-07-04
---

# Feature: CLI restructure — Product

Restructure the `vibe` CLI into a **3-app monorepo** and, as its core payload,
migrate the six spec-framework shell scripts (`validate`, `setup`, `list-specs`,
`lessons-for`, `promote`, `scan-merges`) into native Python. Split by **consumer
+ import tier**: two stdlib-only agent tools — `vibe-flow` (the hooks + flow-
management commands) and `vibe-spec` (the former `.sh` scripts) — and one rich
human tool (`vibe`) for setup, management, and pretty flow/spec output, all
sharing a lean `vibe-core` library. This finishes the flow port (from
[vibe-cli](../vibe-cli/product.md)) and the spec port in one structure, keeps
each agent entry point lean (few deps, stdlib hot path), and does a **hard
cutoff** to Python:
packages become the single source of truth and the bash scripts are removed.

**Parent:** [../../product.md](../../product.md)
**Architecture:** [tech.md](tech.md)
**Plan:** [plan.md](plan.md)

---

## Scope

| | |
|---|---|
| **Owns** | The workspace restructure into `vibe-core` / `vibe-flow` / `vibe-spec` / `vibe-cli`; the three console_scripts; native Python for all six spec commands + the byte-parity suites; the **hard-cutoff** asset move (packages canonical, root `spec/`/`flow/` + `.agents/skills/*` repointed, bash scripts removed, `spec/SKILL.md` repointed at `vibe-spec`); dependency minimization (lean agent tools, stdlib hot path, `pydantic` dropped); the GitHub + install-script distribution |
| **Does not own** | The `.spec/` document *format* and validation *rules* (behavior reproduced, not changed); `state-machine.json` / cursor *schema* (canonical data owned by vibe-flow; loaded, never hardcoded); the flow *behavior* already shipped by vibe-cli (re-homed into `vibe_flow`, no logic change) |

**Supersedes.** Re-homes the flow CLI from [vibe-cli](../vibe-cli/product.md) into
`vibe_flow`, and moves ownership of `spec/scripts/*.sh` into `vibe_spec` — the
`.sh` originals are removed at cutover (R4), not kept as a permanent fallback.

---

## Requirements

### Requirement: Three apps, minimal deps, fast hot path (R1)

The CLI SHALL ship as three console_scripts split by consumer, keeping
dependencies as few as possible: `vibe` (human) uses `typer` + `rich`; the agent
tools (`vibe-flow`, `vibe-spec`) default to stdlib `argparse` and add a third-
party dependency only where it clearly earns its place. The per-Edit hot path
(`vibe-flow hook`) MUST stay stdlib-only, so its import latency matches today's
`vibe-hook` — a separate constraint from dep-count.

#### Scenario: Agent tool stays lean

- **Given** a fresh environment
- **When** `vibe-spec` (or `vibe-flow`) is installed
- **Then** it pulls `vibe-core` plus at most a small, justified dependency — never `typer`/`rich`/`pydantic` transitively

#### Scenario: Hot path stays stdlib (latency, not dep-count)

- **Given** the per-Edit guard, now `vibe-flow hook guard`
- **When** it fires during an implementation turn
- **Then** it imports stdlib only, so its cost matches today's `vibe-hook`, not the rich `vibe`

### Requirement: Native parity for all six spec commands (R2)

`vibe-spec {validate,setup,list,lessons-for,promote,scan-merges}` SHALL each
produce output and exit codes byte-identical to the current bash script across a
fixture matrix, reimplemented in Python rather than shelling out.

#### Scenario: `vibe-spec validate` matches the script byte-for-byte

- **Given** a `.spec/` tree with a known mix of errors and warnings
- **When** `vibe-spec validate` runs and `bash spec/scripts/validate.sh` runs (in CI)
- **Then** their stdout, stderr, and exit codes are identical, no `bash` spawned at runtime

### Requirement: One behavior source, two renderers (R3)

Command *behavior* SHALL live once in the stdlib packages; the human `vibe`
SHALL import that logic and apply a `rich` renderer, never fork it. The byte-
parity target is the stdlib app's plain output; `vibe`'s pretty output is not
parity-bound.

#### Scenario: Rich and plain share one implementation

- **Given** `vibe-spec validate` (plain) and `vibe spec validate` (rich)
- **When** both run on the same tree
- **Then** they call the same `vibe_spec.validate` logic; only rendering differs, and the plain output is the one pinned against `validate.sh`

### Requirement: Hard cutoff to Python, guarded by preflight (R4)

At cutover the `.sh` scripts SHALL be removed, `spec/SKILL.md` repointed at
`vibe-spec`, and the packages made the single asset source of truth — all in one
unit, no mirrored transition. Python is an accepted dependency; the skill context
is kept safe by a `vibe doctor` / `vibe-spec`-self-check preflight and the
install script placing the three entry points on `PATH`.

#### Scenario: Cutover is atomic and parity-gated

- **Given** every `vibe-spec` command is byte-parity-green vs its `.sh` origin
- **When** the cutover unit lands
- **Then** the `.sh` scripts are gone, `spec/SKILL.md` invokes `vibe-spec`, the asset-sync test targets the packages, and no state exists where bundled ≠ source

#### Scenario: Preflight guards a Python-less context

- **Given** a context where `vibe-spec` is not on `PATH`
- **When** the spec skill runs `vibe doctor` (or the command's self-check)
- **Then** it reports the missing tool + the install command, rather than failing obscurely

---

## Decisions

- **D1 — Monorepo, 4 packages, 3 apps (uv workspace).** `vibe-core` (stdlib lib)
  ← `vibe-flow` / `vibe-spec` (stdlib apps) ← `vibe` (typer+rich app). Deps point
  only downward. Makes "install only what you need" structural, not conventional.
- **D2 — `vibe-flow` subsumes `vibe-hook`.** Hooks become `vibe-flow hook
  inject|guard|gate` alongside the agent flow verbs; `vibe-hook` is retired.
- **D3 — argparse by default on agent paths; typer only in `vibe`.** `argparse`
  is the default for `vibe-flow`/`vibe-spec` because it adds nothing to install;
  a light dep is allowed if it earns its place. The binding rule is latency, not
  dep-count: the `vibe-flow hook` path MUST stay stdlib-only, pinned by a fresh-
  interpreter import-cost test (no `typer`/`rich`/`pydantic`).
- **D4 — Logic + plain renderer in the stdlib packages; `vibe` re-renders (R3).**
- **D5 — As few deps as possible.** Default to stdlib (`json` + `dataclasses`);
  drop `pydantic`. `vibe`'s third-party deps are `typer` + `rich`; the agent
  tools stay at zero-or-few. Add a dependency only when it clearly beats stdlib.
- **D6 — Hard cutoff.** Packages become the asset source of truth and the `.sh`
  scripts are removed at cutover — no mirror window, no permanent bash fallback.
  Bash stays only as the *CI parity oracle* during the port.
- **D7 — Distribute via GitHub + install script, not PyPI.** Mirroring GSD's
  installer model: an install script (or `uv tool install` from the GH checkout)
  puts `vibe` / `vibe-flow` / `vibe-spec` on `PATH`; `vibe init` still provisions
  the project. Dist/package *names* are internal, so naming is not a blocker.

---

## Non-Goals

- Changing any validation rule, output wording, or `.spec/` format — behavior-
  preserving port.
- A PyPI release (deferred; D7).
- Porting the spec skill's subagents (prompt assets, not scripts).

---

## Open Questions

1. **`validate.sh` fidelity (OPEN).** ~640 lines of `awk`/`grep`/`sed`
   (SF3–SF16); byte-parity is the bar and the largest risk. The optional SF4
   network lint stays a bash shell-out (or is dropped with a documented
   divergence). Confirm byte-parity — not "equivalent" — is required.
2. **Install-script shape (OPEN).** Reuse/replace the existing root `install.sh`
   as the GH bootstrap (clone/download → `uv tool install` the three tools →
   `vibe init`), matching GSD's "installer required, don't copy files" model.
