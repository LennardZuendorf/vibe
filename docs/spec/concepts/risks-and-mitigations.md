---
type: concept
updated: 2026-10-19
---

# Risks & Mitigations

| Risk | Mitigation |
|---|---|
| Binary absent on a target platform/arch | Shim discovery falls to frozen bash fallback (guard/stop) or exits 0 with one line elsewhere; `doctor` reports it. |
| Plugin installed without a binary present | `install.sh` falls back to `cargo install --locked`; the two hard blocks still run bash. |
| Cross-version drift between the three tools | `contracts/run.sh` is byte-exact CI for every tool; additive-only within a major; `doctor` warns on major mismatch. |
| Dual-runtime limbo during the port | `port-order` CI job fails if TUI or signals code exists before `tests/parity/GREEN.hard`. |
| Global instruct tier makes every repo noisy | Global blocks default to `session-start`; `user-prompt.level` entry is explicit opt-in, hard-linted. |
| A strictness bump breaks an install overnight | Every check ships `warn`; promotion to `error` happens only in the compound that migrated the last live spec. |
| `docs/spec` rename breaks a live repo | `migrate` asserts population (before ≥1, after ==0, files ≥ floor) and inserts allow markers before any check can fire. |
| Windows lacks symlinks and `~/.config` | `@AGENTS.md` import instead of symlink; `%APPDATA%\vibe\` global tier; cargo-dist ships an msvc target. |
| Registry points at moved/deleted repos (v0.5) | `space list` warns on stale entries and keeps them; repair is re-init or `--uninstall`, never a destructive guess. |
| Worktree remove drops a mid-flow feature's state (v0.5) | `remove` refuses a mid-flow cursor; the only hard refusal in the lifecycle, pinned by a discriminating test. |
