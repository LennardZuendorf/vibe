// engine/tests/machine.test.mjs — engine/machine.mjs (js-core/2).

import { test, assert, assertEqual } from './run.mjs';
import { makeSandbox } from './run.mjs';
import { loadMachine, stateOf } from '../machine.mjs';

test('loadMachine: returns the contract shape from the real state-machine.json', () => {
  const sandbox = makeSandbox();
  try {
    const machine = loadMachine(sandbox.dir);
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
    const machine = loadMachine(sandbox.dir);
    assert(!('$comment' in machine), 'machine should only carry the contract fields');
  } finally {
    sandbox.cleanup();
  }
});

test('stateOf: looks up a known compound state', () => {
  const sandbox = makeSandbox();
  try {
    const machine = loadMachine(sandbox.dir);
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
    const machine = loadMachine(sandbox.dir);
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
    loadMachine(sandbox.dir);
  } catch (err) {
    threw = true;
    assertEqual(err.code, 'ENOENT');
  }
  assert(threw, 'expected loadMachine to throw for an absent machine file');
});
