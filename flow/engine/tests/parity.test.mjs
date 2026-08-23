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

import {
  mkdirSync,
  writeFileSync,
  readFileSync,
  copyFileSync,
  existsSync,
  rmSync,
  utimesSync,
  symlinkSync,
} from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import {
  test,
  assert,
  assertEqual,
  assertMatch,
  skip,
  makeSandbox,
  makeHookSandbox,
  runCommand,
  runCli,
  mkTempRoot,
  registeredTests,
} from './run.mjs';
import { runSet } from '../commands/state.mjs';
import { runOrders } from '../commands/orders.mjs';
import { runDoctrine } from '../commands/doctrine.mjs';
import { runDoctor } from '../commands/doctor.mjs';
import {
  runDoctrineHook,
  runInjectHook,
  runGuardHook,
  runGateHook,
} from '../commands/hook.mjs';

const REPO_ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..', '..', '..');
const ORDERS_SRC = path.join(REPO_ROOT, 'flow', 'scripts', 'orders.sh');
const DOCTRINE_SRC = path.join(REPO_ROOT, 'flow', 'scripts', 'doctrine.sh');
const SET_STATE_SRC = path.join(REPO_ROOT, 'flow', 'scripts', 'set-state.sh');
const DOCTOR_SRC = path.join(REPO_ROOT, 'flow', 'scripts', 'doctor.sh');
const VALIDATE_STATE_SRC = path.join(REPO_ROOT, 'flow', 'scripts', 'validate-state.sh');
const ADAPTERS_RUN_SH = path.join(REPO_ROOT, 'flow', 'tests', 'adapters', 'run.sh');
// The two FROZEN pre-port bash hooks (flow/hooks-fallback/, restored verbatim from
// `main`) — the only oracles guard/gate have, because js-core/7 replaced the
// real hooks with Node shims IN PLACE. See those files' own headers.
const GUARD_ORACLE = path.join(REPO_ROOT, 'flow', 'hooks-fallback', 'pre-tool-use-guard.sh');
const GATE_ORACLE = path.join(REPO_ROOT, 'flow', 'hooks-fallback', 'stop-gate.sh');
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
function runOracleScript(scriptPath, args, mode, extraEnv = {}, extraOpts = {}) {
  if (mode === 'no-jq') {
    const dir = noJqShimDir();
    return runCommand(path.join(dir, 'bash'), [scriptPath, ...args], {
      env: { PATH: dir, ...extraEnv },
      ...extraOpts,
    });
  }
  return runCommand('bash', [scriptPath, ...args], { env: extraEnv, ...extraOpts });
}

// The two blocking hooks shell out (guard -> detect-context.sh decide; gate ->
// git). Their no-jq leg must strip jq from the SPAWNED process too, or the
// oracle runs jq-less while the engine's own sub-process still finds jq and the
// "no-jq" comparison is only half honest. mkshim()'s tool list carries no `git`,
// so the gate's no-jq leg gets it symlinked in — that is an augmentation of the
// PATH the shim builds, never an edit to mkshim's own extracted body (jq
// exclusion, the only thing the shim exists for, is untouched and still
// postcondition-asserted above).
let gitLinkedIntoShim = false;
function noJqBash() {
  const dir = noJqShimDir();
  if (!gitLinkedIntoShim) {
    gitLinkedIntoShim = true;
    const gitPath = runCommand('bash', ['-c', 'command -v git || true']).stdout.trim();
    if (gitPath) {
      try {
        symlinkSync(gitPath, path.join(dir, 'git'));
      } catch {
        // already linked by a previous run in this process — fine
      }
    }
  }
  return { bash: path.join(dir, 'bash'), env: { PATH: dir } };
}

// A spawn* function shaped like the ones hook.mjs injects (returns
// {error, status, stdout}), routed through the same interpreter/PATH the oracle
// leg used, so both sides of a no-jq comparison really are jq-less.
function asSpawnResult(r) {
  return { error: r.error, status: r.code, stdout: r.stdout, stderr: r.stderr };
}

