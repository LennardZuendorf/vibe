---
type: concept
updated: 2026-10-19
---

# Requirements

At a project level, vibe must:

1. **Three independent tools.** Each installable alone, useful alone; no tool ever requires another — anti-bundling is a design value.
2. **Static binaries, no runtime.** Rust, zero runtime deps; a missing binary degrades to the frozen bash fallbacks for the two hard blocks and to silence elsewhere.
3. **Two platforms from one source.** Claude Code and OpenCode plugins per tool; Codex and other agents read the committed memory.
4. **Committed memory is portable.** The spec tree and each tool's `AGENTS.md` block are Markdown, readable by any agent or none.
5. **Contracts are the only coupling.** Versioned, golden-tested, additive within a major version.
6. **flow stands alone.** A standalone `AGENTS.md` block makes the agent drive the flow without instruct; instruct only strengthens adherence.
7. **The writer is the gate.** Legal edges and human gates are enforced by the cursor writer, not by prose.
8. **Injection is layered and budgeted.** Four tiers; trigger classes level/edge/event; line budgets linted; no live state at session start.
9. **Enforce in code, explain in prose.** Prose may route to a coded rule, never restate it. Data over prose: anything the machine knows is interpolated, never restated in English.
10. **Degrade gracefully.** Missing peers, binaries, networks, or trees warn; sessions never break.
11. **One installer, safe lifecycle.** Subsets, platforms, dry-run, and an uninstall that reverses only recorded actions.
12. **spec is OpenSpec-grammar compatible and layout-native.** Interchange works both ways; vibe's own document model stays.
13. **Strictness ratchets.** Every check ships warn-first; it becomes an error only after migration.
14. **Signals never gate.** Progress signals are advisory by construction.
15. **Ported code proves itself.** Frozen oracles, differential parity, and blast-radius order — highest-risk units port first.
16. **One front door (v0.5).** The `vibe` CLI dispatches to installed tools and owns the cross-tool capabilities; a missing peer degrades to one advice line, never a failure.
17. **Isolated feature workspaces (v0.5).** `vibe worktree` provisions a git worktree per feature with its own runtime state, a bootstrapped harness, and compound-time cleanup.
18. **Personal memory travels (v0.5).** A global lessons library and global instruction blocks follow the user across repos and machines; repo-owned memory always wins on conflict.
