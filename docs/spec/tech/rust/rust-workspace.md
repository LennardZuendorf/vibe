---
type: tech
applies:
  - "crates/**"
  - "Cargo.toml"
  - "rust-toolchain.toml"
  - "deny.toml"
updated: 2026-11-04
---

# Workspace

```text
crates/
├── vibe            # bin: vibe (C0 store/projection front door)
├── vibe-core       # lib, publish = false — internal only
├── vibe-spec       # bin: vibe-spec
├── vibe-flow       # bin: vibe-flow (F5/F6 source)
└── vibe-parity     # dev-only differential harness
```

The Rust spec baseline, F5/F6 and C0 are present. C1–C6 remain pending.
`vibe-instruct`,
`vibe-signals`, TUI features and their extra dependencies remain pending.

Each tool binary path-depends on `vibe-core` only; no tool crate depends on
another (`cargo metadata` CI test). MSRV 1.88 (the `ratatui` floor), pinned
in `rust-toolchain.toml` and `rust-version`, bumped only at a tagged
release, built at MSRV in CI. Dependency allowlist via `cargo-deny`: `clap`
(derive), `serde`, `serde_json`. `ratatui` + `crossterm` remain future
TUI dependencies, not current workspace requirements. No `toml`, `regex`, `tokio`, or `tempfile` —
JSON is the only config format; glob matching and atomic writes (tmp +
rename, 0600 on unix) are hand-written in core and parity-proven against
the bash oracles.

Errors: core returns `Result<T, vibe_core::Error>`; no panics on input
(clippy `-D unwrap_used`). Bins map errors to the exit codes in
[tech-contracts.md](../contracts.md).
