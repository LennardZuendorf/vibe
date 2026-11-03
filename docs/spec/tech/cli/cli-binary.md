---
type: tech
applies:
  - "crates/vibe/**"
  - "vibe/**"
updated: 2026-11-03
---

# The vibe binary

`crates/vibe`, bin `vibe`, depends on `vibe-core`, `clap` and `serde_json`; it depends on no peer tool crate.

## Implemented C0 surface

Store, projection, sync, hook registration, install, uninstall, status and doctor are described in [Global store and projection](cli-store.md). `vibe spec …` runs `$VIBE_SPEC_BIN`, else `vibe-spec` on `PATH`, with inherited stdio and the peer’s exit code. A missing peer prints one advice line and exits 0. `doctor` checks harness CLIs, store parsing, state and lock.

The front door is a fourth, user-level hook owner for SessionStart projection only, one entry per harness merged by `vibe install` (D48, amending D25 and D40). It registers no plugin hook. Spec, flow and instruct retain their own hook ownership (D25).

## Planned C1–C6 surface

C1–C6 remain NOT STARTED. C1 adds `spec|flow|instruct` dispatch through the shim’s five discovery tiers, registry and peer checks. Native [worktree lifecycle](cli-worktree.md) and [global spec space and blocks](cli-space.md) remain planned.

The three tool binaries remain independently installable with their own plugins and release trains; `vibe` is a convenience front door (D40). Making the installer default to `vibe` and pulling the three tools along remains planned release/install work.
