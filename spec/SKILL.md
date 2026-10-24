---
name: spec
description: |
  You MUST use this before changing what the product does, how it is built,
  or any file under the spec root (docs/spec) or docs/superpowers: features,
  refactors, architecture, decisions, PRDs, plans, roadmaps, and also
  one-line fixes, typos and new flags. "Quick" and "just fix it" still
  route here first. Invoke it before reading files, before writing code,
  before brainstorming alone, before answering "how should X be built",
  and whenever the user mentions spec, PRD, design doc, plan, or roadmap.
user-invocable: true
argument-hint: "[strategy|feature [<feature>]|interview [<feature>]|promote <feature>|diff <feature>|health|research <feature>|lessons-for <tag>|product|tech|design|plan|lessons|setup|validate]"
allowed-tools:
  - Read
  - Edit
  - Write
  - Glob
  - Grep
  - Bash
  - Agent
compatibility: Requires the vibe-spec binary on PATH (see the README Install section). Without it, the legacy scripts check a flat legacy root only. macOS and Linux.
---

# spec

<SUBAGENT-STOP>
If you were dispatched as a subagent for a specific task, skip this skill. Your dispatch
prompt holds your orders.
</SUBAGENT-STOP>

## Overview

**Core principle:** the spec holds current decisions, never history or backlog;
code is truth, so when spec and code disagree, fix the spec in the same change.
`<root>` is the directory `vibe-spec root` prints. Bundled commands are run, not read.

<HARD-GATE>
Do not write code, plans, or design docs until this skill has routed the
work. Routing is not optional. A reply to the user is not routing.
</HARD-GATE>

## The Rule

1. Invoke this skill before you act on any change to behaviour, structure, or docs.
2. Run `vibe-spec lessons-for <area tags>` for the area you touch, and read that
   area's hub. Read the whole `<root>/lessons.md` only at strategy and wrap-up.
   No spec root (`vibe-spec root` exits 3) → run `/spec setup` first.
3. Route the work by the table below. Announce the route in one line.
4. Write only where the route says. Nowhere else.
5. Finish: bump `updated:` on every spec you edited, then run `vibe-spec validate`.

## Routing

| Work | Do this | Writes |
|---|---|---|
| Strategy: product direction, architecture, decisions, the Feature Sequence, design language, topic docs | Load [strategy.md](strategy.md). Edit the root spec yourself (dialogue via `superpowers:brainstorming` allowed) | `<root>` root docs only |
| Feature, new or structural | Invoke `superpowers:brainstorming`, then `superpowers:writing-plans`. Topic = feature name. | `docs/superpowers/specs/`, `docs/superpowers/plans/` |
| Feature, bounded (one file, no new requirement) | Invoke `superpowers:brainstorming` and ask for a short in-chat design, no design doc | code; root-doc drift goes through the Strategy row |
| Bug | Invoke `superpowers:systematic-debugging`; fix in place | code; root-doc drift goes through the Strategy row |
| Typo, rename, new flag (no behaviour question) | Edit directly; run `vibe-spec validate` if a spec changed | code |
| Implementing an approved plan | Follow the plan task by task. The design doc is frozen. | code and tests |
| Question or investigation (not design: "how should we build X" is a feature) | Answer from the spec and the code | nothing |
| Feature wrap-up (compound) | Promote durable facts into root specs + `lessons.md`, then delete the feature's artifacts | `<root>` root docs, `lessons.md` |
| Fallback | Load [feature.md](feature.md) and run it | `<root>/features/<feature>/` |

**Not for:** explaining code with no change planned, running tests, git housekeeping, formatting data.

Fallback only when: superpowers is not installed, your human partner says *self*,
`.vibe/spec.json` sets `"featureLayer": "vibe"`, or OpenSpec interchange is
required. No other trigger exists.

One named buildable unit → feature. Spans every feature → strategy. Two rows fit → the heavier one.

A wrapped-up feature (DONE in root `plan.md`) has no feature folder. Route to
its live surface (code, tests, root docs). Never restore the folder.

**$ARGUMENTS:**

