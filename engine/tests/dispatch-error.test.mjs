// engine/tests/dispatch-error.test.mjs — cli.mjs must distinguish "the
// command module itself doesn't exist yet" (report "not implemented yet")
// from "the command module exists but fails to resolve one of its own
// imports" (a real bug — surface the real error, don't swallow it).

import { writeFileSync, rmSync, existsSync, mkdirSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { test, assert, assertIncludes, runCli } from './run.mjs';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const COMMANDS_DIR = path.join(__dirname, '..', 'commands');
// 'doctor' is not implemented as of js-core/1; safe to borrow transiently.
const TARGET = path.join(COMMANDS_DIR, 'doctor.mjs');

function withTempCommandModule(source, fn) {
  const preexisting = existsSync(TARGET);
  assert(!preexisting, `refusing to clobber an existing ${TARGET} — a later unit must have landed it`);
  mkdirSync(COMMANDS_DIR, { recursive: true });
  writeFileSync(TARGET, source);
  try {
    fn();
  } finally {
    rmSync(TARGET, { force: true });
  }
}

test('a command module that fails to resolve its own import surfaces the real error', () => {
  withTempCommandModule(
    "import './does-not-exist-anywhere.mjs';\nexport default function () {}\n",
    () => {
      const result = runCli(['doctor']);
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
    },
  );
});

test('a genuinely missing command module still reports "not implemented yet"', () => {
  // No temp file created — doctor.mjs genuinely does not exist yet.
  assert(!existsSync(TARGET), 'precondition: doctor.mjs must not exist for this case');
  const result = runCli(['doctor']);
  assert(result.code !== 0, `expected non-zero exit, got ${result.code}`);
  assertIncludes(result.stderr.toLowerCase(), 'not implemented');
});
