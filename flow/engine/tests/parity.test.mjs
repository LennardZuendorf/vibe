// engine/tests/parity.test.mjs — the cross-command parity matrix (js-core/8,
// R1, R2, R5): every ported command x 5 cursor states x {jq, no-jq}, plus
// hook.mjs (js-core/7's orchestration layer, not a bash-oracle port itself,
// but held to the same never-diverges bar for its two oracle-backed
// sub-hooks). Each per-command *.test.mjs file already carries its own deep,
// adversarial matrix (missing markers, corrupt cursors, escaping-sensitive
// features, ...) — this file's job is breadth, not depth: one small,
// systematically-looped table proving the whole surface area combines
// cleanly, so a regression that only shows up when e.g. doctor AND a
// feature.impl cursor AND no-jq combine has somewhere to be caught.
//
// The 5 cursor states (tech.md's own matrix definition): cursor absent,
// idle, feature.impl with a feature, feature.impl with a null feature,
// quick.verify.
//
// The no-jq leg reuses the REAL mkshim() (and its mktmp() helper) from
// flow/tests/adapters/run.sh, extracted by function name at test time —
// never hand-copied — so this suite's PATH-stripping is byte-identical to
// the bash suite's own, per tech.md: "The jq-absent leg uses the existing
// mkshim helper ... so both suites shim identically." Do NOT fold doctrine's
// own CLAUDE_PROJECT_DIR cursor-precedence axis in here — that axis is
// doctrine-specific (see root.mjs's resolveProjectCursorDir() header); state,
// orders, doctor, and hook have no such axis, and doctrine.test.mjs already
// owns that coverage in full.

import { mkdtempSync, mkdirSync, writeFileSync, readFileSync, copyFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { test, assert, assertEqual, assertMatch, skip, makeSandbox, runCommand } from './run.mjs';
import { runSet } from '../commands/state.mjs';
import { runOrders } from '../commands/orders.mjs';
import { runDoctrine } from '../commands/doctrine.mjs';
import { runDoctor } from '../commands/doctor.mjs';
import { runDoctrineHook, runInjectHook } from '../commands/hook.mjs';

const REPO_ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..', '..', '..');
const ORDERS_SRC = path.join(REPO_ROOT, 'flow', 'scripts', 'orders.sh');
const DOCTRINE_SRC = path.join(REPO_ROOT, 'flow', 'scripts', 'doctrine.sh');
const SET_STATE_SRC = path.join(REPO_ROOT, 'flow', 'scripts', 'set-state.sh');
const DOCTOR_SRC = path.join(REPO_ROOT, 'flow', 'scripts', 'doctor.sh');
const VALIDATE_STATE_SRC = path.join(REPO_ROOT, 'flow', 'scripts', 'validate-state.sh');
const ADAPTERS_RUN_SH = path.join(REPO_ROOT, 'flow', 'tests', 'adapters', 'run.sh');
const REAL_SKILL_MD = readFileSync(path.join(REPO_ROOT, 'flow', 'SKILL.md'), 'utf8');
const REAL_MACHINE_SRC = path.join(REPO_ROOT, 'flow', 'state-machine.json');
const REAL_DEPS_SRC = path.join(REPO_ROOT, 'flow', 'reference', 'deps.json');

const JQ_PRESENT = runCommand('bash', ['-c', 'command -v jq >/dev/null 2>&1']).code === 0;

// ---------------------------------------------------------------------------
// mkshim() reuse — extracted verbatim from adapters/run.sh, not sourced (the
// rest of that file is a monolithic test script, not a library) and not
// hand-copied. A single lazily-built, memoized "jq excluded" shim dir serves
// the whole matrix, mirroring doctor.test.mjs's own noJqPath() memoization
// pattern (left in the OS tmpdir for the run, same as that helper).
// ---------------------------------------------------------------------------

function extractBashFunction(src, name) {
  const lines = src.split('\n');
  const startIdx = lines.findIndex((l) => l.trim().startsWith(`${name}() {`));
  if (startIdx === -1) {
    throw new Error(`bash function '${name}' not found in ${ADAPTERS_RUN_SH} — its shape changed`);
  }
  const startLine = lines[startIdx];
  if (/\}\s*$/.test(startLine)) return startLine; // single-line function
  const body = [startLine];
  for (let i = startIdx + 1; i < lines.length; i += 1) {
    body.push(lines[i]);
    if (lines[i].trim() === '}') break;
  }
  return body.join('\n');
}

