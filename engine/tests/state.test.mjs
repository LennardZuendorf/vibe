// engine/tests/state.test.mjs — engine/commands/state.mjs (js-core/3, R2, R6).
//
// The oracle is flow/scripts/set-state.sh. Never hand-copy its expected
// output: every parity assertion here spawns a COPY of the real script
// (byte-identical, via copyFileSync) inside a throwaway sandbox laid out as
// <sandbox>/flow/{scripts/set-state.sh, state.json, state-machine.json} —
// the exact SCRIPT_DIR/SKILL_DIR-relative layout the script expects — so its
// self-relative path resolution runs for real, against sandbox files, and
// never touches this repo's live flow/state.json.

import { mkdtempSync, mkdirSync, writeFileSync, readFileSync, copyFileSync, rmSync, cpSync } from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { test, assert, assertEqual, assertMatch, makeSandbox, runCommand } from './run.mjs';
import { runSet, runGet } from '../commands/state.mjs';
import { readCursor } from '../cursor.mjs';

const REPO_ROOT = path.resolve(path.dirname(new URL(import.meta.url).pathname), '..', '..');
const ORACLE_SRC = path.join(REPO_ROOT, 'flow', 'scripts', 'set-state.sh');

// ---------------------------------------------------------------------------
// Oracle sandbox — a copy of set-state.sh plus its own flow/ cursor+machine,
// never the real ones.
// ---------------------------------------------------------------------------

function makeOracleSandbox({ cursor } = {}) {
  const sandbox = makeSandbox({ cursor });
  const scriptsDir = path.join(sandbox.flowDir, 'scripts');
  mkdirSync(scriptsDir, { recursive: true });
  const scriptPath = path.join(scriptsDir, 'set-state.sh');
  copyFileSync(ORACLE_SRC, scriptPath);
  return { ...sandbox, scriptPath };
}

function runOracle(scriptPath, args) {
  return runCommand('bash', [scriptPath, ...args]);
}

// Cursor bytes with the `updated` timestamp blanked out, for a
// timestamp-agnostic byte comparison. The format itself is asserted
// separately with a regex (below).
function normalizeCursorBytes(raw) {
  return raw.replace(/"updated": "[^"]*"/, '"updated": "<TS>"');
}

const TIMESTAMP_RE = /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}Z$/;

// ---------------------------------------------------------------------------
// Parity matrix — set, with and without an existing cursor.
// ---------------------------------------------------------------------------

const FIXTURES = [
  { name: 'from idle, set with feature', start: null, target: 'feature.impl', feature: 'demo' },
  { name: 'from idle, set without feature (feature.* warns but still writes)', start: null, target: 'feature.design', feature: undefined },
  {
    name: 'existing feature carried forward across phases',
    start: { flow: 'feature', phase: 'design', feature: 'carried-along', updated: '2020-01-01T00:00:00Z' },
    target: 'feature.plan',
    feature: undefined,
  },
  {
    name: 'idle clears an existing feature',
    start: { flow: 'feature', phase: 'impl', feature: 'to-be-cleared', updated: '2020-01-01T00:00:00Z' },
    target: 'idle',
    feature: undefined,
  },
  {
    name: 'new feature argument overrides an existing one',
    start: { flow: 'feature', phase: 'impl', feature: 'old', updated: '2020-01-01T00:00:00Z' },
    target: 'feature.verify',
    feature: 'new',
  },
  { name: 'single-token target (idle) from a fresh cursor', start: null, target: 'idle', feature: undefined },
  { name: 'quick flow, no feature ever set', start: null, target: 'quick.triage', feature: undefined },
];

