# shards-code — Codex Adapter

This file is the Codex-facing adapter for shards-code. It points Codex at the
platform-neutral workflow core.

## Canonical Core

- Durable planning: `.spec/`
- Runtime flow state: `.agents/flow/`
- Workflow skills: `.agents/skills/code-*`
- Spec framework skill: `.agents/skills/spec/`

Codex should not invent a separate state model. `.agents/flow` is shared with
Claude Code and any future adapter.

## Skill Routing

Use the matching `code-*` skill for recurring workflow work:

| Intent | Skill |
|---|---|
| Setup or repair workflow harness | `code-setup` |
| Strategy docs | `code-strategy` |
| Feature lifecycle | `code-feature` |
| Small fix | `code-quick` |
| Verification | `code-verify` |
| Lessons/archive/spec promotion | `code-compound` |
| Scope amendment | `code-amend` |

`code-*` skills are shims: they delegate to other skills with injected routing.
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