const ADAPTERS_SRC = readFileSync(ADAPTERS_RUN_SH, 'utf8');
const MKTMP_FN = extractBashFunction(ADAPTERS_SRC, 'mktmp');
const MKSHIM_FN = extractBashFunction(ADAPTERS_SRC, 'mkshim');

let cachedNoJqShimDir;
function noJqShimDir() {
  if (cachedNoJqShimDir !== undefined) return cachedNoJqShimDir;
  const script = `${MKTMP_FN}\n${MKSHIM_FN}\nmkshim jq`;
  const res = runCommand('bash', ['-c', script]);
  assert(res.code === 0, `mkshim() failed while building the no-jq shim: ${res.stderr}`);
  const dir = res.stdout.trim();
  assert(dir.length > 0, 'mkshim() printed an empty dir path');

  // Review round 1, Minor 3: a semantically-broken mkshim BODY (one that
  // still leaves jq reachable) was caught only incidentally — 5 doctor legs
  // went red because doctor is handed jqPresent:false while its oracle sees
  // jq, and the other 20 no-jq legs would have run WITH jq and reported
  // green. Assert the postcondition directly, so shim failure is loud and
  // immediate rather than a side effect five tests later.
  const probe = runCommand(path.join(dir, 'bash'), ['-c', 'command -v jq'], { env: { PATH: dir } });
  assert(
    probe.code !== 0 && probe.stdout.trim() === '',
    `mkshim jq produced a dir where jq is STILL reachable (${probe.stdout.trim()}) — every "no-jq" leg below would silently run with jq`,
  );

  cachedNoJqShimDir = dir;
  return cachedNoJqShimDir;
}

// Every `x jq` leg below needs jq to actually be on this runner's PATH.
// Without it, runOracleScript('jq') just inherits the same jq-less PATH the
// 'no-jq' leg pins, so the jq half of the matrix silently degenerates into a
// byte-identical duplicate of the no-jq half — 30 tests reporting `ok` while
// asserting nothing new. CI's own "engine suite with jq stripped from PATH"
// step runs on exactly that PATH, so this is not hypothetical. A real skip
// keeps the counts honest (review round 1, Finding 4).
function requireJqFor(mode) {
  if (mode === 'jq' && !JQ_PRESENT) {
    skip('requires jq on PATH — the jq leg would otherwise duplicate the no-jq leg and assert nothing new');
  }
}

// Runs a bash ORACLE script under a given jq mode. 'jq' inherits this
// process's real PATH unchanged (whatever the runner actually has — the
// per-test JQ_PRESENT gates below account for a runner with no jq at all);
// 'no-jq' runs it via the shim's own bash, PATH pinned to the shim dir, the
// same invocation shape adapters/run.sh's own mkshim call sites use
// (`PATH="$NOJQ" "$NOJQ/bash" ...`), so jq is unreachable no matter what the
// ambient PATH holds.
function runOracleScript(scriptPath, args, mode, extraEnv = {}) {
  if (mode === 'no-jq') {
    const dir = noJqShimDir();
    return runCommand(path.join(dir, 'bash'), [scriptPath, ...args], { env: { PATH: dir, ...extraEnv } });
  }
  return runCommand('bash', [scriptPath, ...args], { env: extraEnv });
}

// ---------------------------------------------------------------------------
// The 5-state cursor matrix (shared verbatim across every command below).
// ---------------------------------------------------------------------------

