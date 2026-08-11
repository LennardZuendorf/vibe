// engine/tests/orders.test.mjs — engine/commands/orders.mjs (js-core/4, R2).
//
// The oracle is flow/scripts/orders.sh. Never hand-copy its expected output:
// every parity assertion spawns a COPY of the real script (byte-identical,
// via copyFileSync) inside a throwaway sandbox laid out as
// <sandbox>/flow/{scripts/orders.sh, state.json, state-machine.json} plus
// <sandbox>/.agents/skills/vibe/SKILL.md and a <sandbox>/.spec marker — the
// exact SCRIPT_DIR/SKILL_DIR-relative + marker-search layout the script
// resolves against (see orders.sh's own SKILLS_DIR comment) — so its
// self-resolution runs for real, against sandbox files, and never touches
// this repo's live flow/state.json.
//
// Adversarial, not just happy paths (per the task brief): all 13 states, a
// missing skill file, a missing AND a misspelled closing marker (the
// intentional extractBlock divergence — pinned here, not just asserted in
// blocks.test.mjs), an escaping-sensitive feature name, an absent cursor, a
// corrupt cursor, and a fresh non-git install-layout CLI run.

import {
  mkdtempSync,
  mkdirSync,
  writeFileSync,
  readFileSync,
  copyFileSync,
  rmSync,
  cpSync,
} from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { test, assert, assertEqual, assertMatch, makeSandbox, runCommand } from './run.mjs';
import { runOrders } from '../commands/orders.mjs';
import { loadMachine } from '../machine.mjs';

const REPO_ROOT = path.resolve(path.dirname(new URL(import.meta.url).pathname), '..', '..');
const ORACLE_SRC = path.join(REPO_ROOT, 'flow', 'scripts', 'orders.sh');
const REAL_SKILL_MD = readFileSync(path.join(REPO_ROOT, 'flow', 'SKILL.md'), 'utf8');
const GENERIC_FALLBACK =
  'state=unknown · read .agents/skills/vibe/state-machine.json and pick the matching vibe phase · transition via set-state.sh';

// Whether this runner has jq. orders.sh's no-jq degrade path is DESIGNED to
// be byte-identical to the jq path (see flow/tests/run.sh's own no-jq parity
// checks) for well-formed cursors — but it gets there via documented
// shortcuts (hardcoded "vibe" skill, sed-grabbed cursor fields) that the
// task brief says NOT to copy into the port because they are not general.
// The escaping-sensitive-feature fixture below is exactly where a sed-based
// no-jq cursor read can diverge from a real JSON parse; guard that one
// comparison behind JQ_PRESENT so the suite still passes with jq stripped
// from PATH (unit 8's no-jq CI leg), without silently asserting nothing.
const JQ_PRESENT = runCommand('bash', ['-c', 'command -v jq >/dev/null 2>&1']).code === 0;

// ---------------------------------------------------------------------------
// Oracle sandbox — a copy of orders.sh plus its own flow/ cursor+machine and
// .agents/skills/vibe/SKILL.md, never the real ones.
// ---------------------------------------------------------------------------

function makeOrdersSandbox({ cursor, skillMd = REAL_SKILL_MD, includeSkillFile = true } = {}) {
  const sandbox = makeSandbox({ cursor });
  const scriptsDir = path.join(sandbox.flowDir, 'scripts');
  mkdirSync(scriptsDir, { recursive: true });
  const scriptPath = path.join(scriptsDir, 'orders.sh');
  copyFileSync(ORACLE_SRC, scriptPath);

  const skillsDir = path.join(sandbox.dir, '.agents', 'skills');
  const vibeSkillDir = path.join(skillsDir, 'vibe');
  mkdirSync(vibeSkillDir, { recursive: true });
  if (includeSkillFile) {
    writeFileSync(path.join(vibeSkillDir, 'SKILL.md'), skillMd);
  }

  return { ...sandbox, scriptPath, skillsDir, vibeSkillDir };
}

function runOracle(scriptPath, args = []) {
  return runCommand('bash', [scriptPath, ...args]);
}

