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
  mkdirSync,
  writeFileSync,
  readFileSync,
  rmSync,
  utimesSync,
  statSync,
} from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import {
  test,
  assert,
  assertEqual,
  assertMatch,
  assertIncludes,
  runCommand,
  makeHookSandbox,
  skip,
} from './run.mjs';
import {
  runDoctrineHook,
  runInjectHook,
  runGuardHook,
  runGateHook,
} from '../commands/hook.mjs';

const REPO_ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..', '..', '..');

// ---------------------------------------------------------------------------
// Fixture — makeHookSandbox() now lives in run.mjs so parity.test.mjs's
// guard/gate ORACLE differential builds the byte-identical install-shaped
// layout the frozen bash hooks self-locate through (js-core/8 final review,
// I3). Two hand-copied fixtures would let the two suites be honest about
// different layouts.
// ---------------------------------------------------------------------------

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

// js-core/8 final review, M1 — this test used to report `ok` with the exclusion
// it names DELETED. Its fixture reported the RECEIPT ITSELF as the changed path,
// so `st.mtimeMs > receiptStat.mtimeMs` compared the file to itself and was
// false either way; the exclusion was entirely unprotected, in this suite and in
// the bash one.
//
// A discriminating fixture needs a DIFFERENT file under evidence/ that is
// genuinely NEWER than the receipt being checked — the real shape being a
// quick.md written after feature-demo.md in the same session. Without the
// exclusion that blocks a *.verify state forever: a session wedge, which is why
// the exclusion is worth a fixture that can actually see it.
test('runGateHook predicate 2: a DIFFERENT, NEWER file under the evidence dir never counts as staleness', () => {
  const sb = makeHookSandbox({ cursor: { flow: 'feature', phase: 'verify', feature: 'demo', updated: '2026-01-01T00:00:00Z' } });
  try {
    mkdirSync(path.join(sb.vibeDir, 'evidence'), { recursive: true });
    const receipt = path.join(sb.vibeDir, 'evidence', 'feature-demo.md');
    writeFileSync(receipt, 'evidence\n');
    const receiptTs = new Date('2020-01-01T00:00:00Z');
    utimesSync(receipt, receiptTs, receiptTs);

    // A sibling receipt, written AFTER this one — strictly newer, and reported
    // by porcelain under its own name.
    const sibling = path.join(sb.vibeDir, 'evidence', 'quick.md');
    writeFileSync(sibling, 'a later receipt\n');
    const siblingTs = new Date('2020-06-01T00:00:00Z');
    utimesSync(sibling, siblingTs, siblingTs);

    const porcelain = ` M ${path.posix.join('.agents/skills/vibe/evidence', 'quick.md')}\n`;
    const spawnGit = (args) =>
      args.includes('rev-parse')
        ? { error: null, status: 0, stdout: 'true\n' }
        : { error: null, status: 0, stdout: porcelain };

    const result = runGateHook(sb.root, sb.vibeDir, '{}', { spawnGit });
    assertEqual(result.code, 0, 'a newer SIBLING receipt must never make the gate call its own receipt stale');

    // Discriminating control: the identical fixture with the path moved OUT of
    // the evidence dir does block — so the exit 0 above is the exclusion firing,
    // not the mtime comparison quietly failing to see anything.
    const outsidePath = path.join(sb.dir, 'notes.md');
    writeFileSync(outsidePath, 'a later note\n');
    utimesSync(outsidePath, siblingTs, siblingTs);
    const control = runGateHook(sb.root, sb.vibeDir, '{}', {
      spawnGit: (args) =>
        args.includes('rev-parse')
          ? { error: null, status: 0, stdout: 'true\n' }
          : { error: null, status: 0, stdout: ' M notes.md\n' },
    });
    assertEqual(control.code, 2, 'control: the same mtime relationship OUTSIDE evidence/ must block');
  } finally {
    sb.cleanup();
  }
});

// js-core/8 final review, M5 — every stale-receipt fixture in both suites pins
// the receipt to 2000-01-01 or 2020-01-01, so a mutation requiring the changed
// file to be a hundred SECONDS newer survived the whole suite. The real
// timescale is sub-second: the receipt is written and a file is touched moments
// later in the same turn. These three cases pin the comparison at millisecond
// granularity and in both directions, including the exact-tie boundary.
//
// Deliberately engine-only (injected porcelain, no bash): the ORACLE's `-nt`
// compares st_mtim, whose resolution is not guaranteed on every platform this
// ships to — stock bash 3.2 on macOS is the case that matters. Asserting a 20 ms
// delta against the oracle differential would be pinning the RUNNER's timestamp
// resolution, not the port. Logged as residual risk instead.
function gateWithPorcelain(sb, porcelain) {
  return runGateHook(sb.root, sb.vibeDir, '{}', {
    spawnGit: (args) =>
      args.includes('rev-parse')
        ? { error: null, status: 0, stdout: 'true\n' }
        : { error: null, status: 0, stdout: porcelain },
  });
}

