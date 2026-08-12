// engine/tests/doctrine.test.mjs — engine/commands/doctrine.mjs (js-core/5, R2).
//
// The oracle is flow/scripts/doctrine.sh. Never hand-copy its expected
// output: every parity assertion spawns a COPY of the real script
// (byte-identical, via copyFileSync) inside a throwaway sandbox laid out as
// <sandbox>/flow/{scripts/doctrine.sh, state.json, state-machine.json} plus
// <sandbox>/.agents/skills/vibe/SKILL.md and a <sandbox>/.spec marker — the
// same SCRIPT_DIR/SKILL_DIR-relative + marker-search layout
// orders.test.mjs's sandbox uses (doctrine.sh's self-location logic is
// byte-identical to orders.sh's) — so its self-resolution runs for real,
// against sandbox files, and never touches this repo's live flow/state.json.
//
// CLAUDE_PROJECT_DIR is ambient in every Claude Code session, and this
// repo's own .agents/skills/vibe is a real symlink to flow/ — so an
// unneutralised CLAUDE_PROJECT_DIR=<this repo> would make doctrine's own
// precedence rule (see doctrine.mjs) silently redirect the cursor read to
// this repo's LIVE flow/state.json instead of the sandbox fixture (review
// round 1, Finding 1). Every test whose cursor line the assertions actually
// depend on either neutralises CLAUDE_PROJECT_DIR via `withProjectDir` or
// sets it to a specific, controlled value — never inherits the ambient one.
//
// Adversarial, not just happy paths (per the task brief): missing doctrine
// block, no doctrine markers at all, missing AND misspelled closing marker,
// an empty/blank-only block, absent SKILL.md, absent cursor, corrupt
// cursor, feature present vs null, and the CLAUDE_PROJECT_DIR precedence
// cases carried forward from unit 2's review.

import { mkdtempSync, mkdirSync, writeFileSync, readFileSync, copyFileSync, rmSync, cpSync } from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { test, assert, assertEqual, assertMatch, assertIncludes, makeSandbox, runCommand } from './run.mjs';
import { runDoctrine } from '../commands/doctrine.mjs';

const REPO_ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..', '..', '..');
const ORACLE_SRC = path.join(REPO_ROOT, 'flow', 'scripts', 'doctrine.sh');
const REAL_SKILL_MD = readFileSync(path.join(REPO_ROOT, 'flow', 'SKILL.md'), 'utf8');

// The oracle's no-jq degrade path sed-extracts `.feature` only when it is a
// QUOTED string on its own line — a bare number or a multi-line object
// never matches that pattern, so the no-jq oracle path itself diverges from
// its OWN jq path for these fixtures (a pre-existing, documented bash
// quirk, not something this port reproduces). Guard those comparisons
// behind jq actually being on PATH, same pattern as orders.test.mjs.
const JQ_PRESENT = runCommand('bash', ['-c', 'command -v jq >/dev/null 2>&1']).code === 0;

// ---------------------------------------------------------------------------
// Oracle sandbox — a copy of doctrine.sh plus its own flow/ cursor+machine
// and .agents/skills/vibe/SKILL.md, never the real ones.
// ---------------------------------------------------------------------------

function makeDoctrineSandbox({ cursor, skillMd = REAL_SKILL_MD, includeSkillFile = true } = {}) {
  const sandbox = makeSandbox({ cursor });
  const scriptsDir = path.join(sandbox.flowDir, 'scripts');
  mkdirSync(scriptsDir, { recursive: true });
  const scriptPath = path.join(scriptsDir, 'doctrine.sh');
  copyFileSync(ORACLE_SRC, scriptPath);

  const skillsDir = path.join(sandbox.dir, '.agents', 'skills');
  const vibeSkillDir = path.join(skillsDir, 'vibe');
  mkdirSync(vibeSkillDir, { recursive: true });
  if (includeSkillFile) {
    writeFileSync(path.join(vibeSkillDir, 'SKILL.md'), skillMd);
  }

  return { ...sandbox, scriptPath, skillsDir, vibeSkillDir };
}

function runOracle(scriptPath, opts = {}) {
  return runCommand('bash', [scriptPath], opts);
}

function runEngine(sandbox) {
  return runDoctrine(sandbox.flowDir, sandbox.skillsDir);
}

