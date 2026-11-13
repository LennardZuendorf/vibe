---
type: tech
applies:
  - "crates/**"
  - "Cargo.toml"
  - "rust-toolchain.toml"
  - "deny.toml"
updated: 2026-11-12
---

# Workspace

```text
crates/
├── vibe            # bin: vibe (C0 store/projection and embedded spec entry)
├── vibe-core       # lib, publish = false — internal only
├── vibe-spec       # shared spec lib + compatibility bin: vibe-spec
├── vibe-flow       # bin: vibe-flow (F5/F6 source)
└── vibe-parity     # dev-only differential harness
```

The Rust spec baseline, F5/F6 and C0 are present. C1–C6 remain pending.
`vibe-instruct`,
`vibe-signals`, TUI features and their extra dependencies remain pending.

Tools use `vibe-core` primitives. The sole approved tool-crate edge is central
`vibe -> vibe-spec`; spec vocabulary stays out of core. The workspace graph test
enforces that exception and confines the reviewed rmcp/Tokio dependency closure
to the spec MCP adapter and its central embedding path. It grants no network
transport or async dependency permission to other tools/core.
MSRV remains 1.88, pinned in `rust-toolchain.toml` and `rust-version`, bumped only
at a tagged release and built in CI. `cargo-deny` checks the reviewed dependency
and license graph. Existing clap/serde/serde_json remain; ratatui/crossterm are
future TUI dependencies. No toml, regex or tempfile is admitted. JSON remains
the config format; core glob/atomic-write primitives retain frozen parity.

Errors: core returns `Result<T, vibe_core::Error>`; no panics on input
(clippy `-D unwrap_used`). Bins map errors to the exit codes in
[tech-contracts.md](../contracts.md).
