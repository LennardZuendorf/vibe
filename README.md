<div align="center">

<img src="docs/img/logo.svg" alt="vibe" width="360">

**A self-hosting spec + workflow harness for coding with agents.**

[![CI](https://github.com/LennardZuendorf/vibe/actions/workflows/ci.yml/badge.svg)](https://github.com/LennardZuendorf/vibe/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
![bash](https://img.shields.io/badge/built%20with-bash%20%2B%20markdown%20%2B%20json-informational)

</div>

---

vibe is two things that ship together but stand alone:

- **The spec framework** — a durable `.spec/` planning layer (product / tech /
  design / plan / lessons) with templates and a validator. Works with **any**
  agent, or none.
- **The vibe flow** — a state-machine workflow for **Claude Code** that routes
  each phase (strategy / feature / quick) to the right skills and subagents,
  injects per-turn "orders", and guards its own write invariants with hooks.

Everything is bash, Markdown, and JSON — no runtime, no build step. The repo
builds itself with its own harness (it is self-hosting), so what you install is
exactly what is dogfooded here.

## Which half do you want?

| You want… | Install | Needs Claude Code? |
|---|---|---|
| **Durable, validated planning docs** for a project (any agent, or solo) | `./install.sh <repo> --only spec` | No |
| **The full workflow harness** — flow state machine, per-turn routing, hooks | `./install.sh <repo>` (both halves) | Yes, for the hooks |
| **Just the flow engine** without the spec skill | `./install.sh <repo> --only flow` | Yes |

New here? Read the first two screens and you will know which command to run.

## Install

**Prerequisites:** `bash` and [`jq`](https://jqlang.github.io/jq/) (the scripts
degrade gracefully without `jq`, but it is recommended). `git` for the target repo.

```bash
git clone https://github.com/LennardZuendorf/vibe.git
cd vibe

# Full install into your project (spec framework + flow + Claude adapter):
./install.sh /path/to/your/repo

# Preview first — prints the plan, writes nothing:
./install.sh /path/to/your/repo --dry-run

# Spec framework only (no flow, no hooks):
./install.sh /path/to/your/repo --only spec

# Also symlink CLAUDE.md -> AGENTS.md (opt-in, never clobbers a real file):
./install.sh /path/to/your/repo --adapters claude
```

A full install **copies** the platform-neutral core into `<repo>/.agents/skills/`,
**merges** `AGENTS.md` inside managed markers (your prose is never touched),
seeds and gitignores the flow cursor, and writes the Claude hook wiring into
`.claude/settings.json`. Re-running is idempotent and preserves a live flow cursor.
(`--only spec` copies just the spec skill — no `AGENTS.md` merge or cursor.)

**Claude Code hooks** (full / flow install): `install.sh` writes `.claude/settings.json`
with four auto-wired hooks (`SessionStart`, `UserPromptSubmit`, `PreToolUse`, `Stop`) and
copies the hook scripts into `.claude/hooks/`. The spec + flow skills work as plain project
files immediately; the flow hooks activate as soon as the settings.json wiring is in place.
`/flow` is a native project command — no plugin registration required.

**Uninstall** removes only what vibe installed and keeps your content
(`.spec/**`, your `AGENTS.md` prose, and the flow cursor unless `--yes`):

```bash
./install.sh /path/to/your/repo --uninstall            # cursor kept; use --dry-run to preview
./install.sh /path/to/your/repo --uninstall --yes       # also remove the flow cursor
./install.sh /path/to/your/repo --uninstall --only spec  # remove just one half
```

## The spec framework

Every project using vibe gets a `.spec/` tree — the single source of truth for
what you are building, why, and how. It ships as a bundled skill (`spec`) that
works standalone or drives the flow's authoring phases.

```
.spec/
├── product.md, tech.md, design.md, plan.md, lessons.md   ← ROOT (persistent role, current content)
└── features/<name>/
    ├── product.md    required     what this feature does (requirements + Scope)
    ├── tech.md       required     how it is built (paths, contracts, layout)
    ├── plan.md       recommended  stable <name>/n unit IDs; verification per unit
    ├── design.md     optional     UI/UX or design-system fragment
    └── research.md   optional     findings from spikes / investigations
```

Root files carry no backlog and no archaeology; feature folders are
branch-scoped — written at design, consumed at impl, merged (cross-cutting parts)
at compound, then deleted before the branch merges. **Code is truth.**

```bash
/spec setup            # initialise .spec/ with templates
/spec strategy         # write root product/tech/design/plan
/spec feature <name>   # scope and design a named feature
/spec validate         # check structural consistency
```

Deep dive: [`spec/README.md`](spec/README.md).

## The vibe flow

Everything starts at `idle`. The agent self-locates, then drives one flow. The
cursor `.agents/skills/vibe/state.json` = `{flow, phase, feature}` points at one
state in `state-machine.json` — the source of truth for each state's skill,
delegates, write surface, and legal `next`. Transition only via
`set-state.sh <flow.phase>`.

```mermaid
flowchart LR
    I((idle)) --> SB
    subgraph strategy
        SB[brainstorm] --> SS[spec]
    end
    subgraph feature
        D[design] --> P[plan] -. human gate .-> IM[impl] --> V[verify]
        V -. human gate .-> C[compound]
        V -->|targeted fix| IM
        V -->|major drift| P
    end
    subgraph quick
        T[triage] --> F[fix] --> QV[verify]
    end
    I --> D
    I --> T
    SS --> I
    C --> I
    QV --> I
```

> Simplified view — see [`flow/README.md`](flow/README.md) for the setup states.

The workflow is **one skill** (`vibe`) with a router plus per-phase files
(`setup`, `strategy`, `feature`, `quick`, `verify`, `compound`). A `SessionStart`
hook re-injects the working-model doctrine each session (and on `compact`); each
turn, the `UserPromptSubmit` hook injects the current state's **imperative** orders
— they name the literal transition command to run when the job is done (resolved
from the linked skill by `orders.sh`), and prepend a `vibe-drift:` nudge only when
working-tree activity contradicts the cursor; a `PreToolUse` hook guards the three
write invariants; a `Stop` hook runs warn-first exit checks and blocks in
`*.verify` without a fresh evidence receipt. A scope edit is not a state: edit
within the current state's write surface and stay put — `set-state.sh idle`
always aborts.

### Driving the flow with `/flow`

`/flow` is the transition command. Pass the target state — plus a feature name
when entering a feature flow:

```text
/flow feature.design my-feature   # start a feature; names the feature
/flow feature.plan                # advance to planning
/flow idle                        # abort — always legal
```

It reads the current cursor, refuses a target that is not in the state's `next`,
and otherwise calls `set-state.sh` for you — you never hand-edit the cursor. Most
edges **auto-advance**: the agent transitions and keeps working without asking.
The flow stops only at a **gated edge**, and a gated edge needs an explicit
confirm token before `/flow` will cross it. The two gates are
`feature.plan → feature.impl` (approve the plan units + pick the impl mode) and
`feature.verify → feature.compound` (approve shipping); the
`quick.triage → feature.design` escalation also confirms, because it renames the
work.

### A worked first run — see the loop once

A one-line bug, start to finish on the **quick** flow. You type the `/flow`
command; the inject hook answers with that state's telegraphic orders:

```text
> /flow quick.triage

skill=vibe · READ .spec/lessons.md first · defect: delegate superpowers:systematic-debugging (diagnose only, no fix) | non-defect: self-scope, no delegate · escalation to feature.design: announce AND confirm · done → set-state.sh quick.fix
```

The orders name the one job and the literal exit command. You reproduce the bug,
then advance and write the fix + its test:

```text
> /flow quick.fix

skill=vibe · delegate TDD + receiving-code-review on verify-routed re-entry · WRITE src/** (+ optional .spec/quick/<slug>.md note) · no root spec writes · done → set-state.sh quick.verify
```

With the fix in, you verify:

```text
> /flow quick.verify

skill=vibe · delegate verification-before-completion + code-reviewer · gather EVIDENCE the fix works and breaks nothing · no root spec writes · ... · findings → set-state.sh quick.fix | else → set-state.sh idle
```

Now the `Stop` gate has teeth: in `quick.verify` it **refuses to end the turn**
until a fresh `evidence/quick.md` receipt exists (staleness is git-derived). Write
the receipt with your evidence, and `set-state.sh idle` closes the loop.

### What actually enforces what

Only a few things are *hard*; the rest is convention the flow surfaces but does
not block on.

| Mechanism | Strength | What it does |
|---|---|---|
| `PreToolUse` guard — 3 write invariants | **Hard block** (exit 2) | `state.json` only via `set-state.sh`; `.spec/lessons.md` only in `feature.compound`, `setup.apply`, `strategy.spec`, or `quick.verify`; root `.spec/{product,tech,design,plan}.md` only in `strategy.spec`, `feature.compound`, or `setup.apply` |
| `PreToolUse` guard — Bash sniffer | **Warning** | the hard blocks intercept **file-tool** calls (Edit / Write / NotebookEdit) only; a raw `echo >> .spec/lessons.md` is caught by a text-scan sniffer that **warns**, never blocks (false positives are certain, so it can only nudge) |
| `Stop` gate — evidence receipt | **Hard block** (exit 2) | in a `*.verify` state, refuses to stop until a fresh `evidence/…` receipt exists (staleness is git-derived). This tooth fires **with or without `jq`** — jq-less, the cursor is read via sed and the block is byte-identical |
| `set-state.sh` — the cursor writer | **Not a gate** | it validates the target state *name* and writes the cursor; it does **not** enforce edge legality. Which edges are legal (and which need a confirm) is `/flow` convention, not a hook |
| everything else | **Warning** | auto-advance nudges, stuck-phase / impl-without-tests smells, per-turn orders — advisory only. Warnings are relayed back and appear at your **next prompt**, not mid-turn |

Everything degrades gracefully: a missing script or an unreadable cursor exits 0
and never ends the session. Absent `jq` only costs the machine-derived warn nudges —
the write invariants and the evidence-receipt block still fire (they read the flat
cursor via sed).

Deep dive: [`flow/README.md`](flow/README.md).

## Dependencies

vibe bundles only the `spec` skill. The flow *delegates* to external skills and
subagents, declared once in [`flow/reference/deps.json`](flow/reference/deps.json)
and reported by `doctor.sh`. **Every dependency degrades gracefully — a missing
one warns, never hard-fails.**

| Dependency | Kind | Source | If absent |
|---|---|---|---|
| superpowers | skill-collection | [obra/superpowers](https://github.com/obra/superpowers) | flow phases self-execute from their constraint documents |
| feature-dev | subagent-collection | Claude Code plugin: feature-dev | the orchestrator performs the explore / architect / review step inline |

## Platform support

vibe is portable by design; capability scales with the host.

| Host | What works | What is absent |
|---|---|---|
| **Claude Code** | Everything: spec skill, flow, `/flow` command, per-turn inject, guard + gate hooks | — |
| **Other `AGENTS.md` readers** (Codex, etc.) | Spec framework + instructions; agents follow the written flow manually | Hooks (no per-turn inject / guard / gate) |
| **Bare git / any editor** | Spec framework: `.spec/` docs, templates, `validate.sh` | Flow automation, hooks |

## Health & updates

```bash
# One-command install health report (warn-only, always exits 0):
bash /path/to/your/repo/.agents/skills/vibe/scripts/doctor.sh

# Update: re-run the installer. It refreshes the managed core and preserves your
# .spec/, AGENTS.md prose, and live flow cursor.
./install.sh /path/to/your/repo
```

## Layout

```text
your-repo/                     # after install
├── .agents/skills/
│   ├── spec/                  # bundled spec framework (real dir)
│   └── vibe/                  # flow: router, phase files, state machine, scripts
├── .claude/                   # Claude adapter: /flow command + four hooks + settings.json (flow half)
├── .spec/                     # your durable project memory
└── AGENTS.md                  # merged instructions (CLAUDE.md may symlink here)
```

In **this** repo the canonical halves live at [`spec/`](spec/) and
[`flow/`](flow/); `.agents/skills/{spec,vibe}` are compatibility symlinks (the
portable runtime interface). The installer dereferences them into real
directories in your target.

## Tests

```bash
bash tests/run.sh      # spec + flow + adapters suites; CI runs the full matrix
```

CI runs `shellcheck` on every tracked `*.sh`, the combined suite,
`spec/scripts/validate.sh`, and `spec/scripts/check-drift.sh` (compound / doc-drift
gate) on every push and PR.

## Documentation

- [`spec/README.md`](spec/README.md) — the spec framework, standalone.
- [`flow/README.md`](flow/README.md) — the flow: states, orders, hooks, degrade.
- [`.spec/product.md`](.spec/product.md) · [`.spec/tech.md`](.spec/tech.md) ·
  [`.spec/plan.md`](.spec/plan.md) — the harness's own specs (a living worked
  example of a filled-in `.spec/` tree).
- [`CHANGELOG.md`](CHANGELOG.md) — release notes.

## License

[MIT](LICENSE) © 2026 Lennard Zündorf