const FIXED_TS = '2026-01-01T00:00:00Z';
const JQ_MODES = ['jq', 'no-jq'];

const CURSOR_FIXTURES = [
  { name: 'cursor absent', absent: true },
  { name: 'idle', cursor: { flow: 'idle', phase: 'idle', feature: null, updated: FIXED_TS } },
  { name: 'feature.impl with a feature', cursor: { flow: 'feature', phase: 'impl', feature: 'widget', updated: FIXED_TS } },
  { name: 'feature.impl with a null feature', cursor: { flow: 'feature', phase: 'impl', feature: null, updated: FIXED_TS } },
  { name: 'quick.verify', cursor: { flow: 'quick', phase: 'verify', feature: null, updated: FIXED_TS } },
];

assertEqual(CURSOR_FIXTURES.length, 5, 'sanity: the matrix is defined over exactly 5 cursor states');

// ---------------------------------------------------------------------------
// Sandbox builders.
//
// state/orders/doctrine/hook share ONE combined, READ-shaped sandbox layout
// (mirrors state.test.mjs/orders.test.mjs/doctrine.test.mjs's own
// makeSandbox()-based fixtures exactly: vibeDir === sandbox.flowDir,
// skillsDir === sandbox.dir/.agents/skills) — safe to share because orders/
// doctrine/hook never write the cursor. state DOES write it, so its own
// sub-matrix below builds a fresh oracle+engine PAIR per case instead of
// reusing this one, the same isolation state.test.mjs's FIXTURES loop uses.
//
// doctor needs a different, install-shaped layout (root separate from
// vibeDir) and gets its own builder.
// ---------------------------------------------------------------------------

function makeReadSandbox(fixture) {
  const sandbox = makeSandbox({ cursor: fixture.cursor });
  if (fixture.absent) rmSync(sandbox.cursorPath, { force: true });

  const scriptsDir = path.join(sandbox.flowDir, 'scripts');
  mkdirSync(scriptsDir, { recursive: true });
  copyFileSync(ORDERS_SRC, path.join(scriptsDir, 'orders.sh'));
  copyFileSync(DOCTRINE_SRC, path.join(scriptsDir, 'doctrine.sh'));

  const skillsDir = path.join(sandbox.dir, '.agents', 'skills');
  const vibeSkillDir = path.join(skillsDir, 'vibe');
  mkdirSync(vibeSkillDir, { recursive: true });
  writeFileSync(path.join(vibeSkillDir, 'SKILL.md'), REAL_SKILL_MD);

  return {
    ...sandbox,
    ordersPath: path.join(scriptsDir, 'orders.sh'),
    doctrinePath: path.join(scriptsDir, 'doctrine.sh'),
    skillsDir,
  };
}

function makeStateOracleSandbox(fixture) {
  const sandbox = makeSandbox({ cursor: fixture.cursor });
  if (fixture.absent) rmSync(sandbox.cursorPath, { force: true });
  const scriptsDir = path.join(sandbox.flowDir, 'scripts');
  mkdirSync(scriptsDir, { recursive: true });
  const scriptPath = path.join(scriptsDir, 'set-state.sh');
  copyFileSync(SET_STATE_SRC, scriptPath);
  return { ...sandbox, scriptPath };
}

function makeStateEngineSandbox(fixture) {
  const sandbox = makeSandbox({ cursor: fixture.cursor });
  if (fixture.absent) rmSync(sandbox.cursorPath, { force: true });
  return sandbox;
}

const DOCTOR_HOOK_SCRIPTS = ['session-start-doctrine.sh', 'user-prompt-submit-inject.sh', 'pre-tool-use-guard.sh', 'stop-gate.sh'];

