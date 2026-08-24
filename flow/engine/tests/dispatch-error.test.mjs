// engine/tests/dispatch-error.test.mjs — cli.mjs must distinguish "the
// command module itself doesn't exist yet" (report "not implemented yet")
// from "the command module exists but fails to resolve one of its own
// imports" (a real bug — surface the real error, don't swallow it).
//
// All four real commands (state/orders/doctrine/doctor) are implemented as
// of js-core/6, so there is no longer a genuinely unimplemented name in
// cli.mjs's own COMMANDS array to borrow for this. Both cases below run
// against a synthetic placeholder command instead, via a throwaway copy of
// cli.mjs whose COMMANDS array carries one extra entry — see
// makeCliWithPlaceholderCommand in run.mjs. The real, shipped cli.mjs is
// never modified.

import { writeFileSync, rmSync, existsSync } from 'node:fs';
import path from 'node:path';
import { test, assert, assertIncludes, runCommand, makeCliWithPlaceholderCommand } from './run.mjs';

function withTempCommandModule(cli, source, fn) {
  const target = path.join(cli.commandsDir, `${cli.placeholder}.mjs`);
  assert(!existsSync(target), `refusing to clobber an existing ${target}`);
  writeFileSync(target, source);
  try {
    fn(target);
  } finally {
    rmSync(target, { force: true });
  }
}

test('a command module that fails to resolve its own import surfaces the real error', () => {
  const cli = makeCliWithPlaceholderCommand();
  try {
    withTempCommandModule(cli, "import './does-not-exist-anywhere.mjs';\nexport default function () {}\n", () => {
      const result = runCommand(process.execPath, [cli.cliPath, cli.placeholder]);
      assert(result.code !== 0, `expected non-zero exit, got ${result.code}`);
      assertIncludes(
        result.stderr,
        'does-not-exist-anywhere.mjs',
        'a broken internal import must surface its own missing specifier, not a generic message',
      );
      assert(
        !result.stderr.toLowerCase().includes('not implemented'),
        'a real internal error must not be misreported as "not implemented yet"',
      );
    });
  } finally {
    cli.cleanup();
  }
});

test('a genuinely missing command module still reports "not implemented yet"', () => {
  const cli = makeCliWithPlaceholderCommand();
  try {
    const target = path.join(cli.commandsDir, `${cli.placeholder}.mjs`);
    assert(!existsSync(target), 'precondition: the placeholder module must not exist for this case');
    const result = runCommand(process.execPath, [cli.cliPath, cli.placeholder]);
    assert(result.code !== 0, `expected non-zero exit, got ${result.code}`);
    assertIncludes(result.stderr.toLowerCase(), 'not implemented');
  } finally {
    cli.cleanup();
  }
});
