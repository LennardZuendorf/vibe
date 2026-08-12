# js-core/8 fix round 1, part A — scan-and-suite half

Branch worktree base: `8c97eff` (`claude/vibe-toolkit-review-ky9ici`).
Charge: review-task-8 Critical, Important 2, Important 3, Important 4, Minor 8, Minor 9,
plus the two comment overclaims from the js-core/5 re-review.

**All discharged.** Nothing was deferred except the two limits stated under
*Known limits* below, which are stated rather than papered over.

---

## What changed

| File | Change |
|---|---|
| `flow/engine/tests/primitives.test.mjs` | Rewritten. Ingredient-based invariant + real comment tokenizer + recursive tree scope + 18-mutant × 3-site corpus. 6 tests → 34. |
| `flow/engine/tests/parity.test.mjs` | 30 `× jq` legs and the `KNOWN DIVERGENCE` pin now `skip()` instead of reporting vacuous `ok`; `noJqShimDir()` asserts jq is really unreachable; module-scope tmpdirs cleaned on exit. |
| `flow/engine/cursor.mjs` | `cursorPath()` exported (was private). Behaviour unchanged. |
| `flow/engine/commands/doctor.mjs` | `checkCursor()` gets the cursor path from `cursorPath()` instead of re-deriving `joinMaybe(vibeDir, 'state.json')`. Guarded identically to `checkMachine()`'s `machinePath()` call, so a non-string `vibeDir` still degrades to "absent". Byte-identical output. |
| `flow/engine/root.mjs` | `pluginVibeDir()` comment corrected (js-core/5 re-review). One added comment on `resolveProjectCursorDir()` explaining why it is *not* routed through `cursorPath()`. No behaviour change. |
| `flow/engine/commands/orders.mjs`, `commands/doctrine.mjs` | `jqAltRaw()` comments corrected (js-core/5 re-review). Comment-only. |

Reserved files (`install.sh`, `flow/engine/tests/run.mjs`, `flow/tests/adapters/run.sh`,
`tests/run.sh`, `.github/workflows/ci.yml`) — **untouched**. `flow/state.json` — untouched
(absent in this worktree; main checkout still `cksum 39839891 104`). `.spec/**` — untouched.

---

## Finding 1 + Finding 2 (Critical / Important) — the invariant is inverted

The old scan matched call **shapes**: `JSON.parse` within a 300-character forward window of
`readFileSync` and a filename literal, plus two hand-written path-join regexes. Adding a
regex per evasion is the same defect with a longer list, and sound proximity matching needs
a parser this engine will not take (R5, zero dependencies).

So the invariant now forbids the **ingredients** rather than detecting the shape. A module
that never names the file cannot read it; a module that never names `CLAUDE_PROJECT_DIR`,
`.agents`, `.spec`/`.git`, `process.cwd` or `import.meta.url` has no leg of `resolveRoot()`
left to re-derive; a module that never writes `<!--` cannot re-implement `extractBlock()`.

Eight ingredients, each with its owning module:

| ingredient | pattern | owner |
|---|---|---|
| `cursor-file` | `state.json` (incl. regex-escaped `state\.json`) | `cursor.mjs` |
| `machine-file` | `state-machine.json` (incl. escaped) | `machine.mjs` |
| `project-dir-env` | `CLAUDE_PROJECT_DIR` — any spelling | `root.mjs` |
| `vibe-layout` | `.agents` | `root.mjs` |
| `root-markers` | `.spec` / `.git` as path tokens | `root.mjs` |
| `cwd-fallback` | `process.cwd` | `root.mjs` |
| `self-location` | `import.meta.url` | `root.mjs` |
| `marker-grammar` | `<!--` / `-->` | `blocks.mjs` |

Note the spellings this defeats *by construction*: aliased imports, split read-then-parse,
paths in variables, template literals, `path.resolve` vs `path.join`, destructured or
aliased `process.env`, and `readJson()` reuse — none of them can avoid naming the
ingredient somewhere in the module.

### The comment tokenizer

