// engine/tests/primitives.test.mjs — the duplicate-primitive scan (js-core/8,
// R1, R5).
//
// tech.md's contract: "Commands resolve their dirs once at dispatch and use
// these functions; none re-derives a primitive, parses cursor or machine
// JSON directly, or hardcodes a layout path." This file is what makes that
// mechanical instead of reviewed: scanCommandsDir() below statically scans
// every flow/engine/commands/*.mjs file (hook.mjs included — it is NOT a
// bash-oracle port, but the SAME single-primitive contract applies to it,
// per the task brief) for three concrete re-derivation shapes:
//
//   1. A direct cursor/machine JSON parse — `JSON.parse(...readFileSync...)`
//      near a literal 'state.json'/'state-machine.json' path, bypassing
//      readCursor()/loadMachine() (cursor.mjs / machine.mjs).
//   2. A raw re-derivation of machinePath()'s own join — `path.join(...,
//      'state-machine.json')` or `joinMaybe(..., 'state-machine.json')`,
//      instead of calling the exported machinePath(vibeDir).
//   3. A raw read of `process.env.CLAUDE_PROJECT_DIR`, instead of calling
//      the exported resolveProjectCursorDir() (root.mjs), which single-
//      sources both the env lookup AND the `.agents/skills/vibe` layout
//      constant for that ONE precedence rule.
//
// What is deliberately NOT banned (false-positive traps a cruder scan would
// fall into): a bare `path.join(vibeDir, 'state.json')` for an EXISTENCE
// check only (doctor.mjs's checkCursor) — tech.md is explicit that no
// primitive owns that literal, so a raw join is the sanctioned shape, same
// as SKILL.md/deps.json/.claude/** joins. Also not banned: hook.mjs's own
// `path.join(root, '.agents', 'skills', 'vibe')` (vibeLogDir) — that mirrors
// the ORIGINAL hook scripts' own root-relative literals for the warnings
// log and evidence receipts, a different question from resolveVibeDir()'s
// or resolveProjectCursorDir()'s, so there is nothing to single-source it
// against (see hook.mjs's own header). And plain mentions of the filenames
// in warn()/ok() MESSAGE TEXT (doctor.mjs's checkMachine, for instance) —
// only actual path-construction and JSON-parse call SHAPES trip the scan,
// never a string that happens to contain "state.json" as prose.
//
// Doc comments in these files narrate exactly these review lessons using
// the literal syntax of the banned shapes (e.g. doctor.mjs's own comment
// quotes `joinMaybe(vibeDir, 'state-machine.json')` as the bug it fixed) —
// so comments are stripped before scanning, not just as tidiness but because
// skipping that step would make the scan permanently red on files that are
// already correct.
//
// Per the task brief: doctrine's CLAUDE_PROJECT_DIR cursor axis is NOT
// folded into any shared parity helper here — this file only asserts the
// scan's own shape, never re-implements doctrine's precedence rule.

