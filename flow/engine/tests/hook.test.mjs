// engine/tests/hook.test.mjs — engine/commands/hook.mjs (js-core/7).
//
// Unlike state/orders/doctrine/doctor, there is no single bash oracle FILE
// here — hook.mjs reimplements what each `.claude/hooks/*.sh` shim did
// AROUND its ported command. Coverage below mixes hermetic unit tests
// (spawn* functions injected via opts, no real bash/git needed) with a
// handful of integration tests that copy the REAL detect-context.sh (and, for
// the gate, a real git repo) into a sandbox, mirroring
// flow/tests/adapters/run.sh's own "hooks against a real install" section so
// the two suites stay honest about the same behaviour.

import {
  mkdtempSync,
  mkdirSync,
  writeFileSync,
  readFileSync,
  copyFileSync,
  rmSync,
  utimesSync,
} from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { test, assert, assertEqual, assertMatch, assertIncludes, runCommand } from './run.mjs';
import {
  runDoctrineHook,
  runInjectHook,
  runGuardHook,
  runGateHook,
} from '../commands/hook.mjs';

const REPO_ROOT = path.resolve(path.dirname(new URL(import.meta.url).pathname), '..', '..', '..');
const DETECT_ORACLE = path.join(REPO_ROOT, 'flow', 'scripts', 'detect-context.sh');
const REAL_SKILL_MD = readFileSync(path.join(REPO_ROOT, 'flow', 'SKILL.md'), 'utf8');
const REAL_MACHINE = path.join(REPO_ROOT, 'flow', 'state-machine.json');

// ---------------------------------------------------------------------------
// Fixture — a real install-shaped layout: <root>/.agents/skills/vibe/{state.json,
// state-machine.json, SKILL.md, scripts/detect-context.sh, warnings.log?,
// evidence/}. Mirrors what `install.sh` lays down, handcrafted (not run
// through bash install.sh) to keep the suite fast and hermetic by default.
// ---------------------------------------------------------------------------