Ingredient bans cannot see prose: these modules quote every banned ingredient at length in
their doc comments, on purpose. `stripComments()` is a real tokenizer over string literals,
template literals (including code inside `${}`), regex literals, and both comment forms;
comments are replaced with spaces (newlines preserved) so violation line numbers stay true.

A naive per-line cut at the first `//` is **not** sufficient and is pinned as such:
`hook.mjs`'s `/(^|\/)src\//` regex literal ends in `\//`, which contains the sequence `//`,
so the naive cut swallows the rest of that line — and any violation living there. Test
`the stripper is load-bearing` asserts the tokenizer keeps that line and that the naive cut
corrupts it. Eight stripper tests, including a whole-tree property check that the stripper
is length-preserving, line-count-preserving and idempotent on every real engine module.

---

## Finding 3 (Important) — scope

`scanEngineTree()` walks **every** `.mjs` under `flow/engine/`, recursively.
In scope today (11 files): `blocks.mjs`, `cli.mjs`, `cursor.mjs`, `json.mjs`, `machine.mjs`,
`root.mjs`, `commands/{doctor,doctrine,hook,orders,state}.mjs`. The suite asserts
`cli.mjs`, `blocks.mjs` and `commands/hook.mjs` are in scope and that `tests/` is the only
exclusion.

All five R1 primitives now have coverage: root resolver (4 ingredients), cursor reader,
cursor **writer**, machine loader, block extractor. Three of those had none before.

---

## Evasion table — before / after, mutation-proved

18 mutants × 3 plant sites = 54 cells. The "before" column runs the **verbatim** scan logic
from `8c97eff` (transcribed into a harness); both columns run the same mutant corpus,
imported from the test file itself. Every cell is a real planted mutation against a scratch
copy of the engine, scanned, then discarded.