for (const fx of FIXTURES) {
  test(`parity: cursor bytes match oracle — ${fx.name}`, () => {
    const oracle = makeOracleSandbox({ cursor: fx.start ?? undefined });
    const engine = makeSandbox({ cursor: fx.start ?? undefined });
    try {
      const args = fx.feature ? [fx.target, fx.feature] : [fx.target];

      const oracleResult = runOracle(oracle.scriptPath, args);
      assertEqual(oracleResult.code, 0, `oracle failed: ${oracleResult.stderr}`);

      const engineResult = runSet(engine.flowDir, args);
      assertEqual(engineResult.code, 0, `engine failed: ${engineResult.stderr}`);

      const oracleBytes = readFileSync(oracle.cursorPath, 'utf8');
      const engineBytes = readFileSync(engine.cursorPath, 'utf8');

      assertEqual(
        normalizeCursorBytes(engineBytes),
        normalizeCursorBytes(oracleBytes),
        `cursor bytes diverge for ${fx.name}\noracle:\n${oracleBytes}\nengine:\n${engineBytes}`,
      );

      const engineTs = JSON.parse(engineBytes).updated;
      assertMatch(engineTs, TIMESTAMP_RE, `engine 'updated' must match bash date -u format, got ${engineTs}`);
      const oracleTs = JSON.parse(oracleBytes).updated;
      assertMatch(oracleTs, TIMESTAMP_RE, `oracle 'updated' must match bash date -u format, got ${oracleTs}`);
    } finally {
      oracle.cleanup();
      engine.cleanup();
    }
  });
}

test('parity: stdout arrow + next line match the oracle', () => {
  const oracle = makeOracleSandbox();
  const engine = makeSandbox();
  try {
    const oracleResult = runOracle(oracle.scriptPath, ['feature.plan']);
    const engineResult = runSet(engine.flowDir, ['feature.plan']);

    assertEqual(engineResult.stdout, oracleResult.stdout);
  } finally {
    oracle.cleanup();
    engine.cleanup();
  }
});

test('parity: WARN line for feature.* with no feature matches oracle stderr shape (non-blocking, exit 0)', () => {
  const oracle = makeOracleSandbox();
  const engine = makeSandbox();
  try {
    const oracleResult = runOracle(oracle.scriptPath, ['feature.design']);
    const engineResult = runSet(engine.flowDir, ['feature.design']);

    assertEqual(oracleResult.code, 0);
    assertEqual(engineResult.code, 0);
    assertMatch(oracleResult.stderr, /WARN: entering 'feature\.design' with no feature set/);
    assertMatch(engineResult.stderr, /WARN: entering 'feature\.design' with no feature set/);
  } finally {
    oracle.cleanup();
    engine.cleanup();
  }
});

test('parity: unknown state is rejected by both, cursor left untouched', () => {
  const oracle = makeOracleSandbox();
  const engine = makeSandbox();
  try {
    const before = readFileSync(engine.cursorPath, 'utf8');

    const oracleResult = runOracle(oracle.scriptPath, ['not.a.real.state']);
    const engineResult = runSet(engine.flowDir, ['not.a.real.state']);

    assert(oracleResult.code !== 0, 'oracle should reject an unknown state');
    assertEqual(engineResult.code, 1, 'engine should reject an unknown state');
    assertMatch(engineResult.stderr, /is not a known state/);

    const after = readFileSync(engine.cursorPath, 'utf8');
    assertEqual(after, before, 'a rejected target must not touch the cursor');
  } finally {
    oracle.cleanup();
    engine.cleanup();
  }
});

// ---------------------------------------------------------------------------
// Direct unit coverage (discriminating: each fails if its behaviour reverts)
// ---------------------------------------------------------------------------

test('runSet: no target given is a named, non-zero error; cursor untouched', () => {
  const sandbox = makeSandbox();
  try {
    const before = readFileSync(sandbox.cursorPath, 'utf8');
    const result = runSet(sandbox.flowDir, []);
    assertEqual(result.code, 1);
    assertMatch(result.stderr, /no target state given/);
    assertEqual(readFileSync(sandbox.cursorPath, 'utf8'), before);
  } finally {
    sandbox.cleanup();
  }
});

test('runSet: key order is flow, phase, feature, updated (JSON.stringify defaults would not guarantee this)', () => {
  const sandbox = makeSandbox();
  try {
    runSet(sandbox.flowDir, ['feature.impl', 'demo']);
    const parsed = JSON.parse(readFileSync(sandbox.cursorPath, 'utf8'));
    assertEqual(Object.keys(parsed), ['flow', 'phase', 'feature', 'updated']);
  } finally {
    sandbox.cleanup();
  }
});

