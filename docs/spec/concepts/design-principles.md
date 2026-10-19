---
type: concept
updated: 2026-10-19
---

# Design Principles

1. **The current state should be obvious.** An agent and a human should be able to tell what flow is active, what phase comes next, and which files are in scope without reconstructing history.
2. **The workflow should feel guided, not trapped.** Warnings and legal next states should make the right path easy while preserving escape hatches for recovery.
3. **Delegation should be explicit.** When `flow` uses another skill, it names the output path and expected artifact so the delegated skill cannot invent its own file layout.
4. **Adapter copy should be boring.** Codex, Claude Code, and OpenCode wording may differ, but their rules should point back to the same tool and spec-root contracts.
5. **Each tool must feel whole alone.** Running `spec`, `flow`, or `instruct` by itself should never look like a piece is missing — the other two are enhancements, not prerequisites.
6. **The gate is a mechanism, not a request.** A human gate is enforced by the state-machine writer refusing an unconfirmed edge, never by a hook or prose asking nicely.
7. **The front door advises, it never blocks (v0.5).** The `vibe` CLI and its worktree/space capabilities degrade to advice lines; the only hard refusals remain the flow writer's gate and the worktree mid-flow `remove`.
