// engine/tests/doctor.test.mjs — engine/commands/doctor.mjs (js-core/6, R2).
//
// The oracle is flow/scripts/doctor.sh. Never hand-copy its expected output:
// every parity assertion spawns a COPY of the real script (byte-identical,
// via copyFileSync) inside a throwaway sandbox laid out as
// <sandbox>/flow/scripts/doctor.sh plus <sandbox>/.agents/skills/{spec,vibe}
// and <sandbox>/.claude/{hooks,settings.json} plus a <sandbox>/.spec marker
// — doctor.sh's own find_root(SCRIPT_DIR) walks flow/scripts -> flow ->
// sandbox root (marker) — so its self-resolution runs for real, against
// sandbox files, and never touches this repo's live flow/state.json.
//
// CLAUDE_PROJECT_DIR is ambient in every Claude Code session; doctor.sh has
// NO CLAUDE_PROJECT_DIR axis at all (unlike doctrine.sh's cursor rule — see
// commands/doctor.mjs's header), so the parity assertions below call
// runDoctor() directly with the sandbox's own resolved root/vibeDir/
// skillsDir, bypassing resolveRoot()/resolveVibeDir()/resolveSkillsDir()
// entirely (same pattern doctrine.test.mjs/orders.test.mjs use for their
// core parity matrix) — only the dedicated CLI tests near the bottom
// exercise the real resolve* wiring, and those explicitly neutralise or
// pin CLAUDE_PROJECT_DIR per the js-core/5 lesson.
//
// $HOME is read by the oracle's dep_present() and the instruction.coverage
// plugin probe. Every test that cares fixtures its OWN throwaway HOME and
// passes it to both sides explicitly (oracle via spawn env, engine via
// opts.home) — never the real machine's ~/.claude.
//
// Adversarial, not just happy paths (per the task brief): missing
// state-machine.json, a broken core.spec/core.vibe symlink, an invalid
// cursor, a missing validate-state.sh, an absent deps.json dependency,
// missing hook scripts, settings.json present but not wiring a hook, and
// the no-jq path (guarded behind JQ_PRESENT, plus a couple of pinned
// divergence tests that spawn the oracle with jq genuinely removed from
// PATH via a symlink farm, never hand-copied).

import {
  mkdtempSync,
  mkdirSync,
  writeFileSync,
  readFileSync,
  readdirSync,
  copyFileSync,
  symlinkSync,
  rmSync,
  chmodSync,
  cpSync,
} from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { test, assert, assertEqual, assertMatch, runCommand } from './run.mjs';
import { runDoctor } from '../commands/doctor.mjs';

const REPO_ROOT = path.resolve(path.dirname(new URL(import.meta.url).pathname), '..', '..');
const ORACLE_SRC = path.join(REPO_ROOT, 'flow', 'scripts', 'doctor.sh');
const VALIDATE_STATE_SRC = path.join(REPO_ROOT, 'flow', 'scripts', 'validate-state.sh');
const REAL_MACHINE = readFileSync(path.join(REPO_ROOT, 'flow', 'state-machine.json'), 'utf8');
const REAL_SKILL_MD = readFileSync(path.join(REPO_ROOT, 'flow', 'SKILL.md'), 'utf8');
const REAL_DEPS = readFileSync(path.join(REPO_ROOT, 'flow', 'reference', 'deps.json'), 'utf8');

const HOOK_SCRIPTS = [
  'session-start-doctrine.sh',
  'user-prompt-submit-inject.sh',
  'pre-tool-use-guard.sh',
  'stop-gate.sh',
];

const JQ_PRESENT = runCommand('bash', ['-c', 'command -v jq >/dev/null 2>&1']).code === 0;

// ---------------------------------------------------------------------------
// A PATH with jq genuinely absent, built once (lazily) via a symlink farm
// that carries every OTHER binary on the real PATH (bash, node, coreutils,
// ...). Excluding jq's whole directory from PATH would also drop bash —
// this is the same problem the CI "jq stripped from PATH" leg has to solve,
// solved here in-process for a couple of pinned no-jq divergence tests that
// spawn the real oracle. Memoized: readdir'ing every PATH dir is not free.
// ---------------------------------------------------------------------------

let cachedNoJqPath;
function noJqPath() {
  if (cachedNoJqPath !== undefined) return cachedNoJqPath;
  if (!JQ_PRESENT) {
    cachedNoJqPath = process.env.PATH ?? '';
    return cachedNoJqPath;
  }
  const shimDir = mkdtempSync(path.join(tmpdir(), 'vibe-doctor-nojq-bin-'));
  const dirs = (process.env.PATH ?? '').split(':').filter(Boolean);
  for (const dir of dirs) {
    let entries;
    try {
      entries = readdirSync(dir);
    } catch {
      continue;
    }
    for (const name of entries) {
      if (name === 'jq') continue;
      const link = path.join(shimDir, name);
      try {
        symlinkSync(path.join(dir, name), link);
      } catch {
        // duplicate name across PATH dirs (first writer wins) — fine, that
        // mirrors normal PATH lookup precedence.
      }
    }
  }
  cachedNoJqPath = shimDir;
  return cachedNoJqPath;
}