function runEngine(sandbox, args = []) {
  return runOrders(sandbox.flowDir, sandbox.skillsDir, args);
}

// ---------------------------------------------------------------------------
// Parity matrix — all 13 states, explicit-arg form. Cursor carries a feature
// so every skill-owning block's <feature> interpolation is exercised too.
// ---------------------------------------------------------------------------

test('parity: all 13 machine states resolve byte-identical orders (explicit arg)', () => {
  const sandbox = makeOrdersSandbox({
    cursor: { flow: 'idle', phase: 'idle', feature: 'widget', updated: '2026-01-01T00:00:00Z' },
  });
  try {
    const machine = loadMachine(sandbox.flowDir);
    const states = Object.keys(machine.states);
    assertEqual(states.length, 13, `expected 13 machine states, got ${states.length}: ${states.join(', ')}`);

    for (const state of states) {
      const oracleResult = runOracle(sandbox.scriptPath, [state]);
      const engineResult = runEngine(sandbox, [state]);

      assertEqual(oracleResult.code, 0, `oracle failed for ${state}: ${oracleResult.stderr}`);
      assertEqual(engineResult.code, 0, `engine failed for ${state}: ${engineResult.stderr}`);
      assertEqual(
        engineResult.stdout,
        oracleResult.stdout,
        `stdout diverges for state '${state}'\noracle: ${JSON.stringify(oracleResult.stdout)}\nengine: ${JSON.stringify(engineResult.stdout)}`,
      );
    }
  } finally {
    sandbox.cleanup();
  }
});

test('parity: idle resolves through the machine inline inject, not a skill block', () => {
  const sandbox = makeOrdersSandbox({ cursor: { flow: 'idle', phase: 'idle', feature: null, updated: '2026-01-01T00:00:00Z' } });
  try {
    const oracleResult = runOracle(sandbox.scriptPath, ['idle']);
    const engineResult = runEngine(sandbox, ['idle']);
    assertEqual(engineResult.stdout, oracleResult.stdout);
    assertMatch(engineResult.stdout, /^state=idle · no active flow/);
  } finally {
    sandbox.cleanup();
  }
});

test('parity: current-state resolution from the cursor (no explicit arg) matches the oracle', () => {
  const sandbox = makeOrdersSandbox({
    cursor: { flow: 'feature', phase: 'impl', feature: 'widget', updated: '2026-01-01T00:00:00Z' },
  });
  try {
    const oracleResult = runOracle(sandbox.scriptPath, []);
    const engineResult = runEngine(sandbox, []);
    assertEqual(engineResult.stdout, oracleResult.stdout);
    assertMatch(engineResult.stdout, /widget\/n/, 'feature interpolated into the block');
    assertMatch(engineResult.stdout, /^skill=vibe/, 'feature.impl resolves through the vibe skill block');
  } finally {
    sandbox.cleanup();
  }
});

test('parity: explicit arg overrides the cursor state, feature still comes from the cursor', () => {
  const sandbox = makeOrdersSandbox({
    cursor: { flow: 'idle', phase: 'idle', feature: 'widget', updated: '2026-01-01T00:00:00Z' },
  });
  try {
    const oracleResult = runOracle(sandbox.scriptPath, ['quick.fix']);
    const engineResult = runEngine(sandbox, ['quick.fix']);
    assertEqual(engineResult.stdout, oracleResult.stdout);
    assert(!engineResult.stdout.startsWith('state=idle'), 'explicit arg must win over the idle cursor');
  } finally {
    sandbox.cleanup();
  }
});

test('parity: <feature> interpolation replaces every occurrence, no leftover placeholder', () => {
  const sandbox = makeOrdersSandbox({
    cursor: { flow: 'feature', phase: 'plan', feature: 'demo', updated: '2026-01-01T00:00:00Z' },
  });
  try {
    const oracleResult = runOracle(sandbox.scriptPath, ['feature.plan']);
    const engineResult = runEngine(sandbox, ['feature.plan']);
    assertEqual(engineResult.stdout, oracleResult.stdout);
    assertMatch(engineResult.stdout, /demo\/n/);
    assert(!engineResult.stdout.includes('<feature>'), 'no literal placeholder should survive interpolation');
  } finally {
    sandbox.cleanup();
  }
});