function modeSpawn(mode, cwd) {
  if (mode === 'no-jq') {
    const { bash, env } = noJqBash();
    return {
      spawnDecide: (args) => asSpawnResult(runCommand(bash, args, { env, cwd })),
      spawnGit: (args) => asSpawnResult(runCommand(path.join(noJqShimDir(), 'git'), args, { env, cwd })),
    };
  }
  return {
    spawnDecide: (args) => asSpawnResult(runCommand('bash', args, { cwd })),
    spawnGit: (args) => asSpawnResult(runCommand('git', args, { cwd })),
  };
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
  const dir = mkTempRoot('vibe-parity-doctor-');
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
        const engineResult = runDoctrine(sandbox.skillsDir);
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

const DOCTOR_HOME = mkTempRoot('vibe-parity-doctor-home-');

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
// hook pre-tool-use-guard / hook stop-gate — THE TWO BLOCKING HOOKS, against
// the frozen bash oracles (js-core/8 final review, I3).
//
// Until now the matrix imported only runDoctrineHook/runInjectHook, so the two
// exported functions that can exit 2 — stop a tool call, stop a turn — had NO
// bash-oracle comparison anywhere in the tree. Not because the port was
// untrusted, but because js-core/7 replaced the real `.claude/hooks/*.sh` with
// Node shims IN PLACE: the oracles ceased to exist, and after this branch
// merges `main` does not hold them either. The final review had to reconstruct
// them from git history to run the differential once.
//
// flow/hooks-fallback/{pre-tool-use-guard,stop-gate}.sh are those bash originals,
// restored verbatim and frozen. This section is the differential made
// PERMANENT: every comparison below is regenerable by anyone, in the suite, on
// every run — which is the property the branch was missing, not the parity.
//
// Compared per case: exit code, stdout, stderr, AND the warnings-relay log
// bytes (a warn that reaches stderr but never the relay is invisible to the
// model next turn — a real behaviour difference the first three would miss).
// ---------------------------------------------------------------------------

function readWarnLog(sb) {
  try {
    return readFileSync(sb.warnLogPath, 'utf8');
  } catch {
    return '';
  }
}

function resetWarnLog(sb) {
  rmSync(sb.warnLogPath, { force: true });
}

// The oracle self-locates everything off $ROOT = CLAUDE_PROJECT_DIR (never a
// vibeDir argument), so the sandbox root must be handed to it explicitly —
// inheriting the ambient session's CLAUDE_PROJECT_DIR would point the oracle at
// this very repo while the engine reads the sandbox.
function oracleEnv(sb) {
  return { CLAUDE_PROJECT_DIR: sb.root };
}

// One (oracle, engine) pair -> a divergence string, or null when identical.
function compareHook(label, oracleRun, engineRun) {
  const parts = [];
  if (oracleRun.code !== engineRun.code) parts.push(`rc bash=${oracleRun.code} node=${engineRun.code}`);
  if (oracleRun.stdout !== engineRun.stdout) {
    parts.push(`stdout bash=${JSON.stringify(oracleRun.stdout)} node=${JSON.stringify(engineRun.stdout)}`);
  }
  if (oracleRun.stderr !== engineRun.stderr) {
    parts.push(`stderr bash=${JSON.stringify(oracleRun.stderr)} node=${JSON.stringify(engineRun.stderr)}`);
  }
  if (oracleRun.log !== engineRun.log) {
    parts.push(`warnings.log bash=${JSON.stringify(oracleRun.log)} node=${JSON.stringify(engineRun.log)}`);
  }
  return parts.length ? `${label}: ${parts.join(' | ')}` : null;
}

// ---------------------------------------------------------------------------
// guard — 5 fixtures x {jq, no-jq}, each looping every stdin shape below.
//
// Shapes cover both handler arms (Bash sniffer / file-tool path policy), all
// three verdict classes (allow / warn / block), the sanctioned-writer exemption,
// the absolute-path strip, and every degrade the oracle spells out (empty
// stdin, unparseable stdin, missing tool_name, missing path).
//
// WHICH BRANCH THIS MATRIX COMPARES: the guard answers from policy.mjs
// IN-PROCESS when the install carries usable policy data, and falls back to
// spawning detect-context.sh when it does not (inject-triggers/5). This
// fixture ships no content/policy.json, so the fallback is what runs — which is
// the branch the frozen bash oracle also takes, and therefore the only
// comparison that means anything here. Asserted, not assumed: a fixture that
// silently grew a policy file would turn every case below into a comparison of
// the engine against itself. hook.test.mjs owns the in-process branch, and
// pins the two branches against each other directly.
// ---------------------------------------------------------------------------

const GUARD_STDIN_SHAPES = [
  ['empty stdin', () => ''],
  ['unparseable stdin', () => 'not json at all'],
  ['empty object', () => '{}'],
  ['Bash, no command key', () => '{"tool_name":"Bash","tool_input":{}}'],
  ['Bash, harmless command', () => '{"tool_name":"Bash","tool_input":{"command":"echo hi"}}'],
  ['Bash, READ of a guarded path (no write op)', () => '{"tool_name":"Bash","tool_input":{"command":"cat .spec/lessons.md"}}'],
  ['Bash, append to lessons', () => '{"tool_name":"Bash","tool_input":{"command":"echo x >> .spec/lessons.md"}}'],
  ['Bash, sed -i a root spec', () => '{"tool_name":"Bash","tool_input":{"command":"sed -i s/a/b/ .spec/product.md"}}'],
  ['Bash, tee a root spec', () => '{"tool_name":"Bash","tool_input":{"command":"tee .spec/tech.md"}}'],
  ['Bash, redirect into the cursor', () => '{"tool_name":"Bash","tool_input":{"command":"echo x > .agents/skills/vibe/state.json"}}'],
  ['Bash, cursor write VIA set-state.sh (sanctioned)', () => '{"tool_name":"Bash","tool_input":{"command":"bash .agents/skills/vibe/scripts/set-state.sh idle > .agents/skills/vibe/state.json"}}'],
  ['Bash, rm the flow/ cursor spelling', () => '{"tool_name":"Bash","tool_input":{"command":"rm flow/state.json"}}'],
  ['Write, no path key', () => '{"tool_name":"Write","tool_input":{}}'],
  ['Write, lessons.md', () => '{"tool_name":"Write","tool_input":{"file_path":".spec/lessons.md"}}'],
  ['Write, root product.md', () => '{"tool_name":"Write","tool_input":{"file_path":".spec/product.md"}}'],
  ['Edit, the cursor itself', () => '{"tool_name":"Edit","tool_input":{"file_path":".agents/skills/vibe/state.json"}}'],
  ['Write, ordinary source file', () => '{"tool_name":"Write","tool_input":{"file_path":"src/app.ts"}}'],
  ['Write, a FEATURE spec (not a root one)', () => '{"tool_name":"Write","tool_input":{"file_path":".spec/features/demo/product.md"}}'],
  ['Write, ./-prefixed lessons', () => '{"tool_name":"Write","tool_input":{"file_path":"./.spec/lessons.md"}}'],
  ['NotebookEdit, notebook_path on a root spec', () => '{"tool_name":"NotebookEdit","tool_input":{"notebook_path":".spec/plan.md"}}'],
  ['Write, ABSOLUTE path under the root (must be stripped)', (sb) => JSON.stringify({ tool_name: 'Write', tool_input: { file_path: path.join(sb.root, '.spec', 'lessons.md') } })],
];

for (const fixture of CURSOR_FIXTURES) {
  for (const mode of JQ_MODES) {
    test(`parity matrix: hook pre-tool-use-guard — ${fixture.name} x ${mode}`, () => {
      requireJqFor(mode);
      const sb = makeHookSandbox({ cursor: fixture.cursor });
      if (fixture.absent) rmSync(sb.cursorPath, { force: true });
      assert(
        !existsSync(path.join(sb.vibeDir, 'content', 'policy.json')),
        'this matrix compares the SPAWN branch: a policy.json in the fixture would make the guard answer ' +
          'in-process and stop exercising the bash oracle at all',
      );
      const spawns = modeSpawn(mode, sb.root);
      const divergences = [];
      try {
        for (const [shapeName, build] of GUARD_STDIN_SHAPES) {
          const stdin = build(sb);

          resetWarnLog(sb);
          const o = runOracleScript(GUARD_ORACLE, [], mode, oracleEnv(sb), { cwd: sb.root, input: stdin });
          const oracleRun = { code: o.code, stdout: o.stdout, stderr: o.stderr, log: readWarnLog(sb) };

          resetWarnLog(sb);
          const e = runGuardHook(sb.root, stdin, { spawnDecide: spawns.spawnDecide });
          const engineRun = { code: e.code, stdout: e.stdout, stderr: e.stderr, log: readWarnLog(sb) };

          const d = compareHook(shapeName, oracleRun, engineRun);
          if (d) divergences.push(d);
        }
        assertEqual(
          divergences,
          [],
          `guard diverged from its frozen bash oracle (${fixture.name} x ${mode}):\n${divergences.join('\n')}`,
        );
      } finally {
        sb.cleanup();
      }
    });
  }
}

// ---------------------------------------------------------------------------
// stop-gate — its own fixture list, because the blocking tooth lives in the two
// *.verify states and the shared CURSOR_FIXTURES carries only one of them (and
// never feature.verify, where the cursor's feature name selects the receipt
// path). Registered in FAMILY_FIXTURES so the structural matrix floor below
// checks this family against ITS table rather than the default one.
//
// Each case is additionally crossed with three receipt states (none / fresh /
// stale) inside the test, over a REAL git repo so both sides read the same
// `git status --porcelain` bytes rather than an injected fake.
// ---------------------------------------------------------------------------

const GATE_FIXTURES = [
  ...CURSOR_FIXTURES,
  { name: 'feature.verify with a feature', cursor: { flow: 'feature', phase: 'verify', feature: 'demo', updated: FIXED_TS } },
  { name: 'feature.verify with a null feature', cursor: { flow: 'feature', phase: 'verify', feature: null, updated: FIXED_TS } },
  // inject-triggers/6 fix round 2 — the fixture that can EXECUTE the code path
  // round 1 changed. Every fixture above commits its whole tree, so the matrix
  // contained zero untracked rows and `-uall` made no observable difference in
  // it: a green that could not tell "checked and clean" from "checked nothing"
  // (this repo's Active Rule 1). See UNTRACKED_WHY below for the divergence it
  // pins.
  {
    name: 'untracked directory (engine enumerates, oracle collapses)',
    cursor: { flow: 'quick', phase: 'verify', feature: null, updated: FIXED_TS },
    untracked: true,
  },
];

const GATE_STDIN_SHAPES = [
  ['empty stdin', ''],
  ['empty object', '{}'],
  ['unparseable stdin', 'not json at all'],
  ['stop_hook_active false', '{"stop_hook_active":false}'],
  ['stop_hook_active true (boolean)', '{"stop_hook_active":true}'],
  ['stop_hook_active null', '{"stop_hook_active":null}'],
  ['unrelated keys only', '{"session_id":"abc","transcript_path":"/tmp/x"}'],
];

// 'evidence-sibling' is the discriminating case for the evidence-dir exclusion
// (js-core/8 final review, M1): receipts are FRESH relative to every ordinary
// change, and the only thing newer than the receipt under test is ANOTHER
// receipt beside it. Without the exclusion the gate calls its own receipt stale
// and a *.verify state blocks forever. Both receipts are committed in
// buildGateSandbox() so porcelain names them individually — an untracked
// evidence dir collapses to `?? .agents/skills/vibe/evidence/` and the loop only
// ever sees a directory, which is true of both sides and therefore proof of
// nothing.
const RECEIPT_STATES = ['none', 'fresh', 'stale', 'evidence-sibling'];
const STALE_TS = new Date('2000-01-01T00:00:00Z');
const FRESH_TS = new Date('2030-01-01T00:00:00Z');

// Predicate 3's line. The DIRECTION of this divergence reversed in
// inject-triggers/5: the engine no longer has a predicate 3 at all (R6 deleted
// the stuck-phase nudge — the per-turn level channel states the same thing on
// every turn), while the FROZEN oracle still fires it whenever it can resolve
// NEXT, which is to say whenever jq is present (it reads NEXT through
// detect-context.sh's jq-gated `snapshot`). So the line is now stripped from
// the ORACLE side, never the engine's, and the engine is asserted to be free of
// it BEFORE any strip happens — a strip that ran on both sides could hide a
// real difference, and one that ran unasserted could hide the deletion failing.
const PRED3_RE = /^vibe-gate: still in .*\(warn-only\)\n/m;
const PRED3_LOG_RE = /^gate: still in .*\(warn-only\)\n/m;

// ---------------------------------------------------------------------------
// PLATFORM DIALECT PROBE — does the local `sed` implement GNU BRE alternation?
//
// The oracle's no-jq leg reads the re-entry token with
// `s/..."stop_hook_active"...\(true\|false\).*/\1/p`. `\|` is a GNU extension:
// BSD sed (stock macOS) reads it as a LITERAL `|`, the expression matches
// nothing, and the oracle runs the gate on an invocation it was explicitly told
// had already fired. See the BSD-sed KNOWN DIVERGENCE pin below for the full
// direction and reasoning.
//
// Detected by CAPABILITY, never by `uname`: the property that matters is the sed
// DIALECT, not the OS — a GNU sed on macOS (Homebrew) and a BSD sed on Linux
// must each land on the correct branch. The probe resolves `sed` through this
// process's PATH, which is the same binary the no-jq leg gets: mkshim() builds
// its shim dir by symlinking `command -v sed`.
// ---------------------------------------------------------------------------
const SED_HAS_BRE_ALTERNATION = (() => {
  const probe = runCommand('bash', ['-c', "printf 'zbz' | sed -n 's/.*\\(a\\|b\\).*/\\1/p'"]);
  return probe.code === 0 && probe.stdout.trim() === 'b';
})();

// The one gate stdin shape whose re-entry token the oracle's sed leg can only
// read on a GNU sed: a bare `true`. (`false`/`null`/absent all resolve to "not a
// re-entry" on BOTH dialects, so they stay in the byte-for-byte comparison.)
const REENTRY_TRUE_SHAPE = 'stop_hook_active true (boolean)';

// ---------------------------------------------------------------------------
// KNOWN DIVERGENCE (UNTRACKED DIRECTORY) — inject-triggers/6 fix round 2.
// Direction, stated once: THE ENGINE IS RIGHT. THE FROZEN ORACLE IS WRONG.
//
// The oracle runs `git status --porcelain` with no `-uall`, so a wholly
// untracked directory arrives as ONE row — the directory. Its staleness test is
// then `[[ "$ROOT/vendor/" -nt "$receipt" ]]`, a stat of the DIRECTORY, and a
// directory's mtime does not move when a file inside it is edited in place. So
// the oracle answers "nothing newer than the receipt" for a tree that changed,
// and a `*.verify` Stop passes on a stale receipt. The engine enumerates the
// files and blocks.
//
// The engine must NEVER be "fixed" to match. This is the only blocking tooth in
// the harness; matching the oracle here would mean shipping a known way to
// bypass it by keeping work in an un-ignored untracked directory.
//
// The oracle stays BYTE-FROZEN — its evidentiary value is that it is unedited,
// so what this records is that the historical bash hook genuinely had this hole
// and the port closed it. Nothing user-facing is affected: the shipped
// stop-gate.sh is a three-line `exec node` shim.
//
// Not a skip. In the receipt states where the two disagree the ENGINE's
// behaviour is asserted in full (blocks, names the FILE not the directory), and
// the ORACLE's is asserted too (passes) — so if the oracle ever starts catching
// this, the pin fails and must be re-derived rather than rotting into a silent
// carve-out. The post-loop control additionally fails if the divergence never
// occurred at all.
const UNTRACKED_FILE_TS = new Date('2035-01-01T00:00:00Z'); // newer than FRESH_TS and SIBLING_TS
const UNTRACKED_DIR_TS = new Date('1990-01-01T00:00:00Z'); // older than STALE_TS
const UNTRACKED_FILE_REL = 'vendor/lib.sh';
// The receipt states in which the untracked FILE is the only thing newer than
// the receipt under test, so the collapsed-directory oracle misses it. In
// 'stale' (receipt at 2000) the tracked `pkg/src/deep.sh` (2025) is newer too
// and sorts FIRST, so both sides block on the identical path and the ordinary
// byte comparison runs; in 'none' both block on the missing receipt.
const ORACLE_BLIND_RECEIPTS = new Set(['fresh', 'evidence-sibling']);
const UNTRACKED_WHY =
  'ENGINE RIGHT / ORACLE WRONG: the oracle collapses a wholly-untracked directory to one row and stats the ' +
  'DIRECTORY, whose mtime does not move when a file inside it is edited in place — so it passes a stale receipt';

const BSD_SED_WHY =
  "ENGINE RIGHT / ORACLE WRONG ON THIS PLATFORM: this sed has no GNU BRE alternation, so the oracle's no-jq " +
  're-entry read (\\(true\\|false\\)) matches nothing and it runs the gate on a Stop it was told had already fired';

function buildGateSandbox(fixture) {
  const sb = makeHookSandbox({ cursor: fixture.cursor, gitInit: true });
  if (fixture.absent) rmSync(sb.cursorPath, { force: true });

  mkdirSync(path.join(sb.dir, 'pkg', 'src'), { recursive: true });
  writeFileSync(path.join(sb.dir, 'pkg', 'src', 'deep.sh'), 'v1\n');
  mkdirSync(path.join(sb.dir, 'tests'), { recursive: true });
  writeFileSync(path.join(sb.dir, 'tests', 'keep.sh'), 'v1\n');
  mkdirSync(path.join(sb.vibeDir, 'evidence'), { recursive: true });
  for (const name of RECEIPT_NAMES) {
    writeFileSync(path.join(sb.vibeDir, 'evidence', name), 'commands + output\n');
  }
  // Commit everything, so porcelain reports PER-FILE paths afterwards. An
  // uncommitted tree collapses to `?? .agents/` and the staleness loop would
  // only ever compare directory mtimes — true of both sides, and therefore
  // proof of nothing.
  runCommand('git', ['-C', sb.dir, 'add', '-A']);
  runCommand('git', ['-C', sb.dir, '-c', 'user.email=t@t', '-c', 'user.name=t', 'commit', '-qm', 'init']);
  // One tracked modification: ` M pkg/src/deep.sh`, the path the staleness loop
  // must actually stat. mtime 2025 sits between STALE_TS and FRESH_TS, so the
  // receipt state alone decides whether it reads as stale.
  writeFileSync(path.join(sb.dir, 'pkg', 'src', 'deep.sh'), 'v2\n');
  utimesSync(path.join(sb.dir, 'pkg', 'src', 'deep.sh'), STALE_TS, new Date('2025-01-01T00:00:00Z'));

  // The untracked-directory fixture: one wholly-untracked directory holding one
  // file EDITED IN PLACE long after the directory itself was last written.
  // git reports it as `?? vendor/` (oracle, no -uall) or as the file itself
  // (engine, -uall) — and the two stats disagree:
  //   directory mtime 1990  ->  older than every receipt state  ->  oracle: pass
  //   file      mtime 2035  ->  newer than every receipt state  ->  engine: block
  // Editing a file in place does not move its directory's mtime, which is
  // exactly why a collapsed row cannot answer the staleness question. The
  // directory is stamped LAST, after the file, or writing the file would move
  // it back to now.
  if (fixture.untracked) {
    const vendor = path.join(sb.dir, 'vendor');
    mkdirSync(vendor, { recursive: true });
    const lib = path.join(vendor, 'lib.sh');
    writeFileSync(lib, 'edited in place, long after the directory was created\n');
    utimesSync(lib, UNTRACKED_FILE_TS, UNTRACKED_FILE_TS);
    utimesSync(vendor, UNTRACKED_DIR_TS, UNTRACKED_DIR_TS);
  }
  return sb;
}

const RECEIPT_NAMES = ['feature-demo.md', 'quick.md'];
// Strictly newer than FRESH_TS — the sibling receipt must beat the one under
// test, or the exclusion has nothing to exclude.
const SIBLING_TS = new Date('2031-01-01T00:00:00Z');

function applyReceiptState(sb, state) {
  const evidence = path.join(sb.vibeDir, 'evidence');
  if (state === 'none') {
    rmSync(evidence, { recursive: true, force: true });
    return;
  }
  mkdirSync(evidence, { recursive: true });
  for (const name of RECEIPT_NAMES) {
    const p = path.join(evidence, name);
    writeFileSync(p, 'commands + output\n');
    const ts = state === 'stale' ? STALE_TS : FRESH_TS;
    utimesSync(p, ts, ts);
  }
  if (state === 'evidence-sibling') {
    // quick.md is rewritten (so porcelain names it) and stamped NEWER than
    // feature-demo.md. For the feature.verify fixtures the receipt under test is
    // feature-demo.md, so the only thing newer than it lives inside evidence/.
    const sibling = path.join(evidence, 'quick.md');
    writeFileSync(sibling, 'a LATER receipt, written after feature-demo.md\n');
    utimesSync(sibling, SIBLING_TS, SIBLING_TS);
  }
}

for (const fixture of GATE_FIXTURES) {
  for (const mode of JQ_MODES) {
    test(`parity matrix: hook stop-gate — ${fixture.name} x ${mode}`, () => {
      requireJqFor(mode);
      const sb = buildGateSandbox(fixture);
      const spawns = modeSpawn(mode, sb.root);
      const divergences = [];
      let pred3StrippedAtLeastOnce = false;
      let bsdSedOracleDivergedAtLeastOnce = false;
      let untrackedOracleDivergedAtLeastOnce = false;
      try {
        for (const receiptState of RECEIPT_STATES) {
          applyReceiptState(sb, receiptState);
          for (const [shapeName, stdin] of GATE_STDIN_SHAPES) {
            const label = `${shapeName} / receipt=${receiptState}`;

            resetWarnLog(sb);
            const o = runOracleScript(GATE_ORACLE, [], mode, oracleEnv(sb), { cwd: sb.root, input: stdin });
            const oracleRun = { code: o.code, stdout: o.stdout, stderr: o.stderr, log: readWarnLog(sb) };

            resetWarnLog(sb);
            const e = runGateHook(sb.root, sb.vibeDir, stdin, { spawnGit: spawns.spawnGit });
            const engineRun = { code: e.code, stdout: e.stdout, stderr: e.stderr, log: readWarnLog(sb) };

            // KNOWN DIVERGENCE (PLATFORM), full statement in the pin below this
            // loop. On a sed without GNU BRE alternation the oracle's no-jq leg
            // cannot read a bare `true` re-entry token, so it runs the gate. NOT
            // a blanket skip: the case still runs and the ENGINE's behaviour is
            // still asserted, against the fail-safe contract (return early, rc 0,
            // no output, nothing queued to the relay) instead of against a leg
            // this platform cannot execute. If the engine ever starts blocking on
            // `stop_hook_active: true`, these assertions go red on BSD sed
            // exactly as the byte comparison does on GNU sed.
            if (mode === 'no-jq' && !SED_HAS_BRE_ALTERNATION && shapeName === REENTRY_TRUE_SHAPE) {
              assertEqual(engineRun.code, 0, `${label}: ${BSD_SED_WHY}. The engine must return early — a Stop gate that blocks its own re-invocation is a block loop`);
              assertEqual(engineRun.stdout, '', `${label}: ${BSD_SED_WHY}. A re-entrant Stop must emit nothing on stdout`);
              assertEqual(engineRun.stderr, '', `${label}: ${BSD_SED_WHY}. A re-entrant Stop must emit nothing on stderr`);
              assertEqual(engineRun.log, '', `${label}: ${BSD_SED_WHY}. A re-entrant Stop must queue nothing to the warnings relay`);
              if (compareHook(label, oracleRun, engineRun) !== null) bsdSedOracleDivergedAtLeastOnce = true;
              continue;
            }

            assert(
              !PRED3_RE.test(engineRun.stderr) && !PRED3_LOG_RE.test(engineRun.log),
              `${label}: the engine emitted the stuck-phase nudge — R6 deleted predicate 3, so this strip would ` +
                'be papering over a resurrected surface instead of a declared divergence',
            );
            if (PRED3_RE.test(oracleRun.stderr) || PRED3_LOG_RE.test(oracleRun.log)) {
              pred3StrippedAtLeastOnce = true;
              oracleRun.stderr = oracleRun.stderr.replace(PRED3_RE, '');
              oracleRun.log = oracleRun.log.replace(PRED3_LOG_RE, '');
            }

            // KNOWN DIVERGENCE (UNTRACKED DIRECTORY), full statement above.
            // Deliberately placed AFTER the predicate-3 handling: the oracle
            // PASSES these cases, so it reaches predicate 3 and emits the nudge
            // R6 deleted — and that emission is what keeps the pred3
            // anti-inertness control below honest for this fixture.
            // Only where the untracked file is the ONLY thing newer than the
            // receipt, and only on a stdin shape that actually runs the gate —
            // a re-entrant Stop returns early on both sides and stays in the
            // byte comparison.
            if (fixture.untracked && ORACLE_BLIND_RECEIPTS.has(receiptState) && shapeName !== REENTRY_TRUE_SHAPE) {
              assertEqual(engineRun.code, 2, `${label}: ${UNTRACKED_WHY}. The engine must BLOCK — this is the only blocking tooth there is`);
              assert(
                engineRun.stderr.includes(UNTRACKED_FILE_REL),
                `${label}: ${UNTRACKED_WHY}. The engine must name the FILE (${UNTRACKED_FILE_REL}), not the directory; got:\n${engineRun.stderr}`,
              );
              assertEqual(
                oracleRun.code,
                0,
                `${label}: the frozen oracle is expected to MISS this. If it now blocks, this platform's git or the ` +
                  'oracle changed — re-derive the pin rather than leaving a carve-out that suppresses a real comparison',
              );
              if (compareHook(label, oracleRun, engineRun) !== null) untrackedOracleDivergedAtLeastOnce = true;
              continue;
            }

            const d = compareHook(label, oracleRun, engineRun);
            if (d) divergences.push(d);
          }
        }
        assertEqual(
          divergences,
          [],
          `stop-gate diverged from its frozen bash oracle (${fixture.name} x ${mode}):\n${divergences.join('\n')}`,
        );
        // The strip above must have been EXERCISED wherever it is applicable,
        // never silently inert: with jq the oracle fires predicate 3 for every
        // non-idle state that has legal next states, so a run in which nothing
        // was ever stripped means the oracle stopped emitting it — and then the
        // comparison is no longer proving the engine dropped anything.
        // Anti-inertness control for the untracked-directory carve-out: it must
        // have STOOD IN for a real divergence at least once, in both jq modes
        // (the divergence is about porcelain, not jq). If the two sides agreed
        // everywhere, the carve-out is suppressing nothing and must be deleted
        // so the byte comparison runs — the same bar the BSD-sed pin carries.
        if (fixture.untracked) {
          assert(
            untrackedOracleDivergedAtLeastOnce,
            `untracked-directory carve-out is INERT for ${fixture.name} x ${mode}: the oracle agreed with the ` +
              'engine on every receipt state, so it no longer stands in for a divergence — delete the carve-out ' +
              'and let the byte comparison run',
          );
        }
        if (mode === 'jq' && fixture.cursor && fixture.cursor.flow !== 'idle') {
          assert(
            pred3StrippedAtLeastOnce,
            'jq: the oracle never emitted predicate 3 for a non-idle state — nothing was stripped, so this ' +
              'matrix is no longer evidence that R6 removed anything',
          );
        }
        if (mode === 'no-jq' && fixture.cursor && fixture.cursor.flow !== 'idle') {
          // Same anti-inertness bar for the platform carve-out: on a BSD sed
          // every non-idle fixture has at least one receipt state where the
          // oracle really does warn or block on a re-entrant Stop. If none of
          // them diverged, this sed grew `\|` support (or the oracle changed)
          // and the carve-out is now silently suppressing a real comparison.
          if (!SED_HAS_BRE_ALTERNATION) {
            assert(
              bsdSedOracleDivergedAtLeastOnce,
              `BSD-sed carve-out is INERT for ${fixture.name}: the oracle agreed with the engine on every ` +
                `'${REENTRY_TRUE_SHAPE}' case, so it is no longer standing in for a divergence — delete the ` +
                'carve-out and let the byte comparison run',
            );
          }
        }
      } finally {
        sb.cleanup();
      }
    });
  }
}

// ---------------------------------------------------------------------------
// KNOWN DIVERGENCE (PLATFORM) — `stop_hook_active: true` on a sed WITHOUT GNU
// BRE alternation. Direction, stated once: THE ENGINE IS RIGHT. THE ORACLE'S
// no-jq SED LEG IS WRONG ON THIS PLATFORM.
//
// The frozen oracle reads the re-entry token without jq as
//   sed -n 's/.*"stop_hook_active"[[:space:]]*:[[:space:]]*\(true\|false\).*/\1/p'
// `\|` (alternation inside a BRE) is a GNU sed extension. BSD sed — stock on
// macOS — treats it as a LITERAL `|`, so the expression matches nothing,
// STOP_ACTIVE comes back empty, the oracle concludes "not a re-entry" and RUNS
// THE GATE: warning, or exiting 2, on the one invocation whose entire purpose is
// to not fire twice. hook.mjs parses the JSON and returns early, correctly. This
// is what turned the macOS CI leg red while ubuntu stayed green (5 of 466).
//
// The engine must NEVER be "fixed" to match: returning early on
// stop_hook_active IS the re-entry guard. Reproducing the BSD sed leg would make
// the Stop hook block its own re-invocation — a block loop and a wedged session,
// which is precisely the failure the STRING pins below already resolved in the
// other direction.
//
// The oracle stays BYTE-FROZEN. It is a verbatim historical copy and its whole
// evidentiary value is that it is unedited — so what this records is that the
// historical bash hook genuinely WAS broken on a jq-less macOS, and the port
// fixed it. Nothing user-facing is affected: the shipped
// .claude/hooks/stop-gate.sh is a three-line `exec node` shim with no sed in it.
//
// Runs on BOTH dialects, with dialect-conditional expectations — never skipped:
// the engine half of the contract is asserted everywhere, and the oracle half
// asserts the dialect really does behave as claimed, so the pin cannot rot.
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// The untracked-directory pin above only means anything while the ORACLE really
// does collapse — and that is a git CONFIG decision, not a git constant. A
// developer (or a CI image) with
//   [status]
//     showUntrackedFiles = all
// in ~/.gitconfig makes the frozen oracle enumerate too, the two sides agree,
// and the pin fails as a FALSE RED on their machine only. Observed exactly that
// way: `HOME=<fake with that option> node run.mjs` -> 659 passed, 2 failed, and
// only that fixture failed.
//
// run.mjs neutralizes it for the whole suite (GIT_CONFIG_GLOBAL/SYSTEM +
// GIT_CONFIG_NOSYSTEM). This test is what keeps that neutralization from
// rotting into a comment: it exercises the config in BOTH ambient states, and
// the hostile state is produced by DELETING the neutralization from the child
// env — not by hoping the ambient machine has the option unset, which is the
// exact shape this repo's `unsetEnv` lesson exists for.
// ---------------------------------------------------------------------------
test('hermetic: an ambient status.showUntrackedFiles cannot change what the gate parity fixtures prove', () => {
  const fixture = GATE_FIXTURES.find((f) => f.untracked);
  assert(fixture, 'floor: the untracked-directory fixture must exist for this test to guard anything');
  const sb = buildGateSandbox(fixture);
  const home = mkTempRoot('vibe-parity-home-');
  const emptyHome = mkTempRoot('vibe-parity-home-empty-');
  const NEUTRALIZERS = ['GIT_CONFIG_GLOBAL', 'GIT_CONFIG_SYSTEM', 'GIT_CONFIG_NOSYSTEM'];
  const plainPorcelain = (opts) => runCommand('git', ['-C', sb.dir, 'status', '--porcelain'], opts).stdout;
  try {
    writeFileSync(path.join(home, '.gitconfig'), '[status]\n\tshowUntrackedFiles = all\n');

    // (a) THE FAILURE MODE, reproduced on purpose: hostile config, and the
    // harness's neutralization deleted from the child environment. git
    // enumerates, so the oracle would see what the engine sees and the
    // divergence pin would go red for a reason that has nothing to do with the
    // code. This is the control that proves (b) is doing work.
    const hostile = plainPorcelain({ env: { HOME: home }, unsetEnv: NEUTRALIZERS });
    assert(
      hostile.includes(`${UNTRACKED_FILE_REL}`),
      `floor: with the neutralization removed, an ambient showUntrackedFiles=all must make plain --porcelain ` +
        `enumerate ${UNTRACKED_FILE_REL} — if it does not, this test is no longer reproducing the failure it guards:\n${hostile}`,
    );

    // (b) Same hostile config, neutralization in place (the harness default):
    // git collapses the directory again, which is the behaviour the frozen
    // oracle is pinned against.
    const neutralized = plainPorcelain({ env: { HOME: home } });
    assert(
      neutralized.includes('?? vendor/') && !neutralized.includes(UNTRACKED_FILE_REL),
      `the suite's git-config neutralization is not in effect — an ambient ~/.gitconfig is reaching the ` +
        `fixtures:\n${neutralized}`,
    );

    // (c) The other ambient state, asserted rather than assumed: an empty HOME
    // must produce the identical bytes. A neutralization that only worked when
    // something was there to neutralize would be indistinguishable from luck.
    assertEqual(
      plainPorcelain({ env: { HOME: emptyHome } }),
      neutralized,
      'a clean HOME and a hostile HOME must give the fixtures byte-identical porcelain',
    );

    // (d) And the engine is unaffected in every state — it passes `-uall`
    // explicitly, so it enumerates no matter what the config says.
    for (const [label, opts] of [
      ['hostile HOME', { env: { HOME: home } }],
      ['empty HOME', { env: { HOME: emptyHome } }],
      ['hostile HOME, neutralization deleted', { env: { HOME: home }, unsetEnv: NEUTRALIZERS }],
    ]) {
      const engineView = runCommand('git', ['-C', sb.dir, 'status', '--porcelain', '-uall'], opts).stdout;
      assert(
        engineView.includes(UNTRACKED_FILE_REL),
        `${label}: the engine's own argv must enumerate regardless of config; got:\n${engineView}`,
      );
    }
  } finally {
    rmSync(home, { recursive: true, force: true });
    rmSync(emptyHome, { recursive: true, force: true });
    sb.cleanup();
  }
});

test("KNOWN DIVERGENCE (BSD sed): stop_hook_active true — the ENGINE returns early (right); the ORACLE's no-jq sed leg misses the token because \\| is GNU-only", () => {
  const sb = buildGateSandbox({
    name: 'quick.verify',
    cursor: { flow: 'quick', phase: 'verify', feature: null, updated: FIXED_TS },
  });
  try {
    applyReceiptState(sb, 'none'); // receipt-less: the gate BLOCKS if it ever runs
    const stdin = '{"stop_hook_active":true}';
    const o = runOracleScript(GATE_ORACLE, [], 'no-jq', oracleEnv(sb), { cwd: sb.root, input: stdin });
    const e = runGateHook(sb.root, sb.vibeDir, stdin, { spawnGit: modeSpawn('no-jq', sb.root).spawnGit });

    // The ENGINE's contract, asserted on every platform — this is the fail-safe
    // behaviour and the thing that must never regress.
    assertEqual(e.code, 0, 'the engine must return early on a re-entrant Stop — a gate that blocks its own re-invocation is a block loop');
    assertEqual(e.stdout, '', 'a re-entrant Stop must emit nothing on stdout');
    assertEqual(e.stderr, '', 'a re-entrant Stop must emit nothing on stderr');

    // Discriminating control: SAME sandbox, re-entry field absent -> it really
    // does block, so the exit 0 above is the guard firing and not a fixture that
    // happens to satisfy the gate.
    const control = runGateHook(sb.root, sb.vibeDir, '{}', { spawnGit: modeSpawn('no-jq', sb.root).spawnGit });
    assertEqual(control.code, 2, 'control: without the re-entry field this fixture blocks');

    if (SED_HAS_BRE_ALTERNATION) {
      assertEqual(o.code, 0, "GNU sed: the oracle's sed leg DOES read the token, so both sides short-circuit and there is no divergence to pin here");
      assertEqual(o.stderr, '', 'GNU sed: the oracle short-circuits silently, exactly like the engine');
    } else {
      assertEqual(
        o.code,
        2,
        `BSD sed — ${BSD_SED_WHY}. The oracle falls through into the blocking tooth. If this ever reads 0, this ` +
          'platform grew `\\|` support (or the oracle was edited) and the pin should be re-derived, not deleted silently',
      );
      assertMatch(o.stderr, /BLOCKED/, 'BSD sed: the oracle blocks a Stop it was explicitly told had already fired');
    }
  } finally {
    sb.cleanup();
  }
});

// ---------------------------------------------------------------------------
// KNOWN DIVERGENCE — `stop_hook_active` as the STRING "true" (js-core/8 final
// review, M4). Deliberate, and the reason it is deliberate is that the ORACLE
// DISAGREES WITH ITSELF here: `jq -r '.stop_hook_active // false'` renders the
// string as `true` and short-circuits, while the sed leg matches only a bare
// `true`/`false` token and falls through into the blocking tooth. There is no
// single bash behaviour to be byte-identical to.
//
// The engine matches the jq leg. Not because jq is more authoritative, but
// because the two failure modes are not symmetric: reading it as "already
// re-entered" costs one turn of gate enforcement, reading it as "first entry"
// makes the Stop hook block its own re-invocation — a block loop, and a wedged
// session. The port had it the other way round, which is what the differential
// found (30 of 252 comparisons, all on this one input).
//
// Claude Code sends a JSON boolean today, so this is synthetic. It is pinned
// anyway because a platform that ever quotes the field would wedge every
// *.verify session, and because the pin states which leg was chosen and why.
// ---------------------------------------------------------------------------

const STRING_ACTIVE = '{"stop_hook_active":"true"}';

test('KNOWN DIVERGENCE: stop_hook_active as a STRING — engine matches the oracle\'s jq leg (fail-safe), not its sed leg x jq', () => {
  requireJqFor('jq');
  const sb = buildGateSandbox({ name: 'feature.verify with a feature', cursor: { flow: 'feature', phase: 'verify', feature: 'demo', updated: FIXED_TS } });
  try {
    applyReceiptState(sb, 'none'); // receipt-less: the gate would BLOCK if it ran
    const o = runOracleScript(GATE_ORACLE, [], 'jq', oracleEnv(sb), { cwd: sb.root, input: STRING_ACTIVE });
    const e = runGateHook(sb.root, sb.vibeDir, STRING_ACTIVE, { spawnGit: modeSpawn('jq', sb.root).spawnGit });
    assertEqual(o.code, 0, 'oracle sanity: the jq leg short-circuits on a string "true"');
    assertEqual(e.code, 0, 'engine matches it — never blocks its own re-invocation');
    assertEqual(e.stderr, '', 'a re-entrant Stop must emit nothing at all');

    // Discriminating control: the SAME sandbox, with the field absent, DOES
    // block — so the exit 0 above is the re-entry guard firing, not a receipt
    // that happened to satisfy the gate.
    const control = runGateHook(sb.root, sb.vibeDir, '{}', { spawnGit: modeSpawn('jq', sb.root).spawnGit });
    assertEqual(control.code, 2, 'control: without the re-entry field this fixture blocks');
  } finally {
    sb.cleanup();
  }
});

test('KNOWN DIVERGENCE: stop_hook_active as a STRING — the oracle\'s own sed leg would have blocked x no-jq', () => {
  const sb = buildGateSandbox({ name: 'feature.verify with a feature', cursor: { flow: 'feature', phase: 'verify', feature: 'demo', updated: FIXED_TS } });
  try {
    applyReceiptState(sb, 'none');
    const o = runOracleScript(GATE_ORACLE, [], 'no-jq', oracleEnv(sb), { cwd: sb.root, input: STRING_ACTIVE });
    const e = runGateHook(sb.root, sb.vibeDir, STRING_ACTIVE, { spawnGit: modeSpawn('no-jq', sb.root).spawnGit });
    assertEqual(o.code, 2, 'oracle sanity: without jq the sed leg does not recognise the quoted value and blocks');
    assertEqual(
      e.code,
      0,
      'THIS IS THE DIVERGENCE: the engine deliberately does not reproduce the sed leg here — a hook that blocks ' +
        'its own re-invocation is a block loop. If this ever equals 2, the fail-safe choice has been reverted.',
    );
  } finally {
    sb.cleanup();
  }
});

// The `cursor.flow = 5` KNOWN DIVERGENCE pin lived here until
// inject-triggers/5. It pinned oracle "Cursor: 5.impl." against engine
// "Cursor: idle.impl." — jq's `//` truthiness versus readCursor()'s
// `typeof raw.flow === 'string'` guard. R4 deleted the cursor line from BOTH
// implementations, so neither side reads a cursor here at all and the
// divergence has no surface left to appear on. The guard it documented is
// still cursor.mjs's, and cursor.test.mjs owns it directly.

// ---------------------------------------------------------------------------
// PERFORMANCE BUDGET (js-core/8 final review, I2).
//
// tech.md's Performance Budget says, in these words: "`parity.test.mjs` records
// wall time per command and fails above 150 ms". No such code existed anywhere
// under engine/tests — the spec named an assertion that did not exist, which is
// the worst kind of number to leave in a document: the next reader believes it
// is enforced. This is that assertion, in the file the spec names.
//
// WHAT IS MEASURED: the real end-to-end cost a hook pays — process spawn, module
// graph load, and the command — via the CLI, not an in-process call. An
// in-process measurement would exclude Node startup, which is most of the cost
// and the entire reason the budget exists.
//
// WHY THE MINIMUM, NOT THE MEAN OR MEDIAN: a wall-time assertion on shared CI is
// exactly the test everyone learns to re-run. Scheduler noise is strictly
// ADDITIVE — a run can be delayed, never accelerated — so min-of-N is the
// least-biased estimator of a command's true cost, and it is the only statistic
// a noisy neighbour cannot inflate. It keeps its teeth: a genuine regression
// (an added dependency, a synchronous scan, a second machine parse) raises the
// floor along with everything else. The median is RECORDED alongside it, so a
// broadly-slower run is visible in the log even though it does not fail the
// build.
//
// WHY THAT IS NOT ENOUGH, AND WHAT THE CONTROL IS FOR: min-of-N only survives
// noise that some runs escape. Self-attacked by oversubscribing this machine 3x
// (12 busy loops on 4 cores), EVERY run is contended and the floor itself
// inflates — measured: `doctrine` min 155 ms, and the assertion goes red for a
// reason no one can act on. So the run is CALIBRATED: a bare `node -e 0` spawn,
// which the engine must pay before executing a single line of vibe code, is
// timed the same way. Measured here: ~27 ms idle, ~63-71 ms under that same 3x
// load. When the control alone eats more than 40% of the whole budget the runner
// is not delivering usable timings, and the test SKIPS with the number rather
// than failing — a visible "could not measure", never a silent pass. Expressing
// the control ceiling as a fraction of the documented ceiling keeps 150 the only
// number anyone has to maintain.
//
// The measured floor on the development machine is ~48 ms for orders (bash
// oracle ~18 ms), so the 150 ms ceiling carries roughly 3x headroom. Note
// separately that tech.md's aspirational "under 50 ms cold for orders" TARGET is
// met only at the minimum, not at the median — a spec-drift item for compound,
// deliberately NOT resolved here by relaxing the documented number.
// ---------------------------------------------------------------------------

const PERF_CEILING_MS = 150; // tech.md, Performance Budget — "fails above 150 ms"
const PERF_RUNS = 9;
// Above this, a bare interpreter start dominates the budget and the measurement
// describes the runner rather than the engine.
const PERF_CONTROL_CEILING_MS = PERF_CEILING_MS * 0.4;

function minOf(fn) {
  const samples = [];
  for (let i = 0; i < PERF_RUNS; i += 1) {
    const t0 = process.hrtime.bigint();
    fn();
    samples.push(Number(process.hrtime.bigint() - t0) / 1e6);
  }
  samples.sort((a, b) => a - b);
  return samples;
}

const PERF_COMMANDS = [
  ['orders', ['orders']],
  ['doctrine', ['doctrine']],
  ['state get', ['state', 'get']],
  ['doctor', ['doctor']],
];

test('performance budget: every ported command stays under tech.md\'s 150 ms wall-time ceiling', () => {
  const control = minOf(() => runCommand(process.execPath, ['-e', '0']));
  const controlMin = control[0];

  const report = [];
  const over = [];

  for (const [name, argv] of PERF_COMMANDS) {
    let lastResult;
    const samples = minOf(() => {
      lastResult = runCli(argv);
      // A command that CRASHES is fast. Without this the budget would happily
      // certify a broken engine as well within its performance envelope.
      assertEqual(lastResult.code, 0, `\`vibe ${name}\` exited ${lastResult.code} while being timed: ${lastResult.stderr}`);
      assert(lastResult.stdout.length > 0, `\`vibe ${name}\` produced no output while being timed — nothing was measured`);
    });
    const min = samples[0];
    const median = samples[(samples.length - 1) >> 1];
    const max = samples[samples.length - 1];
    report.push(`${name}: min=${min.toFixed(1)}ms median=${median.toFixed(1)}ms max=${max.toFixed(1)}ms (n=${PERF_RUNS})`);
    if (min > PERF_CEILING_MS) {
      over.push(`${name}: ${min.toFixed(1)}ms > ${PERF_CEILING_MS}ms (median ${median.toFixed(1)}ms, max ${max.toFixed(1)}ms)`);
    }
  }

  // "records wall time per command" — the numbers go to the log on every run,
  // pass, fail or skip, so a slow trend is observable before it becomes a
  // failure. The control is recorded too: it is what makes a number in this log
  // comparable across machines.
  console.log(
    `        wall time: ${report.join(' | ')} || control (bare node -e 0): min=${controlMin.toFixed(1)}ms`,
  );

  // Calibration gate, AFTER the measurements so the numbers are always logged.
  // MUST be skip(), never a bare return: a return reports `ok` and is
  // indistinguishable from a budget that was actually checked.
  if (controlMin > PERF_CONTROL_CEILING_MS) {
    skip(
      `runner is contended — a bare \`node -e 0\` spawn alone costs ${controlMin.toFixed(1)}ms, over ` +
        `${PERF_CONTROL_CEILING_MS.toFixed(0)}ms (40% of the whole ${PERF_CEILING_MS}ms budget). ` +
        'These timings describe the machine, not the engine; measured above and recorded, not asserted on',
    );
  }

  assertEqual(
    over,
    [],
    'a ported command is over tech.md\'s Performance Budget at its FASTEST of ' +
      `${PERF_RUNS} runs, on a runner whose bare interpreter start measured ${controlMin.toFixed(1)}ms — ` +
      `machine noise cannot explain this, it is a real regression:\n${over.join('\n')}`,
  );
});

// ---------------------------------------------------------------------------
// The matrix's own SHAPE (js-core/8 fix round 3; re-review round 2, Minor 2).
//
// runner.test.mjs asserts the jq-leg population is non-empty and that the two
// halves are the same size. Both are satisfied by a matrix that collapsed from
// 5 fixtures to 1 — and `jqHalfVerdict({total:1, executed:1})` is `ok`, so the
// gate above it would report a clean sweep of a matrix that no longer exists.
// "Adding legs widens the floor on its own" was not true: only removing the
// LAST leg was caught.
//
// This is the floor that says what it means, derived from the tables the matrix
// actually iterates (CURSOR_FIXTURES x JQ_MODES) rather than from a number
// anyone has to remember to raise. Every family that registers at all must
// cover every fixture in every mode.
// ---------------------------------------------------------------------------

const MATRIX_NAME_RE = /^parity matrix: (.+) — (.+) x (jq|no-jq)$/;

// A family may iterate its OWN fixture table when the shared one cannot express
// what it needs to cover — stop-gate's blocking tooth lives in the two *.verify
// states, and CURSOR_FIXTURES carries only one of them and never a
// feature.verify with a feature name to resolve the receipt path from. The
// override is declared here, so "this family iterates a different table" stays a
// checked claim rather than an unexplained set of extra cells.
const FAMILY_FIXTURES = new Map([['hook stop-gate', GATE_FIXTURES]]);

test('parity matrix: every family covers every cursor fixture in BOTH jq modes', () => {
  const families = new Map(); // family -> Set('<fixture> x <mode>')
  for (const t of registeredTests()) {
    const m = MATRIX_NAME_RE.exec(t.name);
    if (!m) continue;
    if (!families.has(m[1])) families.set(m[1], new Set());
    families.get(m[1]).add(`${m[2]} x ${m[3]}`);
  }

  assert(
    families.size > 0,
    `no registered test name matched ${MATRIX_NAME_RE} — the parity matrix has been renamed or removed, ` +
      'which disarms the jq-half gate in run.mjs; update this pattern to the new convention',
  );

  const cellsFor = (fixtures) => {
    const cells = [];
    for (const fixture of fixtures) {
      for (const mode of JQ_MODES) cells.push(`${fixture.name} x ${mode}`);
    }
    return cells;
  };

  const gaps = [];
  for (const [family, seen] of [...families].sort()) {
    const fixtures = FAMILY_FIXTURES.get(family) ?? CURSOR_FIXTURES;
    const expected = cellsFor(fixtures);
    for (const cell of expected) if (!seen.has(cell)) gaps.push(`${family}: missing '${cell}'`);
    for (const cell of seen) if (!expected.includes(cell)) gaps.push(`${family}: unexpected '${cell}'`);
    // An override may only WIDEN the shared table, never narrow it: otherwise
    // FAMILY_FIXTURES becomes a way to shrink a family's coverage while this
    // floor reports it complete.
    for (const shared of CURSOR_FIXTURES) {
      if (!fixtures.some((f) => f.name === shared.name)) {
        gaps.push(`${family}: its FAMILY_FIXTURES override drops the shared fixture '${shared.name}'`);
      }
    }
  }
  assertEqual(
    gaps,
    [],
    'the parity matrix is (that family\'s fixtures) x JQ_MODES per family — a family that stopped generating a cell ' +
      'shrinks the population the jq-half gate counts without emptying it, which is invisible to a ' +
      `non-empty floor:\n${gaps.join('\n')}`,
  );

  // The two BLOCKING hooks must be IN the matrix. They were the whole gap the
  // final review found: every other family could be present and green while
  // guard/gate — the only two commands that can exit 2 — had no oracle at all.
  for (const required of ['hook pre-tool-use-guard', 'hook stop-gate']) {
    assert(
      families.has(required),
      `'${required}' is not in the parity matrix — the two hooks that can BLOCK a tool call or a turn must ` +
        'always be compared against their frozen bash oracles (flow/hooks-fallback/), never trusted from a port review',
    );
  }

  // And the population is a real matrix, not a single surviving cell.
  assert(
    CURSOR_FIXTURES.length >= 2 && GATE_FIXTURES.length > CURSOR_FIXTURES.length && JQ_MODES.length === 2,
    `the matrix collapsed: ${CURSOR_FIXTURES.length} shared fixture(s), ${GATE_FIXTURES.length} gate fixture(s) x ${JQ_MODES.length} mode(s)`,
  );
});

// The oracle differential's own INPUT TABLES (self-attack on this file).
//
// Every table below can be trimmed to a single entry while the test NAMES, the
// registered count, the per-family fixture floor above and the jq-half gate all
// stay exactly the same — the differential keeps reporting a clean sweep of a
// surface that no longer exists. That is the same failure mode fix round 3 found
// in the fixture matrix, one level down in the inputs.
//
// Deliberately a TEST, not a module-scope assert: a floor that throws at import
// time crashes the runner (`test runner crashed: ...`, exit 1) instead of
// reporting a named failure among its peers. Loud either way, but only one of
// them tells you which floor broke without reading a stack trace.
test('parity oracles: the differential\'s input tables have not been trimmed to a token entry', () => {
  const thin = [];
  const floors = [
    ['GUARD_STDIN_SHAPES', GUARD_STDIN_SHAPES.length, 20],
    ['GATE_STDIN_SHAPES', GATE_STDIN_SHAPES.length, 7],
    ['RECEIPT_STATES', RECEIPT_STATES.length, 4],
    ['GATE_FIXTURES', GATE_FIXTURES.length, 7],
  ];
  for (const [name, actual, floor] of floors) {
    if (actual < floor) thin.push(`${name}: ${actual} entr(ies), floor ${floor}`);
  }
  assertEqual(
    thin,
    [],
    'an input table of the guard/gate oracle differential has shrunk. The comparison still runs and still ' +
      `passes — over almost nothing:\n${thin.join('\n')}`,
  );

  // The receipt states are named, not just counted: the two BLOCKING outcomes
  // and the M1 discriminating case are what the tooth IS, and a same-sized table
  // of four harmless states would satisfy a bare length floor.
  assertEqual(
    [...RECEIPT_STATES].sort(),
    ['evidence-sibling', 'fresh', 'none', 'stale'],
    'the gate differential must keep all four receipt states: missing (block), fresh (pass), stale (block), ' +
      'and a newer SIBLING receipt under evidence/ (must NOT block)',
  );

  // Likewise for the stdin shapes that decide whether the gate runs at all.
  const gateShapeNames = GATE_STDIN_SHAPES.map(([n]) => n);
  // REENTRY_TRUE_SHAPE by reference, not by re-spelling: the BSD-sed carve-out
  // above selects its case by matching that exact name, so a rename that missed
  // one of the two would silently disarm the carve-out AND this floor together.
  for (const required of ['empty stdin', 'unparseable stdin', REENTRY_TRUE_SHAPE]) {
    assert(
      gateShapeNames.includes(required),
      `the gate differential lost its '${required}' shape — that is a re-entry/degrade path, not a filler case`,
    );
  }
});

// The frozen oracles must actually BE the pre-port bash hooks, not a
// convenience rewrite that drifted toward the port it is supposed to judge.
// Structural, since their provenance (git `main`) will not be reachable forever:
// each must carry the FROZEN banner, must still contain the load-bearing
// spellings the port had to reproduce, and must contain no trace of the Node
// shim that replaced it.
test('parity oracles: flow/hooks-fallback/*.sh are the frozen bash originals, not shims', () => {
  const guard = readFileSync(GUARD_ORACLE, 'utf8');
  const gate = readFileSync(GATE_ORACLE, 'utf8');
  for (const [name, src] of [['pre-tool-use-guard.sh', guard], ['stop-gate.sh', gate]]) {
    assertMatch(src, /FROZEN PARITY ORACLE/, `${name} lost its frozen-oracle banner`);
    assert(!/cli\.mjs/.test(src), `${name} has been overwritten with the Node shim — the oracle is gone again`);
    assert(!/exec node/.test(src), `${name} has been overwritten with the Node shim — the oracle is gone again`);
  }
  assertMatch(guard, /detect-context\.sh|"\$DETECT" decide/, 'the guard oracle no longer delegates to detect-context.sh');
  assertMatch(guard, /sniff_bash/, 'the guard oracle lost its warn-only Bash sniffer');
  assertMatch(gate, /stop_hook_active/, 'the gate oracle lost its re-entry guard');
  assertMatch(gate, /-nt "\$receipt"/, 'the gate oracle lost its receipt staleness comparison');
});