// ---------------------------------------------------------------------------
// Sandbox builder — fine-grained control over every doctor.sh check surface.
// ---------------------------------------------------------------------------

function makeBaseSandbox() {
  const dir = mkdtempSync(path.join(tmpdir(), 'vibe-doctor-test-'));
  mkdirSync(path.join(dir, '.spec'), { recursive: true }); // marker for find_root()

  const scriptsDir = path.join(dir, 'flow', 'scripts');
  mkdirSync(scriptsDir, { recursive: true });
  const scriptPath = path.join(scriptsDir, 'doctor.sh');
  copyFileSync(ORACLE_SRC, scriptPath);

  const skillsDir = path.join(dir, '.agents', 'skills');
  const vibeDir = path.join(skillsDir, 'vibe');
  mkdirSync(vibeDir, { recursive: true });

  let cleaned = false;
  function cleanup() {
    if (cleaned) return;
    cleaned = true;
    rmSync(dir, { recursive: true, force: true });
  }

  return { dir, scriptPath, skillsDir, vibeDir, cleanup };
}

function addSpecSkill(sandbox, mode) {
  const specDir = path.join(sandbox.skillsDir, 'spec');
  rmSync(specDir, { recursive: true, force: true }); // clear whatever the healthy default left behind
  if (mode === 'absent') return;
  if (mode === 'realdir') {
    mkdirSync(specDir, { recursive: true });
    return;
  }
  if (mode === 'symlink') {
    const target = path.join(sandbox.dir, 'real-spec');
    mkdirSync(target, { recursive: true });
    symlinkSync(target, specDir);
    return;
  }
  if (mode === 'broken-symlink') {
    symlinkSync(path.join(sandbox.dir, 'does-not-exist'), specDir);
    return;
  }
  throw new Error(`unknown spec-skill mode: ${mode}`);
}

function addMachine(sandbox, content = REAL_MACHINE) {
  if (content === undefined) return;
  writeFileSync(path.join(sandbox.vibeDir, 'state-machine.json'), content);
}

function addCursor(sandbox, cursor) {
  if (cursor === undefined) return;
  const text = typeof cursor === 'string' ? cursor : `${JSON.stringify(cursor, null, 2)}\n`;
  writeFileSync(path.join(sandbox.vibeDir, 'state.json'), text);
}

function addValidateState(sandbox, { executable = true } = {}) {
  const scriptsDir = path.join(sandbox.vibeDir, 'scripts');
  mkdirSync(scriptsDir, { recursive: true });
  const dest = path.join(scriptsDir, 'validate-state.sh');
  copyFileSync(VALIDATE_STATE_SRC, dest);
  chmodSync(dest, executable ? 0o755 : 0o644);
}

function addSkillMd(sandbox, content = REAL_SKILL_MD) {
  writeFileSync(path.join(sandbox.vibeDir, 'SKILL.md'), content);
}

function addDeps(sandbox, content = REAL_DEPS) {
  if (content === undefined) return;
  const refDir = path.join(sandbox.vibeDir, 'reference');
  mkdirSync(refDir, { recursive: true });
  writeFileSync(path.join(refDir, 'deps.json'), content);
}

function addHooks(sandbox, names = HOOK_SCRIPTS) {
  const hooksDir = path.join(sandbox.dir, '.claude', 'hooks');
  mkdirSync(hooksDir, { recursive: true });
  for (const name of names) {
    writeFileSync(path.join(hooksDir, name), '#!/usr/bin/env bash\necho stub\n');
  }
}

function addSettings(sandbox, wiredNames) {
  mkdirSync(path.join(sandbox.dir, '.claude'), { recursive: true });
  const lines = wiredNames.map((n) => `bash "$CLAUDE_PROJECT_DIR/.claude/hooks/${n}"`);
  const body = { hooks: { note: lines } };
  writeFileSync(path.join(sandbox.dir, '.claude', 'settings.json'), `${JSON.stringify(body, null, 2)}\n`);
}

// A fully "healthy install" sandbox — every check ok. Individual tests
// mutate/omit pieces of this to force specific warns.
function makeHealthySandbox() {
  const sandbox = makeBaseSandbox();
  addSpecSkill(sandbox, 'realdir');
  addMachine(sandbox);
  addCursor(sandbox, { flow: 'idle', phase: 'idle', feature: null, updated: '2026-01-01T00:00:00Z' });
  addValidateState(sandbox);
  addSkillMd(sandbox);
  addHooks(sandbox);
  addSettings(sandbox, HOOK_SCRIPTS);
  addDeps(sandbox);
  return sandbox;
}

