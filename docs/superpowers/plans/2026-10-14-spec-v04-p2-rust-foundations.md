# Spec v0.4 Phase 2 — Rust Foundations Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Freeze today's bash and JS as permanent oracles, pin the Spec JSON v1 and exit-code contracts, and stand up the Cargo workspace (`vibe-core`, `vibe-spec`, `vibe-parity`) with a working differential parity harness over a generated spec corpus — rows F2, F3, F4.

**Architecture:** Oracles first (no Rust yet), then contracts validated by a zero-dependency Node script, then the workspace and core primitives, then the parity harness and corpus. Until F16 supplies a Rust candidate, parity runs the oracle against itself: that proves the corpus and oracles are deterministic and every corpus class meets its floor. A test-only Rust candidate (`parity-fixture`) proves the harness turns red on a planted mutant.

**Tech Stack:** bash 3.2+ (shellcheck-clean), Node ≥18 (zero deps), Rust 1.88 MSRV / edition 2024 (`clap`, `serde`, `serde_json` only).

**Spec:** `docs/superpowers/specs/2026-10-13-spec-v04-design.md` (§7 Rust foundations, §9 Testing). Program index: `docs/superpowers/plans/2026-10-13-spec-v04.md`. Contract and workspace detail: `.spec/tech-contracts.md`, `.spec/tech-rust.md`, `.spec/tech-spec.md` ("Root and discovery").

## Global Constraints

- Commits: Conventional Commits, imperative, lowercase, ≤50-char subject, no body, no footer.
- MSRV 1.88, pinned in `rust-toolchain.toml` and workspace `rust-version`; edition 2024; `resolver = "3"`.
- Dependency allowlist: `clap` (derive), `serde`, `serde_json`. No `toml`, `regex`, `tokio`, `tempfile`.
- Glob matching and atomic writes (tmp + rename, 0600 on unix) are hand-written in `vibe-core`.
- `vibe-core` holds no state names, spec paths, or channel names; no tool crate depends on another.
- Errors are `Result<T, vibe_core::Error>`; no `unwrap`/`expect` outside tests (`clippy::unwrap_used`, `clippy::expect_used` deny).
- Exit codes: 0 ok · 1 findings/refused · 2 usage · 3 no spec tree (`vibe-spec` queries only).
- Contracts: additive-only within a major; readers ignore unknown fields.
- Freeze an oracle before porting the script it proves. An oracle never changes; oracle bugs become `expected-divergence` pins fixed in Rust only.
- Every guard asserts its own population (a floor on what it examined).
- Run every affected suite and show its output before claiming a task done. Any task that adds or edits a tracked `*.sh` runs `bash tests/run.sh` and the CI shellcheck command.
- Never run timing suites (`node flow/engine/tests/run.mjs`, `bash tests/run.sh`) concurrently with other suites or `cargo` builds.
- Outward-facing actions (push, PR, issue edits) need an explicit owner yes at that moment.
- Scratch files go in `.superpowers/p2/` (gitignored), never `/tmp`.
- Never Edit/Write a file under `tests/parity/corpus/`: the write guard suffix-matches nested `.spec/` paths and blocks them, and the corpus is generated. Change `corpus_spec.rs` and regenerate.
- Tasks run with the flow cursor at `idle`, as in Phase 1: source edits warn there, and every `.spec/` edit goes through `strategy.spec`.

## Review Focus

1. **A sandbox walk-up escaping the sandbox.** `find_spec_root` and `check-drift.sh` walk up to `.git`. Without a `.git/` in the sandbox both sides of a self-check would read the same stray `.spec/` above it and still agree. Expected: a no-root case prints `No .spec/ directory found in current project`. Test: Task 8, `no_root_cases_stop_at_the_sandbox_git_dir`.
2. **Developer environment leaking into a run.** `VIBE_DESIGN_LINT=1` makes `validate.sh` call `npx` over the network; `SPEC_DIR` and `CLAUDE_PROJECT_DIR` redirect scripts. Expected: a run sees only `PATH`, `HOME`, `LC_ALL`, `TZ`. Test: Task 7, `runs_see_only_path_home_locale_and_timezone`.
3. **A Windows checkout rewriting frozen bytes.** CRLF conversion would change oracle, golden and corpus bytes. Expected: `.gitattributes` marks those trees `-text`. Test: Task 1, the `.gitattributes` assertion in `tests/oracles/run.sh`.
4. **A symlink inside a corpus case or oracle tree.** Copying would follow it out of the repo. Expected: staging refuses it with a clear error. Test: Task 7, `a_symlink_in_the_corpus_is_refused`.
5. **The corpus generator deleting a directory it did not write.** Expected: it refuses any existing directory without its `.generated` marker. Test: Task 8, `the_generator_refuses_a_directory_it_did_not_write`.

## Rulings (owner review)

These decide what the spec leaves open. Each lists its cost if wrong.

- **P2-R1 — Order.** The index says `F2 ∥ F3 → F4`, but F3's `vibe-parity` crate needs F4's workspace. This plan runs: Task 1 (F3 oracle freeze, no Rust) → Tasks 2–3 (F2) → Tasks 4–6 (F4) → Tasks 7–8 (F3 harness and corpus). Cost if wrong: none to the code; the row sequence in `.spec/plan.md` reads slightly differently.
- **P2-R2 — Spec JSON v1 shapes are new.** `tech-contracts.md` names the four queries but no fields. Every *required* field becomes permanent under additive-only, so required sets are minimal: `root {v, root, source}`, `lessons-for {v, tags, lessons[{title, rule, tags}]}`, `plan {v, feature, rows[{id, status}]}`, `feature {v, feature, layer, docs[{kind, path}]}`. `root.source` mirrors discovery steps 1–5 (`flag`, `env`, `config`, `docs-spec`, `dot-spec`), pinned by `contracts/run.mjs`. Spec JSON is a new `--json` output; the legacy `lessons-for.sh --format json` array stays byte-identical for parity (F16). Cost if wrong: a breaking change later needs `v: 2`.
- **P2-R3 — Contracts validate with Node.** `contracts/run.sh` uses a hand-written JSON Schema subset in Node (already a CI dependency). Unsupported keywords are errors, `additionalProperties` may only be absent or `true`. Cost if wrong: when F23 retires Node, the validator moves to Rust.
- **P2-R4 — Oracle set.** 24 tracked scripts (`spec/scripts` 7, `flow/scripts` 10, `flow/hooks-fallback` 2, `.claude/hooks` 5) + 4 setup templates + 15 `flow/engine` modules = 43 files, mirrored at their repo paths. Flow data files (`state-machine.json`, `policy.json`) are not frozen here; F5 freezes its corpus. Cost if wrong: F5 adds them.
- **P2-R5 — What parity compares.** Exit code, stdout bytes, and the bytes of every file left in the sandbox — not stderr (§7 names those three). Both sides stage at the same absolute path, one after the other, so absolute paths in output match. Cost if wrong: stderr differences go unnoticed until a later phase adds them.
- **P2-R6 — Self-check until F16.** CI's `parity` job runs `--self` (oracle vs oracle) until F16 adds `vibe-spec` as the candidate. The red-on-mutant proof uses the test candidate `parity-fixture`. Cost if wrong: none; F16 switches the job to `--candidate`.
- **P2-R7 — Local limits.** This machine has Homebrew Rust 1.98 and no rustup or cargo-deny. MSRV is proven locally by `every_resolved_package_builds_at_msrv` (all resolved `rust_version` ≤ 1.88) and in CI by the 1.88 leg; `cargo-deny` runs in CI only (local proxy: `cargo tree --workspace -d` prints nothing). Cost if wrong: an MSRV or deny failure surfaces only in CI, after the owner approves a push.
- **P2-R8 — Frozen oracles are not shellchecked.** CI lints `git ls-files '*.sh' | grep -v '^oracles/'`, so a newer shellcheck never forces an edit to a frozen file. Cost if wrong: none; the live originals are still linted until retired.
- **P2-R9 — Gate honesty.** The exit gate needs CI green on 3 OS × stable + MSRV, which needs a push. Rows F2–F4 stay `IN PROGRESS — local gate green; CI pending push` until the owner approves the push and CI is green.

## File Structure

```text
.gitattributes                     # -text for byte-exact trees            (Task 1)
oracles/
├── README.md                      # what is frozen and why                 (Task 1)
├── MANIFEST.sha256                # sha256 of every file under bash/, js/  (Task 1)
├── check.sh                       # manifest + population check            (Task 1)
├── bash/…                         # 24 scripts + 4 templates, repo paths   (Task 1)
└── js/…                           # 15 flow/engine modules                 (Task 1)
tests/oracles/run.sh               # discriminating tests for check.sh      (Task 1)
contracts/
├── lib/schema.mjs                 # JSON Schema subset validator           (Task 2)
├── tests/schema.test.mjs          # validator unit tests                   (Task 2)
├── spec-json.schema.json          # Spec JSON v1                           (Task 3)
├── exit-codes.json                # exit-code contract v1                  (Task 3)
├── exit-codes.schema.json         #                                        (Task 3)
├── fixtures/spec-json/<query>/{valid,invalid}/*.json                       (Task 3)
├── run.mjs                        # validates contracts/                   (Task 3)
├── tests/run.test.mjs             # discriminating tests for run.mjs       (Task 3)
└── run.sh                         # entry point                            (Task 3)
Cargo.toml  Cargo.lock  rust-toolchain.toml  deny.toml                      (Task 4)
crates/vibe-core/                  # error, exit (T4); fs, glob (T5)
crates/vibe-spec/                  # bin stub: --version, usage exit 2      (Task 4)
crates/vibe-parity/                # workspace checks (T6); harness (T7); corpus (T8)
tests/parity/spec.json             # spec parity suite                      (Task 8)
tests/parity/corpus/spec/…         # generated corpus                       (Task 8)
tests/run.sh                       # + suites oracles, contracts            (Tasks 1, 3)
.github/workflows/ci.yml           # + manifest, contracts, rust-check, rust-test, parity
```

---

### Task 1: F3 — freeze the oracles

**Files:**
- Create: `oracles/` (43 frozen files), `oracles/MANIFEST.sha256`, `oracles/check.sh`, `oracles/README.md`, `tests/oracles/run.sh`, `.gitattributes`
- Modify: `tests/run.sh` (suite arrays), `.github/workflows/ci.yml` (shellcheck step), `.spec/lessons.md` (CI shellcheck command, inside `strategy.spec`)

**Interfaces:**
- Produces: `oracles/bash/<repo path>` and `oracles/js/<path under flow/engine>`; `bash oracles/check.sh [dir]` → exit 0 and `oracles: 43 files match MANIFEST.sha256`, exit 1 on any defect. Task 8 stages `oracles/bash/spec` (`scripts/`, `reference/templates/`) as each sandbox's `.oracle/`.

- [ ] **Step 1: Write the failing test**

Create `tests/oracles/run.sh`:

```bash
#!/usr/bin/env bash
# Discriminating tests for oracles/check.sh: the real tree and a clean copy
# pass; a changed byte, a missing file, an unlisted file, a missing manifest
# and a population below the floor each fail.
set -uo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
CHECK="$ROOT/oracles/check.sh"
pass=0
fail=0

expect() { # name want-rc [dir]
  local name="$1" want="$2" rc=0
  shift 2
  bash "$CHECK" "$@" >/dev/null 2>&1 || rc=$?
  if [[ "$rc" == "$want" ]]; then
    pass=$((pass + 1)); echo "  PASS [oracles] $name"
  else
    fail=$((fail + 1)); echo "  FAIL [oracles] $name (rc=$rc, want $want)"
  fi
}

copy() { # prints a fresh copy of oracles/
  local d
  d="$(mktemp -d)"
  cp -R "$ROOT/oracles/." "$d/"
  printf '%s\n' "$d"
}

echo "=== oracles: manifest check ==="
expect "the real tree passes" 0

d="$(copy)"; expect "a clean copy passes" 0 "$d"; rm -rf "$d"

d="$(copy)"; printf ' ' >>"$d/bash/spec/scripts/validate.sh"
expect "one appended byte fails" 1 "$d"; rm -rf "$d"

d="$(copy)"; sed 's/exit 1/exit 9/' "$d/bash/spec/scripts/promote.sh" >"$d/p" && mv "$d/p" "$d/bash/spec/scripts/promote.sh"
expect "a same-path content change fails" 1 "$d"; rm -rf "$d"

d="$(copy)"; rm "$d/js/cli.mjs"
expect "a missing file fails" 1 "$d"; rm -rf "$d"

d="$(copy)"; : >"$d/bash/extra.sh"
expect "an unlisted file fails" 1 "$d"; rm -rf "$d"

d="$(copy)"; rm "$d/MANIFEST.sha256"
expect "a missing manifest fails" 1 "$d"; rm -rf "$d"

d="$(copy)"; rm "$d/js/json.mjs"; grep -v ' js/json.mjs$' "$d/MANIFEST.sha256" >"$d/m" && mv "$d/m" "$d/MANIFEST.sha256"
expect "a population below the floor fails" 1 "$d"; rm -rf "$d"

attr="$(git -C "$ROOT" check-attr text -- oracles/bash/spec/scripts/validate.sh 2>/dev/null)"
if [[ "$attr" == *": text: unset" ]]; then
  pass=$((pass + 1)); echo "  PASS [oracles] .gitattributes disables EOL conversion under oracles/"
else
  fail=$((fail + 1)); echo "  FAIL [oracles] .gitattributes disables EOL conversion under oracles/ (got: $attr)"
fi

echo ""
echo "=== results: $pass passed, $fail failed ==="
[[ "$fail" -eq 0 ]]
```

- [ ] **Step 2: Run it to verify it fails**

Run: `bash tests/oracles/run.sh; echo rc=$?`
Expected: every case FAIL (no `oracles/check.sh`), `rc=1`.

- [ ] **Step 3: Freeze the oracles**

Create `oracles/README.md` first (the freeze appends the source commit to it):

```markdown
# Frozen oracles

Byte-exact copies of the implementation the Rust port must match (D30, F3):

- `bash/` — every tracked script under `spec/scripts/`, `flow/scripts/`,
  `flow/hooks-fallback/` and `.claude/hooks/`, at its repo path, plus the
  four setup templates `spec/scripts/setup.sh` reads.
- `js/` — every non-test module of `flow/engine/`.

`MANIFEST.sha256` lists every file; `bash oracles/check.sh` fails on any
byte change, missing or unlisted file, or fewer files than its floor. An
oracle never changes to match a bug: a defect found during parity becomes an
`expected-divergence` pin in the parity suite, fixed in Rust only.
```

Then run from the repo root, exactly once, on a clean tree:

```bash
#!/usr/bin/env bash
# One-time freeze: copies today's scripts and engine into oracles/ (run from the repo root).
set -euo pipefail
[[ ! -e oracles/bash && ! -e oracles/js ]] || { echo "freeze: oracles/ already populated" >&2; exit 1; }
git diff --quiet HEAD -- spec/scripts spec/reference/templates flow/scripts flow/hooks-fallback flow/engine .claude/hooks ||
  { echo "freeze: uncommitted changes in a frozen source; commit or revert them first" >&2; exit 1; }
git ls-files spec/scripts flow/scripts flow/hooks-fallback .claude/hooks | grep '\.sh$' | while IFS= read -r f; do
  mkdir -p "oracles/bash/$(dirname "$f")"
  cp -p "$f" "oracles/bash/$f"
done
mkdir -p oracles/bash/spec/reference/templates
for t in product tech design plan; do
  cp -p "spec/reference/templates/$t.md" oracles/bash/spec/reference/templates/
done
git ls-files flow/engine | grep -v '^flow/engine/tests/' | grep '\.mjs$' | while IFS= read -r f; do
  rel="${f#flow/engine/}"
  mkdir -p "oracles/js/$(dirname "$rel")"
  cp -p "$f" "oracles/js/$rel"
done
(cd oracles && find bash js -type f | LC_ALL=C sort | xargs shasum -a 256 >MANIFEST.sha256)
printf '\nFrozen from commit %s.\n' "$(git rev-parse HEAD)" >>oracles/README.md
wc -l <oracles/MANIFEST.sha256
```

Expected last line: `43`, and `oracles/README.md` ends with `Frozen from commit <sha>.`. Then create `oracles/check.sh`:

```bash
#!/usr/bin/env bash
# Verifies the frozen oracles against MANIFEST.sha256. Fails on any byte
# change, missing file, unlisted file, or a population below the floor.
# Usage: bash oracles/check.sh [oracles-dir]
set -euo pipefail
FLOOR=43
dir="${1:-$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)}"
cd "$dir"
[[ -f MANIFEST.sha256 ]] || { echo "oracles: no MANIFEST.sha256 in $dir" >&2; exit 1; }
listed="$(awk '{ print $2 }' MANIFEST.sha256 | LC_ALL=C sort)"
actual="$(find bash js -type f 2>/dev/null | LC_ALL=C sort)"
if [[ "$listed" != "$actual" ]]; then
  echo "oracles: the file set differs from MANIFEST.sha256 (< listed, > on disk):" >&2
  diff <(printf '%s\n' "$listed") <(printf '%s\n' "$actual") >&2 || true
  exit 1
fi
count="$(printf '%s\n' "$listed" | grep -c .)"
if [[ "$count" -lt "$FLOOR" ]]; then
  echo "oracles: $count files listed, floor $FLOOR" >&2
  exit 1
fi
if ! out="$(shasum -a 256 -c MANIFEST.sha256 2>&1)"; then
  printf '%s\n' "$out" | grep -v ': OK$' >&2
  echo "oracles: hash mismatch — a frozen oracle changed" >&2
  exit 1
fi
echo "oracles: $count files match MANIFEST.sha256"
```

Create `.gitattributes`:

```gitattributes
# Byte-exact trees: frozen oracles, contract goldens, the parity corpus.
# No end-of-line conversion on any platform.
oracles/** -text
contracts/fixtures/** -text
tests/parity/corpus/** -text
```

- [ ] **Step 4: Register the suite**

In `tests/run.sh`, replace the three suite arrays with:

```bash
SUITE_NAMES=(spec flow adapters engine oracles)
SUITE_INTERPRETERS=("$SELF_BASH" "$SELF_BASH" "$SELF_BASH" node "$SELF_BASH")
SUITE_SCRIPTS=(
  "spec/tests/run.sh"
  "flow/tests/run.sh"
  "flow/tests/adapters/run.sh"
  "flow/engine/tests/run.mjs"
  "tests/oracles/run.sh"
)
```

- [ ] **Step 5: Exclude frozen copies from CI shellcheck and check the manifest**

In `.github/workflows/ci.yml`, job `check`, replace the step `Shellcheck (all tracked *.sh)` with:

```yaml
      # oracles/ is byte-frozen (F3): a newer shellcheck must never force an
      # edit there, so the frozen copies are excluded. Their live originals
      # are still linted until they are retired.
      - name: Shellcheck (all tracked *.sh except frozen oracles)
        run: git ls-files '*.sh' | grep -v '^oracles/' | xargs shellcheck

      - name: Frozen oracles match MANIFEST.sha256
        run: bash oracles/check.sh
```

