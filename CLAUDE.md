# shards-code — Claude Code Adapter

This file is the Claude Code-facing adapter for shards-code. It is not the
workflow source of truth.

## Canonical Core

- Durable planning lives in `.spec/`.
- Runtime flow state lives in `.agents/flow/`.
- Workflow shims are agent skills under `.agents/skills/code-*`.
- Platform-specific Claude files under `.claude/` read the core; they do not own
  a separate state model.

## Workflow Rule

When the user asks for strategy, feature work, quick fixes, verification, or
compounding, prefer the matching `code-*` skill:

| Intent | Skill |
|---|---|
| Setup or repair workflow harness | `code-setup` |
| Project strategy or refocus | `code-strategy` |
| Named feature lifecycle | `code-feature` |
| Small bounded fix | `code-quick` |
| Evidence before completion | `code-verify` |
| Lessons, promotion, archive | `code-compound` |
| Scope correction | `code-amend` |

Each `code-*` skill must inject exact `.spec/` output paths when delegating to
other skills such as `spec` or `superpowers:*`.

When `.agents/flow` names a caveman level, use it as the communication density:
`lite` for nuanced setup/strategy, `full` for implementation/verification, and
`ultra` for quick triage or compact receipts.

## Adapter Boundary

Do not treat `.claude/state.json`, `.claude/state-machine.json`, or
`.claude/skills/` as canonical. Claude commands and hooks should read
`.agents/flow/state.json` and `.agents/flow/state-machine.json`.

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