test('runSet: two-space indent and trailing newline, byte-checked', () => {
  const sandbox = makeSandbox();
  try {
    runSet(sandbox.flowDir, ['idle']);
    const raw = readFileSync(sandbox.cursorPath, 'utf8');
    assert(raw.startsWith('{\n  "flow": '), `expected 2-space indent, got: ${raw.slice(0, 30)}`);
    assert(raw.endsWith('}\n'), 'expected a single trailing newline');
  } finally {
    sandbox.cleanup();
  }
});

test('runSet: feature carry-forward preserved when no new feature is given', () => {
  const sandbox = makeSandbox({
    cursor: { flow: 'feature', phase: 'design', feature: 'js-core', updated: '2020-01-01T00:00:00Z' },
  });
  try {
    runSet(sandbox.flowDir, ['feature.plan']);
    const cursor = readCursor(sandbox.flowDir);
    assertEqual(cursor.feature, 'js-core');
    assertEqual(cursor.state, 'feature.plan');
  } finally {
    sandbox.cleanup();
  }
});

test('runSet: new feature argument wins over a carried-forward one', () => {
  const sandbox = makeSandbox({
    cursor: { flow: 'feature', phase: 'design', feature: 'old-feature', updated: '2020-01-01T00:00:00Z' },
  });
  try {
    runSet(sandbox.flowDir, ['feature.plan', 'new-feature']);
    assertEqual(readCursor(sandbox.flowDir).feature, 'new-feature');
  } finally {
    sandbox.cleanup();
  }
});

test('runSet: idle clears an existing feature even if none is passed', () => {
  const sandbox = makeSandbox({
    cursor: { flow: 'feature', phase: 'impl', feature: 'js-core', updated: '2020-01-01T00:00:00Z' },
  });
  try {
    runSet(sandbox.flowDir, ['idle']);
    const cursor = readCursor(sandbox.flowDir);
    assertEqual(cursor.feature, null);
    assertEqual(cursor.state, 'idle');
  } finally {
    sandbox.cleanup();
  }
});

test('runSet: unknown state is rejected and lists known states', () => {
  const sandbox = makeSandbox();
  try {
    const result = runSet(sandbox.flowDir, ['bogus.state']);
    assertEqual(result.code, 1);
    assertMatch(result.stderr, /'bogus\.state' is not a known state/);
    assertMatch(result.stderr, /idle/); // known-states list should include idle
  } finally {
    sandbox.cleanup();
  }
});

test('runSet: single-token target (idle) writes flow===phase, not idle.idle', () => {
  const sandbox = makeSandbox({
    cursor: { flow: 'quick', phase: 'fix', feature: null, updated: '2020-01-01T00:00:00Z' },
  });
  try {
    runSet(sandbox.flowDir, ['idle']);
    const parsed = JSON.parse(readFileSync(sandbox.cursorPath, 'utf8'));
    assertEqual(parsed.flow, 'idle');
    assertEqual(parsed.phase, 'idle');
  } finally {
    sandbox.cleanup();
  }
});

test('runSet: stdout prints the arrow line and next: line for a state with next states', () => {
  const sandbox = makeSandbox();
  try {
    const result = runSet(sandbox.flowDir, ['feature.impl']);
    assertEqual(result.stdout, '-> feature.impl\n   next: feature.verify, idle\n');
  } finally {
    sandbox.cleanup();
  }
});

test('runSet: gate enforcement is explicitly out of scope — a gated edge writes without a confirm token', () => {
  // feature.plan -> feature.impl is a human gate in the machine's `gates` map,
  // but set-state.sh (and therefore this port) never checks it — that is
  // machine-teeth's job. Writing straight through must succeed here.
  const sandbox = makeSandbox({
    cursor: { flow: 'feature', phase: 'plan', feature: 'js-core', updated: '2020-01-01T00:00:00Z' },
  });
  try {
    const result = runSet(sandbox.flowDir, ['feature.impl']);
    assertEqual(result.code, 0, 'gate must not block the writer-only port');
    assertEqual(readCursor(sandbox.flowDir).state, 'feature.impl');
  } finally {
    sandbox.cleanup();
  }
});

