// engine/tests/state.test.mjs — engine/commands/state.mjs (js-core/3, R2, R6).
//
// The oracle is flow/scripts/set-state.sh. Never hand-copy its expected
// output: every parity assertion here spawns a COPY of the real script
// (byte-identical, via copyFileSync) inside a throwaway sandbox laid out as
// <sandbox>/flow/{scripts/set-state.sh, state.json, state-machine.json} —
// the exact SCRIPT_DIR/SKILL_DIR-relative layout the script expects — so its
// self-relative path resolution runs for real, against sandbox files, and
// never touches this repo's live flow/state.json.
//
// Review round 1 additions (Finding 3): the matrix below is now adversarial,
// not just happy paths — prototype-polluting keys, a corrupt cursor, a
// missing machine file, an empty-string feature, and an escaping-sensitive
// feature name. Every failure-path case asserts BOTH the exit code AND that
// the cursor bytes are byte-identical before/after, on both the oracle and
// the engine, so a regression that silently succeeds-but-corrupts cannot
// slip through as "well, it exited non-zero somewhere".

import { mkdtempSync, mkdirSync, writeFileSync, readFileSync, copyFileSync, rmSync, cpSync } from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { test, assert, assertEqual, assertMatch, makeSandbox, mkTempRoot, runCommand } from './run.mjs';
import runState, { runSet, runGet } from '../commands/state.mjs';
import { readCursor } from '../cursor.mjs';

const REPO_ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..', '..', '..');
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

// Whether this runner has jq. The oracle's `next:` stdout line is jq-only
// (see set-state.sh's own comment); the engine emits it unconditionally.
// Guard full-stdout comparisons on this so a future no-jq CI leg (unit 8)
// FAILS LOUDLY on a real divergence rather than silently comparing nothing —
// it still compares the jq-independent arrow line either way.
const JQ_PRESENT = runCommand('bash', ['-c', 'command -v jq >/dev/null 2>&1']).code === 0;

function assertStdoutParity(engineStdout, oracleStdout, msg) {
  if (JQ_PRESENT) {
    assertEqual(engineStdout, oracleStdout, msg);
  } else {
    const engineArrow = engineStdout.split('\n')[0];
    const oracleArrow = oracleStdout.split('\n')[0];
    assertEqual(engineArrow, oracleArrow, `${msg} (arrow line only — no jq on this runner)`);
  }
}

// ---------------------------------------------------------------------------
// Parity matrix — set, with and without an existing cursor. Every fixture
// now also parity-checks stdout, not just cursor bytes.
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
  {
    // Adversarial: empty string is falsy in both bash's `-n` test and JS
    // truthiness, so it must NOT become the literal feature "" — it must
    // fall through to the same carry-forward/clear precedence as "no arg".
    name: 'empty-string feature argument is treated as absent, not as the feature ""',
    start: { flow: 'feature', phase: 'design', feature: 'kept', updated: '2020-01-01T00:00:00Z' },
    target: 'feature.plan',
    feature: '',
    explicitArgs: (fx) => [fx.target, ''],
  },
  {
    // Adversarial: escaping-sensitive characters must survive both jq's
    // --arg quoting and JSON.stringify's escaping identically.
    name: 'feature name with quotes, backslash, and non-ASCII characters',
    start: null,
    target: 'feature.design',
    feature: 'o"Brien\\path\\name — π',
  },
];

