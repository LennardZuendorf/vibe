---
type: tech
applies:
  - "crates/**"
  - "Cargo.toml"
  - "rust-toolchain.toml"
  - "deny.toml"
updated: 2026-10-19
---

# Workspace

```text
crates/
├── vibe-core       # lib, publish = false — internal only
├── vibe-spec       # bin: vibe-spec
├── vibe-flow       # bin: vibe-flow (feature `tui` default on)
├── vibe-instruct   # bin: vibe-instruct
├── vibe-signals    # lib — Signal, SignalProvider; flow depends on it, core does not
└── vibe-parity     # dev-only differential harness
```

The spec program (F2–F4, F16–F20) creates `vibe-core`, `vibe-spec` and
`vibe-parity` only; the flow, instruct and signals crates arrive with their rows.

Each tool binary path-depends on `vibe-core` only; no tool crate depends on
another (`cargo metadata` CI test). MSRV 1.88 (the `ratatui` floor), pinned
in `rust-toolchain.toml` and `rust-version`, bumped only at a tagged
release, built at MSRV in CI. Dependency allowlist via `cargo-deny`: `clap`
(derive), `serde`, `serde_json`; `ratatui` + `crossterm` in `vibe-flow`
only, behind feature `tui`. No `toml`, `regex`, `tokio`, or `tempfile` —
JSON is the only config format; glob matching and atomic writes (tmp +
rename, 0600 on unix) are hand-written in core and parity-proven against
the bash oracles.

Errors: core returns `Result<T, vibe_core::Error>`; no panics on input
(clippy `-D unwrap_used`). Bins map errors to the exit codes in
[tech-contracts.md](../contracts.md).
