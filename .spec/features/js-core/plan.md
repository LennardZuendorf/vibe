---
type: feature-plan
feature: js-core
sibling: tech.md
parent: ../../plan.md
updated: 2026-08-10
---

# js-core — Implementation Plan

**Requirements:** [product.md](product.md) · **Architecture:** [tech.md](tech.md)
**Parent plan:** [../../plan.md](../../plan.md) — Feature Sequence row 12

**Feature gate:** js-core is DONE when every ported command is byte-identical to
its bash original across the parity matrix with and without `jq`, the shims
degrade cleanly without Node, and CI runs the engine suite alongside the three
existing ones.

## Problem Frame

Four primitives are reimplemented 5 / 5 / 4 / 3 times across the bash tree, each
with a hand-maintained jq-optional twin. Every feature after this one would pay
that tax twice. js-core collapses the primitives to one implementation each and
proves the collapse by parity rather than by review.

The risk is a silent behaviour change during the port. The mitigation is that
parity is the acceptance test, not a nice-to-have: the bash originals stay in
the tree and are executed by the suite as the oracle.

## Requirements Trace

Every unit below cites the R-IDs it satisfies. Do not renumber.

| ID | Requirement | Units |
|---|---|---|
| R1 | One implementation per primitive | js-core/2, js-core/8 |
| R2 | Byte-identical parity, with and without jq | js-core/3, js-core/4, js-core/5, js-core/6, js-core/8 |
| R3 | Correct resolution on a fresh, non-git target | js-core/2, js-core/7 |
| R4 | Absent Node degrades, never breaks | js-core/7 |
| R5 | No runtime dependencies | js-core/1, js-core/8 |
| R6 | Unknown input fails loudly, missing input degrades quietly | js-core/2, js-core/3 |

## Key Technical Decisions

- **Bash originals stay.** They are the parity oracle; deletion is sequenced into
  whichever later feature supersedes each script.
- **`detect-context.sh` is not ported here.** Its policy becomes `policy.json` in
  content-layer; porting it twice is waste. The guard shim still calls bash.
- **Serialization is reproduced, not inherited.** The engine emits the bash
  writer's exact cursor format rather than `JSON.stringify` defaults.
- **Self-relative root resolution precedes marker search**, per the stranger-eval
  lesson — install targets often have neither `.git` nor `.spec`.

## Global Constraints

Executors: read this section before starting any unit.

- Node 18+, ESM (`.mjs`), zero dependencies, no bundler, no TypeScript.
- No behaviour changes. If parity fails, fix the port — never adjust the oracle.
- Do not delete or edit any `flow/scripts/*.sh` in this feature.
- Every unit ends green on `bash tests/run.sh` *and* `node engine/tests/run.mjs`.
- Commits cite the unit ID (`js-core/3`).

## Unit IDs

Stable `js-core/n`. Dependencies are same-feature only.

### js-core/1 — Package skeleton and CLI dispatch

**Goal:** A runnable `vibe` entry point with subcommand dispatch and a test harness.
**Requirements:** R5
**Dependencies:** —
**Files:**
```
engine/cli.mjs           # arg parse, dispatch, error taxonomy, exit codes
engine/tests/run.mjs     # assert helpers, sandbox fixture builder, runner
package.json             # name, type: module, bin, engines: node >=18; no deps
```
**Steps:**
- [ ] Write a failing harness test asserting `cli.mjs --help` lists the four commands
- [ ] Implement dispatch with an unknown-subcommand path exiting non-zero and naming the input
- [ ] Add the sandbox fixture builder (temp repo with cursor + machine)
**Test scenarios:** unknown subcommand exits non-zero and names itself; `--help` lists commands; runner reports pass/fail counts and exits non-zero on failure.
**Verification:** `node engine/tests/run.mjs` green; `ls node_modules` absent.

### js-core/2 — Primitives: root, json, cursor, machine, blocks

**Goal:** The five singular primitives, each with its error path.
**Requirements:** R1, R3, R6
**Dependencies:** js-core/1
**Files:**
```
engine/root.mjs      # CLAUDE_PROJECT_DIR -> self-relative -> marker -> cwd
engine/json.mjs      # readJson, writeJsonAtomic (sibling temp + rename)
engine/cursor.mjs    # readCursor (absent -> idle, malformed -> throw), writeCursor
engine/machine.mjs   # loadMachine, stateOf
engine/blocks.mjs    # extractBlock — one grammar, legacy closer accepted
```
**Interfaces:** Produces the API in tech.md § Contract. Consumes nothing but `node:fs`/`node:path`.
**Steps:**
- [ ] Test-first: root resolves in a bare `mktemp -d` with no `.git`/`.spec`
- [ ] Test-first: absent cursor → `idle`; malformed cursor → `CursorParseError`
- [ ] Test-first: `extractBlock` returns undefined on a missing closer rather than the file tail
- [ ] Implement the five modules
**Test scenarios:** fresh non-git target resolves (R3); malformed cursor throws named error (R6); missing cursor is idle (R6); unterminated marker block does not leak the file tail — the bug the bash `sed` range has today.
**Verification:** `node engine/tests/run.mjs` green including the bare-directory fixture.

### js-core/3 — Port `state` (writer only)