function makeDoctorSandbox(fixture) {
  const dir = mkdtempSync(path.join(tmpdir(), 'vibe-parity-doctor-'));
  mkdirSync(path.join(dir, '.spec'), { recursive: true }); // marker for doctor.sh's find_root()

  const scriptsDir = path.join(dir, 'flow', 'scripts');
  mkdirSync(scriptsDir, { recursive: true });
  const scriptPath = path.join(scriptsDir, 'doctor.sh');
  copyFileSync(DOCTOR_SRC, scriptPath);

  const skillsDir = path.join(dir, '.agents', 'skills');
  const vibeDir = path.join(skillsDir, 'vibe');
  mkdirSync(path.join(skillsDir, 'spec'), { recursive: true }); // core.spec: ok
  mkdirSync(vibeDir, { recursive: true });
  copyFileSync(REAL_MACHINE_SRC, path.join(vibeDir, 'state-machine.json'));
  writeFileSync(path.join(vibeDir, 'SKILL.md'), REAL_SKILL_MD);

  const validateDir = path.join(vibeDir, 'scripts');
  mkdirSync(validateDir, { recursive: true });
  const validatePath = path.join(validateDir, 'validate-state.sh');
  copyFileSync(VALIDATE_STATE_SRC, validatePath);
  // chmod +x — matches doctor.test.mjs's addValidateState({executable: true}).
  runCommand('chmod', ['+x', validatePath]);

  const refDir = path.join(vibeDir, 'reference');
  mkdirSync(refDir, { recursive: true });
  copyFileSync(REAL_DEPS_SRC, path.join(refDir, 'deps.json'));

  const cursorPath = path.join(vibeDir, 'state.json');
  if (!fixture.absent) {
    writeFileSync(cursorPath, `${JSON.stringify(fixture.cursor, null, 2)}\n`);
  }

  let cleaned = false;
  function cleanup() {
    if (cleaned) return;
    cleaned = true;
    rmSync(dir, { recursive: true, force: true });
  }

  return { dir, root: dir, scriptPath, vibeDir, skillsDir, cursorPath, cleanup };
}

// Cursor bytes with `updated` blanked — same normalization state.test.mjs's
// FIXTURES matrix uses, reused here rather than re-derived.
function normalizeCursorBytes(raw) {
  return raw.replace(/"updated": "[^"]*"/, '"updated": "<TS>"');
}

// ---------------------------------------------------------------------------
// orders — 5 fixtures x {jq, no-jq}. Well-formed cursors only (this matrix's
// job is breadth), so the oracle's no-jq degrade is byte-identical to its
// own jq path (verified separately, per-fixture, below) — full stdout
// equality is asserted unconditionally on both legs.
// ---------------------------------------------------------------------------

for (const fixture of CURSOR_FIXTURES) {
  for (const mode of JQ_MODES) {
    test(`parity matrix: orders — ${fixture.name} x ${mode}`, () => {
      requireJqFor(mode);
      const sandbox = makeReadSandbox(fixture);
      try {
        const oracleResult = runOracleScript(sandbox.ordersPath, [], mode);
        const engineResult = runOrders(sandbox.flowDir, sandbox.skillsDir, []);
        assertEqual(oracleResult.code, 0, `oracle failed (${mode}): ${oracleResult.stderr}`);
        assertEqual(engineResult.code, 0);
        assertEqual(
          engineResult.stdout,
          oracleResult.stdout,
          `orders parity broke for ${fixture.name} x ${mode}\noracle: ${JSON.stringify(oracleResult.stdout)}\nengine: ${JSON.stringify(engineResult.stdout)}`,
        );
      } finally {
        sandbox.cleanup();
      }
    });
  }
}

// ---------------------------------------------------------------------------
// doctrine — same 5 x 2 grid.
// ---------------------------------------------------------------------------