In `.spec/lessons.md`, lesson "A new shell script runs the umbrella suite, not just its own", change `` `git ls-files '*.sh' | xargs shellcheck` (the CI command) `` to `` `git ls-files '*.sh' | grep -v '^oracles/' | xargs shellcheck` (the CI command) ``. Do this inside `strategy.spec`:

```bash
bash .agents/skills/vibe/scripts/set-state.sh strategy.brainstorm
bash .agents/skills/vibe/scripts/set-state.sh strategy.spec
# edit .spec/lessons.md, bump its `updated:` if present
bash .agents/skills/vibe/scripts/regen-active-rules.sh
bash .agents/skills/vibe/scripts/set-state.sh idle
```

- [ ] **Step 6: Run the checks**

```bash
bash tests/oracles/run.sh
git ls-files '*.sh' | grep -v '^oracles/' | xargs shellcheck; echo "shellcheck rc=$?"
bash tests/run.sh 2>&1 | tail -10
```
Expected: `=== results: 9 passed, 0 failed ===`; `shellcheck rc=0`; `=== ALL SUITES PASSED ===` with `PASS  oracles`.

- [ ] **Step 7: Commit**

```bash
git add oracles tests/oracles tests/run.sh .gitattributes .github/workflows/ci.yml .spec/lessons.md AGENTS.md
git commit -m "test(parity): freeze bash and js oracles"
```

---

### Task 2: F2 — JSON Schema subset validator

**Files:**
- Create: `contracts/lib/schema.mjs`, `contracts/tests/schema.test.mjs`

**Interfaces:**
- Produces: `checkSchema(schema) → string[]` (problems with the schema itself) and `validate(root, schema, value) → string[]` (problems with `value`; `root` resolves `#/$defs/<name>`). Task 3 imports both from `./lib/schema.mjs`.

- [ ] **Step 1: Write the failing test**

Create `contracts/tests/schema.test.mjs`:

```javascript
// Unit tests for contracts/lib/schema.mjs. Run: node contracts/tests/schema.test.mjs
import assert from 'node:assert/strict';
import { checkSchema, validate } from '../lib/schema.mjs';

let passed = 0;
let failed = 0;
function test(name, fn) {
  try { fn(); passed += 1; console.log(`  PASS ${name}`); }
  catch (err) { failed += 1; console.log(`  FAIL ${name}\n    ${err.message.split('\n').join('\n    ')}`); }
}

const root = {
  $defs: {
    item: { type: 'object', required: ['id'], properties: { id: { type: 'string', minLength: 1 } } },
  },
  type: 'object',
  required: ['v', 'items'],
  properties: {
    v: { const: 1 },
    kind: { enum: ['a', 'b'] },
    count: { type: 'integer', minimum: 0 },
    date: { type: 'string', pattern: '^[0-9]{4}-[0-9]{2}-[0-9]{2}$' },
    items: { type: 'array', minItems: 1, items: { $ref: '#/$defs/item' } },
  },
};
const ok = (value) => validate(root, root, value);

test('a conforming value has no errors', () => {
  assert.deepEqual(ok({ v: 1, kind: 'a', count: 0, date: '2026-10-14', items: [{ id: 'x' }] }), []);
});
test('unknown fields are ignored', () => {
  assert.deepEqual(ok({ v: 1, items: [{ id: 'x', extra: true }], future: { any: 1 } }), []);
});
test('a missing required field fails', () => {
  assert.match(ok({ v: 1 }).join('\n'), /missing required "items"/);
});
test('a wrong const fails', () => assert.match(ok({ v: 2, items: [{ id: 'x' }] }).join('\n'), /const 1/));
test('a value outside enum fails', () => assert.match(ok({ v: 1, kind: 'c', items: [{ id: 'x' }] }).join('\n'), /not one of/));
test('a wrong type fails', () => assert.match(ok({ v: 1, items: 'x' }).join('\n'), /expected array/));
test('a float is not an integer', () => assert.match(ok({ v: 1, count: 1.5, items: [{ id: 'x' }] }).join('\n'), /expected integer/));
test('below minimum fails', () => assert.match(ok({ v: 1, count: -1, items: [{ id: 'x' }] }).join('\n'), /minimum/));
test('a pattern mismatch fails', () => assert.match(ok({ v: 1, date: '14.10.2026', items: [{ id: 'x' }] }).join('\n'), /does not match/));
test('too few items fails', () => assert.match(ok({ v: 1, items: [] }).join('\n'), /fewer than 1/));
test('$ref is followed into items', () => assert.match(ok({ v: 1, items: [{ id: '' }] }).join('\n'), /\$\.items\[0\]\.id: shorter than 1/));
test('an unresolved $ref is an error', () => {
  assert.match(validate({}, { $ref: '#/$defs/none' }, 1).join('\n'), /unresolved/);
});
test('checkSchema accepts the test schema', () => assert.deepEqual(checkSchema(root), []));
test('checkSchema rejects an unsupported keyword', () => {
  assert.match(checkSchema({ type: 'object', oneOf: [] }).join('\n'), /unsupported keyword "oneOf"/);
});
test('checkSchema rejects additionalProperties false, nested too', () => {
  const s = { properties: { a: { type: 'object', additionalProperties: false } } };
  assert.match(checkSchema(s).join('\n'), /#\/properties\/a: additionalProperties/);
});
test('checkSchema rejects a non-local $ref', () => {
  assert.match(checkSchema({ $ref: 'https://example.com/x.json' }).join('\n'), /\$ref must be/);
});

console.log(`\n=== results: ${passed} passed, ${failed} failed ===`);
process.exit(failed === 0 ? 0 : 1);
```

- [ ] **Step 2: Run it to verify it fails**

Run: `node contracts/tests/schema.test.mjs; echo rc=$?`
Expected: `ERR_MODULE_NOT_FOUND` for `../lib/schema.mjs`, non-zero rc.

- [ ] **Step 3: Write the validator**

Create `contracts/lib/schema.mjs`:

```javascript
// Minimal JSON Schema subset validator (zero dependencies).
//
// Supported keywords: $schema, $comment, title, description, x-vibe-compat,
// $defs, $ref (local "#/$defs/<name>" only), type, const, enum, required,
// properties, additionalProperties (absent or true only), items, minItems,
// minLength, minimum, pattern. Any other keyword is a schema error: a schema
// must not rely on a keyword this validator would silently ignore.

const KEYWORDS = new Set([
  '$schema', '$comment', 'title', 'description', 'x-vibe-compat', '$defs', '$ref',
  'type', 'const', 'enum', 'required', 'properties', 'additionalProperties',
  'items', 'minItems', 'minLength', 'minimum', 'pattern',
]);

const isObject = (v) => v !== null && typeof v === 'object' && !Array.isArray(v);

/** Returns a list of problems with `schema` itself; empty when usable. */
export function checkSchema(schema, at = '#') {
  if (!isObject(schema)) return [`${at}: a schema must be an object`];
  const errors = [];
  for (const key of Object.keys(schema)) {
    if (!KEYWORDS.has(key)) errors.push(`${at}: unsupported keyword "${key}"`);
  }
  if ('additionalProperties' in schema && schema.additionalProperties !== true) {
    errors.push(`${at}: additionalProperties must be absent or true (readers ignore unknown fields)`);
  }
  if ('$ref' in schema && !/^#\/\$defs\/[A-Za-z0-9_-]+$/.test(schema.$ref)) {
    errors.push(`${at}: $ref must be "#/$defs/<name>"`);
  }
  for (const [name, sub] of Object.entries(schema.$defs ?? {})) errors.push(...checkSchema(sub, `${at}/$defs/${name}`));
  for (const [name, sub] of Object.entries(schema.properties ?? {})) errors.push(...checkSchema(sub, `${at}/properties/${name}`));
  if ('items' in schema) errors.push(...checkSchema(schema.items, `${at}/items`));
  return errors;
}

function typeOf(v) {
  if (v === null) return 'null';
  if (Array.isArray(v)) return 'array';
  if (Number.isInteger(v)) return 'integer';
  return typeof v;
}

function typeMatches(want, v) {
  const got = typeOf(v);
  return want === got || (want === 'number' && got === 'integer');
}

/** Validates `value` against `schema`; `root` resolves $ref. Returns error strings. */
export function validate(root, schema, value, at = '$') {
  if (schema.$ref) {
    const name = schema.$ref.slice('#/$defs/'.length);
    const target = root.$defs?.[name];
    if (!target) return [`${at}: unresolved $ref ${schema.$ref}`];
    return validate(root, target, value, at);
  }
  const errors = [];
  if (schema.type !== undefined) {
    const types = Array.isArray(schema.type) ? schema.type : [schema.type];
    if (!types.some((t) => typeMatches(t, value))) {
      return [`${at}: expected ${types.join('|')}, got ${typeOf(value)}`];
    }
  }
  if ('const' in schema && JSON.stringify(value) !== JSON.stringify(schema.const)) {
    errors.push(`${at}: expected const ${JSON.stringify(schema.const)}`);
  }
  if (schema.enum && !schema.enum.some((e) => JSON.stringify(e) === JSON.stringify(value))) {
    errors.push(`${at}: ${JSON.stringify(value)} is not one of ${JSON.stringify(schema.enum)}`);
  }
  if (typeof value === 'string') {
    if (schema.minLength !== undefined && [...value].length < schema.minLength) errors.push(`${at}: shorter than ${schema.minLength}`);
    if (schema.pattern !== undefined && !new RegExp(schema.pattern, 'u').test(value)) errors.push(`${at}: does not match ${schema.pattern}`);
  }
  if (typeof value === 'number' && schema.minimum !== undefined && value < schema.minimum) {
    errors.push(`${at}: below minimum ${schema.minimum}`);
  }
  if (Array.isArray(value)) {
    if (schema.minItems !== undefined && value.length < schema.minItems) errors.push(`${at}: fewer than ${schema.minItems} items`);
    if (schema.items) value.forEach((item, i) => errors.push(...validate(root, schema.items, item, `${at}[${i}]`)));
  }
  if (isObject(value)) {
    for (const key of schema.required ?? []) {
      if (!(key in value)) errors.push(`${at}: missing required "${key}"`);
    }
    for (const [key, sub] of Object.entries(schema.properties ?? {})) {
      if (key in value) errors.push(...validate(root, sub, value[key], `${at}.${key}`));
    }
  }
  return errors;
}
```

- [ ] **Step 4: Run it to verify it passes**

Run: `node contracts/tests/schema.test.mjs`
Expected: `=== results: 16 passed, 0 failed ===`.

- [ ] **Step 5: Commit**

```bash
git add contracts/lib contracts/tests/schema.test.mjs
git commit -m "feat(contracts): add json schema subset validator"
```

---

### Task 3: F2 — Spec JSON v1 and exit-code contracts

**Files:**
- Create: `contracts/spec-json.schema.json`, `contracts/exit-codes.json`, `contracts/exit-codes.schema.json`, 16 goldens under `contracts/fixtures/spec-json/`, `contracts/run.mjs`, `contracts/tests/run.test.mjs`, `contracts/run.sh`
- Modify: `tests/run.sh`, `.github/workflows/ci.yml`

**Interfaces:**
- Consumes: `checkSchema`, `validate` (Task 2).
- Produces: `contracts/exit-codes.json` (Task 4's `vibe_core::exit` test reads it); `node contracts/run.mjs [--dir <dir>]` → exit 0 and `contracts: 2 schemas, 8 valid + 8 invalid goldens — OK`; `bash contracts/run.sh` runs both test files then `run.mjs`.

- [ ] **Step 1: Write the failing test**

Create `contracts/tests/run.test.mjs`:

```javascript
// Discriminating tests for contracts/run.mjs: a clean copy passes; each
// planted defect fails with its own message. Run: node contracts/tests/run.test.mjs
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { cpSync, mkdtempSync, readFileSync, rmSync, unlinkSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const CONTRACTS = join(dirname(fileURLToPath(import.meta.url)), '..');
let passed = 0;
let failed = 0;

function check(name, mutate, wantExit, wantText) {
  const dir = mkdtempSync(join(tmpdir(), 'contracts-'));
  try {
    cpSync(CONTRACTS, dir, { recursive: true });
    mutate(dir);
    const r = spawnSync(process.execPath, [join(CONTRACTS, 'run.mjs'), '--dir', dir], { encoding: 'utf8' });
    assert.equal(r.status, wantExit, `exit ${r.status}\n${r.stdout}${r.stderr}`);
    if (wantText) assert.match(r.stdout + r.stderr, wantText);
    passed += 1;
    console.log(`  PASS ${name}`);
  } catch (err) {
    failed += 1;
    console.log(`  FAIL ${name}\n    ${err.message.split('\n').join('\n    ')}`);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
}

const edit = (dir, rel, fn) => {
  const p = join(dir, rel);
  writeFileSync(p, `${JSON.stringify(fn(JSON.parse(readFileSync(p, 'utf8'))), null, 2)}\n`);
};
const SPEC = 'spec-json.schema.json';
const FX = 'fixtures/spec-json';

check('a clean copy passes', () => {}, 0, /OK$/m);
check('a valid golden missing "v" fails', (d) => edit(d, `${FX}/root/valid/dot-spec.json`, ({ v, ...rest }) => rest), 1, /missing required "v"/);
check('additionalProperties false fails', (d) => edit(d, SPEC, (s) => { s.$defs.plan.additionalProperties = false; return s; }), 1, /additionalProperties must be absent or true/);
check('an unsupported keyword fails', (d) => edit(d, SPEC, (s) => { s.$defs.root.oneOf = []; return s; }), 1, /unsupported keyword "oneOf"/);
check('a missing compat statement fails', (d) => edit(d, 'exit-codes.schema.json', ({ ['x-vibe-compat']: _, ...rest }) => rest), 1, /x-vibe-compat must be/);
check('an invalid golden that validates fails', (d) => writeFileSync(join(d, `${FX}/plan/invalid/row-without-status.json`), '{ "v": 1, "feature": "auth", "rows": [] }\n'), 1, /validates, but must fail/);
check('too few invalid goldens fails the floor', (d) => unlinkSync(join(d, `${FX}/feature/invalid/wrong-major.json`)), 1, /feature: 1 invalid golden\(s\) \(floor 2\)/);
check('an unknown-field golden without one fails', (d) => writeFileSync(join(d, `${FX}/plan/valid/unknown-field.json`), '{ "v": 1, "feature": "auth", "rows": [] }\n'), 1, /holds no field outside the schema/);
check('a reordered root.source enum fails', (d) => edit(d, SPEC, (s) => { s.$defs.root.properties.source.enum.reverse(); return s; }), 1, /discovery order/);
check('an extra cli exit code fails', (d) => edit(d, 'exit-codes.json', (c) => { c.contexts.cli['4'] = 'new'; return c; }), 1, /cli has/);
check('a missing query def fails', (d) => edit(d, SPEC, (s) => { delete s.$defs.feature; return s; }), 1, /\$defs\.feature missing/);
check('unparseable JSON fails', (d) => writeFileSync(join(d, `${FX}/root/valid/dot-spec.json`), '{ nope'), 1, /dot-spec\.json/);

console.log(`\n=== results: ${passed} passed, ${failed} failed ===`);
process.exit(failed === 0 ? 0 : 1);
```

- [ ] **Step 2: Run it to verify it fails**

Run: `node contracts/tests/run.test.mjs; echo rc=$?`
Expected: every case FAIL (no `run.mjs`), `rc=1`.

- [ ] **Step 3: Write the schemas and the exit-code contract**

Create `contracts/spec-json.schema.json`:

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "Spec JSON v1",
  "description": "Answers of the four read-only vibe-spec queries when called with --json: root, lessons-for, plan, feature. Each answer is one JSON object with \"v\": 1. This is a new output; it is not the legacy `lessons-for.sh --format json` array, which stays byte-identical for parity.",
  "x-vibe-compat": "additive-within-major; readers-ignore-unknown-fields",
  "$defs": {
    "root": {
      "type": "object",
      "required": ["v", "root", "source"],
      "properties": {
        "v": { "const": 1 },
        "root": { "type": "string", "minLength": 1, "pattern": "^(/|[A-Za-z]:[\\\\/])", "description": "Absolute path of the spec root." },
        "source": { "enum": ["flag", "env", "config", "docs-spec", "dot-spec"], "description": "Discovery step that found the root (tech-spec.md, Root and discovery, steps 1-5)." },
        "depth": { "type": "integer", "minimum": 0, "description": "Parent directories walked before the hit (step 6); 0 for the cwd." },
        "layout": { "enum": ["flat", "layered"] }
      }
    },
    "lessons-for": {
      "type": "object",
      "required": ["v", "tags", "lessons"],
      "properties": {
        "v": { "const": 1 },
        "tags": { "type": "array", "minItems": 1, "items": { "type": "string", "minLength": 1 } },
        "lessons": { "type": "array", "items": { "$ref": "#/$defs/lesson" } }
      }
    },
    "lesson": {
      "type": "object",
      "required": ["title", "rule", "tags"],
      "properties": {
        "title": { "type": "string", "minLength": 1 },
        "rule": { "type": "string", "minLength": 1 },
        "tags": { "type": "array", "items": { "type": "string", "minLength": 1 } },
        "pattern": { "type": "string" },
        "date": { "type": "string", "pattern": "^[0-9]{4}-[0-9]{2}-[0-9]{2}$" },
        "pinned": { "type": "boolean" }
      }
    },
    "plan": {
      "type": "object",
      "required": ["v", "feature", "rows"],
      "properties": {
        "v": { "const": 1 },
        "feature": { "$ref": "#/$defs/feature-name" },
        "rows": { "type": "array", "items": { "$ref": "#/$defs/plan-row" } },
        "source": { "$ref": "#/$defs/doc" }
      }
    },
    "plan-row": {
      "type": "object",
      "required": ["id", "status"],
      "properties": {
        "id": { "type": "string", "minLength": 1 },
        "status": { "type": "string", "minLength": 1 },
        "title": { "type": "string" },
        "gate": { "type": "string" }
      }
    },
    "feature": {
      "type": "object",
      "required": ["v", "feature", "layer", "docs"],
      "properties": {
        "v": { "const": 1 },
        "feature": { "$ref": "#/$defs/feature-name" },
        "layer": { "enum": ["superpowers", "fallback"] },
        "docs": { "type": "array", "items": { "$ref": "#/$defs/doc" } },
        "status": { "type": "string" },
        "summary": { "type": "string" }
      }
    },
    "feature-name": { "type": "string", "pattern": "^[a-z0-9][a-z0-9-]*$" },
    "doc": {
      "type": "object",
      "required": ["kind", "path"],
      "properties": {
        "kind": { "enum": ["design", "plan", "product", "tech"] },
        "path": { "type": "string", "minLength": 1, "description": "Repo-relative path with forward slashes." }
      }
    }
  }
}
```

Create `contracts/exit-codes.json`:

```json
{
  "v": 1,
  "contexts": {
    "cli": { "0": "ok", "1": "findings or refused", "2": "usage error" },
    "vibe-spec-query": { "3": "no spec tree found" },
    "hook": { "0": "pass, or degraded", "2": "block" }
  }
}
```

Create `contracts/exit-codes.schema.json`:

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "Exit-code contract v1",
  "description": "Process exit codes every vibe binary uses, per context. contracts/run.mjs pins the exact code sets.",
  "x-vibe-compat": "additive-within-major; readers-ignore-unknown-fields",
  "type": "object",
  "required": ["v", "contexts"],
  "properties": {
    "v": { "const": 1 },
    "contexts": {
      "type": "object",
      "required": ["cli", "vibe-spec-query", "hook"],
      "properties": {
        "cli": { "type": "object" },
        "vibe-spec-query": { "type": "object" },
        "hook": { "type": "object" }
      }
    }
  }
}
```

