#!/usr/bin/env node
// engine/tests/run.mjs — the JS-suite harness: assert helpers, sandbox fixture
// builder, process-spawn helpers, and the test runner/discoverer.
//
// This is the foundation every later js-core unit's *.test.mjs runs under.
// Test files live alongside this one as engine/tests/*.test.mjs and register
// their cases by importing `test` from this module — the runner discovers
// and imports them, then executes the shared registry.
//
// Usage:
//   node engine/tests/run.mjs              # run every discovered test
//   node engine/tests/run.mjs <substring>   # run only tests whose file or
//                                            # name includes <substring>
//                                            # (repeatable: any match runs)
//
// Exit codes: 0 all selected tests passed; 1 at least one failed or the
// runner itself crashed while discovering/loading test files.

import { strict as nodeAssert } from 'node:assert';
import {
  readdirSync,
  readFileSync,
  mkdtempSync,
  mkdirSync,
  writeFileSync,
  copyFileSync,
  realpathSync,
  rmSync,
} from 'node:fs';
import { spawnSync } from 'node:child_process';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const TESTS_DIR = __dirname;
const REPO_ROOT = path.resolve(__dirname, '..', '..', '..');
const CLI_PATH = path.join(REPO_ROOT, 'flow', 'engine', 'cli.mjs');

// ---------------------------------------------------------------------------
// Assert helpers
// ---------------------------------------------------------------------------

export function assert(cond, msg = 'assertion failed') {
  if (!cond) throw new Error(msg);
}

export function assertEqual(actual, expected, msg) {
  nodeAssert.deepStrictEqual(
    actual,
    expected,
    msg ?? `expected ${JSON.stringify(expected)}, got ${JSON.stringify(actual)}`,
  );
}

export function assertMatch(str, re, msg) {
  assert(
    typeof str === 'string' && re.test(str),
    msg ?? `expected ${JSON.stringify(str)} to match ${re}`,
  );
}

export function assertIncludes(haystack, needle, msg) {
  assert(
    typeof haystack === 'string' && haystack.includes(needle),
    msg ?? `expected ${JSON.stringify(haystack)} to include ${JSON.stringify(needle)}`,
  );
}

// A test calls `skip(reason)` to opt out of an unmet precondition (e.g. no
// jq on PATH) — it must show up in the summary as a SKIP, never silently
// count as a pass. A plain early `return` inside a passing test body is
// indistinguishable from "ran and asserted nothing was wrong"; the CI leg
// that strips jq from PATH needs to see how many assertions actually ran.
export class Skipped extends Error {
  constructor(reason = 'skipped') {
    super(reason);
    this.name = 'Skipped';
  }
}

export function skip(reason) {
  throw new Skipped(reason);
}

// ---------------------------------------------------------------------------
// The jq-half gate (js-core/8 fix round 2, re-review Finding 5)
// ---------------------------------------------------------------------------
//
// `skip()` above made the jq half of the parity matrix report honestly. It did
// not make anything REQUIRE that half to run. With jq absent the suite prints
// `289 passed, 0 failed, 44 skipped` and exits 0 — so a runner image that
// dropped preinstalled jq would silently stop verifying R2 ("ported command
// output matches its bash original") on every leg, with both CI legs green and
// a skip count nobody asserts on. That is the same vacuous-green shape the
// zero-discovered-files guard in main() exists to prevent, one level up.
//
// The matrix generates its cases as `<case> x jq` / `<case> x no-jq`, so the
// population is identified by name — a convention, which is why the gate fails
// LOUDLY (not silently) when nothing matches it any more.
export const JQ_LEG_NAME_RE = / x jq$/;

// The OTHER half of the same matrix. It is what makes the gate's applicability
// self-anchoring: a run that carries `x no-jq` legs but no `x jq` legs is not a
// run without a matrix, it is a run whose jq half has been renamed away — the
// exact disarm that used to happen silently. Exported so runner.test.mjs and
// parity.test.mjs share one spelling of the convention instead of three.
export const NO_JQ_LEG_NAME_RE = / x no-jq$/;

