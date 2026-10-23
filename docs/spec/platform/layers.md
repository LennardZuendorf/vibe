---
type: platform
updated: 2026-10-19
---

# Layers

| Layer | Owns | Carrier |
|---|---|---|
| Memory | `docs/spec/` (`.spec/` legacy), AGENTS.md managed blocks, `.vibe/*.json`, `.vibe/blocks/` | repo, committed | <!-- literal -->
| Spec | spec tree, templates, validation, OpenSpec interchange | plugin (`spec/`) |
| Flow | state machine, cursor, guard, ledger, signals, TUI | plugin (`flow/`) |
| Instruct | tiers, blocks, channels, budgets, sources | plugin (`instruct/`) |
| Core | root resolution, atomic writes, marker grammar, hook parsing | crate (`vibe-core`, internal) |
| Contracts | payload v1, marker grammar v1, spec JSON v1, provider manifest v1 | `contracts/` schemas + goldens |
| Carriers | hook shims, plugin manifests, `install.sh` | `bin/hook.sh`, `.claude-plugin/` |
| Front door (v0.5) | tool dispatch, worktree lifecycle, global spec space, global blocks | crate (`vibe`, plugin `vibe/`) |
