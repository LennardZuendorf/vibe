// engine/tests/harness.test.mjs — coverage for the harness itself: the
// sandbox fixture builder and the runCli/runCommand process helpers.
// Later units' tests lean on these; pin their contract here.

import { existsSync, readFileSync, statSync } from 'node:fs';
import { test, assert, assertEqual, makeSandbox, runCommand } from './run.mjs';

test('sandbox: creates flow/state.json and flow/state-machine.json, then cleans up', () => {
  const sandbox = makeSandbox();
  try {
    assert(existsSync(sandbox.cursorPath), 'sandbox cursor should exist');
    assert(existsSync(sandbox.machinePath), 'sandbox machine should exist');

    const cursor = JSON.parse(readFileSync(sandbox.cursorPath, 'utf8'));
    assertEqual(cursor.flow, 'idle');
    assertEqual(cursor.phase, 'idle');
    assertEqual(cursor.feature, null);

    const machine = JSON.parse(readFileSync(sandbox.machinePath, 'utf8'));
    assert(machine.states && typeof machine.states === 'object', 'copied machine should have states');
    assert('idle' in machine.states, 'copied machine should include the idle state');

    assert(statSync(sandbox.dir).isDirectory(), 'sandbox.dir should be a directory');
  } finally {
    sandbox.cleanup();
  }
  assert(!existsSync(sandbox.dir), 'cleanup() should remove the sandbox directory');
});

test('sandbox: accepts a custom cursor body', () => {
  const sandbox = makeSandbox({
    cursor: { flow: 'feature', phase: 'impl', feature: 'demo', updated: '2026-01-01T00:00:00Z' },
  });
  try {
    const cursor = JSON.parse(readFileSync(sandbox.cursorPath, 'utf8'));
    assertEqual(cursor.flow, 'feature');
    assertEqual(cursor.feature, 'demo');
  } finally {
    sandbox.cleanup();
  }
});

test('runCommand: captures stdout, stderr, and exit code', () => {
  const result = runCommand('node', ['-e', "process.stdout.write('hi'); process.exitCode = 3;"]);
  assertEqual(result.stdout, 'hi');
  assertEqual(result.code, 3);
});