| evasion | primitive | before `commands/` | before `cli.mjs` | before `commands/sub/` | after `commands/` | after `cli.mjs` | after `commands/sub/` |
|---|---|---|---|---|---|---|---|
| M1 nested read+parse | cursor reader | CAUGHT | EVADED | EVADED | CAUGHT | CAUGHT | CAUGHT |
| M2 split read-then-parse (`json.mjs`'s own style) | cursor reader | EVADED | EVADED | EVADED | CAUGHT | CAUGHT | CAUGHT |
| M3 aliased `readFileSync` import | cursor reader | EVADED | EVADED | EVADED | CAUGHT | CAUGHT | CAUGHT |
| M4 path held in a variable | cursor reader | EVADED | EVADED | EVADED | CAUGHT | CAUGHT | CAUGHT |
| M15 `readJson(path.join(...))` | cursor reader | EVADED | EVADED | EVADED | CAUGHT | CAUGHT | CAUGHT |
| M16 alias + filename const + template literal | cursor reader | EVADED | EVADED | EVADED | CAUGHT | CAUGHT | CAUGHT |
| M17 duplicate cursor **writer** | cursor writer | EVADED | EVADED | EVADED | CAUGHT | CAUGHT | CAUGHT |
| M5 raw `path.join` machine path | machine loader | CAUGHT | EVADED | EVADED | CAUGHT | CAUGHT | CAUGHT |
| M6 nested call inside join args | machine loader | EVADED | EVADED | EVADED | CAUGHT | CAUGHT | CAUGHT |
| M7 template-literal machine path | machine loader | EVADED | EVADED | EVADED | CAUGHT | CAUGHT | CAUGHT |
| M8 `path.resolve` instead of `path.join` | machine loader | EVADED | EVADED | EVADED | CAUGHT | CAUGHT | CAUGHT |
| M9 `process.env.CLAUDE_PROJECT_DIR` | root resolver | CAUGHT | EVADED | EVADED | CAUGHT | CAUGHT | CAUGHT |
| M10 destructured `process.env` | root resolver | EVADED | EVADED | EVADED | CAUGHT | CAUGHT | CAUGHT |
| M11 aliased `process.env` | root resolver | EVADED | EVADED | EVADED | CAUGHT | CAUGHT | CAUGHT |
| M18b duplicate root resolver (marker search) | root resolver | EVADED | EVADED | EVADED | CAUGHT | CAUGHT | CAUGHT |
| M19 re-derived `.agents/skills/vibe` join | root resolver | EVADED | EVADED | EVADED | CAUGHT | CAUGHT | CAUGHT |
| M20 self-relative from `import.meta.url` | root resolver | EVADED | EVADED | EVADED | CAUGHT | CAUGHT | CAUGHT |
| M18a duplicate block extractor | block extractor | EVADED | EVADED | EVADED | CAUGHT | CAUGHT | CAUGHT |

**before: 3/54 caught · after: 54/54 caught.**

The before column reproduces the reviewer's own results exactly: only M1, M5 and M9 in
`commands/*.mjs`, matching the reported "one syntactic spelling, one directory".

Two further discriminating tests:

* a **second** `import.meta.url` line in `cli.mjs` is caught — the cli.mjs allowance is a
  one-LINE waiver, not a file exemption, so a smuggled root resolver there still trips;
* **editing** a waived line un-waives it (a trailing comment does not, since the trimmed
  code is unchanged; changing the code does).

And a real-tree mutation proof outside the suite: appending M2 to the real
`flow/engine/commands/orders.mjs` turned `primitives.test` from `34 passed, 0 failed` to
`14 passed, 20 failed`; reverting returned it to `34 passed, 0 failed`, `git status` clean.

---

## False positives — the scan passes clean on the real tree, with these exemptions

`scanEngineTree()` reports **0 violations** on the real tree. Every exemption is enumerated,
individually reasoned, and itself asserted. Nothing is a blanket file exemption.

**Directory exemption (one).** `flow/engine/tests/` is excluded. Test files legitimately
name `state.json` and build `.agents/skills/vibe` fixture layouts — they are fixtures, not
shipped engine code. The clean-tree test asserts this is the only exclusion.

**Owner exemptions (eight).** The module that defines a primitive is unrestricted for its
own ingredient (`machinePath()` must build the machine path inside `machine.mjs`;
`resolveProjectCursorDir()` must know the layout inside `root.mjs`). A test asserts every
owner still actually carries its ingredient, so a stale owner entry fails.

**Line waivers (22 lines).** Each is an exact trimmed source line plus a reason code; a
second occurrence anywhere, or an edit to the line, is a violation again. A test asserts
every waiver is still used, so the list cannot rot into a blanket exemption, and a test
asserts every waiver names a known reason code and a live ingredient.

| file | count | reason code | what they are |
|---|---|---|---|
| `commands/hook.mjs` | 7 | `bash-sniffer` | the guard's warn-only sniffer patterns and labels, mirroring `detect-context.sh`'s guarded-path classes — matched against someone else's shell command, never a path this engine builds |
| `commands/hook.mjs` | 2 | `hook-root-literal` | `vibeLogDir()` and `evidRel`: the original `.sh` hooks' own `$ROOT`-relative literals for the warnings log and evidence receipts (js-core/7 review sanctioned this) |
| `commands/hook.mjs` | 2 | `oracle-text` | the two "abort with: bash …/set-state.sh idle" message lines |
| `commands/doctor.mjs` | 4 | `oracle-text` | `state-machine.json` inside the three `warn()`/`ok()` message strings, plus the "no doctrine coverage" line |
| `commands/doctor.mjs` | 1 | `marker-presence-probe` | the opener-only `includes('<!-- vibe:doctrine -->')` check — `extractBlock()` returns `undefined` for an opener with no closer, so routing it through `blocks.mjs` would change behaviour |
| `commands/doctor.mjs` | 1 | `layout-name-probe` | `path.basename(agentsDir) === '.agents'` in `rootForReport()` — a name comparison walking structure off an already-resolved primitive |
| `commands/orders.mjs` | 2 | `oracle-text` | the `GENERIC_FALLBACK` string (one line, two ingredients) |
| `cli.mjs` | 1 | `cli-self-dispatch` | `fileURLToPath(import.meta.url)` for locating its own `commands/` dir |
| `root.mjs` | 1 | `spec-sanctioned-exemption` | `resolveProjectCursorDir()`'s `existsSync(path.join(candidate, 'state.json'))` |

Of these 22, **21 cannot read or construct anything** — they are message text, sniffer
patterns matched against foreign input, or name comparisons. The two `hook-root-literal`
lines are the honest exception: they *are* real layout literals, sanctioned by the js-core/7
review because they mirror the original `.sh` hooks' own paths rather than a vibe primitive's
resolution. The per-line waiver is precisely what stops a second one appearing silently.

**Two exemptions I removed instead of granting.**

* `doctor.mjs`'s `joinMaybe(vibeDir, 'state.json')` was a genuine second place that knew the
  cursor filename. Fixed at the source: `cursorPath()` is now exported from `cursor.mjs`
  (the same pattern `machinePath()` already sets) and `doctor.mjs` calls it.
* `root.mjs`'s equivalent line was tried the same way and **reverted**: `root.test.mjs`
  copies `root.mjs` alone into synthetic install layouts, so importing `cursor.mjs` would
  drag `json.mjs` in behind it and broke 8 tests (`ERR_MODULE_NOT_FOUND`). `root.mjs` is the
  self-location primitive and must stay importable standalone, so this one is a one-line
  waiver with that reason recorded in both the code and the waiver table.

---

## Finding 4 (Important) — vacuous green with jq absent

Reproduced first. On the jq-stripped PATH, pre-fix:

```
parity.test.mjs  →  61 passed, 0 failed, 0 skipped
  ok    parity.test.mjs :: KNOWN DIVERGENCE: cursor.flow = 5 (number) …
```

The pin reported `ok` while its body did nothing, and all 30 `× jq` legs ran on the same
jq-less PATH as their `no-jq` twins, asserting nothing new.

Post-fix, same PATH:

```
parity.test.mjs  →  30 passed, 0 failed, 31 skipped
  skip  parity.test.mjs :: KNOWN DIVERGENCE: … (requires jq on PATH — the divergence lives in jq's // truthiness)
```

31 = the 30 `× jq` matrix legs + the divergence pin. With jq present, unchanged at
`61 passed, 0 failed, 0 skipped` — the teeth are intact, only the counts stopped lying.

The 5 `state × jq` cases no longer silently downgrade to first-line-only comparison either:
`requireJqFor(mode)` skips the leg, so `mode === 'jq'` now *implies* jq is present and the
full-stdout branch always runs.

### Minor 8 — a jq-leaky `mkshim` body is now loud

`noJqShimDir()` asserts its own postcondition directly:

```js
const probe = runCommand(path.join(dir, 'bash'), ['-c', 'command -v jq'], { env: { PATH: dir } });
assert(probe.code !== 0 && probe.stdout.trim() === '', …);
```

Mutation-proved without touching the reserved `flow/tests/adapters/run.sh`: the same
assertion expression was run against the real `mkshim jq` output and against a deliberately
leaky shim dir that still exposes `jq`.

```
real mkshim jq dir  -> assertion PASSES (jq unreachable)
leaky mkshim body   -> assertion FAILS LOUDLY (correct)
```

Previously a leak was caught only incidentally by 5 `doctor … x no-jq` legs; the other 20
no-jq legs would have run *with* jq and reported green.

### Minor 9 — `DOCTOR_HOME` leak

Both module-scope tmpdirs (`DOCTOR_HOME` and the memoized no-jq shim dir) are removed in a
`process.on('exit')` handler. Verified by listing `/tmp/vibe-parity-doctor-home-*` before
and after a full-suite run: **no new directories**.

---

## js-core/5 re-review — the two overclaiming comments (comment-only, no behaviour change)

**`jqAltRaw()`** in `commands/orders.mjs` and `commands/doctrine.mjs`. The comments now state
the real scope: they reproduce jq's `//` truthiness and 2-space pretty-print, **not** the
oracle's full capture pipeline. `JSON.stringify` re-serializes numbers (`1.0`→`1`, `1e2`→`100`,
`-0`→`0`, integers past 2^53 lose precision); integer-like object keys are reordered to the
front by JS property ordering where jq preserves document order; and a string `feature`
ending in newlines keeps them, where bash `$(...)` would strip them. All pre-existing or
improved versus pre-fix, all reachable only via a hand-edited cursor — but divergences, and
now named as such.