// epochSeconds may be fractional — utimesSync takes seconds, mtimeMs reports
// milliseconds, so a 20 ms delta is expressible exactly.
function makeStalenessFixture(receiptEpoch, fileEpoch) {
  const sb = makeHookSandbox({ cursor: { flow: 'feature', phase: 'verify', feature: 'demo', updated: '2026-01-01T00:00:00Z' } });
  mkdirSync(path.join(sb.vibeDir, 'evidence'), { recursive: true });
  const receipt = path.join(sb.vibeDir, 'evidence', 'feature-demo.md');
  writeFileSync(receipt, 'evidence\n');
  utimesSync(receipt, receiptEpoch, receiptEpoch);
  mkdirSync(path.join(sb.dir, 'src'), { recursive: true });
  const changed = path.join(sb.dir, 'src', 'app.sh');
  writeFileSync(changed, 'code\n');
  utimesSync(changed, fileEpoch, fileEpoch);

  // Self-attack on these tests: a filesystem whose mtime granularity is coarser
  // than the delta (an old HFS+ volume, some network mounts) collapses the two
  // stamps and the assertion below becomes a statement about the RUNNER, not the
  // code — a red build nobody can act on. Assert the fixture actually
  // materialised the delta it asks about, and skip honestly if the filesystem
  // could not express it. Never a bare `return`: that reports `ok` and is
  // indistinguishable from a pin that ran.
  const want = Math.round((fileEpoch - receiptEpoch) * 1000);
  const got = Math.round(statSync(changed).mtimeMs - statSync(receipt).mtimeMs);
  if (got !== want) {
    skip(`filesystem mtime granularity cannot express a ${want} ms delta (observed ${got} ms) — this fixture would test the runner, not the gate`);
  }
  return sb;
}

const RECEIPT_EPOCH = 1767225600; // 2026-01-01T00:00:00Z, as seconds

test('runGateHook predicate 2: staleness is a MILLISECOND comparison — 20 ms newer already blocks', () => {
  const sb = makeStalenessFixture(RECEIPT_EPOCH, RECEIPT_EPOCH + 0.02);
  try {
    const result = gateWithPorcelain(sb, ' M src/app.sh\n');
    assertEqual(result.code, 2, 'a file 20 ms newer than the receipt is stale — the real timescale of a verify turn');
    assertIncludes(result.stderr, 'stale');
  } finally {
    sb.cleanup();
  }
});

test('runGateHook predicate 2: staleness is directional — 20 ms OLDER than the receipt never blocks', () => {
  const sb = makeStalenessFixture(RECEIPT_EPOCH, RECEIPT_EPOCH - 0.02);
  try {
    assertEqual(gateWithPorcelain(sb, ' M src/app.sh\n').code, 0);
  } finally {
    sb.cleanup();
  }
});

test('runGateHook predicate 2: an exact mtime TIE is not stale (strictly greater, matching bash -nt)', () => {
  const sb = makeStalenessFixture(RECEIPT_EPOCH, RECEIPT_EPOCH);
  try {
    assertEqual(
      gateWithPorcelain(sb, ' M src/app.sh\n').code,
      0,
      'bash `-nt` is strictly greater; a `>=` here would block on every file written in the receipt\'s own instant',
    );
  } finally {
    sb.cleanup();
  }
});

// js-core/8 final review — the 36-mutation sweep's fifth survivor was `skip
// deletions`, classified EQUIVALENT on the reasoning that statSync throws on a
// deleted path and `continue`s anyway. That holds only while a D-flagged path is
// actually absent, and git produces the opposite routinely: `git rm --cached
// <file>` leaves an INDEX deletion (`D `) for a file that still exists on disk
// (with a separate `??` line for the same path). With the skip removed, that
// existing, newer file blocks a *.verify state via a line the oracle explicitly
// ignores. So it is not equivalent — it is oracle-matching behaviour with a
// reachable fixture, and it is pinned here rather than waived.
test('runGateHook predicate 2: a D-flagged porcelain line is skipped even when the file EXISTS and is newer', () => {
  const sb = makeHookSandbox({ cursor: { flow: 'feature', phase: 'verify', feature: 'demo', updated: '2026-01-01T00:00:00Z' } });
  try {
    mkdirSync(path.join(sb.vibeDir, 'evidence'), { recursive: true });
    const receipt = path.join(sb.vibeDir, 'evidence', 'feature-demo.md');
    writeFileSync(receipt, 'evidence\n');
    const past = new Date('2020-01-01T00:00:00Z');
    utimesSync(receipt, past, past);

    // Present on disk, and much newer than the receipt.
    writeFileSync(path.join(sb.dir, 'unstaged.md'), 'still here\n');

    assertEqual(
      gateWithPorcelain(sb, 'D  unstaged.md\n').code,
      0,
      'an index-deletion line must be skipped by xy, not by statSync happening to throw',
    );

    // Control: the identical file reported under a NON-deletion status does
    // block — so the exit 0 above is the D-skip, not an inert fixture.
    assertEqual(
      gateWithPorcelain(sb, ' M unstaged.md\n').code,
      2,
      'control: the same existing, newer file blocks when its status is not a deletion',
    );
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