// runDoctrine reads CLAUDE_PROJECT_DIR straight from process.env (matching
// the oracle, which reads it from its own environment) — so tests that
// exercise the precedence rule set/restore it around the call rather than
// threading it through as an argument.
function withProjectDir(value, fn) {
  const prev = process.env.CLAUDE_PROJECT_DIR;
  if (value === undefined) {
    delete process.env.CLAUDE_PROJECT_DIR;
  } else {
    process.env.CLAUDE_PROJECT_DIR = value;
  }
  try {
    return fn();
  } finally {
    if (prev === undefined) delete process.env.CLAUDE_PROJECT_DIR;
    else process.env.CLAUDE_PROJECT_DIR = prev;
  }
}

// ---------------------------------------------------------------------------
// Happy path — doctrine block emitted verbatim + cursor line.
// ---------------------------------------------------------------------------

test('parity: doctrine block emitted verbatim, idle cursor with no feature', () => {
  const sandbox = makeDoctrineSandbox({
    cursor: { flow: 'idle', phase: 'idle', feature: null, updated: '2026-01-01T00:00:00Z' },
  });
  try {
    withProjectDir(undefined, () => {
      const oracleResult = runOracle(sandbox.scriptPath);
      const engineResult = runEngine(sandbox);

      assertEqual(oracleResult.code, 0);
      assertEqual(engineResult.code, 0);
      assertEqual(engineResult.stdout, oracleResult.stdout);
      assertMatch(engineResult.stdout, /^vibe flow — working model\./, 'doctrine block content leads the output');
      assertMatch(engineResult.stdout, /\nCursor: idle\.\n$/, 'cursor line appended in the current format');
    });
  } finally {
    sandbox.cleanup();
  }
});

test('parity: cursor line carries the feature when the cursor has one', () => {
  const sandbox = makeDoctrineSandbox({
    cursor: { flow: 'feature', phase: 'impl', feature: 'widget', updated: '2026-01-01T00:00:00Z' },
  });
  try {
    withProjectDir(undefined, () => {
      const oracleResult = runOracle(sandbox.scriptPath);
      const engineResult = runEngine(sandbox);

      assertEqual(engineResult.stdout, oracleResult.stdout);
      assertMatch(engineResult.stdout, /\nCursor: feature\.impl \(feature=widget\)\.\n$/);
    });
  } finally {
    sandbox.cleanup();
  }
});

// ---------------------------------------------------------------------------
// Missing block, no markers at all, empty/blank block → silent exit 0.
// ---------------------------------------------------------------------------

test('parity: SKILL.md with no doctrine markers at all → silent exit 0', () => {
  const sandbox = makeDoctrineSandbox({ skillMd: '# some skill\n\nno markers here at all\n' });
  try {
    const oracleResult = runOracle(sandbox.scriptPath);
    const engineResult = runEngine(sandbox);

    assertEqual(oracleResult.code, 0);
    assertEqual(engineResult.code, 0);
    assertEqual(oracleResult.stdout, '');
    assertEqual(engineResult.stdout, '');
    assertEqual(engineResult.stdout, oracleResult.stdout);
  } finally {
    sandbox.cleanup();
  }
});

test('parity: an empty doctrine block (opener immediately followed by closer) → silent exit 0', () => {
  const skillMd = ['<!-- vibe:doctrine -->', '<!-- /vibe:doctrine -->', ''].join('\n');
  const sandbox = makeDoctrineSandbox({ skillMd });
  try {
    const oracleResult = runOracle(sandbox.scriptPath);
    const engineResult = runEngine(sandbox);

    assertEqual(oracleResult.code, 0);
    assertEqual(engineResult.code, 0);
    assertEqual(oracleResult.stdout, '');
    assertEqual(engineResult.stdout, oracleResult.stdout);
  } finally {
    sandbox.cleanup();
  }
});

test('parity: a doctrine block containing only blank lines → silent exit 0 (command-substitution trim)', () => {
  const skillMd = ['<!-- vibe:doctrine -->', '', '', '<!-- /vibe:doctrine -->', ''].join('\n');
  const sandbox = makeDoctrineSandbox({ skillMd });
  try {
    const oracleResult = runOracle(sandbox.scriptPath);
    const engineResult = runEngine(sandbox);

    assertEqual(oracleResult.code, 0);
    assertEqual(oracleResult.stdout, '', 'oracle sanity: $(...) strips a blank-only block down to empty');
    assertEqual(engineResult.stdout, oracleResult.stdout);
  } finally {
    sandbox.cleanup();
  }
});

