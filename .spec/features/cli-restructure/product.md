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
+ import tier**: two lean agent tools — `vibe-flow` (the hooks + flow-management
commands) and `vibe-spec` (the former `.sh` scripts) — and one rich human tool
(`vibe`) for setup, management, and pretty flow/spec output, all sharing a lean
`vibe-core` library. This finishes the flow port (from
[vibe-cli](../vibe-cli/product.md)) and the spec port in one structure, keeps
each agent entry point lean (few deps, stdlib hot path), and does a **hard
cutoff to Python for BOTH halves**: the packages become the single source of
truth, and every runtime `.sh` (flow *and* spec) plus the plugin bash-shim hooks
are removed, guarded by an install-script preflight.

**Parent:** [../../product.md](../../product.md)
**Architecture:** [tech.md](tech.md)
**Plan:** [plan.md](plan.md)

---

## Scope

| | |
|---|---|
| **Owns** | The workspace restructure into `vibe-core` / `vibe-flow` / `vibe-spec` / `vibe-cli`; the three console_scripts; native Python for all six spec commands + the byte-parity suites + frozen golden fixtures; the **hard cutoff** (packages canonical; remove flow *and* spec `.sh`; retire the plugin bash-shim hooks; repoint `spec/SKILL.md` + `flow/SKILL.md` + all skill/template/AGENTS.md prose at the binaries; rewrite `ci.yml`; port/retire `tests/spec/run.sh`); the in-place `vibe-hook`→`vibe-flow` migration; the GitHub install-script distribution; dependency minimization (`pydantic` dropped); doc/CHANGELOG updates for the new command + install |
| **Does not own** | The `.spec/` document *format* and validation *rules* (behavior reproduced, not changed, save the documented divergences below); `state-machine.json` / cursor *schema* (canonical data owned by vibe-flow; loaded, never hardcoded); the flow *behavior* already shipped by vibe-cli (re-homed into `vibe_flow`, no logic change) |

**Supersedes.** Re-homes the flow CLI + flow bash engine from
[vibe-cli](../vibe-cli/product.md)/vibe-flow into `vibe_flow`; moves
`spec/scripts/*.sh` into `vibe_spec`; reworks
[install-tooling](../install-tooling/product.md)'s `install.sh` into the GitHub
bootstrap; and retires [platform-adapters](../platform-adapters/product.md)'s
plugin bash-shim hooks in favour of settings.json → `vibe-flow hook`. All `.sh`
are removed at cutover, not kept as a permanent fallback.

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
produce output and exit codes byte-identical to the current bash script over its
*deterministic* content, reimplemented in Python rather than shelling out.
Behavior-defining bash quirks MUST be reproduced (per-script `SPEC_DIR` handling,
unescaped hand-rolled JSON, regex-as-data matching, `LC_COLLATE` glob order,
byte-slice previews, `wc -l` line counts). Documented divergences are limited to:
the SF4 network lint (dropped — see D9), and `setup`'s trailing absolute-path /
`bash validate.sh` hint lines (rewritten for the new layout — excluded from the
parity comparison).

#### Scenario: `vibe-spec validate` matches the script byte-for-byte

- **Given** a `.spec/` tree with a known mix of errors and warnings, `VIBE_DESIGN_LINT` unset
- **When** `vibe-spec validate` runs and `bash spec/scripts/validate.sh` runs (in CI, pre-cutover)
- **Then** their stdout, stderr, and exit codes are identical, no `bash` spawned at runtime

#### Scenario: A frozen golden fixture survives the cutover

- **Given** captured golden stdout/stderr/exit per command, frozen before the `.sh` are deleted
- **When** a post-cutover regression test runs
- **Then** it still has a reference to diff against, even though the bash origin is gone

### Requirement: One behavior source, two renderers (R3)

Command *behavior* SHALL live once in the stdlib packages, returning a structured
result (including any warnings and, for mutating verbs, old→new state); the human
`vibe` SHALL import that logic and apply a `rich` renderer, never fork it. The
byte-parity target is the stdlib app's plain output; `vibe`'s pretty output is not
parity-bound.

#### Scenario: A mutating verb keeps logic and rendering separate

- **Given** `vibe-flow go <state>` (plain) and `vibe go <state>` (rich)
- **When** both run
- **Then** the mutation + legality check + any "no feature set" warning are computed once in `vibe_flow` and returned as a result object; each app only formats it

### Requirement: Hard cutoff to Python, guarded by preflight (R4)

At cutover the flow *and* spec `.sh` scripts SHALL be removed, the plugin bash-
shim hooks retired, `spec/SKILL.md` + `flow/SKILL.md` + every skill/template/
`AGENTS.md`-template reference repointed at the binaries, `ci.yml` +
`tests/spec/run.sh` migrated off bash, and the packages made the single asset
source of truth — after golden fixtures are frozen (R2) and every command is
parity-green. The auto-executing `!` embed in `spec/SKILL.md` MUST degrade to a
documented one-liner (not a raw shell error) when the binary is absent, and a
`vibe doctor` / self-check preflight MUST report a missing tool + its install
command.

#### Scenario: Cutover leaves no live `.sh` reference

- **Given** the cutover unit has landed
- **When** the repo is grepped for live `.sh` invocations across skills, hooks, `tests/`, `.github/`, `install.sh`, and the `AGENTS.md` template
- **Then** none remain; the asset-sync test targets the packages; no state exists where bundled ≠ source

