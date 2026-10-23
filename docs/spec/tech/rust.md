---
type: hub
applies:
  - "crates/**"
  - "oracles/**"
  - "tests/parity/**"
updated: 2026-10-19
---

# Rust Workspace — Tech

[tech.md](../tech.md)

Branch doc for the Rust implementation: workspace layout, the port from the
Node engine and bash scripts, the hook shim, the TUI, and distribution.
**Shipped today:** `flow/engine/` (Node/ESM) and the bash scripts still
drive everything. F2–F4 delivered the foundations: frozen oracles under
`oracles/` (43 files, `MANIFEST.sha256`), the Cargo workspace (`vibe-core`
with glob matching and atomic write, `vibe-spec`, `vibe-parity`), and the
parity harness. F16 made `vibe-spec` the first candidate. `list-specs`,
`lessons-for` and `scan-merges` are parity-green over
`tests/parity/corpus/spec/` and live on this repo (`tests/parity/run.sh`);
`validate` is parity-green over the corpus with F17's checks off
(`tests/parity/legacy-strictness.json`, P3b-R2).
`init` stays self-check only until F20.

## Notes

- [Workspace](rust/rust-workspace.md) — crates, MSRV, dependency allowlist, errors
- [Shim and discovery](rust/rust-shim-discovery.md) — hook shim and the five-tier binary discovery
- [Oracles and parity](rust/rust-oracles-parity.md) — frozen oracles, the differential harness, glob matching
- [Parity pins and port order](rust/rust-parity-pins.md) — pinned divergences, port order, carried rows
- [Machine teeth, ledger and TUI](rust/rust-flow-runtime.md) — state set, ledger and signals, TUI v1
- [Distribution and CI matrix](rust/rust-distribution-ci.md) — cargo-dist targets, installer steps, CI jobs