for (const fixture of CURSOR_FIXTURES) {
  for (const mode of JQ_MODES) {
    test(`parity matrix: doctrine — ${fixture.name} x ${mode}`, () => {
      requireJqFor(mode);
      const sandbox = makeReadSandbox(fixture);
      const prevEnv = process.env.CLAUDE_PROJECT_DIR;
      delete process.env.CLAUDE_PROJECT_DIR; // never let the ambient session env redirect the cursor read
      try {
        const oracleResult = runOracleScript(sandbox.doctrinePath, [], mode);
        const engineResult = runDoctrine(sandbox.flowDir, sandbox.skillsDir);
        assertEqual(oracleResult.code, 0, `oracle failed (${mode}): ${oracleResult.stderr}`);
        assertEqual(engineResult.code, 0);
        assertEqual(
          engineResult.stdout,
          oracleResult.stdout,
          `doctrine parity broke for ${fixture.name} x ${mode}\noracle: ${JSON.stringify(oracleResult.stdout)}\nengine: ${JSON.stringify(engineResult.stdout)}`,
        );
      } finally {
        sandbox.cleanup();
        if (prevEnv !== undefined) process.env.CLAUDE_PROJECT_DIR = prevEnv;
      }
    });
  }
}

// ---------------------------------------------------------------------------
// state — 5 x 2. Writer, so oracle/engine get independent sandboxes (never
// shared with the read-only commands above). Target is a single neutral,
// always-legal state ('quick.fix', distinct from every fixture) so the same
// grid exercises carry-forward/clear precedence identically from every
// starting cursor. Byte parity on the written cursor (normalized), plus
// stdout parity gated on whether jq was ACTUALLY available for that leg
// (mode='jq' only proves the jq path when this runner truly has jq — same
// gating state.test.mjs's own assertStdoutParity uses).
// ---------------------------------------------------------------------------

const STATE_TARGET = 'quick.fix';

for (const fixture of CURSOR_FIXTURES) {
  for (const mode of JQ_MODES) {
    test(`parity matrix: state — ${fixture.name} x ${mode}`, () => {
      requireJqFor(mode);
      const oracle = makeStateOracleSandbox(fixture);
      const engine = makeStateEngineSandbox(fixture);
      try {
        const oracleResult = runOracleScript(oracle.scriptPath, [STATE_TARGET], mode);
        const engineResult = runSet(engine.flowDir, [STATE_TARGET]);
        assertEqual(oracleResult.code, 0, `oracle failed (${mode}): ${oracleResult.stderr}`);
        assertEqual(engineResult.code, 0, `engine failed: ${engineResult.stderr}`);

        const oracleBytes = readFileSync(oracle.cursorPath, 'utf8');
        const engineBytes = readFileSync(engine.cursorPath, 'utf8');
        assertEqual(
          normalizeCursorBytes(engineBytes),
          normalizeCursorBytes(oracleBytes),
          `cursor bytes diverge for ${fixture.name} x ${mode}\noracle:\n${oracleBytes}\nengine:\n${engineBytes}`,
        );

        // mode === 'jq' now implies jq really is present — requireJqFor()
        // skipped the leg otherwise, instead of silently downgrading to the
        // first-line-only comparison below while still printing a full pass
        // (review round 1, Finding 4).
        if (mode === 'jq') {
          assertEqual(engineResult.stdout, oracleResult.stdout, `stdout diverges for ${fixture.name} x ${mode}`);
        } else {
          // The oracle's `next:` line is jq-only; the engine emits it
          // unconditionally — compare only the jq-independent arrow line.
          assertEqual(
            engineResult.stdout.split('\n')[0],
            oracleResult.stdout.split('\n')[0],
            `arrow line diverges for ${fixture.name} x ${mode} (no jq on this leg)`,
          );
        }
      } finally {
        oracle.cleanup();
        engine.cleanup();
      }
    });
  }
}

// ---------------------------------------------------------------------------
// doctor — 5 x 2. jqPresent is passed explicitly to the engine (it never
// shells to jq for its own logic; doctor.mjs's tool.jq check is the one
// genuine `jq --version` spawn — see doctor.mjs's header), matching what the
// oracle leg actually had on PATH for that run.
// ---------------------------------------------------------------------------

const DOCTOR_HOME = mkdtempSync(path.join(tmpdir(), 'vibe-parity-doctor-home-'));

