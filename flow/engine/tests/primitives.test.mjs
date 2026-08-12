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

import { readFileSync, readdirSync, mkdtempSync, mkdirSync, cpSync, writeFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { test, assert, assertEqual } from './run.mjs';

const REPO_ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..', '..', '..');
const ENGINE_DIR = path.join(REPO_ROOT, 'flow', 'engine');
const COMMANDS_DIR = path.join(ENGINE_DIR, 'commands');

// ---------------------------------------------------------------------------
// The comment stripper.
//
// The scan below bans INGREDIENTS (filename literals, env-var names, layout
// constants) rather than call shapes, so it must never see prose. These
// modules' doc comments quote every banned ingredient at length — on purpose,
// they narrate the review lessons that produced the bans — so a scan that
// string-matched raw source would be permanently, wrongly red. That
// false-positive pressure is exactly why the js-core/8 scan stayed narrow;
// stripping comments properly is what buys the room to widen it.
//
// A naive "cut each line at the first //" is NOT sufficient and is not used:
// hook.mjs's `/(^|\/)src\//` regex literal ends in `\//`, which contains the
// two-character sequence `//`, so a naive cut would silently swallow the rest
// of that line — and with it any violation living there. Hence a real
// tokenizer that tracks string literals, template literals (including code
// inside `${}`), regex literals, and both comment forms.
//
// Comments are replaced with spaces (newlines preserved) rather than deleted,
// so offsets and line numbers in the stripped text still address the original
// file — that is what lets a violation report its real line number.
// ---------------------------------------------------------------------------

// A `/` starts a regex literal (rather than division) only in an operand
// position. Single-char lookback covers the operator cases; the word set
// covers the keyword cases (`return /re/`, `typeof /re/`, ...). `)` and `]`
// and identifiers/numbers are deliberately absent — after those, `/` is
// division.
const REGEX_OK_CHARS = new Set([
  '', '(', ',', '=', ':', '[', '!', '&', '|', '?', '{', '}', ';', '+', '-', '*', '%', '~', '^', '<', '>',
]);
const REGEX_OK_WORDS = new Set([
  'return', 'typeof', 'case', 'in', 'of', 'do', 'else', 'yield', 'await', 'new', 'delete', 'void',
  'instanceof', 'throw',
]);

function isWordChar(c) {
  return /[A-Za-z0-9_$]/.test(c);
}

// The identifier ending at index `end` (exclusive), or '' if the character
// there is not a word character.
function wordEndingAt(src, end) {
  let i = end;
  while (i > 0 && isWordChar(src[i - 1])) i -= 1;
  return src.slice(i, end);
}

// Consumes a quoted string starting at `i` (src[i] is the quote); returns the
// index just past the closing quote.
function endOfQuoted(src, i, quote) {
  let j = i + 1;
  while (j < src.length) {
    const c = src[j];
    if (c === '\\') {
      j += 2;
      continue;
    }
    if (c === quote) return j + 1;
    if (c === '\n' && quote !== '`') return j; // unterminated — degrade, never loop forever
    j += 1;
  }
  return src.length;
}

// Consumes a regex literal starting at `i` (src[i] === '/'); returns the index
// just past the closing `/` plus flags, or -1 if no closer is found on the
// line (in which case the `/` was not a regex after all).
function endOfRegex(src, i) {
  let j = i + 1;
  let inClass = false;
  while (j < src.length) {
    const c = src[j];
    if (c === '\\') {
      j += 2;
      continue;
    }
    if (c === '\n') return -1;
    if (inClass) {
      if (c === ']') inClass = false;
    } else if (c === '[') {
      inClass = true;
    } else if (c === '/') {
      j += 1;
      while (j < src.length && /[a-z]/.test(src[j])) j += 1; // flags
      return j;
    }
    j += 1;
  }
  return -1;
}

export function stripComments(src) {
  let out = '';
  let i = 0;
  let prevSig = ''; // last significant CODE character emitted
  let prevSigIdx = -1;
  // Context stack: 'code' frames track brace depth so a `}` closing a
  // template-literal `${` expression pops back into the template.
  const stack = [{ kind: 'code', brace: 0, root: true }];

  const emit = (s) => {
    out += s;
  };

  while (i < src.length) {
    const ctx = stack[stack.length - 1];
    const c = src[i];
    const d = src[i + 1];

    if (ctx.kind === 'template') {
      if (c === '\\') {
        emit(src.slice(i, i + 2));
        i += 2;
        continue;
      }
      if (c === '`') {
        emit(c);
        i += 1;
        stack.pop();
        prevSig = '`';
        prevSigIdx = i;
        continue;
      }
      if (c === '$' && d === '{') {
        emit('${');
        i += 2;
        stack.push({ kind: 'code', brace: 0 });
        prevSig = '';
        prevSigIdx = -1;
        continue;
      }
      emit(c);
      i += 1;
      continue;
    }

    if (c === '/' && d === '/') {
      let j = i;
      while (j < src.length && src[j] !== '\n') j += 1;
      emit(' '.repeat(j - i));
      i = j;
      continue;
    }

    if (c === '/' && d === '*') {
      const close = src.indexOf('*/', i + 2);
      const j = close === -1 ? src.length : close + 2;
      for (let k = i; k < j; k += 1) emit(src[k] === '\n' ? '\n' : ' ');
      i = j;
      continue;
    }

    if (c === '"' || c === "'") {
      const j = endOfQuoted(src, i, c);
      emit(src.slice(i, j));
      prevSig = c;
      prevSigIdx = j;
      i = j;
      continue;
    }

    if (c === '`') {
      emit(c);
      i += 1;
      stack.push({ kind: 'template' });
      continue;
    }

    if (c === '/') {
      const word = prevSigIdx >= 0 ? wordEndingAt(src, prevSigIdx) : '';
      const regexAllowed = REGEX_OK_CHARS.has(prevSig) || REGEX_OK_WORDS.has(word);
      if (regexAllowed) {
        const j = endOfRegex(src, i);
        if (j !== -1) {
          emit(src.slice(i, j));
          prevSig = '/';
          prevSigIdx = j;
          i = j;
          continue;
        }
      }
    }

    if (c === '{') ctx.brace += 1;
    if (c === '}') {
      if (ctx.brace === 0 && !ctx.root) {
        emit(c);
        i += 1;
        stack.pop();
        prevSig = '}';
        prevSigIdx = i;
        continue;
      }
      ctx.brace -= 1;
    }

    emit(c);
    if (!/\s/.test(c)) {
      prevSig = c;
      prevSigIdx = i + 1;
    }
    i += 1;
  }

  return out;
}

// ---------------------------------------------------------------------------
// Source discovery — every *.mjs under flow/engine/, RECURSIVELY, returned as
// paths relative to `dir`.
//
// EXEMPTION (the only one): flow/engine/tests/. Test files legitimately name
// state.json, build .agents/skills/vibe fixture layouts, and re-derive paths
// on purpose — they are fixtures, not shipped engine code, and R1's
// "one implementation per primitive" is a statement about the engine, not
// about its test scaffolding. Nothing else under flow/engine/ is exempt.
// ---------------------------------------------------------------------------

const EXCLUDED_DIRS = new Set(['tests']);

export function listEngineSources(dir, prefix = '') {
  const out = [];
  for (const entry of readdirSync(dir, { withFileTypes: true }).sort((a, b) => (a.name < b.name ? -1 : 1))) {
    const rel = prefix ? `${prefix}/${entry.name}` : entry.name;
    if (entry.isDirectory()) {
      if (EXCLUDED_DIRS.has(entry.name)) continue;
      out.push(...listEngineSources(path.join(dir, entry.name), rel));
    } else if (entry.isFile() && entry.name.endsWith('.mjs')) {
      out.push(rel);
    }
  }
  return out;
}

// ---------------------------------------------------------------------------
// stripComments() tests (RED first — see the tokenizer below).
// ---------------------------------------------------------------------------

test('stripComments: removes a line comment but keeps the code before it', () => {
  assertEqual(stripComments('const a = 1; // state.json\n'), `const a = 1;${' '.repeat(14)}\n`);
});

test('stripComments: does NOT treat // inside a string literal as a comment', () => {
  assertEqual(stripComments("const u = 'http://x'; const b = 2;\n"), "const u = 'http://x'; const b = 2;\n");
});

test('stripComments: does NOT treat // inside a regex literal as a comment', () => {
  const src = 'const t = /(^|\\/)src\\//.test(l);\n';
  assertEqual(stripComments(src), src);
});

test('stripComments: removes a block comment, preserving newlines so line numbers survive', () => {
  assertEqual(stripComments('a;/* x\ny */b;\n'), `a;${' '.repeat(4)}\n${' '.repeat(4)}b;\n`);
});

test('stripComments: keeps /* inside a string literal', () => {
  const src = "const g = '.claude/**/*';\n";
  assertEqual(stripComments(src), src);
});

test('stripComments: keeps code inside a template literal ${} expression', () => {
  const src = 'const s = `a${process.env.CLAUDE_PROJECT_DIR}b`;\n';
  assertEqual(stripComments(src), src);
});

test('stripComments: strips a comment inside a template literal ${} expression', () => {
  assertEqual(stripComments('const s = `a${x // c\n}b`;\n'), 'const s = `a${x     \n}b`;\n');
});

test('stripComments: preserves total length and line count on every real engine module', () => {
  for (const rel of listEngineSources(ENGINE_DIR)) {
    const raw = readFileSync(path.join(ENGINE_DIR, rel), 'utf8');
    const stripped = stripComments(raw);
    assertEqual(stripped.length, raw.length, `${rel}: stripper must be length-preserving`);
    assertEqual(
      stripped.split('\n').length,
      raw.split('\n').length,
      `${rel}: stripper must preserve line count`,
    );
    assertEqual(stripComments(stripped), stripped, `${rel}: stripper must be idempotent`);
  }
});

// ---------------------------------------------------------------------------
// The invariant: INGREDIENTS, not shapes.
//
// The previous form of this scan matched call SHAPES — `JSON.parse` within
// 300 characters of `readFileSync` and a `'state.json'` literal, and two
// hand-written path-join regexes. Review round 1 proved that approach
// unsound in the only way that matters: the repo's OWN house style evaded
// it. `const raw = readFileSync(p, 'utf8'); const j = JSON.parse(raw);` is
// literally what json.mjs:11-14 does, and it walked straight through. So did
// `readJson(path.join(vibeDir, 'state.json'))`, an aliased import, a path
// held in a variable, `path.resolve` instead of `path.join`, a template
// literal, a destructured `process.env`, and an aliased `process.env`.
//
// Adding a regex per evasion is the same defect with a longer list — sound
// proximity matching needs a real parser, and this engine ships with zero
// dependencies. So the invariant is INVERTED. Instead of detecting HOW a file
// re-derives a primitive, forbid the file from holding the INGREDIENTS at
// all:
//
//   * a module that never names 'state.json' cannot read or write the cursor
//     — it must go through readCursor()/writeCursor()
//   * a module that never names 'state-machine.json' cannot load the machine
//     — it must go through machinePath()/loadMachine()
//   * a module that never names CLAUDE_PROJECT_DIR, `.agents`, `.spec`/`.git`,
//     `process.cwd`, or `import.meta.url` has no leg of resolveRoot()/
//     resolveVibeDir()/resolveProjectCursorDir() left to re-derive
//   * a module that never writes the `<!--` marker grammar cannot
//     re-implement extractBlock()
//
// That covers all five primitives R1 names (root resolver, cursor reader,
// cursor writer, machine loader, block extractor) and is spelling-agnostic:
// every evasion above still has to name the file, the variable, or the
// grammar somewhere in the module.
//
// KNOWN LIMIT (stated, not papered over): a whole-file literal ban cannot see
// through string CONCATENATION or computed names — `'state' + '.json'`,
// `['state', 'json'].join('.')`, `Buffer.from(...)`. Closing that needs an
// AST, which needs a dependency this engine will not take (R5). This is the
// strongest sound subset, not a complete one.
//
// OWNERS vs WAIVERS. An ingredient's `owners` are the modules that define the
// primitive — unrestricted there, by design (machinePath() must build the
// machine path inside machine.mjs; resolveProjectCursorDir() must know the
// layout inside root.mjs). Everything else needs a WAIVER: an exact,
// individually-reasoned source line. Waivers are matched on the full trimmed
// line text, so editing a waived line, or adding a second occurrence
// anywhere, trips the scan again. Stale waivers fail too (see the
// "every waiver is still used" test), so the list cannot silently rot into a
// blanket exemption.
// ---------------------------------------------------------------------------

const INGREDIENTS = [
  {
    id: 'cursor-file',
    // 'state.json' plain, or with the dot regex-escaped (`state\.json`).
    re: /state\\?\.json/,
    owners: ['cursor.mjs'],
    why: "names the cursor file — read/write it through readCursor()/writeCursor() (cursor.mjs), which own that filename",
  },
  {
    id: 'machine-file',
    re: /state-machine\\?\.json/,
    owners: ['machine.mjs'],
    why: "names the state-machine file — get its path from machinePath() and read it through loadMachine() (machine.mjs)",
  },
  {
    id: 'project-dir-env',
    re: /CLAUDE_PROJECT_DIR/,
    owners: ['root.mjs'],
    why: 'names CLAUDE_PROJECT_DIR — resolveRoot()/resolveProjectCursorDir() (root.mjs) own that precedence rule',
  },
  {
    id: 'vibe-layout',
    re: /\\?\.agents/,
    owners: ['root.mjs'],
    why: 'hardcodes the `.agents/skills/vibe` layout — resolveVibeDir()/resolveProjectCursorDir() (root.mjs) single-source it',
  },
  {
    id: 'root-markers',
    re: /(^|[^A-Za-z0-9_$.])\\?\.(spec|git)\b/,
    owners: ['root.mjs'],
    why: "re-derives the upward `.spec`/`.git` marker search — resolveRoot() (root.mjs) owns it",
  },
  {
    id: 'cwd-fallback',
    re: /process\.cwd/,
    owners: ['root.mjs'],
    why: "re-derives resolveRoot()'s cwd fallback leg — root.mjs owns it",
  },
  {
    id: 'self-location',
    re: /import\.meta\.url/,
    owners: ['root.mjs'],
    why: "self-locates from the module's own URL — that is resolveRoot()/resolveVibeDir()'s ingredient (root.mjs)",
  },
  {
    id: 'marker-grammar',
    re: /<!--|-->/,
    owners: ['blocks.mjs'],
    why: 'writes the marker-block grammar — extractBlock() (blocks.mjs) is the one grammar',
  },
];

// Waiver reason codes. Each says WHY the occurrence cannot re-derive a
// primitive — none of them constructs a path or reads a file, with the single
// documented exception of `hook-root-literal` (called out in its own text).
const REASONS = {
  'oracle-text':
    'byte-parity MESSAGE TEXT copied from the bash oracle — a string printed to the user, never a path or a read. Changing it breaks R2 parity.',
  'bash-sniffer':
    "mirrors detect-context.sh's guarded-path CLASSES for the warn-only Bash sniffer — a pattern matched against someone else's shell command, not a path this engine builds or opens.",
  'hook-root-literal':
    "mirrors the ORIGINAL .sh hook's own $ROOT-relative literal for the warnings log / evidence receipts (hook.mjs header; js-core/7 review). This IS a real layout literal — the waiver is per-LINE precisely so a SECOND one cannot be added silently.",
  'layout-name-probe':
    'a path.basename() equality check on a directory NAME, walking structure off an already-resolved primitive — no path is constructed and no new resolution algorithm exists (doctor.mjs rootForReport(), round-1 Finding 3).',
  'marker-presence-probe':
    "an opener-only substring probe, deliberately NOT extractBlock(): the oracle's `grep -q '<!-- vibe:doctrine -->'` succeeds on an opener with no closer, where extractBlock() correctly returns undefined. Routing it through blocks.mjs would change behaviour and break parity.",
  'cli-self-dispatch':
    'cli.mjs locates its own commands/ directory for module dispatch, not the repo root. A one-LINE waiver rather than an owner entry, so a second import.meta.url line here — a smuggled root resolver — still trips the scan.',
  'spec-sanctioned-exemption':
    "tech.md's Contract defines resolveProjectCursorDir() as doctrine's CLAUDE_PROJECT_DIR-first cursor rule 'gated on that state.json existing' — knowing the filename is its documented job. NOT routed through cursorPath() because root.mjs is the self-location primitive and must stay importable alone (root.test.mjs copies this single file into synthetic install layouts); importing cursor.mjs would drag json.mjs in behind it.",
};

// Exact-line waivers. `line` is matched against the TRIMMED source line after
// comment-stripping, so a change to the line — or a second occurrence
// anywhere else in the file — is a violation again.
const WAIVERS = [
  { file: 'cli.mjs', id: 'self-location', reason: 'cli-self-dispatch', line: 'const __filename = fileURLToPath(import.meta.url);' },

  { file: 'root.mjs', id: 'cursor-file', reason: 'spec-sanctioned-exemption', line: "return fs.existsSync(path.join(candidate, 'state.json')) ? candidate : undefined;" },

  { file: 'commands/doctor.mjs', id: 'machine-file', reason: 'oracle-text', line: 'return warn(\'machine\', `state-machine.json missing at ${p} — flow harness incomplete`);' },
  { file: 'commands/doctor.mjs', id: 'machine-file', reason: 'oracle-text', line: "return warn('machine', 'state-machine.json is present but not valid JSON');" },
  { file: 'commands/doctor.mjs', id: 'machine-file', reason: 'oracle-text', line: "return ok('machine', 'state-machine.json present');" },
  { file: 'commands/doctor.mjs', id: 'marker-grammar', reason: 'marker-presence-probe', line: "doctrineBlock = fs.readFileSync(skillMdPath, 'utf8').includes('<!-- vibe:doctrine -->');" },
  { file: 'commands/doctor.mjs', id: 'marker-grammar', reason: 'oracle-text', line: "'no doctrine coverage — no <!-- vibe:doctrine --> block, no wired SessionStart hook, no per-user plugin; run install.sh (--local or --global) / setup.apply'," },
  { file: 'commands/doctor.mjs', id: 'vibe-layout', reason: 'layout-name-probe', line: "if (path.basename(agentsDir) === '.agents') {" },

  { file: 'commands/hook.mjs', id: 'vibe-layout', reason: 'hook-root-literal', line: "return path.join(root, '.agents', 'skills', 'vibe');" },
  { file: 'commands/hook.mjs', id: 'root-markers', reason: 'bash-sniffer', line: 'const LESSONS_RE = /(^|[^A-Za-z0-9_])\\.spec\\/lessons\\.md/;' },
  { file: 'commands/hook.mjs', id: 'root-markers', reason: 'bash-sniffer', line: 'const ROOT_SPEC_RE = /(^|[^A-Za-z0-9_])\\.spec\\/(product|tech|design|plan)\\.md/;' },
  { file: 'commands/hook.mjs', id: 'root-markers', reason: 'bash-sniffer', line: "if (LESSONS_RE.test(cmd)) return '.spec/lessons.md';" },
  { file: 'commands/hook.mjs', id: 'root-markers', reason: 'bash-sniffer', line: "if (ROOT_SPEC_RE.test(cmd)) return 'a root .spec/{product,tech,design,plan}.md doc';" },
  { file: 'commands/hook.mjs', id: 'cursor-file', reason: 'bash-sniffer', line: 'const STATE_JSON_RE = /(\\.agents\\/skills\\/vibe\\/state\\.json|(^|[^A-Za-z0-9_])flow\\/state\\.json)/;' },
  { file: 'commands/hook.mjs', id: 'vibe-layout', reason: 'bash-sniffer', line: 'const STATE_JSON_RE = /(\\.agents\\/skills\\/vibe\\/state\\.json|(^|[^A-Za-z0-9_])flow\\/state\\.json)/;' },
  { file: 'commands/hook.mjs', id: 'cursor-file', reason: 'bash-sniffer', line: "return '.agents/skills/vibe/state.json (use set-state.sh)';" },
  { file: 'commands/hook.mjs', id: 'vibe-layout', reason: 'bash-sniffer', line: "return '.agents/skills/vibe/state.json (use set-state.sh)';" },
  { file: 'commands/hook.mjs', id: 'vibe-layout', reason: 'oracle-text', line: 'line(`  not verifying? abort with: bash .agents/skills/vibe/scripts/set-state.sh idle`),' },
  { file: 'commands/hook.mjs', id: 'vibe-layout', reason: 'oracle-text', line: "line('  not verifying? abort with: bash .agents/skills/vibe/scripts/set-state.sh idle')," },
  { file: 'commands/hook.mjs', id: 'vibe-layout', reason: 'hook-root-literal', line: "const evidRel = '.agents/skills/vibe/evidence';" },

  { file: 'commands/orders.mjs', id: 'machine-file', reason: 'oracle-text', line: "'state=unknown · read .agents/skills/vibe/state-machine.json and pick the matching vibe phase · transition via set-state.sh';" },
  { file: 'commands/orders.mjs', id: 'vibe-layout', reason: 'oracle-text', line: "'state=unknown · read .agents/skills/vibe/state-machine.json and pick the matching vibe phase · transition via set-state.sh';" },
];

function waiverKey(w) {
  return `${w.file}|${w.id}|${w.line}`;
}

// ---------------------------------------------------------------------------
// The scan.
// ---------------------------------------------------------------------------

// Returns a flat, sorted array of violation strings — empty when the tree is
// clean. `dir` is a flow/engine/ root (the real one, or a scratch copy).
export function scanEngineTree(dir) {
  const violations = [];
  for (const rel of listEngineSources(dir)) {
    const src = stripComments(readFileSync(path.join(dir, rel), 'utf8'));
    const lines = src.split('\n');
    for (const ing of INGREDIENTS) {
      if (ing.owners.includes(rel)) continue;
      for (let i = 0; i < lines.length; i += 1) {
        if (!ing.re.test(lines[i])) continue;
        const trimmed = lines[i].trim();
        const waived = WAIVERS.some((w) => w.file === rel && w.id === ing.id && w.line === trimmed);
        if (waived) continue;
        violations.push(`${rel}:${i + 1}: [${ing.id}] ${ing.why}\n    | ${trimmed}`);
      }
    }
  }
  return violations.sort();
}

// Which waivers actually fired during a scan of `dir` — used to fail on a
// stale waiver, so the list can never quietly become a blanket exemption.
export function usedWaivers(dir) {
  const used = new Set();
  for (const rel of listEngineSources(dir)) {
    const src = stripComments(readFileSync(path.join(dir, rel), 'utf8'));
    for (const line of src.split('\n')) {
      const trimmed = line.trim();
      for (const w of WAIVERS) {
        if (w.file !== rel || w.line !== trimmed) continue;
        const ing = INGREDIENTS.find((x) => x.id === w.id);
        if (ing && ing.re.test(line)) used.add(waiverKey(w));
      }
    }
  }
  return used;
}

// ---------------------------------------------------------------------------
// The real tree — must be, and stay, clean.
// ---------------------------------------------------------------------------

test('primitive scan: the whole flow/engine/ tree (recursively, tests/ excluded) is clean', () => {
  const files = listEngineSources(ENGINE_DIR);
  assert(files.includes('cli.mjs'), 'sanity: cli.mjs must be in scope for this scan');
  assert(files.includes('blocks.mjs'), 'sanity: blocks.mjs must be in scope for this scan');
  assert(files.includes('commands/hook.mjs'), 'sanity: commands/hook.mjs must be in scope for this scan');
  assert(
    files.length >= 11,
    `sanity: expected at least 11 engine modules in scope, found ${files.length}: ${files.join(', ')}`,
  );
  assert(
    !files.some((f) => f.startsWith('tests/')),
    `tests/ must be the ONLY exclusion and must actually be excluded, got: ${files.join(', ')}`,
  );

  const violations = scanEngineTree(ENGINE_DIR);
  assertEqual(violations, [], `primitive scan found violations:\n${violations.join('\n')}`);
});

test('primitive scan: every ingredient owner really carries its ingredient (no stale owner entries)', () => {
  for (const ing of INGREDIENTS) {
    for (const owner of ing.owners) {
      const src = stripComments(readFileSync(path.join(ENGINE_DIR, owner), 'utf8'));
      assert(
        ing.re.test(src),
        `owner ${owner} no longer carries ingredient '${ing.id}' — the exemption is stale, drop it`,
      );
    }
  }
});

test('primitive scan: every waiver is still used (a stale waiver is a blanket exemption in waiting)', () => {
  const used = usedWaivers(ENGINE_DIR);
  const stale = WAIVERS.filter((w) => !used.has(waiverKey(w))).map(waiverKey);
  assertEqual(stale, [], `stale waivers — the waived line is gone, so drop the waiver:\n${stale.join('\n')}`);
});

test('primitive scan: every waiver carries a known, spelled-out reason code', () => {
  for (const w of WAIVERS) {
    assert(
      Object.prototype.hasOwnProperty.call(REASONS, w.reason),
      `waiver ${waiverKey(w)} has no reason code — an unexplained waiver is how the round-1 defect was born`,
    );
    assert(
      INGREDIENTS.some((ing) => ing.id === w.id),
      `waiver ${waiverKey(w)} names an ingredient that no longer exists`,
    );
  }
});

test('primitive scan: comments quoting the banned ingredients do not trip it', () => {
  // Every module in this engine narrates the review lessons that produced
  // these bans, quoting `state.json`, `.agents/skills/vibe`,
  // `CLAUDE_PROJECT_DIR` and `<!-- vibe:doctrine -->` verbatim in prose. If
  // the stripper regressed, the clean-tree test above would go red on files
  // that are already correct — this pins the specific prose it depends on.
  const doctorRaw = readFileSync(path.join(COMMANDS_DIR, 'doctor.mjs'), 'utf8');
  assert(
    doctorRaw.includes("joinMaybe(vibeDir,") && doctorRaw.includes("'state-machine.json')"),
    "sanity: doctor.mjs must still carry the round-2 review-lesson comment this test depends on",
  );
  assertEqual(scanEngineTree(ENGINE_DIR).filter((v) => v.startsWith('commands/doctor.mjs')), []);
});

test('primitive scan: the stripper is load-bearing — a naive per-line // cut would corrupt hook.mjs', () => {
  // hook.mjs's `/(^|\/)src\//` regex literal ends in `\//`, which contains
  // the two-character sequence `//`. A naive "cut at the first //" stripper
  // silently swallows the rest of that line — and with it any violation
  // living there. This is why stripComments() is a tokenizer.
  const raw = readFileSync(path.join(COMMANDS_DIR, 'hook.mjs'), 'utf8');
  const naive = raw
    .split('\n')
    .map((l) => {
      const idx = l.indexOf('//');
      return idx === -1 ? l : l.slice(0, idx);
    })
    .join('\n');
  const proper = stripComments(raw);
  assert(
    proper.includes('/(^|\\/)src\\//.test(l)'),
    'the tokenizer must keep the whole regex-literal line intact',
  );
  assert(
    !naive.includes('/(^|\\/)src\\//.test(l)'),
    'sanity: the naive cut really does corrupt that line — if this ever fails, the hazard is gone and this test can go',
  );
});

// ---------------------------------------------------------------------------
// Discriminating — the mutant corpus.
//
// Round 1 planted these against the previous scan and proved that all but
// three walked through it. Each is a real re-derivation of one of R1's five
// primitives, spelled the way a developer working in THIS codebase would
// most naturally spell it. Every one is planted in three places, because
// round 1 also proved the scan's SCOPE was narrower than R1's scenario
// ("any file under engine/"): a command module, the CLI entry point, and a
// nested subdirectory that did not exist yet.
// ---------------------------------------------------------------------------

export const MUTANTS = [
  {
    id: 'M1 nested read+parse (the one shape the old scan caught)',
    primitive: 'cursor reader',
    code: "function dupCursorRead(vibeDir) {\n  return JSON.parse(fs.readFileSync(path.join(vibeDir, 'state.json'), 'utf8'));\n}\n",
  },
  {
    id: 'M2 split read-then-parse (json.mjs\'s own house style)',
    primitive: 'cursor reader',
    code: "function dupCursorRead(vibeDir) {\n  const raw = fs.readFileSync(path.join(vibeDir, 'state.json'), 'utf8');\n  return JSON.parse(raw);\n}\n",
  },
  {
    id: 'M3 aliased readFileSync import',
    primitive: 'cursor reader',
    code: "import { readFileSync as _rfs } from 'node:fs';\nfunction dupCursorRead(vibeDir) {\n  return JSON.parse(_rfs(path.join(vibeDir, 'state.json'), 'utf8'));\n}\n",
  },
  {
    id: 'M4 path held in a variable',
    primitive: 'cursor reader',
    code: "function dupCursorRead(vibeDir) {\n  const cursorFile = path.join(vibeDir, 'state.json');\n  return JSON.parse(fs.readFileSync(cursorFile, 'utf8'));\n}\n",
  },
  {
    id: "M15 the engine's own readJson() against the cursor path",
    primitive: 'cursor reader',
    code: "import { readJson as _rj } from '../json.mjs';\nfunction dupCursorRead(vibeDir) {\n  return _rj(path.join(vibeDir, 'state.json'));\n}\n",
  },
  {
    id: 'M16 alias + filename constant + template-literal path',
    primitive: 'cursor reader',
    code: "import { readFileSync as _rfs2 } from 'node:fs';\nconst CURSOR_FILE = 'state.json';\nfunction dupCursorRead(vibeDir) {\n  return JSON.parse(_rfs2(`${vibeDir}/${CURSOR_FILE}`, 'utf8'));\n}\n",
  },
  {
    id: 'M17 duplicate cursor WRITER',
    primitive: 'cursor writer',
    code: "import { writeFileSync as _wfs } from 'node:fs';\nfunction dupCursorWrite(vibeDir, body) {\n  _wfs(path.join(vibeDir, 'state.json'), `${JSON.stringify(body, null, 2)}\\n`);\n}\n",
  },
  {
    id: 'M5 raw path.join for the machine file',
    primitive: 'machine loader',
    code: "function dupMachinePath(vibeDir) {\n  return path.join(vibeDir, 'state-machine.json');\n}\n",
  },
  {
    id: 'M6 nested call inside the join arguments',
    primitive: 'machine loader',
    code: "function dupMachinePath(vibeDir) {\n  return path.join(path.resolve(vibeDir), 'state-machine.json');\n}\n",
  },
  {
    id: 'M7 template-literal machine path',
    primitive: 'machine loader',
    code: 'function dupMachinePath(vibeDir) {\n  return `${vibeDir}/state-machine.json`;\n}\n',
  },
  {
    id: 'M8 path.resolve instead of path.join',
    primitive: 'machine loader',
    code: "function dupMachinePath(vibeDir) {\n  return path.resolve(vibeDir, 'state-machine.json');\n}\n",
  },
  {
    id: 'M9 direct process.env member read',
    primitive: 'root resolver',
    code: 'function dupProjectDir() {\n  return process.env.CLAUDE_PROJECT_DIR;\n}\n',
  },
  {
    id: 'M10 destructured process.env',
    primitive: 'root resolver',
    code: 'function dupProjectDir() {\n  const { CLAUDE_PROJECT_DIR } = process.env;\n  return CLAUDE_PROJECT_DIR;\n}\n',
  },
  {
    id: 'M11 aliased process.env',
    primitive: 'root resolver',
    code: 'const _env = process.env;\nfunction dupProjectDir() {\n  return _env.CLAUDE_PROJECT_DIR;\n}\n',
  },
  {
    id: 'M18b duplicate root resolver (upward marker search)',
    primitive: 'root resolver',
    code: "function dupResolveRoot(start) {\n  let dir = start;\n  for (;;) {\n    if (fs.existsSync(path.join(dir, '.spec')) || fs.existsSync(path.join(dir, '.git'))) return dir;\n    const up = path.dirname(dir);\n    if (up === dir) return undefined;\n    dir = up;\n  }\n}\n",
  },
  {
    id: 'M19 re-derived `.agents/skills/vibe` layout join',
    primitive: 'root resolver',
    code: "function dupVibeDir(root) {\n  return path.join(root, '.agents', 'skills', 'vibe');\n}\n",
  },
  {
    id: 'M20 self-relative resolution from the module\'s own URL',
    primitive: 'root resolver',
    code: 'const _here = path.dirname(fileURLToPath(import.meta.url));\nfunction dupSelfRoot() {\n  return path.dirname(path.dirname(_here));\n}\n',
  },
  {
    id: 'M18a duplicate block extractor (marker grammar re-implementation)',
    primitive: 'block extractor',
    code: "function dupExtractBlock(text, id) {\n  const open = '<!-- ' + id + ' -->';\n  return text.split('\\n').findIndex((l) => l === open);\n}\n",
  },
];

// Where each mutant is planted. `commands/state.mjs` is an ordinary command
// module; `cli.mjs` and `commands/sub/dup.mjs` are the two scope holes round
// 1 demonstrated (M12/M13 and M14).
export const PLANT_SITES = ['commands/state.mjs', 'cli.mjs', 'commands/sub/dup.mjs'];

const NEW_FILE_PREAMBLE =
  "import fs from 'node:fs';\nimport path from 'node:path';\nimport { fileURLToPath } from 'node:url';\n\n";

// A scratch copy of flow/engine/ WITHOUT tests/ (the scan skips it anyway,
// and copying ~8k lines of fixtures per mutant would be pure waste).
export function makeScratchEngine() {
  const dir = mkdtempSync(path.join(tmpdir(), 'vibe-primscan-'));
  cpSync(ENGINE_DIR, dir, {
    recursive: true,
    filter: (src) => !src.split(path.sep).includes('tests'),
  });
  return dir;
}

export function plantMutant(dir, site, code) {
  const target = path.join(dir, site);
  if (site.includes('/sub/')) {
    mkdirSync(path.dirname(target), { recursive: true });
    writeFileSync(target, NEW_FILE_PREAMBLE + code);
    return;
  }
  writeFileSync(target, `${readFileSync(target, 'utf8')}\n${code}`);
}

for (const mutant of MUTANTS) {
  test(`discriminating: ${mutant.primitive} — ${mutant.id} — is caught in every plant site`, () => {
    for (const site of PLANT_SITES) {
      const dir = makeScratchEngine();
      try {
        assertEqual(scanEngineTree(dir), [], 'sanity: an untouched scratch copy must start clean');
        plantMutant(dir, site, mutant.code);
        const violations = scanEngineTree(dir);
        assert(
          violations.some((v) => v.startsWith(`${site}:`)),
          `EVADED: ${mutant.id} planted in ${site} was not caught.\nscan reported: ${JSON.stringify(violations)}`,
        );
      } finally {
        rmSync(dir, { recursive: true, force: true });
      }
    }
  });
}

test('discriminating: a second import.meta.url line in cli.mjs is caught (the waiver is one LINE, not a file exemption)', () => {
  const dir = makeScratchEngine();
  try {
    plantMutant(dir, 'cli.mjs', 'const _smuggled = fileURLToPath(import.meta.url);\n');
    const violations = scanEngineTree(dir);
    assert(
      violations.some((v) => v.startsWith('cli.mjs:') && v.includes('self-location')),
      `expected the extra import.meta.url line to be caught, got: ${JSON.stringify(violations)}`,
    );
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test('discriminating: an edit to a waived line stops being waived', () => {
  const dir = makeScratchEngine();
  try {
    const target = path.join(dir, 'cli.mjs');
    const patched = readFileSync(target, 'utf8').replace(
      'const __filename = fileURLToPath(import.meta.url);',
      'const __filename = fileURLToPath(import.meta.url); // reworded',
    );
    assert(patched.includes('// reworded'), 'sanity: the waived cli.mjs line must still exist to be edited');
    writeFileSync(target, patched);
    // Comment-stripping leaves trailing whitespace, so the trimmed line is
    // unchanged — the waiver still matches. Now change the CODE itself.
    assertEqual(scanEngineTree(dir), [], 'a trailing comment alone must not break the waiver');

    writeFileSync(
      target,
      readFileSync(target, 'utf8').replace(
        'const __filename = fileURLToPath(import.meta.url); // reworded',
        'const __filename = fileURLToPath(import.meta.url), _extra = 1;',
      ),
    );
    const violations = scanEngineTree(dir);
    assert(
      violations.some((v) => v.startsWith('cli.mjs:') && v.includes('self-location')),
      `changing a waived line must un-waive it, got: ${JSON.stringify(violations)}`,
    );
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});