test('parity: absent SKILL.md → silent exit 0', () => {
  const sandbox = makeDoctrineSandbox({ includeSkillFile: false });
  try {
    const oracleResult = runOracle(sandbox.scriptPath);
    const engineResult = runEngine(sandbox);

    assertEqual(oracleResult.code, 0);
    assertEqual(engineResult.code, 0);
    assertEqual(oracleResult.stdout, '');
    assertEqual(engineResult.stdout, oracleResult.stdout);
  } finally {
    sandbox.cleanup();
  }
});

// ---------------------------------------------------------------------------
// Missing / misspelled closing marker — the DELIBERATE extractBlock
// divergence (same as orders.test.mjs): the oracle's `sed -n '/open/,/close/p'`
// with no matching close address leaks the rest of the file as "block
// content"; extractBlock returns undefined instead, so the engine falls
// straight through to silent exit 0 rather than reproducing the leak.
// ---------------------------------------------------------------------------

function noCloserSkillMd() {
  return [
    '<!-- vibe:doctrine -->',
    'first real doctrine line',
    'SECRET LEAKED LINE ONE',
    'SECRET LEAKED LINE TWO',
    'trailing line eaten by the oracle sed $d',
  ].join('\n');
}

test('divergence: a missing closing marker leaks the file tail on the oracle, NOT on the engine', () => {
  const sandbox = makeDoctrineSandbox({ skillMd: noCloserSkillMd() });
  try {
    const oracleResult = runOracle(sandbox.scriptPath);
    const engineResult = runEngine(sandbox);

    assertEqual(oracleResult.code, 0);
    assertEqual(engineResult.code, 0);

    // The oracle really does leak — proves the fixture reproduces the bug,
    // not just that we assume it does.
    assert(
      oracleResult.stdout.includes('SECRET LEAKED LINE'),
      `expected the oracle to leak the file tail, got: ${JSON.stringify(oracleResult.stdout)}`,
    );
    assert(
      !engineResult.stdout.includes('SECRET LEAKED LINE'),
      `engine must never leak the file tail, got: ${JSON.stringify(engineResult.stdout)}`,
    );
    assertEqual(engineResult.stdout, '', 'engine falls through to silent exit 0 instead');
    assert(engineResult.stdout !== oracleResult.stdout, 'this is a documented, intentional divergence from the oracle');
  } finally {
    sandbox.cleanup();
  }
});

function misspelledCloserSkillMd() {
  return [
    '<!-- vibe:doctrine -->',
    'first real doctrine line',
    'SECRET LEAKED LINE',
    '<!-- /vibe:doctine -->', // typo'd closer: does not match
  ].join('\n');
}

test('divergence: a misspelled closing marker also leaks on the oracle, NOT on the engine', () => {
  const sandbox = makeDoctrineSandbox({ skillMd: misspelledCloserSkillMd() });
  try {
    const oracleResult = runOracle(sandbox.scriptPath);
    const engineResult = runEngine(sandbox);

    assert(
      oracleResult.stdout.includes('SECRET LEAKED LINE'),
      `expected the oracle to leak past the misspelled closer, got: ${JSON.stringify(oracleResult.stdout)}`,
    );
    assert(
      !engineResult.stdout.includes('SECRET LEAKED LINE'),
      `engine must never leak past a misspelled closer, got: ${JSON.stringify(engineResult.stdout)}`,
    );
    assertEqual(engineResult.stdout, '');
  } finally {
    sandbox.cleanup();
  }
});

// ---------------------------------------------------------------------------
// Absent / corrupt cursor.
// ---------------------------------------------------------------------------

test('parity: absent cursor → Cursor: idle., matching the oracle', () => {
  const sandbox = makeDoctrineSandbox();
  try {
    rmSync(sandbox.cursorPath);
    withProjectDir(undefined, () => {
      const oracleResult = runOracle(sandbox.scriptPath);
      const engineResult = runEngine(sandbox);
      assertEqual(engineResult.stdout, oracleResult.stdout);
      assertMatch(engineResult.stdout, /\nCursor: idle\.\n$/);
    });
  } finally {
    sandbox.cleanup();
  }
});

test('parity: corrupt (unparseable) cursor degrades to idle on both sides, exit 0', () => {
  const sandbox = makeDoctrineSandbox();
  try {
    writeFileSync(sandbox.cursorPath, '{ this is not valid json');
    withProjectDir(undefined, () => {
      const oracleResult = runOracle(sandbox.scriptPath);
      const engineResult = runEngine(sandbox);
      assertEqual(oracleResult.code, 0, `oracle should degrade, not fail: ${oracleResult.stderr}`);
      assertEqual(engineResult.code, 0, `engine should degrade, not fail: ${engineResult.stderr}`);
      assertEqual(engineResult.stdout, oracleResult.stdout);
      assertMatch(engineResult.stdout, /\nCursor: idle\.\n$/);
    });
  } finally {
    sandbox.cleanup();
  }
});