// CI's own jq-stripped leg is SUPPOSED to skip the entire jq half; a runner
// that has quietly lost jq looks identical from inside the process. Only an
// explicit signal can separate them, so the stripped leg declares itself with
// VIBE_NO_JQ=1 — and the gate then holds it to that claim in both directions.
export function jqHalfVerdict({ total, executed, noJqOptIn }) {
  if (total === 0) {
    return {
      ok: false,
      reason:
        `no test name matched ${JQ_LEG_NAME_RE} — the parity matrix's jq legs have been renamed or removed. ` +
        'The jq-half gate is counting an empty population and can no longer fail; ' +
        'update JQ_LEG_NAME_RE in tests/run.mjs to the new naming convention.',
    };
  }
  if (noJqOptIn) {
    if (executed > 0) {
      return {
        ok: false,
        reason:
          `VIBE_NO_JQ=1 was set but ${executed} of ${total} jq-leg tests executed — jq is still reachable. ` +
          'This leg exists to exercise the no-jq degrade paths; it is not stripping jq, so that half is the one not running.',
      };
    }
    return { ok: true, reason: `jq half deliberately skipped (VIBE_NO_JQ=1): 0 of ${total} jq-leg tests ran` };
  }
  if (executed === 0) {
    return {
      ok: false,
      reason:
        `the jq half of the parity matrix did not run: 0 of ${total} jq-leg tests executed (all skipped). ` +
        'jq is missing from PATH, so every `x jq` case degenerates into a duplicate of its no-jq twin. ' +
        'Install jq — or, if this is the deliberate jq-stripped leg, set VIBE_NO_JQ=1 so the skip is declared.',
    };
  }
  return { ok: true, reason: `jq half ran: ${executed} of ${total} jq-leg tests executed` };
}

export async function assertThrows(fn, msg) {
  let threw = false;
  try {
    await fn();
  } catch {
    threw = true;
  }
  assert(threw, msg ?? 'expected function to throw');
}

// ---------------------------------------------------------------------------
// Process-spawn helpers (used by parity tests against the bash originals too)
// ---------------------------------------------------------------------------

// Spawns any command synchronously and normalizes the result shape. Never
// throws on a non-zero exit — callers assert on `.code`.
//
// `opts.unsetEnv` is a list of variable names DELETED from the child's
// environment. Not naming a variable in `opts.env` is not the same as the child
// not having it: the base is `process.env`, so whatever the runner inherited is
// inherited again. A control case that means "this variable is absent" must say
// so, or it silently becomes "absent unless someone upstream set it" — a test
// that passes only because the ambient environment happened to be clean. This
// suite has been bitten by that shape twice now (CLAUDE_PROJECT_DIR in
// flow/tests/run.sh, VIBE_NO_JQ in runner.test.mjs's own jq-gate case, which
// failed on CI's jq-stripped leg and nowhere else), so the escape lives on the
// shared helper rather than being hand-rolled per site.
// Order matters and used to be backwards (js-core/8 fix round 2 re-review,
// Minor 1): deleting AFTER the merge made `unsetEnv` win over a value the
// caller passed explicitly in `opts.env`, while the paragraph above said the
// opposite. `unsetEnv` is a list of names not to INHERIT, so it applies to the
// inherited base and an explicit `opts.env` value still wins — which is the
// only reading under which passing both is not simply a contradiction. Pinned
// in both directions in runner.test.mjs.
export function runCommand(cmd, args = [], opts = {}) {
  const env = { ...process.env };
  for (const key of opts.unsetEnv ?? []) delete env[key];
  Object.assign(env, opts.env ?? {});
  const result = spawnSync(cmd, args, {
    cwd: opts.cwd ?? REPO_ROOT,
    env,
    input: opts.input,
    encoding: 'utf8',
  });
  return {
    code: result.status,
    stdout: result.stdout ?? '',
    stderr: result.stderr ?? '',
    error: result.error ?? null,
  };
}

// Convenience wrapper for invoking the vibe CLI under test.
export function runCli(args = [], opts = {}) {
  return runCommand(process.execPath, [CLI_PATH, ...args], opts);
}