test('parity: <feature> placeholder survives verbatim when the cursor has no feature', () => {
  const sandbox = makeOrdersSandbox({
    cursor: { flow: 'idle', phase: 'idle', feature: null, updated: '2026-01-01T00:00:00Z' },
  });
  try {
    const oracleResult = runOracle(sandbox.scriptPath, ['feature.impl']);
    const engineResult = runEngine(sandbox, ['feature.impl']);
    assertEqual(engineResult.stdout, oracleResult.stdout);
    assertMatch(engineResult.stdout, /<feature>\/n/);
  } finally {
    sandbox.cleanup();
  }
});

// ---------------------------------------------------------------------------
// Absent skill file → generic fallback.
// ---------------------------------------------------------------------------

test('parity: missing skill file falls all the way through to the generic fallback', () => {
  const sandbox = makeOrdersSandbox({ includeSkillFile: false });
  try {
    const oracleResult = runOracle(sandbox.scriptPath, ['feature.impl']);
    const engineResult = runEngine(sandbox, ['feature.impl']);
    assertEqual(oracleResult.code, 0);
    assertEqual(engineResult.code, 0);
    assertEqual(oracleResult.stdout, `${GENERIC_FALLBACK}\n`);
    assertEqual(engineResult.stdout, `${GENERIC_FALLBACK}\n`);
    assertEqual(engineResult.stdout, oracleResult.stdout);
  } finally {
    sandbox.cleanup();
  }
});

// ---------------------------------------------------------------------------
// Missing / misspelled closing marker — the DELIBERATE extractBlock
// divergence: the oracle's `sed -n '/open/,/close/p'` with no matching close
// address leaks the rest of the file as "block content"; extractBlock
// returns undefined instead, so the engine falls through to the correct
// fallback rather than reproducing the leak. Pinned here at the orders
// command level, not just in blocks.test.mjs's primitive-level test.
// ---------------------------------------------------------------------------

function noCloserSkillMd() {
  return [
    '<!-- vibe:orders:quick.fix -->',
    'first real orders line',
    'SECRET LEAKED LINE ONE',
    'SECRET LEAKED LINE TWO',
    'trailing line eaten by the oracle sed $d',
  ].join('\n');
}

test('divergence: a missing closing marker leaks the file tail on the oracle, NOT on the engine', () => {
  const sandbox = makeOrdersSandbox({ skillMd: noCloserSkillMd() });
  try {
    const oracleResult = runOracle(sandbox.scriptPath, ['quick.fix']);
    const engineResult = runEngine(sandbox, ['quick.fix']);

    assertEqual(oracleResult.code, 0);
    assertEqual(engineResult.code, 0);

    // The oracle really does leak — this proves the fixture reproduces the
    // bug, not just that we assume it does.
    assert(
      oracleResult.stdout.includes('SECRET LEAKED LINE'),
      `expected the oracle to leak the file tail, got: ${JSON.stringify(oracleResult.stdout)}`,
    );

    // The engine must NOT leak — it falls through to quick.fix's inline
    // inject (null, since quick.fix is skill-owning) and lands on the
    // generic fallback instead.
    assert(
      !engineResult.stdout.includes('SECRET LEAKED LINE'),
      `engine must never leak the file tail, got: ${JSON.stringify(engineResult.stdout)}`,
    );
    assertEqual(engineResult.stdout, `${GENERIC_FALLBACK}\n`);

    assert(engineResult.stdout !== oracleResult.stdout, 'this is a documented, intentional divergence from the oracle');
  } finally {
    sandbox.cleanup();
  }
});

function misspelledCloserSkillMd() {
  return [
    '<!-- vibe:orders:quick.fix -->',
    'first real orders line',
    'SECRET LEAKED LINE',
    '<!-- /vibe:order -->', // typo'd closer: matches neither exact nor legacy form
  ].join('\n');
}

