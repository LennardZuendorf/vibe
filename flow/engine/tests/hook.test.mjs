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
  existsSync,
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
  makeContentSandbox,
  skip,
} from './run.mjs';
import {
  runDoctrineHook,
  runInjectHook,
  runGuardHook,
  runGateHook,
  collapseWarnLines,
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

// The relay log is created lazily, so "nothing was queued" is spelled either as
// an empty file or as no file at all — both mean the same thing to the drain.
function readRelay(sb) {
  try {
    return readFileSync(sb.warnLogPath, 'utf8');
  } catch {
    return '';
  }
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
    // R4: SessionStart output replays verbatim on --resume, so no live state
    // may ride it. Population floor above, negative here.
    assert(!result.stdout.includes('Cursor:'), 'the doctrine hook must not carry the cursor');
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
// user-prompt-submit-inject — the trigger-classed payload (inject-triggers/4)
//
// The cadences: level every turn, edge only on the turn after the cursor
// moved, event only on a turn that carried one. What these cases protect is
// BYTE-STABILITY — two quiet turns in the same state emit identical bytes, so
// the prompt cache holds — and its converse, that a cursor move still delivers
// the full payload exactly once.
// ---------------------------------------------------------------------------

const TRIGGER_DEFAULTS = {
  version: 1,
  channels: {
    'user-prompt.level': { render: 'summary', trigger: 'level', budget: 2, blocks: ['flow.level'] },
    'user-prompt.edge': { render: 'summary', trigger: 'edge', budget: 15, blocks: ['flow.edge'] },
    'user-prompt.event': { render: 'summary', trigger: 'event', blocks: ['flow.event'] },
    'user-prompt': { render: 'summary', blocks: ['style.rule'] },
  },
};

const TRIGGER_BLOCKS = {
  'flow/level.md': '---\nid: flow.level\nchannels: [user-prompt.level]\n---\nLEVEL state={{state}} · {{transition}}\n',
  'flow/edge.md': '---\nid: flow.edge\nchannels: [user-prompt.edge]\n---\nEDGE {{orders}}\n',
  'flow/event.md': '---\nid: flow.event\nchannels: [user-prompt.event]\n---\nEVENT something happened\n',
  'style/rule.md': '---\nid: style.rule\nchannels: [user-prompt]\n---\nSTYLE RULE\n',
};

function makeTriggerSandbox(cursor) {
  return makeContentSandbox({ defaults: TRIGGER_DEFAULTS, blocks: TRIGGER_BLOCKS, cursor });
}

// A quiet turn: no drift, no queued warnings — the state the byte-stability
// contract is about.
function quietTurn(sb) {
  return runInjectHook(sb.dir, sb.vibeDir, sb.skillsDir, { spawnInfer: () => ({ error: null, stdout: '' }) });
}

function countOccurrences(haystack, needle) {
  return haystack.split(needle).length - 1;
}

test('inject: two quiet turns in the same state are BYTE-IDENTICAL (the prompt cache contract)', () => {
  const sb = makeTriggerSandbox();
  try {
    // Turn 1 is the first inject ever, so it is an edge turn by definition
    // (nothing recorded yet) — the steady state starts at turn 2.
    const first = quietTurn(sb);
    assertIncludes(first.stdout, 'EDGE ', 'precondition: the first inject after install carries the edge payload');

    const second = quietTurn(sb);
    const third = quietTurn(sb);
    assertEqual(second.stdout, third.stdout, 'two quiet turns in one state must not differ by a single byte');
    assert(!second.stdout.includes('EDGE '), 'the edge payload must not ride a turn where nothing moved');
    assertIncludes(second.stdout, 'LEVEL state=idle', 'the level line rides every turn');
    assertIncludes(second.stdout, 'STYLE RULE', 'and so do the standing rules');
    assertEqual(
      second.stdout.replace(/\n$/, '').split('\n').length,
      2,
      'the steady-state payload is the level line plus the standing rules, nothing else',
    );
  } finally {
    sb.cleanup();
  }
});

test('inject: the turn AFTER a transition carries the edge payload exactly once', () => {
  const sb = makeTriggerSandbox();
  try {
    quietTurn(sb); // first-ever inject: records the idle cursor
    assert(!quietTurn(sb).stdout.includes('EDGE '), 'precondition: the cursor is settled');

    writeCursor(sb, { flow: 'feature', phase: 'impl', feature: 'demo', updated: '2026-01-02T00:00:00Z' });

    const moved = quietTurn(sb);
    assertEqual(countOccurrences(moved.stdout, 'EDGE '), 1, 'the edge payload rides the turn after the move, once');
    assertIncludes(moved.stdout, 'LEVEL state=feature.impl');

    const after = quietTurn(sb);
    assert(!after.stdout.includes('EDGE '), 'and not again while the cursor sits still');
    assertEqual(quietTurn(sb).stdout, after.stdout, 'the new state has its own byte-stable steady payload');
  } finally {
    sb.cleanup();
  }
});

test('inject: the recorded key is the cursor the payload was composed FOR, written once per turn', () => {
  const sb = makeTriggerSandbox({ flow: 'feature', phase: 'impl', feature: 'demo', updated: '2026-01-01T00:00:00Z' });
  try {
    quietTurn(sb);
    assertEqual(readFileSync(path.join(sb.dir, '.vibe', 'last-inject'), 'utf8'), 'feature.impl demo\n');
  } finally {
    sb.cleanup();
  }
});

test('inject: a BROKEN edge marker fails open — the edge payload repeats rather than being lost', () => {
  const sb = makeTriggerSandbox();
  try {
    // A directory where the marker file belongs: unreadable and unwritable, so
    // neither the compare nor the record can succeed on any turn.
    mkdirSync(path.join(sb.dir, '.vibe', 'last-inject'), { recursive: true });
    for (const turn of [1, 2]) {
      const result = quietTurn(sb);
      assertEqual(result.code, 0, `turn ${turn} must still exit 0`);
      assertIncludes(result.stdout, 'EDGE ', `turn ${turn}: fail open means one payload too many, never one too few`);
    }
  } finally {
    sb.cleanup();
  }
});

test('inject: the event channel rides ONLY a turn that carried an event', () => {
  const sb = makeTriggerSandbox();
  try {
    quietTurn(sb);
    assert(!quietTurn(sb).stdout.includes('EVENT '), 'a quiet turn carries no event text');

    const drifted = runInjectHook(sb.dir, sb.vibeDir, sb.skillsDir, {
      spawnInfer: () => ({ error: null, stdout: 'drift:feature.impl:src edits in idle\n' }),
    });
    assertIncludes(drifted.stdout, 'EVENT ', 'a drift nudge is an event');

    writeFileSync(sb.warnLogPath, 'guard: something (warn-only)\n');
    const warned = quietTurn(sb);
    assertIncludes(warned.stdout, 'EVENT ', 'a queued warning is an event too');
    assert(!quietTurn(sb).stdout.includes('EVENT '), 'and the turn after it is quiet again');
  } finally {
    sb.cleanup();
  }
});

test('inject: emission order is drift, level, edge, other prompt channels, warns', () => {
  const sb = makeTriggerSandbox();
  try {
    writeFileSync(sb.warnLogPath, 'guard: something (warn-only)\n');
    const result = runInjectHook(sb.dir, sb.vibeDir, sb.skillsDir, {
      spawnInfer: () => ({ error: null, stdout: 'drift:feature.impl:src edits in idle\n' }),
    });
    const lines = result.stdout.split('\n').filter(Boolean);
    assertMatch(lines[0], /^vibe-drift: /);
    assertMatch(lines[1], /^LEVEL /);
    assertMatch(lines[2], /^EDGE /);
    assertMatch(lines[3], /^EVENT /);
    assertEqual(lines[4], 'STYLE RULE');
    assertMatch(lines[5], /^vibe-warn: /);
  } finally {
    sb.cleanup();
  }
});

test('inject: the edge channel CARRIES the orders — they are never emitted twice on an edge turn', () => {
  const sb = makeTriggerSandbox();
  try {
    const first = quietTurn(sb);
    // 'no active flow' is text only the idle ORDERS carry, so counting it
    // separates "the edge payload rendered {{orders}}" from "the hook also
    // emitted the raw orders alongside it".
    assertEqual(countOccurrences(first.stdout, 'no active flow'), 1, 'the orders appear once, inside the edge payload');
    assert(!first.stdout.startsWith('state='), 'the raw orders no longer lead the payload when a channel owns them');
  } finally {
    sb.cleanup();
  }
});

// BACKWARD COMPATIBILITY. Two shapes must behave exactly as they did before
// this feature: a project whose content tree has no edge channel, and an
// install with no content tree at all. Both keep the raw orders on every turn.
test('inject: a content tree with NO edge channel keeps the raw orders on every turn', () => {
  const sb = makeContentSandbox({
    defaults: { version: 1, channels: { 'user-prompt': { render: 'summary', blocks: ['style.rule'] } } },
    blocks: { 'style/rule.md': TRIGGER_BLOCKS['style/rule.md'] },
  });
  try {
    const first = quietTurn(sb);
    const second = quietTurn(sb);
    assertMatch(first.stdout, /^state=idle/, 'the orders still lead the payload');
    assertEqual(first.stdout, second.stdout, 'and still ride every turn, unchanged');
    assertIncludes(second.stdout, 'STYLE RULE');
    // The inject IS recorded even with no edge channel (deferred fix from unit
    // 4's review): gating the record on the edge channel having produced text
    // left the marker permanently absent, so cursorChangedSince() answered
    // "moved" forever — a cursor that is always wrong rather than sometimes.
    assertEqual(
      readFileSync(path.join(sb.dir, '.vibe', 'last-inject'), 'utf8'),
      'idle\n',
      'every composed turn records the cursor it was composed for, edge channel or not',
    );
  } finally {
    sb.cleanup();
  }
});

// The take-over is decided on EVIDENCE — what the edge channel actually
// composed — not on what its config declared (fix round 1, Critical). Both
// cases below have a non-empty declared block list, and in both the orders
// would otherwise vanish from every turn, forever, at exit 0.
test('inject: a MISSING edge block never takes the orders away (declared is not delivered)', () => {
  const sb = makeTriggerSandbox();
  try {
    // Precondition: this sandbox's edge channel works, so the loss below is
    // the missing block and not an inert fixture.
    assertIncludes(quietTurn(sb).stdout, 'EDGE ', 'precondition: the edge channel delivers the orders');

    rmSync(path.join(sb.vibeDir, 'content', 'blocks', 'flow', 'edge.md'));
    for (const turn of [1, 2]) {
      const result = quietTurn(sb);
      assertEqual(result.code, 0, `turn ${turn}: still exit 0`);
      assertIncludes(
        result.stdout,
        'no active flow',
        `turn ${turn}: the orders come back when nothing else delivers them`,
      );
    }
  } finally {
    sb.cleanup();
  }
});

test('inject: an edge block WITHOUT {{orders}} never takes the orders away either', () => {
  const sb = makeContentSandbox({
    defaults: {
      version: 1,
      channels: {
        'user-prompt.edge': { render: 'summary', trigger: 'edge', blocks: ['own.edge'] },
      },
    },
    // A project's own edge block: it renders fine, it just does not carry the
    // orders. `render --check` errors on this shape; the hook must survive it
    // regardless, because a live session never runs the lint.
    blocks: { 'own/edge.md': '---\nid: own.edge\nchannels: [user-prompt.edge]\n---\nPROJECT EDGE TEXT\n' },
  });
  try {
    const first = quietTurn(sb);
    assertIncludes(first.stdout, 'PROJECT EDGE TEXT', 'the project block still rides the edge turn');
    assertIncludes(first.stdout, 'no active flow', 'and the orders are still emitted, because that block lacks them');
    assertIncludes(quietTurn(sb).stdout, 'no active flow', 'on every turn, not just the first');
  } finally {
    sb.cleanup();
  }
});

test('inject: an EMPTY edge channel does not claim the orders (a channel that composes nothing carries nothing)', () => {
  const sb = makeContentSandbox({
    defaults: {
      version: 1,
      channels: {
        'user-prompt.edge': { render: 'summary', trigger: 'edge', blocks: [] },
        'user-prompt': { render: 'summary', blocks: ['style.rule'] },
      },
    },
    blocks: { 'style/rule.md': TRIGGER_BLOCKS['style/rule.md'] },
  });
  try {
    assertMatch(quietTurn(sb).stdout, /^state=idle/);
  } finally {
    sb.cleanup();
  }
});

test('inject: a MALFORMED project config degrades to the pre-content payload, never throws', () => {
  const sb = makeContentSandbox({
    defaults: TRIGGER_DEFAULTS,
    blocks: TRIGGER_BLOCKS,
    project: '{ not json at all',
  });
  try {
    const result = quietTurn(sb);
    assertEqual(result.code, 0);
    // The shipped layer still resolves, so the cadences still work — the point
    // is that a broken layer costs output, never the turn.
    assertIncludes(result.stdout, 'LEVEL state=idle');
  } finally {
    sb.cleanup();
  }
});

// ---------------------------------------------------------------------------
// The warnings relay is BOUNDED and DEDUPLICATED (inject-triggers, R5).
//
// The drain writes into an append-only prompt stream, so its size is not a
// tidiness question: every line it emits is re-read on every later turn of the
// session. A guard tripped in a loop used to cost one line per trip, forever.
// ---------------------------------------------------------------------------

function warnLinesOf(result) {
  return result.stdout.split('\n').filter((l) => l.startsWith('vibe-warn:'));
}

test('relay: 40 identical queued warnings collapse to ONE line carrying (x40), and the log is truncated', () => {
  const sb = makeHookSandbox();
  try {
    const msg = 'guard: outside an impl state (warn-only)';
    writeFileSync(sb.warnLogPath, `${msg}\n`.repeat(40));
    const result = quietTurn({ ...sb, dir: sb.root });
    const warns = warnLinesOf(result);
    assertEqual(warns.length, 1, `40 copies must collapse to one line, got ${JSON.stringify(warns)}`);
    assertEqual(warns[0], `vibe-warn: ${msg} (x40)`);
    assertEqual(readFileSync(sb.warnLogPath, 'utf8'), '', 'the log is truncated exactly as before');
  } finally {
    sb.cleanup();
  }
});

test('relay: a line queued exactly once carries NO count suffix', () => {
  const sb = makeHookSandbox();
  try {
    writeFileSync(sb.warnLogPath, 'guard: one and only (warn-only)\n');
    assertEqual(warnLinesOf(quietTurn({ ...sb, dir: sb.root })), ['vibe-warn: guard: one and only (warn-only)']);
  } finally {
    sb.cleanup();
  }
});

test('relay: 50 DISTINCT queued warnings emit 10 lines plus a "+40 more" trailer', () => {
  const sb = makeHookSandbox();
  try {
    const queued = Array.from({ length: 50 }, (_, i) => `guard: distinct warning ${i} (warn-only)`);
    writeFileSync(sb.warnLogPath, `${queued.join('\n')}\n`);
    const warns = warnLinesOf(quietTurn({ ...sb, dir: sb.root }));
    assertEqual(warns.length, 11, 'ten lines plus one trailer');
    assertEqual(warns[0], 'vibe-warn: guard: distinct warning 0 (warn-only)', 'first-seen order is preserved');
    assertEqual(warns[9], 'vibe-warn: guard: distinct warning 9 (warn-only)', 'the cap keeps the OLDEST ten');
    assertEqual(warns[10], 'vibe-warn: +40 more');
    assertEqual(readFileSync(sb.warnLogPath, 'utf8'), '', 'truncated: what the cap dropped is dropped for good');
  } finally {
    sb.cleanup();
  }
});

test('relay: exactly 10 distinct warnings emit 10 lines and NO trailer (the cap is not off by one)', () => {
  const sb = makeHookSandbox();
  try {
    const queued = Array.from({ length: 10 }, (_, i) => `guard: warning ${i} (warn-only)`);
    writeFileSync(sb.warnLogPath, `${queued.join('\n')}\n`);
    const warns = warnLinesOf(quietTurn({ ...sb, dir: sb.root }));
    assertEqual(warns.length, 10);
    assert(!warns.some((l) => l.includes('more')), `no trailer at the cap, got ${JSON.stringify(warns)}`);
  } finally {
    sb.cleanup();
  }
});

test('relay: dedupe runs BEFORE the cap — 400 lines of 3 distinct warnings emit 3, not 10', () => {
  const sb = makeHookSandbox();
  try {
    const three = ['guard: a (warn-only)', 'guard: b (warn-only)', 'guard: c (warn-only)'];
    let body = '';
    for (let i = 0; i < 400; i += 1) body += `${three[i % 3]}\n`;
    const warns = warnLinesOf((writeFileSync(sb.warnLogPath, body), quietTurn({ ...sb, dir: sb.root })));
    assertEqual(warns, [
      'vibe-warn: guard: a (warn-only) (x134)',
      'vibe-warn: guard: b (warn-only) (x133)',
      'vibe-warn: guard: c (warn-only) (x133)',
    ]);
  } finally {
    sb.cleanup();
  }
});

test('relay: collapseWarnLines is total — empty, blank-only and trailing-newline-less input never throw', () => {
  assertEqual(collapseWarnLines(''), []);
  assertEqual(collapseWarnLines('\n\n\n'), []);
  assertEqual(collapseWarnLines('a\na'), ['a (x2)']);
  assertEqual(collapseWarnLines(undefined), []);
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
// pre-tool-use-guard — the verdict is answered IN-PROCESS (inject-triggers/5).
//
// Unit 2 made detect-context.sh delegate `decide` to `vibe policy decide`, so a
// hook that spawned that script to reach a verdict was spawning node again:
// node -> bash -> node per guarded Edit. The guard now calls policy.mjs
// directly and keeps the spawn as the fallback for UNUSABLE policy data.
//
// What these cases have to prove, in order: (1) the in-process branch really
// runs (the spawn is never reached), (2) its verdicts are the SAME verdicts the
// bash branch gives — every guarded path x every machine state, which is unit
// 2's differential replayed through the hook — and (3) unusable data still
// falls back instead of silently answering `allow`, which is what would make
// every hard block disappear.
// ---------------------------------------------------------------------------

const POLICY_SRC = path.join(REPO_ROOT, 'flow', 'content', 'policy.json');

function withPolicy(sb, body) {
  const dir = path.join(sb.vibeDir, 'content');
  mkdirSync(dir, { recursive: true });
  writeFileSync(path.join(dir, 'policy.json'), body ?? readFileSync(POLICY_SRC, 'utf8'));
}

function guard(sb, filePath, opts = {}) {
  return runGuardHook(sb.root, JSON.stringify({ tool_name: 'Write', tool_input: { file_path: filePath } }), opts);
}

// A spawnDecide that records whether it was reached and answers a verdict no
// policy would ever produce, so a result carrying it proves the fallback ran.
function spyDecide(stdout = 'allow\n') {
  const calls = [];
  return { calls, spawnDecide: (args) => (calls.push(args), { error: null, status: 0, stdout }) };
}

test('guard: the write-invariant verdict is answered in-process — the bash spawn is never reached', () => {
  const sb = makeHookSandbox();
  try {
    withPolicy(sb);
    const spy = spyDecide('allow\n');
    const result = guard(sb, '.spec/lessons.md', { spawnDecide: spy.spawnDecide });
    assertEqual(result.code, 2, 'lessons.md at idle is a hard block');
    assertIncludes(result.stderr, 'BLOCKED');
    assertEqual(spy.calls.length, 0, 'no node -> bash -> node round trip when the policy is usable');
  } finally {
    sb.cleanup();
  }
});

test('guard: in-process verdicts translate exactly as before — block -> 2, warn -> 0 + relay, allow -> silent', () => {
  const sb = makeHookSandbox();
  try {
    withPolicy(sb);

    const blocked = guard(sb, '.agents/skills/vibe/state.json');
    assertEqual(blocked.code, 2);
    assertIncludes(blocked.stderr, 'BLOCKED');
    assertIncludes(blocked.stderr, 'set-state.sh');

    rmSync(sb.warnLogPath, { force: true });
    const warned = guard(sb, 'src/x.sh');
    assertEqual(warned.code, 0);
    assertIncludes(warned.stderr, 'vibe-guard: warn —');
    assertIncludes(readRelay(sb), 'guard:');

    rmSync(sb.warnLogPath, { force: true });
    const allowed = guard(sb, 'notes/scratch.md');
    assertEqual(allowed.code, 0);
    assertEqual(allowed.stderr, '');
    assertEqual(readRelay(sb), '', 'an allow queues nothing');
  } finally {
    sb.cleanup();
  }
});

// The differential: in-process (policy present) vs the bash branch of the real
// detect-context.sh (policy removed, so the script cannot delegate either).
// Same paths, same states, same fixture — the only variable is which branch
// answered.
test('guard: in-process and bash-branch verdicts agree on every guarded path x every machine state', () => {
  const machine = JSON.parse(readFileSync(path.join(REPO_ROOT, 'flow', 'state-machine.json'), 'utf8'));
  const states = Object.keys(machine.states);
  const paths = [
    '.spec/lessons.md',
    '.spec/product.md',
    '.spec/tech.md',
    '.spec/design.md',
    '.spec/plan.md',
    '.agents/skills/vibe/state.json',
    '.spec/features/demo/product.md',
    'AGENTS.md',
    'CLAUDE.md',
    'src/app.ts',
    'tests/app.test.ts',
    'README.md',
    './.spec/lessons.md',
  ];
  assert(states.length >= 10 && paths.length >= 13, 'population floor: the matrix must span the real state list');

  const sb = makeHookSandbox();
  const divergences = [];
  let compared = 0;
  try {
    for (const key of states) {
      const [flow, phase = flow] = key.split('.');
      writeCursor(sb, { flow, phase, feature: 'demo', updated: '2026-01-01T00:00:00Z' });
      for (const p of paths) {
        withPolicy(sb);
        rmSync(sb.warnLogPath, { force: true });
        const inproc = guard(sb, p);
        const inprocLog = readRelay(sb);

        rmSync(path.join(sb.vibeDir, 'content', 'policy.json'), { force: true });
        rmSync(sb.warnLogPath, { force: true });
        const bash = guard(sb, p);
        const bashLog = readRelay(sb);

        compared += 1;
        if (inproc.code !== bash.code || inproc.stderr !== bash.stderr || inprocLog !== bashLog) {
          divergences.push(
            `${key} x ${p}: in-process rc=${inproc.code} ${JSON.stringify(inproc.stderr)} ${JSON.stringify(inprocLog)} ` +
              `| bash rc=${bash.code} ${JSON.stringify(bash.stderr)} ${JSON.stringify(bashLog)}`,
          );
        }
      }
    }
    assertEqual(divergences, [], `the controller ruling changed a verdict:\n${divergences.join('\n')}`);
    assertEqual(compared, states.length * paths.length, 'every cell was actually compared');
  } finally {
    sb.cleanup();
  }
});

// Unusable policy data must FALL BACK, never answer. `decide` over zero rules
// says `allow` for every path on earth, so answering from one would delete every
// hard block silently — the exact failure `vibe policy decide`'s exit 2 exists
// to prevent.
for (const [label, body] of [
  ['a truncated file', '{ "version": 1, "rules": ['],
  ['a version this engine does not understand', '{"version":99,"rules":[{"id":"x","match":["a"],"arms":[{"states":"*","verdict":"block","reason":"r"}]}]}'],
  ['an empty rule set', '{"version":1,"rules":[]}'],
  ['no rules key at all', '{"version":1}'],
]) {
  test(`guard: ${label} falls back to the bash branch instead of answering allow`, () => {
    const sb = makeHookSandbox();
    try {
      withPolicy(sb, body);
      const spy = spyDecide('block:the fallback answered\n');
      const result = guard(sb, '.spec/lessons.md', { spawnDecide: spy.spawnDecide });
      assertEqual(spy.calls.length, 1, 'the fallback must be reached exactly once');
      assertEqual(result.code, 2, 'and its verdict is the one that counts');
      assertIncludes(result.stderr, 'the fallback answered');
    } finally {
      sb.cleanup();
    }
  });
}

test('guard: unusable policy + the REAL detect-context.sh still hard-blocks (the fallback is not theoretical)', () => {
  const sb = makeHookSandbox();
  try {
    withPolicy(sb, '{"version":1,"rules":[]}');
    const result = guard(sb, '.agents/skills/vibe/state.json');
    assertEqual(result.code, 2, 'an empty rule set must not delete the cursor block');
    assertIncludes(result.stderr, 'BLOCKED');
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

// ---------------------------------------------------------------------------
// inject-triggers/6 — the harness must not stale its OWN receipt.
//
// Found on a real install, not theorized: install, one inject turn, then a
// `*.verify` state with a receipt written moments earlier. `recordInject()`
// creates `.vibe/last-inject`, nothing gitignored it, and `git status
// --porcelain` collapses the wholly-untracked directory to a single `?? .vibe/`
// line — which the staleness scan read as "changed after it was written" and
// BLOCKED, over a path the human never touched. That wedges the only blocking
// tooth in the harness; the exit is `set-state.sh idle`, i.e. abandoning the
// flow.
//
// Fixed twice over — install.sh gitignores the marker (see
// flow/tests/adapters/run.sh) and the scan skips this harness's own runtime
// writes. Each case below carries a CONTROL with the identical mtime
// relationship on a path that is NOT vibe runtime state, so an exit 0 proves
// the exclusion fired rather than the fixture being inert.
// ---------------------------------------------------------------------------

// FIX ROUND 1, IMPORTANT 1 — these cases are driven by a REAL git repo and the
// porcelain git ACTUALLY emits, never by a hand-written line. The first
// revision of this fix pinned ` M .vibe/blocks/team.md`, a spelling git only
// produces once something under `.vibe/` is already tracked; on a shipped
// install the whole directory is untracked and plain `--porcelain` collapses it
// to `?? .vibe/`. The test agreed with the comment beside the code and neither
// agreed with git, so an authored block edited after the receipt slipped
// through. Reading the real bytes is what makes the claim checkable.
//
// A receipt stamped 2020 with everything else written now — the shape of the
// real bug. Everything is committed first, so each row below is a genuine
// untracked-or-modified path and not an artefact of an uncommitted tree.
function makeRealGitFixture({ gitignore } = {}) {
  const sb = makeHookSandbox({
    cursor: { flow: 'quick', phase: 'verify', feature: null, updated: '2026-01-01T00:00:00Z' },
    gitInit: true,
  });
  mkdirSync(path.join(sb.vibeDir, 'evidence'), { recursive: true });
  const receipt = path.join(sb.vibeDir, 'evidence', 'quick.md');
  writeFileSync(receipt, 'commands + output\n');
  writeFileSync(path.join(sb.dir, 'src.txt'), 'v1\n');
  if (gitignore) writeFileSync(path.join(sb.dir, '.gitignore'), `${gitignore}\n`);
  runCommand('git', ['-C', sb.dir, 'add', '-A'], { cwd: sb.dir });
  runCommand('git', ['-C', sb.dir, '-c', 'user.email=t@t', '-c', 'user.name=t', 'commit', '-qm', 'init'], { cwd: sb.dir });
  const past = new Date('2020-01-01T00:00:00Z');
  utimesSync(receipt, past, past);
  return sb;
}

// What the gate itself will see: the same argv gitPorcelain() runs.
function realPorcelain(sb) {
  return runCommand('git', ['-C', sb.dir, 'status', '--porcelain', '-uall'], { cwd: sb.dir }).stdout;
}

// No injected spawnGit — the real binary, on the real repo.
function gateOnRealGit(sb) {
  return runGateHook(sb.root, sb.vibeDir, '{}', {});
}

test('runGateHook predicate 2: the inject marker never stales the receipt, on the porcelain git really emits', () => {
  const sb = makeRealGitFixture();
  try {
    // Exactly what one inject turn leaves behind on a target that does not
    // gitignore it (an install made before the marker existed).
    mkdirSync(path.join(sb.dir, '.vibe'), { recursive: true });
    writeFileSync(path.join(sb.dir, '.vibe', 'last-inject'), 'quick.verify\n');

    const porcelain = realPorcelain(sb);
    assertEqual(
      porcelain,
      '?? .vibe/last-inject\n',
      'floor: git enumerates the marker as its own row (this is what -uall buys; plain --porcelain says `?? .vibe/`)',
    );
    assertEqual(
      gateOnRealGit(sb).code,
      0,
      'the inject marker is this harness writing to itself — it can never make a receipt stale',
    );
  } finally {
    sb.cleanup();
  }
});

// THE case the first revision got wrong. Same untracked `.vibe/` directory, one
// authored block inside it: the exclusion must reach the marker and nothing
// else. Under plain `--porcelain` both files hide behind one `?? .vibe/` row and
// this exits 0 — a stale receipt passing while the injected content changed.
test('runGateHook predicate 2: an authored block under .vibe/blocks STILL stales the receipt (real porcelain)', () => {
  const sb = makeRealGitFixture();
  try {
    mkdirSync(path.join(sb.dir, '.vibe', 'blocks'), { recursive: true });
    writeFileSync(path.join(sb.dir, '.vibe', 'last-inject'), 'quick.verify\n');
    writeFileSync(path.join(sb.dir, '.vibe', 'blocks', 'team.md'), 'authored\n');

    assertEqual(
      realPorcelain(sb),
      '?? .vibe/blocks/team.md\n?? .vibe/last-inject\n',
      'floor: the marker and the authored block are SEPARATE rows — the whole premise of the exclusion',
    );
    const result = gateOnRealGit(sb);
    assertEqual(result.code, 2, 'a block that changes what every later turn injects is a change the receipt does not describe');
    assertIncludes(result.stderr, '.vibe/blocks/team.md');
  } finally {
    sb.cleanup();
  }
});

// The shipped install: `.gitignore` carries the marker (install.sh step 5), so
// git never mentions it at all — and the authored block still blocks.
test('runGateHook predicate 2: with the marker gitignored, only the authored block reaches the scan', () => {
  const sb = makeRealGitFixture({ gitignore: '.vibe/last-inject' });
  try {
    mkdirSync(path.join(sb.dir, '.vibe'), { recursive: true });
    writeFileSync(path.join(sb.dir, '.vibe', 'last-inject'), 'quick.verify\n');
    assertEqual(realPorcelain(sb), '', 'floor: the belt alone already hides the marker');
    assertEqual(gateOnRealGit(sb).code, 0);

    mkdirSync(path.join(sb.dir, '.vibe', 'blocks'), { recursive: true });
    writeFileSync(path.join(sb.dir, '.vibe', 'blocks', 'team.md'), 'authored\n');
    assertEqual(realPorcelain(sb), '?? .vibe/blocks/team.md\n');
    assertEqual(gateOnRealGit(sb).code, 2, 'control: the gitignored marker is excluded, its neighbour is not');
  } finally {
    sb.cleanup();
  }
});

test('runGateHook predicate 2: the warnings relay log is the harness writing to itself, not a change', () => {
  const sb = makeRealGitFixture();
  try {
    writeFileSync(sb.warnLogPath, 'guard: something\n');
    assertEqual(realPorcelain(sb), '?? .agents/skills/vibe/warnings.log\n', 'floor: git does report it');
    assertEqual(
      gateOnRealGit(sb).code,
      0,
      'the guard appends to this log on the same turns the gate runs',
    );

    // Control: an ordinary untracked file in the same repo, same mtime relation.
    writeFileSync(path.join(sb.dir, 'notes.md'), 'later\n');
    const result = gateOnRealGit(sb);
    assertEqual(result.code, 2, 'control: a path that is not vibe runtime state must still block');
    assertIncludes(result.stderr, 'notes.md');
  } finally {
    sb.cleanup();
  }
});

// -uall changes what an untracked DIRECTORY looks like to the scan, so pin that
// directly: a foreign untracked directory is enumerated per file and blocks by
// naming the file, not the directory.
test('runGateHook predicate 2: a foreign untracked directory is enumerated per file and blocks by file name', () => {
  const sb = makeRealGitFixture();
  try {
    mkdirSync(path.join(sb.dir, '.other'), { recursive: true });
    writeFileSync(path.join(sb.dir, '.other', 'thing.md'), 'x\n');
    assertEqual(realPorcelain(sb), '?? .other/thing.md\n');
    const result = gateOnRealGit(sb);
    assertEqual(result.code, 2);
    assertIncludes(result.stderr, '.other/thing.md');
  } finally {
    sb.cleanup();
  }
});

// The injected-porcelain legs stay too: they are the only way to pin the
// spellings git produces on OTHER repo shapes (a tracked file under `.vibe/`,
// once a project commits its authored blocks), which no single fixture emits.
test('runGateHook predicate 2: the marker is excluded in its TRACKED spelling as well', () => {
  const sb = makeRealGitFixture();
  try {
    mkdirSync(path.join(sb.dir, '.vibe'), { recursive: true });
    writeFileSync(path.join(sb.dir, '.vibe', 'last-inject'), 'quick.verify\n');
    assertEqual(gateWithPorcelain(sb, ' M .vibe/last-inject\n').code, 0);
  } finally {
    sb.cleanup();
  }
});

// ---------------------------------------------------------------------------
// FIX ROUND 2 — `-uall` made the blocking tooth fail OPEN at scale, and the
// fix is to fail CLOSED on an undeterminable working tree.
//
// One row per untracked FILE means the output grows with the tree. Measured on
// real repos: 14,400 un-ignored untracked files produce 1.83 MB of porcelain
// with short paths, and 5,072,090 bytes with ~330-character ones — the size
// follows PATH LENGTH, not file count alone. Either way it overran spawnSync's
// DEFAULT 1 MiB maxBuffer. spawnSync then hands back
// `status: null, error: ENOBUFS`, the old code swallowed it and returned '',
// and the scan read that as "nothing changed" — so a modified `src.txt` newer
// than the receipt exited 0. The only blocking tooth in the harness, waved
// through in exactly the repos whose working tree is hardest to track.
//
// Both halves are pinned below with REAL files and a REAL spawn: the raised
// ceiling reads a tree the old default could not, and a genuine ENOBUFS blocks
// instead of passing. Only the THRESHOLD is scaled in the second test (via the
// documented `porcelainMaxBuffer` seam) — the overrun itself is real.
// ---------------------------------------------------------------------------

// Build `count` untracked files with very long paths, so porcelain crosses a
// byte budget with as few filesystem writes as the suite can get away with
// (~620 bytes of output per file). Every file is stamped WELL BEFORE the
// receipt: the tree must be large, not stale — otherwise these fixtures would
// block for the ordinary staleness reason and prove nothing about the buffer.
// Returns the porcelain size in bytes.
function plantUntrackedTree(sb, count) {
  const deep = path.join(sb.dir, 'vendor', 'a'.repeat(200), 'b'.repeat(200));
  mkdirSync(deep, { recursive: true });
  const ancient = new Date('1990-01-01T00:00:00Z');
  for (let i = 0; i < count; i += 1) {
    const f = path.join(deep, `${'c'.repeat(190)}-${i}.txt`);
    writeFileSync(f, 'x\n');
    utimesSync(f, ancient, ancient);
  }
  return realPorcelain(sb).length;
}

test('runGateHook predicate 2: a >1 MiB untracked tree is still READ — the raised buffer is what makes the tooth work at scale', () => {
  const sb = makeRealGitFixture();
  try {
    const bytes = plantUntrackedTree(sb, 1800);
    assert(
      bytes > 1024 * 1024,
      `floor: this fixture must exceed the 1 MiB DEFAULT maxBuffer to mean anything, got ${bytes} bytes`,
    );

    // The one thing that must be noticed: a tracked file modified after the
    // receipt. Under the old default buffer git's output was discarded and this
    // exited 0.
    writeFileSync(path.join(sb.dir, 'src.txt'), 'v2 — edited after the receipt\n');

    const result = gateOnRealGit(sb);
    assertEqual(result.code, 2, 'a receipt older than a modified source file must block, however big the untracked tree is');
    assertIncludes(result.stderr, 'src.txt');
    assertIncludes(result.stderr, 'stale');
  } finally {
    sb.cleanup();
  }
});

test('runGateHook predicate 2: a real ENOBUFS is an UNDETERMINABLE tree and BLOCKS — never a silent pass', () => {
  const sb = makeRealGitFixture();
  try {
    const bytes = plantUntrackedTree(sb, 200);
    assert(bytes > 4096, `floor: the fixture must overrun the 4 KiB ceiling below, got ${bytes} bytes`);

    // Real files, real git, real spawn — only the ceiling is scaled so the test
    // runs in a second. Nothing here is mocked.
    const result = runGateHook(sb.root, sb.vibeDir, '{}', { porcelainMaxBuffer: 4096 });
    assertEqual(result.code, 2, '"I could not look" must never resolve to "nothing changed" on a blocking tooth');
    assertIncludes(result.stderr, 'working tree state could not be determined');
    assertIncludes(result.stderr, 'ENOBUFS');
    assertIncludes(result.stderr, 'not verifying? abort with');

    // Discriminating control: the SAME repo with a ceiling that fits passes,
    // so the block above is the overrun and not something else about the tree.
    assertEqual(
      runGateHook(sb.root, sb.vibeDir, '{}', { porcelainMaxBuffer: 32 * 1024 * 1024 }).code,
      0,
      'control: with a buffer that fits, this same tree is clean and the gate passes',
    );
  } finally {
    sb.cleanup();
  }
});

// The other spawn failures that mean the same thing. A non-zero `git status`
// (a git that rejects `-uall`, a broken index) is just as undeterminable as
// ENOBUFS — and `rev-parse` failing is NOT: that is "no git / not a work tree",
// the documented existence-only degrade every non-git install target relies on.
test('runGateHook predicate 2: a failing `git status` blocks, while "not a git repo" still passes', () => {
  const sb = makeRealGitFixture();
  try {
    const statusFails = (args) =>
      args.includes('rev-parse')
        ? { error: null, status: 0, stdout: 'true\n' }
        : { error: null, status: 128, stdout: '' };
    const blocked = runGateHook(sb.root, sb.vibeDir, '{}', { spawnGit: statusFails });
    assertEqual(blocked.code, 2, 'a work tree whose status cannot be read is undeterminable, not clean');
    assertIncludes(blocked.stderr, 'could not be determined');

    const noGit = runGateHook(sb.root, sb.vibeDir, '{}', { spawnGit: () => ({ error: new Error('no git') }) });
    assertEqual(noGit.code, 0, 'a non-git target keeps its existence-only pass — fail-closed here would wedge every one of them');
    assertEqual(noGit.stderr, '');
  } finally {
    sb.cleanup();
  }
});

test('runGateHook predicate 2: an undeterminable tree with NO receipt still reports the missing receipt, not the buffer', () => {
  const sb = makeRealGitFixture();
  try {
    rmSync(path.join(sb.vibeDir, 'evidence'), { recursive: true, force: true });
    plantUntrackedTree(sb, 200);
    const result = runGateHook(sb.root, sb.vibeDir, '{}', { porcelainMaxBuffer: 4096 });
    assertEqual(result.code, 2);
    assertIncludes(result.stderr, "needs an evidence receipt before 'done'");
  } finally {
    sb.cleanup();
  }
});

// R6 — predicate 3 (the stuck-phase nudge) is DELETED. The level channel names
// the state on every turn, so the nudge was duplication that also queued a
// relay line per Stop. Asserted for every non-idle state the machine has, not
// just one: a deletion that only holds for the state someone remembered to test
// is not a deletion.
test('runGateHook: no state nudges any more — predicate 3 is gone (R6)', () => {
  const machine = JSON.parse(readFileSync(path.join(REPO_ROOT, 'flow', 'state-machine.json'), 'utf8'));
  const states = Object.keys(machine.states).filter((k) => k !== 'idle');
  assert(states.length >= 10, `population floor: expected the machine's full state list, got ${states.length}`);
  let nudgeless = 0;
  for (const key of states) {
    const [flow, phase = flow] = key.split('.');
    const sb = makeHookSandbox({ cursor: { flow, phase, feature: 'demo', updated: '2026-01-01T00:00:00Z' } });
    try {
      const result = runGateHook(sb.root, sb.vibeDir, '{}', { spawnGit: () => ({ error: new Error('no git') }) });
      assert(!result.stderr.includes('still in'), `${key}: the stuck-phase nudge is deleted, but stderr carried it`);
      assert(
        !readRelay(sb).includes('still in'),
        `${key}: the stuck-phase nudge is deleted, but it was queued to the relay`,
      );
      nudgeless += 1;
    } finally {
      sb.cleanup();
    }
  }
  assertEqual(nudgeless, states.length, 'every non-idle state was actually examined');
});

test('runGateHook: a non-idle state with nothing else to say is now SILENT (exit 0, empty stderr, empty relay)', () => {
  const sb = makeHookSandbox({ cursor: { flow: 'feature', phase: 'impl', feature: 'demo', updated: '2026-01-01T00:00:00Z' } });
  try {
    const result = runGateHook(sb.root, sb.vibeDir, '{}', { spawnGit: () => ({ error: new Error('no git') }) });
    assertEqual(result.code, 0);
    assertEqual(result.stderr, '');
    assertEqual(readRelay(sb), '', 'nothing queued for the next inject to drain');
  } finally {
    sb.cleanup();
  }
});

// CONTROL for the two negatives above: the gate still reaches stderr AND the
// relay when it has something to say. Without this, "no nudge" would be
// satisfied by a gate that stopped running at all.
test('runGateHook: the surviving warn path still writes stderr and the relay', () => {
  const sb = makeHookSandbox({ cursor: { flow: 'feature', phase: 'verify', feature: null, updated: '2026-01-01T00:00:00Z' } });
  try {
    const result = runGateHook(sb.root, sb.vibeDir, '{}', { spawnGit: () => ({ error: new Error('no git') }) });
    assertEqual(result.code, 0);
    assertIncludes(result.stderr, 'names no feature');
    assertIncludes(readFileSync(sb.warnLogPath, 'utf8'), 'gate:');
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

test('runGateHook: integration — real git repo, non-verify state, no changes -> exit 0 and nothing queued', () => {
  const sb = makeHookSandbox({
    gitInit: true,
    cursor: { flow: 'feature', phase: 'impl', feature: 'demo', updated: '2026-01-01T00:00:00Z' },
  });
  try {
    const result = runGateHook(sb.root, sb.vibeDir, '{}', {});
    assertEqual(result.code, 0);
    assertEqual(result.stderr, '', 'a clean tree in a building state has nothing to warn about since R6');
    assertEqual(readRelay(sb), '');
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
    assertIncludes(result.stdout, 'sessions are ephemeral');
    assert(!result.stdout.includes('Cursor:'), 'end to end, the SessionStart payload names no state (R4)');
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