// Module-scope tmpdirs have no per-test finally to clean them, so they leaked
// one directory per run (review round 1, Minor 4). Removed on process exit
// instead — after the last test that could still need them.
process.on('exit', () => {
  rmSync(DOCTOR_HOME, { recursive: true, force: true });
  if (cachedNoJqShimDir) rmSync(cachedNoJqShimDir, { recursive: true, force: true });
});

for (const fixture of CURSOR_FIXTURES) {
  for (const mode of JQ_MODES) {
    test(`parity matrix: doctor — ${fixture.name} x ${mode}`, () => {
      requireJqFor(mode);
      const sandbox = makeDoctorSandbox(fixture);
      try {
        const oracleResult = runOracleScript(sandbox.scriptPath, [], mode, { HOME: DOCTOR_HOME });
        // 'jq' mode: omit the override entirely so runDoctor() does REAL jq
        // detection (spawns `jq --version`), matching production — a
        // forced `jqPresent: true` with no version text would wrongly
        // report "jq present ()" instead of the real version string.
        const engineOpts = { home: DOCTOR_HOME };
        if (mode === 'no-jq') engineOpts.jqPresent = false;
        const engineResult = runDoctor(sandbox.root, sandbox.vibeDir, sandbox.skillsDir, engineOpts);
        assertEqual(oracleResult.code, 0, `oracle failed (${mode}): ${oracleResult.stderr}`);
        assertEqual(engineResult.code, 0);
        assertEqual(
          engineResult.stdout,
          oracleResult.stdout,
          `doctor parity broke for ${fixture.name} x ${mode}\noracle:\n${oracleResult.stdout}\nengine:\n${engineResult.stdout}`,
        );
      } finally {
        sandbox.cleanup();
      }
    });
  }
}

// ---------------------------------------------------------------------------
// hook — added to the matrix per the task brief. hook.mjs is NOT a single
// bash-oracle port (see its own header), so there is no `hook.sh` to spawn;
// the two sub-hooks with an oracle-comparable shape are session-start-
// doctrine (a thin pass-through to runDoctrine — compared against
// doctrine.sh) and user-prompt-submit-inject (orders.sh's output plus a
// drift check and a warnings-log drain, both empty here: no
// detect-context.sh in this sandbox, so drift is skipped entirely, and no
// warnings were ever queued — reducing byte-for-byte to orders.sh's own
// output). guard/gate have no bash-oracle-comparable single-command shape
// (they wrap detect-context.sh's own decide/infer, already matrixed by
// hook.test.mjs's integration tests) and are out of scope here.
// ---------------------------------------------------------------------------

for (const fixture of CURSOR_FIXTURES) {
  for (const mode of JQ_MODES) {
    test(`parity matrix: hook session-start-doctrine — ${fixture.name} x ${mode}`, () => {
      requireJqFor(mode);
      const sandbox = makeReadSandbox(fixture);
      const prevEnv = process.env.CLAUDE_PROJECT_DIR;
      delete process.env.CLAUDE_PROJECT_DIR;
      try {
        const oracleResult = runOracleScript(sandbox.doctrinePath, [], mode);
        const hookResult = runDoctrineHook(sandbox.flowDir, sandbox.skillsDir);
        assertEqual(oracleResult.code, 0, `oracle failed (${mode}): ${oracleResult.stderr}`);
        assertEqual(hookResult.code, 0);
        assertEqual(
          hookResult.stdout,
          oracleResult.stdout,
          `hook session-start-doctrine parity broke for ${fixture.name} x ${mode}`,
        );
      } finally {
        sandbox.cleanup();
        if (prevEnv !== undefined) process.env.CLAUDE_PROJECT_DIR = prevEnv;
      }
    });

    test(`parity matrix: hook user-prompt-submit-inject — ${fixture.name} x ${mode}`, () => {
      requireJqFor(mode);
      const sandbox = makeReadSandbox(fixture);
      try {
        const oracleResult = runOracleScript(sandbox.ordersPath, [], mode);
        const hookResult = runInjectHook(sandbox.dir, sandbox.flowDir, sandbox.skillsDir, {});
        assertEqual(oracleResult.code, 0, `oracle failed (${mode}): ${oracleResult.stderr}`);
        assertEqual(hookResult.code, 0);
        assertEqual(
          hookResult.stdout,
          oracleResult.stdout,
          `hook user-prompt-submit-inject parity broke for ${fixture.name} x ${mode} (no detect-context.sh in this sandbox, so this reduces to orders.sh parity)`,
        );
      } finally {
        sandbox.cleanup();
      }
    });
  }
}

