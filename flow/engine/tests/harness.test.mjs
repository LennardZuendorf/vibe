// engine/tests/harness.test.mjs — coverage for the harness itself: the
// sandbox fixture builder and the runCli/runCommand process helpers.
// Later units' tests lean on these; pin their contract here.

import { existsSync, readFileSync, statSync } from 'node:fs';
import path from 'node:path';
import { test, assert, assertEqual, assertIncludes, makeSandbox, makeHookSandbox, runCommand } from './run.mjs';

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

// The contract the rest of the suite's `assert(result.code !== 0)` sites rest
// on. `null !== 0`, so a command that never ran used to read as the failure the
// test was hoping for.
test('runCommand: a command that cannot be spawned THROWS, naming the command — never a null code', () => {
  const missing = 'vibe-no-such-binary-9f3a2c';
  let caught;
  try {
    runCommand(missing, ['--flag', 'arg']);
  } catch (err) {
    caught = err;
  }
  assert(caught !== undefined, 'an unspawnable binary must throw, not return a null code');
  assertIncludes(caught.message, missing, 'the message must name the command so a red build is actionable');
  assertIncludes(caught.message, '--flag arg', 'the message must carry the arguments too');

  // Control, and this case's population floor: the throw is about being unable
  // to RUN the command, not about failing. A command that runs and exits
  // non-zero still returns its code, exactly as before.
  const ran = runCommand('node', ['-e', 'process.exitCode = 4;']);
  assertEqual(ran.code, 4, 'a real non-zero exit is still a returned code, not a throw');
});

test('runCommand: a signalled child reports the signal by name, not a null code', () => {
  const killed = runCommand('node', ['-e', "process.kill(process.pid, 'SIGKILL');"]);
  assertEqual(killed.code, 'signal:SIGKILL', 'a killed child ran — it is not a spawn error, and it is not exit 0 either');
  assert(killed.code !== 0, 'and it still reads as a failure to every `code !== 0` site');
});

// The property the three ignored `git` exit codes inside makeHookSandbox were
// supposed to guarantee. Without it a fixture that never became a repository
// was handed back as if it had, and every git-shaped assertion against it was
// really about the failed init.
test('makeHookSandbox({gitInit:true}): the sandbox really IS a git repository', () => {
  const sb = makeHookSandbox({ gitInit: true });
  try {
    const probe = runCommand('git', ['-C', sb.dir, 'rev-parse', '--git-dir'], { cwd: sb.dir });
    assertEqual(probe.code, 0, `git rev-parse failed in the sandbox: ${probe.stderr}`);
    assert(probe.stdout.trim().length > 0, 'floor: rev-parse must have printed a git dir, not an empty answer');
    assert(existsSync(path.join(sb.dir, '.git')), 'the sandbox must carry its own .git, not inherit an outer repo');

    const email = runCommand('git', ['-C', sb.dir, 'config', 'user.email'], { cwd: sb.dir });
    assertEqual(email.code, 0, `git config user.email failed: ${email.stderr}`);
    assertEqual(email.stdout.trim(), 't@t', 'the identity the fixture claims to have configured must be readable back');
  } finally {
    sb.cleanup();
  }

  // Control: gitInit defaults to false, so the guarded commands are genuinely
  // conditional — this test is not simply asserting that git exists.
  const plain = makeHookSandbox();
  try {
    assert(!existsSync(path.join(plain.dir, '.git')), 'without gitInit the sandbox must NOT be a repository');
  } finally {
    plain.cleanup();
  }
});
