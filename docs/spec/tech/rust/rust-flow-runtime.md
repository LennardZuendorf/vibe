---
type: tech
applies:
  - "crates/vibe-flow/**"
  - "crates/vibe-signals/**"
  - "flow/state-machine.json"
  - ".vibe/run/flow/**"
updated: 2026-11-11
---

# Flow runtime: current source and pending targets

F5/F6 implement legacy `state get|set`, policy commands and guard/Stop hooks.
Legacy `state set` validates names and writes atomically; it remains unenforced.
F7 now has explicit v2 progression and task evidence, with production trusted
approval/evidence checks unavailable/deny. This is source status, not a release.

## Discovery and adoption
`FlowPaths` resolves project roots and canonical `.agents/skills/flow` resources,
retaining admitted legacy locations. Runtime lives under `.vibe/run/flow`.
`state upgrade <run> <plan-digest> <scope-digest>` is explicit and rewinds mutation
phases to planning. Legacy writers hold the shared lock and refuse adopted v2.
`state recover` checks the observed corrupt digest; adoption markers are sticky.
The [contract](../../parts/flow/concepts/flow-contract.md) defines fallback.

## v2 command boundary
`state transition` takes a typed request with expected revision and checks legal
edges under an exclusive lock. Immutable revision records precede the cursor.
Every implementation/ship entry needs a verified grant; production returns
unavailable/deny. No confirm flag manufactures authority. Abort is separate from
completion. `state task-result` validates TaskBrief/TaskResult and required checks;
retries accept identical records, and dependencies bind accepted record identity.
`contracts/flow-runtime.schema.json` defines these serialized records.

`state candidate` hashes framed paths, content, modes, deletions and symlink
values using Git's object digest without writing an object or commit. It includes
tracked and all untracked/ignored paths except `.vibe/run/flow` and `target`.
Non-Git roots, submodules and unsupported entries fail. Task records, command
output metadata and digests remain untrusted observations. Native confinement,
human-origin approval and race-free execution have not been established.

## Skill and adapter boundary

`vibe` is the root guide, `flow` the lifecycle and `spec` the artifact capability.
Flow uses progressive procedures/references and generic-agent-compatible briefs.
D12 orders remain authored in SKILL.md and checked by `flow/tests/run.sh`.
Migration preserves cursor/receipts before replacing owned discovery links.
Native Claude Code, OpenCode and Codex blocking is deferred; hook registration
and installed-package instruction scenarios do not prove complete action coverage.

## Pending targets

F7's trusted authority, shared audit ledger and executable GSD aliases remain
pending. `.planning/` is read-only context. F8 adds Rust orders/doctrine/doctor,
payload emission and provider manifest; JS remains active. F11/F12 add Instruct
composition and hook ownership. F9 signals and F10 TUI/playback remain pending.
The proposed append-only `ledger.jsonl` with transition/hook/signal events is
separate from current immutable revision/task records; advisory signals never
become blocking evidence. See [D27](../../decisions/d27-the-writer-is-the-gate.md)
and [D28](../../decisions/d28-ledger-advisory-signals.md).
