// engine/tests/runner.test.mjs — the runner must fail loud, not report green
// on a broken selection: a filter matching zero tests, or discovery finding
// zero test files at all, must both exit non-zero with a stated reason.

import { mkdtempSync, copyFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { test, assert, assertEqual, assertIncludes, runCommand } from './run.mjs';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const RUN_MJS = path.join(__dirname, 'run.mjs');

test('a filter matching zero tests exits non-zero and says so', () => {
  const result = runCommand(process.execPath, [RUN_MJS, 'zzz-no-such-test-xyz']);
  assert(result.code !== 0, `expected non-zero exit, got ${result.code}`);
  const combined = result.stdout + result.stderr;
  assertIncludes(combined, 'zzz-no-such-test-xyz', 'should name the filter that matched nothing');
});

test('zero discovered test files exits non-zero and says so', () => {
  // Isolate run.mjs in an empty directory so its own discovery (readdirSync
  // of its own dirname for *.test.mjs) finds nothing to import.
  const dir = mkdtempSync(path.join(tmpdir(), 'vibe-runner-empty-'));
  try {
    const copy = path.join(dir, 'run.mjs');
    copyFileSync(RUN_MJS, copy);
    const result = runCommand(process.execPath, [copy]);
    assert(result.code !== 0, `expected non-zero exit, got ${result.code}`);
    const combined = result.stdout + result.stderr;
    assertIncludes(combined, 'no test files', 'should state that no test files were found');
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test('normal run with matching tests still exits 0', () => {
  const result = runCommand(process.execPath, [RUN_MJS, 'harness.test.mjs']);
  assertEqual(result.code, 0, `expected exit 0, got ${result.code}; stderr: ${result.stderr}`);
});
