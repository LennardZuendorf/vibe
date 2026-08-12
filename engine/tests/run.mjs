#!/usr/bin/env node
// engine/tests/run.mjs — the JS-suite harness: assert helpers, sandbox fixture
// builder, process-spawn helpers, and the test runner/discoverer.
//
// This is the foundation every later js-core unit's *.test.mjs runs under.
// Test files live alongside this one as engine/tests/*.test.mjs and register
// their cases by importing `test` from this module — the runner discovers
// and imports them, then executes the shared registry.
//
// Usage:
//   node engine/tests/run.mjs              # run every discovered test
//   node engine/tests/run.mjs <substring>   # run only tests whose file or
//                                            # name includes <substring>
//                                            # (repeatable: any match runs)
//
// Exit codes: 0 all selected tests passed; 1 at least one failed or the
// runner itself crashed while discovering/loading test files.

import { strict as nodeAssert } from 'node:assert';
import {
  readdirSync,
  readFileSync,
  mkdtempSync,
  mkdirSync,
  writeFileSync,
  copyFileSync,
  rmSync,
} from 'node:fs';
import { spawnSync } from 'node:child_process';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const TESTS_DIR = __dirname;
const REPO_ROOT = path.resolve(__dirname, '..', '..');
const CLI_PATH = path.join(REPO_ROOT, 'engine', 'cli.mjs');

// ---------------------------------------------------------------------------
// Assert helpers
// ---------------------------------------------------------------------------

export function assert(cond, msg = 'assertion failed') {
  if (!cond) throw new Error(msg);
}

export function assertEqual(actual, expected, msg) {
  nodeAssert.deepStrictEqual(
    actual,
    expected,
    msg ?? `expected ${JSON.stringify(expected)}, got ${JSON.stringify(actual)}`,
  );
}

export function assertMatch(str, re, msg) {
  assert(
    typeof str === 'string' && re.test(str),
    msg ?? `expected ${JSON.stringify(str)} to match ${re}`,
  );
}

export function assertIncludes(haystack, needle, msg) {
  assert(
    typeof haystack === 'string' && haystack.includes(needle),
    msg ?? `expected ${JSON.stringify(haystack)} to include ${JSON.stringify(needle)}`,
  );
}

// A test calls `skip(reason)` to opt out of an unmet precondition (e.g. no
// jq on PATH) — it must show up in the summary as a SKIP, never silently
// count as a pass. A plain early `return` inside a passing test body is
// indistinguishable from "ran and asserted nothing was wrong"; the CI leg
// that strips jq from PATH needs to see how many assertions actually ran.
export class Skipped extends Error {
  constructor(reason = 'skipped') {
    super(reason);
    this.name = 'Skipped';
  }
}

export function skip(reason) {
  throw new Skipped(reason);
}

export async function assertThrows(fn, msg) {
  let threw = false;
  try {
    await fn();
  } catch {
    threw = true;
  }
  assert(threw, msg ?? 'expected function to throw');
}

// ---------------------------------------------------------------------------
// Process-spawn helpers (used by parity tests against the bash originals too)
// ---------------------------------------------------------------------------

// Spawns any command synchronously and normalizes the result shape. Never
// throws on a non-zero exit — callers assert on `.code`.
export function runCommand(cmd, args = [], opts = {}) {
  const result = spawnSync(cmd, args, {
    cwd: opts.cwd ?? REPO_ROOT,
    env: { ...process.env, ...(opts.env ?? {}) },
    input: opts.input,
    encoding: 'utf8',
  });
  return {
    code: result.status,
    stdout: result.stdout ?? '',
    stderr: result.stderr ?? '',
    error: result.error ?? null,
  };
}

// Convenience wrapper for invoking the vibe CLI under test.
export function runCli(args = [], opts = {}) {
  return runCommand(process.execPath, [CLI_PATH, ...args], opts);
}

// Builds a throwaway copy of cli.mjs whose COMMANDS array carries one extra
// placeholder name, alongside an empty commands/ dir for it to resolve
// against. All four real commands (state/orders/doctrine/doctor) are
// implemented as of js-core/6, so cli.mjs's dispatch-error paths ("module
// genuinely missing" vs "module exists but its own import is broken") have
// no real unimplemented command left to exercise them against — this gives
// dispatch-error.test.mjs / cli.test.mjs a synthetic one without touching
// the real, shipped COMMANDS array. Returns {cliPath, commandsDir,
// placeholder, cleanup()}.
export function makeCliWithPlaceholderCommand(placeholder = 'zzz-test-placeholder') {
  const dir = mkdtempSync(path.join(tmpdir(), 'vibe-cli-placeholder-'));
  const src = readFileSync(CLI_PATH, 'utf8');
  const marker = "const COMMANDS = ['state', 'orders', 'doctrine', 'doctor'];";
  assert(src.includes(marker), 'cli.mjs COMMANDS array literal has changed shape — update this test helper');
  const patched = src.replace(marker, `const COMMANDS = ['state', 'orders', 'doctrine', 'doctor', '${placeholder}'];`);
  const cliPath = path.join(dir, 'cli.mjs');
  writeFileSync(cliPath, patched);
  const commandsDir = path.join(dir, 'commands');
  mkdirSync(commandsDir, { recursive: true });

  let cleaned = false;
  function cleanup() {
    if (cleaned) return;
    cleaned = true;
    rmSync(dir, { recursive: true, force: true });
  }

  return { cliPath, commandsDir, placeholder, cleanup };
}

