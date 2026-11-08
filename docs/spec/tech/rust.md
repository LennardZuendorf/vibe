---
type: hub
applies:
  - "crates/**"
  - "oracles/**"
  - "tests/parity/**"
updated: 2026-11-04
---

# Rust Workspace — Tech

[tech.md](../tech.md)

Branch doc for the Rust implementation: workspace layout, the port from the
Node engine and bash scripts, the hook shim, the TUI, and distribution.
**Current source:** the workspace contains `vibe`, `vibe-core`, `vibe-spec`, `vibe-parity` and `vibe-flow`. The Rust spec baseline, flow F5/F6 and C0 store/projection front door are implemented. The later C1–C6 CLI modules remain pending. JS still composes injection; migration shell shims remain. Flow transition enforcement, ledger, Rust orders/payloads, instruct, signals and TUI remain pending. Source completion does not establish release or installed runtime behavior.

## Notes

- [Workspace](rust/rust-workspace.md) — crates, MSRV, dependency allowlist, errors
- [Shim and discovery](rust/rust-shim-discovery.md) — hook shim and the five-tier binary discovery
- [Oracles and parity](rust/rust-oracles-parity.md) — frozen oracles, the differential harness, glob matching
- [Parity pins and port order](rust/rust-parity-pins.md) — pinned divergences, port order, carried rows
- [Flow runtime](rust/rust-flow-runtime.md) — surface, repo-local layout, skill contracts, pending targets
- [Machine teeth and ledger](rust/rust-flow-gates.md) — the transition check, ledger events, pending ledger and signals
- [Distribution and CI matrix](rust/rust-distribution-ci.md) — cargo-dist targets, installer steps, CI jobs
