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

`vibe config` exposes exactly seven operations: `init`, `import-mcp`, `status`,
`sync`, `install`, `uninstall`, `doctor`. Their flags and handlers are preserved;
see [Global store and projection](cli-store.md). Top-level commands are
`config`, `hook`, `spec` and generated `help`. Bare `vibe config` shows usage
and exits 2 without effects. Old `store`, `sync`, `install`, `uninstall` and
`doctor` commands are removed without aliases; they exit 2 before effects.
`config store` and `config hook` do not exist.

Embedded `vibe spec`
needs no peer or Node; `VIBE_SPEC_BIN` is deprecated and ignored. Raw spec arguments preserve OS strings and `--`. Compatibility
`vibe-spec` remains available for installed skills/hooks. Central source delivery
does not change release/installer distribution.

The front door owns SessionStart projection only, one entry per harness merged
by `vibe config install` (D48). It registers no plugin hook. Spec, flow and instruct
retain their own hook ownership (D25).

## Planned C1–C6 surface

C1–C6 remain NOT STARTED. Approved spec-projects-mcp is a spec-only C1/F22
carve-out; it does not deliver general dispatch, Spaces or worktree lifecycle.
See [Projects and MCP](../spec/spec-projects-mcp.md),
[worktree lifecycle](cli-worktree.md) and [global spec space](cli-space.md).
Independent tool installation/release trains remain; making the installer
default to central `vibe` remains release/install work.