function makeHookSandbox({ cursor, includeDetect = true, gitInit = false } = {}) {
  const dir = mkdtempSync(path.join(tmpdir(), 'vibe-hook-test-'));
  const vibeDir = path.join(dir, '.agents', 'skills', 'vibe');
  const scriptsDir = path.join(vibeDir, 'scripts');
  mkdirSync(scriptsDir, { recursive: true });
  copyFileSync(REAL_MACHINE, path.join(vibeDir, 'state-machine.json'));
  writeFileSync(path.join(vibeDir, 'SKILL.md'), REAL_SKILL_MD);
  if (includeDetect) {
    copyFileSync(DETECT_ORACLE, path.join(scriptsDir, 'detect-context.sh'));
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

function writeCursor(sb, body) {
  writeFileSync(sb.cursorPath, `${JSON.stringify(body, null, 2)}\n`);
}

// runDoctrineHook (via doctrine.mjs) and a real detect-context.sh `infer`
// spawn both read CLAUDE_PROJECT_DIR straight from process.env — ambient in
// every Claude Code session — so tests exercising them against a sandbox
// must neutralise it around the call, same pattern as doctrine.test.mjs's
// own withProjectDir().
function withProjectDir(value, fn) {
  const prev = process.env.CLAUDE_PROJECT_DIR;
  if (value === undefined) delete process.env.CLAUDE_PROJECT_DIR;
  else process.env.CLAUDE_PROJECT_DIR = value;
  try {
    return fn();
  } finally {
    if (prev === undefined) delete process.env.CLAUDE_PROJECT_DIR;
    else process.env.CLAUDE_PROJECT_DIR = prev;
  }
}

// ---------------------------------------------------------------------------
// session-start-doctrine
// ---------------------------------------------------------------------------

test('runDoctrineHook delegates to the ported doctrine command and always exits 0', () => {
  const sb = makeHookSandbox();
  try {
    const result = withProjectDir(sb.root, () => runDoctrineHook(sb.vibeDir, sb.skillsDir));
    assertEqual(result.code, 0);
    assertIncludes(result.stdout, 'sessions are ephemeral');
    assertIncludes(result.stdout, 'Cursor: idle.');
  } finally {
    sb.cleanup();
  }
});

test('runDoctrineHook degrades silently when the skill file is absent', () => {
  const sb = makeHookSandbox();
  try {
    rmSync(path.join(sb.vibeDir, 'SKILL.md'));
    const result = runDoctrineHook(sb.vibeDir, sb.skillsDir);
    assertEqual(result.code, 0);
    assertEqual(result.stdout, '');
  } finally {
    sb.cleanup();
  }
});

// ---------------------------------------------------------------------------
// user-prompt-submit-inject — drift-first nudge + orders + warnings relay
// ---------------------------------------------------------------------------

test('runInjectHook: no drift -> orders are the first (and only) content', () => {
  const sb = makeHookSandbox();
  try {
    const result = runInjectHook(sb.root, sb.vibeDir, sb.skillsDir, {
      spawnInfer: () => ({ error: null, stdout: '' }),
    });
    assertEqual(result.code, 0);
    assertMatch(result.stdout, /^state=idle/);
  } finally {
    sb.cleanup();
  }
});

test('runInjectHook: a drift verdict is prepended as line 1, stripped of the drift:<state>: prefix', () => {
  const sb = makeHookSandbox();
  try {
    const result = runInjectHook(sb.root, sb.vibeDir, sb.skillsDir, {
      spawnInfer: () => ({
        error: null,
        stdout: 'drift:feature.impl:src edits in idle — set-state.sh feature.impl\n',
      }),
    });
    const first = result.stdout.split('\n')[0];
    assertEqual(first, 'vibe-drift: src edits in idle — set-state.sh feature.impl');
    assertMatch(result.stdout, /state=idle/, 'orders still follow the drift line');
  } finally {
    sb.cleanup();
  }
});

test('runInjectHook: drains the warnings relay to stdout once, prefixed vibe-warn:, then truncates', () => {
  const sb = makeHookSandbox();
  try {
    writeFileSync(sb.warnLogPath, 'gate: still in feature.impl (warn-only)\nguard: outside idle write (warn-only)\n');
    const result = runInjectHook(sb.root, sb.vibeDir, sb.skillsDir, {
      spawnInfer: () => ({ error: null, stdout: '' }),
    });
    const warnLines = result.stdout.split('\n').filter((l) => l.startsWith('vibe-warn:'));
    assertEqual(warnLines.length, 2);
    assertEqual(readFileSync(sb.warnLogPath, 'utf8'), '');

    const again = runInjectHook(sb.root, sb.vibeDir, sb.skillsDir, {
      spawnInfer: () => ({ error: null, stdout: '' }),
    });
    assert(!again.stdout.includes('vibe-warn:'), 'a drained relay must not repeat on the next turn');
  } finally {
    sb.cleanup();
  }
});

test('runInjectHook: integration — real detect-context.sh infer produces the drift line for idle + src edit', () => {
  const sb = makeHookSandbox({ gitInit: true });
  try {
    mkdirSync(path.join(sb.dir, 'src'), { recursive: true });
    writeFileSync(path.join(sb.dir, 'src', 'app.sh'), 'x\n');
    const result = withProjectDir(sb.root, () => runInjectHook(sb.root, sb.vibeDir, sb.skillsDir, {}));
    assertMatch(result.stdout.split('\n')[0], /^vibe-drift:/);
    assertMatch(result.stdout, /state=idle/);
  } finally {
    sb.cleanup();
  }
});

// ---------------------------------------------------------------------------
// pre-tool-use-guard
// ---------------------------------------------------------------------------

test('runGuardHook: empty stdin -> exit 0, nothing written', () => {
  const sb = makeHookSandbox();
  try {
    const result = runGuardHook(sb.root, '', {});
    assertEqual(result.code, 0);
    assertEqual(result.stdout, '');
    assertEqual(result.stderr, '');
  } finally {
    sb.cleanup();
  }
});

test('runGuardHook: missing detect-context.sh -> exit 0, never blocks', () => {
  const sb = makeHookSandbox({ includeDetect: false });
  try {
    const result = runGuardHook(sb.root, JSON.stringify({ tool_name: 'Write', tool_input: { file_path: '.spec/lessons.md' } }), {});
    assertEqual(result.code, 0);
  } finally {
    sb.cleanup();
  }
});

test('runGuardHook: Bash sniffer warns (never blocks) on a write-shaped op against a guarded path', () => {
  const sb = makeHookSandbox();
  try {
    const stdin = JSON.stringify({ tool_name: 'Bash', tool_input: { command: 'echo x >> .spec/lessons.md' } });
    const result = runGuardHook(sb.root, stdin, {});
    assertEqual(result.code, 0);
    assertIncludes(result.stderr, 'warn');
    assertIncludes(result.stderr, '.spec/lessons.md');
    assertIncludes(readFileSync(sb.warnLogPath, 'utf8'), 'lessons.md');
  } finally {
    sb.cleanup();
  }
});

test('runGuardHook: Bash sniffer never warns on a pure read (no write-shaped op)', () => {
  const sb = makeHookSandbox();
  try {
    const stdin = JSON.stringify({ tool_name: 'Bash', tool_input: { command: 'grep foo .spec/lessons.md' } });
    const result = runGuardHook(sb.root, stdin, {});
    assertEqual(result.code, 0);
    assertEqual(result.stderr, '');
  } finally {
    sb.cleanup();
  }
});

test('runGuardHook: Bash sniffer never warns about state.json when the command runs set-state.sh', () => {
  const sb = makeHookSandbox();
  try {
    const stdin = JSON.stringify({
      tool_name: 'Bash',
      tool_input: { command: 'bash .agents/skills/vibe/scripts/set-state.sh idle > .agents/skills/vibe/state.json' },
    });
    const result = runGuardHook(sb.root, stdin, {});
    assertEqual(result.code, 0);
    assertEqual(result.stderr, '');
  } finally {
    sb.cleanup();
  }
});

test('runGuardHook: file-tool verdict translation — block -> exit 2 with the reason', () => {
  const sb = makeHookSandbox();
  try {
    const stdin = JSON.stringify({ tool_name: 'Write', tool_input: { file_path: '.spec/lessons.md' } });
    const result = runGuardHook(sb.root, stdin, {
      spawnDecide: () => ({ error: null, status: 0, stdout: 'block:not allowed right now\n' }),
    });
    assertEqual(result.code, 2);
    assertIncludes(result.stderr, 'BLOCKED');
    assertIncludes(result.stderr, 'not allowed right now');
  } finally {
    sb.cleanup();
  }
});

test('runGuardHook: file-tool verdict translation — warn -> exit 0, stderr + relay log', () => {
  const sb = makeHookSandbox();
  try {
    const stdin = JSON.stringify({ tool_name: 'Write', tool_input: { file_path: 'src/x.sh' } });
    const result = runGuardHook(sb.root, stdin, {
      spawnDecide: () => ({ error: null, status: 0, stdout: 'warn:outside an impl state\n' }),
    });
    assertEqual(result.code, 0);
    assertIncludes(result.stderr, 'warn');
    assertIncludes(readFileSync(sb.warnLogPath, 'utf8'), 'outside an impl state');
  } finally {
    sb.cleanup();
  }
});

test('runGuardHook: file-tool verdict translation — allow -> exit 0, silent', () => {
  const sb = makeHookSandbox();
  try {
    const stdin = JSON.stringify({ tool_name: 'Write', tool_input: { file_path: 'src/x.sh' } });
    const result = runGuardHook(sb.root, stdin, {
      spawnDecide: () => ({ error: null, status: 0, stdout: 'allow\n' }),
    });
    assertEqual(result.code, 0);
    assertEqual(result.stderr, '');
  } finally {
    sb.cleanup();
  }
});

test('runGuardHook: a spawn failure degrades to allow, never blocks', () => {
  const sb = makeHookSandbox();
  try {
    const stdin = JSON.stringify({ tool_name: 'Write', tool_input: { file_path: 'src/x.sh' } });
    const result = runGuardHook(sb.root, stdin, {
      spawnDecide: () => ({ error: new Error('boom'), stdout: '' }),
    });
    assertEqual(result.code, 0);
  } finally {
    sb.cleanup();
  }
});

test('runGuardHook: a root-prefixed absolute path is normalized before being decided', () => {
  const sb = makeHookSandbox();
  try {
    let seenPath;
    const stdin = JSON.stringify({
      tool_name: 'Write',
      tool_input: { file_path: path.join(sb.root, '.spec', 'lessons.md') },
    });
    runGuardHook(sb.root, stdin, {
      spawnDecide: (args) => {
        seenPath = args[2];
        return { error: null, status: 0, stdout: 'allow\n' };
      },
    });
    assertEqual(seenPath, '.spec/lessons.md');
  } finally {
    sb.cleanup();
  }
});

test('runGuardHook: NotebookEdit uses notebook_path', () => {
  const sb = makeHookSandbox();
  try {
    const stdin = JSON.stringify({ tool_name: 'NotebookEdit', tool_input: { notebook_path: 'nb.ipynb' } });
    const result = runGuardHook(sb.root, stdin, {
      spawnDecide: () => ({ error: null, status: 0, stdout: 'allow\n' }),
    });
    assertEqual(result.code, 0);
  } finally {
    sb.cleanup();
  }
});

test('runGuardHook: integration — real detect-context.sh blocks a direct state.json edit (exit 2)', () => {
  const sb = makeHookSandbox();
  try {
    const stdin = JSON.stringify({ tool_name: 'Write', tool_input: { file_path: '.agents/skills/vibe/state.json' } });
    const result = runGuardHook(sb.root, stdin, {});
    assertEqual(result.code, 2);
    assertIncludes(result.stderr, 'BLOCKED');
  } finally {
    sb.cleanup();
  }
});

test('runGuardHook: integration — real detect-context.sh allows src/ writes during feature.impl', () => {
  const sb = makeHookSandbox({ cursor: { flow: 'feature', phase: 'impl', feature: 'demo', updated: '2026-01-01T00:00:00Z' } });
  try {
    const stdin = JSON.stringify({ tool_name: 'Write', tool_input: { file_path: 'src/x.sh' } });
    const result = runGuardHook(sb.root, stdin, {});
    assertEqual(result.code, 0);
    assertEqual(result.stderr, '');
  } finally {
    sb.cleanup();
  }
});

// ---------------------------------------------------------------------------
// stop-gate
// ---------------------------------------------------------------------------

test('runGateHook: stop_hook_active suppresses every check (re-entry guard)', () => {
  const sb = makeHookSandbox({ cursor: { flow: 'feature', phase: 'verify', feature: 'demo', updated: '2026-01-01T00:00:00Z' } });
  try {
    const result = runGateHook(sb.root, sb.vibeDir, JSON.stringify({ stop_hook_active: true }), {});
    assertEqual(result.code, 0);
    assertEqual(result.stderr, '');
  } finally {
    sb.cleanup();
  }
});

test('runGateHook: missing detect-context.sh -> exit 0, never blocks', () => {
  const sb = makeHookSandbox({
    includeDetect: false,
    cursor: { flow: 'feature', phase: 'verify', feature: 'demo', updated: '2026-01-01T00:00:00Z' },
  });
  try {
    const result = runGateHook(sb.root, sb.vibeDir, '{}', {});
    assertEqual(result.code, 0);
  } finally {
    sb.cleanup();
  }
});

// Predicate 1's oracle regex is `(^|/)src/` applied to a RAW porcelain line
// ("XY path"), so a top-level "src/..." change never matches (the char
// before "src/" is the porcelain status column's trailing space, not "/" or
// start-of-string) — confirmed against real bash:
//   printf ' M src/app.sh\n' | grep -qE '(^|/)src/'   # no match
// Only a NESTED path ("pkg/src/...") matches. This is a latent bug in the
// bash oracle itself, faithfully reproduced here, not introduced by the
// port — fixtures below use a nested path so the assertion is meaningful.
test('runGateHook predicate 1 (TDD, warn-only): src changed with no tests changed in feature.impl', () => {
  const sb = makeHookSandbox({ cursor: { flow: 'feature', phase: 'impl', feature: 'demo', updated: '2026-01-01T00:00:00Z' } });
  try {
    const result = runGateHook(sb.root, sb.vibeDir, '{}', {
      spawnGit: (args) =>
        args.includes('rev-parse')
          ? { error: null, status: 0, stdout: 'true\n' }
          : { error: null, status: 0, stdout: ' M pkg/src/app.sh\n' },
    });
    assertEqual(result.code, 0);
    assertIncludes(result.stderr, 'TDD expects');
    assertIncludes(readFileSync(sb.warnLogPath, 'utf8'), 'gate: in feature.impl, src changed');
  } finally {
    sb.cleanup();
  }
});

test('runGateHook predicate 1: matching a top-level (non-nested) src/ change never fires — pinned oracle quirk', () => {
  const sb = makeHookSandbox({ cursor: { flow: 'feature', phase: 'impl', feature: 'demo', updated: '2026-01-01T00:00:00Z' } });
  try {
    const result = runGateHook(sb.root, sb.vibeDir, '{}', {
      spawnGit: (args) =>
        args.includes('rev-parse')
          ? { error: null, status: 0, stdout: 'true\n' }
          : { error: null, status: 0, stdout: ' M src/app.sh\n' },
    });
    assert(!result.stderr.includes('TDD expects'), 'a top-level src/ change must not trip predicate 1, matching the oracle');
  } finally {
    sb.cleanup();
  }
});

test('runGateHook predicate 1: no warn when tests/ also changed', () => {
  const sb = makeHookSandbox({ cursor: { flow: 'feature', phase: 'impl', feature: 'demo', updated: '2026-01-01T00:00:00Z' } });
  try {
    const result = runGateHook(sb.root, sb.vibeDir, '{}', {
      spawnGit: (args) =>
        args.includes('rev-parse')
          ? { error: null, status: 0, stdout: 'true\n' }
          : { error: null, status: 0, stdout: ' M pkg/src/app.sh\n M pkg/tests/app.test.sh\n' },
    });
    assert(!result.stderr.includes('TDD expects'));
  } finally {
    sb.cleanup();
  }
});

test('runGateHook predicate 2 (blocking): feature.verify with no receipt -> exit 2, names the path', () => {
  const sb = makeHookSandbox({ cursor: { flow: 'feature', phase: 'verify', feature: 'demo', updated: '2026-01-01T00:00:00Z' } });
  try {
    const result = runGateHook(sb.root, sb.vibeDir, '{}', { spawnGit: () => ({ error: new Error('no git') }) });
    assertEqual(result.code, 2);
    assertIncludes(result.stderr, 'BLOCKED');
    assertIncludes(result.stderr, path.join('evidence', 'feature-demo.md'));
  } finally {
    sb.cleanup();
  }
});

test('runGateHook predicate 2: feature.verify with a fresh receipt -> exit 0', () => {
  const sb = makeHookSandbox({ cursor: { flow: 'feature', phase: 'verify', feature: 'demo', updated: '2026-01-01T00:00:00Z' } });
  try {
    mkdirSync(path.join(sb.vibeDir, 'evidence'), { recursive: true });
    writeFileSync(path.join(sb.vibeDir, 'evidence', 'feature-demo.md'), 'commands + output\n');
    const result = runGateHook(sb.root, sb.vibeDir, '{}', { spawnGit: () => ({ error: new Error('no git') }) });
    assertEqual(result.code, 0);
  } finally {
    sb.cleanup();
  }
});

test('runGateHook predicate 2: feature.verify with no feature named -> warn-only, exit 0', () => {
  const sb = makeHookSandbox({ cursor: { flow: 'feature', phase: 'verify', feature: null, updated: '2026-01-01T00:00:00Z' } });
  try {
    const result = runGateHook(sb.root, sb.vibeDir, '{}', { spawnGit: () => ({ error: new Error('no git') }) });
    assertEqual(result.code, 0);
    assertIncludes(result.stderr, 'cannot resolve the evidence receipt');
  } finally {
    sb.cleanup();
  }
});

test('runGateHook predicate 2: quick.verify uses the fixed quick.md receipt name', () => {
  const sb = makeHookSandbox({ cursor: { flow: 'quick', phase: 'verify', feature: null, updated: '2026-01-01T00:00:00Z' } });
  try {
    const result = runGateHook(sb.root, sb.vibeDir, '{}', { spawnGit: () => ({ error: new Error('no git') }) });
    assertEqual(result.code, 2);
    assertIncludes(result.stderr, path.join('evidence', 'quick.md'));
  } finally {
    sb.cleanup();
  }
});

test('runGateHook predicate 2: a receipt older than a changed src file is stale -> exit 2', () => {
  const sb = makeHookSandbox({ cursor: { flow: 'feature', phase: 'verify', feature: 'demo', updated: '2026-01-01T00:00:00Z' } });
  try {
    mkdirSync(path.join(sb.vibeDir, 'evidence'), { recursive: true });
    const receipt = path.join(sb.vibeDir, 'evidence', 'feature-demo.md');
    writeFileSync(receipt, 'old evidence\n');
    const past = new Date('2020-01-01T00:00:00Z');
    utimesSync(receipt, past, past);

    mkdirSync(path.join(sb.dir, 'src'), { recursive: true });
    writeFileSync(path.join(sb.dir, 'src', 'app.sh'), 'new\n');

    const result = runGateHook(sb.root, sb.vibeDir, '{}', {
      spawnGit: (args) =>
        args.includes('rev-parse')
          ? { error: null, status: 0, stdout: 'true\n' }
          : { error: null, status: 0, stdout: ' M src/app.sh\n' },
    });
    assertEqual(result.code, 2);
    assertIncludes(result.stderr, 'stale');
    assertIncludes(result.stderr, 'src/app.sh');
  } finally {
    sb.cleanup();
  }
});

test('runGateHook predicate 2: a change under the evidence dir itself never counts as staleness', () => {
  const sb = makeHookSandbox({ cursor: { flow: 'feature', phase: 'verify', feature: 'demo', updated: '2026-01-01T00:00:00Z' } });
  try {
    mkdirSync(path.join(sb.vibeDir, 'evidence'), { recursive: true });
    const receipt = path.join(sb.vibeDir, 'evidence', 'feature-demo.md');
    writeFileSync(receipt, 'evidence\n');
    const past = new Date('2020-01-01T00:00:00Z');
    utimesSync(receipt, past, past);

    const result = runGateHook(sb.root, sb.vibeDir, '{}', {
      spawnGit: (args) =>
        args.includes('rev-parse')
          ? { error: null, status: 0, stdout: 'true\n' }
          : {
              error: null,
              status: 0,
              stdout: ` M ${path.posix.join('.agents/skills/vibe/evidence', 'feature-demo.md')}\n`,
            },
    });
    assertEqual(result.code, 0);
  } finally {
    sb.cleanup();
  }
});

test('runGateHook predicate 3 (warn-only): non-idle state with legal next states nudges toward set-state.sh', () => {
  const sb = makeHookSandbox({ cursor: { flow: 'feature', phase: 'impl', feature: 'demo', updated: '2026-01-01T00:00:00Z' } });
  try {
    const result = runGateHook(sb.root, sb.vibeDir, '{}', { spawnGit: () => ({ error: new Error('no git') }) });
    assertEqual(result.code, 0);
    assertIncludes(result.stderr, 'still in feature.impl');
    assertIncludes(readFileSync(sb.warnLogPath, 'utf8'), 'gate:');
  } finally {
    sb.cleanup();
  }
});

test('runGateHook predicate 3: idle never nudges', () => {
  const sb = makeHookSandbox();
  try {
    const result = runGateHook(sb.root, sb.vibeDir, '{}', { spawnGit: () => ({ error: new Error('no git') }) });
    assertEqual(result.code, 0);
    assertEqual(result.stderr, '');
  } finally {
    sb.cleanup();
  }
});

test('runGateHook: corrupt cursor degrades to idle, never throws or blocks', () => {
  const sb = makeHookSandbox();
  try {
    writeFileSync(sb.cursorPath, '{ not json');
    const result = runGateHook(sb.root, sb.vibeDir, '{}', { spawnGit: () => ({ error: new Error('no git') }) });
    assertEqual(result.code, 0);
  } finally {
    sb.cleanup();
  }
});

test('runGateHook: integration — real git repo, non-verify state, no changes -> predicate 3 warns and relays', () => {
  const sb = makeHookSandbox({
    gitInit: true,
    cursor: { flow: 'feature', phase: 'impl', feature: 'demo', updated: '2026-01-01T00:00:00Z' },
  });
  try {
    const result = runGateHook(sb.root, sb.vibeDir, '{}', {});
    assertEqual(result.code, 0);
    assertIncludes(readFileSync(sb.warnLogPath, 'utf8'), 'gate:');
  } finally {
    sb.cleanup();
  }
});

// ---------------------------------------------------------------------------
// CLI wiring — `vibe hook <name>` end to end, real stdin, no injected spawns.
// ---------------------------------------------------------------------------

test('CLI: `vibe hook session-start-doctrine` end to end', () => {
  const sb = makeHookSandbox();
  try {
    const result = runCommand(
      process.execPath,
      [path.join(REPO_ROOT, 'flow', 'engine', 'cli.mjs'), 'hook', 'session-start-doctrine'],
      { env: { CLAUDE_PROJECT_DIR: sb.root }, input: '' },
    );
    assertEqual(result.code, 0);
    assertIncludes(result.stdout, 'Cursor: idle.');
  } finally {
    sb.cleanup();
  }
});

test('CLI: `vibe hook pre-tool-use-guard` end to end blocks a state.json edit (exit 2)', () => {
  const sb = makeHookSandbox();
  try {
    const result = runCommand(
      process.execPath,
      [path.join(REPO_ROOT, 'flow', 'engine', 'cli.mjs'), 'hook', 'pre-tool-use-guard'],
      {
        env: { CLAUDE_PROJECT_DIR: sb.root },
        input: JSON.stringify({ tool_name: 'Write', tool_input: { file_path: '.agents/skills/vibe/state.json' } }),
      },
    );
    assertEqual(result.code, 2);
    assertIncludes(result.stderr, 'BLOCKED');
  } finally {
    sb.cleanup();
  }
});

test('CLI: `vibe hook stop-gate` end to end blocks a receipt-less feature.verify (exit 2)', () => {
  const sb = makeHookSandbox({ cursor: { flow: 'feature', phase: 'verify', feature: 'demo', updated: '2026-01-01T00:00:00Z' } });
  try {
    const result = runCommand(
      process.execPath,
      [path.join(REPO_ROOT, 'flow', 'engine', 'cli.mjs'), 'hook', 'stop-gate'],
      { env: { CLAUDE_PROJECT_DIR: sb.root }, input: '{}' },
    );
    assertEqual(result.code, 2);
    assertIncludes(result.stderr, 'BLOCKED');
  } finally {
    sb.cleanup();
  }
});

test('CLI: `vibe hook user-prompt-submit-inject` end to end emits idle orders', () => {
  const sb = makeHookSandbox();
  try {
    const result = runCommand(
      process.execPath,
      [path.join(REPO_ROOT, 'flow', 'engine', 'cli.mjs'), 'hook', 'user-prompt-submit-inject'],
      { env: { CLAUDE_PROJECT_DIR: sb.root }, input: '{}' },
    );
    assertEqual(result.code, 0);
    assertMatch(result.stdout, /^state=idle/);
  } finally {
    sb.cleanup();
  }
});

test('CLI: `vibe hook <unknown>` degrades to a silent no-op, never breaks', () => {
  const sb = makeHookSandbox();
  try {
    const result = runCommand(process.execPath, [path.join(REPO_ROOT, 'flow', 'engine', 'cli.mjs'), 'hook', 'not-a-real-hook'], {
      env: { CLAUDE_PROJECT_DIR: sb.root },
      input: '',
    });
    assertEqual(result.code, 0);
    assertEqual(result.stdout, '');
  } finally {
    sb.cleanup();
  }
});
