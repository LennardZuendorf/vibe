---
name: spec
description: |
  You MUST use this before changing what the product does, how it is built,
  or any file under docs/spec, docs/spec or docs/superpowers — features,
  refactors, architecture, decisions, PRDs, plans, roadmaps, and also
  one-line fixes, typos and new flags. "Quick" and "just fix it" still
  route here first. Invoke it before reading files, before writing code,
  before brainstorming alone, before answering "how should we build X",
  and whenever the user mentions spec, PRD, design doc, plan, or roadmap.
user-invocable: true
argument-hint: "[strategy|feature [<name>]|interview [<name>]|promote <name>|audit|diff <name>|health|research <name>|lessons-for <tag>|product|tech|design|plan|lessons|setup|validate]"
allowed-tools:
  - Read
  - Edit
  - Write
  - Glob
  - Grep
  - Bash
  - Agent
compatibility: Requires bash. macOS and Linux.
metadata:
  author: lennarddib
  version: 2.0

context:
  - feature.md
  - strategy.md
  - reference/product.md
  - reference/tech.md
  - reference/plan.md

agents:
  - name: spec-promoter
    path: agents/spec-promoter.md
    trigger: feature.compound promote step
  - name: spec-health
    path: agents/spec-health.md
    trigger: /spec health
    user-invocable: true

superpowers:
  - superpowers:brainstorming          # feature design (default layer)
  - superpowers:writing-plans          # feature plan (default layer)
  - code-explorer                      # HOW codebase trace (fallback)
  - code-architect                     # HOW approach sketch (fallback)
  - superpowers:verification-before-completion  # audit quality framing
  - superpowers:finishing-a-development-branch  # compound lesson narrative

outputs:
  - docs/superpowers/specs/YYYY-MM-DD-<name>-design.md   # default feature layer
  - docs/superpowers/plans/YYYY-MM-DD-<name>.md          # default feature layer
  - docs/spec/features/<name>/product.md   # fallback
  - docs/spec/features/<name>/tech.md      # fallback
  - docs/spec/features/<name>/design.md    # fallback
  - docs/spec/features/<name>/plan.md      # fallback
  - docs/spec/product.md            # strategy + compound only
  - docs/spec/tech.md               # strategy + compound only
  - docs/spec/lessons.md            # compound only
  - docs/spec/.config.yaml          # setup only

reads:
  - docs/spec/product.md
  - docs/spec/tech.md
  - docs/spec/design.md
  - docs/spec/lessons.md
  - docs/spec/plan.md
  - docs/spec/.config.yaml
  - docs/superpowers/specs/
  - docs/superpowers/plans/
  - docs/spec/features/<name>/

delegates:
  - role: spec-promoter
    when: compound (promote durable facts to root, then delete feature artifacts)
    superpowers: [superpowers:finishing-a-development-branch]
  - role: spec-health
    when: audit / structural assessment (/spec health)
    superpowers: []

phases:
  - setup.apply
  - strategy.spec
  - feature.design
  - feature.plan
  - feature.compound
---

# spec

<HARD-GATE>
Do not write code, plans, or design docs until this skill has routed the
work. Routing is not optional. A reply to the user is not routing.
</HARD-GATE>

## The Rule

1. Invoke this skill before you act on any change to behaviour, structure, or docs.
2. Read `docs/spec/lessons.md` and the root doc of the area you touch. No `docs/spec/` yet → run `/spec setup` first.
3. Route the work by the table below. Announce the route in one line.
4. Write only where the route says. Nowhere else.
5. Finish: bump `updated:` on every spec you edited, then run `vibe-spec validate`.

## Routing

| Work | Do this | Writes |
|---|---|---|
| Strategy: product direction, architecture, decisions, root `plan.md` sequence, `design.md`, branch docs | Load [strategy.md](strategy.md). Edit the root spec yourself (dialogue via `superpowers:brainstorming` allowed) | `docs/spec/` root only |
| Feature, new or structural | Invoke `superpowers:brainstorming`, then `superpowers:writing-plans`. Topic = feature name. | `docs/superpowers/specs/`, `docs/superpowers/plans/` |
| Feature, bounded | Invoke `superpowers:brainstorming` (bounded path) | code; root-doc drift is fixed via strategy.spec |
| Bug or quick fix | Invoke `superpowers:systematic-debugging`; fix in place | code; root-doc drift is fixed via strategy.spec |
| Implementing an approved plan | Follow the plan task by task. The design doc is frozen. | code and tests |
| Question or investigation (not design: "how should we build X" is a feature) | Answer from the spec and the code | nothing |
| Feature wrap-up (compound) | Promote durable facts into root specs + `lessons.md`, then delete the feature's artifacts | root `docs/spec/`, `lessons.md` |
| Fallback | Load [feature.md](feature.md) and run it | `docs/spec/features/<name>/` |