test('divergence: a misspelled closing marker also leaks on the oracle, NOT on the engine', () => {
  const sandbox = makeOrdersSandbox({ skillMd: misspelledCloserSkillMd() });
  try {
    const oracleResult = runOracle(sandbox.scriptPath, ['quick.fix']);
    const engineResult = runEngine(sandbox, ['quick.fix']);

    assert(
      oracleResult.stdout.includes('SECRET LEAKED LINE'),
      `expected the oracle to leak past the misspelled closer, got: ${JSON.stringify(oracleResult.stdout)}`,
    );
    assert(
      !engineResult.stdout.includes('SECRET LEAKED LINE'),
      `engine must never leak past a misspelled closer, got: ${JSON.stringify(engineResult.stdout)}`,
    );
    assertEqual(engineResult.stdout, `${GENERIC_FALLBACK}\n`);
  } finally {
    sandbox.cleanup();
  }
});

// ---------------------------------------------------------------------------
// Absent / corrupt cursor.
// ---------------------------------------------------------------------------

test('parity: absent cursor resolves to idle, matching the oracle', () => {
  const sandbox = makeOrdersSandbox();
  try {
    rmSync(sandbox.cursorPath);
    const oracleResult = runOracle(sandbox.scriptPath, []);
    const engineResult = runEngine(sandbox, []);
    assertEqual(engineResult.stdout, oracleResult.stdout);
    assertMatch(engineResult.stdout, /^state=idle/);
  } finally {
    sandbox.cleanup();
  }
});

test('parity: corrupt (unparseable) cursor degrades to idle on both sides, exit 0', () => {
  const sandbox = makeOrdersSandbox();
  try {
    writeFileSync(sandbox.cursorPath, '{ this is not valid json');
    const oracleResult = runOracle(sandbox.scriptPath, []);
    const engineResult = runEngine(sandbox, []);
    assertEqual(oracleResult.code, 0, `oracle should degrade, not fail: ${oracleResult.stderr}`);
    assertEqual(engineResult.code, 0, `engine should degrade, not fail: ${engineResult.stderr}`);
    assertEqual(engineResult.stdout, oracleResult.stdout);
    assertMatch(engineResult.stdout, /^state=idle/);
  } finally {
    sandbox.cleanup();
  }
});

test('parity: corrupt cursor with an explicit state arg still resolves that state (only the FEATURE degrades)', () => {
  const sandbox = makeOrdersSandbox();
  try {
    writeFileSync(sandbox.cursorPath, '{ this is not valid json');
    const oracleResult = runOracle(sandbox.scriptPath, ['quick.triage']);
    const engineResult = runEngine(sandbox, ['quick.triage']);
    assertEqual(engineResult.stdout, oracleResult.stdout);
    assertMatch(engineResult.stdout, /^skill=vibe/);
  } finally {
    sandbox.cleanup();
  }
});

// ---------------------------------------------------------------------------
// Escaping-sensitive feature name — the no-jq oracle path is sed-based and
// can mis-parse embedded quotes/backslashes; guard the oracle comparison
// behind JQ_PRESENT (this runner's PATH really has jq, or it doesn't) so the
// suite stays meaningful either way instead of silently skipping.
// ---------------------------------------------------------------------------

test('feature interpolation survives quotes, backslashes, and non-ASCII characters', () => {
  const tricky = 'o"Brien\\path\\name — π';
  const sandbox = makeOrdersSandbox({
    cursor: { flow: 'feature', phase: 'plan', feature: tricky, updated: '2026-01-01T00:00:00Z' },
  });
  try {
    const engineResult = runEngine(sandbox, ['feature.plan']);
    assertEqual(engineResult.code, 0);
    assertMatch(engineResult.stdout, /^skill=vibe/);
    assert(
      engineResult.stdout.includes(`${tricky}/n`),
      `expected interpolated feature '${tricky}/n' in: ${JSON.stringify(engineResult.stdout)}`,
    );
    assert(!engineResult.stdout.includes('<feature>'), 'no literal placeholder should survive');

    if (JQ_PRESENT) {
      const oracleResult = runOracle(sandbox.scriptPath, ['feature.plan']);
      assertEqual(
        engineResult.stdout,
        oracleResult.stdout,
        `escaping-sensitive feature diverges from the jq oracle path\noracle: ${JSON.stringify(oracleResult.stdout)}\nengine: ${JSON.stringify(engineResult.stdout)}`,
      );
    }
  } finally {
    sandbox.cleanup();
  }
});

