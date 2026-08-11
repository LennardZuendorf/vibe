// engine/tests/machine.test.mjs — engine/machine.mjs (js-core/2).

import { mkdtempSync, copyFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { test, assert, assertEqual, makeSandbox } from './run.mjs';
import { loadMachine, stateOf } from '../machine.mjs';

const REPO_ROOT = path.resolve(path.dirname(new URL(import.meta.url).pathname), '..', '..');

test('loadMachine: returns the contract shape from the real state-machine.json', () => {
  const sandbox = makeSandbox();
  try {
    const machine = loadMachine(sandbox.flowDir);
    assertEqual(
      Object.keys(machine).sort(),
      ['flows', 'gates', 'initial', 'phases', 'states', 'style', 'version'].sort(),
    );
    assertEqual(machine.initial, 'idle');
    assert(Array.isArray(machine.flows) && machine.flows.includes('feature'), 'expected flows to include feature');
    assert('idle' in machine.states, 'expected idle state');
  } finally {
    sandbox.cleanup();
  }
});

test('loadMachine: does not leak the raw $comment scratch field', () => {
  const sandbox = makeSandbox();
  try {
    const machine = loadMachine(sandbox.flowDir);
    assert(!('$comment' in machine), 'machine should only carry the contract fields');
  } finally {
    sandbox.cleanup();
  }
});

test('stateOf: looks up a known compound state', () => {
  const sandbox = makeSandbox();
  try {
    const machine = loadMachine(sandbox.flowDir);
    const rec = stateOf(machine, 'feature.impl');
    assert(rec, 'expected a state record for feature.impl');
    assertEqual(rec.skill, 'vibe');
    assert(Array.isArray(rec.next) && rec.next.includes('feature.verify'));
  } finally {
    sandbox.cleanup();
  }
});

test('stateOf: unknown state returns undefined, not throw', () => {
  const sandbox = makeSandbox();
  try {
    const machine = loadMachine(sandbox.flowDir);
    assertEqual(stateOf(machine, 'not.a.real.state'), undefined);
  } finally {
    sandbox.cleanup();
  }
});

test('loadMachine: throws on a missing state-machine.json', async () => {
  const sandbox = makeSandbox();
  sandbox.cleanup(); // now the dir itself is gone
  let threw = false;
  try {
    loadMachine(sandbox.flowDir);
  } catch (err) {
    threw = true;
    assertEqual(err.code, 'ENOENT');
  }
  assert(threw, 'expected loadMachine to throw for an absent machine file');
});

// js-core/2 review, Finding 1 (CRITICAL) — loadMachine(root) used to join
// 'flow/state-machine.json' onto a project root, which threw ENOENT for
// every command on a real install target (no flow/ dir there). loadMachine
// now takes vibeDir directly — the directory state-machine.json actually
// lives in — so this must work for a bare directory with the file placed
// straight inside it, no flow/ nesting required.
test('CRITICAL (Finding 1): loadMachine works given vibeDir directly, no flow/ subdir required', () => {
  const vibeDir = mkdtempSync(path.join(tmpdir(), 'vibe-machine-install-'));
  try {
    copyFileSync(
      path.join(REPO_ROOT, 'flow', 'state-machine.json'),
      path.join(vibeDir, 'state-machine.json'),
    );
    const machine = loadMachine(vibeDir);
    assert('idle' in machine.states, 'expected idle state from the copied machine');
    assert(!('$comment' in machine));
  } finally {
    rmSync(vibeDir, { recursive: true, force: true });
  }
});
