// engine/tests/cli.test.mjs — dispatch, --help, and error-taxonomy coverage
// for engine/cli.mjs (js-core/1).

import { test, assert, assertEqual, assertIncludes, runCli, runCommand, makeCliWithPlaceholderCommand } from './run.mjs';

const COMMANDS = ['state', 'orders', 'doctrine', 'doctor', 'hook'];

test('--help lists the five commands', () => {
  const result = runCli(['--help']);
  assertEqual(result.code, 0, `expected exit 0, got ${result.code}; stderr: ${result.stderr}`);
  for (const name of COMMANDS) {
    assertIncludes(result.stdout, name, `--help output missing command '${name}'`);
  }
});

test('-h is an alias for --help', () => {
  const result = runCli(['-h']);
  assertEqual(result.code, 0);
  assertIncludes(result.stdout, 'state');
});

test('no arguments prints help and exits 0', () => {
  const result = runCli([]);
  assertEqual(result.code, 0);
  assertIncludes(result.stdout, 'Usage');
});

test('unknown subcommand exits non-zero and names itself', () => {
  const result = runCli(['bogus-command']);
  assert(result.code !== 0, `expected non-zero exit, got ${result.code}`);
  assertIncludes(result.stderr, 'bogus-command', 'stderr should name the unknown subcommand');
});

test('known-but-unimplemented subcommand exits non-zero, not-implemented message', () => {
  // All four real commands are implemented as of js-core/6 — there is no
  // longer a genuinely unimplemented name in cli.mjs's own COMMANDS array.
  // Exercise the same dispatch path against a synthetic placeholder command
  // instead (see makeCliWithPlaceholderCommand's own header).
  const cli = makeCliWithPlaceholderCommand();
  try {
    const result = runCommand(process.execPath, [cli.cliPath, cli.placeholder]);
    assert(result.code !== 0, `expected '${cli.placeholder}' to exit non-zero when its module is missing`);
    assertIncludes(
      result.stderr.toLowerCase(),
      'not implemented',
      `stderr for '${cli.placeholder}' should say not implemented`,
    );
  } finally {
    cli.cleanup();
  }
});

test('dispatcher never throws an unhandled rejection (no stack trace leaks to stdout)', () => {
  const result = runCli(['totally-unknown-xyz']);
  // A crashed/unhandled-rejection process prints a Node stack trace starting
  // with "at " frames and usually a non-1 or signal-based exit; assert we get
  // the clean named error path instead.
  assertEqual(result.code, 1);
  assert(!result.stderr.includes('UnhandledPromiseRejection'), 'must not surface an unhandled rejection');
});