// ---------------------------------------------------------------------------
// CLAUDE_PROJECT_DIR precedence — carried forward from unit 2's review.
// The oracle prefers $CLAUDE_PROJECT_DIR/.agents/skills/vibe/state.json for
// the CURSOR when that file exists, falling back to the skill-local
// state.json otherwise. Only the cursor lookup honours this; the doctrine
// block itself always comes from the skill-local SKILL.md (skillsDir),
// exactly like the oracle's SKILL_MD, which never moves.
// ---------------------------------------------------------------------------

test('parity: CLAUDE_PROJECT_DIR cursor wins over the skill-local cursor when it exists and differs', () => {
  const sandbox = makeDoctrineSandbox({
    cursor: { flow: 'idle', phase: 'idle', feature: null, updated: '2026-01-01T00:00:00Z' },
  });
  const projectDir = mkdtempSync(path.join(tmpdir(), 'vibe-doctrine-project-'));
  try {
    const projectVibeDir = path.join(projectDir, '.agents', 'skills', 'vibe');
    mkdirSync(projectVibeDir, { recursive: true });
    writeFileSync(
      path.join(projectVibeDir, 'state.json'),
      `${JSON.stringify(
        { flow: 'quick', phase: 'verify', feature: 'project-feature', updated: '2026-02-02T00:00:00Z' },
        null,
        2,
      )}\n`,
    );

    const oracleResult = runOracle(sandbox.scriptPath, { env: { CLAUDE_PROJECT_DIR: projectDir } });
    const engineResult = withProjectDir(projectDir, () => runEngine(sandbox));

    assertEqual(oracleResult.code, 0);
    assertEqual(engineResult.code, 0);
    assertEqual(engineResult.stdout, oracleResult.stdout);
    assertMatch(
      engineResult.stdout,
      /\nCursor: quick\.verify \(feature=project-feature\)\.\n$/,
      'the PROJECT cursor must win, not the skill-local idle cursor',
    );
    assert(
      !engineResult.stdout.includes('Cursor: idle'),
      'a shortcut that ignored CLAUDE_PROJECT_DIR would wrongly report the skill-local idle cursor',
    );
  } finally {
    sandbox.cleanup();
    rmSync(projectDir, { recursive: true, force: true });
  }
});

test('parity: CLAUDE_PROJECT_DIR set but its cursor file is absent → falls back to the skill-local cursor', () => {
  const sandbox = makeDoctrineSandbox({
    cursor: { flow: 'feature', phase: 'design', feature: 'local-feature', updated: '2026-01-01T00:00:00Z' },
  });
  const projectDir = mkdtempSync(path.join(tmpdir(), 'vibe-doctrine-project-empty-'));
  try {
    // CLAUDE_PROJECT_DIR points somewhere real, but with no
    // .agents/skills/vibe/state.json inside it — the oracle's own
    // `-f "$CLAUDE_PROJECT_DIR/..."` guard must fail and fall through.
    const oracleResult = runOracle(sandbox.scriptPath, { env: { CLAUDE_PROJECT_DIR: projectDir } });
    const engineResult = withProjectDir(projectDir, () => runEngine(sandbox));

    assertEqual(engineResult.stdout, oracleResult.stdout);
    assertMatch(engineResult.stdout, /\nCursor: feature\.design \(feature=local-feature\)\.\n$/);
  } finally {
    sandbox.cleanup();
    rmSync(projectDir, { recursive: true, force: true });
  }
});

test('parity: CLAUDE_PROJECT_DIR unset → skill-local cursor, unchanged from before the precedence rule existed', () => {
  const sandbox = makeDoctrineSandbox({
    cursor: { flow: 'setup', phase: 'apply', feature: null, updated: '2026-01-01T00:00:00Z' },
  });
  try {
    const oracleResult = runOracle(sandbox.scriptPath, { env: { CLAUDE_PROJECT_DIR: '' } });
    const engineResult = withProjectDir(undefined, () => runEngine(sandbox));
    assertEqual(engineResult.stdout, oracleResult.stdout);
    assertMatch(engineResult.stdout, /\nCursor: setup\.apply\.\n$/);
  } finally {
    sandbox.cleanup();
  }
});