Fallback only when: superpowers is not installed, the user says *self*,
`.vibe/spec.json` sets `"featureLayer": "vibe"`, or OpenSpec interchange is
required. No other trigger exists.

One named buildable unit → feature. Spans every feature → strategy. Two rows fit → the heavier one.

A wrapped-up feature (DONE in root `plan.md`) has no feature folder. Route to
its live surface (code, tests, root docs). Never restore the folder.

**$ARGUMENTS:**

| Argument | Action |
|---|---|
| `strategy` | Load [strategy.md](strategy.md) |
| `feature [<name>]` | Route as a feature (table above); fallback loads [feature.md](feature.md) + `docs/spec/features/<name>/` |
| `interview [<name>]` | Fallback only: run `superpowers:brainstorming` for the WHAT phase, steps 1–2, in this skill |
| `promote <name>` | Run `vibe-spec promote <name>` through the spec-promoter agent (fallback folders) |
| `audit` | Run `vibe-spec validate` |
| `diff <name>` | Run `vibe-spec scan-merges <name>` — pending merge blocks |
| `health` | Run the spec-health agent — structural assessment |
| `research <name>` | Fallback only: open `docs/spec/features/<name>/research.md` |
| `lessons-for <tag>` | Run `vibe-spec lessons-for <tag>` |
| `product` · `tech` · `design` · `plan` · `lessons` | Load that root doc and follow its links |
| `setup` | Run the setup interview (§ Setup) |
| `validate` | Run `vibe-spec validate` |
| _(none)_ | Route from the task (table above) |

## Red Flags

| Thought | Reality |
|---|---|
| "This is too small for a spec" | Small work routes too: bounded features to the bounded row, fixes to the quick-fix row. Route it. |
| "It's a one-line fix, I'll just fix it" | Quick fixes route to the bug/quick-fix row. Invoke this skill first. |
| "It's a small targeted change, I'll read the file first" | Invoke this skill before reading code, in any permission mode. |
| "I'll update the spec after the code" | The spec leads. Route first. |
| "The user only asked a question" | "How should we build X" is design. Route it. |
| "I know this codebase" | Lessons exist because agents knew the codebase. Read them. |
| "Superpowers will handle it" | Superpowers designs. This skill decides where it is recorded. Invoke both. |
| "I'll write the feature spec into `docs/spec/features/`" | Only on a fallback trigger. Default is superpowers' own artifacts. |
| "The plan already exists, I can edit the design doc" | Design docs are frozen during impl. Route back to design. |

## Layers

Root layer — persistent, high level, current only. No spec holds a backlog; long-term ideas go to an external tracker.