for (const fx of FIXTURES) {
  test(`parity: cursor bytes + stdout match oracle — ${fx.name}`, () => {
    const oracle = makeOracleSandbox({ cursor: fx.start ?? undefined });
    const engine = makeSandbox({ cursor: fx.start ?? undefined });
    try {
      const args = fx.explicitArgs ? fx.explicitArgs(fx) : fx.feature ? [fx.target, fx.feature] : [fx.target];

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

      assertStdoutParity(engineResult.stdout, oracleResult.stdout, `stdout diverges for ${fx.name}`);

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
    const oracleBefore = readFileSync(oracle.cursorPath, 'utf8');
    const engineBefore = readFileSync(engine.cursorPath, 'utf8');

    const oracleResult = runOracle(oracle.scriptPath, ['not.a.real.state']);
    const engineResult = runSet(engine.flowDir, ['not.a.real.state']);

    assert(oracleResult.code !== 0, 'oracle should reject an unknown state');
    assertEqual(engineResult.code, 1, 'engine should reject an unknown state');
    assertMatch(engineResult.stderr, /is not a known state/);

    assertEqual(readFileSync(oracle.cursorPath, 'utf8'), oracleBefore, 'oracle cursor must be untouched');
    assertEqual(readFileSync(engine.cursorPath, 'utf8'), engineBefore, 'engine cursor must be untouched');
  } finally {
    oracle.cleanup();
    engine.cleanup();
  }
});

// ---------------------------------------------------------------------------
// Adversarial matrix (review round 1, Finding 3) — each case is a sad path
// that was NOT covered before, asserting exit code AND cursor-bytes-
// unchanged (or correctly-recovered, for the corrupt-cursor case) on both
// sides. Findings 1 and 2 both live in exactly this gap.
// ---------------------------------------------------------------------------

// Finding 1: Object.prototype members must not be readable as machine
// states through a bare `machine.states[key]` lookup.
const PROTOTYPE_KEYS = ['constructor', 'toString', 'valueOf', '__proto__', 'hasOwnProperty'];

for (const key of PROTOTYPE_KEYS) {
  test(`adversarial: prototype key '${key}' is rejected by both, not resolved as a state`, () => {
    const oracle = makeOracleSandbox();
    const engine = makeSandbox();
    try {
      const oracleBefore = readFileSync(oracle.cursorPath, 'utf8');
      const engineBefore = readFileSync(engine.cursorPath, 'utf8');

      const oracleResult = runOracle(oracle.scriptPath, [key]);
      const engineResult = runSet(engine.flowDir, [key]);

      assert(oracleResult.code !== 0, `oracle must reject prototype key '${key}'`);
      assertEqual(engineResult.code, 1, `engine must reject prototype key '${key}'`);
      assertMatch(engineResult.stderr, /is not a known state/);

      assertEqual(readFileSync(oracle.cursorPath, 'utf8'), oracleBefore, `oracle cursor untouched for '${key}'`);
      assertEqual(readFileSync(engine.cursorPath, 'utf8'), engineBefore, `engine cursor untouched for '${key}'`);
    } finally {
      oracle.cleanup();
      engine.cleanup();
    }
  });
}

test('adversarial: stateOf() itself rejects prototype keys directly (root-cause coverage, not just via runSet)', async () => {
  const { loadMachine, stateOf } = await import('../machine.mjs');
  const sandbox = makeSandbox();
  try {
    const machine = loadMachine(sandbox.flowDir);
    for (const key of PROTOTYPE_KEYS) {
      assertEqual(stateOf(machine, key), undefined, `stateOf must not resolve inherited '${key}'`);
    }
    // Sanity: a real state still resolves — this isn't just "always undefined".
    assert(stateOf(machine, 'idle') !== undefined, 'a real state must still resolve');
  } finally {
    sandbox.cleanup();
  }
});

// Finding 2: a corrupt cursor must not brick the one CLI writer path that
// can recover it. The oracle recovers via jq's `// "null" ... || echo
// "null"` fallback; the engine must match that recovery, not throw.
test('adversarial: corrupt (unparseable) cursor recovers on both sides — writer degrades feature to null, exits 0', () => {
  const oracle = makeOracleSandbox();
  const engine = makeSandbox();
  try {
    writeFileSync(oracle.cursorPath, '{ this is not valid json');
    writeFileSync(engine.cursorPath, '{ this is not valid json');

    const oracleResult = runOracle(oracle.scriptPath, ['feature.impl']);
    const engineResult = runSet(engine.flowDir, ['feature.impl']);

    assertEqual(oracleResult.code, 0, `oracle should recover, got stderr: ${oracleResult.stderr}`);
    assertEqual(engineResult.code, 0, `engine should recover, got stderr: ${engineResult.stderr}`);

    const oracleCursor = JSON.parse(readFileSync(oracle.cursorPath, 'utf8'));
    const engineCursor = JSON.parse(readFileSync(engine.cursorPath, 'utf8'));

    assertEqual(oracleCursor.feature, null, 'oracle must degrade the unreadable feature to null');
    assertEqual(engineCursor.feature, null, 'engine must degrade the unreadable feature to null');
    assertEqual(oracleCursor.flow, 'feature');
    assertEqual(engineCursor.flow, 'feature');
    assertEqual(oracleCursor.phase, 'impl');
    assertEqual(engineCursor.phase, 'impl');

    // js-core/8: upgraded from field-by-field assertions (above, kept as
    // named-failure documentation) to full normalized byte parity — the
    // same comparison the FIXTURES matrix above already applies to every
    // well-formed transition, now covering the recovery path too so a
    // regression in key order/indent/trailing-newline on this specific
    // path cannot slip through just because the individual fields matched.
    const oracleBytes = readFileSync(oracle.cursorPath, 'utf8');
    const engineBytes = readFileSync(engine.cursorPath, 'utf8');
    assertEqual(
      normalizeCursorBytes(engineBytes),
      normalizeCursorBytes(oracleBytes),
      `recovered cursor bytes diverge from the oracle\noracle:\n${oracleBytes}\nengine:\n${engineBytes}`,
    );
  } finally {
    oracle.cleanup();
    engine.cleanup();
  }
});

test('adversarial: readCursor() itself still throws CursorParseError on a corrupt cursor — only the writer degrades', () => {
  const sandbox = makeSandbox();
  try {
    writeFileSync(sandbox.cursorPath, '{ this is not valid json');
    let threw = false;
    try {
      readCursor(sandbox.flowDir);
    } catch (err) {
      threw = true;
      assertEqual(err.name, 'CursorParseError');
    }
    assert(threw, 'readCursor must keep throwing — Finding 2 says fix the writer, not the reader');
  } finally {
    sandbox.cleanup();
  }
});

test('adversarial: runGet does NOT degrade a corrupt cursor — reports the error instead of a fake idle', () => {
  const sandbox = makeSandbox();
  try {
    writeFileSync(sandbox.cursorPath, '{ this is not valid json');
    const result = runGet(sandbox.flowDir);
    assertEqual(result.code, 1, 'get must surface a corrupt cursor as an error, not silently answer idle');
    assertMatch(result.stderr, /malformed/);
  } finally {
    sandbox.cleanup();
  }
});

// Missing machine file: both sides must fail loudly, cursor must be left
// exactly as it was.
test('adversarial: missing state-machine.json fails loudly on both sides, cursor untouched', () => {
  const oracle = makeOracleSandbox();
  const engine = makeSandbox();
  try {
    rmSync(oracle.machinePath);
    rmSync(engine.machinePath);

    const oracleBefore = readFileSync(oracle.cursorPath, 'utf8');
    const engineBefore = readFileSync(engine.cursorPath, 'utf8');

    const oracleResult = runOracle(oracle.scriptPath, ['idle']);
    const engineResult = runSet(engine.flowDir, ['idle']);

    assert(oracleResult.code !== 0, 'oracle should fail with no machine file');
    assertEqual(engineResult.code, 1, 'engine should fail with no machine file');
    assertMatch(engineResult.stderr, /state machine/i);

    assertEqual(readFileSync(oracle.cursorPath, 'utf8'), oracleBefore, 'oracle cursor untouched');
    assertEqual(readFileSync(engine.cursorPath, 'utf8'), engineBefore, 'engine cursor untouched');
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

// runSet's header promises it NEVER throws, so unit 7's hook shims may call it
// straight through without a try/catch of their own. Destructuring `args` broke
// that promise before any of the failure paths below it could run: a shim
// handed a null/objecty argument list got a TypeError, not a named result.
// Every hostile shape here must come back as the ordinary no-target error.
test('runSet: a non-array or non-string argument list is a named error, never a throw', () => {
  const sandbox = makeSandbox();
  try {
    const before = readFileSync(sandbox.cursorPath, 'utf8');
    const hostile = [
      ['null', null],
      ['undefined', undefined],
      ['a plain object', {}],
      ['a string', 'feature.impl'],
      ['a number', 7],
      ['an array holding a non-string target', [123]],
      ['an array holding an object target', [{ flow: 'feature' }]],
    ];
    // Floor: the loop must actually have cases, or this test asserts nothing.
    assert(hostile.length === 7, `expected 7 hostile shapes, got ${hostile.length}`);
    for (const [label, args] of hostile) {
      let result;
      try {
        result = runSet(sandbox.flowDir, args);
      } catch (err) {
        assert(false, `${label}: runSet threw instead of returning a result: ${err && err.message}`);
      }
      assertEqual(result.code, 1, `${label}: must be a non-zero named error`);
      assertMatch(result.stderr, /no target state given/, `${label}: and it is the no-target error`);
    }
    assertEqual(readFileSync(sandbox.cursorPath, 'utf8'), before, 'no hostile shape may touch the cursor');

    // Control: the SAME call shape with a real target still works, so the
    // normalization above rejects bad arguments rather than all arguments.
    assertEqual(runSet(sandbox.flowDir, ['feature.impl', 'demo']).code, 0);
  } finally {
    sandbox.cleanup();
  }
});

test('state run(): a non-array argv is the named unknown-subcommand error, not a TypeError', async () => {
  const bareCwd = mkTempRoot('vibe-state-argv-');
  const prevEnv = process.env.CLAUDE_PROJECT_DIR;
  process.env.CLAUDE_PROJECT_DIR = bareCwd;
  const realWrite = process.stderr.write.bind(process.stderr);
  let captured = '';
  process.stderr.write = (chunk) => {
    captured += chunk;
    return true;
  };
  try {
    // The destructuring sits outside run()'s own try/catch, so this used to
    // throw past the result it is supposed to return.
    const codes = [];
    for (const argv of [null, undefined, {}, 'get']) {
      codes.push(await runState(argv, { root: bareCwd }));
    }
    process.stderr.write = realWrite;
    assertEqual(codes.length, 4, 'floor: all four hostile argv shapes must have been called');
    for (const code of codes) assertEqual(code, 1, 'every hostile argv is exit 1');
    assertMatch(captured, /unknown subcommand/);
  } finally {
    process.stderr.write = realWrite;
    if (prevEnv === undefined) delete process.env.CLAUDE_PROJECT_DIR;
    else process.env.CLAUDE_PROJECT_DIR = prevEnv;
    rmSync(bareCwd, { recursive: true, force: true });
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

// Byte-exact, not just JSON.parse — a revert to an unindented
// `JSON.stringify(x)` (no 2-space arg) or a dropped trailing newline would
// still parse fine and this test would not have caught it before. Units 4-6
// consume this exact shape.
test('runGet: stdout is byte-exact pretty JSON (2-space indent, trailing newline), not just parseable', () => {
  const sandbox = makeSandbox({
    cursor: { flow: 'feature', phase: 'impl', feature: 'js-core', updated: '2026-01-01T00:00:00Z' },
  });
  try {
    const result = runGet(sandbox.flowDir);
    const expected =
      '{\n' +
      '  "flow": "feature",\n' +
      '  "phase": "impl",\n' +
      '  "feature": "js-core",\n' +
      '  "updated": "2026-01-01T00:00:00Z",\n' +
      '  "state": "feature.impl"\n' +
      '}\n';
    assertEqual(result.stdout, expected);
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
  cpSync(path.join(REPO_ROOT, 'flow', 'engine'), engineDir, {
    recursive: true,
    filter: (src) => !src.includes(`${path.sep}tests${path.sep}`) && !src.endsWith(`${path.sep}tests`),
  });
  copyFileSync(
    path.join(REPO_ROOT, 'flow', 'state-machine.json'),
    path.join(vibeDir, 'state-machine.json'),
  );

  // The oracle's stdout for the same target, so the expectation is spawned,
  // never hand-copied — even for this CLI-level end-to-end check.
  const oracle = makeOracleSandbox();

  const unrelatedCwd = mkdtempSync(path.join(tmpdir(), 'vibe-state-cli-cwd-'));
  const prevEnv = process.env.CLAUDE_PROJECT_DIR;
  delete process.env.CLAUDE_PROJECT_DIR;

  try {
    const oracleResult = runOracle(oracle.scriptPath, ['quick.triage']);
    assertEqual(oracleResult.code, 0, `oracle failed: ${oracleResult.stderr}`);

    const result = runCommand(
      process.execPath,
      [path.join(engineDir, 'cli.mjs'), 'state', 'set', 'quick.triage'],
      { cwd: unrelatedCwd },
    );
    assertEqual(result.code, 0, `stderr: ${result.stderr}`);
    assertStdoutParity(result.stdout, oracleResult.stdout, 'CLI end-to-end stdout diverges from oracle');

    const cursor = JSON.parse(readFileSync(path.join(vibeDir, 'state.json'), 'utf8'));
    assertEqual(cursor.flow, 'quick');
    assertEqual(cursor.phase, 'triage');
  } finally {
    oracle.cleanup();
    rmSync(installRoot, { recursive: true, force: true });
    rmSync(unrelatedCwd, { recursive: true, force: true });
    if (prevEnv !== undefined) process.env.CLAUDE_PROJECT_DIR = prevEnv;
  }
});

test('CLI: `vibe state` with no subcommand is a named error, not a crash', () => {
  // Defensive sandboxing even though this path never reaches
  // readCursor/loadMachine (see the branch order in state.mjs's run()): cwd
  // is a bare, unrelated temp dir and CLAUDE_PROJECT_DIR is cleared, so a
  // future refactor that makes this path touch a cursor can never resolve
  // to this repo's live flow/state.json.
  const bareCwd = mkdtempSync(path.join(tmpdir(), 'vibe-state-cli-nosubcmd-'));
  const prevEnv = process.env.CLAUDE_PROJECT_DIR;
  process.env.CLAUDE_PROJECT_DIR = bareCwd;
  try {
    const result = runCommand(process.execPath, [path.join(REPO_ROOT, 'flow', 'engine', 'cli.mjs'), 'state'], {
      cwd: bareCwd,
    });
    assertEqual(result.code, 1);
    assertMatch(result.stderr, /unknown subcommand/);
  } finally {
    rmSync(bareCwd, { recursive: true, force: true });
    if (prevEnv === undefined) delete process.env.CLAUDE_PROJECT_DIR;
    else process.env.CLAUDE_PROJECT_DIR = prevEnv;
  }
});
