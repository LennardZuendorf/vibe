---
type: entrypoint
scope: design
children:
  - features/spec-framework/design.md
  - features/code-flow/design.md
  - features/platform-adapters/design.md
updated: 2026-05-14
---

# shards-code — Design

Cross-cutting design language for the personal coding workflow. This is not a
visual product; design here means interaction shape, information hierarchy,
agent-facing tone, and the ergonomics of moving through work.

**Product:** [product.md](product.md)
**Architecture:** [tech.md](tech.md)

---

## Design Principles

1. **The current state should be obvious.** An agent and a human should be able
   to tell what flow is active, what phase comes next, and which files are in
   scope without reconstructing history.
2. **The workflow should feel guided, not trapped.** Warnings and legal next
   states should make the right path easy while preserving escape hatches for
   recovery.
3. **Delegation should be explicit.** When a `code-*` skill uses another skill,
   it names the output path and expected artifact so the delegated skill cannot
   invent its own file layout.
4. **Adapter copy should be boring.** Codex and Claude Code wording may differ,
   but their rules should point back to the same `.agents/flow` and `.spec`
   contracts.

---

## Interaction Conventions

- Use `code-*` skill names for recurring workflows.
- Use `.agents/flow` for runtime state and `.spec` for durable memory.
- In agent-facing prompts, phrase constraints as positive targets first:
  "write only these paths" before "do not write elsewhere."
- End each phase with a concise receipt: changed files, verification evidence,
  and next legal transition.

---

## Information Hierarchy

| Surface | Primary Question It Answers |
|---|---|
| `AGENTS.md` / `CLAUDE.md` | How should this runtime behave in this repo? |
| `.agents/skills/code-*` | What should the agent do for this workflow phase? |
| `.agents/flow/state.json` | What phase are we currently in? |
| `.agents/flow/state-machine.json` | What phases and transitions are legal? |
| `.spec/**` | What are we building, why, how, and what remains? |

---

## Feature Design Index

| Feature | Design Detail |
|---|---|
| spec-framework | [features/spec-framework/design.md](features/spec-framework/design.md) |
| code-flow | [features/code-flow/design.md](features/code-flow/design.md) |
| platform-adapters | [features/platform-adapters/design.md](features/platform-adapters/design.md) |
