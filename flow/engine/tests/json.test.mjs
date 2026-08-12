// engine/tests/json.test.mjs — engine/json.mjs (js-core/2).

import { readdirSync, readFileSync, existsSync, writeFileSync, mkdirSync } from 'node:fs';
import path from 'node:path';
import { test, assert, assertEqual, assertThrows, makeSandbox } from './run.mjs';
import { readJson, writeJsonAtomic } from '../json.mjs';

test('readJson: parses a well-formed file', () => {
  const sandbox = makeSandbox();
  try {
    const data = readJson(sandbox.machinePath);
    assert(data && typeof data === 'object', 'expected an object');
    assert('idle' in data.states, 'expected the copied machine to include idle');
  } finally {
    sandbox.cleanup();
  }
});

test('readJson: throws ENOENT for an absent file', async () => {
  const sandbox = makeSandbox();
  try {
    const missing = path.join(sandbox.dir, 'nope.json');
    await assertThrows(() => readJson(missing));
    try {
      readJson(missing);
      assert(false, 'expected readJson to throw');
    } catch (err) {
      assertEqual(err.code, 'ENOENT');
    }
  } finally {
    sandbox.cleanup();
  }
});

test('readJson: throws SyntaxError for malformed JSON text', () => {
  const sandbox = makeSandbox();
  try {
    const badPath = path.join(sandbox.dir, 'bad.json');
    writeFileSync(badPath, '{ not json');
    let threw = false;
    try {
      readJson(badPath);
    } catch (err) {
      threw = true;
      assert(
        err instanceof SyntaxError,
        `expected SyntaxError, got ${err && err.constructor && err.constructor.name}`,
      );
    }
    assert(threw, 'expected readJson to throw on malformed JSON');
  } finally {
    sandbox.cleanup();
  }
});

test('writeJsonAtomic: writes 2-space-indented JSON with a trailing newline', () => {
  const sandbox = makeSandbox();
  try {
    const target = path.join(sandbox.dir, 'out.json');
    writeJsonAtomic(target, {
      flow: 'idle',
      phase: 'idle',
      feature: null,
      updated: '2026-01-01T00:00:00Z',
    });
    const raw = readFileSync(target, 'utf8');
    assertEqual(
      raw,
      '{\n  "flow": "idle",\n  "phase": "idle",\n  "feature": null,\n  "updated": "2026-01-01T00:00:00Z"\n}\n',
    );
  } finally {
    sandbox.cleanup();
  }
});

test('writeJsonAtomic: leaves no stray temp file behind in the target directory', () => {
  const sandbox = makeSandbox();
  try {
    const target = path.join(sandbox.dir, 'out2.json');
    writeJsonAtomic(target, { a: 1 });
    const leftovers = readdirSync(sandbox.dir).filter(
      (f) => f !== 'out2.json' && f !== 'flow' && f !== '.spec',
    );
    assertEqual(leftovers, [], `unexpected leftover files: ${leftovers.join(', ')}`);
    assert(existsSync(target), 'target file should exist');
  } finally {
    sandbox.cleanup();
  }
});

test('writeJsonAtomic: overwrites an existing file in place (rename semantics)', () => {
  const sandbox = makeSandbox();
  try {
    const target = path.join(sandbox.dir, 'out3.json');
    writeJsonAtomic(target, { v: 1 });
    writeJsonAtomic(target, { v: 2 });
    assertEqual(JSON.parse(readFileSync(target, 'utf8')), { v: 2 });
  } finally {
    sandbox.cleanup();
  }
});

// js-core/2 review, Finding 3 (Important) — no cleanup on write failure.
// Force the rename leg to fail (target is an existing directory, so
// renaming a regular temp file onto it always errors) and assert the temp
// file this call created does not survive the failure — mirrors bash's
// `trap 'rm -f "$TMP"' EXIT`.
test('writeJsonAtomic: unlinks its temp file when the write/rename fails, leaking nothing', () => {
  const sandbox = makeSandbox();
  try {
    const target = path.join(sandbox.dir, 'out4.json');
    mkdirSync(target); // target is a directory, so renaming onto it must fail

    let threw = false;
    try {
      writeJsonAtomic(target, { a: 1 });
    } catch {
      threw = true;
    }
    assert(threw, 'expected writeJsonAtomic to propagate the rename failure');

    const leftovers = readdirSync(sandbox.dir).filter((f) => f.includes('.tmp'));
    assertEqual(leftovers, [], `temp file leaked after a failed write: ${leftovers.join(', ')}`);
  } finally {
    sandbox.cleanup();
  }
});