- [ ] **Step 4: Write the goldens**

Each query has two valid goldens (one named `unknown-field.json`, carrying a top-level field the schema does not define) and two invalid ones.

`contracts/fixtures/spec-json/root/valid/dot-spec.json`:

```json
{ "v": 1, "root": "/home/dev/app/.spec", "source": "dot-spec", "depth": 1, "layout": "flat" }
```

`contracts/fixtures/spec-json/root/valid/unknown-field.json`:

```json
{ "v": 1, "root": "C:\\work\\app\\docs\\spec", "source": "docs-spec", "added-in-v1.1": true }
```

`contracts/fixtures/spec-json/root/invalid/relative-root.json`:

```json
{ "v": 1, "root": "docs/spec", "source": "docs-spec" }
```

`contracts/fixtures/spec-json/root/invalid/unknown-source.json`:

```json
{ "v": 1, "root": "/app/docs/spec", "source": "guess" }
```

`contracts/fixtures/spec-json/lessons-for/valid/two-lessons.json`:

```json
{
  "v": 1,
  "tags": ["bash"],
  "lessons": [
    { "title": "A headless CLI inside a read loop eats the loop's input", "rule": "Give every stdin reader inside a while-read loop </dev/null.", "tags": ["bash", "harness"], "pattern": "claude -p swallowed scenarios.tsv.", "date": "2026-10-14" },
    { "title": "Guard bash 3.2 arrays", "rule": "Expand as ${arr[@]+\"${arr[@]}\"}.", "tags": ["bash", "bash32"], "pinned": true }
  ]
}
```

`contracts/fixtures/spec-json/lessons-for/valid/unknown-field.json`:

```json
{ "v": 1, "tags": ["none-match"], "lessons": [], "truncated": false }
```

`contracts/fixtures/spec-json/lessons-for/invalid/legacy-array.json`:

```json
[ { "title": "t", "pattern": "p", "rule": "r", "tags": "bash", "date": "2026-10-14" } ]
```

`contracts/fixtures/spec-json/lessons-for/invalid/tags-as-string.json`:

```json
{ "v": 1, "tags": ["bash"], "lessons": [ { "title": "t", "rule": "r", "tags": "bash, harness" } ] }
```

`contracts/fixtures/spec-json/plan/valid/superpowers-plan.json`:

```json
{
  "v": 1,
  "feature": "spec-v04-p2-rust-foundations",
  "source": { "kind": "plan", "path": "docs/superpowers/plans/2026-10-14-spec-v04-p2-rust-foundations.md" },
  "rows": [
    { "id": "Task 1", "status": "done", "title": "Freeze the oracles" },
    { "id": "Task 2", "status": "todo" }
  ]
}
```

`contracts/fixtures/spec-json/plan/valid/unknown-field.json`:

```json
{ "v": 1, "feature": "auth", "rows": [ { "id": "M1", "status": "planned", "gate": "login works", "owner": "x" } ], "milestone": "M1" }
```

`contracts/fixtures/spec-json/plan/invalid/bad-feature-name.json`:

```json
{ "v": 1, "feature": "Auth Flow", "rows": [] }
```

`contracts/fixtures/spec-json/plan/invalid/row-without-status.json`:

```json
{ "v": 1, "feature": "auth", "rows": [ { "id": "M1" } ] }
```

`contracts/fixtures/spec-json/feature/valid/superpowers.json`:

```json
{
  "v": 1,
  "feature": "csv-export",
  "layer": "superpowers",
  "docs": [
    { "kind": "design", "path": "docs/superpowers/specs/2026-10-14-csv-export-design.md" },
    { "kind": "plan", "path": "docs/superpowers/plans/2026-10-14-csv-export.md" }
  ],
  "status": "in progress",
  "summary": "Export reports as CSV."
}
```

`contracts/fixtures/spec-json/feature/valid/unknown-field.json`:

```json
{ "v": 1, "feature": "auth", "layer": "fallback", "docs": [ { "kind": "product", "path": "docs/spec/features/auth/product.md", "lines": 40 } ], "openspec": true }
```

`contracts/fixtures/spec-json/feature/invalid/unknown-layer.json`:

```json
{ "v": 1, "feature": "auth", "layer": "openspec", "docs": [] }
```

`contracts/fixtures/spec-json/feature/invalid/wrong-major.json`:

```json
{ "v": 2, "feature": "auth", "layer": "fallback", "docs": [] }
```

- [ ] **Step 5: Write the runner and entry point**

Create `contracts/run.mjs`:

```javascript
// Validates contracts/: every schema is usable and states the compatibility
// rule; every Spec JSON golden validates (valid/) or fails (invalid/); the
// exit-code sets are exact; population floors hold.
// Usage: node contracts/run.mjs [--dir <contracts dir>]
import { existsSync, readdirSync, readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { checkSchema, validate } from './lib/schema.mjs';

const COMPAT = 'additive-within-major; readers-ignore-unknown-fields';
const QUERIES = ['root', 'lessons-for', 'plan', 'feature'];
// .spec/tech-spec.md "Root and discovery", steps 1-5; step 6 repeats 4-5 per parent (`depth`).
const ROOT_SOURCES = ['flag', 'env', 'config', 'docs-spec', 'dot-spec'];
const EXIT_CODES = { cli: ['0', '1', '2'], 'vibe-spec-query': ['3'], hook: ['0', '2'] };
const FLOOR = { schemas: 2, validPerQuery: 2, invalidPerQuery: 2 };

const at = process.argv.indexOf('--dir');
const dir = at > 0 ? process.argv[at + 1] : dirname(fileURLToPath(import.meta.url));
const errors = [];
const readJson = (p) => {
  try { return JSON.parse(readFileSync(p, 'utf8')); } catch (err) { errors.push(`${p}: ${err.message}`); return undefined; }
};
const jsonFiles = (p) => (existsSync(p) ? readdirSync(p).filter((f) => f.endsWith('.json')).sort() : []);

const schemaFiles = jsonFiles(dir).filter((f) => f.endsWith('.schema.json'));
if (schemaFiles.length < FLOOR.schemas) errors.push(`only ${schemaFiles.length} schema(s) (floor ${FLOOR.schemas})`);
const schemas = {};
for (const f of schemaFiles) {
  const s = readJson(join(dir, f));
  if (s === undefined) continue;
  schemas[f] = s;
  errors.push(...checkSchema(s).map((e) => `${f}: ${e}`));
  if (s['x-vibe-compat'] !== COMPAT) errors.push(`${f}: x-vibe-compat must be "${COMPAT}"`);
}

let valid = 0;
let invalid = 0;
const spec = schemas['spec-json.schema.json'];
if (!spec) errors.push('spec-json.schema.json missing');
for (const q of spec ? QUERIES : []) {
  const def = spec.$defs?.[q];
  if (!def) { errors.push(`spec-json: $defs.${q} missing`); continue; }
  if (def.properties?.v?.const !== 1 || !def.required?.includes('v')) errors.push(`spec-json: ${q} must require "v" with const 1`);
  const base = join(dir, 'fixtures', 'spec-json', q);
  const goods = jsonFiles(join(base, 'valid'));
  const bads = jsonFiles(join(base, 'invalid'));
  if (goods.length < FLOOR.validPerQuery) errors.push(`${q}: ${goods.length} valid golden(s) (floor ${FLOOR.validPerQuery})`);
  if (bads.length < FLOOR.invalidPerQuery) errors.push(`${q}: ${bads.length} invalid golden(s) (floor ${FLOOR.invalidPerQuery})`);
  if (!goods.includes('unknown-field.json')) errors.push(`${q}: valid/unknown-field.json missing`);
  for (const f of goods) {
    const v = readJson(join(base, 'valid', f));
    if (v === undefined) continue;
    valid += 1;
    const e = validate(spec, def, v);
    if (e.length) errors.push(`${q}/valid/${f}: ${e.join('; ')}`);
    if (f === 'unknown-field.json' && !Object.keys(v ?? {}).some((k) => !(k in (def.properties ?? {})))) {
      errors.push(`${q}/valid/unknown-field.json: holds no field outside the schema`);
    }
  }
  for (const f of bads) {
    const v = readJson(join(base, 'invalid', f));
    if (v === undefined) continue;
    invalid += 1;
    if (validate(spec, def, v).length === 0) errors.push(`${q}/invalid/${f}: validates, but must fail`);
  }
}
const sources = spec?.$defs?.root?.properties?.source?.enum;
if (spec && JSON.stringify(sources) !== JSON.stringify(ROOT_SOURCES)) {
  errors.push(`spec-json: root.source is ${JSON.stringify(sources)}, discovery order is ${JSON.stringify(ROOT_SOURCES)}`);
}

const exitSchema = schemas['exit-codes.schema.json'];
const codes = existsSync(join(dir, 'exit-codes.json')) ? readJson(join(dir, 'exit-codes.json')) : undefined;
if (!exitSchema || codes === undefined) errors.push('exit-codes.json or exit-codes.schema.json missing');
else {
  errors.push(...validate(exitSchema, exitSchema, codes).map((e) => `exit-codes.json: ${e}`));
  for (const [ctx, want] of Object.entries(EXIT_CODES)) {
    const got = Object.keys(codes.contexts?.[ctx] ?? {}).sort();
    if (JSON.stringify(got) !== JSON.stringify(want)) errors.push(`exit-codes.json: ${ctx} has ${JSON.stringify(got)}, want ${JSON.stringify(want)}`);
  }
}

for (const e of errors) console.error(`contracts: ${e}`);
console.log(`contracts: ${schemaFiles.length} schemas, ${valid} valid + ${invalid} invalid goldens — ${errors.length ? 'FAIL' : 'OK'}`);
process.exit(errors.length ? 1 : 0);
```

Create `contracts/run.sh` (and `chmod +x contracts/run.sh`):

```bash
#!/usr/bin/env bash
# Validates contracts/: the schema validator's own tests, the discriminating
# run tests, then the contracts themselves. Needs Node >= 18 and no tool crate.
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
command -v node >/dev/null 2>&1 || { echo "contracts: node not found on PATH" >&2; exit 2; }
node "$HERE/tests/schema.test.mjs"
node "$HERE/tests/run.test.mjs"
node "$HERE/run.mjs"
```

- [ ] **Step 6: Wire the suite and CI**

In `tests/run.sh`, extend the three arrays:

```bash
SUITE_NAMES=(spec flow adapters engine oracles contracts)
SUITE_INTERPRETERS=("$SELF_BASH" "$SELF_BASH" "$SELF_BASH" node "$SELF_BASH" "$SELF_BASH")
SUITE_SCRIPTS=(
  "spec/tests/run.sh"
  "flow/tests/run.sh"
  "flow/tests/adapters/run.sh"
  "flow/engine/tests/run.mjs"
  "tests/oracles/run.sh"
  "contracts/run.sh"
)
```

In `.github/workflows/ci.yml`, job `check`, after `Frozen oracles match MANIFEST.sha256`:

```yaml
      - name: Contracts (schemas, goldens, exit codes)
        run: bash contracts/run.sh
```

- [ ] **Step 7: Run the checks**

```bash
bash contracts/run.sh
shellcheck contracts/run.sh
bash tests/run.sh 2>&1 | tail -10
```
Expected: `=== results: 16 passed, 0 failed ===`, `=== results: 12 passed, 0 failed ===`, `contracts: 2 schemas, 8 valid + 8 invalid goldens — OK`; shellcheck silent; `=== ALL SUITES PASSED ===` with `PASS  contracts`.

- [ ] **Step 8: Commit**

```bash
git add contracts tests/run.sh .github/workflows/ci.yml
git commit -m "feat(contracts): add spec json v1 and exit codes"
```

---

### Task 4: F4 — Cargo workspace scaffold

**Files:**
- Create: `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml`, `deny.toml`, `crates/vibe-core/{Cargo.toml,src/lib.rs,src/error.rs,src/exit.rs,tests/contract.rs}`, `crates/vibe-spec/{Cargo.toml,src/main.rs,tests/cli.rs}`, `crates/vibe-parity/{Cargo.toml,src/lib.rs}`
- Modify: `.gitignore` (add `/target/`)

**Interfaces:**
- Consumes: `contracts/exit-codes.json` (Task 3).
- Produces: `vibe_core::{Error, Result}`; `vibe_core::Error::{Io { path: PathBuf, source: std::io::Error }, Invalid(String)}`; `vibe_core::exit::{OK, FINDINGS, USAGE, NO_ROOT}: u8` = 0, 1, 2, 3. `vibe-spec --version` → `vibe-spec 0.4.0`, exit 0; no args or unknown args → usage on stderr, exit 2.

- [ ] **Step 1: Write the workspace files**

Create `Cargo.toml`:

```toml
[workspace]
resolver = "3"
members = ["crates/vibe-core", "crates/vibe-spec", "crates/vibe-parity"]

[workspace.package]
version = "0.4.0"
edition = "2024"
rust-version = "1.88"
license = "MIT"
publish = false

[workspace.dependencies]
vibe-core = { path = "crates/vibe-core" }
clap = { version = "4.5", default-features = false, features = ["std", "derive", "help", "usage", "error-context"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"

[workspace.lints.rust]
unsafe_code = "forbid"

[workspace.lints.clippy]
unwrap_used = "deny"
expect_used = "deny"
```

Create `rust-toolchain.toml`:

```toml
# MSRV (D19). Bumped only at a tagged release. CI's stable leg overrides
# this with RUSTUP_TOOLCHAIN; local Homebrew cargo (no rustup) ignores it.
[toolchain]
channel = "1.88"
components = ["clippy", "rustfmt"]
profile = "minimal"
```

Create `deny.toml`:

```toml
# cargo-deny policy (D19). CI runs it via EmbarkStudios/cargo-deny-action.
# The direct-dependency allowlist (clap, serde, serde_json) is enforced
# structurally by crates/vibe-parity/tests/workspace_graph.rs; this file
# bans the named crates anywhere in the graph.
[graph]
all-features = true

[advisories]
version = 2
yanked = "deny"

[licenses]
version = 2
allow = ["MIT", "Apache-2.0", "Unicode-3.0"]
confidence-threshold = 0.9

[bans]
multiple-versions = "deny"
wildcards = "deny"
allow-wildcard-paths = true
deny = [
  { crate = "toml", reason = "D19: JSON is the only config format" },
  { crate = "regex", reason = "D19: matching is hand-written in vibe-core" },
  { crate = "tokio", reason = "D19: no async runtime" },
  { crate = "tempfile", reason = "D19: atomic writes are hand-written in vibe-core" },
]

[sources]
unknown-registry = "deny"
unknown-git = "deny"
```

Append to `.gitignore`:

```gitignore

# Cargo build output (F4).
/target/
```

Create `crates/vibe-core/Cargo.toml`:

```toml
[package]
name = "vibe-core"
description = "Shared internals for the vibe tools. Internal only."
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true
publish.workspace = true

[dev-dependencies]
serde_json = { workspace = true }

[lints]
workspace = true
```

Create `crates/vibe-spec/Cargo.toml`:

```toml
[package]
name = "vibe-spec"
description = "The vibe spec tool."
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true
publish.workspace = true

[dependencies]
vibe-core = { workspace = true }
clap = { workspace = true }

[lints]
workspace = true
```

Create `crates/vibe-parity/Cargo.toml`:

```toml
[package]
name = "vibe-parity"
description = "Differential parity harness: frozen oracle vs candidate. Dev-only."
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true
publish.workspace = true

[dependencies]
vibe-core = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }

[lints]
workspace = true
```

Create `crates/vibe-parity/src/lib.rs`:

```rust
//! Differential parity harness (F3) and workspace structure checks (F4).
```

- [ ] **Step 2: Write the failing tests**

Create `crates/vibe-spec/tests/cli.rs`:

```rust
#![allow(clippy::unwrap_used, clippy::expect_used)]
use std::process::Command;

fn run(args: &[&str]) -> (i32, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_vibe-spec"))
        .args(args)
        .output()
        .expect("spawn vibe-spec");
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

#[test]
fn version_exits_zero() {
    let (code, out, _) = run(&["--version"]);
    assert_eq!(code, 0);
    assert_eq!(out, "vibe-spec 0.4.0\n");
}

#[test]
fn no_arguments_is_a_usage_error() {
    let (code, out, err) = run(&[]);
    assert_eq!(code, 2, "stdout={out} stderr={err}");
    assert!(err.contains("Usage: vibe-spec"), "{err}");
}

#[test]
fn unknown_command_is_a_usage_error() {
    let (code, _, err) = run(&["frobnicate"]);
    assert_eq!(code, 2);
    assert!(err.contains("frobnicate"), "{err}");
}
```

Create `crates/vibe-core/tests/contract.rs`:

```rust
#![allow(clippy::unwrap_used, clippy::expect_used)]
//! `vibe_core::exit` matches contracts/exit-codes.json, and core holds no tool vocabulary.
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;
use vibe_core::exit;

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
fn exit_codes_match_the_contract() {
    let text = fs::read_to_string(repo().join("contracts/exit-codes.json")).unwrap();
    let contract: Value = serde_json::from_str(&text).unwrap();
    let meaning = |ctx: &str, code: u8| {
        contract["contexts"][ctx][code.to_string()]
            .as_str()
            .map(str::to_owned)
    };
    assert!(meaning("cli", exit::OK).is_some());
    assert!(meaning("cli", exit::FINDINGS).is_some());
    assert!(meaning("cli", exit::USAGE).is_some());
    assert!(meaning("vibe-spec-query", exit::NO_ROOT).is_some());
    let codes = [exit::OK, exit::FINDINGS, exit::USAGE, exit::NO_ROOT];
    assert_eq!(codes, [0, 1, 2, 3]);
}

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

/// Words that name a tool's vocabulary. Core must take these as input, never spell them.
const TOOL_VOCABULARY: [&str; 12] = [
    ".spec",
    "docs/spec",
    "docs/superpowers",
    "state-machine",
    "state.json",
    "policy.json",
    "AGENTS.md",
    "CLAUDE.md",
    "feature.",
    "strategy.",
    "user-prompt",
    "session-start",
];

#[test]
fn core_sources_hold_no_tool_vocabulary() {
    let mut files = Vec::new();
    rust_files(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("src"),
        &mut files,
    );
    assert!(files.len() >= 3, "scanned only {} files", files.len());
    for file in &files {
        let text = fs::read_to_string(file).unwrap();
        for word in TOOL_VOCABULARY {
            assert!(
                !text.contains(word),
                "{} spells tool vocabulary {word:?}",
                file.display()
            );
        }
    }
}
```

