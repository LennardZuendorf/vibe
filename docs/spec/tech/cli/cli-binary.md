---
type: tech
applies:
  - "crates/vibe/**"
  - "vibe/**"
updated: 2026-10-19
---

# The vibe binary

`crates/vibe`, bin `vibe`, depends on `vibe-core` only (same `cargo metadata`
no-tool-depends-on-tool rule). Subcommands:

| Subcommand | Behaviour |
|---|---|
| `vibe spec\|flow\|instruct …` | Exec the peer binary through the same five-tier discovery as `bin/hook.sh`; a missing peer prints one "not installed" advice line, exit 0 — advice, never failure. |
| `vibe worktree …` | Native — worktree lifecycle ([Worktree lifecycle](cli-worktree.md)). |
| `vibe space …` | Native — global spec space ([Global spec space and blocks](cli-space.md)). |
| `vibe doctor` | Peer checks + registry + global-tier consistency. |

The three tool binaries remain independently installable with their own
plugins and release trains; `vibe` is a convenience front door, never a
requirement (D40). The installer's default target becomes `vibe` (which pulls
the three tools along); `--only` subsets are unchanged. The `vibe` plugin
carries the front-door command surface only — no hooks of its own; spec,
flow, and instruct keep their hook ownership (D25).