// ---------------------------------------------------------------------------
// CLI end-to-end wiring — a full install-layout fixture (engine/ copied in,
// no flow/ dir at all, no .git/.spec anywhere), spawning the real `vibe
// orders` entry point, the same way js-core/2's "install target" tests and
// js-core/3's CLI end-to-end test exercise self-relative resolution for
// real. R3: fresh non-git target returns idle orders, not `state=unknown`.
// ---------------------------------------------------------------------------

test('CLI: `vibe orders` on a fresh non-git install-layout fixture returns idle orders, not state=unknown', () => {
  const installRoot = mkdtempSync(path.join(tmpdir(), 'vibe-orders-cli-install-'));
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
  copyFileSync(path.join(REPO_ROOT, 'flow', 'SKILL.md'), path.join(vibeDir, 'SKILL.md'));
  // Deliberately no state.json (absent cursor) and no .git/.spec anywhere
  // under installRoot — the "stranger eval" shape: self-relative resolution
  // must be what saves this, not a marker walk.

  const unrelatedCwd = mkdtempSync(path.join(tmpdir(), 'vibe-orders-cli-cwd-'));
  const prevEnv = process.env.CLAUDE_PROJECT_DIR;
  delete process.env.CLAUDE_PROJECT_DIR;

  try {
    const machine = JSON.parse(readFileSync(path.join(vibeDir, 'state-machine.json'), 'utf8'));
    const expectedIdleInject = machine.states.idle.inject;

    const result = runCommand(process.execPath, [path.join(engineDir, 'cli.mjs'), 'orders'], {
      cwd: unrelatedCwd,
    });

    assertEqual(result.code, 0, `stderr: ${result.stderr}`);
    assertEqual(result.stdout, `${expectedIdleInject}\n`);
    assert(!result.stdout.startsWith('state=unknown'), 'a fresh non-git target must not collapse to the generic fallback');
  } finally {
    rmSync(installRoot, { recursive: true, force: true });
    rmSync(unrelatedCwd, { recursive: true, force: true });
    if (prevEnv !== undefined) process.env.CLAUDE_PROJECT_DIR = prevEnv;
  }
});

test('CLI: `vibe orders` with an explicit state arg on the install-layout fixture resolves that skill block', () => {
  const installRoot = mkdtempSync(path.join(tmpdir(), 'vibe-orders-cli-arg-'));
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
  copyFileSync(path.join(REPO_ROOT, 'flow', 'SKILL.md'), path.join(vibeDir, 'SKILL.md'));

  const unrelatedCwd = mkdtempSync(path.join(tmpdir(), 'vibe-orders-cli-arg-cwd-'));
  const prevEnv = process.env.CLAUDE_PROJECT_DIR;
  delete process.env.CLAUDE_PROJECT_DIR;

  try {
    const result = runCommand(
      process.execPath,
      [path.join(engineDir, 'cli.mjs'), 'orders', 'quick.triage'],
      { cwd: unrelatedCwd },
    );
    assertEqual(result.code, 0, `stderr: ${result.stderr}`);
    assertMatch(result.stdout, /^skill=vibe · READ \.spec\/lessons\.md first/);
  } finally {
    rmSync(installRoot, { recursive: true, force: true });
    rmSync(unrelatedCwd, { recursive: true, force: true });
    if (prevEnv !== undefined) process.env.CLAUDE_PROJECT_DIR = prevEnv;
  }
});
