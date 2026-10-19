---
type: tech
applies:
  - "install.sh"
  - ".github/**"
  - "dist-workspace.toml"
updated: 2026-10-19
---

# Distribution

cargo-dist targets: `{x86_64,aarch64}-unknown-linux-musl`,
`{x86_64,aarch64}-apple-darwin`, `x86_64-pc-windows-msvc`; each artifact
ships SHA256 + attestation. Independent tags per tool: `spec-v*`,
`flow-v*`, `instruct-v*`; first Rust releases `flow-v0.4.0`,
`spec-v0.4.0`, `instruct-v0.1.0`. `install.sh` steps: detect triple →
fetch `vibe-<tool>-<ver>-<triple>.tar.gz` + `SHA256SUMS` from GitHub
Releases → verify → install to `--bin-dir` (default `~/.local/bin`) → on
failure, `cargo install --locked` → register the plugin (Claude:
marketplace add + `claude plugin install <tool>@vibe --scope user`;
OpenCode: link the shim into `~/.config/opencode/plugins/`).
`~/.vibe/installed.json` records every action taken; `--uninstall`
reverses only what it recorded — never a guess.

## CI matrix

| Job | Runs |
|---|---|
| `rust-check` | fmt, clippy `-D warnings`, `cargo-deny` (the bash job holds `check`) |
| `rust-test` | unit + integration, 3 OS × stable + MSRV |
| `parity` | `crates/vibe-parity` vs. the spec oracles only until F5, then both oracle sets (ubuntu + node, dev-only) |
| `shellcheck` | `bin/hook.sh`, `bin/fallback/*.sh`, `install.sh` |
| `cross-smoke` | build all 5 targets; `--version` under qemu for aarch64-linux |
| `port-order` | fails while TUI/signals code exists ahead of `GREEN.hard` |
| `release` | cargo-dist on tag, per tool |