import { readFileSync, readdirSync, mkdtempSync, cpSync, writeFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { test, assert, assertEqual } from './run.mjs';

const REPO_ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..', '..', '..');
const COMMANDS_DIR = path.join(REPO_ROOT, 'flow', 'engine', 'commands');

// ---------------------------------------------------------------------------
// The scan itself.
// ---------------------------------------------------------------------------

// Strips `//` line comments. None of these files use `/* */` block comments
// (verified: zero `*/` occurrences anywhere in commands/*.mjs — the only
// `/*`-looking substrings are `.claude/**` glob text inside `//` comments),
// and none embed a `//` inside a string literal that matters here, so a
// naive per-line cut at the first `//` is exact for this codebase, not an
// approximation that could silently swallow code.
function stripLineComments(src) {
  return src
    .split('\n')
    .map((line) => {
      const idx = line.indexOf('//');
      return idx === -1 ? line : line.slice(0, idx);
    })
    .join('\n');
}

const RAW_MACHINE_JOIN_RE = /\b(?:path\.join|joinMaybe)\([^)]*['"]state-machine\.json['"]/;
const PROJECT_DIR_ENV_RE = /process\.env(?:\.CLAUDE_PROJECT_DIR\b|\[\s*['"]CLAUDE_PROJECT_DIR['"]\s*\])/;
const CURSOR_OR_MACHINE_FILE_RE = /state(?:-machine)?\.json/;

// Scans one already-decommented source string, returning every violation
// label that applies (a file can trip more than one).
function scanSource(text) {
  const hits = [];

  // 1. JSON.parse(...readFileSync...) with a cursor/machine filename nearby
  // — a small forward window, not the whole file, so an unrelated
  // JSON.parse elsewhere in the same file (hook.mjs's stdin parse, for
  // instance) can never combine with a state.json mention far away to
  // produce a false positive.
  let idx = text.indexOf('JSON.parse');
  while (idx !== -1) {
    const window = text.slice(idx, idx + 300);
    if (/readFileSync/.test(window) && CURSOR_OR_MACHINE_FILE_RE.test(window)) {
      hits.push('direct-json-parse');
      break;
    }
    idx = text.indexOf('JSON.parse', idx + 1);
  }

  // 2. A raw re-derivation of machinePath()'s own join.
  if (RAW_MACHINE_JOIN_RE.test(text)) {
    hits.push('raw-machine-join');
  }

  // 3. A raw read of CLAUDE_PROJECT_DIR — resolveProjectCursorDir()'s job.
  if (PROJECT_DIR_ENV_RE.test(text)) {
    hits.push('raw-project-dir-env');
  }

  return hits;
}

const LABELS = {
  'direct-json-parse':
    "parses cursor/machine JSON directly (JSON.parse + readFileSync near a 'state.json'/'state-machine.json' path) — use readCursor()/loadMachine() instead",
  'raw-machine-join':
    "re-derives the state-machine.json path via a raw join — use machinePath() from machine.mjs instead",
  'raw-project-dir-env':
    'reads process.env.CLAUDE_PROJECT_DIR directly — use resolveProjectCursorDir() from root.mjs instead',
};

// Scans every *.mjs file directly under `dir` (flow/engine/commands/, or a
// scratch copy of it), returning a flat array of "file: reason" violation
// strings — empty when the directory is clean. Exported implicitly via the
// module scope only (test-only helper, not part of the shipped engine).
function scanCommandsDir(dir) {
  const violations = [];
  const files = readdirSync(dir)
    .filter((f) => f.endsWith('.mjs'))
    .sort();
  for (const file of files) {
    const raw = readFileSync(path.join(dir, file), 'utf8');
    const src = stripLineComments(raw);
    for (const hit of scanSource(src)) {
      violations.push(`${file}: ${LABELS[hit]}`);
    }
  }
  return violations;
}

// ---------------------------------------------------------------------------
// The real repo — must be, and stay, clean. This is the mechanical form of
// tech.md's "Commands resolve their dirs once at dispatch..." contract:
// doctor.mjs (machinePath()) and doctrine.mjs (resolveProjectCursorDir())
// are exactly the two commands that own the primitives this scan protects,
// so their continued PASS here is the proof those exemptions need nothing
// special — the primitives themselves are what make the scan pass.
// ---------------------------------------------------------------------------

test('duplicate-primitive scan: flow/engine/commands/ (including hook.mjs) is clean', () => {
  const files = readdirSync(COMMANDS_DIR).filter((f) => f.endsWith('.mjs'));
  assert(files.includes('hook.mjs'), 'sanity: hook.mjs must be in scope for this scan');
  assert(files.length >= 5, `sanity: expected at least 5 command modules, found ${files.length}: ${files.join(', ')}`);

  const violations = scanCommandsDir(COMMANDS_DIR);
  assertEqual(violations, [], `duplicate-primitive scan found violations:\n${violations.join('\n')}`);
});

// ---------------------------------------------------------------------------
// Discriminating — per the task brief and the uninstall-test lesson, a scan
// that never fires is worse than none. Each case below mutates a SCRATCH
// COPY of commands/ (never the real tree) to plant exactly one banned
// shape, and asserts scanCommandsDir() catches it. Every case first proves
// the untouched copy is clean, so a failure here is provably caused by the
// planted mutation, not a pre-existing false positive.
// ---------------------------------------------------------------------------

function makeScratchCommandsDir() {
  const dir = mkdtempSync(path.join(tmpdir(), 'vibe-primscan-'));
  cpSync(COMMANDS_DIR, dir, { recursive: true });
  return dir;
}

test('discriminating: the scan FAILS when a command is edited to re-read the cursor directly (not via readCursor())', () => {
  const dir = makeScratchCommandsDir();
  try {
    assertEqual(scanCommandsDir(dir), [], 'sanity: an untouched scratch copy must start clean');

    const target = path.join(dir, 'state.mjs');
    const patched =
      `${readFileSync(target, 'utf8')}\n` +
      `import fsDup from 'node:fs';\n` +
      `import pathDup from 'node:path';\n` +
      `function duplicatedCursorRead(vibeDir) {\n` +
      `  return JSON.parse(fsDup.readFileSync(pathDup.join(vibeDir, 'state.json'), 'utf8'));\n` +
      `}\n`;
    writeFileSync(target, patched);

    const violations = scanCommandsDir(dir);
    assert(violations.length > 0, 'expected the scan to flag the duplicated cursor reader');
    assert(
      violations.some((v) => v.startsWith('state.mjs:') && v.includes('JSON.parse')),
      `expected a state.mjs direct-json-parse violation, got: ${JSON.stringify(violations)}`,
    );
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test('discriminating: the scan FAILS when a command re-derives machinePath()\'s own join', () => {
  const dir = makeScratchCommandsDir();
  try {
    const target = path.join(dir, 'orders.mjs'); // already imports `path` and `fs`
    const patched =
      `${readFileSync(target, 'utf8')}\n` +
      `function duplicatedMachinePath(vibeDir) {\n` +
      `  return path.join(vibeDir, 'state-machine.json');\n` +
      `}\n`;
    writeFileSync(target, patched);

    const violations = scanCommandsDir(dir);
    assert(
      violations.some((v) => v.startsWith('orders.mjs:') && v.includes('machinePath')),
      `expected an orders.mjs raw-machine-join violation, got: ${JSON.stringify(violations)}`,
    );
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test('discriminating: the scan FAILS when a command reads CLAUDE_PROJECT_DIR directly instead of resolveProjectCursorDir()', () => {
  const dir = makeScratchCommandsDir();
  try {
    const target = path.join(dir, 'orders.mjs');
    const patched =
      `${readFileSync(target, 'utf8')}\n` +
      `function duplicatedProjectDirRead() {\n` +
      `  return process.env.CLAUDE_PROJECT_DIR;\n` +
      `}\n`;
    writeFileSync(target, patched);

    const violations = scanCommandsDir(dir);
    assert(
      violations.some((v) => v.startsWith('orders.mjs:') && v.includes('CLAUDE_PROJECT_DIR')),
      `expected an orders.mjs raw-project-dir-env violation, got: ${JSON.stringify(violations)}`,
    );
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test('discriminating: the scan does NOT flag hook.mjs\'s own root-relative .agents/skills/vibe join (vibeLogDir) — a different, sanctioned literal', () => {
  // Regression guard for the scan's OWN false-positive risk: hook.mjs
  // legitimately joins '.agents'/'skills'/'vibe' off ROOT for the warnings
  // log and evidence receipts (mirrors the original .sh hooks' own
  // literals — see hook.mjs's header). If a future tightening of this scan
  // started flagging any '.agents'+'skills'+'vibe' combination, this would
  // catch it before it broke the real, already-reviewed hook.mjs.
  const violations = scanCommandsDir(COMMANDS_DIR).filter((v) => v.startsWith('hook.mjs:'));
  assertEqual(violations, [], `hook.mjs must stay clean under the real scan, got: ${JSON.stringify(violations)}`);
});

test('discriminating: comments describing the banned shapes (doctor.mjs\'s own review-lesson prose) do not trip the scan', () => {
  // doctor.mjs's machine-check comment literally quotes
  // `joinMaybe(vibeDir, 'state-machine.json')` as the bug round 2 removed —
  // proof the scan strips comments rather than string-matching the raw
  // file, or this file would be permanently (and wrongly) red.
  const raw = readFileSync(path.join(COMMANDS_DIR, 'doctor.mjs'), 'utf8');
  assert(
    raw.includes("joinMaybe(vibeDir,") && raw.includes("'state-machine.json')"),
    'sanity: doctor.mjs must still carry the review-lesson comment this test depends on',
  );
  const violations = scanCommandsDir(COMMANDS_DIR).filter((v) => v.startsWith('doctor.mjs:'));
  assertEqual(violations, [], `doctor.mjs must stay clean under the real scan, got: ${JSON.stringify(violations)}`);
});
