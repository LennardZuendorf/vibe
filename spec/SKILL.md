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
| Feature wrap-up | Follow § Wrap-up | `<root>` root docs, `lessons.md` |
| Fallback | Load [feature.md](feature.md) and run it | `<root>/features/<feature>/` |

**Not for:** explaining code with no change planned, running tests, git housekeeping, formatting data.

Fallback only when: superpowers is not installed, your human partner says *self*,
`.vibe/spec.json` sets `"featureLayer": "vibe"`, or OpenSpec interchange is
required. No other trigger exists.

One named buildable unit → feature. Spans every feature → strategy. Two rows fit → the heavier one.

A wrapped-up feature has no folder: route to its live surface, never restore the folder.

**$ARGUMENTS:**

| Argument | Action |
|---|---|
| `interview [<feature>]` | Fallback only: run [feature.md](feature.md) steps 1–2 yourself (the WHAT interview) |
| `research <feature>` | Fallback only: open `<root>/features/<feature>/research.md` |
| `promote <feature>` | Run § Wrap-up step 1 |
| `diff <feature>` · `lessons-for <tag>` · `validate` | Run `vibe-spec scan-merges <feature>` · `vibe-spec lessons-for <tag>` · `vibe-spec validate` |
| `health` | Run the spec-health agent (`vibe-spec validate` + `vibe-spec check-drift`) |
| `product` · `tech` · `design` · `plan` · `lessons` | Load that root doc and follow its links |
| `setup` | Run the setup interview in [reference/setup.md](reference/setup.md) |
| `strategy` · `feature [<feature>]` · _(none)_ | Route by the table above |

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

Root layer — persistent, high level, current only. Guides: [product](reference/product.md) · [tech](reference/tech.md) · [design](reference/design.md) · [plan](reference/plan.md).

Feature layer — branch-scoped, deleted before the branch merges:

- Default: `docs/superpowers/{specs,plans}/YYYY-MM-DD-<feature>*.md` (design, plan).
- Fallback: `<root>/features/<feature>/{product,tech}.md`, optional `design.md`, `plan.md` (unit IDs `<feature>/n`), `research.md`; rules in [feature.md](feature.md).

Rules: read before write; root specs stay high level; one concern per doc (product
docs hold no code, tech docs no UX opinion); link parent ↔ child both ways; load
only what the task needs.

## Wrap-up

Create a todo for EACH step before starting; mark one done only after its check passes.
1. Preview: `vibe-spec promote <feature> --dry-run`. Show the output to your human
   partner and get their yes. Apply through spec-promoter (§ Roles and dispatch).
2. Lesson: append one tagged entry to `<root>/lessons.md` only when a durable rule
   surfaced (format: [strategy.md](strategy.md) § Lessons).
3. Row: mark the feature DONE in the root Feature Sequence (the `plan.md` hub or the
   sequence note it links), one line, linking the live surface.
4. Remove the feature layer: superpowers → `vibe-spec promote <feature> --done`;
   fallback → `git mv <root>/features/<feature> <root>/archive/<feature>`.
5. Check: `vibe-spec validate` and `vibe-spec check-drift --pre-merge`. Fix and
   re-run until both exit 0.
6. Fallback only: ask your human partner to delete `<root>/archive/<feature>/`.
7. Last: invoke `superpowers:finishing-a-development-branch` for the git close-out only.

## Roles and dispatch

- spec-promoter ([agents/spec-promoter.md](agents/spec-promoter.md), model sonnet): Wrap-up step 1.
- spec-health ([agents/spec-health.md](agents/spec-health.md), model haiku): `/spec health`.

Dispatch `spec-promoter` with the feature name; show its dry run; after your human
partner's yes, dispatch it again with "apply: yes". `DONE` or `DONE_WITH_CONCERNS` →
use the report; `NEEDS_CONTEXT` → supply it, re-dispatch once; `BLOCKED` or a second
`NEEDS_CONTEXT` → do the step yourself. You run `--done` (step 4).

## Commands

Run `vibe-spec promote` and `vibe-spec init` from the project root; every other
`vibe-spec` command finds the spec root from any directory.

If `vibe-spec` is not on PATH, say "vibe-spec is not installed" and point your
human partner to the README Install section. On a flat legacy root you may run
`bash <skill-dir>/scripts/validate.sh` (`<skill-dir>` is the directory that holds
this file). On any other root no validator runs: say "validation unavailable"
in your finish line and never claim the tree validated.

## Config

Read [reference/setup.md](reference/setup.md) § Config (`.vibe/spec.json` keys `featureLayer`,
`suggest`, `superpowers`) before you route. It also holds `/spec setup` and the flat templates.

## Current Project State

!`vibe-spec list-specs 2>/dev/null || echo "(vibe-spec is not installed; see § Commands)"`