// Builds a throwaway copy of cli.mjs whose COMMANDS array carries one extra
// placeholder name, alongside an empty commands/ dir for it to resolve
// against. All four real commands (state/orders/doctrine/doctor) are
// implemented as of js-core/6, so cli.mjs's dispatch-error paths ("module
// genuinely missing" vs "module exists but its own import is broken") have
// no real unimplemented command left to exercise them against — this gives
// dispatch-error.test.mjs / cli.test.mjs a synthetic one without touching
// the real, shipped COMMANDS array. Returns {cliPath, commandsDir,
// placeholder, cleanup()}.
export function makeCliWithPlaceholderCommand(placeholder = 'zzz-test-placeholder') {
  const dir = mkTempRoot('vibe-cli-placeholder-');
  const src = readFileSync(CLI_PATH, 'utf8');
  const marker = "const COMMANDS = ['state', 'orders', 'doctrine', 'doctor', 'hook'];";
  assert(src.includes(marker), 'cli.mjs COMMANDS array literal has changed shape — update this test helper');
  const patched = src.replace(marker, `const COMMANDS = ['state', 'orders', 'doctrine', 'doctor', 'hook', '${placeholder}'];`);
  const cliPath = path.join(dir, 'cli.mjs');
  writeFileSync(cliPath, patched);
  const commandsDir = path.join(dir, 'commands');
  mkdirSync(commandsDir, { recursive: true });

  let cleaned = false;
  function cleanup() {
    if (cleaned) return;
    cleaned = true;
    rmSync(dir, { recursive: true, force: true });
  }

  return { cliPath, commandsDir, placeholder, cleanup };
}

// ---------------------------------------------------------------------------
// Sandbox fixture builder
// ---------------------------------------------------------------------------

// mkTempRoot — mkdtempSync, then realpath. The ONLY correct way to build a temp
// root a fixture will later compare a resolved path against.
//
// os.tmpdir() hands back the UNRESOLVED $TMPDIR (on macOS `/var/folders/…`,
// where /var is a symlink to private/var), but Node's ESM loader realpaths
// `import.meta.url` — so any module imported out of that fixture sees
// `/private/var/folders/…` and every self-relative path it derives is in the
// canonical form. Comparing that against the raw mkdtempSync string fails on
// macOS while passing on Linux, where /tmp is a real directory (js-core/8 fix
// round 1: 9 of the 10 macOS engine failures were exactly this). Normalizing
// here — not in root.mjs — keeps the production resolver free of a realpathSync
// on its hot path; the canonical path is the right thing for it to return.
export function mkTempRoot(prefix) {
  return realpathSync(mkdtempSync(path.join(tmpdir(), prefix)));
}

// Builds a throwaway temp repo containing a flow/state.json cursor and a
// flow/state-machine.json (copied byte-for-byte from the real repo so
// fixtures never drift from the actual machine definition). Returns paths
// and a cleanup() that removes the whole sandbox.
export function makeSandbox({ cursor } = {}) {
  const dir = mkTempRoot('vibe-engine-test-');
  const flowDir = path.join(dir, 'flow');
  mkdirSync(flowDir, { recursive: true });

  const machineSrc = path.join(REPO_ROOT, 'flow', 'state-machine.json');
  const machinePath = path.join(flowDir, 'state-machine.json');
  copyFileSync(machineSrc, machinePath);

  const cursorPath = path.join(flowDir, 'state.json');
  const cursorBody = cursor ?? {
    flow: 'idle',
    phase: 'idle',
    feature: null,
    updated: '2026-01-01T00:00:00Z',
  };
  writeFileSync(cursorPath, `${JSON.stringify(cursorBody, null, 2)}\n`);

  // Marker so self-relative/marker root resolution has something to find on
  // a bare non-git target, mirroring the "stranger eval" fixture shape.
  mkdirSync(path.join(dir, '.spec'), { recursive: true });

  let cleaned = false;
  function cleanup() {
    if (cleaned) return;
    cleaned = true;
    rmSync(dir, { recursive: true, force: true });
  }

  return { dir, flowDir, cursorPath, machinePath, cleanup };
}

