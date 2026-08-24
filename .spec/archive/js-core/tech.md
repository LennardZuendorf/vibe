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
what keeps "packaging is plugin-runtime's decision" an honest deferral. Corrected
during `js-core/7`, which the original top-level layout would have blocked
outright: the shims resolve
`$CLAUDE_PROJECT_DIR/.agents/skills/vibe/engine/cli.mjs`, and nothing shipped an
engine there.

`resolveVibeDir()`'s self-relative leg is a **separate** question, and the move
does not change it here. That leg expects `…/vibe/engine`; this repo's directory
is named `flow`, so resolution still falls through to `root + .agents/skills/vibe`
in the dogfood repo and matches self-relatively only in an install target, where
the directory really is `vibe`. The fallback is correct behaviour, not a bug —
but it does mean the primary leg is exercised by fixtures and real targets and
never by this repo's own runtime. Treat any change to it as untested-by-dogfood
and pin it in `flow/engine/tests/root.test.mjs`. (An earlier revision of this
note claimed the move made that leg fire here; it does not.)

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
// The plugin leg additionally requires the parent directory be named `skills`,
// so an arbitrary <x>/vibe/engine does not satisfy it.
resolveSkillsDir(opts?) -> string          // parent of the vibe dir; finds sibling skills
resolveProjectCursorDir(opts?) -> string | undefined
// doctrine-ONLY. Reproduces doctrine.sh's CLAUDE_PROJECT_DIR-first cursor rule,
// gated on that state.json existing. Single-sources the `.agents/skills/vibe`
// layout constant so no command re-derives it. No other command has this axis.

// cursor.mjs — takes the VIBE DIR, not the repo root
cursorPath(vibeDir) -> string   // the ONE place the cursor filename is spelled
readCursor(vibeDir) -> {flow, phase, feature, updated, state}  // state = "<flow>.<phase>"
                     | {state: "idle", ...}                     // when absent
                     // throws CursorParseError when present but malformed.
                     // Missing flow/phase in otherwise-valid JSON soft-default
                     // to "idle" per field, matching jq's `// "idle"`.
writeCursor(vibeDir, {flow, phase, feature})                    // atomic; temp unlinked on failure

// machine.mjs
machinePath(vibeDir) -> string  // the ONE place the machine filename is spelled
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

The matrix also covers the two hooks that can **block** — guard (`PreToolUse`)
and gate (`Stop`). Their bash oracles are frozen in
`flow/engine/tests/oracles/` rather than left in place, because porting a script
overwrites the very thing that proves the port: 210 guard + 392 gate
comparisons per run, on a real git repo, comparing rc + stdout + stderr +
warnings-log bytes. Where the oracle disagrees with itself across its own jq and
sed legs, the engine follows the **fail-safe** leg — losing one turn of
enforcement is recoverable, wedging a session in a block loop is not — and the
resulting divergence from the other leg is pinned in both modes with a control.

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

The scan enforces two mechanical clauses, both stronger than "don't duplicate":

1. **No module may obtain a primitive's *location* by any means** unless it is an
   allowlisted consumer — banned as *ingredients* (the filename literals, the
   layout constants, `CLAUDE_PROJECT_DIR`, `import.meta`, `process.cwd`, and the
   path-helper identifiers), matched against the whole comment-stripped file so
   line wrapping cannot split a token pair. The allowlist pins exact lines *and
   occurrence counts*, so a second use in an allowlisted file is still a
   violation, and re-export laundering is unwaivable.
2. **No module may reach outside the scanned source set.** Every
   specifier-shaped literal resolves against its own module's directory and must
   land inside the scanned tree, which closes that tree under module resolution
   by induction. Because the rule names no syntax, static `import`, dynamic
   `import()`, `require`, and `createRequire` are all covered without being
   enumerated — the test tree is not a back door.

The residual gap needs an AST plus constant folding, which R5's zero-dependency
rule forbids; those cases are pinned as executable tests that assert they *do*
evade, so the documented limit is the measured one.

Root resolution order is `CLAUDE_PROJECT_DIR` → self-relative → upward marker
search → cwd. Self-relative precedes marker search because install targets
frequently have neither `.git` nor `.spec`. That ordering is spec-mandated and
pinned by test — swapping the legs must fail the suite.

Co-located tests are source-only: `install.sh` scrubs every `tests/` directory
from the payload at any depth, and the adapter suite asserts structurally that
no test artifact reaches an install target.
<!-- /merge -->

## Performance Budget

Hooks run on every turn and every tool call, so engine startup is the budget
that matters. Target: under 50 ms cold for `orders` on a warm filesystem —
achievable with no dependencies, four small module loads, and a single JSON
parse. `parity.test.mjs` records wall time per command and fails above 150 ms,
leaving headroom without pinning a flaky threshold.

**As delivered:** `orders` measures min 47.9 ms / median ~50 ms on an idle
runner — inside the 150 ms ceiling with roughly 3× headroom, but *level with*
the 50 ms target rather than under it. The target is aspirational; the 150 ms
ceiling is the enforced contract. The assertion calibrates each run against a
bare `node -e 0` spawn (27 ms idle, 63–71 ms contended) and **skips with the
measured number** when the runner cannot deliver a usable measurement, so a
loaded CI box never produces either a silent pass or an unactionable red.

## Open Questions

None. Packaging (npm name, plugin payload path) is plugin-runtime's decision;
`VIBE_ENGINE` exists as the indirection so js-core does not need the answer.