| Argument | Action |
|---|---|
| `strategy` · `feature [<feature>]` | Route by the table above |
| `interview [<feature>]` | Fallback only: run [feature.md](feature.md) steps 1–2 yourself (the WHAT interview) |
| `research <feature>` | Fallback only: open `<root>/features/<feature>/research.md` |
| `promote <feature>` | Run `vibe-spec promote <feature>` through the spec-promoter agent |
| `diff <feature>` · `lessons-for <tag>` · `validate` | Run `vibe-spec scan-merges <feature>` · `vibe-spec lessons-for <tag>` · `vibe-spec validate` |
| `health` | Run the spec-health agent (`vibe-spec validate` + `vibe-spec check-drift`) |
| `product` · `tech` · `design` · `plan` · `lessons` | Load that root doc and follow its links |
| `setup` | Run the setup interview in [reference/setup.md](reference/setup.md) |
| _(none)_ | Route from the task |

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
| "I'll write the feature spec into `<root>/features/`" | Only on a fallback trigger. Default is superpowers' own artifacts. |
| "The plan already exists, I can edit the design doc" | Design docs are frozen during impl. Route back to design. |

**Any of these thoughts means: stop, invoke this skill, route.**

## Layers

`vibe-spec root --json` prints `layout`.
- **layered** (what `vibe-spec init` creates): the four hubs `product|tech|design|plan.md`
  are `type: hub`, ≤80 lines, and only link. Detail lives in typed notes (≤60 lines)
  under `areas/ flows/ concepts/ platform/ tech/ decisions/ questions/` or
  `parts/<part>/`. A topic is a folder note, never a root `*-<topic>.md`.
  Read [reference/layered.md](reference/layered.md) before you add a note.
- **flat** (legacy): the root docs and templates in [reference/setup.md](reference/setup.md) § Flat layout apply.

Root layer — persistent, high level, current only; long-term ideas go to an external tracker.
What each root doc holds: [product](reference/product.md) · [tech](reference/tech.md) · [design](reference/design.md) · [plan](reference/plan.md).

Feature layer — branch-scoped, deleted before the branch merges:

- Default: `docs/superpowers/specs/YYYY-MM-DD-<feature>-design.md` + `docs/superpowers/plans/YYYY-MM-DD-<feature>.md`.
- Fallback: `<root>/features/<feature>/{product,tech}.md`, optional `design.md`, `plan.md` (stable `<feature>/n` unit IDs), `research.md`. Rules: [feature.md](feature.md).

Rules:

1. Read before write: edit only specs you read this session.
2. Root specs stay high level; feature detail lives in the feature layer.
3. One concern per doc: product docs hold no code, tech docs no UX opinion.
4. Bump `updated:` on every edit.
5. Link parent ↔ child both ways. Load only what the task needs.
6. Compound order: promote durable facts → record lessons → mark the feature DONE in root `plan.md` as a one-line note linking its live surface → delete the feature artifacts. A fallback folder moves to `archive/<name>/`; prompt the user to delete it once validation passes, before merge. Never read `archive/` for active work.

## Roles

| Role | Executor | Phase | Constraint |
|---|---|---|---|
| spec-promoter | `vibe-spec promote` + `superpowers:finishing-a-development-branch` | feature.compound | `reference/tech.md` merge markers |
| spec-health | `vibe-spec validate` | any (`/spec health`, `/spec audit`) | `vibe-spec validate` output |

Agents live under [agents/](agents/). The fallback WHAT interview runs `superpowers:brainstorming` and the HOW trace runs `code-explorer` + `code-architect` from this skill itself.

## Commands

Every command is `vibe-spec <command>`; it finds the spec root from anywhere in the
project. Run `vibe-spec promote <feature>` and `vibe-spec init` from the project root.

If `vibe-spec` is not on PATH, say "vibe-spec is not installed" and point your
human partner to the README Install section. On a flat legacy root you may run
`bash <skill-dir>/scripts/validate.sh` (`<skill-dir>` is the directory that holds
this file). On any other root no validator runs: say "validation unavailable"
in your finish line and never claim the tree validated.

## Config

The keys `featureLayer`, `suggest` and `superpowers` live in `.vibe/spec.json`. Read
[reference/setup.md](reference/setup.md) § Config before you route; it also holds
`/spec setup` and the flat-layout templates.

## Current Project State

!`vibe-spec list-specs 2>/dev/null || echo "(vibe-spec is not installed; see § Commands)"`