// makeHookSandbox — a real INSTALL-shaped layout: <root>/.agents/skills/vibe/
// {state.json, state-machine.json, SKILL.md, scripts/detect-context.sh,
// warnings.log?, evidence/}. Mirrors what `install.sh` lays down, handcrafted
// (not run through bash install.sh) to keep the suite fast and hermetic.
//
// Shared here rather than owned by hook.test.mjs (js-core/8 final review, I3):
// parity.test.mjs's guard/gate oracle differential spawns the FROZEN bash hooks
// in tests/oracles/, which self-locate through exactly this layout
// ($ROOT/.agents/skills/vibe/scripts/detect-context.sh, and detect-context.sh's
// own SKILL_DIR=scripts/..). A second, hand-copied fixture in the parity file
// could drift from the one hook.test.mjs asserts against, and then the two
// suites would be honest about DIFFERENT layouts.
export function makeHookSandbox({ cursor, includeDetect = true, gitInit = false } = {}) {
  const dir = mkTempRoot('vibe-hook-test-');
  const vibeDir = path.join(dir, '.agents', 'skills', 'vibe');
  const scriptsDir = path.join(vibeDir, 'scripts');
  mkdirSync(scriptsDir, { recursive: true });
  copyFileSync(path.join(REPO_ROOT, 'flow', 'state-machine.json'), path.join(vibeDir, 'state-machine.json'));
  writeFileSync(
    path.join(vibeDir, 'SKILL.md'),
    readFileSync(path.join(REPO_ROOT, 'flow', 'SKILL.md'), 'utf8'),
  );
  if (includeDetect) {
    copyFileSync(
      path.join(REPO_ROOT, 'flow', 'scripts', 'detect-context.sh'),
      path.join(scriptsDir, 'detect-context.sh'),
    );
  }
  const cursorPath = path.join(vibeDir, 'state.json');
  const cursorBody = cursor ?? { flow: 'idle', phase: 'idle', feature: null, updated: '2026-01-01T00:00:00Z' };
  writeFileSync(cursorPath, `${JSON.stringify(cursorBody, null, 2)}\n`);
  mkdirSync(path.join(dir, '.spec'), { recursive: true });

  if (gitInit) {
    runCommand('git', ['init', '-q'], { cwd: dir });
    runCommand('git', ['-C', dir, 'config', 'user.email', 't@t'], { cwd: dir });
    runCommand('git', ['-C', dir, 'config', 'user.name', 't'], { cwd: dir });
  }

  const skillsDir = path.join(dir, '.agents', 'skills');
  const warnLogPath = path.join(vibeDir, 'warnings.log');

  let cleaned = false;
  function cleanup() {
    if (cleaned) return;
    cleaned = true;
    rmSync(dir, { recursive: true, force: true });
  }

  return { dir, root: dir, vibeDir, skillsDir, scriptsDir, cursorPath, warnLogPath, cleanup };
}

// ---------------------------------------------------------------------------
// Test registry + runner
// ---------------------------------------------------------------------------

const registry = []; // { file, name, fn }
let currentFile = '(unknown)';

function setCurrentFile(file) {
  currentFile = file;
}

// Test files call: import { test } from './run.mjs'; test('name', async () => {...})
export function test(name, fn) {
  registry.push({ file: currentFile, name, fn });
}

// Read-only view of the registry, so a test can assert on the SHAPE of the
// suite itself (see runner.test.mjs's jq-leg population floor) without being
// able to mutate what will run.
export function registeredTests() {
  return registry.map(({ file, name }) => ({ file, name }));
}

function discoverTestFiles() {
  return readdirSync(TESTS_DIR)
    .filter((f) => f.endsWith('.test.mjs'))
    .sort()
    .map((f) => path.join(TESTS_DIR, f));
}

async function loadTestFiles(files) {
  for (const file of files) {
    setCurrentFile(path.basename(file));
    await import(pathToFileURL(file).href);
  }
}

function selectTests(filters) {
  if (filters.length === 0) return registry;
  return registry.filter(
    ({ file, name }) => filters.some((f) => file.includes(f) || name.includes(f)),
  );
}