// ---------------------------------------------------------------------------
// Sandbox fixture builder
// ---------------------------------------------------------------------------

// Builds a throwaway temp repo containing a flow/state.json cursor and a
// flow/state-machine.json (copied byte-for-byte from the real repo so
// fixtures never drift from the actual machine definition). Returns paths
// and a cleanup() that removes the whole sandbox.
export function makeSandbox({ cursor } = {}) {
  const dir = mkdtempSync(path.join(tmpdir(), 'vibe-engine-test-'));
  const flowDir = path.join(dir, 'flow');
  mkdirSync(flowDir, { recursive: true });

  const machineSrc = path.join(REPO_ROOT, 'flow', 'state-machine.json');
  const machinePath = path.join(flowDir, 'state-machine.json');
  copyFileSync(machineSrc, machinePath);

  const cursorPath = path.join(flowDir, 'state.json');
  const cursorBody = cursor ?? {
    flow: 'idle',
    phase: 'idle',
    feature: null,
    updated: '2026-01-01T00:00:00Z',
  };
  writeFileSync(cursorPath, `${JSON.stringify(cursorBody, null, 2)}\n`);

  // Marker so self-relative/marker root resolution has something to find on
  // a bare non-git target, mirroring the "stranger eval" fixture shape.
  mkdirSync(path.join(dir, '.spec'), { recursive: true });

  let cleaned = false;
  function cleanup() {
    if (cleaned) return;
    cleaned = true;
    rmSync(dir, { recursive: true, force: true });
  }

  return { dir, flowDir, cursorPath, machinePath, cleanup };
}

// ---------------------------------------------------------------------------
// Test registry + runner
// ---------------------------------------------------------------------------

const registry = []; // { file, name, fn }
let currentFile = '(unknown)';

function setCurrentFile(file) {
  currentFile = file;
}

// Test files call: import { test } from './run.mjs'; test('name', async () => {...})
export function test(name, fn) {
  registry.push({ file: currentFile, name, fn });
}

function discoverTestFiles() {
  return readdirSync(TESTS_DIR)
    .filter((f) => f.endsWith('.test.mjs'))
    .sort()
    .map((f) => path.join(TESTS_DIR, f));
}

async function loadTestFiles(files) {
  for (const file of files) {
    setCurrentFile(path.basename(file));
    await import(pathToFileURL(file).href);
  }
}

function selectTests(filters) {
  if (filters.length === 0) return registry;
  return registry.filter(
    ({ file, name }) => filters.some((f) => file.includes(f) || name.includes(f)),
  );
}

async function main() {
  const filters = process.argv.slice(2);
  const files = discoverTestFiles();

  if (files.length === 0) {
    // Fail loud: a runner that reports "0 total, exit 0" on broken
    // discovery is indistinguishable from a runner that ran everything and
    // found it clean. Units 2-8 drive this via filtered runs — a typo or a
    // discovery bug must not read as green.
    console.error('no test files found (engine/tests/*.test.mjs) — treating as a failure');
    process.exitCode = 1;
    return;
  }

  await loadTestFiles(files);

  const selected = selectTests(filters);
  if (filters.length > 0 && selected.length === 0) {
    console.error(`no tests matched filter(s): ${filters.join(', ')} — treating as a failure`);
    process.exitCode = 1;
    return;
  }

  let pass = 0;
  let fail = 0;
  let skipped = 0;

  for (const { file, name, fn } of selected) {
    try {
      await fn();
      pass += 1;
      console.log(`  ok    ${file} :: ${name}`);
    } catch (err) {
      if (err instanceof Skipped) {
        skipped += 1;
        console.log(`  skip  ${file} :: ${name} (${err.message})`);
        continue;
      }
      fail += 1;
      console.log(`  FAIL  ${file} :: ${name}`);
      console.log(`        ${err && err.stack ? err.stack : err}`);
    }
  }

  console.log('');
  console.log(`${pass} passed, ${fail} failed, ${skipped} skipped, ${selected.length} total`);

  process.exitCode = fail > 0 ? 1 : 0;
}

const isMain = process.argv[1] && path.resolve(process.argv[1]) === __filename;
if (isMain) {
  main().catch((err) => {
    console.error('test runner crashed:', err && err.stack ? err.stack : err);
    process.exitCode = 1;
  });
}