// ---------------------------------------------------------------------------
// The precedence RULING: resolveVibeDir()/resolveSkillsDir() ignore
// CLAUDE_PROJECT_DIR once the installed self-relative chain validates
// (root.mjs's own contract, unchanged) — so the doctrine BLOCK always comes
// from the engine's own installed skill, while the CURSOR line follows
// CLAUDE_PROJECT_DIR when it has one. Pinned end-to-end via the real CLI on
// an install-layout fixture whose project cursor deliberately differs from
// the skill-local one, so a shortcut in either direction fails this test.
// ---------------------------------------------------------------------------

test('CLI: doctrine block follows the installed skill (self-relative), but the Cursor line follows CLAUDE_PROJECT_DIR', () => {
  const installRoot = mkdtempSync(path.join(tmpdir(), 'vibe-doctrine-cli-install-'));
  const vibeDir = path.join(installRoot, '.agents', 'skills', 'vibe');
  const engineDir = path.join(vibeDir, 'engine');
  mkdirSync(vibeDir, { recursive: true });
  cpSync(path.join(REPO_ROOT, 'flow', 'engine'), engineDir, {
    recursive: true,
    filter: (src) => !src.includes(`${path.sep}tests${path.sep}`) && !src.endsWith(`${path.sep}tests`),
  });
  writeFileSync(
    path.join(vibeDir, 'SKILL.md'),
    ['<!-- vibe:doctrine -->', 'INSTALLED SKILL DOCTRINE TEXT', '<!-- /vibe:doctrine -->', ''].join('\n'),
  );
  // The skill-local cursor (next to the installed engine) — deliberately
  // DIFFERENT from the project cursor below, so only a real CLAUDE_PROJECT_DIR
  // lookup (not a shortcut, not ignoring it) produces the expected output.
  writeFileSync(
    path.join(vibeDir, 'state.json'),
    `${JSON.stringify({ flow: 'idle', phase: 'idle', feature: null, updated: '2026-01-01T00:00:00Z' }, null, 2)}\n`,
  );

  const projectDir = mkdtempSync(path.join(tmpdir(), 'vibe-doctrine-cli-project-'));
  const projectVibeDir = path.join(projectDir, '.agents', 'skills', 'vibe');
  mkdirSync(projectVibeDir, { recursive: true });
  writeFileSync(
    path.join(projectVibeDir, 'state.json'),
    `${JSON.stringify(
      { flow: 'quick', phase: 'fix', feature: 'plugin-project', updated: '2026-03-03T00:00:00Z' },
      null,
      2,
    )}\n`,
  );

  const unrelatedCwd = mkdtempSync(path.join(tmpdir(), 'vibe-doctrine-cli-cwd-'));
  const prevEnv = process.env.CLAUDE_PROJECT_DIR;

  try {
    const result = runCommand(process.execPath, [path.join(engineDir, 'cli.mjs'), 'doctrine'], {
      cwd: unrelatedCwd,
      env: { CLAUDE_PROJECT_DIR: projectDir },
    });

    assertEqual(result.code, 0, `stderr: ${result.stderr}`);
    assertMatch(result.stdout, /^INSTALLED SKILL DOCTRINE TEXT\n/, 'doctrine block sourced from the installed skill, not the project');
    assertMatch(
      result.stdout,
      /\nCursor: quick\.fix \(feature=plugin-project\)\.\n$/,
      'cursor line sourced from CLAUDE_PROJECT_DIR, not the skill-local (idle) cursor',
    );
  } finally {
    rmSync(installRoot, { recursive: true, force: true });
    rmSync(projectDir, { recursive: true, force: true });
    rmSync(unrelatedCwd, { recursive: true, force: true });
    if (prevEnv !== undefined) process.env.CLAUDE_PROJECT_DIR = prevEnv;
    else delete process.env.CLAUDE_PROJECT_DIR;
  }
});

// ---------------------------------------------------------------------------
// The PLUGIN layout specifically (review round 1, Finding 2): engine at
// <PLUGIN_ROOT>/skills/vibe/engine — NOT nested under .agents/skills/vibe —
// which is the real shape build-plugin.sh ships (see
// plugin/hooks/session-start.sh's own
// `${CLAUDE_PLUGIN_ROOT}/skills/vibe/scripts/doctrine.sh`). Before the
// root.mjs fix, resolveVibeDir()'s self-relative check only recognized the
// vendored 3-level chain, so this layout fell through to the
// CLAUDE_PROJECT_DIR-honouring fallback and printed the WRONG (project's)
// doctrine block — or nothing at all once the project had no SKILL.md of
// its own, while the oracle (self-location, never CLAUDE_PROJECT_DIR for
// the block) kept printing the plugin's own doctrine regardless.
// ---------------------------------------------------------------------------

