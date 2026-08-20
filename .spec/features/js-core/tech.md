---
type: feature-tech
feature: js-core
sibling: product.md
parent: ../../tech.md
updated: 2026-08-10
---

# js-core — Technical

How the engine is laid out, what each module owns, and how parity is proven.

## Files

```text
engine/
├── cli.mjs                 # arg parse + dispatch; the only process entry
├── root.mjs                # resolveRoot()  — self-relative, marker fallback
├── cursor.mjs              # readCursor() / writeCursor()  — the only JSON cursor I/O
├── machine.mjs             # loadMachine()  — state-machine.json + lookups
├── blocks.mjs              # extractBlock() — the one marker grammar
├── json.mjs                # readJson()/writeJsonAtomic() — mktemp + rename
└── commands/
    ├── state.mjs           # get | set  (writer only; gating lands in machine-teeth)
    ├── orders.mjs          # resolve current state's orders
    ├── doctrine.mjs        # emit the doctrine block
    └── doctor.mjs          # health report, always exit 0

engine/tests/
├── run.mjs                 # harness: assert helpers, sandbox fixtures
├── parity.test.mjs         # R2 — bash vs engine, with and without jq
├── degrade.test.mjs        # R3, R4 — fresh target, absent node
└── primitives.test.mjs     # R1, R6 — duplicate-primitive scan, error paths

.claude/hooks/*.sh          # rewritten shims (3 lines each)
```

## Contract — API

```js
// root.mjs
resolveRoot(opts?: {cwd?: string}) -> string
// Order: CLAUDE_PROJECT_DIR -> self-relative (import.meta.url) -> upward
// .spec/.git search -> cwd. Self-relative precedes marker search because a
// fresh install target has neither marker (see the stranger-eval lesson).

// cursor.mjs
readCursor(root) -> {flow, phase, feature, updated, state}   // state = "<flow>.<phase>"
                 | {state: "idle", ...}                       // when absent
                 // throws CursorParseError when present but malformed
writeCursor(root, {flow, phase, feature}) -> void             // atomic, preserves shape

// machine.mjs
loadMachine(root) -> {states, flows, phases, gates, initial, version, style}
stateOf(machine, key) -> stateRecord | undefined

// blocks.mjs
extractBlock(text, id) -> string | undefined
// ONE grammar. Accepts the legacy asymmetric closer during js-core so ported
// output stays byte-identical; content-layer migrates authors to the single form.
```

Every command receives `{root, cursor, machine}` resolved once by `cli.mjs` and
never re-reads them. This is what the duplicate-primitive scan enforces.

## Implementation Detail

**Atomic writes.** `writeJsonAtomic` writes to a sibling temp file then
`rename`s, matching the bash `mktemp` + `mv -f` behaviour so a crash mid-write
never truncates the cursor. The temp file is created in the target's own
directory so the rename stays same-filesystem.

**Byte parity.** The bash writers hand-roll a `printf` that claims byte-identity
with `jq`'s pretty-print. The engine reproduces that exact serialization — two
space indent, key order `flow, phase, feature, updated`, trailing newline —
rather than relying on `JSON.stringify` defaults. `parity.test.mjs` compares
`cksum` of both writers' output across the fixture matrix.

**The parity matrix.** Every ported command × {cursor absent, `idle`,
`feature.impl` with feature, `feature.impl` with null feature, `quick.verify`} ×
{jq present, jq absent}. The jq-absent leg uses the existing `mkshim` helper
from `flow/tests/adapters/run.sh` so both suites shim identically.

**Error taxonomy.** `CursorParseError` and `UnknownStateError` exit 1 with a
one-line stderr naming the file and the cause. Everything else in `doctor`
degrades to a warn line and exit 0, preserving the never-end-the-session rule.

**Hook shims.** Each becomes:

```bash
#!/usr/bin/env bash
command -v node >/dev/null 2>&1 || exit 0
exec node "${VIBE_ENGINE:-$CLAUDE_PROJECT_DIR/.agents/skills/vibe/engine}/cli.mjs" hook "<name>"
```

`exec` replaces the shell so exit codes pass through unchanged — which is what
keeps the guard's exit 2 and the gate's exit 2 working. The `command -v node`
guard is what satisfies R4.

**What is NOT ported here.** `detect-context.sh` stays bash for this feature:
its policy becomes `policy.json` in content-layer, and porting it twice would be
waste. `merge-agents.sh`, `merge-settings.sh`, `regen-active-rules.sh`,
`check-skills.sh`, and `validate-state.sh` likewise stay until their superseding
feature. The guard shim therefore still calls the bash `detect-context.sh`
during js-core.

<!-- merge -->
### Engine module boundaries

The engine's primitives are singular by contract, not by convention: one root
resolver, one cursor reader/writer, one machine loader, one block extractor,
one atomic JSON writer. Commands are pure functions over
`{root, cursor, machine}` resolved once at dispatch. A duplicate-primitive scan
in the suite fails the build when a command re-derives any of them.

Root resolution order is `CLAUDE_PROJECT_DIR` → self-relative → upward marker
search → cwd. Self-relative precedes marker search because install targets
frequently have neither `.git` nor `.spec`.
<!-- /merge -->

## Performance Budget

Hooks run on every turn and every tool call, so engine startup is the budget
that matters. Target: under 50 ms cold for `orders` on a warm filesystem —
achievable with no dependencies, four small module loads, and a single JSON
parse. `parity.test.mjs` records wall time per command and fails above 150 ms,
leaving headroom without pinning a flaky threshold.

## Open Questions

None. Packaging (npm name, plugin payload path) is plugin-runtime's decision;
`VIBE_ENGINE` exists as the indirection so js-core does not need the answer.
