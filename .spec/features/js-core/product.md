---
type: feature-product
feature: js-core
sibling: tech.md
parent: ../../product.md
updated: 2026-08-10
---

# js-core — Product

The engine that every later feature stands on. Today the same four jobs — find
the repo root, read the cursor, extract a marker block, parse the machine — are
reimplemented five, five, four, and three times respectively across ~4.2k lines
of bash, each with a hand-maintained jq-optional twin. js-core replaces that
with one Node implementation and proves it by producing byte-identical output to
the scripts it retires.

This feature is deliberately **behaviour-preserving**. It changes no policy, no
prose, and no state machine semantics. Its whole value is that everything after
it — the content layer, the gate teeth, the plugin runtime — gets written once
instead of twice.

## Scope

| | |
|---|---|
| **Owns** | `engine/` package: CLI dispatch, the single root resolver, cursor reader/writer, machine loader, marker/block extractor primitive, JSON I/O. Ports of `set-state`, `orders`, `doctrine`, `doctor`. The JS test harness. Bash hook shims that `exec` the engine. |
| **Does not own** | Content blocks, channels, `policy.json` (→ content-layer). Trigger classes and the `SessionStart` cursor-line fix (→ inject-triggers). Gate and edge enforcement (→ machine-teeth). Plugin manifest, `vibe init`, `vibe vendor`, install bug fixes (→ plugin-runtime). Spec validators, drift, promote (→ spec-js). |
| **Deferred** | Deleting the bash originals. They stay in the tree until the feature that supersedes each one lands, so a bisect never lands on a repo with neither implementation. |

## Requirements

### Requirement: One implementation per primitive (R1)

The engine MUST expose exactly one root resolver, one cursor reader, one cursor
writer, one machine loader, and one block extractor. Every engine command MUST
use them rather than re-deriving.

#### Scenario: A second cursor reader cannot be introduced silently

- **Given** the engine exposes `readCursor()` in `engine/cursor.mjs`
- **When** the suite runs its duplicate-primitive check
- **Then** it fails if any file under `engine/` other than `cursor.mjs` parses
  cursor JSON directly, naming the offending file

### Requirement: Byte-identical parity with the scripts it replaces (R2)

For every ported command, engine stdout MUST be byte-identical to the bash
script's stdout for the same cursor and repo state — including the no-jq path,
whose output the bash versions promise is already identical.

#### Scenario: Ported command output matches its bash original

- **Given** a cursor at `feature.impl` with feature `demo`
- **When** the suite runs both `bash flow/scripts/orders.sh` and `vibe orders`
- **Then** the two stdouts compare equal byte-for-byte, and the same holds for
  `set-state.sh`/`vibe state set`, `doctrine.sh`/`vibe doctrine`, and
  `doctor.sh`/`vibe doctor`

#### Scenario: Parity holds with jq absent

- **Given** `jq` is removed from `PATH` via a shim
- **When** the parity comparison runs again
- **Then** the outputs still compare equal, confirming the engine needs no jq
  and the bash no-jq path was a faithful twin

### Requirement: Correct resolution on a fresh, non-git target (R3)

Root resolution MUST succeed in a target with no `.git` and no `.spec`,
resolving relative to the engine's own location rather than an upward marker
search.

#### Scenario: Orders resolve in a bare directory

- **Given** the engine is installed into a bare `mktemp -d` with no `.git` and
  no `.spec`
- **When** `vibe orders` runs there
- **Then** it emits the `idle` orders rather than `state=unknown`

### Requirement: Absent Node degrades, never breaks (R4)

Hook shims MUST exit 0 and emit nothing when Node is unavailable, so a session
in a repo without Node loses enforcement but never fails.

#### Scenario: A hook shim with no Node on PATH

- **Given** `node` is absent from `PATH` via a shim
- **When** the `UserPromptSubmit` hook shim runs
- **Then** it exits 0, writes nothing to stdout, and the session proceeds

#### Scenario: The guard shim cannot block when Node is absent

- **Given** `node` is absent and the guard shim receives a `.spec/lessons.md` write
- **When** the shim runs
- **Then** it exits 0 rather than 2 — enforcement is lost, not inverted into a
  spurious block

### Requirement: The engine carries no runtime dependencies (R5)

The engine MUST run on stock Node 18+ with no `node_modules` at runtime, so the
plugin payload stays a file copy and needs no install step.

#### Scenario: Engine runs from a payload with no dependencies installed

- **Given** `engine/` copied into a directory with no `node_modules`
- **When** `node engine/cli.mjs doctor` runs
- **Then** it completes and exits 0

### Requirement: Unknown input fails loudly, missing input degrades quietly (R6)

An unknown subcommand or malformed cursor MUST produce a non-zero exit with a
named reason. A *missing* cursor MUST resolve to `idle`, matching current
behaviour.

#### Scenario: Malformed cursor is reported, not silently treated as idle

- **Given** a `state.json` containing invalid JSON
- **When** `vibe state get` runs
- **Then** it exits non-zero naming the file and the parse error — closing the
  gap where all five bash readers silently degraded a corrupt cursor to `idle`

#### Scenario: Missing cursor is idle

- **Given** no `state.json` exists
- **When** `vibe state get` runs
- **Then** it reports `idle` and exits 0

## Outputs

- `engine/` — `cli.mjs`, `root.mjs`, `cursor.mjs`, `machine.mjs`, `blocks.mjs`,
  `commands/{state,orders,doctrine,doctor}.mjs`
- `engine/tests/` — harness plus the parity, degrade, and duplicate-primitive suites
- Hook shims rewritten to `exec` the engine, retaining their exit-0 degrade

## Non-Goals

- No behaviour changes. A parity failure is a bug in the port, not an improvement.
- No deletion of bash originals in this feature.
- No TypeScript, no bundler, no dependencies.

## Open Questions

None blocking. The bash originals' removal is sequenced per superseding feature,
recorded in each of those features' Scope rather than here.
