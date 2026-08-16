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
// an empty/blank-only block, and absent SKILL.md.
//
// NO CURSOR ANYWHERE (inject-triggers, R4). Until unit 5 this command appended
// a live `Cursor: <state>.` line, and roughly half this file existed to pin
// that line's format, its jq-truthiness edge cases, and its CLAUDE_PROJECT_DIR
// precedence rule. SessionStart output REPLAYS on --resume, so the line was
// stale by construction; it is deleted from the command and from the oracle
// together, and the cases that only described it are deleted with it. What
// replaces them is a NEGATIVE assertion with a population floor
// (`emits the block and NO cursor line`, below): a payload that names no state
// only proves something while there is still a payload.

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
  return runDoctrine(sandbox.skillsDir);
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
// Happy path — the doctrine block, verbatim, and nothing after it (R4).
// ---------------------------------------------------------------------------

test('parity: doctrine block emitted verbatim, and NO cursor line follows it', () => {
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
      // Population floor first: "names no state" is only evidence while there
      // IS a payload to examine.
      assertMatch(engineResult.stdout, /^vibe flow — working model\./, 'doctrine block content leads the output');
      assert(engineResult.stdout.length > 200, 'sanity: the block really was emitted, so the negative below has input');
      assert(!engineResult.stdout.includes('Cursor:'), 'SessionStart output must not carry the cursor — it replays stale on --resume');
      assert(!oracleResult.stdout.includes('Cursor:'), 'oracle sanity: the bash side dropped the same line');
    });
  } finally {
    sandbox.cleanup();
  }
});

// The strongest statement of R4: the payload is a function of the SKILL.md
// block alone. Four cursors that used to produce four different last lines —
// including an absent one and an unparseable one — now produce identical bytes,
// on both sides.
test('parity: the payload is identical for every cursor, present, absent or corrupt', () => {
  const cursors = [
    { flow: 'idle', phase: 'idle', feature: null, updated: '2026-01-01T00:00:00Z' },
    { flow: 'feature', phase: 'impl', feature: 'widget', updated: '2026-01-01T00:00:00Z' },
    { flow: 'quick', phase: 'verify', feature: 'other', updated: '2026-01-01T00:00:00Z' },
  ];
  const outputs = [];
  for (const cursor of cursors) {
    const sandbox = makeDoctrineSandbox({ cursor });
    try {
      withProjectDir(undefined, () => {
        const oracleResult = runOracle(sandbox.scriptPath);
        const engineResult = runEngine(sandbox);
        assertEqual(engineResult.code, 0);
        assertEqual(engineResult.stdout, oracleResult.stdout, 'engine and oracle agree for this cursor');
        outputs.push(engineResult.stdout);
      });
    } finally {
      sandbox.cleanup();
    }
  }
  // Absent cursor, and a present-but-unparseable one.
  for (const mutate of [(sb) => rmSync(sb.cursorPath), (sb) => writeFileSync(sb.cursorPath, '{ this is not valid json')]) {
    const sandbox = makeDoctrineSandbox();
    try {
      mutate(sandbox);
      withProjectDir(undefined, () => {
        const oracleResult = runOracle(sandbox.scriptPath);
        const engineResult = runEngine(sandbox);
        assertEqual(oracleResult.code, 0, `oracle should degrade, not fail: ${oracleResult.stderr}`);
        assertEqual(engineResult.code, 0, `engine should degrade, not fail: ${engineResult.stderr}`);
        assertEqual(engineResult.stdout, oracleResult.stdout);
        outputs.push(engineResult.stdout);
      });
    } finally {
      sandbox.cleanup();
    }
  }
  assert(outputs[0].length > 200, 'sanity: there is a payload to compare');
  for (const out of outputs) {
    assertEqual(out, outputs[0], 'the cursor must not reach SessionStart output by any route');
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
// CLAUDE_PROJECT_DIR is now INERT for this command (R4). It used to select
// which cursor the summary line read; with no summary line there is nothing
// left for it to select, and the doctrine BLOCK has always come from the
// engine's own installed skill (root.mjs's contract, unchanged). Pinned in
// both directions: a project cursor that differs from the skill-local one
// changes no byte, and the block still follows the installed skill.
// ---------------------------------------------------------------------------

test('parity: a CLAUDE_PROJECT_DIR cursor changes nothing — the env var no longer reaches the output', () => {
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

    const oracleWith = runOracle(sandbox.scriptPath, { env: { CLAUDE_PROJECT_DIR: projectDir } });
    const oracleWithout = runOracle(sandbox.scriptPath, { env: { CLAUDE_PROJECT_DIR: '' } });
    const engineWith = withProjectDir(projectDir, () => runEngine(sandbox));
    const engineWithout = withProjectDir(undefined, () => runEngine(sandbox));

    assertEqual(oracleWith.code, 0);
    assertEqual(engineWith.code, 0);
    assert(engineWith.stdout.length > 200, 'sanity: there is a payload for the comparison to be about');
    assertEqual(engineWith.stdout, oracleWith.stdout, 'engine and oracle agree with the env var set');
    assertEqual(engineWith.stdout, engineWithout.stdout, 'the project cursor must not reach the engine output');
    assertEqual(oracleWith.stdout, oracleWithout.stdout, 'nor the oracle output');
    assert(!engineWith.stdout.includes('project-feature'), 'the project cursor must not leak in any spelling');
  } finally {
    sandbox.cleanup();
    rmSync(projectDir, { recursive: true, force: true });
  }
});

test('CLI: the doctrine block follows the installed skill (self-relative), and no cursor rides along', () => {
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
    assertEqual(result.stdout, 'INSTALLED SKILL DOCTRINE TEXT\n', 'the block, whole, and nothing after it');
    assert(!result.stdout.includes('plugin-project'), 'no project cursor rides SessionStart output');
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
    assert(!engineResult.stdout.includes('Cursor:'), 'no cursor line on either side, whatever the layout');
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

for (const [label, arg] of [['undefined', undefined], ['null', null], ['a number', 42], ['an object', {}]]) {
  test(`runDoctrine never throws: skillsDir is ${label}`, () => {
    let threw = false;
    let result;
    try {
      result = runDoctrine(arg);
    } catch {
      threw = true;
    }
    assert(!threw, `runDoctrine must not throw when skillsDir is ${label}`);
    assertEqual(result.code, 0);
    assertEqual(result.stdout, '');
  });
}

// ---------------------------------------------------------------------------
// CLI end-to-end wiring on a fresh non-git install-layout fixture (R3, same
// pattern as orders.test.mjs / state.test.mjs's install-target tests).
// ---------------------------------------------------------------------------

test('CLI: `vibe doctrine` on a fresh non-git install-layout fixture emits the block, and only the block', () => {
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
    assert(!result.stdout.includes('Cursor:'), 'no cursor line, even on an install with no cursor to name');
  } finally {
    rmSync(installRoot, { recursive: true, force: true });
    rmSync(unrelatedCwd, { recursive: true, force: true });
    if (prevEnv !== undefined) process.env.CLAUDE_PROJECT_DIR = prevEnv;
  }
});

// The non-string `feature` cases (jq's `//` truthiness, its 2-space
// pretty-print, and the `feature: false` agreement) lived here until unit 5.
// They pinned jqAltRaw(), which existed only to render the cursor line — with
// the line gone the function is gone, and a test for a code path that no longer
// exists is exactly the vacuous green this repo's lessons warn about. The
// `feature`-independence they cared about is now covered by
// 'the payload is identical for every cursor', above.