function makeHomeFixture({ skills = [], plugins = [] } = {}) {
  const home = mkdtempSync(path.join(tmpdir(), 'vibe-doctor-home-'));
  for (const name of skills) {
    mkdirSync(path.join(home, '.claude', 'skills', name), { recursive: true });
  }
  for (const relDir of plugins) {
    mkdirSync(path.join(home, '.claude', 'plugins', relDir), { recursive: true });
  }
  return home;
}

function runOracle(sandbox, opts = {}) {
  return runCommand('bash', [sandbox.scriptPath], { env: { HOME: opts.home ?? '/nonexistent-home', ...opts.env } });
}

function runEngine(sandbox, opts = {}) {
  return runDoctor(sandbox.dir, sandbox.vibeDir, sandbox.skillsDir, {
    home: opts.home ?? '/nonexistent-home',
    ...opts.engineOpts,
  });
}

function cleanupAll(...items) {
  for (const item of items) {
    if (!item) continue;
    if (typeof item === 'string') rmSync(item, { recursive: true, force: true });
    else if (typeof item.cleanup === 'function') item.cleanup();
  }
}

// ---------------------------------------------------------------------------
// Happy path.
// ---------------------------------------------------------------------------

test('parity: fully healthy install — every check ok, exit 0', () => {
  const sandbox = makeHealthySandbox();
  const home = makeHomeFixture({ skills: ['superpowers', 'feature-dev'] });
  try {
    const oracleResult = runOracle(sandbox, { home });
    const engineResult = runEngine(sandbox, { home });
    assertEqual(oracleResult.code, 0);
    assertEqual(engineResult.code, 0);
    assertEqual(engineResult.stdout, oracleResult.stdout);
    if (JQ_PRESENT) {
      assert(!engineResult.stdout.includes('warn '), `expected an all-ok run, got:\n${engineResult.stdout}`);
    }
    assertMatch(engineResult.stdout, /^# vibe doctor — /);
  } finally {
    cleanupAll(sandbox, home);
  }
});

test('parity: header line names the resolved root', () => {
  const sandbox = makeHealthySandbox();
  try {
    const engineResult = runEngine(sandbox);
    assertEqual(engineResult.stdout.split('\n')[0], `# vibe doctor — ${sandbox.dir}`);
  } finally {
    sandbox.cleanup();
  }
});

// ---------------------------------------------------------------------------
// core.spec / core.vibe — check_link_or_dir matrix. Each case would fail on
// a shortcut (e.g. one that only ever checks fs.existsSync would not
// distinguish "broken symlink" from "absent").
// ---------------------------------------------------------------------------

for (const mode of ['realdir', 'symlink', 'broken-symlink', 'absent']) {
  test(`parity: core.spec — ${mode}`, () => {
    const sandbox = makeHealthySandbox();
    try {
      addSpecSkill(sandbox, mode); // overwrite the healthy default
      const oracleResult = runOracle(sandbox);
      const engineResult = runEngine(sandbox);
      assertEqual(engineResult.stdout, oracleResult.stdout);
      const verdict = mode === 'realdir' || mode === 'symlink' ? 'ok  ' : 'warn';
      assertMatch(engineResult.stdout, new RegExp(`\\n${verdict.trimEnd()}\\s+core\\.spec `));
    } finally {
      sandbox.cleanup();
    }
  });
}

test('parity: core.vibe absent — cascades to machine/cursor/deps all reporting missing, never throws', () => {
  const dir = mkdtempSync(path.join(tmpdir(), 'vibe-doctor-novibe-'));
  mkdirSync(path.join(dir, '.spec'), { recursive: true });
  const scriptsDir = path.join(dir, 'flow', 'scripts');
  mkdirSync(scriptsDir, { recursive: true });
  const scriptPath = path.join(scriptsDir, 'doctor.sh');
  copyFileSync(ORACLE_SRC, scriptPath);
  const skillsDir = path.join(dir, '.agents', 'skills');
  mkdirSync(skillsDir, { recursive: true });
  // Deliberately no .agents/skills/vibe at all.
  const vibeDir = path.join(skillsDir, 'vibe');
  try {
    const oracleResult = runCommand('bash', [scriptPath], { env: { HOME: '/nonexistent-home' } });
    const engineResult = runDoctor(dir, vibeDir, skillsDir, { home: '/nonexistent-home' });
    assertEqual(oracleResult.code, 0);
    assertEqual(engineResult.code, 0);
    assertEqual(engineResult.stdout, oracleResult.stdout);
    assertMatch(engineResult.stdout, /warn core\.vibe /);
    assertMatch(engineResult.stdout, /warn machine state-machine\.json missing/);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

// ---------------------------------------------------------------------------
// machine — missing, invalid JSON, valid.
// ---------------------------------------------------------------------------

test('parity: machine — missing state-machine.json', () => {
  const sandbox = makeHealthySandbox();
  try {
    rmSync(path.join(sandbox.vibeDir, 'state-machine.json'));
    const oracleResult = runOracle(sandbox);
    const engineResult = runEngine(sandbox);
    assertEqual(engineResult.stdout, oracleResult.stdout);
    assertMatch(engineResult.stdout, /warn machine state-machine\.json missing at .* — flow harness incomplete/);
  } finally {
    sandbox.cleanup();
  }
});

test('parity: machine — present but not valid JSON (jq present required for this to differ)', () => {
  if (!JQ_PRESENT) return; // the no-jq variant is pinned separately below
  const sandbox = makeHealthySandbox();
  try {
    addMachine(sandbox, '{ this is not json');
    const oracleResult = runOracle(sandbox);
    const engineResult = runEngine(sandbox);
    assertEqual(engineResult.stdout, oracleResult.stdout);
    assertMatch(engineResult.stdout, /warn machine state-machine\.json is present but not valid JSON/);
  } finally {
    sandbox.cleanup();
  }
});

test('divergence pin: machine — invalid JSON but jq absent still reports ok (documented oracle shortcut)', () => {
  const sandbox = makeHealthySandbox();
  try {
    addMachine(sandbox, '{ this is not json');
    const oracleResult = runCommand('bash', [sandbox.scriptPath], {
      env: { HOME: '/nonexistent-home', PATH: noJqPath() },
    });
    const engineResult = runDoctor(sandbox.dir, sandbox.vibeDir, sandbox.skillsDir, {
      home: '/nonexistent-home',
      jqPresent: false,
    });
    assertEqual(oracleResult.code, 0);
    assertEqual(engineResult.code, 0);
    assertEqual(engineResult.stdout, oracleResult.stdout);
    assertMatch(engineResult.stdout, /ok {3}machine state-machine\.json present/, 'no-jq path never validates, always ok');
  } finally {
    sandbox.cleanup();
  }
});

// ---------------------------------------------------------------------------
// cursor — absent, invalid, missing/non-executable validate-state.sh,
// valid with various flow/phase/feature shapes.
// ---------------------------------------------------------------------------

test('parity: cursor — absent cursor is idle, ok', () => {
  const sandbox = makeHealthySandbox();
  try {
    rmSync(path.join(sandbox.vibeDir, 'state.json'));
    const oracleResult = runOracle(sandbox);
    const engineResult = runEngine(sandbox);
    assertEqual(engineResult.stdout, oracleResult.stdout);
    assertMatch(engineResult.stdout, /ok {3}cursor no flow cursor \(idle\)/);
  } finally {
    sandbox.cleanup();
  }
});

test('parity: cursor — invalid (unparseable) JSON, jq present', () => {
  if (!JQ_PRESENT) return;
  const sandbox = makeHealthySandbox();
  try {
    addCursor(sandbox, '{ not json at all');
    const oracleResult = runOracle(sandbox);
    const engineResult = runEngine(sandbox);
    assertEqual(engineResult.stdout, oracleResult.stdout);
    assertMatch(engineResult.stdout, /warn cursor flow cursor present but invalid/);
  } finally {
    sandbox.cleanup();
  }
});

test('parity: cursor — missing validate-state.sh degrades to invalid, jq present', () => {
  if (!JQ_PRESENT) return;
  const sandbox = makeHealthySandbox();
  try {
    rmSync(path.join(sandbox.vibeDir, 'scripts', 'validate-state.sh'));
    const oracleResult = runOracle(sandbox);
    const engineResult = runEngine(sandbox);
    assertEqual(engineResult.stdout, oracleResult.stdout);
    assertMatch(engineResult.stdout, /warn cursor flow cursor present but invalid/);
  } finally {
    sandbox.cleanup();
  }
});

test('parity: cursor — non-executable validate-state.sh degrades to invalid, jq present', () => {
  if (!JQ_PRESENT) return;
  const sandbox = makeHealthySandbox();
  try {
    addValidateState(sandbox, { executable: false });
    const oracleResult = runOracle(sandbox);
    const engineResult = runEngine(sandbox);
    assertEqual(engineResult.stdout, oracleResult.stdout);
    assertMatch(engineResult.stdout, /warn cursor flow cursor present but invalid/);
  } finally {
    sandbox.cleanup();
  }
});

test('parity: cursor — valid, with a feature set', () => {
  if (!JQ_PRESENT) return;
  const sandbox = makeHealthySandbox();
  try {
    addCursor(sandbox, { flow: 'feature', phase: 'impl', feature: 'widget', updated: '2026-01-01T00:00:00Z' });
    const oracleResult = runOracle(sandbox);
    const engineResult = runEngine(sandbox);
    assertEqual(engineResult.stdout, oracleResult.stdout);
    assertMatch(engineResult.stdout, /ok {3}cursor flow cursor valid \(feature\.impl feature=widget\)/);
  } finally {
    sandbox.cleanup();
  }
});

test('divergence pin: cursor — present without jq is unverified-ok, never runs validate-state.sh', () => {
  const sandbox = makeHealthySandbox();
  try {
    addCursor(sandbox, '{ not even json'); // would be invalid if validated
    const oracleResult = runCommand('bash', [sandbox.scriptPath], {
      env: { HOME: '/nonexistent-home', PATH: noJqPath() },
    });
    const engineResult = runDoctor(sandbox.dir, sandbox.vibeDir, sandbox.skillsDir, {
      home: '/nonexistent-home',
      jqPresent: false,
    });
    assertEqual(oracleResult.code, 0);
    assertEqual(engineResult.code, 0);
    assertEqual(engineResult.stdout, oracleResult.stdout);
    assertMatch(engineResult.stdout, /ok {3}cursor flow cursor present \(unverified/);
  } finally {
    sandbox.cleanup();
  }
});

// ---------------------------------------------------------------------------
// Claude adapter wiring — missing hook scripts, settings.json present but
// not wiring one, settings.json entirely absent (both sub-cases).
// ---------------------------------------------------------------------------

test('parity: adapter — a missing hook script warns for that script only', () => {
  const sandbox = makeHealthySandbox();
  try {
    rmSync(path.join(sandbox.dir, '.claude', 'hooks', 'stop-gate.sh'));
    const oracleResult = runOracle(sandbox);
    const engineResult = runEngine(sandbox);
    assertEqual(engineResult.stdout, oracleResult.stdout);
    assertMatch(engineResult.stdout, /warn adapter\.script\.stop-gate\.sh/);
    assertMatch(engineResult.stdout, /ok {3}adapter\.script\.session-start-doctrine\.sh/);
  } finally {
    sandbox.cleanup();
  }
});

test('parity: adapter — settings.json present but missing one hook name (issue #12 gap)', () => {
  const sandbox = makeHealthySandbox();
  try {
    addSettings(sandbox, HOOK_SCRIPTS.filter((n) => n !== 'pre-tool-use-guard.sh'));
    const oracleResult = runOracle(sandbox);
    const engineResult = runEngine(sandbox);
    assertEqual(engineResult.stdout, oracleResult.stdout);
    assertMatch(engineResult.stdout, /warn adapter\.activation hooks present but NOT wired.*pre-tool-use-guard\.sh/);
  } finally {
    sandbox.cleanup();
  }
});

test('parity: adapter — settings.json absent, all scripts present', () => {
  const sandbox = makeHealthySandbox();
  try {
    rmSync(path.join(sandbox.dir, '.claude', 'settings.json'));
    const oracleResult = runOracle(sandbox);
    const engineResult = runEngine(sandbox);
    assertEqual(engineResult.stdout, oracleResult.stdout);
    assertMatch(engineResult.stdout, /warn adapter\.activation \.claude\/settings\.json absent — hooks not activated/);
  } finally {
    sandbox.cleanup();
  }
});

test('parity: adapter — settings.json absent AND scripts missing', () => {
  const sandbox = makeHealthySandbox();
  try {
    rmSync(path.join(sandbox.dir, '.claude'), { recursive: true, force: true });
    const oracleResult = runOracle(sandbox);
    const engineResult = runEngine(sandbox);
    assertEqual(engineResult.stdout, oracleResult.stdout);
    assertMatch(
      engineResult.stdout,
      /warn adapter\.activation \.claude\/settings\.json absent and hook scripts missing/,
    );
  } finally {
    sandbox.cleanup();
  }
});

// ---------------------------------------------------------------------------
// instruction.coverage — three carriers, independently and combined.
// ---------------------------------------------------------------------------

test('parity: instruction.coverage — no carriers at all warns', () => {
  const sandbox = makeHealthySandbox();
  try {
    addSkillMd(sandbox, '# vibe skill\n\nno doctrine marker here\n');
    rmSync(path.join(sandbox.dir, '.claude'), { recursive: true, force: true });
    const oracleResult = runOracle(sandbox);
    const engineResult = runEngine(sandbox);
    assertEqual(engineResult.stdout, oracleResult.stdout);
    assertMatch(engineResult.stdout, /warn instruction\.coverage no doctrine coverage/);
  } finally {
    sandbox.cleanup();
  }
});

test('parity: instruction.coverage — doctrine block only', () => {
  const sandbox = makeHealthySandbox();
  try {
    addSkillMd(sandbox, ['<!-- vibe:doctrine -->', 'text', '<!-- /vibe:doctrine -->', ''].join('\n'));
    rmSync(path.join(sandbox.dir, '.claude'), { recursive: true, force: true });
    const oracleResult = runOracle(sandbox);
    const engineResult = runEngine(sandbox);
    assertEqual(engineResult.stdout, oracleResult.stdout);
    assertMatch(engineResult.stdout, /ok {3}instruction\.coverage doctrine reaches the agent via: doctrine block$/m);
  } finally {
    sandbox.cleanup();
  }
});

test('parity: instruction.coverage — SessionStart wired only (no doctrine block, settings present)', () => {
  const sandbox = makeHealthySandbox();
  try {
    addSkillMd(sandbox, '# vibe skill\n\nno doctrine marker here\n');
    // addSettings already wired session-start-doctrine.sh via the healthy default.
    const oracleResult = runOracle(sandbox);
    const engineResult = runEngine(sandbox);
    assertEqual(engineResult.stdout, oracleResult.stdout);
    assertMatch(
      engineResult.stdout,
      /ok {3}instruction\.coverage doctrine reaches the agent via: SessionStart hook$/m,
    );
  } finally {
    sandbox.cleanup();
  }
});

test('parity: instruction.coverage — per-user plugin only, discovered via a case-insensitive nested plugin.json', () => {
  const sandbox = makeHealthySandbox();
  const home = mkdtempSync(path.join(tmpdir(), 'vibe-doctor-home-plugin-'));
  try {
    addSkillMd(sandbox, '# vibe skill\n\nno doctrine marker here\n');
    rmSync(path.join(sandbox.dir, '.claude'), { recursive: true, force: true });
    const pluginDir = path.join(home, '.claude', 'plugins', 'someorg', 'VIBE-plugin');
    mkdirSync(pluginDir, { recursive: true });
    writeFileSync(path.join(pluginDir, 'plugin.json'), '{}\n');

    const oracleResult = runOracle(sandbox, { home });
    const engineResult = runEngine(sandbox, { home });
    assertEqual(engineResult.stdout, oracleResult.stdout);
    assertMatch(engineResult.stdout, /ok {3}instruction\.coverage doctrine reaches the agent via: per-user plugin$/m);
  } finally {
    sandbox.cleanup();
    rmSync(home, { recursive: true, force: true });
  }
});

test('parity: instruction.coverage — all three carriers combine in the listed order', () => {
  const sandbox = makeHealthySandbox();
  const home = mkdtempSync(path.join(tmpdir(), 'vibe-doctor-home-allcarriers-'));
  try {
    const pluginDir = path.join(home, '.claude', 'plugins', 'vibe-plugin');
    mkdirSync(pluginDir, { recursive: true });
    writeFileSync(path.join(pluginDir, 'plugin.json'), '{}\n');

    const oracleResult = runOracle(sandbox, { home });
    const engineResult = runEngine(sandbox, { home });
    assertEqual(engineResult.stdout, oracleResult.stdout);
    assertMatch(
      engineResult.stdout,
      /doctrine reaches the agent via: doctrine block, SessionStart hook, per-user plugin$/m,
    );
  } finally {
    sandbox.cleanup();
    rmSync(home, { recursive: true, force: true });
  }
});

// ---------------------------------------------------------------------------
// deps.manifest + per-dep presence.
// ---------------------------------------------------------------------------

test('parity: deps.manifest — missing deps.json', () => {
  const sandbox = makeHealthySandbox();
  try {
    rmSync(path.join(sandbox.vibeDir, 'reference'), { recursive: true, force: true });
    const oracleResult = runOracle(sandbox);
    const engineResult = runEngine(sandbox);
    assertEqual(engineResult.stdout, oracleResult.stdout);
    assertMatch(engineResult.stdout, /warn deps\.manifest deps\.json missing at /);
  } finally {
    sandbox.cleanup();
  }
});

test('parity: deps.manifest — invalid JSON, jq present', () => {
  if (!JQ_PRESENT) return;
  const sandbox = makeHealthySandbox();
  try {
    addDeps(sandbox, '{ not json');
    const oracleResult = runOracle(sandbox);
    const engineResult = runEngine(sandbox);
    assertEqual(engineResult.stdout, oracleResult.stdout);
    assertMatch(engineResult.stdout, /warn deps\.manifest deps\.json is not valid JSON/);
  } finally {
    sandbox.cleanup();
  }
});

test('divergence pin: deps.manifest — jq absent warns "unavailable", never reads the list', () => {
  const sandbox = makeHealthySandbox();
  try {
    const oracleResult = runCommand('bash', [sandbox.scriptPath], {
      env: { HOME: '/nonexistent-home', PATH: noJqPath() },
    });
    const engineResult = runDoctor(sandbox.dir, sandbox.vibeDir, sandbox.skillsDir, {
      home: '/nonexistent-home',
      jqPresent: false,
    });
    assertEqual(oracleResult.code, 0);
    assertEqual(engineResult.code, 0);
    assertEqual(engineResult.stdout, oracleResult.stdout);
    assertMatch(engineResult.stdout, /warn deps\.manifest deps\.json present but jq unavailable/);
    assert(!engineResult.stdout.includes('dep.superpowers'), 'no per-dep lines when jq is unavailable');
  } finally {
    sandbox.cleanup();
  }
});

test('parity: deps — a dependency present via $HOME/.claude/skills/<name>', () => {
  if (!JQ_PRESENT) return;
  const sandbox = makeHealthySandbox();
  const home = makeHomeFixture({ skills: ['superpowers'] });
  try {
    const oracleResult = runOracle(sandbox, { home });
    const engineResult = runEngine(sandbox, { home });
    assertEqual(engineResult.stdout, oracleResult.stdout);
    assertMatch(engineResult.stdout, /ok {3}dep\.superpowers skill-collection 'superpowers' present on disk/);
    assertMatch(engineResult.stdout, /warn dep\.feature-dev subagent-collection 'feature-dev' not found/);
  } finally {
    cleanupAll(sandbox, home);
  }
});

test('parity: deps — a dependency present via a case-insensitive nested match under $HOME/.claude/plugins', () => {
  if (!JQ_PRESENT) return;
  const sandbox = makeHealthySandbox();
  const home = makeHomeFixture({ plugins: [path.join('vendor', 'sub', 'SUPERPOWERS')] });
  try {
    const oracleResult = runOracle(sandbox, { home });
    const engineResult = runEngine(sandbox, { home });
    assertEqual(engineResult.stdout, oracleResult.stdout);
    assertMatch(engineResult.stdout, /ok {3}dep\.superpowers skill-collection 'superpowers' present on disk/);
  } finally {
    cleanupAll(sandbox, home);
  }
});

test('parity: deps — absent dependency reports the manifest degrade text verbatim', () => {
  if (!JQ_PRESENT) return;
  const sandbox = makeHealthySandbox();
  const home = makeHomeFixture();
  try {
    const oracleResult = runOracle(sandbox, { home });
    const engineResult = runEngine(sandbox, { home });
    assertEqual(engineResult.stdout, oracleResult.stdout);
    assertMatch(
      engineResult.stdout,
      /warn dep\.feature-dev subagent-collection 'feature-dev' not found — degrade: the orchestrator performs the explore \/ architect \/ review step inline/,
    );
  } finally {
    cleanupAll(sandbox, home);
  }
});

test('parity: deps — a dependency six levels deep under $HOME\\/.claude\\/plugins is NOT found (maxdepth 5)', () => {
  if (!JQ_PRESENT) return;
  const sandbox = makeHealthySandbox();
  const home = makeHomeFixture({ plugins: [path.join('a', 'b', 'c', 'd', 'e', 'superpowers')] });
  try {
    const oracleResult = runOracle(sandbox, { home });
    const engineResult = runEngine(sandbox, { home });
    assertEqual(engineResult.stdout, oracleResult.stdout);
    assertMatch(engineResult.stdout, /warn dep\.superpowers skill-collection 'superpowers' not found/);
  } finally {
    cleanupAll(sandbox, home);
  }
});

// ---------------------------------------------------------------------------
// tool.jq — reflects whatever this machine's ambient PATH actually has.
// ---------------------------------------------------------------------------

test('parity: tool.jq reflects ambient jq presence', () => {
  const sandbox = makeHealthySandbox();
  try {
    const oracleResult = runOracle(sandbox);
    const engineResult = runEngine(sandbox);
    assertEqual(engineResult.stdout, oracleResult.stdout);
    if (JQ_PRESENT) {
      assertMatch(engineResult.stdout, /ok {3}tool\.jq jq present \(/);
    } else {
      assertMatch(engineResult.stdout, /warn tool\.jq jq not installed/);
    }
  } finally {
    sandbox.cleanup();
  }
});

test('divergence pin: tool.jq — genuinely no jq on PATH', () => {
  const sandbox = makeHealthySandbox();
  try {
    const oracleResult = runCommand('bash', [sandbox.scriptPath], {
      env: { HOME: '/nonexistent-home', PATH: noJqPath() },
    });
    const engineResult = runDoctor(sandbox.dir, sandbox.vibeDir, sandbox.skillsDir, {
      home: '/nonexistent-home',
      jqPresent: false,
    });
    assertEqual(engineResult.stdout, oracleResult.stdout);
    assertMatch(
      engineResult.stdout,
      /warn tool\.jq jq not installed \(recommended, not required\) — set-state writes the cursor via printf, the guard extracts paths via sed, state reads degrade to idle; cursor \+ manifest checks unverified/,
    );
  } finally {
    sandbox.cleanup();
  }
});

// ---------------------------------------------------------------------------
// Never-throws contract for malformed argument shapes (review lesson from
// js-core/4 & /5).
// ---------------------------------------------------------------------------

test('runDoctor never throws: root is undefined', () => {
  const sandbox = makeHealthySandbox();
  try {
    let threw = false;
    let result;
    try {
      result = runDoctor(undefined, sandbox.vibeDir, sandbox.skillsDir, { home: '/nonexistent-home' });
    } catch {
      threw = true;
    }
    assert(!threw, 'runDoctor must not throw when root is undefined');
    assertEqual(result.code, 0);
  } finally {
    sandbox.cleanup();
  }
});

test('runDoctor never throws: vibeDir is null', () => {
  const sandbox = makeHealthySandbox();
  try {
    let threw = false;
    let result;
    try {
      result = runDoctor(sandbox.dir, null, sandbox.skillsDir, { home: '/nonexistent-home' });
    } catch {
      threw = true;
    }
    assert(!threw, 'runDoctor must not throw when vibeDir is null');
    assertEqual(result.code, 0);
    assertMatch(result.stdout, /warn core\.vibe null is absent/);
  } finally {
    sandbox.cleanup();
  }
});

test('runDoctor never throws: skillsDir is a number', () => {
  const sandbox = makeHealthySandbox();
  try {
    let threw = false;
    let result;
    try {
      result = runDoctor(sandbox.dir, sandbox.vibeDir, 42, { home: '/nonexistent-home' });
    } catch {
      threw = true;
    }
    assert(!threw, 'runDoctor must not throw when skillsDir is a number');
    assertEqual(result.code, 0);
  } finally {
    sandbox.cleanup();
  }
});

test('runDoctor never throws: opts is undefined', () => {
  const sandbox = makeHealthySandbox();
  try {
    let threw = false;
    let result;
    try {
      result = runDoctor(sandbox.dir, sandbox.vibeDir, sandbox.skillsDir);
    } catch {
      threw = true;
    }
    assert(!threw, 'runDoctor must not throw when opts is omitted');
    assertEqual(result.code, 0);
  } finally {
    sandbox.cleanup();
  }
});

test('runDoctor never throws: everything malformed at once (kitchen sink), still exit 0', () => {
  let threw = false;
  let result;
  try {
    result = runDoctor(123, [], { not: 'a string' }, { home: 456, jqPresent: 'not-a-boolean' });
  } catch {
    threw = true;
  }
  assert(!threw, 'runDoctor must not throw on a pile of malformed arguments');
  assertEqual(result.code, 0);
  assertEqual(typeof result.stdout, 'string');
});

// ---------------------------------------------------------------------------
// CLI end-to-end wiring on a fresh non-git install-layout fixture (same
// pattern as orders.test.mjs / doctrine.test.mjs / state.test.mjs).
// ---------------------------------------------------------------------------

test('CLI: `vibe doctor` on a fresh non-git install-layout fixture exits 0 and reports the install', () => {
  const installRoot = mkdtempSync(path.join(tmpdir(), 'vibe-doctor-cli-fresh-'));
  const vibeDir = path.join(installRoot, '.agents', 'skills', 'vibe');
  const engineDir = path.join(vibeDir, 'engine');
  mkdirSync(vibeDir, { recursive: true });
  cpSync(path.join(REPO_ROOT, 'engine'), engineDir, {
    recursive: true,
    filter: (src) => !src.includes(`${path.sep}tests${path.sep}`) && !src.endsWith(`${path.sep}tests`),
  });
  copyFileSync(path.join(REPO_ROOT, 'flow', 'SKILL.md'), path.join(vibeDir, 'SKILL.md'));
  copyFileSync(path.join(REPO_ROOT, 'flow', 'state-machine.json'), path.join(vibeDir, 'state-machine.json'));
  // Deliberately no state.json, no .claude/, no reference/deps.json, no
  // .agents/skills/spec — the "stranger eval" shape: absence must degrade
  // to warns/idle-ok, never a crash.

  const unrelatedCwd = mkdtempSync(path.join(tmpdir(), 'vibe-doctor-cli-fresh-cwd-'));
  const prevEnv = process.env.CLAUDE_PROJECT_DIR;
  delete process.env.CLAUDE_PROJECT_DIR;

  try {
    const result = runCommand(process.execPath, [path.join(engineDir, 'cli.mjs'), 'doctor'], {
      cwd: unrelatedCwd,
      env: { HOME: '/nonexistent-home' },
    });
    assertEqual(result.code, 0, `stderr: ${result.stderr}`);
    assertMatch(result.stdout, new RegExp(`^# vibe doctor — ${installRoot.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')}\n`));
    assertMatch(result.stdout, /ok {3}cursor no flow cursor \(idle\)/);
    assertMatch(result.stdout, /warn deps\.manifest deps\.json missing at /);
  } finally {
    rmSync(installRoot, { recursive: true, force: true });
    rmSync(unrelatedCwd, { recursive: true, force: true });
    if (prevEnv !== undefined) process.env.CLAUDE_PROJECT_DIR = prevEnv;
  }
});

test('CLI: `vibe doctor` with CLAUDE_PROJECT_DIR set to this repo matches the real oracle end-to-end', () => {
  const prevEnv = process.env.CLAUDE_PROJECT_DIR;
  process.env.CLAUDE_PROJECT_DIR = REPO_ROOT;
  try {
    const oracleResult = runCommand('bash', [ORACLE_SRC], { cwd: REPO_ROOT, env: { CLAUDE_PROJECT_DIR: REPO_ROOT } });
    const engineResult = runCommand(process.execPath, [path.join(REPO_ROOT, 'engine', 'cli.mjs'), 'doctor'], {
      cwd: REPO_ROOT,
      env: { CLAUDE_PROJECT_DIR: REPO_ROOT },
    });
    assertEqual(engineResult.code, 0, `stderr: ${engineResult.stderr}`);
    assertEqual(engineResult.stdout, oracleResult.stdout);
  } finally {
    if (prevEnv === undefined) delete process.env.CLAUDE_PROJECT_DIR;
    else process.env.CLAUDE_PROJECT_DIR = prevEnv;
  }
});
