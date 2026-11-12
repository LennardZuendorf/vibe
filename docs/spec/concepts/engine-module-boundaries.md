---
type: concept
updated: 2026-11-12
---

# Engine Module Boundaries

`vibe-core` is `publish = false`, internal-only, and holds every shared
primitive: root resolution, atomic writes, marker grammar, layered JSON
config, hook-input parsing, peer probe, contract types, ledger.
The sole tool-crate edge is central `vibe -> vibe-spec`, enforced by the workspace
graph test. Core primitives stay spec-free; the shared spec application API owns
spec behavior, while CLI and MCP adapters own parsing/transport.
Outside the explicit central spec adapter, no module names a state, a spec
path, or a channel outside its own tool. Two
orders are pinned by test: repo-root resolution in core (`CLAUDE_PROJECT_DIR`
→ `VIBE_ROOT` → upward search for `.vibe/`/`.git` → cwd) and spec-root
discovery in `vibe-spec` (`--root` → env → config → `docs/spec/` → `.spec/` <!-- literal -->
→ walk up to `.git`). Ported units carry a frozen oracle and a
differential parity harness before they replace the original — order,
corpus, and the deletion gate: [tech-rust.md](../tech/rust.md).
