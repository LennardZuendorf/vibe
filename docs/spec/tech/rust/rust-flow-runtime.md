---
type: tech
applies:
  - "crates/vibe-flow/**"
  - "crates/vibe-signals/**"
  - "flow/state-machine.json"
  - ".vibe/run/flow/**"
updated: 2026-11-21
---

# Flow runtime: current source and pending targets

Rust Flow implements cursor and policy operations, Guard and Stop hooks, legal
transitions with explicit confirmation, and the local ledger. F8 adds
`orders`, `doctrine`, `doctor`, `emit`, `init` and `render agents-md` for the
payload, provider manifest and managed block. See the
[Flow sequence](../../parts/flow/concepts/flow-feature-sequence.md).
`policy check` refuses with exit 1; `decide` preserves exit 2.

## Adopted v2 runs

[#96](https://github.com/LennardZuendorf/vibe/pull/96) adds explicit
`state upgrade`, `candidate`, `task-result`, `recover` and `transition`.
Adopted state is revision-checked and authoritative. Legacy writers cannot
downgrade it. Damaged adopted state blocks instead of falling back. Candidate
digests and task records identify inputs, and task records are untrusted.
Verify and ship fail closed without a trusted evidence collector. Scoped
product-write authority remains unavailable. [#101](https://github.com/LennardZuendorf/vibe/issues/101)
tracks the remaining design.

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

F11/F12 are in active planning and require feature approval.
JS remains available until retirement.
F9 signals and F10 TUI/playback remain pending; advisory signals cannot
become blocking evidence. The writer-gate and ledger target decisions remain
[D27](../../decisions/d27-the-writer-is-the-gate.md) and
[D28](../../decisions/d28-ledger-advisory-signals.md).
`.planning/` is read-only context and never authoritative state.
Legacy receipts use freshness. Adopted v2 runs use the fail-closed path above.
Pending recorded flows save to `~/.config/vibe/flows/<name>.json` (`%APPDATA%`
on Windows); global storage remains a deferred target. A project selects a
linear machine subset through `.vibe/flow.json` `{"machine":"<path>"}`,
falling back to its own machine. These targets do not describe current runtime.
