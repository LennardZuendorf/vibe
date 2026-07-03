---
type: feature-product
feature: vibe-cli
sibling: tech.md
parent: ../../product.md
updated: 2026-07-03
---

# Feature: vibe CLI — Product

Fold the flow harness into one installable command. Today setup is three
disjoint mechanisms — `install.sh` copies files, a manual `/plugin` step
registers hooks, and an in-agent "set up vibe" writes the rest — and none
reference the others, so the "flow fires every turn" promise stays dark until
an undiscoverable GUI step happens. The `vibe` CLI supersedes all of that: it
*is* the state machine, and it makes project setup a single command. Built with
Python (typer + rich) and distributed via `uv tool install` (mirroring
[LennardZuendorf/indexed](https://github.com/LennardZuendorf/indexed)), it
replaces the bash flow scripts and `install.sh`, and registers hooks that fire
with no `/plugin` dance.

**Parent:** [../../product.md](../../product.md)
**Architecture:** [tech.md](tech.md)
**Plan:** [plan.md](plan.md)

---

## Personas

- **Developer-architect** — me (root [Target User](../../product.md)): shaping a
  portable personal workflow, wanting `where am I / what's next` and safe
  transitions without hand-reading state.
- **Stranger / adopter** — a fresh install target from root
  [release-docs](../release-docs/product.md): must go from zero to a working,
  hook-firing project in one command, and leave cleanly if they choose.

---

## Scope

| | |
|---|---|
| **Owns** | The `vibe` / `vibe-hook` CLI package; the ported flow logic (state-machine load, cursor transitions, policy oracle + the 3 hard blocks, orders/D12 resolution, active-rules regen, doctor, `AGENTS.md` marker-merge, install/uninstall/update); the three hooks delivered via `.claude/settings.json`; dependency-plugin orchestration via `claude plugin` |
| **Does not own** | The spec skill internals (the CLI wraps `validate.sh`/`setup.sh`, it does not rewrite them); Claude Code itself; MCP wiring; the `state-machine.json` and cursor *schema* (stay canonical data owned by vibe-flow; the CLI loads them, never hardcodes) |

**Supersedes (bash mechanisms).** This feature is the live implementation of flow
machinery that four sibling specs still describe as bash; ownership of each moves here:

- **vibe-flow** — the flow scripts (`set-state`, `detect-context`, `orders`,
  `regen-active-rules`, `doctor`). `state-machine.json` and the cursor *schema* stay
  canonical **data owned by vibe-flow**; the CLI owns only the engine that loads and
  transitions over them.
- **install-tooling** — `install.sh`, `doctor.sh`, `deps.json`.
- **platform-adapters** — the hook wiring and plugin manifest.
- **agent-instructions** — `merge-agents.sh` (the `AGENTS.md` marker-merge).

The **spec framework** (`validate.sh` / `setup.sh`) is *not* superseded — the CLI wraps
it and it stays owned by the spec skill. The siblings' specs still document the retired
bash by design; this feature is where those mechanisms now live.

---

## Requirements

### Requirement: One-command project setup (R1)

`vibe init [PATH]` SHALL provision a project in a single command: copy the spec
and flow skills, register the three flow hooks, merge the managed `AGENTS.md`
instruction block, seed the flow cursor and gitignore it, and offer to install
the dependency plugins (superpowers, feature-dev, caveman). No `git clone`, no
manual `/plugin`, no separate in-agent "set up vibe" step.

#### Scenario: Fresh target fully provisioned

- **Given** a clean project directory with no vibe artifacts
- **When** `vibe init --yes` runs
- **Then** the target has the spec + flow skills, three registered hooks, a managed `AGENTS.md` block, a seeded gitignored cursor, and the dependency plugins installed — and a "what happened / your next step" summary is printed

#### Scenario: Hooks fire next session with no registration step

- **Given** a project where `vibe init` has completed
- **When** the next agent session starts and the user submits a prompt or edits a file
- **Then** the inject, guard, and gate hooks fire immediately, with no `/plugin` GUI step ever performed

### Requirement: Orientation and legal transitions (R2)

The CLI SHALL answer "where am I and what may I do next" and SHALL move the
cursor only through legal transitions. `vibe status` shows the current
flow/phase/feature plus legal next states; `vibe next` lists those states;
`vibe go <state>` transitions only when the state machine permits it.

#### Scenario: Status shows cursor and legal next

- **Given** a project mid-flow at `feature.impl`
- **When** `vibe status` runs
- **Then** it shows flow/phase/feature and the legal next states as a panel, without hand-reading the cursor file

#### Scenario: Illegal transition is refused

- **Given** the cursor at `feature.design`
- **When** `vibe go feature.verify` runs (not a legal next)
- **Then** the transition is rejected with the legal options named, and the cursor is unchanged

### Requirement: Health report with CI gate (R3)

`vibe doctor` SHALL report install health in one run — skills, hooks, cursor
validity, `AGENTS.md` block, dependency plugins — as a rich report, each
degraded check paired with a fix hint. The default run is warn-only and never
blocks; `--exit-code` returns nonzero on any failure for CI.

#### Scenario: Degraded dependency warns with a fix hint, CI can gate

- **Given** a project with a missing dependency plugin
- **When** `vibe doctor` runs, then `vibe doctor --exit-code` runs
- **Then** the default run prints a warn line naming the dep, its degrade consequence, and the fix command, exit 0; and `--exit-code` returns nonzero

### Requirement: Surgical, cursor-safe lifecycle (R4)

`vibe uninstall [PATH]` SHALL remove only the artifacts init created (per-file
inverse), preserving `.spec/**`, user-authored `AGENTS.md` prose, and the flow
cursor. `vibe update [PATH]` SHALL re-provision the managed files idempotently,
preserving the live cursor and user prose.

#### Scenario: Uninstall preserves user content

- **Given** an installed target with user edits outside the managed markers, a `.spec/` tree, and a live cursor
- **When** `vibe uninstall` runs
- **Then** the managed skills, hooks, and block are gone while `.spec/**`, the user prose, and (absent explicit confirm) the cursor remain

#### Scenario: Update preserves a live cursor

- **Given** a target at `feature.impl <feature>`
- **When** `vibe update` re-provisions the managed files
- **Then** the managed files are refreshed and the cursor still reads `feature.impl <feature>`, user prose untouched

### Requirement: Spec framework wrapped, not rewritten (R5)

The CLI SHALL front the spec half without reimplementing it: `vibe spec
validate` and `vibe spec setup` produce the same result as the spec framework
run on its own, and the spec commands work without the flow runtime installed.
The spec framework stays the standalone, any-agent half.

#### Scenario: Wrapped validate matches standalone, needs no flow runtime

- **Given** a project with the spec framework present
- **When** `vibe spec validate` runs
- **Then** its result equals running the spec framework directly, and it succeeds even where the flow cursor and hooks are absent

### Requirement: Runtime safety — fast guard, persistent install (R6)

The per-edit guard SHALL stay near current latency so implementation turns feel
no lag, and `vibe init` SHALL detect an ephemeral (non-persistent) invocation
and warn that hooks will not persist rather than silently no-op.

#### Scenario: Guard adds no perceptible lag

- **Given** an implementation turn with many edits
- **When** each edit triggers the write-policy guard
- **Then** the guard verdict returns fast enough to be imperceptible, matching the prior bash guard's feel

#### Scenario: Ephemeral run warns instead of silently failing

- **Given** `vibe init` invoked so that `vibe`/`vibe-hook` will not resolve on PATH afterward
- **When** init runs its prerequisite check
- **Then** it warns that the hooks would not fire and points to the persistent install, rather than provisioning hooks that silently no-op

---

## Decisions

- **D1 — Full flow port (Approach A).** The CLI owns the flow; the bash flow
  scripts become vestigial. Chosen over a hybrid front-door or a thin wrapper,
  both of which leave two sources of truth — the exact friction being removed.
- **D2 — Hooks via `.claude/settings.json`.** The three hooks register in
  settings.json and fire immediately; the plugin manifest is kept only as an
  optional secondary channel. This removes the manual `/plugin` step.
- **D3 — Runtime shift accepted.** The flow half now requires a Python install
  (`uv tool install vibe-flow`), trading the pure-bash property for
  one-command setup. Mitigated: the spec half stays bash and standalone, and
  the hook hot-path stays dependency-light.
- **D4 — Legacy retired with a migration note.** The bash flow scripts and
  `install.sh` are superseded by this CLI and retired; a migration note points
  existing installs at `vibe init` / `vibe update`. The Scope **Supersedes** note
  lists which sibling mechanisms move here.
- **D5 — Names.** The command stays `vibe` (plus `vibe-hook` for the hook
  hot-path); the distributed package is `vibe-flow`.

---

## Non-Goals

- Rewriting the spec skill — the CLI wraps `validate.sh` / `setup.sh`, it does
  not replace them.
- MCP wiring (writing a project `.mcp.json`) — out of scope for v1.
- Windows support — the Python hook improves portability, but v1 targets POSIX.
- Auto-trusting the workspace — the one-time "trust this folder?" dialog stays
  manual.