**`pluginVibeDir()`** in `root.mjs`. The claim that it uses "the SAME probe" as the oracle
and that "the two checks never disagree" is removed. It additionally requires
`basename(skillsDir) === 'skills'`, strictly narrower than the oracle's name-agnostic
`$SKILL_PARENT/vibe/SKILL.md`. The demonstrated divergence is recorded in the comment: an
engine at `<X>/mySkills/vibe/engine` fails this probe, so `resolveVibeDir()` falls through
and the engine prints the **project's** doctrine while the oracle prints the plugin's own.
Every shipped layout (`skills/`, `.agents/skills/`, `.claude/skills/`) is named `skills`, so
no shipped configuration reaches the gap — the behaviour deliberately stays.

---

## Known limits (stated, not papered over)

1. **String concatenation and computed names.** A whole-file literal ban cannot see through
   `'state' + '.json'`, `['state','json'].join('.')`, or a name built at runtime. Closing
   that requires an AST, which requires a dependency R5 forbids. This is the strongest
   *sound* subset, and the test file's header says so.
2. **`hook.mjs`'s two `hook-root-literal` waivers** are real layout literals, not text. They
   are sanctioned by the js-core/7 review and pinned per-line, but they are the one place
   where a waiver covers something that could in principle grow into a re-derivation.

