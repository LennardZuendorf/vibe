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

Use caveman levels from the active flow state when present:

- `lite` for setup, strategy, compound, and scope amendment.
- `full` for implementation and verification.
- `ultra` for quick triage and compact subagent receipts.

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
