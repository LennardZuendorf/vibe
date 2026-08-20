// engine/tests/cli.test.mjs — dispatch, --help, and error-taxonomy coverage
// for engine/cli.mjs (js-core/1).

import { test, assert, assertEqual, assertIncludes, runCli } from './run.mjs';

const COMMANDS = ['state', 'orders', 'doctrine', 'doctor'];

// Commands land one unit at a time (js-core/3-6). Each ported command drops
// out of this list — its own *.test.mjs covers real behaviour instead. Only
// the still-unimplemented ones should hit the "not implemented yet" path.
const NOT_YET_IMPLEMENTED = ['doctor'];

test('--help lists the four commands', () => {
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
  for (const name of NOT_YET_IMPLEMENTED) {
    const result = runCli([name]);
    assert(result.code !== 0, `expected '${name}' to exit non-zero before it is implemented`);
    assertIncludes(result.stderr.toLowerCase(), 'not implemented', `stderr for '${name}' should say not implemented`);
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