Neither is new in this change; both were previously invisible rather than bounded.

---

## Spec drift to report (not edited — `.spec/**` is another state's write surface)

1. **`tech.md` § Contract, `cursor.mjs` block** lists `readCursor(vibeDir)` and
   `writeCursor(vibeDir, …)`. It should also list `cursorPath(vibeDir) -> string` — the one
   place `state.json`'s path is joined — mirroring how the `machine.mjs` block already lists
   `machinePath()`. The export was added by this fix.
2. **`plan.md`'s js-core/8 step text says `engine/commands/*`** while `product.md`'s R1
   scenario says "any file **under `engine/`**". The scan now implements R1 (the requirement
   of record); the plan step text is the stale half and should be reconciled at compound.
3. **`tech.md` §"Why the engine lives under `flow/`"** and `root.mjs`'s `pluginVibeDir()`
   are now consistent, but `tech.md`'s Contract note on `resolveVibeDir()` ("recognising
   BOTH payload layouts") could usefully record that the plugin leg additionally requires
   the parent directory to be named `skills`, which the oracle does not.

---

## Verification

| environment | result |
|---|---|
| `node flow/engine/tests/run.mjs` | **332 passed, 0 failed, 0 skipped** (was 304) |
| `CLAUDE_PROJECT_DIR=/home/user/vibe node flow/engine/tests/run.mjs` | **332 passed, 0 failed, 0 skipped** |
| `node flow/engine/tests/run.mjs` with jq stripped from `PATH` | **288 passed, 0 failed, 44 skipped** (44 = 31 new honest skips + 13 pre-existing) |
| `bash flow/tests/run.sh` | 221 passed, 0 failed |
| `bash tests/run.sh` | `PASS spec / PASS flow / PASS adapters / PASS engine`, rc 0 |
| `bash .agents/skills/spec/scripts/validate.sh` | 0 errors, 0 warnings, rc 0 |
| `cksum /home/user/vibe/flow/state.json` | `39839891 104` — unchanged; the file does not exist in this worktree and was never created |

Test count 304 → 332: `primitives.test.mjs` 6 → 34 (+28), `parity.test.mjs` unchanged at 61.