function buildPluginDoctrineFixture() {
  const pluginRoot = mkdtempSync(path.join(tmpdir(), 'vibe-doctrine-plugin-'));
  const vibeDir = path.join(pluginRoot, 'skills', 'vibe');
  const scriptsDir = path.join(vibeDir, 'scripts');
  const engineDir = path.join(vibeDir, 'engine');
  mkdirSync(scriptsDir, { recursive: true });
  copyFileSync(ORACLE_SRC, path.join(scriptsDir, 'doctrine.sh'));
  cpSync(path.join(REPO_ROOT, 'flow', 'engine'), engineDir, {
    recursive: true,
    filter: (src) => !src.includes(`${path.sep}tests${path.sep}`) && !src.endsWith(`${path.sep}tests`),
  });
  writeFileSync(
    path.join(vibeDir, 'SKILL.md'),
    ['<!-- vibe:doctrine -->', 'PLUGIN DOCTRINE TEXT', '<!-- /vibe:doctrine -->', ''].join('\n'),
  );
  return { pluginRoot, vibeDir, scriptsDir, engineDir };
}

test('parity: plugin layout (skills/vibe/engine) — project has its own SKILL.md, plugin doctrine still wins on both sides', () => {
  const { pluginRoot, vibeDir, scriptsDir, engineDir } = buildPluginDoctrineFixture();
  const projectDir = mkdtempSync(path.join(tmpdir(), 'vibe-doctrine-plugin-project-'));
  const projectVibeDir = path.join(projectDir, '.agents', 'skills', 'vibe');
  mkdirSync(projectVibeDir, { recursive: true });
  writeFileSync(
    path.join(projectVibeDir, 'SKILL.md'),
    ['<!-- vibe:doctrine -->', 'PROJECT DOCTRINE TEXT (must not leak here)', '<!-- /vibe:doctrine -->', ''].join('\n'),
  );
  writeFileSync(
    path.join(projectVibeDir, 'state.json'),
    `${JSON.stringify({ flow: 'quick', phase: 'fix', feature: 'proj', updated: '2026-04-04T00:00:00Z' }, null, 2)}\n`,
  );
  const unrelatedCwd = mkdtempSync(path.join(tmpdir(), 'vibe-doctrine-plugin-cwd-'));

  try {
    const oracleResult = runCommand('bash', [path.join(scriptsDir, 'doctrine.sh')], {
      cwd: unrelatedCwd,
      env: { CLAUDE_PROJECT_DIR: projectDir },
    });
    const engineResult = runCommand(process.execPath, [path.join(engineDir, 'cli.mjs'), 'doctrine'], {
      cwd: unrelatedCwd,
      env: { CLAUDE_PROJECT_DIR: projectDir },
    });

    assertEqual(oracleResult.code, 0);
    assertEqual(engineResult.code, 0, `stderr: ${engineResult.stderr}`);
    assertMatch(oracleResult.stdout, /^PLUGIN DOCTRINE TEXT\n/, 'oracle sanity: block is the plugin\'s own');
    assertMatch(engineResult.stdout, /^PLUGIN DOCTRINE TEXT\n/, 'block must be the plugin\'s own, not the project\'s');
    assert(!engineResult.stdout.includes('PROJECT DOCTRINE TEXT'), 'must never leak the project doctrine block');
    assertEqual(engineResult.stdout, oracleResult.stdout);
    assertMatch(engineResult.stdout, /\nCursor: quick\.fix \(feature=proj\)\.\n$/, 'cursor still follows CLAUDE_PROJECT_DIR');
  } finally {
    rmSync(pluginRoot, { recursive: true, force: true });
    rmSync(projectDir, { recursive: true, force: true });
    rmSync(unrelatedCwd, { recursive: true, force: true });
  }
});