async function main() {
  const filters = process.argv.slice(2);
  const files = discoverTestFiles();

  if (files.length === 0) {
    // Fail loud: a runner that reports "0 total, exit 0" on broken
    // discovery is indistinguishable from a runner that ran everything and
    // found it clean. Units 2-8 drive this via filtered runs — a typo or a
    // discovery bug must not read as green.
    console.error('no test files found (engine/tests/*.test.mjs) — treating as a failure');
    process.exitCode = 1;
    return;
  }

  await loadTestFiles(files);

  const selected = selectTests(filters);
  if (filters.length > 0 && selected.length === 0) {
    console.error(`no tests matched filter(s): ${filters.join(', ')} — treating as a failure`);
    process.exitCode = 1;
    return;
  }

  let pass = 0;
  let fail = 0;
  let skipped = 0;
  let jqLegTotal = 0;
  let jqLegExecuted = 0;
  let noJqLegTotal = 0;

  for (const { file, name, fn } of selected) {
    const isJqLeg = JQ_LEG_NAME_RE.test(name);
    if (isJqLeg) jqLegTotal += 1;
    if (NO_JQ_LEG_NAME_RE.test(name)) noJqLegTotal += 1;
    try {
      await fn();
      pass += 1;
      if (isJqLeg) jqLegExecuted += 1;
      console.log(`  ok    ${file} :: ${name}`);
    } catch (err) {
      if (err instanceof Skipped) {
        skipped += 1;
        console.log(`  skip  ${file} :: ${name} (${err.message})`);
        continue;
      }
      fail += 1;
      // A jq leg that RAN and failed still ran — the gate below asks whether the
      // half executed, not whether it passed; `fail` already carries that.
      if (isJqLeg) jqLegExecuted += 1;
      console.log(`  FAIL  ${file} :: ${name}`);
      console.log(`        ${err && err.stack ? err.stack : err}`);
    }
  }

  console.log('');
  console.log(`${pass} passed, ${fail} failed, ${skipped} skipped, ${selected.length} total`);

  // Only a full, unfiltered run of the real suite can speak for the matrix.
  // A filtered run deliberately selects a subset, and runner.test.mjs spawns
  // this file against synthetic single-test directories — neither carries the
  // parity matrix, so neither is evidence about it either way.
  //
  // Applicability is DERIVED from what actually registered, never from a
  // filename. It used to be `path.basename(f) === 'parity.test.mjs'`: a
  // hand-written name pinned by nothing, so `git mv parity.test.mjs
  // parity-matrix.test.mjs` — an ordinary refactor with no reason to touch this
  // file — switched the whole gate off and both CI legs stayed green
  // (js-core/8 fix round 2 re-review, Important 3; .spec/lessons.md: hand-written
  // values rot silently).
  //
  // Either half of the matrix makes the gate applicable, which is what stops the
  // derivation from being self-disarming: renaming only the `x jq` legs leaves
  // `x no-jq` legs registered, so the gate still runs and reports a jq-leg
  // population of ZERO — loudly — instead of quietly concluding there is no
  // matrix here. Renaming BOTH halves is caught by parity.test.mjs's structural
  // matrix floor and runner.test.mjs's registry floor.
  let jqGateFailed = false;
  const carriesMatrix = jqLegTotal > 0 || noJqLegTotal > 0;
  if (filters.length === 0 && carriesMatrix) {
    const verdict = jqHalfVerdict({
      total: jqLegTotal,
      executed: jqLegExecuted,
      noJqOptIn: process.env.VIBE_NO_JQ === '1',
    });
    if (verdict.ok) {
      console.log(`jq half: ${verdict.reason}`);
    } else {
      jqGateFailed = true;
      console.error(`jq-half gate: ${verdict.reason}`);
    }
  }

  process.exitCode = fail > 0 || jqGateFailed ? 1 : 0;
}

// `__filename` comes from `import.meta.url`, which Node's ESM loader has already
// realpath'd — so argv[1] must be realpath'd too, not merely path.resolve'd
// (which normalizes `.`/`..` but never follows symlinks). Without this the
// runner silently no-ops to exit 0 whenever any component of its invocation path
// is a symlink: `.agents/skills/vibe` -> `flow/`, a symlinked checkout, or every
// macOS temp dir (/var -> private/var). realpathSync throws on a path that does
// not exist, so fall back to the plain resolution.
const argvPath = (() => {
  const raw = process.argv[1];
  if (!raw) return null;
  try {
    return realpathSync(raw);
  } catch {
    return path.resolve(raw);
  }
})();
const isMain = argvPath !== null && argvPath === __filename;
if (isMain) {
  main().catch((err) => {
    console.error('test runner crashed:', err && err.stack ? err.stack : err);
    process.exitCode = 1;
  });
}
