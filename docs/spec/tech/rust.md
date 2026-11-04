---
type: hub
applies:
  - "crates/**"
  - "oracles/**"
  - "tests/parity/**"
updated: 2026-11-03
---

# Rust Workspace — Tech

[tech.md](../tech.md)

Branch doc for the Rust implementation: workspace layout, the port from the
Node engine and bash scripts, the hook shim, the TUI, and distribution.
**Current source:** the workspace contains `vibe-core`, `vibe-spec`, `vibe-parity` and `vibe-flow`. The Rust spec baseline and flow F5/F6 are implemented. JS still composes injection; migration shell shims remain. Flow transition enforcement, ledger, Rust orders/payloads, instruct, signals and TUI remain pending. Source completion does not establish release or installed runtime behavior.

## Notes

- [Workspace](rust/rust-workspace.md) — crates, MSRV, dependency allowlist, errors
- [Shim and discovery](rust/rust-shim-discovery.md) — hook shim and the five-tier binary discovery
- [Oracles and parity](rust/rust-oracles-parity.md) — frozen oracles, the differential harness, glob matching
- [Parity pins and port order](rust/rust-parity-pins.md) — pinned divergences, port order, carried rows
- [Machine teeth, ledger and TUI](rust/rust-flow-runtime.md) — state set, ledger and signals, TUI v1
- [Distribution and CI matrix](rust/rust-distribution-ci.md) — cargo-dist targets, installer steps, CI jobs