// ---------------------------------------------------------------------------
// KNOWN DIVERGENCE (documented, not a bug — do NOT "fix" the engine to
// match): a hand-edited cursor with a NUMERIC `flow` of 5.
//
// doctrine.sh's current_state() computes `flow=$(jq -r '.flow // "idle"'
// "$STATE")` — jq's `//` treats a number as truthy (only `false`/`null`
// fall through), so it prints the literal "5", and doctrine.sh echoes
// "Cursor: 5.impl." with NO validation against the machine at all (unlike
// doctor's cursor check, which delegates to validate-state.sh and would
// reject "5.impl" as an unknown state on both sides identically — no
// divergence there). readCursor()'s own contract (cursor.mjs) requires
// `typeof raw.flow === 'string'` before accepting it and defaults to "idle"
// otherwise — that guard exists specifically so a corrupted/wrong-typed
// field can never be silently treated as valid (see cursor.mjs's header on
// CursorParseError), so the engine reports "Cursor: idle.impl." instead.
//
// Only a hand-edited cursor can ever produce this (writeCursor() never
// emits a non-string flow), but it is a REAL, directly observable
// divergence once it happens — pinned here per the task brief so it can
// never drift silently.
// ---------------------------------------------------------------------------

test('KNOWN DIVERGENCE: cursor.flow = 5 (number) — oracle "Cursor: 5.impl.", engine "Cursor: idle.impl." — do not change this', () => {
  // The divergence lives specifically in jq's `//` truthiness; without jq on
  // this runner the oracle takes its sed degrade path instead (a DIFFERENT,
  // already-covered code path), so there is nothing to pin here. This MUST be
  // skip(), not a bare `return`: a return reports `ok` and is indistinguishable
  // from a pin that actually ran — and CI's jq-stripped leg runs on exactly
  // that PATH, so the pin read green while asserting nothing (review round 1,
  // Finding 4).
  if (!JQ_PRESENT) {
    skip("requires jq on PATH — the divergence lives in jq's // truthiness");
  }
  const sandbox = makeReadSandbox({ cursor: { flow: 5, phase: 'impl', feature: null, updated: FIXED_TS } });
  const prevEnv = process.env.CLAUDE_PROJECT_DIR;
  delete process.env.CLAUDE_PROJECT_DIR;
  try {
    const oracleResult = runOracleScript(sandbox.doctrinePath, [], 'jq');
    const engineResult = runDoctrine(sandbox.flowDir, sandbox.skillsDir);
    assertEqual(oracleResult.code, 0);
    assertEqual(engineResult.code, 0);

    assertMatch(oracleResult.stdout, /\nCursor: 5\.impl\.\n$/, 'oracle sanity: jq\'s // treats the number 5 as truthy');
    assertMatch(engineResult.stdout, /\nCursor: idle\.impl\.\n$/, 'engine: a non-string flow is never trusted, defaults to idle');

    assert(
      engineResult.stdout !== oracleResult.stdout,
      'this IS the documented divergence — if this ever passes as equal, the pin itself needs updating, not silently deleting',
    );
  } finally {
    sandbox.cleanup();
    if (prevEnv !== undefined) process.env.CLAUDE_PROJECT_DIR = prevEnv;
  }
});
