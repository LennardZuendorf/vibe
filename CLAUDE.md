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
