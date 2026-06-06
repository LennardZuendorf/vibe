---
type: entrypoint
scope: design
design_format: google-labs-code/design.md-inspired
children:
  - features/spec-framework/design.md
  - features/vibe-flow/design.md
  - features/platform-adapters/design.md
updated: 2026-05-14
---

# vibe — Design

Cross-cutting design language for the personal coding workflow. This is not a
visual product; design here means interaction shape, information hierarchy,
agent-facing tone, and the ergonomics of moving through work.

For UI-heavy projects, `.spec/design.md` should follow or reference the
google-labs-code [`DESIGN.md`](https://github.com/google-labs-code/design.md)
pattern: design tokens in YAML frontmatter plus markdown rationale. vibe
uses the same idea, but this repo's current design doc is prose-first because
the product is an agent workflow rather than a visual interface.

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
3. **Delegation should be explicit.** When a `vibe-*` skill uses another skill,
   it names the output path and expected artifact so the delegated skill cannot
   invent its own file layout.
4. **Adapter copy should be boring.** Codex and Claude Code wording may differ,
   but their rules should point back to the same `.agents/flow` and `.spec`
   contracts.

---

## Interaction Conventions

- Use `vibe-*` skill names for recurring workflows.
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
| `.agents/skills/vibe-*` | What should the agent do for this workflow phase? |
| `.agents/flow/state.json` | What phase are we currently in? |
| `.agents/flow/state-machine.json` | What phases and transitions are legal? |
| `.spec/**` | What are we building, why, how, and what remains? |

---

## Feature Design Index

| Feature | Design Detail |
|---|---|
| spec-framework | [features/spec-framework/design.md](features/spec-framework/design.md) |
| vibe-flow | [features/vibe-flow/design.md](features/vibe-flow/design.md) |
| platform-adapters | [features/platform-adapters/design.md](features/platform-adapters/design.md) |