- [ ] **Step 3: Run them to verify they fail**

Run: `cargo test --workspace 2>&1 | tail -5`
Expected: compile errors — `vibe-spec` has no `src/main.rs`, `vibe-core` has no `src/lib.rs`.

- [ ] **Step 4: Write the minimal implementation**

Create `crates/vibe-core/src/lib.rs`:

```rust
//! Shared internals for the vibe tools.
//!
//! Holds no tool vocabulary: no flow state names, no spec paths, no
//! instruct channel names. Tools pass those in.
pub mod error;
pub mod exit;

pub use error::{Error, Result};
```

Create `crates/vibe-core/src/error.rs`:

```rust
//! The error type every `vibe-core` operation returns.
use std::fmt;
use std::path::PathBuf;

/// Errors returned by `vibe-core`. Binaries map them to exit codes.
#[derive(Debug)]
pub enum Error {
    /// An I/O operation on `path` failed.
    Io {
        /// The path the operation touched.
        path: PathBuf,
        /// The underlying error.
        source: std::io::Error,
    },
    /// The caller passed an argument the operation cannot accept.
    Invalid(String),
}

/// `Result` with [`Error`].
pub type Result<T> = std::result::Result<T, Error>;

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Io { path, source } => write!(f, "{}: {source}", path.display()),
            Error::Invalid(msg) => f.write_str(msg),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Io { source, .. } => Some(source),
            Error::Invalid(_) => None,
        }
    }
}
```

Create `crates/vibe-core/src/exit.rs`:

```rust
//! Process exit codes shared by every vibe binary (contracts/exit-codes.json).

/// Success.
pub const OK: u8 = 0;
/// Findings or a refused action.
pub const FINDINGS: u8 = 1;
/// Usage error.
pub const USAGE: u8 = 2;
/// A query found no tree to answer from.
pub const NO_ROOT: u8 = 3;
```

Create `crates/vibe-spec/src/main.rs`:

```rust
//! `vibe-spec`: the spec tool. Commands arrive with F16–F20.
use std::process::ExitCode;

use clap::Parser;

/// The vibe spec tool.
#[derive(Parser)]
#[command(name = "vibe-spec", version, about, arg_required_else_help = true)]
struct Cli {}

fn main() -> ExitCode {
    let _cli = Cli::parse();
    ExitCode::from(vibe_core::exit::OK)
}
```

- [ ] **Step 5: Run the checks**

```bash
cargo test --workspace 2>&1 | grep -E 'test result|FAILED'
cargo clippy --workspace --all-targets -- -D warnings 2>&1 | tail -1
cargo fmt --all --check && echo fmt-ok
cargo tree --workspace -d 2>&1 | head -3
```
Expected: every `test result: ok` (cli 3, contract 2); clippy reports no issues; `fmt-ok`; `cargo tree -d` prints `nothing to print`.

Plant a vocabulary word to prove the guard bites, then restore:

```bash
mkdir -p .superpowers/p2 && cp crates/vibe-core/src/exit.rs .superpowers/p2/exit.rs.keep
echo '// feature.impl' >> crates/vibe-core/src/exit.rs
cargo test -p vibe-core --test contract 2>&1 | grep 'spells tool vocabulary'
cp .superpowers/p2/exit.rs.keep crates/vibe-core/src/exit.rs
cargo test -p vibe-core --test contract 2>&1 | grep 'test result'
```
Expected: one `exit.rs spells tool vocabulary "feature."` line, then `test result: ok. 2 passed`.

- [ ] **Step 6: Check the engine timing budget with `target/` present**

`doctor` walks the repo to a fixed depth; build output could slow it.

```bash
uptime
node flow/engine/tests/run.mjs 2>&1 | tail -2
```
Expected: `674 passed, 0 failed`. If the 150 ms budget fails, rerun alone on a quiet machine; if it still fails, record a ruling in the ledger and build with `CARGO_TARGET_DIR` outside the worktree for the rest of the phase.

- [ ] **Step 7: Commit**

```bash
git add Cargo.toml Cargo.lock rust-toolchain.toml deny.toml .gitignore crates
git commit -m "feat(core): add cargo workspace scaffold"
```

---

### Task 5: F4 — `vibe-core` glob and atomic write

**Files:**
- Create: `crates/vibe-core/src/glob.rs`, `crates/vibe-core/src/fs.rs` (each with its unit tests)
- Modify: `crates/vibe-core/src/lib.rs`

**Interfaces:**
- Produces: `vibe_core::glob::glob_match(pattern: &str, path: &str) -> bool`; `vibe_core::fs::write_atomic(path: &Path, bytes: &[u8]) -> vibe_core::Result<()>`. F16+ consume both; parity proves them against the bash oracles then.

- [ ] **Step 1: Write the failing tests**

Add both modules to `crates/vibe-core/src/lib.rs` so the tests compile against missing functions:

```rust
//! Shared internals for the vibe tools.
//!
//! Holds no tool vocabulary: no flow state names, no spec paths, no
//! instruct channel names. Tools pass those in.
pub mod error;
pub mod exit;
pub mod fs;
pub mod glob;

pub use error::{Error, Result};
```

Create `crates/vibe-core/src/glob.rs` with only the test module first:

```rust
//! Hand-written glob matching over `/`-separated relative paths.

#[cfg(test)]
mod tests {
    use super::glob_match;

    #[test]
    fn table() {
        let cases: &[(&str, &str, bool)] = &[
            ("", "", true),
            ("", "a", false),
            ("*", "", true),
            ("*.md", "a.md", true),
            ("*.md", "d/a.md", false),
            ("a*", "a/b", false),
            ("**", "a/b/c", true),
            ("**/*.md", "a.md", true),
            ("**/*.md", "d/e/a.md", true),
            ("**/*.md", "d/e/a.txt", false),
            ("docs/**", "docs/a/b", true),
            ("docs/**", "docs", false),
            ("a/**/b", "a/b", true),
            ("a/**/b", "a/x/y/b", true),
            ("a/**/b", "a/x/y/c", false),
            ("?.sh", "a.sh", true),
            ("?.sh", "ab.sh", false),
            ("?", "/", false),
            ("[ab].md", "b.md", true),
            ("[ab].md", "c.md", false),
            ("[!ab].md", "c.md", true),
            ("[!ab].md", "a.md", false),
            ("[^ab].md", "a.md", false),
            ("[a-c]x", "bx", true),
            ("[a-c]x", "dx", false),
            ("[a-]x", "-x", true),
            ("[]]", "]", true),
            ("[!a]", "/", false),
            ("[", "[", true),
            ("[ab", "[ab", true),
            ("\\*", "*", true),
            ("\\*", "a", false),
            ("a\\", "a\\", true),
            ("x/y/z/*", "x/y/z/w.md", true),
            ("x/y/z/*", "x/y/z/a/w.md", false),
            ("é*", "ecole", false),
            ("é*", "étude", true),
        ];
        for &(pattern, path, want) in cases {
            assert_eq!(
                glob_match(pattern, path),
                want,
                "glob_match({pattern:?}, {path:?})"
            );
        }
    }
}
```

Create `crates/vibe-core/src/fs.rs` with only the test module first:

```rust
//! Atomic file writes.

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::write_atomic;
    use crate::Error;
    use std::fs;
    use std::path::{Path, PathBuf};

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("vibe-core-fs-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn entries(dir: &Path) -> Vec<String> {
        let mut names: Vec<String> = fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }

    #[test]
    fn creates_a_new_file() {
        let dir = scratch("new");
        let target = dir.join("out.json");
        write_atomic(&target, b"{}\n").unwrap();
        assert_eq!(fs::read(&target).unwrap(), b"{}\n");
        assert_eq!(entries(&dir), ["out.json"]);
    }

    #[test]
    fn replaces_existing_content() {
        let dir = scratch("replace");
        let target = dir.join("out.txt");
        fs::write(&target, b"old content that is longer").unwrap();
        write_atomic(&target, b"new").unwrap();
        assert_eq!(fs::read(&target).unwrap(), b"new");
        assert_eq!(entries(&dir), ["out.txt"]);
    }

    #[cfg(unix)]
    #[test]
    fn result_is_mode_0600() {
        use std::os::unix::fs::PermissionsExt;
        let dir = scratch("mode");
        let target = dir.join("out.txt");
        fs::write(&target, b"x").unwrap();
        fs::set_permissions(&target, fs::Permissions::from_mode(0o644)).unwrap();
        write_atomic(&target, b"y").unwrap();
        let mode = fs::metadata(&target).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
    }

    #[test]
    fn a_directory_target_fails_and_leaves_no_temp_file() {
        let dir = scratch("isdir");
        let target = dir.join("sub");
        fs::create_dir(&target).unwrap();
        fs::write(target.join("keep"), b"k").unwrap();
        let err = write_atomic(&target, b"x").unwrap_err();
        assert!(matches!(err, Error::Io { .. }), "{err}");
        assert_eq!(entries(&dir), ["sub"]);
        assert_eq!(fs::read(target.join("keep")).unwrap(), b"k");
    }

    #[test]
    fn a_missing_parent_fails() {
        let dir = scratch("noparent");
        let err = write_atomic(&dir.join("absent/out.txt"), b"x").unwrap_err();
        assert!(matches!(err, Error::Io { .. }), "{err}");
        assert!(entries(&dir).is_empty());
    }

    #[test]
    fn a_path_without_a_file_name_is_invalid() {
        for p in ["", "/", ".."] {
            let err = write_atomic(Path::new(p), b"x").unwrap_err();
            assert!(matches!(err, Error::Invalid(_)), "{p:?}: {err}");
        }
    }
}
```

- [ ] **Step 2: Run them to verify they fail**

Run: `cargo test -p vibe-core 2>&1 | grep -E 'error\[E0432\]|unresolved import' | head -3`
Expected: `unresolved import super::glob_match` and `super::write_atomic`.

- [ ] **Step 3: Write the implementations**

Replace `crates/vibe-core/src/glob.rs` with:

```rust
//! Hand-written glob matching over `/`-separated relative paths.
//!
//! `*` matches any run of characters except `/`; `**` matches any run
//! including `/`, and `**/` may also match nothing; `?` matches one
//! character except `/`; `[abc]`, `[a-z]` and `[!abc]` (or `[^abc]`) match one
//! character except `/`; `\x` matches `x` literally. An unterminated `[` is
//! a literal. The whole path must match.

/// Returns whether `path` matches `pattern` in full.
pub fn glob_match(pattern: &str, path: &str) -> bool {
    let p: Vec<char> = pattern.chars().collect();
    let s: Vec<char> = path.chars().collect();
    match_at(&p, 0, &s, 0)
}

fn match_at(p: &[char], pi: usize, s: &[char], si: usize) -> bool {
    let Some(&c) = p.get(pi) else {
        return si == s.len();
    };
    match c {
        '*' if p.get(pi + 1) == Some(&'*') => {
            let rest = pi + 2;
            if p.get(rest) == Some(&'/') && match_at(p, rest + 1, s, si) {
                return true;
            }
            (si..=s.len()).any(|k| match_at(p, rest, s, k))
        }
        '*' => {
            let mut k = si;
            loop {
                if match_at(p, pi + 1, s, k) {
                    return true;
                }
                match s.get(k) {
                    Some(&ch) if ch != '/' => k += 1,
                    _ => return false,
                }
            }
        }
        '?' => matches!(s.get(si), Some(&ch) if ch != '/') && match_at(p, pi + 1, s, si + 1),
        '[' => match class(p, pi, s.get(si).copied()) {
            Some((hit, end)) => hit && match_at(p, end, s, si + 1),
            None => s.get(si) == Some(&'[') && match_at(p, pi + 1, s, si + 1),
        },
        '\\' => match p.get(pi + 1) {
            Some(&lit) => s.get(si) == Some(&lit) && match_at(p, pi + 2, s, si + 1),
            None => s.get(si) == Some(&'\\') && si + 1 == s.len(),
        },
        lit => s.get(si) == Some(&lit) && match_at(p, pi + 1, s, si + 1),
    }
}

/// Parses the class opening at `p[pi] == '['` against `ch`.
/// Returns `(matched, index after ']')`, or `None` when the class never closes.
fn class(p: &[char], pi: usize, ch: Option<char>) -> Option<(bool, usize)> {
    let mut i = pi + 1;
    let negate = matches!(p.get(i), Some('!' | '^'));
    if negate {
        i += 1;
    }
    let first = i;
    let mut hit = false;
    while let Some(&c) = p.get(i) {
        if c == ']' && i > first {
            let ok = matches!(ch, Some(x) if x != '/') && hit != negate;
            return Some((ok, i + 1));
        }
        match (p.get(i + 1), p.get(i + 2)) {
            (Some('-'), Some(&hi)) if hi != ']' => {
                hit |= ch.is_some_and(|x| c <= x && x <= hi);
                i += 3;
            }
            _ => {
                hit |= ch == Some(c);
                i += 1;
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::glob_match;

    #[test]
    fn table() {
        let cases: &[(&str, &str, bool)] = &[
            ("", "", true),
            ("", "a", false),
            ("*", "", true),
            ("*.md", "a.md", true),
            ("*.md", "d/a.md", false),
            ("a*", "a/b", false),
            ("**", "a/b/c", true),
            ("**/*.md", "a.md", true),
            ("**/*.md", "d/e/a.md", true),
            ("**/*.md", "d/e/a.txt", false),
            ("docs/**", "docs/a/b", true),
            ("docs/**", "docs", false),
            ("a/**/b", "a/b", true),
            ("a/**/b", "a/x/y/b", true),
            ("a/**/b", "a/x/y/c", false),
            ("?.sh", "a.sh", true),
            ("?.sh", "ab.sh", false),
            ("?", "/", false),
            ("[ab].md", "b.md", true),
            ("[ab].md", "c.md", false),
            ("[!ab].md", "c.md", true),
            ("[!ab].md", "a.md", false),
            ("[^ab].md", "a.md", false),
            ("[a-c]x", "bx", true),
            ("[a-c]x", "dx", false),
            ("[a-]x", "-x", true),
            ("[]]", "]", true),
            ("[!a]", "/", false),
            ("[", "[", true),
            ("[ab", "[ab", true),
            ("\\*", "*", true),
            ("\\*", "a", false),
            ("a\\", "a\\", true),
            ("x/y/z/*", "x/y/z/w.md", true),
            ("x/y/z/*", "x/y/z/a/w.md", false),
            ("é*", "ecole", false),
            ("é*", "étude", true),
        ];
        for &(pattern, path, want) in cases {
            assert_eq!(
                glob_match(pattern, path),
                want,
                "glob_match({pattern:?}, {path:?})"
            );
        }
    }
}
```

Replace `crates/vibe-core/src/fs.rs` with:

```rust
//! Atomic file writes.
use std::ffi::{OsStr, OsString};
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::{Error, Result};

static COUNTER: AtomicU64 = AtomicU64::new(0);

/// Writes `bytes` to `path` atomically.
///
/// A new temporary file is created exclusively in the same directory
/// (mode 0600 on unix), written, synced, then renamed over `path`. A
/// symlink at `path` is replaced, not followed. On any failure the
/// temporary file is removed and `path` is left as it was.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> Result<()> {
    let name = path
        .file_name()
        .ok_or_else(|| Error::Invalid(format!("not a file path: {}", path.display())))?;
    let dir = match path.parent() {
        Some(d) if !d.as_os_str().is_empty() => d,
        _ => Path::new("."),
    };
    let tmp = tmp_path(dir, name);
    let result = write_then_rename(&tmp, path, bytes);
    if result.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    result
}

fn write_then_rename(tmp: &Path, path: &Path, bytes: &[u8]) -> Result<()> {
    let io = |source| Error::Io {
        path: tmp.to_path_buf(),
        source,
    };
    let mut file = create_exclusive(tmp).map_err(io)?;
    file.write_all(bytes).map_err(io)?;
    file.sync_all().map_err(io)?;
    drop(file);
    fs::rename(tmp, path).map_err(|source| Error::Io {
        path: path.to_path_buf(),
        source,
    })
}

fn create_exclusive(path: &Path) -> std::io::Result<File> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options.open(path)
}

fn tmp_path(dir: &Path, name: &OsStr) -> PathBuf {
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(0);
    let mut file = OsString::from(".");
    file.push(name);
    file.push(format!(".tmp.{}.{n}.{nanos}", std::process::id()));
    dir.join(file)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::write_atomic;
    use crate::Error;
    use std::fs;
    use std::path::{Path, PathBuf};

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("vibe-core-fs-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn entries(dir: &Path) -> Vec<String> {
        let mut names: Vec<String> = fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }

    #[test]
    fn creates_a_new_file() {
        let dir = scratch("new");
        let target = dir.join("out.json");
        write_atomic(&target, b"{}\n").unwrap();
        assert_eq!(fs::read(&target).unwrap(), b"{}\n");
        assert_eq!(entries(&dir), ["out.json"]);
    }

    #[test]
    fn replaces_existing_content() {
        let dir = scratch("replace");
        let target = dir.join("out.txt");
        fs::write(&target, b"old content that is longer").unwrap();
        write_atomic(&target, b"new").unwrap();
        assert_eq!(fs::read(&target).unwrap(), b"new");
        assert_eq!(entries(&dir), ["out.txt"]);
    }

    #[cfg(unix)]
    #[test]
    fn result_is_mode_0600() {
        use std::os::unix::fs::PermissionsExt;
        let dir = scratch("mode");
        let target = dir.join("out.txt");
        fs::write(&target, b"x").unwrap();
        fs::set_permissions(&target, fs::Permissions::from_mode(0o644)).unwrap();
        write_atomic(&target, b"y").unwrap();
        let mode = fs::metadata(&target).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
    }

    #[test]
    fn a_directory_target_fails_and_leaves_no_temp_file() {
        let dir = scratch("isdir");
        let target = dir.join("sub");
        fs::create_dir(&target).unwrap();
        fs::write(target.join("keep"), b"k").unwrap();
        let err = write_atomic(&target, b"x").unwrap_err();
        assert!(matches!(err, Error::Io { .. }), "{err}");
        assert_eq!(entries(&dir), ["sub"]);
        assert_eq!(fs::read(target.join("keep")).unwrap(), b"k");
    }

    #[test]
    fn a_missing_parent_fails() {
        let dir = scratch("noparent");
        let err = write_atomic(&dir.join("absent/out.txt"), b"x").unwrap_err();
        assert!(matches!(err, Error::Io { .. }), "{err}");
        assert!(entries(&dir).is_empty());
    }

    #[test]
    fn a_path_without_a_file_name_is_invalid() {
        for p in ["", "/", ".."] {
            let err = write_atomic(Path::new(p), b"x").unwrap_err();
            assert!(matches!(err, Error::Invalid(_)), "{p:?}: {err}");
        }
    }
}
```

- [ ] **Step 4: Run the checks**

```bash
cargo test -p vibe-core 2>&1 | grep -E 'test result|FAILED'
cargo clippy --workspace --all-targets -- -D warnings 2>&1 | tail -1
cargo fmt --all --check && echo fmt-ok
```
Expected: `test result: ok. 7 passed` (lib: 1 glob table + 6 fs), `ok. 2 passed` (contract); clippy no issues; `fmt-ok`.

- [ ] **Step 5: Commit**

```bash
git add crates/vibe-core
git commit -m "feat(core): add glob matching and atomic write"
```

---

### Task 6: F4 — workspace structure checks and Rust CI

**Files:**
- Create: `crates/vibe-parity/src/workspace.rs` (with unit tests), `crates/vibe-parity/tests/workspace_graph.rs`
- Modify: `crates/vibe-parity/src/lib.rs`, `.github/workflows/ci.yml`

**Interfaces:**
- Produces: `vibe_parity::workspace::{check_graph, check_msrv}(meta: &serde_json::Value) -> Result<usize, Vec<String>>`; constants `MEMBERS`, `CORE`, `ALLOWED_EXTERNAL`, `MSRV`.

- [ ] **Step 1: Write the failing integration test**

Create `crates/vibe-parity/tests/workspace_graph.rs`:

```rust
#![allow(clippy::unwrap_used, clippy::expect_used)]
//! The real workspace satisfies the structural rules in `vibe_parity::workspace`.
use std::process::Command;

use serde_json::Value;
use vibe_parity::workspace::{check_graph, check_msrv};

fn metadata(extra: &[&str]) -> Value {
    let manifest = concat!(env!("CARGO_MANIFEST_DIR"), "/../../Cargo.toml");
    let out = Command::new(env!("CARGO"))
        .args([
            "metadata",
            "--format-version",
            "1",
            "--locked",
            "--manifest-path",
            manifest,
        ])
        .args(extra)
        .output()
        .expect("run cargo metadata");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).expect("parse cargo metadata")
}

#[test]
fn member_graph_holds() {
    let n = check_graph(&metadata(&["--no-deps"])).unwrap_or_else(|e| panic!("{e:#?}"));
    assert_eq!(n, 3);
}

#[test]
fn every_resolved_package_builds_at_msrv() {
    let n = check_msrv(&metadata(&[])).unwrap_or_else(|e| panic!("{e:#?}"));
    assert!(n >= 10, "examined only {n} packages");
}
```

- [ ] **Step 2: Run it to verify it fails**

Run: `cargo test -p vibe-parity --test workspace_graph 2>&1 | grep -E 'error\[E0432\]|unresolved'`
Expected: `unresolved import vibe_parity::workspace`.

- [ ] **Step 3: Write the checks**

Create `crates/vibe-parity/src/workspace.rs`:

```rust
//! Structural checks over `cargo metadata` output (D19, D20).

use serde_json::Value;

/// The only crates the workspace may hold (sorted).
pub const MEMBERS: [&str; 3] = ["vibe-core", "vibe-parity", "vibe-spec"];
/// The shared library; the one member other members may depend on.
pub const CORE: &str = "vibe-core";
/// External crates a member may depend on directly (normal, dev or build).
pub const ALLOWED_EXTERNAL: [&str; 3] = ["clap", "serde", "serde_json"];
/// Minimum supported Rust version, as `(major, minor, patch)`.
pub const MSRV: (u64, u64, u64) = (1, 88, 0);

fn packages(meta: &Value) -> &[Value] {
    meta.get("packages")
        .and_then(Value::as_array)
        .map_or(&[], Vec::as_slice)
}

fn str_field<'a>(v: &'a Value, key: &str) -> &'a str {
    v.get(key).and_then(Value::as_str).unwrap_or("?")
}

/// Checks `cargo metadata --no-deps --format-version 1` output: the member
/// set is exactly [`MEMBERS`], only [`CORE`] may be depended on by another
/// member, core depends on no member, and every external dependency is in
/// [`ALLOWED_EXTERNAL`]. Returns how many members were examined.
pub fn check_graph(meta: &Value) -> Result<usize, Vec<String>> {
    let mut errors = Vec::new();
    let pkgs = packages(meta);
    let mut names: Vec<&str> = pkgs.iter().map(|p| str_field(p, "name")).collect();
    names.sort_unstable();
    if names != MEMBERS {
        errors.push(format!("workspace members are {names:?}, want {MEMBERS:?}"));
    }
    for pkg in pkgs {
        let name = str_field(pkg, "name");
        let deps = pkg
            .get("dependencies")
            .and_then(Value::as_array)
            .map_or(&[][..], Vec::as_slice);
        for dep in deps {
            let dep_name = str_field(dep, "name");
            let is_path = dep.get("path").is_some_and(Value::is_string);
            if MEMBERS.contains(&dep_name) {
                if dep_name != CORE || name == CORE {
                    errors.push(format!("{name} depends on tool crate {dep_name}"));
                }
            } else if is_path {
                errors.push(format!(
                    "{name} has a path dependency outside the workspace: {dep_name}"
                ));
            } else if !ALLOWED_EXTERNAL.contains(&dep_name) {
                errors.push(format!(
                    "{name} depends on {dep_name}, outside the allowlist {ALLOWED_EXTERNAL:?}"
                ));
            }
        }
    }
    if errors.is_empty() {
        Ok(pkgs.len())
    } else {
        Err(errors)
    }
}

fn parse_version(v: &str) -> Option<(u64, u64, u64)> {
    let mut parts = v.split('.').map(str::parse::<u64>);
    let major = parts.next()?.ok()?;
    let minor = parts.next().unwrap_or(Ok(0)).ok()?;
    let patch = parts.next().unwrap_or(Ok(0)).ok()?;
    Some((major, minor, patch))
}

/// Checks full `cargo metadata --format-version 1` output: every resolved
/// package declares a `rust_version` no newer than [`MSRV`]. A package
/// without one fails, because it cannot be proven to build at MSRV.
/// Returns how many packages were examined.
pub fn check_msrv(meta: &Value) -> Result<usize, Vec<String>> {
    let mut errors = Vec::new();
    let pkgs = packages(meta);
    for pkg in pkgs {
        let name = str_field(pkg, "name");
        match pkg.get("rust_version").and_then(Value::as_str) {
            None => errors.push(format!("{name} declares no rust_version")),
            Some(v) => match parse_version(v) {
                Some(rv) if rv <= MSRV => {}
                Some(_) => errors.push(format!("{name} needs Rust {v}, above MSRV {MSRV:?}")),
                None => errors.push(format!("{name} has an unreadable rust_version {v:?}")),
            },
        }
    }
    if pkgs.is_empty() {
        errors.push("cargo metadata listed no packages".to_string());
    }
    if errors.is_empty() {
        Ok(pkgs.len())
    } else {
        Err(errors)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::{check_graph, check_msrv};
    use serde_json::{Value, json};

    fn reg(name: &str) -> Value {
        json!({"name": name, "source": "registry+https://github.com/rust-lang/crates.io-index"})
    }

    fn path(name: &str) -> Value {
        json!({"name": name, "source": null, "path": format!("/w/crates/{name}")})
    }

    fn meta(spec_deps: Vec<Value>, core_deps: Vec<Value>) -> Value {
        json!({"packages": [
            {"name": "vibe-core", "dependencies": core_deps},
            {"name": "vibe-spec", "dependencies": spec_deps},
            {"name": "vibe-parity", "dependencies": [path("vibe-core"), reg("serde_json")]},
        ]})
    }

    #[test]
    fn the_intended_graph_passes() {
        let m = meta(vec![path("vibe-core"), reg("clap")], vec![]);
        assert_eq!(check_graph(&m).unwrap(), 3);
    }

    #[test]
    fn a_tool_crate_depending_on_another_fails() {
        let m = meta(vec![path("vibe-parity")], vec![]);
        let errs = check_graph(&m).unwrap_err();
        assert!(
            errs[0].contains("vibe-spec depends on tool crate vibe-parity"),
            "{errs:?}"
        );
    }

    #[test]
    fn core_depending_on_a_tool_crate_fails() {
        let m = meta(vec![], vec![path("vibe-spec")]);
        assert!(check_graph(&m).is_err());
    }

    #[test]
    fn a_banned_external_crate_fails() {
        let m = meta(vec![reg("regex")], vec![]);
        let errs = check_graph(&m).unwrap_err();
        assert!(errs[0].contains("regex"), "{errs:?}");
    }

    #[test]
    fn an_outside_path_dependency_fails() {
        let m = meta(vec![path("vibe-flow")], vec![]);
        assert!(check_graph(&m).is_err());
    }

    #[test]
    fn an_extra_or_missing_member_fails() {
        let mut m = meta(vec![], vec![]);
        m["packages"]
            .as_array_mut()
            .unwrap()
            .push(json!({"name": "vibe-flow", "dependencies": []}));
        assert!(check_graph(&m).is_err());
        assert!(check_graph(&json!({"packages": []})).is_err());
    }

    #[test]
    fn msrv_accepts_older_and_equal_and_rejects_newer_or_missing() {
        let ok = json!({"packages": [
            {"name": "a", "rust_version": "1.56"},
            {"name": "b", "rust_version": "1.88.0"},
        ]});
        assert_eq!(check_msrv(&ok).unwrap(), 2);
        let newer = json!({"packages": [{"name": "c", "rust_version": "1.89"}]});
        assert!(check_msrv(&newer).unwrap_err()[0].contains("above MSRV"));
        let missing = json!({"packages": [{"name": "d", "rust_version": null}]});
        assert!(check_msrv(&missing).unwrap_err()[0].contains("no rust_version"));
        assert!(check_msrv(&json!({"packages": []})).is_err());
    }
}
```

Replace `crates/vibe-parity/src/lib.rs` with:

```rust
//! Differential parity harness (F3) and workspace structure checks (F4).
pub mod workspace;
```

- [ ] **Step 4: Run the checks**

```bash
cargo test -p vibe-parity 2>&1 | grep -E 'test result|FAILED'
cargo clippy --workspace --all-targets -- -D warnings 2>&1 | tail -1
cargo fmt --all --check && echo fmt-ok
```
Expected: `ok. 7 passed` (unit), `ok. 2 passed` (`member_graph_holds`, `every_resolved_package_builds_at_msrv`).

- [ ] **Step 5: Add the Rust CI jobs**

Append to `.github/workflows/ci.yml`:

```yaml
  rust-check:
    # F4: format, lint and dependency policy. cargo-deny enforces deny.toml:
    # banned crates, licenses, sources, duplicate versions, advisories.
    name: rust — fmt, clippy, deny
    runs-on: ubuntu-latest
    env:
      # fmt and clippy run on stable, the toolchain they were authored
      # against; rust-test's 1.88 leg proves MSRV.
      RUSTUP_TOOLCHAIN: stable
    steps:
      - name: Checkout
        uses: actions/checkout@v4
        with:
          persist-credentials: false

      - name: Toolchain
        run: rustup toolchain install stable --profile minimal --component clippy,rustfmt

      - name: Format
        run: cargo fmt --all --check

      - name: Clippy
        run: cargo clippy --workspace --all-targets --locked -- -D warnings

      - name: cargo-deny
        uses: EmbarkStudios/cargo-deny-action@v2

  rust-test:
    # F4: unit + integration tests on 3 OS × stable + MSRV. RUSTUP_TOOLCHAIN
    # overrides rust-toolchain.toml so the stable leg really runs stable.
    # Tests that spawn bash or compare the corpus byte-for-byte are
    # cfg(unix); Windows runs the rest.
    name: rust — test (${{ matrix.os }}, ${{ matrix.toolchain }})
    strategy:
      fail-fast: false
      matrix:
        os: [ubuntu-latest, macos-latest, windows-latest]
        toolchain: [stable, '1.88']
    runs-on: ${{ matrix.os }}
    env:
      RUSTUP_TOOLCHAIN: ${{ matrix.toolchain }}
    steps:
      - name: Checkout
        uses: actions/checkout@v4
        with:
          persist-credentials: false

      - name: Toolchain
        run: rustup toolchain install ${{ matrix.toolchain }} --profile minimal

      - name: Test
        run: cargo test --workspace --locked
```

Validate: `python3 -c 'import yaml; d=yaml.safe_load(open(".github/workflows/ci.yml")); print(list(d["jobs"]))'`
Expected: `['check', 'macos', 'rust-check', 'rust-test']`.

- [ ] **Step 6: Commit**

```bash
git add crates/vibe-parity .github/workflows/ci.yml
git commit -m "test(core): assert workspace graph and msrv"
```

---

### Task 7: F3 — differential parity harness

**Files:**
- Create: `crates/vibe-parity/src/{suite.rs,sandbox.rs,harness.rs,main.rs}`, `crates/vibe-parity/src/bin/parity-fixture.rs`, `crates/vibe-parity/tests/fixture/{suite.json,oracle/echo.sh,corpus/basic/one/case.json,corpus/basic/one/tree/seed.txt,corpus/basic/two/case.json,corpus/basic/two/tree/seed.txt}`, `crates/vibe-parity/tests/harness.rs`
- Modify: `crates/vibe-parity/src/lib.rs`, `crates/vibe-parity/Cargo.toml` (`default-run`)

**Interfaces:**
- Produces: `Suite::load(&Path) -> Result<Suite, String>`; `Suite::cases(&str) -> Result<Vec<Case>, String>`; `Case::{id, tree, arg_sets}`; `sandbox::{stage, copy_dir, snapshot, ORACLE_DIR, GIT_DIR}`; `harness::{Mode::{SelfCheck, Candidate(PathBuf)}, run_suite(&Suite, &Mode, work: &Path) -> Report, execute, compare, Outcome, Status, RunResult, Report}`; CLI `vibe-parity run <suite.json> (--self | --candidate <binary>)` → exit 0 ok, 1 failures or suite errors, 2 usage.
- Suite JSON: `{v: 1, name, corpus, oracle_root, commands: [{name, oracle: [argv…], candidate: [argv…], classes: [..]}], floors: {class: n}, divergences: [{command, case: "<class>/<case>", field: "exit"|"stdout"|"files", reason}]}`; `{oracle}` and `{candidate}` expand inside argv. Case JSON: `{v: 1, cwd?: "rel/dir", runs?: {command: [[args…], …]}}`, files under `tree/`.

- [ ] **Step 1: Write the fixture**

Create `crates/vibe-parity/tests/fixture/suite.json`:

```json
{
  "v": 1,
  "name": "fixture",
  "corpus": "corpus",
  "oracle_root": "oracle",
  "commands": [
    { "name": "echo", "oracle": ["bash", "{oracle}/echo.sh"], "candidate": ["{candidate}"], "classes": ["basic"] }
  ],
  "floors": { "basic": 2 }
}
```

Create `crates/vibe-parity/tests/fixture/oracle/echo.sh`:

```bash
#!/usr/bin/env bash
# Parity fixture oracle: echoes its arguments and writes out.txt.
set -euo pipefail
printf 'args:%s\n' "$*"
printf 'x\n' >out.txt
```

Create the cases:

```bash
F=crates/vibe-parity/tests/fixture/corpus/basic
mkdir -p "$F/one/tree" "$F/two/tree"
printf '{ "v": 1, "runs": { "echo": [["a"], ["b", "c"]] } }\n' >"$F/one/case.json"
printf 'one\n' >"$F/one/tree/seed.txt"
printf '{ "v": 1 }\n' >"$F/two/case.json"
printf 'two\n' >"$F/two/tree/seed.txt"
```

Create the test candidate `crates/vibe-parity/src/bin/parity-fixture.rs`:

```rust
//! Test candidate for the parity harness. Reproduces
//! `tests/fixture/oracle/echo.sh`; `--mutate=<exit|stdout|file>` makes it
//! diverge on purpose so tests can prove the harness turns red.
use std::process::ExitCode;

fn main() -> ExitCode {
    let mut mutate = String::new();
    let mut args = Vec::new();
    for arg in std::env::args().skip(1) {
        match arg.strip_prefix("--mutate=") {
            Some(m) => mutate = m.to_string(),
            None => args.push(arg),
        }
    }
    let mark = if mutate == "stdout" { "!" } else { "" };
    println!("args:{}{mark}", args.join(" "));
    let body = if mutate == "file" { "y\n" } else { "x\n" };
    if std::fs::write("out.txt", body).is_err() {
        return ExitCode::from(1);
    }
    ExitCode::from(if mutate == "exit" { 3 } else { 0 })
}
```

- [ ] **Step 2: Write the failing tests**

Create `crates/vibe-parity/tests/harness.rs`:

```rust
#![cfg(unix)]
#![allow(clippy::unwrap_used, clippy::expect_used)]
//! The harness passes a faithful candidate and fails each planted mutant.
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::{Value, json};
use vibe_parity::harness::{Mode, Report, Status, execute, run_suite};
use vibe_parity::suite::Suite;

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixture")
}

fn work(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("vibe-parity-test-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn candidate() -> Mode {
    Mode::Candidate(PathBuf::from(env!("CARGO_BIN_EXE_parity-fixture")))
}

/// The fixture suite as JSON, with absolute corpus and oracle paths.
fn base() -> Value {
    let mut v: Value =
        serde_json::from_str(&fs::read_to_string(fixture().join("suite.json")).unwrap()).unwrap();
    v["corpus"] = json!(fixture().join("corpus"));
    v["oracle_root"] = json!(fixture().join("oracle"));
    v
}

fn run(name: &str, suite: Value, mode: &Mode) -> Report {
    let dir = work(name);
    let path = dir.join("suite.json");
    fs::write(&path, serde_json::to_vec(&suite).unwrap()).unwrap();
    let suite = Suite::load(&path).unwrap();
    run_suite(&suite, mode, &dir.join("work"))
}

#[test]
fn self_check_passes_over_every_run() {
    let report = run("self", base(), &Mode::SelfCheck);
    assert!(report.ok(), "{report:#?}");
    assert_eq!(report.results.len(), 3);
}

#[test]
fn a_faithful_candidate_passes() {
    let report = run("faithful", base(), &candidate());
    assert!(report.ok(), "{report:#?}");
    assert_eq!(report.passed(), 3);
}

#[test]
fn each_mutant_fails_on_its_field() {
    for (flag, word) in [
        ("--mutate=stdout", "stdout"),
        ("--mutate=exit", "exit"),
        ("--mutate=file", "files"),
    ] {
        let mut suite = base();
        suite["commands"][0]["candidate"] = json!(["{candidate}", flag]);
        let report = run(&format!("mutant-{word}"), suite, &candidate());
        assert!(!report.ok(), "{flag} passed");
        assert_eq!(report.failed(), 3, "{flag}: {report:#?}");
        for r in &report.results {
            let Status::Fail(msgs) = &r.status else {
                panic!("{flag}: {r:?}")
            };
            assert!(msgs.iter().any(|m| m.starts_with(word)), "{flag}: {msgs:?}");
        }
    }
}

#[test]
fn a_floor_above_the_corpus_fails() {
    let mut suite = base();
    suite["floors"]["basic"] = json!(3);
    let report = run("floor", suite, &Mode::SelfCheck);
    assert!(!report.ok());
    assert!(
        report
            .errors
            .iter()
            .any(|e| e == "class basic: 2 case(s), floor 3"),
        "{:?}",
        report.errors
    );
}

#[test]
fn a_class_without_a_floor_or_with_floor_zero_fails() {
    let mut suite = base();
    suite["floors"] = json!({});
    assert!(
        run("nofloor", suite, &Mode::SelfCheck)
            .errors
            .iter()
            .any(|e| e.contains("has no floor"))
    );
    let mut suite = base();
    suite["floors"]["basic"] = json!(0);
    assert!(
        run("zerofloor", suite, &Mode::SelfCheck)
            .errors
            .iter()
            .any(|e| e.contains("asserts nothing"))
    );
}

#[test]
fn an_empty_class_fails() {
    let mut suite = base();
    suite["commands"][0]["classes"] = json!(["absent"]);
    suite["floors"] = json!({ "absent": 1 });
    let report = run("empty", suite, &Mode::SelfCheck);
    assert!(!report.ok());
    assert!(report.results.is_empty());
}

#[test]
fn pinned_divergences_pass_and_stale_pins_fail() {
    let pins = json!([
        { "command": "echo", "case": "basic/one", "field": "stdout", "reason": "test" },
        { "command": "echo", "case": "basic/two", "field": "stdout", "reason": "test" }
    ]);
    let mut suite = base();
    suite["divergences"] = pins.clone();
    suite["commands"][0]["candidate"] = json!(["{candidate}", "--mutate=stdout"]);
    let report = run("pinned", suite, &candidate());
    assert!(report.ok(), "{report:#?}");
    assert_eq!(report.pinned(), 3);

    let mut suite = base();
    suite["divergences"] = pins;
    let report = run("stale", suite, &candidate());
    assert!(!report.ok());
    assert_eq!(
        report
            .errors
            .iter()
            .filter(|e| e.starts_with("stale pin"))
            .count(),
        2
    );
}

#[test]
fn the_cli_reports_and_exits_by_verdict() {
    let bin = env!("CARGO_BIN_EXE_vibe-parity");
    let suite = fixture().join("suite.json");
    let ok = Command::new(bin)
        .arg("run")
        .arg(&suite)
        .arg("--self")
        .output()
        .unwrap();
    assert_eq!(
        ok.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&ok.stdout)
    );
    assert!(
        String::from_utf8_lossy(&ok.stdout)
            .contains("parity[fixture]: 3 passed, 0 pinned, 0 failed")
    );
    let bad = Command::new(bin).arg("run").arg(&suite).output().unwrap();
    assert_eq!(bad.status.code(), Some(2));
}

#[test]
fn runs_see_only_path_home_locale_and_timezone() {
    let dir = work("env");
    let (code, out) = execute(&["env".to_string()], &dir, &dir).unwrap();
    assert_eq!(code, Some(0));
    let mut keys: Vec<String> = String::from_utf8(out)
        .unwrap()
        .lines()
        .filter_map(|l| l.split_once('=').map(|(k, _)| k.to_string()))
        .collect();
    keys.sort();
    assert_eq!(keys, ["HOME", "LC_ALL", "PATH", "TZ"]);
}

#[test]
fn a_symlink_in_the_corpus_is_refused() {
    let dir = work("symlink");
    let corpus = dir.join("corpus/basic/one");
    fs::create_dir_all(corpus.join("tree")).unwrap();
    fs::write(corpus.join("case.json"), "{ \"v\": 1 }\n").unwrap();
    std::os::unix::fs::symlink("/etc/hosts", corpus.join("tree/link")).unwrap();
    let mut suite = base();
    suite["corpus"] = json!(dir.join("corpus"));
    suite["floors"]["basic"] = json!(1);
    let report = run("symlink-run", suite, &Mode::SelfCheck);
    assert!(!report.ok());
    let Status::Fail(msgs) = &report.results[0].status else {
        panic!("{report:#?}")
    };
    assert!(msgs[0].contains("symlinks are not allowed"), "{msgs:?}");
}
```

- [ ] **Step 3: Run them to verify they fail**

Run: `cargo test -p vibe-parity --test harness 2>&1 | grep -E 'unresolved|E0432' | head -3`
Expected: `unresolved imports vibe_parity::harness`, `vibe_parity::suite`.

- [ ] **Step 4: Write the harness**

Create `crates/vibe-parity/src/suite.rs`:

```rust
//! Parity suite files and corpus cases.
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

/// One parity suite: which commands run over which corpus classes, and the floors.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Suite {
    /// Suite file format major version; must be 1.
    pub v: u32,
    /// Name printed in the summary line.
    pub name: String,
    /// Corpus directory (`<class>/<case>/case.json`), relative to the suite file.
    pub corpus: PathBuf,
    /// Directory staged into every sandbox as `.oracle/`, relative to the suite file.
    pub oracle_root: PathBuf,
    /// Commands to compare.
    pub commands: Vec<CommandSpec>,
    /// Minimum case count per corpus class. Every class a command uses needs one.
    pub floors: BTreeMap<String, usize>,
    /// Known, deliberate differences between oracle and candidate.
    #[serde(default)]
    pub divergences: Vec<Divergence>,
}

/// One command compared between oracle and candidate.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CommandSpec {
    /// Command name; also the key into a case's `runs`.
    pub name: String,
    /// Oracle argv. `{oracle}` expands to the staged oracle directory.
    pub oracle: Vec<String>,
    /// Candidate argv. `{candidate}` expands to the candidate binary.
    pub candidate: Vec<String>,
    /// Corpus classes this command runs over.
    pub classes: Vec<String>,
}

/// What a run compares.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Field {
    /// The exit code.
    Exit,
    /// The stdout bytes.
    Stdout,
    /// The bytes of every file left in the sandbox.
    Files,
}

/// A pinned `expected-divergence`: the candidate differs from the oracle on
/// purpose (an oracle bug fixed only in the port, or a deliberate change).
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Divergence {
    /// Command name.
    pub command: String,
    /// Case id, `<class>/<case>`.
    pub case: String,
    /// The field that differs.
    pub field: Field,
    /// Why the difference is correct.
    pub reason: String,
}

/// A corpus case's `case.json`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CaseSpec {
    /// Case file format major version; must be 1.
    pub v: u32,
    /// Working directory inside the sandbox, relative; empty means the sandbox root.
    #[serde(default)]
    pub cwd: String,
    /// Extra argument lists per command. A command without an entry runs once with none.
    #[serde(default)]
    pub runs: BTreeMap<String, Vec<Vec<String>>>,
}

/// A corpus case on disk.
#[derive(Debug)]
pub struct Case {
    /// Class directory name.
    pub class: String,
    /// Case directory name.
    pub name: String,
    /// The case directory.
    pub dir: PathBuf,
    /// The parsed `case.json`.
    pub spec: CaseSpec,
}

impl Case {
    /// `<class>/<case>`.
    pub fn id(&self) -> String {
        format!("{}/{}", self.class, self.name)
    }

    /// The files staged into the sandbox.
    pub fn tree(&self) -> PathBuf {
        self.dir.join("tree")
    }

    /// Argument lists for `command`; one empty list when the case names none.
    pub fn arg_sets(&self, command: &str) -> Vec<Vec<String>> {
        self.spec
            .runs
            .get(command)
            .cloned()
            .unwrap_or_else(|| vec![Vec::new()])
    }
}

impl Suite {
    /// Loads a suite file; `corpus` and `oracle_root` resolve against its directory.
    pub fn load(path: &Path) -> Result<Suite, String> {
        let text = fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let mut suite: Suite =
            serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
        if suite.v != 1 {
            return Err(format!(
                "{}: unsupported suite version {}",
                path.display(),
                suite.v
            ));
        }
        let base = path.parent().unwrap_or_else(|| Path::new("."));
        suite.corpus = base.join(&suite.corpus);
        suite.oracle_root = base.join(&suite.oracle_root);
        Ok(suite)
    }

    /// The cases of `class`, sorted by name. A missing class directory has none.
    pub fn cases(&self, class: &str) -> Result<Vec<Case>, String> {
        let dir = self.corpus.join(class);
        if !dir.is_dir() {
            return Ok(Vec::new());
        }
        let mut cases = Vec::new();
        let entries = fs::read_dir(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        for entry in entries {
            let case_dir = entry.map_err(|e| format!("{}: {e}", dir.display()))?.path();
            let file = case_dir.join("case.json");
            if !file.is_file() {
                continue;
            }
            let text = fs::read_to_string(&file).map_err(|e| format!("{}: {e}", file.display()))?;
            let spec: CaseSpec =
                serde_json::from_str(&text).map_err(|e| format!("{}: {e}", file.display()))?;
            if spec.v != 1 {
                return Err(format!(
                    "{}: unsupported case version {}",
                    file.display(),
                    spec.v
                ));
            }
            let name = case_dir
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            cases.push(Case {
                class: class.to_string(),
                name,
                dir: case_dir,
                spec,
            });
        }
        cases.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(cases)
    }
}
```

Create `crates/vibe-parity/src/sandbox.rs`:

```rust
//! Staging a corpus case into a sandbox and snapshotting the result.
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

/// Where the oracle is staged inside a sandbox.
pub const ORACLE_DIR: &str = ".oracle";
/// An empty directory at the sandbox root that bounds every upward root search.
pub const GIT_DIR: &str = ".git";

fn err(path: &Path, e: std::io::Error) -> String {
    format!("{}: {e}", path.display())
}

/// Recreates `dest` as a copy of `tree` (if it exists), plus the oracle at
/// `dest/.oracle/` and an empty `dest/.git/`.
pub fn stage(tree: &Path, oracle_root: &Path, dest: &Path) -> Result<(), String> {
    if dest.exists() {
        fs::remove_dir_all(dest).map_err(|e| err(dest, e))?;
    }
    fs::create_dir_all(dest).map_err(|e| err(dest, e))?;
    if tree.is_dir() {
        copy_dir(tree, dest)?;
    }
    copy_dir(oracle_root, &dest.join(ORACLE_DIR))?;
    let git = dest.join(GIT_DIR);
    fs::create_dir_all(&git).map_err(|e| err(&git, e))
}

/// Copies the directory `from` into `to` (created if missing). Symlinks are refused.
pub fn copy_dir(from: &Path, to: &Path) -> Result<(), String> {
    fs::create_dir_all(to).map_err(|e| err(to, e))?;
    for entry in fs::read_dir(from).map_err(|e| err(from, e))? {
        let src = entry.map_err(|e| err(from, e))?.path();
        let Some(name) = src.file_name() else {
            continue;
        };
        let dst = to.join(name);
        let kind = fs::symlink_metadata(&src)
            .map_err(|e| err(&src, e))?
            .file_type();
        if kind.is_symlink() {
            return Err(format!(
                "{}: symlinks are not allowed in a corpus or oracle",
                src.display()
            ));
        } else if kind.is_dir() {
            copy_dir(&src, &dst)?;
        } else {
            fs::copy(&src, &dst).map_err(|e| err(&src, e))?;
        }
    }
    Ok(())
}

/// Maps every file under `root`, except `.oracle/` and `.git/` at the top,
/// to its bytes. Keys are relative paths joined with `/`.
pub fn snapshot(root: &Path) -> Result<BTreeMap<String, Vec<u8>>, String> {
    let mut files = BTreeMap::new();
    walk(root, "", &mut files)?;
    Ok(files)
}

fn walk(dir: &Path, prefix: &str, files: &mut BTreeMap<String, Vec<u8>>) -> Result<(), String> {
    for entry in fs::read_dir(dir).map_err(|e| err(dir, e))? {
        let path = entry.map_err(|e| err(dir, e))?.path();
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        if prefix.is_empty() && (name == ORACLE_DIR || name == GIT_DIR) {
            continue;
        }
        let rel = if prefix.is_empty() {
            name
        } else {
            format!("{prefix}/{name}")
        };
        let kind = fs::symlink_metadata(&path)
            .map_err(|e| err(&path, e))?
            .file_type();
        if kind.is_dir() {
            walk(&path, &rel, files)?;
        } else {
            let bytes = if kind.is_symlink() {
                fs::read_link(&path)
                    .map_err(|e| err(&path, e))?
                    .to_string_lossy()
                    .into_owned()
                    .into_bytes()
            } else {
                fs::read(&path).map_err(|e| err(&path, e))?
            };
            files.insert(rel, bytes);
        }
    }
    Ok(())
}
```

Create `crates/vibe-parity/src/harness.rs`:

```rust
//! Runs oracle and candidate over a suite's corpus and compares the results.
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::sandbox::{ORACLE_DIR, snapshot, stage};
use crate::suite::{Case, CommandSpec, Field, Suite};

/// What the candidate side of each run is.
#[derive(Debug, Clone)]
pub enum Mode {
    /// Run the oracle twice. Proves the corpus and oracle are deterministic.
    SelfCheck,
    /// Run the candidate binary at this path.
    Candidate(PathBuf),
}

/// What one side of a run produced.
#[derive(Debug, PartialEq, Eq)]
pub struct Outcome {
    /// Exit code; `None` when killed by a signal.
    pub exit: Option<i32>,
    /// Stdout bytes.
    pub stdout: Vec<u8>,
    /// Every file left in the sandbox (see [`snapshot`]).
    pub files: BTreeMap<String, Vec<u8>>,
}

/// The verdict for one run.
#[derive(Debug, PartialEq, Eq)]
pub enum Status {
    /// Oracle and candidate agree.
    Pass,
    /// They differ only on pinned fields.
    Pinned(Vec<Field>),
    /// They differ on an unpinned field, or the run could not execute.
    Fail(Vec<String>),
}

/// One command over one case with one argument list.
#[derive(Debug)]
pub struct RunResult {
    /// Command name.
    pub command: String,
    /// Case id, `<class>/<case>`.
    pub case: String,
    /// Extra arguments for this run.
    pub args: Vec<String>,
    /// The verdict.
    pub status: Status,
}

/// Everything a suite run found.
#[derive(Debug, Default)]
pub struct Report {
    /// One entry per run.
    pub results: Vec<RunResult>,
    /// Suite-level problems: floors, unknown classes, stale or unmatched pins.
    pub errors: Vec<String>,
}

impl Report {
    fn count(&self, f: impl Fn(&Status) -> bool) -> usize {
        self.results.iter().filter(|r| f(&r.status)).count()
    }

    /// Runs that passed.
    pub fn passed(&self) -> usize {
        self.count(|s| matches!(s, Status::Pass))
    }

    /// Runs that differ only on pinned fields.
    pub fn pinned(&self) -> usize {
        self.count(|s| matches!(s, Status::Pinned(_)))
    }

    /// Runs that failed.
    pub fn failed(&self) -> usize {
        self.count(|s| matches!(s, Status::Fail(_)))
    }

    /// True when at least one run happened, none failed, and there are no suite errors.
    pub fn ok(&self) -> bool {
        !self.results.is_empty() && self.failed() == 0 && self.errors.is_empty()
    }
}

fn expand(argv: &[String], oracle: &Path, candidate: Option<&Path>) -> Vec<String> {
    let oracle = oracle.to_string_lossy();
    let candidate = candidate
        .map(|c| c.to_string_lossy().into_owned())
        .unwrap_or_default();
    argv.iter()
        .map(|a| {
            a.replace("{oracle}", &oracle)
                .replace("{candidate}", &candidate)
        })
        .collect()
}

/// Runs `argv` in `cwd` with a cleared environment: only `PATH`, `HOME`,
/// `LC_ALL=C` and `TZ=UTC` are set, and stdin is empty. Returns the exit
/// code and stdout.
pub fn execute(argv: &[String], cwd: &Path, home: &Path) -> Result<(Option<i32>, Vec<u8>), String> {
    let (program, args) = argv.split_first().ok_or("empty argv")?;
    let out = Command::new(program)
        .args(args)
        .current_dir(cwd)
        .env_clear()
        .env("PATH", std::env::var_os("PATH").unwrap_or_default())
        .env("HOME", home)
        .env("LC_ALL", "C")
        .env("TZ", "UTC")
        .stdin(Stdio::null())
        .output()
        .map_err(|e| format!("{program}: {e}"))?;
    Ok((out.status.code(), out.stdout))
}

fn run_side(suite: &Suite, case: &Case, argv: &[String], work: &Path) -> Result<Outcome, String> {
    let dest = work.join("sandbox");
    let home = work.join("home");
    stage(&case.tree(), &suite.oracle_root, &dest)?;
    if home.exists() {
        fs::remove_dir_all(&home).map_err(|e| format!("{}: {e}", home.display()))?;
    }
    fs::create_dir_all(&home).map_err(|e| format!("{}: {e}", home.display()))?;
    let cwd = dest.join(&case.spec.cwd);
    if !cwd.is_dir() {
        return Err(format!(
            "{}: cwd {:?} is not a directory in the tree",
            case.id(),
            case.spec.cwd
        ));
    }
    let (exit, stdout) = execute(argv, &cwd, &home)?;
    let files = snapshot(&dest)?;
    Ok(Outcome {
        exit,
        stdout,
        files,
    })
}

fn first_line_diff(a: &[u8], b: &[u8]) -> String {
    let (la, lb): (Vec<&[u8]>, Vec<&[u8]>) = (
        a.split(|&c| c == b'\n').collect(),
        b.split(|&c| c == b'\n').collect(),
    );
    let n = la.iter().zip(&lb).take_while(|(x, y)| x == y).count();
    let show = |l: &Vec<&[u8]>| l.get(n).map(|s| String::from_utf8_lossy(s).into_owned());
    format!(
        "stdout line {}: oracle {:?}, candidate {:?}",
        n + 1,
        show(&la),
        show(&lb)
    )
}

/// Lists the fields on which `oracle` and `candidate` differ, each with a message.
pub fn compare(oracle: &Outcome, candidate: &Outcome) -> Vec<(Field, String)> {
    let mut diffs = Vec::new();
    if oracle.exit != candidate.exit {
        diffs.push((
            Field::Exit,
            format!(
                "exit: oracle {:?}, candidate {:?}",
                oracle.exit, candidate.exit
            ),
        ));
    }
    if oracle.stdout != candidate.stdout {
        diffs.push((
            Field::Stdout,
            first_line_diff(&oracle.stdout, &candidate.stdout),
        ));
    }
    if oracle.files != candidate.files {
        let keys: BTreeSet<&String> = oracle.files.keys().chain(candidate.files.keys()).collect();
        let differing: Vec<&str> = keys
            .into_iter()
            .filter(|k| oracle.files.get(*k) != candidate.files.get(*k))
            .map(String::as_str)
            .collect();
        diffs.push((Field::Files, format!("files differ: {differing:?}")));
    }
    diffs
}

fn check_floors(suite: &Suite, errors: &mut Vec<String>) {
    let used: BTreeSet<&str> = suite
        .commands
        .iter()
        .flat_map(|c| c.classes.iter().map(String::as_str))
        .collect();
    for class in &used {
        if !suite.floors.contains_key(*class) {
            errors.push(format!("class {class} has no floor"));
        }
    }
    for (class, &floor) in &suite.floors {
        if !used.contains(class.as_str()) {
            errors.push(format!("floor for class {class}, which no command uses"));
        }
        if floor == 0 {
            errors.push(format!("class {class}: a floor of 0 asserts nothing"));
            continue;
        }
        match suite.cases(class) {
            Ok(cases) if cases.len() < floor => {
                errors.push(format!(
                    "class {class}: {} case(s), floor {floor}",
                    cases.len()
                ));
            }
            Ok(_) => {}
            Err(e) => errors.push(e),
        }
    }
}

fn judge(diffs: Vec<(Field, String)>, pins: &[(usize, Field)], hits: &mut [bool]) -> Status {
    let mut unpinned = Vec::new();
    let mut pinned = Vec::new();
    for (field, msg) in diffs {
        match pins.iter().find(|(_, f)| *f == field) {
            Some(&(i, f)) => {
                if let Some(hit) = hits.get_mut(i) {
                    *hit = true;
                }
                pinned.push(f);
            }
            None => unpinned.push(msg),
        }
    }
    if !unpinned.is_empty() {
        Status::Fail(unpinned)
    } else if pinned.is_empty() {
        Status::Pass
    } else {
        Status::Pinned(pinned)
    }
}

fn run_command(
    suite: &Suite,
    cmd: &CommandSpec,
    mode: &Mode,
    work: &Path,
    report: &mut Report,
    hits: &mut [bool],
) {
    let oracle_dir = work.join("sandbox").join(ORACLE_DIR);
    let candidate = match mode {
        Mode::SelfCheck => None,
        Mode::Candidate(path) => Some(path.as_path()),
    };
    let oracle_argv = expand(&cmd.oracle, &oracle_dir, None);
    let candidate_argv = match candidate {
        None => oracle_argv.clone(),
        Some(_) => expand(&cmd.candidate, &oracle_dir, candidate),
    };
    for class in &cmd.classes {
        let cases = match suite.cases(class) {
            Ok(cases) => cases,
            Err(e) => {
                report.errors.push(e);
                continue;
            }
        };
        for case in &cases {
            let pins: Vec<(usize, Field)> = match mode {
                Mode::SelfCheck => Vec::new(),
                Mode::Candidate(_) => suite
                    .divergences
                    .iter()
                    .enumerate()
                    .filter(|(_, d)| d.command == cmd.name && d.case == case.id())
                    .map(|(i, d)| (i, d.field))
                    .collect(),
            };
            for args in case.arg_sets(&cmd.name) {
                let with = |base: &[String]| base.iter().chain(&args).cloned().collect::<Vec<_>>();
                let outcome = run_side(suite, case, &with(&oracle_argv), work).and_then(|o| {
                    run_side(suite, case, &with(&candidate_argv), work).map(|c| (o, c))
                });
                let status = match outcome {
                    Ok((o, c)) => judge(compare(&o, &c), &pins, hits),
                    Err(e) => Status::Fail(vec![e]),
                };
                report.results.push(RunResult {
                    command: cmd.name.clone(),
                    case: case.id(),
                    args,
                    status,
                });
            }
        }
    }
}

/// Runs every command of `suite` over its classes. `work` is a scratch
/// directory; each run stages its sandbox at the same path, so absolute paths
/// in output match between oracle and candidate.
pub fn run_suite(suite: &Suite, mode: &Mode, work: &Path) -> Report {
    let mut report = Report::default();
    check_floors(suite, &mut report.errors);
    let mut hits = vec![false; suite.divergences.len()];
    for cmd in &suite.commands {
        run_command(suite, cmd, mode, work, &mut report, &mut hits);
    }
    if matches!(mode, Mode::Candidate(_)) {
        for (pin, hit) in suite.divergences.iter().zip(&hits) {
            if !hit {
                report.errors.push(format!(
                    "stale pin: {} {} {:?} no longer diverges (or matches no run)",
                    pin.command, pin.case, pin.field
                ));
            }
        }
    }
    report
}
```

Replace `crates/vibe-parity/src/lib.rs` with:

```rust
//! Differential parity harness (F3) and workspace structure checks (F4).
//!
//! A suite names commands, an oracle argv and a candidate argv for each, and
//! the corpus classes they run over. Every run stages a case into a sandbox,
//! runs the oracle, restages, runs the candidate, and compares exit code,
//! stdout bytes and the bytes of every file left behind.
pub mod harness;
pub mod sandbox;
pub mod suite;
pub mod workspace;
```

Create `crates/vibe-parity/src/main.rs`:

```rust
//! `vibe-parity`: run a parity suite.
//!
//! Usage:
//!   vibe-parity run <suite.json> --self
//!   vibe-parity run <suite.json> --candidate <binary>
//!
//! Exit 0 when every run passes or is pinned, 1 on any failure or suite
//! error, 2 on a usage error.
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use vibe_core::exit;
use vibe_parity::harness::{Mode, Status, run_suite};
use vibe_parity::suite::Suite;

const USAGE: &str = "usage: vibe-parity run <suite.json> (--self | --candidate <binary>)";

fn usage(msg: &str) -> ExitCode {
    eprintln!("vibe-parity: {msg}\n{USAGE}");
    ExitCode::from(exit::USAGE)
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let rest: Vec<&str> = args.iter().map(String::as_str).collect();
    match rest.as_slice() {
        ["run", suite, "--self"] => run(Path::new(suite), Mode::SelfCheck),
        ["run", suite, "--candidate", bin] => match std::fs::canonicalize(bin) {
            Ok(path) => run(Path::new(suite), Mode::Candidate(path)),
            Err(e) => usage(&format!("candidate {bin}: {e}")),
        },
        _ => usage("bad arguments"),
    }
}

fn run(suite_path: &Path, mode: Mode) -> ExitCode {
    let suite = match Suite::load(suite_path) {
        Ok(suite) => suite,
        Err(e) => return usage(&e),
    };
    let work: PathBuf = std::env::temp_dir().join(format!("vibe-parity-{}", std::process::id()));
    let report = run_suite(&suite, &mode, &work);
    let _ = std::fs::remove_dir_all(&work);
    for r in &report.results {
        let args = r.args.join(" ");
        match &r.status {
            Status::Pass => println!("PASS  {} {} [{args}]", r.command, r.case),
            Status::Pinned(fields) => {
                println!("PIN   {} {} [{args}] {fields:?}", r.command, r.case)
            }
            Status::Fail(msgs) => {
                println!("FAIL  {} {} [{args}]", r.command, r.case);
                for m in msgs {
                    println!("      {m}");
                }
            }
        }
    }
    for e in &report.errors {
        println!("ERROR {e}");
    }
    let label = match mode {
        Mode::SelfCheck => "self-check".to_string(),
        Mode::Candidate(p) => format!("candidate {}", p.display()),
    };
    println!(
        "parity[{}]: {} passed, {} pinned, {} failed, {} error(s), {} runs ({label})",
        suite.name,
        report.passed(),
        report.pinned(),
        report.failed(),
        report.errors.len(),
        report.results.len()
    );
    ExitCode::from(if report.ok() {
        exit::OK
    } else {
        exit::FINDINGS
    })
}
```

In `crates/vibe-parity/Cargo.toml`, add under `[package]` after `publish.workspace = true`:

```toml
default-run = "vibe-parity"
```

- [ ] **Step 5: Run the checks**

```bash
cargo test -p vibe-parity 2>&1 | grep -E 'test result|FAILED'
cargo clippy --workspace --all-targets -- -D warnings 2>&1 | tail -1
cargo fmt --all --check && echo fmt-ok
shellcheck crates/vibe-parity/tests/fixture/oracle/echo.sh
```
Expected: `ok. 10 passed` (harness), unit and graph tests still ok; clippy no issues; `fmt-ok`; shellcheck silent.

- [ ] **Step 6: Commit**

```bash
git add crates/vibe-parity
git commit -m "feat(parity): add differential parity harness"
```

- [ ] **Step 7: Plant a mutant and prove the harness turns red (F3 acceptance)**

The fixture candidate is committed now, so `git diff --numstat` proves the mutant landed.

```bash
cargo build -q -p vibe-parity
./target/debug/vibe-parity run crates/vibe-parity/tests/fixture/suite.json --candidate target/debug/parity-fixture | tail -1
sed -i.bak 's/"x\\n" }/"X\\n" }/' crates/vibe-parity/src/bin/parity-fixture.rs && rm crates/vibe-parity/src/bin/parity-fixture.rs.bak
git diff --numstat crates/vibe-parity/src/bin/parity-fixture.rs
cargo build -q -p vibe-parity
./target/debug/vibe-parity run crates/vibe-parity/tests/fixture/suite.json --candidate target/debug/parity-fixture | tail -2
git checkout -- crates/vibe-parity/src/bin/parity-fixture.rs
cargo build -q -p vibe-parity
./target/debug/vibe-parity run crates/vibe-parity/tests/fixture/suite.json --candidate target/debug/parity-fixture | tail -1
```
Expected, in order: `parity[fixture]: 3 passed, 0 pinned, 0 failed, …`; `1	1	crates/vibe-parity/src/bin/parity-fixture.rs`; `      files differ: ["out.txt"]` and `parity[fixture]: 0 passed, 0 pinned, 3 failed, …`; after the restore, `3 passed` again and `git status --short` clean. Paste all outputs into the ledger as the F3 mutant evidence.

---

### Task 8: F3 — spec corpus, suite and parity CI

**Files:**
- Create: `crates/vibe-parity/src/corpus_spec.rs`, `crates/vibe-parity/tests/spec_corpus.rs`, `tests/parity/spec.json`, `tests/parity/corpus/spec/**` (generated)
- Modify: `crates/vibe-parity/src/lib.rs`, `crates/vibe-parity/src/main.rs`, `.github/workflows/ci.yml`

**Interfaces:**
- Consumes: `Suite`, `run_suite`, `Mode::SelfCheck`, `execute`, `sandbox::{stage, snapshot, ORACLE_DIR}` (Task 7); `oracles/bash/spec` (Task 1).
- Produces: `corpus_spec::generate(templates: &Path, out: &Path) -> Result<usize, String>`, `corpus_spec::MARKER`; CLI `vibe-parity gen-spec-corpus <templates-dir> <out-dir>`; `tests/parity/spec.json` (F16 adds `--candidate target/debug/vibe-spec` and pins). Classes: `no-root`, `seeded`, `filled`, `lessons`, `features`, `malformed`, floor 2 each.

- [ ] **Step 1: Write the failing tests**

Create `crates/vibe-parity/tests/spec_corpus.rs`:

```rust
#![cfg(unix)]
#![allow(clippy::unwrap_used, clippy::expect_used)]
//! The committed spec corpus is exactly what the generator writes, and the
//! frozen spec oracles are deterministic over it.
use std::fs;
use std::path::{Path, PathBuf};

use vibe_parity::corpus_spec::{MARKER, generate};
use vibe_parity::harness::{Mode, execute, run_suite};
use vibe_parity::sandbox::{ORACLE_DIR, snapshot, stage};
use vibe_parity::suite::Suite;

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn scratch(name: &str) -> PathBuf {
    let dir =
        std::env::temp_dir().join(format!("vibe-parity-corpus-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

const TEMPLATES: &str = "oracles/bash/spec/reference/templates";

#[test]
fn committed_corpus_matches_the_generator() {
    let out = scratch("regen").join("spec");
    let n = generate(&repo().join(TEMPLATES), &out).unwrap();
    assert!(n >= 12, "generated only {n} cases");
    let committed = snapshot(&repo().join("tests/parity/corpus/spec")).unwrap();
    let fresh = snapshot(&out).unwrap();
    assert_eq!(
        committed.keys().collect::<Vec<_>>(),
        fresh.keys().collect::<Vec<_>>(),
        "file sets differ: run `cargo run -p vibe-parity -- gen-spec-corpus {TEMPLATES} tests/parity/corpus/spec`"
    );
    for (path, bytes) in &committed {
        assert_eq!(
            Some(bytes),
            fresh.get(path),
            "{path} differs from the generator output"
        );
    }
}

#[test]
fn the_generator_refuses_a_directory_it_did_not_write() {
    let out = scratch("refuse");
    fs::write(out.join("keep.txt"), b"mine").unwrap();
    let err = generate(&repo().join(TEMPLATES), &out).unwrap_err();
    assert!(err.contains(MARKER), "{err}");
    assert_eq!(fs::read(out.join("keep.txt")).unwrap(), b"mine");
}

#[test]
fn spec_oracles_are_deterministic_over_the_corpus() {
    let suite = Suite::load(&repo().join("tests/parity/spec.json")).unwrap();
    let report = run_suite(&suite, &Mode::SelfCheck, &scratch("self"));
    assert!(report.ok(), "{:#?}", report.errors);
    assert!(
        report.results.len() >= 60,
        "only {} runs",
        report.results.len()
    );
}

#[test]
fn no_root_cases_stop_at_the_sandbox_git_dir() {
    let dir = scratch("escape");
    let dest = dir.join("sandbox");
    let case = repo().join("tests/parity/corpus/spec/no-root/subdir");
    stage(&case.join("tree"), &repo().join("oracles/bash/spec"), &dest).unwrap();
    let script = dest.join(ORACLE_DIR).join("scripts/list-specs.sh");
    let argv = ["bash".to_string(), script.to_string_lossy().into_owned()];
    let (code, out) = execute(&argv, &dest.join("src/deep"), &dir).unwrap();
    assert_eq!(code, Some(0));
    assert_eq!(
        String::from_utf8(out).unwrap(),
        "No .spec/ directory found in current project\n"
    );
}
```

- [ ] **Step 2: Run them to verify they fail**

Run: `cargo test -p vibe-parity --test spec_corpus 2>&1 | grep -E 'unresolved|E0432' | head -2`
Expected: `unresolved import vibe_parity::corpus_spec`.

- [ ] **Step 3: Write the generator**

Create `crates/vibe-parity/src/corpus_spec.rs`:

```rust
//! Generates the spec parity corpus (`tests/parity/corpus/spec/`).
//!
//! Deterministic: the same templates always give the same bytes. Classes:
//! `no-root`, `seeded` (the frozen setup templates), `filled`, `lessons`,
//! `features` (merge blocks, promote) and `malformed`. Flow classes arrive
//! with F5.
use std::fs;
use std::path::Path;

use serde_json::{Map, Value, json};

/// Marker file at the corpus root; the generator only replaces a directory that holds it.
pub const MARKER: &str = ".generated";
const MARKER_TEXT: &str = "Generated by `vibe-parity gen-spec-corpus`. Do not edit; regenerate.\n";
const TEMPLATES: [&str; 4] = ["product", "tech", "design", "plan"];
const LESSONS_HEAD: &str = "# Lessons\n\nTagged mistakes. Read at session start.\n\n";

type Files = Vec<(String, String)>;
type Runs = &'static [(&'static str, &'static [&'static [&'static str]])];

struct CaseDef {
    class: &'static str,
    name: &'static str,
    cwd: &'static str,
    runs: Runs,
    files: Files,
}

fn doc(kind: &str, scope: &str, body: &str) -> String {
    format!("---\ntype: {kind}\nscope: {scope}\nupdated: 2026-01-15\n---\n\n{body}")
}

fn lesson(title: &str, rule: &str, tags: &str) -> String {
    format!(
        "### {title}\n**Pattern:** Seen once in the corpus.\n**Rule:** {rule}\n**Tags:** {tags}\n**Date:** 2026-01-02\n\n"
    )
}

/// `base` with each `(path, text)` in `extra` added, replacing any file at that path.
fn with(mut base: Files, extra: &[(&str, &str)]) -> Files {
    for &(path, text) in extra {
        base.retain(|(p, _)| p != path);
        base.push((path.to_string(), text.to_string()));
    }
    base
}

fn seeded(templates: &Path) -> Result<Files, String> {
    let mut files = Vec::new();
    for t in TEMPLATES {
        let path = templates.join(format!("{t}.md"));
        let text = fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        files.push((format!(".spec/{t}.md"), text));
    }
    files.push((".spec/lessons.md".to_string(), LESSONS_HEAD.to_string()));
    Ok(files)
}

fn filled() -> Files {
    let lessons = format!(
        "{LESSONS_HEAD}{}{}",
        lesson(
            "Quote every expansion",
            "Write \"$var\", never $var.",
            "bash, quoting"
        ),
        lesson(
            "Read lessons first",
            "Open lessons.md before editing.",
            "spec, process"
        )
    );
    vec![
        (
            ".spec/product.md".into(),
            doc(
                "entrypoint",
                "product",
                "# Demo — Product\n\nDemo turns notes into reports.\n",
            ),
        ),
        (
            ".spec/tech.md".into(),
            doc(
                "entrypoint",
                "technical",
                "# Demo — Tech\n\nOne bash script, no services.\n",
            ),
        ),
        (
            ".spec/design.md".into(),
            doc(
                "entrypoint",
                "design",
                "# Demo — Design\n\nPlain text output.\n",
            ),
        ),
        (
            ".spec/plan.md".into(),
            doc(
                "entrypoint",
                "plan",
                "# Demo — Plan\n\n| Feature | Status |\n|---|---|\n| auth | IN PROGRESS |\n",
            ),
        ),
        (".spec/lessons.md".into(), lessons),
    ]
}

const FEATURE_PRODUCT: &str =
    "---\ntype: feature-product\nupdated: 2026-01-15\n---\n\n# auth — Product\n\nUsers sign in.\n";
const FEATURE_PLAN: &str =
    "---\ntype: feature-plan\nupdated: 2026-01-15\n---\n\n# auth — Plan\n\n- M1: NOT STARTED\n";
const FEATURE_TECH: &str = "---\ntype: feature-tech\nupdated: 2026-01-15\n---\n\n# auth — Tech\n\n<!-- merge -->\n## Auth\n\nSessions live in signed cookies.\n<!-- /merge -->\n\nLocal notes.\n\n<!-- merge -->\nTokens expire after 1 hour.\n<!-- /merge -->\n";
const FEATURE_TECH_UNCLOSED: &str = "---\ntype: feature-tech\nupdated: 2026-01-15\n---\n\n# auth — Tech\n\n<!-- merge -->\nNever closed.\n";

fn cases(templates: &Path) -> Result<Vec<CaseDef>, String> {
    let seed = seeded(templates)?;
    let tagged = format!(
        "{LESSONS_HEAD}{}{}{}{}",
        lesson("Quote every expansion", "Write \"$var\".", "bash, quoting"),
        lesson(
            "Guard empty arrays",
            "Expand ${a[@]+\"${a[@]}\"}.",
            "bash, bash32"
        ),
        lesson(
            "Route before reading",
            "Invoke the spec skill first.",
            "spec, skills"
        ),
        lesson("Untagged lesson", "Has tags but none match.", "misc"),
    );
    let quotes = format!(
        "{LESSONS_HEAD}{}",
        lesson(
            "A \"quoted\" title with a back\\slash",
            "Escape \"quotes\" and \\ in JSON.",
            "escape"
        )
    );
    let unplanned = doc(
        "entrypoint",
        "plan",
        "# Demo — Plan\n\nNo features listed.\n",
    );
    let bogus_tech = doc(
        "bogus",
        "technical",
        "# Tech\n\nSee [missing](missing.md).\n",
    );
    let branch_doc = "---\ntype: tech-topic\nparent: tech.md\nscope: api\ncovers: endpoints\nupdated: 2026-01-15\n---\n\n# API — Tech\n\n[tech.md](tech.md)\n";
    let feature = |tech: &str, extra: &[(&str, &str)]| {
        let base = with(
            filled(),
            &[
                (".spec/features/auth/product.md", FEATURE_PRODUCT),
                (".spec/features/auth/tech.md", tech),
                (".spec/features/auth/plan.md", FEATURE_PLAN),
            ],
        );
        with(base, extra)
    };
    Ok(vec![
        CaseDef {
            class: "no-root",
            name: "bare",
            cwd: "",
            runs: &[
                ("lessons-for", &[&["bash"]]),
                ("scan-merges", &[&[], &["--format", "json"]]),
            ],
            files: vec![("README.md".into(), "# Demo\n".into())],
        },
        CaseDef {
            class: "no-root",
            name: "subdir",
            cwd: "src/deep",
            runs: &[("lessons-for", &[&["bash"]])],
            files: vec![("src/deep/notes.txt".into(), "echo hi\n".into())],
        },
        CaseDef {
            class: "seeded",
            name: "fresh",
            cwd: "",
            runs: &[("lessons-for", &[&["bash"]])],
            files: seed.clone(),
        },
        CaseDef {
            class: "seeded",
            name: "from-subdir",
            cwd: "src/app",
            runs: &[("lessons-for", &[&["spec"]])],
            files: with(seed, &[("src/app/notes.txt", "echo hi\n")]),
        },
        CaseDef {
            class: "filled",
            name: "minimal",
            cwd: "",
            runs: &[],
            files: filled(),
        },
        CaseDef {
            class: "filled",
            name: "branch-doc",
            cwd: "",
            runs: &[],
            files: with(
                filled(),
                &[
                    (".spec/tech-api.md", branch_doc),
                    ("README.md", "# Demo\n\nAll 12 passing.\n"),
                ],
            ),
        },
        CaseDef {
            class: "lessons",
            name: "tagged",
            cwd: "",
            runs: &[(
                "lessons-for",
                &[
                    &["bash"],
                    &["bash", "spec"],
                    &["--format", "json", "bash"],
                    &["--format=inject", "spec"],
                    &["nomatch"],
                    &["--bogus"],
                    &[],
                ],
            )],
            files: vec![(".spec/lessons.md".into(), tagged)],
        },
        CaseDef {
            class: "lessons",
            name: "quotes",
            cwd: "",
            runs: &[(
                "lessons-for",
                &[&["escape"], &["--format", "json", "escape"]],
            )],
            files: vec![(".spec/lessons.md".into(), quotes)],
        },
        CaseDef {
            class: "features",
            name: "closed-blocks",
            cwd: "",
            runs: &[
                (
                    "scan-merges",
                    &[
                        &[],
                        &["--format", "json"],
                        &["--format", "plain"],
                        &["auth"],
                        &["nope"],
                    ],
                ),
                (
                    "promote",
                    &[&["auth", "--dry-run"], &["auth"], &["missing"]],
                ),
            ],
            files: feature(FEATURE_TECH, &[]),
        },
        CaseDef {
            class: "features",
            name: "unclosed-block",
            cwd: "",
            runs: &[
                ("scan-merges", &[&[], &["--format", "json"]]),
                ("promote", &[&["auth"]]),
            ],
            files: feature(FEATURE_TECH_UNCLOSED, &[(".spec/plan.md", &unplanned)]),
        },
        CaseDef {
            class: "malformed",
            name: "no-frontmatter",
            cwd: "",
            runs: &[],
            files: with(
                filled(),
                &[(".spec/product.md", "# Product without frontmatter\n")],
            ),
        },
        CaseDef {
            class: "malformed",
            name: "broken-link",
            cwd: "",
            runs: &[],
            files: with(filled(), &[(".spec/tech.md", &bogus_tech)]),
        },
    ])
}

fn case_json(def: &CaseDef) -> Value {
    let mut runs = Map::new();
    for &(command, sets) in def.runs {
        runs.insert(command.to_string(), json!(sets));
    }
    let mut v = json!({ "v": 1, "runs": runs });
    if !def.cwd.is_empty() {
        v["cwd"] = json!(def.cwd);
    }
    v
}

fn write(path: &Path, text: &str) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    fs::write(path, text).map_err(|e| format!("{}: {e}", path.display()))
}

/// Writes the corpus into `out` from the setup templates in `templates`.
/// Refuses to touch an existing `out` that lacks the [`MARKER`] file.
/// Returns the number of cases written.
pub fn generate(templates: &Path, out: &Path) -> Result<usize, String> {
    let defs = cases(templates)?;
    for def in &defs {
        let prefix = format!("{}/", def.cwd);
        if !def.cwd.is_empty() && !def.files.iter().any(|(p, _)| p.starts_with(&prefix)) {
            return Err(format!(
                "{}/{}: cwd {} holds no file, so git cannot keep it",
                def.class, def.name, def.cwd
            ));
        }
    }
    if out.exists() {
        if !out.join(MARKER).is_file() {
            return Err(format!(
                "{}: exists and holds no {MARKER}; refusing to replace it",
                out.display()
            ));
        }
        fs::remove_dir_all(out).map_err(|e| format!("{}: {e}", out.display()))?;
    }
    write(&out.join(MARKER), MARKER_TEXT)?;
    for def in &defs {
        let dir = out.join(def.class).join(def.name);
        let text = serde_json::to_string_pretty(&case_json(def)).map_err(|e| e.to_string())?;
        write(&dir.join("case.json"), &format!("{text}\n"))?;
        for (path, body) in &def.files {
            write(&dir.join("tree").join(path), body)?;
        }
    }
    Ok(defs.len())
}
```

Replace `crates/vibe-parity/src/lib.rs` with:

```rust
//! Differential parity harness (F3) and workspace structure checks (F4).
//!
//! A suite names commands, an oracle argv and a candidate argv for each, and
//! the corpus classes they run over. Every run stages a case into a sandbox,
//! runs the oracle, restages, runs the candidate, and compares exit code,
//! stdout bytes and the bytes of every file left behind.
pub mod corpus_spec;
pub mod harness;
pub mod sandbox;
pub mod suite;
pub mod workspace;
```

Replace `crates/vibe-parity/src/main.rs` with (adds `gen-spec-corpus`):

```rust
//! `vibe-parity`: run a parity suite.
//!
//! Usage:
//!   vibe-parity run <suite.json> --self
//!   vibe-parity run <suite.json> --candidate <binary>
//!   vibe-parity gen-spec-corpus <templates-dir> <out-dir>
//!
//! Exit 0 when every run passes or is pinned, 1 on any failure or suite
//! error, 2 on a usage error.
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use vibe_core::exit;
use vibe_parity::harness::{Mode, Status, run_suite};
use vibe_parity::suite::Suite;

const USAGE: &str = "usage: vibe-parity run <suite.json> (--self | --candidate <binary>)\n       vibe-parity gen-spec-corpus <templates-dir> <out-dir>";

fn usage(msg: &str) -> ExitCode {
    eprintln!("vibe-parity: {msg}\n{USAGE}");
    ExitCode::from(exit::USAGE)
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let rest: Vec<&str> = args.iter().map(String::as_str).collect();
    match rest.as_slice() {
        ["run", suite, "--self"] => run(Path::new(suite), Mode::SelfCheck),
        ["run", suite, "--candidate", bin] => match std::fs::canonicalize(bin) {
            Ok(path) => run(Path::new(suite), Mode::Candidate(path)),
            Err(e) => usage(&format!("candidate {bin}: {e}")),
        },
        ["gen-spec-corpus", templates, out] => {
            match vibe_parity::corpus_spec::generate(Path::new(templates), Path::new(out)) {
                Ok(n) => {
                    println!("gen-spec-corpus: {n} cases written to {out}");
                    ExitCode::from(exit::OK)
                }
                Err(e) => {
                    eprintln!("vibe-parity: {e}");
                    ExitCode::from(exit::FINDINGS)
                }
            }
        }
        _ => usage("bad arguments"),
    }
}

fn run(suite_path: &Path, mode: Mode) -> ExitCode {
    let suite = match Suite::load(suite_path) {
        Ok(suite) => suite,
        Err(e) => return usage(&e),
    };
    let work: PathBuf = std::env::temp_dir().join(format!("vibe-parity-{}", std::process::id()));
    let report = run_suite(&suite, &mode, &work);
    let _ = std::fs::remove_dir_all(&work);
    for r in &report.results {
        let args = r.args.join(" ");
        match &r.status {
            Status::Pass => println!("PASS  {} {} [{args}]", r.command, r.case),
            Status::Pinned(fields) => {
                println!("PIN   {} {} [{args}] {fields:?}", r.command, r.case)
            }
            Status::Fail(msgs) => {
                println!("FAIL  {} {} [{args}]", r.command, r.case);
                for m in msgs {
                    println!("      {m}");
                }
            }
        }
    }
    for e in &report.errors {
        println!("ERROR {e}");
    }
    let label = match mode {
        Mode::SelfCheck => "self-check".to_string(),
        Mode::Candidate(p) => format!("candidate {}", p.display()),
    };
    println!(
        "parity[{}]: {} passed, {} pinned, {} failed, {} error(s), {} runs ({label})",
        suite.name,
        report.passed(),
        report.pinned(),
        report.failed(),
        report.errors.len(),
        report.results.len()
    );
    ExitCode::from(if report.ok() {
        exit::OK
    } else {
        exit::FINDINGS
    })
}
```

- [ ] **Step 4: Write the suite and generate the corpus**

Create `tests/parity/spec.json`:

```json
{
  "v": 1,
  "name": "spec",
  "corpus": "corpus/spec",
  "oracle_root": "../../oracles/bash/spec",
  "commands": [
    { "name": "list-specs", "oracle": ["bash", "{oracle}/scripts/list-specs.sh"], "candidate": ["{candidate}", "list-specs"], "classes": ["no-root", "seeded", "filled", "features", "malformed"] },
    { "name": "validate", "oracle": ["bash", "{oracle}/scripts/validate.sh"], "candidate": ["{candidate}", "validate"], "classes": ["no-root", "seeded", "filled", "features", "malformed"] },
    { "name": "lessons-for", "oracle": ["bash", "{oracle}/scripts/lessons-for.sh"], "candidate": ["{candidate}", "lessons-for"], "classes": ["no-root", "seeded", "lessons"] },
    { "name": "scan-merges", "oracle": ["bash", "{oracle}/scripts/scan-merges.sh"], "candidate": ["{candidate}", "scan-merges"], "classes": ["no-root", "features"] },
    { "name": "check-drift", "oracle": ["bash", "{oracle}/scripts/check-drift.sh"], "candidate": ["{candidate}", "check-drift"], "classes": ["no-root", "seeded", "filled", "features", "malformed"] },
    { "name": "promote", "oracle": ["bash", "{oracle}/scripts/promote.sh"], "candidate": ["{candidate}", "promote"], "classes": ["features"] },
    { "name": "setup", "oracle": ["bash", "{oracle}/scripts/setup.sh"], "candidate": ["{candidate}", "init"], "classes": ["no-root", "seeded"] }
  ],
  "floors": { "no-root": 2, "seeded": 2, "filled": 2, "lessons": 2, "features": 2, "malformed": 2 }
}
```

```bash
cargo run -q -p vibe-parity -- gen-spec-corpus oracles/bash/spec/reference/templates tests/parity/corpus/spec
cargo run -q -p vibe-parity -- run tests/parity/spec.json --self | tail -1
```
Expected: `gen-spec-corpus: 12 cases written to tests/parity/corpus/spec`; `parity[spec]: 61 passed, 0 pinned, 0 failed, 0 error(s), 61 runs (self-check)`.

- [ ] **Step 5: Add the parity CI job**

Append to `.github/workflows/ci.yml`:

```yaml
  parity:
    # F3: the frozen spec oracles over the generated corpus. Until F16 ships a
    # candidate, the oracle runs against itself: this proves the corpus and
    # the oracles are deterministic and every class meets its floor.
    name: parity — spec oracles (self-check until F16)
    runs-on: ubuntu-latest
    steps:
      - name: Checkout
        uses: actions/checkout@v4
        with:
          persist-credentials: false

      - name: Toolchain (pinned by rust-toolchain.toml)
        run: rustup toolchain install

      - name: Oracle self-check over the spec corpus
        run: cargo run --locked -q -p vibe-parity -- run tests/parity/spec.json --self
```

- [ ] **Step 6: Run the checks**

```bash
cargo test --workspace 2>&1 | grep -E 'test result|FAILED'
cargo clippy --workspace --all-targets -- -D warnings 2>&1 | tail -1
cargo fmt --all --check && echo fmt-ok
git add -n tests/parity | wc -l
python3 -c 'import yaml; d=yaml.safe_load(open(".github/workflows/ci.yml")); print(list(d["jobs"]))'
```
Expected: `ok. 4 passed` (spec_corpus) and every other suite ok; clippy no issues; `fmt-ok`; 67 files (`spec.json` + `.generated` + 12 `case.json` + 53 tree files); jobs `['check', 'macos', 'rust-check', 'rust-test', 'parity']`.

- [ ] **Step 7: Commit**

```bash
git add crates/vibe-parity tests/parity .github/workflows/ci.yml
git commit -m "test(parity): add spec corpus and self-check"
```

---

### Task 9: Docs and phase gate

**Files:**
- Modify (inside `strategy.spec`): `.spec/tech-rust.md`, `.spec/tech-contracts.md`, `.spec/tech-spec.md`, `.spec/plan.md`, `.spec/lessons.md`; `AGENTS.md` only inside generated markers (via `regen-active-rules.sh`)

**Interfaces:**
- Consumes: everything above.
- Produces: the Phase 3 baseline.

- [ ] **Step 1: Run every affected check and keep the output**

Run each alone, in this order:

```bash
bash tests/run.sh 2>&1 | tail -12
cargo test --workspace --locked 2>&1 | grep -E 'test result|FAILED'
cargo clippy --workspace --all-targets --locked -- -D warnings 2>&1 | tail -1
cargo fmt --all --check && echo fmt-ok
cargo tree --workspace -d 2>&1 | head -2
cargo run --locked -q -p vibe-parity -- run tests/parity/spec.json --self | tail -1
bash oracles/check.sh
bash contracts/run.sh | tail -1
git ls-files '*.sh' | grep -v '^oracles/' | xargs shellcheck; echo "shellcheck rc=$?"
bash spec/scripts/validate.sh | tail -2; echo "validate rc=$?"
bash spec/scripts/check-drift.sh; echo "drift rc=$?"
node .agents/skills/vibe/engine/cli.mjs render --check; echo "render rc=$?"
```
Expected: `ALL SUITES PASSED` (six suites); every cargo result ok; `nothing to print`; `parity[spec]: 61 passed`; `oracles: 43 files match MANIFEST.sha256`; contracts OK; every `rc=0`.

- [ ] **Step 2: Update the docs**

```bash
bash .agents/skills/vibe/scripts/set-state.sh strategy.brainstorm
bash .agents/skills/vibe/scripts/set-state.sh strategy.spec
```

`.spec/tech-rust.md` — replace the "Shipped today" paragraph with:

```markdown
**Shipped today:** `flow/engine/` (Node/ESM) and the bash scripts still
drive everything. F2–F4 delivered the foundations: frozen oracles under
`oracles/` (43 files, `MANIFEST.sha256`), the Cargo workspace (`vibe-core`
with glob matching and atomic write, a `vibe-spec` stub, `vibe-parity`), and
the parity harness, which runs the spec oracles against themselves over
`tests/parity/corpus/spec/` until F16 supplies the candidate.
```

`.spec/tech-contracts.md` — replace the `contracts/` tree with:

````markdown
```text
contracts/
├── spec-json.schema.json           # v1 (F2)
├── exit-codes.json                 # v1 (F2), with exit-codes.schema.json
├── fixtures/spec-json/<query>/{valid,invalid}/*.json
├── lib/schema.mjs                  # JSON Schema subset validator (Node, zero deps)
├── run.mjs                         # validates schemas, goldens, exit codes, floors
└── run.sh                          # entry point; also runs contracts/tests/
```

`payload`, `marker` and `provider-manifest` schemas arrive with their
tools' rows.
````

and add under `## Spec JSON v1`, after the query table:

```markdown
Fields are defined in `contracts/spec-json.schema.json`. Required sets are
minimal: `root {v, root, source}`, `lessons-for {v, tags, lessons}`,
`plan {v, feature, rows}`, `feature {v, feature, layer, docs}`.
`root.source` names the discovery step that hit: `flag`, `env`, `config`,
`docs-spec`, `dot-spec`. Spec JSON is a new `--json` output; the legacy
`lessons-for.sh --format json` array is a different shape and stays
byte-identical for parity.
```

`.spec/tech-spec.md`, section Migrate — after the sentence ending "…or the migration reverts and aborts rather than leaving a half-migrated tree.", add:

```markdown
`oracles/` and `tests/parity/corpus/` hold `.spec/` literals by design
(frozen oracles and legacy-layout fixtures) and are excluded from the
rewrite and from the population count.
```

`.spec/plan.md` — set the Status cells of rows F2, F3 and F4 to `IN PROGRESS — local gate green; CI pending push`. Add one lesson to `.spec/lessons.md` per failed command, reverted change or late-found defect in this phase (Pattern / Rule / Tags / Date: today). Bump `updated:` on every edited spec doc. Then:

```bash
bash .agents/skills/vibe/scripts/regen-active-rules.sh
bash spec/scripts/validate.sh | tail -2
bash spec/scripts/check-drift.sh
bash .agents/skills/vibe/scripts/set-state.sh idle
```

- [ ] **Step 3: Commit**

```bash
git add .spec AGENTS.md
git commit -m "docs(spec): record phase 2 foundations"
```

- [ ] **Step 4: Owner gate**

Report to the owner: the Step 1 outputs, the planted-mutant evidence (Task 7 Step 6), and the open items below. Ask for push approval. After CI is green on all five jobs, set F2–F4 to `DONE` in a `strategy.spec` commit `docs(spec): close phase 2 rows`.

**Owner items (not in this plan's commits):** `AGENTS.md` "Repo layout" and "Commands" are owner prose (R13) and now miss `oracles/`, `contracts/`, `crates/`, and the `cargo`/`contracts/run.sh` commands.
