---
type: tech
applies:
  - "crates/vibe-flow/**"
  - "crates/vibe-signals/**"
  - "flow/state-machine.json"
  - ".vibe/run/flow/**"
updated: 2026-11-08
---

# Flow runtime: current source and pending targets

F5/F6 implement `state get|set`, `policy check|decide|list|render`, and
`hook pre-tool-use-guard|stop-gate` in `crates/vibe-flow/`.
The F7 transition check, its ledger events and the pending ledger and signal
contracts are in [machine teeth](rust-flow-gates.md).
`policy check` refuses with exit 1; `decide` preserves exit 2.

## Repo-local layout

`FlowPaths` owns discovery: `--root` → `CLAUDE_PROJECT_DIR` → `VIBE_ROOT`
→ nearest `.vibe/` or `.git` ancestor → cwd. Data discovery is
`--vibe-dir` → `.agents/flow` when present → `.agents/skills/vibe`.
Runtime remains under `.vibe/run/`; flow does not use a per-user directory.
The [flow contract](../../parts/flow/concepts/flow-contract.md) specifies
absence-only cursor/evidence fallback, physical confinement and diagnostics.

## Current skill contracts

The flow skill has one `SKILL.md`; phase files are removed.
Doctrine is ≤700 bytes; each order block is ≤400 bytes and their average
is ≤250 bytes. The style summary is ≤220 bytes and the AGENTS template
is ≤110 lines. Executable checks live in `flow/tests/run.sh`.

## Pending targets

F8 adds Rust orders/doctrine/doctor, payload emission, provider manifest
and standalone managed AGENTS output. F11/F12 add instruct composition
and the hook ownership handoff. JS remains active until its retirement row.
F9 signals and F10 TUI/playback remain pending; advisory signals cannot
become blocking evidence. The writer-gate and ledger target decisions remain
[D27](../../decisions/d27-the-writer-is-the-gate.md) and
[D28](../../decisions/d28-ledger-advisory-signals.md).
`.planning/` is read-only context and never authoritative state.
Receipt-verdict enforcement is pending; current receipts use freshness only.
Pending recorded flows save to `~/.config/vibe/flows/<name>.json` (`%APPDATA%`
on Windows); global storage remains a deferred target. A project selects a
linear machine subset through `.vibe/flow.json` `{"machine":"<path>"}`,
falling back to its own machine. These targets do not describe current runtime.