test('parity: plugin layout — project has NO SKILL.md at all, plugin doctrine still prints (not silence) on both sides', () => {
  const { pluginRoot, vibeDir, scriptsDir, engineDir } = buildPluginDoctrineFixture();
  const projectDir = mkdtempSync(path.join(tmpdir(), 'vibe-doctrine-plugin-project-nomd-'));
  const projectVibeDir = path.join(projectDir, '.agents', 'skills', 'vibe');
  mkdirSync(projectVibeDir, { recursive: true });
  // Deliberately NO SKILL.md under the project's vibe dir — this is the
  // exact case the reviewer flagged as going silently empty pre-fix.
  writeFileSync(
    path.join(projectVibeDir, 'state.json'),
    `${JSON.stringify({ flow: 'idle', phase: 'idle', feature: null, updated: '2026-05-05T00:00:00Z' }, null, 2)}\n`,
  );
  const unrelatedCwd = mkdtempSync(path.join(tmpdir(), 'vibe-doctrine-plugin-nomd-cwd-'));

  try {
    const oracleResult = runCommand('bash', [path.join(scriptsDir, 'doctrine.sh')], {
      cwd: unrelatedCwd,
      env: { CLAUDE_PROJECT_DIR: projectDir },
    });
    const engineResult = runCommand(process.execPath, [path.join(engineDir, 'cli.mjs'), 'doctrine'], {
      cwd: unrelatedCwd,
      env: { CLAUDE_PROJECT_DIR: projectDir },
    });

    assertEqual(oracleResult.code, 0);
    assertEqual(engineResult.code, 0, `stderr: ${engineResult.stderr}`);
    assertMatch(oracleResult.stdout, /^PLUGIN DOCTRINE TEXT\n/, 'oracle sanity: still prints the plugin doctrine');
    assertMatch(
      engineResult.stdout,
      /^PLUGIN DOCTRINE TEXT\n/,
      'engine must NOT go silent just because the project has no SKILL.md of its own',
    );
    assertEqual(engineResult.stdout, oracleResult.stdout);
  } finally {
    rmSync(pluginRoot, { recursive: true, force: true });
    rmSync(projectDir, { recursive: true, force: true });
    rmSync(unrelatedCwd, { recursive: true, force: true });
  }
});

// ---------------------------------------------------------------------------
// Never-throws contract for malformed argument shapes (review lesson from
// js-core/4, Finding 3/4).
// ---------------------------------------------------------------------------

test('runDoctrine never throws: skillsDir is undefined', () => {
  const sandbox = makeDoctrineSandbox();
  try {
    let threw = false;
    let result;
    try {
      result = runDoctrine(sandbox.flowDir, undefined);
    } catch {
      threw = true;
    }
    assert(!threw, 'runDoctrine must not throw when skillsDir is undefined');
    assertEqual(result.code, 0);
    assertEqual(result.stdout, '');
  } finally {
    sandbox.cleanup();
  }
});

test('runDoctrine never throws: skillsDir is not a string', () => {
  const sandbox = makeDoctrineSandbox();
  try {
    let threw = false;
    let result;
    try {
      result = runDoctrine(sandbox.flowDir, 42);
    } catch {
      threw = true;
    }
    assert(!threw, 'runDoctrine must not throw when skillsDir is a number');
    assertEqual(result.code, 0);
    assertEqual(result.stdout, '');
  } finally {
    sandbox.cleanup();
  }
});

test('runDoctrine never throws: vibeDir is undefined → cursor degrades to idle, block still resolves', () => {
  const sandbox = makeDoctrineSandbox({
    cursor: { flow: 'quick', phase: 'triage', feature: 'x', updated: '2026-01-01T00:00:00Z' },
  });
  try {
    let threw = false;
    let result;
    try {
      result = runDoctrine(undefined, sandbox.skillsDir);
    } catch {
      threw = true;
    }
    assert(!threw, 'runDoctrine must not throw when vibeDir is undefined');
    assertEqual(result.code, 0);
    assertMatch(result.stdout, /^vibe flow — working model\./);
    assertMatch(result.stdout, /\nCursor: idle\.\n$/, 'no vibeDir means no cursor to read — idle, not a crash');
  } finally {
    sandbox.cleanup();
  }
});

test('runDoctrine never throws: vibeDir is null', () => {
  const sandbox = makeDoctrineSandbox();
  try {
    let threw = false;
    let result;
    try {
      result = runDoctrine(null, sandbox.skillsDir);
    } catch {
      threw = true;
    }
    assert(!threw, 'runDoctrine must not throw when vibeDir is null');
    assertEqual(result.code, 0);
    assertMatch(result.stdout, /\nCursor: idle\.\n$/);
  } finally {
    sandbox.cleanup();
  }
});

// ---------------------------------------------------------------------------
// CLI end-to-end wiring on a fresh non-git install-layout fixture (R3, same
// pattern as orders.test.mjs / state.test.mjs's install-target tests).
// ---------------------------------------------------------------------------

