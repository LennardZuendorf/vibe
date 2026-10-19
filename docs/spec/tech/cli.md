---
type: hub
applies:
  - "crates/vibe/**"
  - "vibe/**"
updated: 2026-10-19
---

# vibe CLI — Tech

[tech.md](../tech.md)

Branch doc for the v0.5 front door: the `vibe` binary, the worktree
lifecycle, and the global spec space. **Not started** — the whole arc is
spec-ahead-of-code and gates on v0.4's F22 (installer/release surface).

## Notes

- [The vibe binary](cli/cli-binary.md) — subcommands, dispatch, installer target
- [Repo registry](cli/cli-registry.md) — the one cross-repo discovery mechanism
- [Worktree lifecycle](cli/cli-worktree.md) — create, list, remove, archive
- [Global spec space and blocks](cli/cli-space.md) — global lessons, cross-repo visibility, global blocks
- [Degrade rules, contracts, testing](cli/cli-degrade-testing.md) — degrade rules, contracts touched, testing posture
