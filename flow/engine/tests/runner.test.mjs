// engine/tests/runner.test.mjs — the runner must fail loud, not report green
// on a broken selection: a filter matching zero tests, or discovery finding
// zero test files at all, must both exit non-zero with a stated reason.

import { mkdtempSync, mkdirSync, copyFileSync, writeFileSync, symlinkSync, rmSync, readFileSync, readdirSync } from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import {
  test,
  mkTempRoot,
  assert,
  assertEqual,
  assertIncludes,
  runCommand,
  jqHalfVerdict,
  JQ_LEG_NAME_RE,
  registeredTests,
} from './run.mjs';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const RUN_MJS = path.join(__dirname, 'run.mjs');

test('a filter matching zero tests exits non-zero and says so', () => {
  const result = runCommand(process.execPath, [RUN_MJS, 'zzz-no-such-test-xyz']);
  assert(result.code !== 0, `expected non-zero exit, got ${result.code}`);
  const combined = result.stdout + result.stderr;
  assertIncludes(combined, 'zzz-no-such-test-xyz', 'should name the filter that matched nothing');
});

test('zero discovered test files exits non-zero and says so', () => {
  // Isolate run.mjs in an empty directory so its own discovery (readdirSync
  // of its own dirname for *.test.mjs) finds nothing to import.
  const dir = mkdtempSync(path.join(tmpdir(), 'vibe-runner-empty-'));
  try {
    const copy = path.join(dir, 'run.mjs');
    copyFileSync(RUN_MJS, copy);
    const result = runCommand(process.execPath, [copy]);
    assert(result.code !== 0, `expected non-zero exit, got ${result.code}`);
    const combined = result.stdout + result.stderr;
    assertIncludes(combined, 'no test files', 'should state that no test files were found');
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

// js-core/8 fix round 1: Node's ESM loader realpaths `import.meta.url`, so the
// main-module guard's `__filename` is always the resolved path — while
// `path.resolve(process.argv[1])` normalizes `.`/`..` but NEVER resolves
// symlinks. Spawned through any path containing a symlink the two differed,
// `main()` never ran, and the whole suite exited 0 with no output: the
// anti-vacuous-green tooth, itself vacuously green. Real targets hit this —
// `.agents/skills/vibe` is a symlink to `flow/`, and on macOS every
// `mkdtempSync` path goes through /var -> private/var.
test('the runner still runs when spawned through a symlinked path', () => {
  const dir = mkdtempSync(path.join(tmpdir(), 'vibe-runner-symlink-'));
  try {
    const real = path.join(dir, 'real');
    mkdirSync(real);
    copyFileSync(RUN_MJS, path.join(real, 'run.mjs'));
    writeFileSync(
      path.join(real, 'sentinel.test.mjs'),
      "import { test, assertEqual } from './run.mjs';\n" +
        "test('sentinel executes', () => { assertEqual(1, 1); });\n",
    );
    symlinkSync('real', path.join(dir, 'link'));

    const result = runCommand(process.execPath, [path.join(dir, 'link', 'run.mjs')]);
    const combined = result.stdout + result.stderr;
    assertIncludes(combined, '1 passed, 0 failed', 'the runner must actually execute the discovered test');
    assertEqual(result.code, 0, `expected exit 0, got ${result.code}; output: ${combined}`);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test('normal run with matching tests still exits 0', () => {
  const result = runCommand(process.execPath, [RUN_MJS, 'harness.test.mjs']);
  assertEqual(result.code, 0, `expected exit 0, got ${result.code}; stderr: ${result.stderr}`);
});

// run.mjs's mkTempRoot() comment says a raw mkdtempSync left beside the helper
// "is how the next macOS failure lands" (/var vs /private/var), and js-core/8
// fix round 1 proved it: one half of the fix swept 36 sites onto the helper
// while the other half added fresh raw ones in files it never touched, because
// nothing enforced it. This is that enforcement.
//
// Exact counts, not an upper bound: an errored high pin is invisible under
// `<=`, and the repo has already been bitten by hand-written counts rotting
// silently. Cleaning a file up means lowering its number here; a file that
// reaches 0 comes out of the table. New raw sites anywhere — including in a
// new test file — fail immediately.
const RAW_MKDTEMP_BUDGET = {
  'run.mjs': 1, // mkTempRoot()'s own definition — the one sanctioned call
  'doctrine.test.mjs': 11,
  'orders.test.mjs': 4,
  'state.test.mjs': 4,
  'runner.test.mjs': 2,
  // hook.test.mjs reached 0 and came out of the table (js-core/8 final review,
  // I3): its makeHookSandbox() moved into run.mjs's mkTempRoot()-based helper so
  // parity.test.mjs's guard/gate oracle differential builds the same layout.
  'machine.test.mjs': 1,
};

test('no raw mkdtempSync beyond the pinned legacy budget (mkTempRoot is the temp-root helper)', () => {
  const observed = {};
  for (const name of readdirSync(__dirname).sort()) {
    if (!name.endsWith('.mjs')) continue;
    const matches = readFileSync(path.join(__dirname, name), 'utf8').match(/mkdtempSync\(/g);
    if (matches) observed[name] = matches.length;
  }
  assertEqual(
    observed,
    RAW_MKDTEMP_BUDGET,
    'raw mkdtempSync sites moved. Use mkTempRoot(prefix) from run.mjs — it realpaths the result, which is what keeps macOS (/var -> /private/var) green. If you removed sites, lower the pinned counts to match.',
  );
});

// ---------------------------------------------------------------------------
// js-core/8 fix round 2 (re-review Finding 5) — the jq half of the parity
// matrix must be PROVEN to have run, not merely reported honestly.
//
// Round 1 converted 31 silently-degenerate jq tests into honest skips. That
// fixed the lying counts but left no gate: with jq absent the suite prints
// `289 passed, 0 failed, 44 skipped` and exits 0. The ubuntu CI leg has no
// `Ensure jq` step, so if the runner image ever dropped preinstalled jq, all
// 31 would skip, both legs would stay green, and the entire jq half of R2's
// "ported command output matches its bash original" would stop running with
// nothing to say so.
//
// The gate must distinguish DELIBERATELY STRIPPED (CI's own jq-less leg, which
// exists precisely to exercise the degrade paths and must keep skipping) from
// UNEXPECTEDLY MISSING. Only an explicit opt-in can carry that intent, so
// VIBE_NO_JQ=1 is the signal — and it is bidirectional: if the strip is set but
// the jq legs ran anyway, that leg is not stripping jq and its own half is the
// one silently not running.
// ---------------------------------------------------------------------------

test('jq gate: the jq half running normally is a pass', () => {
  assertEqual(jqHalfVerdict({ total: 30, executed: 30, noJqOptIn: false }).ok, true);
});

test('jq gate: every jq-leg test skipped without the opt-in fails, naming the fix', () => {
  const verdict = jqHalfVerdict({ total: 30, executed: 0, noJqOptIn: false });
  assertEqual(verdict.ok, false, 'a skip-everything jq half must not read as a clean sweep');
  assertIncludes(verdict.reason, '0 of 30');
  assertIncludes(verdict.reason, 'VIBE_NO_JQ=1', 'the reason must name the deliberate-strip opt-in');
});

test('jq gate: the deliberately stripped leg is allowed to skip the whole jq half', () => {
  assertEqual(jqHalfVerdict({ total: 30, executed: 0, noJqOptIn: true }).ok, true);
});

test('jq gate: a stripped leg where jq legs still ran is a failed strip', () => {
  const verdict = jqHalfVerdict({ total: 30, executed: 30, noJqOptIn: true });
  assertEqual(verdict.ok, false, 'VIBE_NO_JQ=1 with jq still reachable means the no-jq half never ran');
  assertIncludes(verdict.reason, 'still reachable');
});

test('jq gate: a jq half that has vanished entirely fails, not passes', () => {
  const verdict = jqHalfVerdict({ total: 0, executed: 0, noJqOptIn: false });
  assertEqual(verdict.ok, false, 'zero discovered jq-leg tests is the vacuous state, not a clean one');
  assertIncludes(verdict.reason, String(JQ_LEG_NAME_RE), 'the reason must name the convention it looked for');
});

test('jq gate: a jq half that has vanished fails even on the stripped leg', () => {
  assertEqual(jqHalfVerdict({ total: 0, executed: 0, noJqOptIn: true }).ok, false);
});

// Structural floor, derived from the live registry rather than pinned as a
// number: the population the gate counts must actually exist under the naming
// convention the gate matches. Renaming the matrix legs breaks this loudly
// here instead of quietly disarming the gate, and adding legs widens the floor
// on its own (.spec/lessons.md: hand-written counts rot silently).
test('jq gate: the jq-leg population it counts is non-empty in the real registry', () => {
  const jqLeg = registeredTests().filter((t) => JQ_LEG_NAME_RE.test(t.name));
  assert(
    jqLeg.length > 0,
    `no registered test name matches ${JQ_LEG_NAME_RE} — the parity matrix's jq legs were renamed or removed, ` +
      'which silently disarms the jq-half gate in run.mjs; update JQ_LEG_NAME_RE to the new convention',
  );
  const noJqLeg = registeredTests().filter((t) => / x no-jq$/.test(t.name));
  assertEqual(
    jqLeg.length,
    noJqLeg.length,
    'the matrix is jq x no-jq — the two halves must be the same size, or one of them is not being generated',
  );
});

// The six tests above pin the VERDICT; this one pins the WIRING. Deleting the
// gate's call site in main() while leaving jqHalfVerdict() intact leaves all of
// them green and the gate dead — a pure function nobody consults. Proving
// otherwise needs a real spawned run whose jq half skips, which a synthetic
// tests directory gives us without touching this runner's own PATH: run.mjs
// beside a `parity.test.mjs` (the gate only speaks for a discovered set that
// CARRIES the matrix) holding one skipping ` x jq` leg and its no-jq twin.
//
// PATH isolation is not enough, and the first version of this test proved it by
// failing on CI's own jq-stripped leg: that leg runs the whole suite under
// VIBE_NO_JQ=1, runCommand's base env is process.env, so the "bare" spawn
// inherited the opt-in, took the deliberately-skipped branch and exited 0. A
// control case for "the variable is unset" has to DELETE it (unsetEnv), and the
// deliberately contaminated parent below is what keeps that true on every leg
// rather than only on the legs whose environment happened to be clean.
test('jq gate: the gate is wired into main() — a spawned run with a skipped jq half exits non-zero', () => {
  const dir = mkTempRoot('vibe-runner-jqgate-');
  const prevOptIn = process.env.VIBE_NO_JQ;
  try {
    copyFileSync(RUN_MJS, path.join(dir, 'run.mjs'));
    writeFileSync(
      path.join(dir, 'parity.test.mjs'),
      "import { test, skip, assertEqual } from './run.mjs';\n" +
        "test('synthetic matrix: probe x jq', () => { skip('synthetic: jq unavailable'); });\n" +
        "test('synthetic matrix: probe x no-jq', () => { assertEqual(1, 1); });\n",
    );
    const runMjs = path.join(dir, 'run.mjs');
    const spawnBare = () => runCommand(process.execPath, [runMjs], { unsetEnv: ['VIBE_NO_JQ'] });

    const assertBareFails = (result, when) => {
      const out = result.stdout + result.stderr;
      assert(
        result.code !== 0,
        `a run whose whole jq half skipped must fail (${when}), got exit ${result.code}; output: ${out}`,
      );
      assertIncludes(out, 'jq-half gate', `the failing run must name the gate that failed it (${when})`);
      assertIncludes(out, '0 of 1', `the gate must report the population it counted (${when})`);
      assert(
        !out.includes('deliberately skipped'),
        `the bare spawn must not inherit the opt-in (${when}) — that is the branch this case exists to exclude; output: ${out}`,
      );
    };

    // As the runner found it — whatever this leg's environment happens to be.
    delete process.env.VIBE_NO_JQ;
    assertBareFails(spawnBare(), 'parent clean');

    // And with the parent DELIBERATELY contaminated, which is CI's jq-stripped
    // leg exactly. Without unsetEnv this spawn exits 0 and the case is vacuous.
    process.env.VIBE_NO_JQ = '1';
    assertBareFails(spawnBare(), 'parent carries VIBE_NO_JQ=1');

    // Same run, opt-in declared: the deliberate strip stays green, so the gate
    // is a gate and not merely a way to fail every jq-less environment.
    delete process.env.VIBE_NO_JQ;
    const optIn = runCommand(process.execPath, [runMjs], { env: { VIBE_NO_JQ: '1' } });
    assertEqual(
      optIn.code,
      0,
      `VIBE_NO_JQ=1 must let a deliberately skipped jq half pass; output: ${optIn.stdout}${optIn.stderr}`,
    );
  } finally {
    if (prevOptIn === undefined) delete process.env.VIBE_NO_JQ;
    else process.env.VIBE_NO_JQ = prevOptIn;
    rmSync(dir, { recursive: true, force: true });
  }
});

// js-core/8 fix round 3 (re-review round 2, Important 3). The gate's
// applicability used to be `path.basename(f) === 'parity.test.mjs'` — a
// hand-written filename referenced nowhere else and asserted by nothing, so
// splitting or renaming the matrix file switched the entire gate off while both
// CI legs stayed green. These two cases pin the derivation that replaced it,
// from the outside, by spawning real runs.
test('jq gate: renaming the matrix FILE does not disarm the gate (applicability is derived, not named)', () => {
  const dir = mkTempRoot('vibe-runner-jqrename-');
  try {
    copyFileSync(RUN_MJS, path.join(dir, 'run.mjs'));
    // Deliberately NOT called parity.test.mjs — that name is what used to be
    // load-bearing. Same legs, skipping jq half, so the gate must still fire.
    writeFileSync(
      path.join(dir, 'parity-matrix-split.test.mjs'),
      "import { test, skip, assertEqual } from './run.mjs';\n" +
        "test('synthetic matrix: probe x jq', () => { skip('synthetic: jq unavailable'); });\n" +
        "test('synthetic matrix: probe x no-jq', () => { assertEqual(1, 1); });\n",
    );
    const r = runCommand(process.execPath, [path.join(dir, 'run.mjs')], { unsetEnv: ['VIBE_NO_JQ'] });
    const out = r.stdout + r.stderr;
    assert(r.code !== 0, `the gate must still fire from a renamed matrix file; output: ${out}`);
    assertIncludes(out, 'jq-half gate', 'the failing run must name the gate');
    assertIncludes(out, '0 of 1', 'the gate must report the population it counted');
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test('jq gate: renaming the jq LEGS fails loudly instead of silently disarming', () => {
  // The residual disarm the derivation could have introduced: if applicability
  // were "some ` x jq` test registered", renaming those legs would make the
  // gate conclude there is no matrix. Applicability is EITHER half, so a
  // surviving no-jq half keeps the gate live and it reports a zero population.
  const dir = mkTempRoot('vibe-runner-jqlegrename-');
  try {
    copyFileSync(RUN_MJS, path.join(dir, 'run.mjs'));
    writeFileSync(
      path.join(dir, 'parity.test.mjs'),
      "import { test, assertEqual } from './run.mjs';\n" +
        "test('synthetic matrix: probe x jqq', () => { assertEqual(1, 1); });\n" +
        "test('synthetic matrix: probe x no-jq', () => { assertEqual(1, 1); });\n",
    );
    const r = runCommand(process.execPath, [path.join(dir, 'run.mjs')], { unsetEnv: ['VIBE_NO_JQ'] });
    const out = r.stdout + r.stderr;
    assert(r.code !== 0, `a renamed jq half must fail the run, not disarm the gate; output: ${out}`);
    assertIncludes(out, 'jq-half gate', 'the failing run must name the gate');
    assertIncludes(out, 'renamed or removed', 'the reason must say the convention stopped matching');
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

// The helper the case above leans on, pinned in its own right — the sweep found
// eight other node-spawn sites with no env argument at all, so the next control
// case that needs a variable absent will reach for this and must be able to
// trust it. Both directions: inherited when nothing is said, gone when it is.
test('runCommand: unsetEnv deletes a variable the child would otherwise inherit', () => {
  const READ_BACK = ['-e', 'process.stdout.write(String(process.env.VIBE_UNSETENV_PROBE))'];
  const prev = process.env.VIBE_UNSETENV_PROBE;
  process.env.VIBE_UNSETENV_PROBE = 'inherited';
  try {
    assertEqual(
      runCommand(process.execPath, READ_BACK).stdout,
      'inherited',
      'baseline: a child inherits process.env, so "not passed in opts.env" does NOT mean "absent"',
    );
    assertEqual(
      runCommand(process.execPath, READ_BACK, { unsetEnv: ['VIBE_UNSETENV_PROBE'] }).stdout,
      'undefined',
      'unsetEnv must delete the variable from the child, not merely decline to set it',
    );
    assertEqual(
      runCommand(process.execPath, READ_BACK, { env: { VIBE_UNSETENV_PROBE: 'explicit' } }).stdout,
      'explicit',
      'unsetEnv must not disturb a value the caller set deliberately',
    );
    // The INTERACTION, which is what the comment above runCommand actually
    // claims and what nothing tested (js-core/8 fix round 2 re-review, Minor 1).
    // Deleting after the merge made this return 'undefined' — the documented
    // contract inverted, and the assertion right above it could not tell,
    // because it passes `env` with no `unsetEnv` at all.
    assertEqual(
      runCommand(process.execPath, READ_BACK, {
        env: { VIBE_UNSETENV_PROBE: 'explicit' },
        unsetEnv: ['VIBE_UNSETENV_PROBE'],
      }).stdout,
      'explicit',
      'unsetEnv is a do-not-INHERIT list applied to the base env; an explicit opts.env value still wins',
    );
  } finally {
    if (prev === undefined) delete process.env.VIBE_UNSETENV_PROBE;
    else process.env.VIBE_UNSETENV_PROBE = prev;
  }
});
