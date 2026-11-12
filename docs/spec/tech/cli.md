---
type: hub
applies:
  - "crates/vibe/**"
  - "vibe/**"
updated: 2026-11-12
---

# vibe CLI — Tech

[tech.md](../tech.md)

Branch doc for the v0.5 front door: the `vibe` binary, the worktree
lifecycle, and the global spec space. C0 `global-store` (D48) is implemented in source; the rest of the arc is
spec-ahead-of-code and gates on v0.4's F22 (installer/release surface).

The approved [spec-projects-mcp](spec/spec-projects-mcp.md) surface is the sole
additional spec-only C1/F22 exception. General C1–C6 remain queued.

## Notes

- [The vibe binary](cli/cli-binary.md) — subcommands, dispatch, installer target
- [Global store and projection](cli/cli-store.md) — store layout, per-harness projection, state, markers (D48)
- [Repo registry](cli/cli-registry.md) — the one cross-repo discovery mechanism
- [Worktree lifecycle](cli/cli-worktree.md) — create, list, remove, archive
- [Global spec space and blocks](cli/cli-space.md) — global lessons, cross-repo visibility, global blocks
- [Degrade rules, contracts, testing](cli/cli-degrade-testing.md) — degrade rules, contracts touched, testing posture
