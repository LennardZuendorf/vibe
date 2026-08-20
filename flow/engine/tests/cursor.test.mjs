// engine/tests/cursor.test.mjs — engine/cursor.mjs (js-core/2, R6).
//
// The two load-bearing scenarios: an absent cursor reads as idle, and a
// present-but-unparseable cursor THROWS a named CursorParseError instead of
// silently degrading to idle — every bash cursor reader gets this wrong.
//
// readCursor/writeCursor take `vibeDir` directly (the directory state.json
// lives in), not a project root — see cursor.mjs's header. Callers resolve
// that directory via resolveVibeDir() in root.mjs; these tests either pass
// a bare fixture dir straight through (readCursor doesn't care how it was
// found) or, for the install-target scenario below, actually exercise
// resolveVibeDir() end to end.

import { mkdtempSync, mkdirSync, writeFileSync, copyFileSync, rmSync, readFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { pathToFileURL } from 'node:url';
import { test, assert, assertEqual, makeSandbox } from './run.mjs';
import { readCursor, writeCursor, CursorParseError } from '../cursor.mjs';

const ROOT_MJS = path.resolve(path.dirname(new URL(import.meta.url).pathname), '..', 'root.mjs');

function bareDir() {
  return mkdtempSync(path.join(tmpdir(), 'vibe-cursor-'));
}

test('readCursor: absent cursor file reads as idle', () => {
  const vibeDir = bareDir();
  try {
    const cursor = readCursor(vibeDir);
    assertEqual(cursor.state, 'idle');
    assertEqual(cursor.flow, 'idle');
    assertEqual(cursor.phase, 'idle');
    assertEqual(cursor.feature, null);
  } finally {
    rmSync(vibeDir, { recursive: true, force: true });
  }
});

test('readCursor: malformed (unparseable) cursor throws a named CursorParseError, not idle', () => {
  const vibeDir = bareDir();
  try {
    writeFileSync(path.join(vibeDir, 'state.json'), '{ this is not valid json');

    let threw = false;
    try {
      readCursor(vibeDir);
    } catch (err) {
      threw = true;
      assert(err instanceof CursorParseError, `expected CursorParseError, got ${err && err.name}`);
      assertEqual(err.name, 'CursorParseError');
    }
    assert(threw, 'expected readCursor to throw on a malformed cursor, not degrade to idle');
  } finally {
    rmSync(vibeDir, { recursive: true, force: true });
  }
});

test('readCursor: a JSON array at the top level is malformed, not idle', () => {
  const vibeDir = bareDir();
  try {
    writeFileSync(path.join(vibeDir, 'state.json'), '[1, 2, 3]');
    let threw = false;
    try {
      readCursor(vibeDir);
    } catch (err) {
      threw = true;
      assert(err instanceof CursorParseError);
    }
    assert(threw, 'a top-level JSON array is not a valid cursor shape');
  } finally {
    rmSync(vibeDir, { recursive: true, force: true });
  }
});

test('readCursor: a well-formed cursor reads flow/phase/feature and derives state', () => {
  const sandbox = makeSandbox({
    cursor: { flow: 'feature', phase: 'impl', feature: 'js-core', updated: '2026-01-01T00:00:00Z' },
  });
  try {
    const cursor = readCursor(sandbox.flowDir);
    assertEqual(cursor.flow, 'feature');
    assertEqual(cursor.phase, 'impl');
    assertEqual(cursor.feature, 'js-core');
    assertEqual(cursor.state, 'feature.impl');
  } finally {
    sandbox.cleanup();
  }
});

test('readCursor: single-token states collapse flow===phase into one key (idle, not idle.idle)', () => {
  const sandbox = makeSandbox();
  try {
    const cursor = readCursor(sandbox.flowDir);
    assertEqual(cursor.state, 'idle');
  } finally {
    sandbox.cleanup();
  }
});

test('writeCursor: writes atomically and readCursor reads back what was written', () => {
  const sandbox = makeSandbox();
  try {
    writeCursor(sandbox.flowDir, { flow: 'quick', phase: 'fix', feature: 'demo' });
    const cursor = readCursor(sandbox.flowDir);
    assertEqual(cursor.flow, 'quick');
    assertEqual(cursor.phase, 'fix');
    assertEqual(cursor.feature, 'demo');
    assertEqual(cursor.state, 'quick.fix');
  } finally {
    sandbox.cleanup();
  }
});

test('writeCursor: omitted feature defaults to null and matches jq key order (flow, phase, feature, updated)', () => {
  const sandbox = makeSandbox();
  try {
    writeCursor(sandbox.flowDir, { flow: 'idle', phase: 'idle' });
    const raw = readFileSync(sandbox.cursorPath, 'utf8');
    const keys = Object.keys(JSON.parse(raw));
    assertEqual(keys, ['flow', 'phase', 'feature', 'updated']);
    assert(raw.endsWith('}\n'), 'expected a trailing newline');
  } finally {
    sandbox.cleanup();
  }
});

test('sandbox self-check: makeSandbox\'s own cursor never triggers CursorParseError', () => {
  const sandbox = makeSandbox();
  try {
    readCursor(sandbox.flowDir); // must not throw
  } finally {
    sandbox.cleanup();
  }
});

// ---------------------------------------------------------------------------
// js-core/2 review, Finding 1 (CRITICAL) — install targets have no flow/ dir.
//
// readCursor(root) used to hardcode path.join(root, 'flow', 'state.json').
// That only ever worked in this dogfood repo, where .agents/skills/vibe is a
// symlink to flow/. A real install target (built by `cp -RL` into
// <TARGET>/.agents/skills/vibe/, per install.sh) has no flow/ directory at
// all — the old code would hit ENOENT and silently report "idle" while the
// real cursor said otherwise: exactly the wrong-answer class the
// CursorParseError fix above exists to prevent, just one layer up.
//
// This builds a fixture laid out exactly like a real install target
// (<tmp>/.agents/skills/vibe/{engine,state.json,state-machine.json}, no
// flow/, no .git, no .spec anywhere) and drives the *real* resolution path:
// a copy of root.mjs physically placed at the nested engine/ location (so
// its self-relative resolution triggers for real) resolves vibeDir, and the
// real (uncopied) cursor.mjs reads through it.
test('CRITICAL (Finding 1): cursor reads correctly from an install-target layout with no flow/ dir', async () => {
  const installRoot = mkdtempSync(path.join(tmpdir(), 'vibe-install-cursor-'));
  const vibeDir = path.join(installRoot, '.agents', 'skills', 'vibe');
  const engineDir = path.join(vibeDir, 'engine');
  mkdirSync(engineDir, { recursive: true });
  copyFileSync(ROOT_MJS, path.join(engineDir, 'root.mjs'));

  writeFileSync(
    path.join(vibeDir, 'state.json'),
    `${JSON.stringify(
      { flow: 'feature', phase: 'impl', feature: 'js-core', updated: '2026-08-11T18:18:41Z' },
      null,
      2,
    )}\n`,
  );

  // A cwd the resolver must NOT wander into via marker search or luck.
  const unrelatedCwd = mkdtempSync(path.join(tmpdir(), 'vibe-install-cwd-'));

  const prevEnv = process.env.CLAUDE_PROJECT_DIR;
  delete process.env.CLAUDE_PROJECT_DIR;

  try {
    assert(!existsFlowDir(installRoot), 'fixture must have no flow/ dir (this is the point)');

    const { resolveVibeDir } = await import(pathToFileURL(path.join(engineDir, 'root.mjs')).href);
    const resolved = resolveVibeDir({ cwd: unrelatedCwd });
    assertEqual(resolved, vibeDir);

    const cursor = readCursor(resolved);
    assertEqual(cursor.state, 'feature.impl');
    assertEqual(cursor.feature, 'js-core');
    assert(cursor.state !== 'idle', 'must not silently report idle for a real, present cursor');
  } finally {
    rmSync(installRoot, { recursive: true, force: true });
    rmSync(unrelatedCwd, { recursive: true, force: true });
    if (prevEnv !== undefined) process.env.CLAUDE_PROJECT_DIR = prevEnv;
  }
});

function existsFlowDir(root) {
  try {
    readFileSync(path.join(root, 'flow', 'state.json'));
    return true;
  } catch {
    return false;
  }
}