| Doc | Holds | Guide |
|---|---|---|
| `docs/spec/product.md` | mini PRD: story, users, requirements, principles, non-goals | [reference/product.md](reference/product.md) |
| `docs/spec/tech.md` | architecture: stack, layout, contracts, build sequence, risks | [reference/tech.md](reference/tech.md) |
| `docs/spec/design.md` | cross-cutting design language, UX conventions ([`DESIGN.md` format](https://github.com/google-labs-code/design.md) for visual tokens) | [reference/design.md](reference/design.md) |
| `docs/spec/plan.md` | feature map, Feature Sequence with whole-feature gates, current focus | [reference/plan.md](reference/plan.md) |
| `docs/spec/lessons.md` | tagged mistakes; read at session start | — |
| `docs/spec/{product,tech}-<topic>.md` | cross-cutting concerns only | [reference/product.md](reference/product.md) |

Feature layer — branch-scoped, deleted before the branch merges:

- Default: `docs/superpowers/specs/YYYY-MM-DD-<name>-design.md` + `docs/superpowers/plans/YYYY-MM-DD-<name>.md`.
- Fallback: `docs/spec/features/<name>/{product,tech}.md`, optional `design.md`, `plan.md` (stable `<name>/n` unit IDs), `research.md`. Rules: [feature.md](feature.md).

Rules:

1. Read before write. Never edit a spec you have not read this session.
2. Root specs stay high level. Feature detail lives in the feature layer.
3. One concern per doc. Product docs hold no code. Tech docs hold no UX opinion.
4. Bump `updated:` on every edit.
5. Link parent ↔ child both ways; list children in the entrypoint frontmatter. Load only what the task needs.
6. Compound order: promote durable facts → record lessons → mark the feature DONE in root `plan.md` as a one-line note linking its live surface → delete the feature artifacts. A fallback folder moves to `archive/<name>/`; prompt the user to delete it once validation passes, before merge. Never read `archive/` for active work.

## Roles

| Role | Executor | Phase | Constraint |
|---|---|---|---|
| spec-promoter | `vibe-spec promote` + `superpowers:finishing-a-development-branch` | feature.compound | `reference/tech.md` merge markers |
| spec-health | `vibe-spec validate` | any (`/spec health`, `/spec audit`) | `vibe-spec validate` output |

Agents live under [agents/](agents/). The fallback WHAT interview runs `superpowers:brainstorming` and the HOW trace runs `code-explorer` + `code-architect` from this skill itself.

## Commands

The commands are the `vibe-spec` binary's. Run `vibe-spec validate` (structure, links, frontmatter),
`vibe-spec list-specs`, `vibe-spec lessons-for <tag>` and `vibe-spec scan-merges <name>` (pending
`<!-- merge -->` blocks) from anywhere in the project; they walk up to the git root to find the spec
root (`vibe-spec root` prints it). Run `vibe-spec promote <name>` and `vibe-spec init` from the project
root. If `vibe-spec` is not on PATH, build it (`cargo build -p vibe-spec`).

## Current Project State

!`vibe-spec list-specs`

## Config

Read `docs/spec/.config.yaml` before any route. A missing file or key means the default.

| Key | Default | When set |
|---|---|---|
| `vibe-flow` | `false` | `true` → the vibe flow cursor owns phase routing and output density |
| `suggest-superpowers` | `true` | `false` → do not offer optional executors (code-explorer, code-architect); the feature layer is unchanged |
| `superpowers.<key>` | `true` | `false` → that executor is unavailable; `brainstorming` or `writing-plans` false → fallback ("superpowers is not installed") |

Output density follows the vibe flow's single `style` note: concise, no
filler; security and irreversible-action warnings stay in full prose.

## Setup

On `/spec setup`:

1. `docs/spec/.config.yaml` present → show settings, update only the one named, confirm, write.
2. Absent → ask three questions, conversationally:
   - **Q1 — Workflow orchestration:** driven by the vibe flow (`/flow`) or by `/spec` directly? → yes | no
   - **Q2 — Available superpowers:** `all`, `none`, or `custom` (per superpower)?
   - **Q3 — Proactive suggestions:** offer optional executors at each step, or stay quiet? Default: yes.
3. Show the summary. Write nothing before the user confirms.
4. On confirm: write `docs/spec/.config.yaml` (always, even with all defaults), then run `vibe-spec init`. Report what it created; next is `/spec strategy` or `/spec feature <name>`.

## Templates

Copy from [reference/templates/](reference/templates/) into `docs/spec/`:

| Template | For |
|---|---|
| [product.md](reference/templates/product.md) · [tech.md](reference/templates/tech.md) · [design.md](reference/templates/design.md) · [plan.md](reference/templates/plan.md) | root docs |
| [product-topic.md](reference/templates/product-topic.md) · [tech-topic.md](reference/templates/tech-topic.md) · [plan-topic.md](reference/templates/plan-topic.md) | cross-cutting branch docs (`type: product-topic` / `tech-topic` / `plan-topic`) |
| [feature-product.md](reference/templates/feature-product.md) · [feature-tech.md](reference/templates/feature-tech.md) · [feature-design.md](reference/templates/feature-design.md) · [feature-plan.md](reference/templates/feature-plan.md) · [research.md](reference/templates/research.md) | fallback feature folder |
