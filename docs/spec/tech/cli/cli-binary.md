---
type: tech
applies:
  - "crates/vibe/**"
  - "vibe/**"
updated: 2026-11-12
---

# The vibe binary

`crates/vibe`, bin `vibe`, embeds the shared `vibe-spec` library. This sole
approved tool-crate dependency does not permit other peer imports or spec
vocabulary in `vibe-core`.

## Implemented source surface

Store, projection, sync, hook registration, install, uninstall, `store status`
and doctor
are described in [Global store and projection](cli-store.md). Embedded `vibe spec`
needs no peer or Node; `VIBE_SPEC_BIN` is deprecated and ignored. Compatibility
`vibe-spec` remains available for installed skills/hooks. Central source delivery
does not change release/installer distribution.

The front door owns SessionStart projection only, one entry per harness merged
by `vibe install` (D48). It registers no plugin hook. Spec, flow and instruct
retain their own hook ownership (D25).

## Planned C1–C6 surface

C1–C6 remain NOT STARTED. Approved spec-projects-mcp is a spec-only C1/F22
carve-out; it does not deliver general dispatch, Spaces or worktree lifecycle.
See [Projects and MCP](../spec/spec-projects-mcp.md),
[worktree lifecycle](cli-worktree.md) and [global spec space](cli-space.md).
Independent tool installation/release trains remain; making the installer
default to central `vibe` remains release/install work.