**Goal:** `vibe state get|set` byte-identical to `set-state.sh`.
**Requirements:** R2, R6
**Dependencies:** js-core/2
**Files:**
```
engine/commands/state.mjs
```
**Steps:**
- [ ] Test-first: cursor bytes after `vibe state set feature.impl demo` equal bash's
- [ ] Implement feature carry-forward: new arg wins, `idle` clears, else preserve
- [ ] Reproduce the exact two-space serialization and key order
**Test scenarios:** set with feature; set with null feature; `idle` clears feature; unknown state rejected; carry-forward preserved across phases. Gate enforcement is explicitly out of scope — that is machine-teeth.
**Verification:** `cksum` of cursor after engine write equals after bash write, across the fixture matrix.

### js-core/4 — Port `orders`

**Goal:** `vibe orders` byte-identical to `orders.sh`, including fallbacks.
**Requirements:** R2
**Dependencies:** js-core/2
**Files:**
```
engine/commands/orders.mjs
```
**Test scenarios:** each of the 12 non-idle states; `idle` via the machine's inline string; absent skill file → generic fallback; `<feature>` interpolation; fresh non-git target returns idle orders not `state=unknown`.
**Verification:** parity against `bash flow/scripts/orders.sh` for all 13 states.

### js-core/5 — Port `doctrine`

**Goal:** `vibe doctrine` byte-identical to `doctrine.sh`.
**Requirements:** R2
**Dependencies:** js-core/2
**Files:**
```
engine/commands/doctrine.mjs
```
**Test scenarios:** doctrine block emitted verbatim; cursor line appended in the current format; missing block → silent exit 0. The `SessionStart` staleness fix is **not** applied here — moving the cursor line is inject-triggers' unit, and applying it now would break parity.
**Verification:** parity against `bash flow/scripts/doctrine.sh` with and without a cursor.

### js-core/6 — Port `doctor`

**Goal:** `vibe doctor` byte-identical to `doctor.sh`, still always exit 0.
**Requirements:** R2
**Dependencies:** js-core/2
**Files:**
```
engine/commands/doctor.mjs
```
**Test scenarios:** ok/warn matrix for each check ID; broken symlink; invalid cursor; absent dependency; always exits 0. Note the jq-presence check becomes vestigial for engine paths but its line is retained for parity and removed in plugin-runtime.
**Verification:** parity against `bash flow/scripts/doctor.sh` across the doctor fixture matrix.

### js-core/7 — Hook shims and degrade

**Goal:** Hooks call the engine and lose nothing when Node is missing.
**Requirements:** R3, R4
**Dependencies:** js-core/3, js-core/4, js-core/5
**Files:**
```
.claude/hooks/session-start-doctrine.sh
.claude/hooks/user-prompt-submit-inject.sh
.claude/hooks/stop-gate.sh
.claude/hooks/pre-tool-use-guard.sh   # still delegates policy to bash detect-context.sh
```
**Steps:**
- [ ] Test-first: with `node` shimmed away, each hook exits 0 and emits nothing
- [ ] Test-first: guard with `node` absent exits 0, not 2
- [ ] Rewrite each shim to `command -v node || exit 0` then `exec node …`
**Test scenarios:** all four hooks with Node present produce prior behaviour; all four with Node absent exit 0 silently; guard exit 2 still propagates through `exec` when Node is present.
**Verification:** `bash flow/tests/adapters/run.sh` green unchanged, plus the new absent-Node cases.

### js-core/8 — Parity matrix, primitive scan, CI

**Goal:** Make the guarantees mechanical rather than reviewed.
**Requirements:** R1, R2, R5
**Dependencies:** js-core/3, js-core/4, js-core/5, js-core/6, js-core/7
**Files:**
```
engine/tests/parity.test.mjs
engine/tests/primitives.test.mjs
tests/run.sh                     # add the engine suite to the aggregator
.github/workflows/ci.yml         # add engine suite + a macOS leg
```
**Steps:**
- [ ] Parity matrix: 4 commands × 5 cursor states × {jq, no-jq}, reusing `mkshim`
- [ ] Duplicate-primitive scan: fail if any `engine/commands/*` parses cursor or machine JSON directly
- [ ] Discriminating check: the scan must fail when a command is edited to re-read the cursor
- [ ] Add engine suite to `tests/run.sh` and CI; add a macOS job
**Test scenarios:** full matrix green; primitive scan fails on a deliberately duplicated reader (discriminating, per the uninstall-test lesson); CI green on ubuntu and macOS.
**Verification:** `bash tests/run.sh` runs four suites, all green, on both CI legs.

## Dependencies

| Unit | Blocks | Blocked by |
|---|---|---|
| js-core/1 | 2 | — |
| js-core/2 | 3, 4, 5, 6 | 1 |
| js-core/3 | 7, 8 | 2 |
| js-core/4 | 7, 8 | 2 |
| js-core/5 | 7, 8 | 2 |
| js-core/6 | 8 | 2 |
| js-core/7 | 8 | 3, 4, 5 |
| js-core/8 | — | 3, 4, 5, 6, 7 |

Units 3–6 are mutually independent and parallelise cleanly in handover mode.

## Progress

| Unit | Status |
|---|---|
| js-core/1 | NOT STARTED |
| js-core/2 | NOT STARTED |
| js-core/3 | NOT STARTED |
| js-core/4 | NOT STARTED |
| js-core/5 | NOT STARTED |
| js-core/6 | NOT STARTED |
| js-core/7 | NOT STARTED |
| js-core/8 | NOT STARTED |

## Open Questions

None blocking. npm package name and the plugin payload path are plugin-runtime
decisions; `VIBE_ENGINE` is the indirection that lets js-core proceed without them.