#### Scenario: Missing binary degrades, not crashes

- **Given** a context where `vibe-spec` is not on `PATH`
- **When** the spec skill loads (auto-running its embed) or the agent runs `vibe doctor`
- **Then** it emits a documented "install vibe-spec" line, not an obscure failure

### Requirement: Installable from GitHub; in-place update migrates cleanly (R5)

Distribution SHALL be a GitHub install script (no PyPI) that places `vibe` /
`vibe-flow` / `vibe-spec` on `PATH` from a checkout, removing any prior combined
`vibe-flow` install first. `vibe update` on a target provisioned by the *previous*
CLI SHALL migrate it — stripping the stale `vibe-hook` hook entries before adding
`vibe-flow hook` — so no hook double-fires and `doctor` stays consistent.

#### Scenario: Update migrates a legacy install without double-firing

- **Given** a target whose `.claude/settings.json` has the old `vibe-hook inject|guard|gate` entries
- **When** `vibe update` runs
- **Then** the result has exactly the three `vibe-flow hook …` entries and zero stale `vibe-hook` entries, and `doctor` reports them healthy

#### Scenario: Install script puts three tools on PATH

- **Given** a fresh checkout and no prior install
- **When** the install script runs
- **Then** `vibe`, `vibe-flow`, and `vibe-spec` resolve on `PATH` (workspace path-deps resolved), and `vibe init` provisions a project

---

## Decisions

- **D1 — Monorepo, 4 packages, 3 apps (uv workspace).** `vibe-core` (stdlib lib)
  ← `vibe-flow` / `vibe-spec` (stdlib apps) ← `vibe` (typer+rich app). Deps point
  only downward. Makes "install only what you need" structural, not conventional.
- **D2 — `vibe-flow` subsumes `vibe-hook`; settings.json is the sole hook path.**
  Hooks become `vibe-flow hook inject|guard|gate`; the plugin bash-shim hook
  adapter is retired (settings.json was already the primary channel).
- **D3 — argparse by default on agent paths; typer only in `vibe`.** `argparse`
  adds nothing to install; a light dep is allowed if it earns its place. The
  binding rule is latency: the `vibe-flow hook` path MUST stay stdlib-only,
  pinned by a fresh-interpreter import-cost test (no `typer`/`rich`/`pydantic`).
- **D4 — Logic + plain renderer in the stdlib packages; `vibe` re-renders (R3).**
  Mutating/multi-message verbs return a result dataclass (old→new + warnings[]) so
  the split stays clean, not just the pure ones like `validate`.
- **D5 — Drop `pydantic` (a phantom dep).** Nothing imports it today; `machine.py`
  already parses JSON with stdlib and `cursor.py`/`doctor.py` use `dataclasses`.
  Removal is deleting an unused direct dep, not a model migration.
- **D6 — Hard cutoff, both halves.** Packages become canonical; flow *and* spec
  `.sh` are removed; plugin bash-shim hooks retired. Golden fixtures are frozen
  first so a reference survives; bash stays only as the CI parity oracle during
  the port, then goes.
- **D7 — Distribute via GitHub install script, not PyPI.** Reworks
  install-tooling's `install.sh` into a bootstrap (`uv tool install` the three
  tools from the checkout, removing the old combined install). One lockstep
  version across the four `pyproject`s + `plugin.json`; names are internal.
- **D8 — `markers.py` stays in `vibe-core` for CLI provisioning; `merges.py` is
  net-new.** `markers.py` is used by `vibe_cli` provisioning (not flow/spec
  logic). The spec merge scanner uses a *different* grammar (bare
  `<!-- merge -->`) with multi-block + nesting detection — an independent module,
  NOT a reuse of the marker-pairing guard.
- **D9 — SF4 network lint dropped.** The advisory `npx @google/design.md` lint is
  nondeterministic (network) and can't be a bash shell-out under an all-bash
  cutoff; it is removed with a documented divergence (advisory only).

---

## Non-Goals

- Changing any validation rule or `.spec/` format — behavior-preserving port
  (save the documented D9 / setup-hint divergences).
- A PyPI release (deferred; D7).
- Porting the spec skill's subagents (prompt assets, not scripts).

---

## Open Questions

All resolved (2026-07-04):

1. **~~glob/collation fidelity~~ RESOLVED.** Capture golden fixtures under
   `LC_ALL=C` and reproduce **byte-order** sort in Python — deterministic across
   machines, no locale data required. Pinned in CI (tech OQ1).
2. **~~Root `spec/`/`flow/` post-cutover~~ RESOLVED: delete + repoint.** Remove the
   root `spec/`/`flow/` trees entirely; the package `_assets/` are the only source,
   and every `.agents/skills/*` symlink + `install.sh` copy + doc reference is
   repointed to resolve through the installed package (no symlink chain). Scoped in
   /10.
3. **~~Versioning~~ RESOLVED.** Single lockstep version across the four
   `pyproject`s + `plugin.json` (D7); frozen in /1.
4. **~~Root-plan registration~~ RESOLVED: registered (2026-07-04).** With owner
   approval, `cli-restructure` is entered as feature 10 (PLANNED) in the root
   `.spec/plan.md` Feature Sequence + Feature-plans table.
