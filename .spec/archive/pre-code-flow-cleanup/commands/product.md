---
type: feature-product
feature: commands
sibling: tech.md
parent: ../../product.md
updated: 2026-05-03
---

# Feature: Commands — Product

The three commands are the entire user surface of shards-code. This doc captures user-facing requirements: when each command is used, what state it produces, and the escape hatches.

**Parent:** [../../product.md](../../product.md)
**Architecture:** [tech.md](tech.md)

---

## Why this feature exists

Without an opinionated command surface, every project re-invents the lifecycle and the user has to remember which skill to invoke. Three commands close that gap by covering the three real workflows: ad-hoc fix, project bootstrap, full feature build. Anything outside these three is a rabbit hole.

---

## Requirements

| # | Requirement |
|---|---|
| R1 | Three commands cover all workflows in v1: `/code:quick`, `/code:strategy`, `/code:feature`. No fourth in v1. |
| R2 | Each command is invoked as a slash command and is self-contained — no setup before use. |
| R3 | Quick is stateless. It does not read or write `.spec/.phase`. |
| R4 | Strategy and feature persist state in `.spec/.phase` so they're resumable across sessions. |
| R5 | Feature has exactly two human gates: after PLAN (before VERIFY) and after REVIEW (before SHIP). Everything else is autonomous. |
| R6 | Feature requires global specs to exist; if not, errors and points at `/code:strategy`. |
| R7 | Strategy is re-runnable for refocus. It must not silently clobber user-written branch docs. |
| R8 | When a quick task balloons (>5 files touched or scope creep detected), it stops and suggests `/code:feature`. |
| R9 | Each command produces a one-line summary at the end and suggests the next command if applicable. |

---

## `/code:quick <task>`

**Use when:** small change, ≤5 files, no architecture decision. Bug fixes, copy tweaks, in-place refactors, dependency bumps.

**User experience:**
- Trivial change (≤2 lines literal text, single file, no logic): implement directly.
- Non-trivial: write `.spec/.quick-plan.md`, ask "Plan written. Proceed?", implement on yes.
- Run tests, light review (no `ce-code-review` — too heavy), done.

**Outputs:** source code changes; ephemeral `.spec/.quick-plan.md` (gitignored) for non-trivial tasks.

**Does NOT:** write specs, update lessons.md, run ce-code-review, push or open PRs.

**Escape hatch:** scope balloons → stop, summarize, suggest `/code:feature <name>`.

---

## `/code:strategy`

**Use when:** bootstrapping a new project, or refocusing one that's drifted from its specs.

**User experience:**
- RESEARCH (delegated to `ce-strategy` + `ce-ideate`): scan codebase, capture findings.
- DISCUSS (delegated to `superpowers:brainstorming`): Socratic, one question at a time.
- SPEC (delegated to `/spec`): write/update `product.md`, `tech.md`, optionally `product-design-language.md`.
- PLAN (optional, delegated to `ce-plan`): "Want a top-level roadmap? y/n".
- Re-runnable for refocus. Asks before overwriting global specs. Branch docs are user-owned and never auto-deleted.

**Outputs:** global specs at `.spec/`, optionally `plan.md`, research notes at `.spec/research/strategy-notes.md`.

**Does NOT:** touch source code, write feature specs, update lessons.md, ship or commit.

---

## `/code:feature <name>`

**Use when:** building a real, named feature. Heavyweight on purpose — this is the discipline command.

**User experience:**

DESIGN cluster (heavy, user-involved):
- RESEARCH → DISCUSS → SPEC → PLAN
- Writes `.spec/features/<name>/{product.md, tech.md, plan.md, research.md, design.md}`
- **Human gate:** "Plan ready. Approve?"

IMPL cluster (autonomous between gates):
- VERIFY (drift check, suggest `/code:amend` on targeted drift)
- WORK (TDD via Superpowers; `ce-work` if user says "ralph it")
- REVIEW (`ce-code-review`; P1 → back to WORK)
- **Human gate:** "Ready to ship?"
- SHIP (`ce-commit-push-pr`)
- COMPOUND (`ce-compound` + built-in: append lessons, merge cross-cutting tech.md sections into globals, archive feature folder)

**Outputs:** feature specs (created in DESIGN, read-only after), source code (during WORK), commit + PR (during SHIP), updated `lessons.md` and global `tech.md` (during COMPOUND only), archived feature folder.

**State transitions** through `.spec/.phase`: see [tech.md](tech.md).

**Resumable:** re-running `/code:feature <same-name>` reads `.phase` and continues from the saved phase.

---

## `/code:amend` (v1.1)

**Use when:** mid-IMPL the codebase has diverged from the feature spec enough that targeted amendment is needed without rewinding to DESIGN.

**Sketch:** read current phase (must be `feature:IMPL:*`), open feature's product/tech.md, diff against codebase, propose targeted patches, user approves each patch, resume IMPL:WORK.

Specced for v1.1 in [tech.md](tech.md). Not built in v1; documented here so the surface is visible.

---

## Cross-Command Behavior

What every command does at the start:
1. Call `bin/detect-context.sh <workflow>` and parse the JSON.
2. Surface non-empty `warnings`.
3. Verify each `skills_to_load` is available; degrade gracefully on missing.
4. Load `global_context` paths only (progressive disclosure).

What every command does at the end:
1. Call `bin/set-phase.sh <next-phase>` to advance state (or `""` for neutral).
2. Print a one-line summary.
3. Suggest the next command if applicable.

What no command may do:
- Write `.spec/.phase` directly.
- Write `.spec/lessons.md` outside `feature:IMPL:COMPOUND`.
- Write global specs outside `strategy:DESIGN:SPEC` or `feature:IMPL:COMPOUND`.
- Skip the human gates in `/code:feature`.

These are enforced by hooks; see [features/hooks/](../hooks/product.md).

---

## Non-Goals

- A fourth command in v1.
- A `/code:status` or `/code:resume` command — `bin/detect-context.sh` already exposes state, and re-running the active command resumes naturally.
- A `/code:undo` or branch-rewind command. Use git.
- A web UI, dashboard, or progress tracker.

---

## Open Questions

1. **Quick threshold.** "Any logic change → plan" is the default. Watch for friction in real use; tune toward stricter or looser as needed.
2. **`ce-work` "ralph it" override syntax.** Plain English ("just code it", "ralph it", "skip TDD") parsed by command, or explicit flag (`/code:feature <name> --no-tdd`)? Default: detect natural language, document the phrases.
3. **Feature reactivation from `archive/`.** Copy back + resume from `feature:DESIGN:SPEC:<name>`, or treat as fresh DESIGN? Default: copy + resume.
4. **Strategy refocus diff UX.** Full diff at once, or sectioned with per-section gates? Default: sectioned.
5. **`/code:amend` priority.** v1.1 default. Promote to v1 only if drift becomes a frequent pain in the first project.
