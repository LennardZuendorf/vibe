---
type: concept
updated: 2026-10-19
---

# Story

vibe started as one kit: a Node engine, two skills, a plugin that carried only part of the runtime, and companion plugins — one of which (caveman) was a hard dependency until `flow-mvp` demoted it to frozen vocabulary. That is the bundle-everything shape that makes today's agent tooling hard to compose, and it left real gaps. `vibe state set` validated a state name but never checked `next` membership or the two human gates — approval was honour-system prose. Every hook shelled out to Node; without it, guidance went quiet and only the two hard blocks survived on frozen bash. And one managed block mixed shared repo conventions with one developer's personal workflow.

v0.4 splits vibe into three small, independent tools — `spec`, `flow`, and `instruct` — each a static Rust binary with zero runtime dependency, each useful with the other two absent. The state-machine writer becomes the gate itself: `vibe-flow state set` refuses an illegal target and a gated edge without `--confirm`, in Rust, with no Node in the path. Instruction injection moves from one hand-rolled content layer into `instruct`'s four-tier system, budgeted per trigger class at the same line counts vibe already proved. Coupling between the three tools moves from shared code to versioned file contracts under `.vibe/`. When it ships, installing one tool or all three is the same command, and losing any one of them degrades the others gracefully instead of breaking them.

v0.5 adds the front door: one `vibe` CLI that ships all three tools and owns what no single tool can — creating isolated feature workspaces (git worktrees, each with its own harness state and a bootstrapped spec/instruct setup), a global lessons library that follows one developer across every repo, and cross-repo visibility into what every vibe-managed project is doing. The three tools never require it; it requires exactly the contracts they already publish.
