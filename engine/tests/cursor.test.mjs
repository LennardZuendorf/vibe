// engine/tests/cursor.test.mjs — engine/cursor.mjs (js-core/2, R6).
//
// The two load-bearing scenarios: an absent cursor reads as idle, and a
// present-but-unparseable cursor THROWS a named CursorParseError instead of
// silently degrading to idle — every bash cursor reader gets this wrong.

import { mkdtempSync, mkdirSync, writeFileSync, rmSync, readFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { test, assert, assertEqual, makeSandbox } from './run.mjs';
import { readCursor, writeCursor, CursorParseError } from '../cursor.mjs';

function bareRoot() {
  return mkdtempSync(path.join(tmpdir(), 'vibe-cursor-'));
}

test('readCursor: absent cursor file (no flow/state.json at all) reads as idle', () => {
  const root = bareRoot();
  try {
    const cursor = readCursor(root);
    assertEqual(cursor.state, 'idle');
    assertEqual(cursor.flow, 'idle');
    assertEqual(cursor.phase, 'idle');
    assertEqual(cursor.feature, null);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

test('readCursor: absent flow/ directory entirely also reads as idle (not a crash)', () => {
  const root = bareRoot(); // no flow/ subdir created at all
  try {
    const cursor = readCursor(root);
    assertEqual(cursor.state, 'idle');
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

test('readCursor: malformed (unparseable) cursor throws a named CursorParseError, not idle', () => {
  const root = bareRoot();
  try {
    mkdirSync(path.join(root, 'flow'), { recursive: true });
    writeFileSync(path.join(root, 'flow', 'state.json'), '{ this is not valid json');

    let threw = false;
    try {
      readCursor(root);
    } catch (err) {
      threw = true;
      assert(err instanceof CursorParseError, `expected CursorParseError, got ${err && err.name}`);
      assertEqual(err.name, 'CursorParseError');
    }
    assert(threw, 'expected readCursor to throw on a malformed cursor, not degrade to idle');
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

test('readCursor: a JSON array at the top level is malformed, not idle', () => {
  const root = bareRoot();
  try {
    mkdirSync(path.join(root, 'flow'), { recursive: true });
    writeFileSync(path.join(root, 'flow', 'state.json'), '[1, 2, 3]');
    let threw = false;
    try {
      readCursor(root);
    } catch (err) {
      threw = true;
      assert(err instanceof CursorParseError);
    }
    assert(threw, 'a top-level JSON array is not a valid cursor shape');
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

test('readCursor: a well-formed cursor reads flow/phase/feature and derives state', () => {
  const sandbox = makeSandbox({
    cursor: { flow: 'feature', phase: 'impl', feature: 'js-core', updated: '2026-01-01T00:00:00Z' },
  });
  try {
    const cursor = readCursor(sandbox.dir);
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
    const cursor = readCursor(sandbox.dir);
    assertEqual(cursor.state, 'idle');
  } finally {
    sandbox.cleanup();
  }
});

test('writeCursor: writes atomically and readCursor reads back what was written', () => {
  const sandbox = makeSandbox();
  try {
    writeCursor(sandbox.dir, { flow: 'quick', phase: 'fix', feature: 'demo' });
    const cursor = readCursor(sandbox.dir);
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
    writeCursor(sandbox.dir, { flow: 'idle', phase: 'idle' });
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
    readCursor(sandbox.dir); // must not throw
  } finally {
    sandbox.cleanup();
  }
});
