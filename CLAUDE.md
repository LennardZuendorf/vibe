# vibe — Claude Code Adapter

This file is the Claude Code-facing adapter for vibe. It is not the
workflow source of truth.

## Canonical Core

- Durable planning lives in `.spec/`.
- Runtime flow state lives in `.agents/flow/`.
- Workflow shims are agent skills under `.agents/skills/vibe-*`.
- Platform-specific Claude files under `.claude/` read the core; they do not own
  a separate state model.

## Workflow Rule

When the user asks for strategy, feature work, quick fixes, verification, or
compounding, prefer the matching `vibe-*` skill:

| Intent | Skill |
|---|---|
| Setup or repair workflow harness | `vibe-setup` |
| Project strategy or refocus | `vibe-strategy` |
| Named feature lifecycle | `vibe-feature` |
| Small bounded fix | `vibe-quick` |
| Evidence before completion | `vibe-verify` |
| Lessons, promotion, archive | `vibe-compound` |
| Scope correction | `vibe-amend` |

Each `vibe-*` skill must inject exact `.spec/` output paths when delegating to
other skills such as `spec` or `superpowers:*`.

A single inject owner (the `UserPromptSubmit` hook) emits one frozen string per
`<flow>.<phase>` state, which also sets the caveman density level. Do not run a
separate caveman tracker in parallel. Level definitions are canonical in
`.spec/product.md` (Communication Levels): `lite` for setup/strategy/design/
compound/amend, `full` for implementation/verification/quick work, `ultra` only
for compound receipts and subagent summaries — never for triage. Regardless of
level, keep security warnings and irreversible-action confirmations in normal
prose; caveman compresses output, never reasoning.

## Adapter Boundary

Do not treat `.claude/state.json`, `.claude/state-machine.json`, or
`.claude/skills/` as canonical. Claude commands and hooks should read
`.agents/flow/state.json` and `.agents/flow/state-machine.json`.

## Flow State & Transitions

- **Read state:** `.agents/flow/state.json` = `{flow, phase, feature, updated}`.
  The compound `<flow>.<phase>` key indexes `.agents/flow/state-machine.json`,
  which holds each state's skill, delegates, caveman level, read/write surface,
  frozen `inject` string, and legal `next` array.
- **Transition only via** `bash .agents/flow/scripts/set-state.sh <flow.phase>
  [feature]`. Never edit `state.json` directly. Check the current state's `next`
  before moving; refuse illegal transitions. Transitions are agent-*suggested* —
  name the next state and confirm before calling the script.
- **Follow the inject literally.** It names the one skill, the write surface, the
  output path, the caveman level, and the next legal state.
- **Helper scripts:** `validate-state.sh` (cursor sanity), `detect-context.sh`
  (state snapshot + allow/warn/block write decision), `regen-active-rules.sh`
  (rebuild the active-rules digest below from `lessons.md` during compound).

The active-rules block at the bottom of this file is **generated** from
`.spec/lessons.md` by `regen-active-rules.sh`. Do not edit inside its markers;
edit `lessons.md` during a compound phase and re-run the script.

## Spec Framework

Use `.agents/skills/spec/SKILL.md` for spec navigation and validation. The root
spec model is:

- `.spec/product.md`
- `.spec/tech.md`
- `.spec/design.md`
- `.spec/plan.md`
- `.spec/lessons.md`

Feature specs live under `.spec/features/<feature>/` with `product.md` and
`tech.md` required, plus optional `design.md`, `plan.md`, and `research.md`.

<!-- vibe:active-rules:start -->
<!-- Generated from .spec/lessons.md by regen-active-rules.sh. Do not edit by hand;
     edit lessons.md and re-run during compound. Top 5, pinned first. -->

### Active Rules

_No lessons recorded yet._
<!-- vibe:active-rules:end -->