test('CLI: `vibe doctrine` on a fresh non-git install-layout fixture emits the block and Cursor: idle.', () => {
  const installRoot = mkdtempSync(path.join(tmpdir(), 'vibe-doctrine-cli-fresh-'));
  const vibeDir = path.join(installRoot, '.agents', 'skills', 'vibe');
  const engineDir = path.join(vibeDir, 'engine');
  mkdirSync(vibeDir, { recursive: true });
  cpSync(path.join(REPO_ROOT, 'flow', 'engine'), engineDir, {
    recursive: true,
    filter: (src) => !src.includes(`${path.sep}tests${path.sep}`) && !src.endsWith(`${path.sep}tests`),
  });
  copyFileSync(path.join(REPO_ROOT, 'flow', 'SKILL.md'), path.join(vibeDir, 'SKILL.md'));
  // Deliberately no state.json (absent cursor) and no .git/.spec anywhere
  // under installRoot — the "stranger eval" shape.

  const unrelatedCwd = mkdtempSync(path.join(tmpdir(), 'vibe-doctrine-cli-fresh-cwd-'));
  const prevEnv = process.env.CLAUDE_PROJECT_DIR;
  delete process.env.CLAUDE_PROJECT_DIR;

  try {
    const result = runCommand(process.execPath, [path.join(engineDir, 'cli.mjs'), 'doctrine'], {
      cwd: unrelatedCwd,
    });
    assertEqual(result.code, 0, `stderr: ${result.stderr}`);
    assertMatch(result.stdout, /^vibe flow — working model\./);
    assertMatch(result.stdout, /\nCursor: idle\.\n$/);
  } finally {
    rmSync(installRoot, { recursive: true, force: true });
    rmSync(unrelatedCwd, { recursive: true, force: true });
    if (prevEnv !== undefined) process.env.CLAUDE_PROJECT_DIR = prevEnv;
  }
});

// ---------------------------------------------------------------------------
// Non-string cursor `feature` (review round 1, Finding 4). jq's `//` treats
// everything except `false`/`null` as truthy — including `0` — so a cursor
// with `"feature": 0` DOES get a Cursor line on the oracle. A plain JS
// `feature ? ... : ...` gets this wrong (0 is JS-falsy). Only reachable via
// a hand-edited cursor; readCursor/writeCursor never produce these shapes.
// ---------------------------------------------------------------------------

test('parity: a numeric feature of 0 still gets a Cursor line (jq truthy, JS-falsy mismatch)', () => {
  const sandbox = makeDoctrineSandbox({
    cursor: { flow: 'quick', phase: 'fix', feature: 0, updated: '2026-01-01T00:00:00Z' },
  });
  try {
    withProjectDir(undefined, () => {
      const engineResult = runEngine(sandbox);
      assertEqual(engineResult.code, 0);
      assertMatch(
        engineResult.stdout,
        /\nCursor: quick\.fix \(feature=0\)\.\n$/,
        'feature=0 must still render, matching jq: only false/null are falsy for //',
      );
      if (JQ_PRESENT) {
        const oracleResult = runOracle(sandbox.scriptPath);
        assertEqual(engineResult.stdout, oracleResult.stdout);
      }
    });
  } finally {
    sandbox.cleanup();
  }
});

test('parity: an object feature renders as pretty (2-space) JSON, matching jq\'s default output', () => {
  const sandbox = makeDoctrineSandbox({
    cursor: { flow: 'quick', phase: 'fix', feature: { x: 1, y: 2 }, updated: '2026-01-01T00:00:00Z' },
  });
  try {
    withProjectDir(undefined, () => {
      const engineResult = runEngine(sandbox);
      assertEqual(engineResult.code, 0);
      assertIncludes(engineResult.stdout, 'Cursor: quick.fix (feature={\n  "x": 1,\n  "y": 2\n}).\n');
      if (JQ_PRESENT) {
        const oracleResult = runOracle(sandbox.scriptPath);
        assertEqual(engineResult.stdout, oracleResult.stdout);
      }
    });
  } finally {
    sandbox.cleanup();
  }
});

test('parity: a boolean-false feature is treated as absent, same as jq (the one value both sides agree is falsy)', () => {
  const sandbox = makeDoctrineSandbox({
    cursor: { flow: 'quick', phase: 'fix', feature: false, updated: '2026-01-01T00:00:00Z' },
  });
  try {
    withProjectDir(undefined, () => {
      const engineResult = runEngine(sandbox);
      assertEqual(engineResult.code, 0);
      assertMatch(engineResult.stdout, /\nCursor: quick\.fix\.\n$/);
      if (JQ_PRESENT) {
        const oracleResult = runOracle(sandbox.scriptPath);
        assertEqual(engineResult.stdout, oracleResult.stdout);
      }
    });
  } finally {
    sandbox.cleanup();
  }
});
