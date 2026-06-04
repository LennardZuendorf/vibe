---
name: vibe-feature
description: |
  Design and build a named feature end to end: design → plan → impl → verify →
  compound. The main build loop. Writes feature specs under
  .spec/features/<name>/ then source under src/ and tests/.
  Trigger on: building a feature, "I need <feature>", implement, add capability,
  feature work, or user says feature.
user-invocable: true
argument-hint: "<feature-name>"
allowed-tools: Read, Edit, Write, Bash
compatibility: Requires bash + jq. macOS and Linux.
metadata:
  author: lennarddib
  version: "1.0"
---

# vibe-feature — build loop

Carries a named feature from spec to verified code. Five states with **two human
gates**: after `plan` (before impl) and after `verify` (before ship/compound).
Everything between is autonomous.

`feature.design → feature.plan →[gate] feature.impl → feature.verify →[gate]
(feature.compound | feature.impl | feature.plan)`

## Procedure

1. **Locate + name.** Read `.agents/flow/state.json`. Establish the feature name
   and set it: `bash .agents/flow/scripts/set-state.sh feature.design <name>`.
   The name is carried in the cursor across all feature states.

2. **Design** (`feature.design`, caveman lite). Read `.spec/lessons.md` first.
   Delegate to `superpowers:brainstorming` for the human dialogue, plus the
   `code-explorer` subagent to trace the codebase and `code-architect` to sketch
   approaches — they compose, they do not compete. Write **only**
   `.spec/features/<name>/product.md` and `tech.md` (inject those exact paths).
   Suggest `feature.plan` when the approach is chosen.

3. **Plan** (`feature.plan`, caveman lite). Delegate to `superpowers:writing-plans`
   + `code-architect`. Write `.spec/features/<name>/plan.md`. Give every
   implementation unit a **stable ID** (`U1`, `U2`, …); IDs never change on
   reorder or split. **Human gate:** confirm the plan before moving to impl.

4. **Impl** (`feature.impl`, caveman full). Delegate to
   `superpowers:executing-plans` + `superpowers:test-driven-development`. Write
   `src/**` and `tests/**` only — do **not** edit `.spec/**`. Cite plan unit IDs
   in tests/commits so state survives re-planning.

5. **Verify** (`feature.verify`, caveman full). Hand to `vibe-verify` /
   `superpowers:verification-before-completion` + `requesting-code-review` +
   `code-reviewer`. Gather evidence per unit ID. **Human gate** before ship.
   Route: pass → `feature.compound`; targeted fix → `feature.impl`; major drift →
   `feature.plan`.

6. **Compound** (`feature.compound`, caveman lite; receipts ultra). Hand to
   `vibe-compound`: record a tagged lesson, promote cross-cutting decisions to
   root specs, archive the feature folder, regenerate the active-rules digest.

## Rules

- Inject exact output paths in every delegation.
- Transitions are agent-*suggested* until proven: name the next state, confirm.
- If a `quick` task escalated here, start at `feature.design`.
- Caveman compresses output, never reasoning. Security/irreversible actions stay
  in normal prose at every level.