// ---------------------------------------------------------------------------
// get — no bash oracle (set-state.sh is writer-only); prints readCursor()'s
// own shape, so this is tested against that primitive directly.
// ---------------------------------------------------------------------------

test('runGet: reflects the current cursor as JSON, absent cursor reads idle', () => {
  const sandbox = makeSandbox();
  sandbox.cleanup(); // now the dir doesn't exist at all -> readCursor's absent path
  const bareDir = mkdtempSync(path.join(tmpdir(), 'vibe-state-get-'));
  try {
    const result = runGet(bareDir);
    assertEqual(result.code, 0);
    const parsed = JSON.parse(result.stdout);
    assertEqual(parsed.state, 'idle');
    assertEqual(parsed.feature, null);
  } finally {
    rmSync(bareDir, { recursive: true, force: true });
  }
});

test('runGet: reflects a set cursor after runSet wrote it', () => {
  const sandbox = makeSandbox();
  try {
    runSet(sandbox.flowDir, ['feature.impl', 'demo']);
    const result = runGet(sandbox.flowDir);
    const parsed = JSON.parse(result.stdout);
    assertEqual(parsed.state, 'feature.impl');
    assertEqual(parsed.feature, 'demo');
  } finally {
    sandbox.cleanup();
  }
});

// ---------------------------------------------------------------------------
// CLI end-to-end wiring — a full install-layout fixture (engine/ copied in,
// no flow/ dir at all), spawning the real `vibe state set` entry point, the
// same way js-core/2's "install target" tests exercise self-relative
// resolution for real rather than mocking it.
// ---------------------------------------------------------------------------

test('CLI: `vibe state set` end-to-end on an install-layout fixture', () => {
  const installRoot = mkdtempSync(path.join(tmpdir(), 'vibe-state-cli-install-'));
  const vibeDir = path.join(installRoot, '.agents', 'skills', 'vibe');
  const engineDir = path.join(vibeDir, 'engine');
  mkdirSync(vibeDir, { recursive: true });
  cpSync(path.join(REPO_ROOT, 'engine'), engineDir, {
    recursive: true,
    filter: (src) => !src.includes(`${path.sep}tests${path.sep}`) && !src.endsWith(`${path.sep}tests`),
  });
  copyFileSync(
    path.join(REPO_ROOT, 'flow', 'state-machine.json'),
    path.join(vibeDir, 'state-machine.json'),
  );

  const unrelatedCwd = mkdtempSync(path.join(tmpdir(), 'vibe-state-cli-cwd-'));
  const prevEnv = process.env.CLAUDE_PROJECT_DIR;
  delete process.env.CLAUDE_PROJECT_DIR;

  try {
    const result = runCommand(
      process.execPath,
      [path.join(engineDir, 'cli.mjs'), 'state', 'set', 'quick.triage'],
      { cwd: unrelatedCwd },
    );
    assertEqual(result.code, 0, `stderr: ${result.stderr}`);
    assertEqual(result.stdout, '-> quick.triage\n   next: quick.fix, feature.design, idle\n');

    const cursor = JSON.parse(readFileSync(path.join(vibeDir, 'state.json'), 'utf8'));
    assertEqual(cursor.flow, 'quick');
    assertEqual(cursor.phase, 'triage');
  } finally {
    rmSync(installRoot, { recursive: true, force: true });
    rmSync(unrelatedCwd, { recursive: true, force: true });
    if (prevEnv !== undefined) process.env.CLAUDE_PROJECT_DIR = prevEnv;
  }
});

test('CLI: `vibe state` with no subcommand is a named error, not a crash', () => {
  // No sandbox/install-layout wiring needed: an empty sub never reaches
  // readCursor/loadMachine (see the branch order in state.mjs's run()), so
  // this is safe to run with vibeDir resolving via the normal default path —
  // it fails before ever touching a cursor.
  const result = runCommand(process.execPath, [path.join(REPO_ROOT, 'engine', 'cli.mjs'), 'state']);
  assertEqual(result.code, 1);
  assertMatch(result.stderr, /unknown subcommand/);
});
