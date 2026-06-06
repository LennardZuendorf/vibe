# vibe — Codex Adapter

This file is the Codex-facing adapter for vibe. It points Codex at the
platform-neutral workflow core.

## Canonical Core

- Durable planning: `.spec/`
- Runtime flow state: `.agents/flow/`
- Workflow skills: `.agents/skills/vibe-*`
- Spec framework skill: `.agents/skills/spec/`

Codex should not invent a separate state model. `.agents/flow` is shared with
Claude Code and any future adapter.

## Flow State & Transitions

- **Read state:** `.agents/flow/state.json` = `{flow, phase, feature, updated}`.
  The compound `<flow>.<phase>` key indexes `.agents/flow/state-machine.json`,
  which holds each state's skill, delegates, caveman level, read/write surface,
  frozen `inject` string, and legal `next` array.
- **Transition only via** `bash .agents/flow/scripts/set-state.sh <flow.phase>
  [feature]`. Never edit `state.json` directly. Check the current state's `next`
  before moving; transitions are suggested, then confirmed.
- **Follow the inject literally.** It names the skill, write surface, path,
  caveman level, and next state.
- **Helper scripts:** `validate-state.sh`, `detect-context.sh` (snapshot +
  allow/warn/block decision), `regen-active-rules.sh` (rebuild the digest below
  during compound).

The active-rules block at the bottom of this file is **generated** from
`.spec/lessons.md` by `regen-active-rules.sh`; edit `lessons.md` during compound,
not the block.

## Skill Routing

Use the matching `vibe-*` skill for recurring workflow work:

| Intent | Skill |
|---|---|
| Setup or repair workflow harness | `vibe-setup` |
| Strategy docs | `vibe-strategy` |
| Feature lifecycle | `vibe-feature` |
| Small fix | `vibe-quick` |
| Verification | `vibe-verify` |
| Lessons/archive/spec promotion | `vibe-compound` |
| Scope amendment | `vibe-amend` |

`vibe-*` skills are shims: they delegate to other skills with injected routing.
When they call `spec`, `superpowers:*`, or subagents, they must name the exact
`.spec/` files that may be written.

A single inject owner emits one frozen string per `<flow>.<phase>` state, which
also sets the caveman density level (level definitions are canonical in
`.spec/product.md`):

- `lite` for setup, strategy, design, compound, and scope amendment.
- `full` for implementation, verification, and quick work (including triage).
- `ultra` only for compound receipts and subagent→orchestrator summaries.

Caveman compresses output, not reasoning. Keep security warnings and
irreversible-action confirmations in normal prose at any level.

## Spec Layout

Root specs:

- `.spec/product.md`
- `.spec/tech.md`
- `.spec/design.md`
- `.spec/plan.md`
- `.spec/lessons.md`

Feature specs:

- `.spec/features/<feature>/product.md`
- `.spec/features/<feature>/tech.md`
- optional `.spec/features/<feature>/design.md`
- optional `.spec/features/<feature>/plan.md`
- optional `.spec/features/<feature>/research.md`

<!-- vibe:active-rules:start -->
<!-- Generated from .spec/lessons.md by regen-active-rules.sh. Do not edit by hand;
     edit lessons.md and re-run during compound. Top 5, pinned first. -->

### Active Rules

_No lessons recorded yet._
<!-- vibe:active-rules:end -->
