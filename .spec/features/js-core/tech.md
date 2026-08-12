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
flow/engine/                # INSIDE the vibe skill dir — see the note below
├── cli.mjs                 # arg parse + dispatch; the only process entry
├── root.mjs                # resolveRoot / resolveVibeDir / resolveSkillsDir /
│                           # resolveProjectCursorDir
├── cursor.mjs              # readCursor() / writeCursor()  — the only JSON cursor I/O
├── machine.mjs             # loadMachine() / machinePath() / stateOf()
├── blocks.mjs              # extractBlock() — the one marker grammar
├── json.mjs                # readJson()/writeJsonAtomic() — temp + rename
└── commands/
    ├── state.mjs           # get | set  (writer only; gating lands in machine-teeth)
    ├── orders.mjs          # resolve current state's orders
    ├── doctrine.mjs        # emit the doctrine block
    ├── doctor.mjs          # health report, always exit 0
    └── hook.mjs            # the four hook entrypoints

flow/engine/tests/          # harness + per-module suites, oracle-spawning
.claude/hooks/*.sh          # shims: `command -v node || exit 0` then `exec node …`
```

**Why the engine lives under `flow/`, not at the repo root.** `install.sh:517`
copies `.agents/skills/vibe` (a symlink to `flow/`) into the target, and
`plugin/skills/vibe` symlinks to the same place. Putting the engine inside that
directory means both carriers ship it with **no packaging change** — which is
what keeps "packaging is plugin-runtime's decision" an honest deferral. It also
means `resolveVibeDir()`'s self-relative leg resolves correctly *in this repo*:
with the engine at the repo root its parent was the root, not the vibe dir, so
the dogfood repo silently exercised the fallback while only fixtures covered the
primary path — the privileged-target lesson, inverted. Corrected during
`js-core/7`, which the original top-level layout would have blocked outright.

## Contract — API

```js
// root.mjs — four resolvers, each with one job
resolveRoot(opts?) -> string
// Repo root, for genuinely root-scoped things (.spec/, .gitignore).
// Order: CLAUDE_PROJECT_DIR -> self-relative (import.meta.url) -> upward
// .spec/.git search -> cwd. Self-relative precedes marker search because a
// fresh install target has neither marker (see the stranger-eval lesson).

resolveVibeDir(opts?) -> string
// The skill dir holding state.json + state-machine.json. Self-relative from
// the engine's own location, recognising BOTH payload layouts:
// <root>/.agents/skills/vibe/engine (installed) and <plugin>/skills/vibe/engine
// (per-user plugin). Deliberately IGNORES CLAUDE_PROJECT_DIR once that chain
// validates, so a per-user plugin reads its own skill, not the project's.
resolveSkillsDir(opts?) -> string          // parent of the vibe dir; finds sibling skills
resolveProjectCursorDir(opts?) -> string | undefined
// doctrine-ONLY. Reproduces doctrine.sh's CLAUDE_PROJECT_DIR-first cursor rule,
// gated on that state.json existing. Single-sources the `.agents/skills/vibe`
// layout constant so no command re-derives it. No other command has this axis.

// cursor.mjs — takes the VIBE DIR, not the repo root
readCursor(vibeDir) -> {flow, phase, feature, updated, state}  // state = "<flow>.<phase>"
                     | {state: "idle", ...}                     // when absent
                     // throws CursorParseError when present but malformed.
                     // Missing flow/phase in otherwise-valid JSON soft-default
                     // to "idle" per field, matching jq's `// "idle"`.
writeCursor(vibeDir, {flow, phase, feature})                    // atomic; temp unlinked on failure

// machine.mjs
loadMachine(vibeDir) -> {states, flows, phases, gates, initial, version, style}
stateOf(machine, key) -> stateRecord | undefined   // own-property guarded:
// a prototype key such as `constructor` must not resolve, matching the oracle

// blocks.mjs
extractBlock(text, id) -> string | undefined
// ONE grammar. Returns undefined on a missing closer — a deliberate divergence
// from the oracle's sed range, which leaks the file tail. content-layer
// migrates authors to the single marker form.
```

Commands resolve their dirs once at dispatch and use these functions; none
re-derives a primitive, parses cursor or machine JSON directly, or hardcodes a
layout path. That is what the duplicate-primitive scan enforces.

Every command's runnable function returns `{code, stdout, stderr}`, never throws
— including on non-string and non-array argument shapes — and never touches
`process.std*`. The hook shims call these directly, so an exception there would
surface as a broken hook rather than a handled exit code.

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
