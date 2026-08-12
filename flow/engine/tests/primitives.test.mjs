// engine/tests/primitives.test.mjs — the duplicate-primitive scan (js-core/8,
// R1, R5).
//
// tech.md's contract: "Commands resolve their dirs once at dispatch and use
// these functions; none re-derives a primitive, parses cursor or machine
// JSON directly, or hardcodes a layout path." This file is what makes that
// mechanical instead of reviewed. scanEngineTree() below reads every module
// under flow/engine/ RECURSIVELY (hook.mjs included — it is NOT a bash-oracle
// port, but the SAME single-primitive contract applies to it; the top-level
// tests/ directory is the one exemption) and forbids each module from holding
// the INGREDIENTS of a primitive it does not own — see the INGREDIENTS block
// below for what that means and why it is an inversion of the call-shape
// matching this scan used to do.
//
// What is deliberately NOT banned (false-positive traps a cruder scan would
// fall into): plain mentions of the filenames in warn()/ok() MESSAGE TEXT
// (doctor.mjs's checkMachine, for instance), and regexes matched against
// someone ELSE's shell command (hook.mjs's Bash sniffer). Those are inert
// text, they construct nothing — but they are not exempt either: each one is
// an individually reasoned, occurrence-counted WAIVER in the list below, so
// none of them can quietly grow a second copy.
//
// Doc comments in these files narrate exactly these review lessons using
// the literal syntax of the banned ingredients (e.g. doctor.mjs's own comment
// quotes `joinMaybe(vibeDir, 'state-machine.json')` as the bug it fixed) —
// so comments are stripped before scanning, not just as tidiness but because
// skipping that step would make the scan permanently red on files that are
// already correct.
//
// Per the task brief: doctrine's CLAUDE_PROJECT_DIR cursor axis is NOT
// folded into any shared parity helper here — this file only asserts the
// scan's own shape, never re-implements doctrine's precedence rule.

import { readFileSync, readdirSync, existsSync, statSync, realpathSync, mkdirSync, cpSync, writeFileSync, rmSync, symlinkSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { test, assert, assertEqual, mkTempRoot } from './run.mjs';

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

// Exclusion is by exact RELATIVE PATH, not by directory name: matching the
// name at any depth made `commands/tests/` exempt too, so a duplicate cursor
// reader dropped in a directory anyone may create was never read at all.
const EXCLUDED_PATHS = new Set(['tests']);

// `.mjs` is not the only module extension the engine can load: package.json
// declares `"type": "module"`, so a bare `.js` file under flow/engine/ is an
// ordinary ESM module, and `.cjs` loads too. Enumerating only `.mjs` left
// "add the duplicate reader in a .js file" as a scope hole with no unusual
// spelling anywhere in it.
const SOURCE_EXTENSIONS = ['.mjs', '.js', '.cjs'];

// A SYMLINK is reported by readdirSync(withFileTypes) as neither isFile()
// nor isDirectory(), so testing those two predicates alone skips it — while
// Node imports it perfectly well. Resolve the link and classify the target
// instead; `seen` guards the cycle a symlinked directory can create. A broken
// link is skipped: there is nothing there to import OR to read.
export function listEngineSources(dir, prefix = '', seen = new Set()) {
  const out = [];
  const real = realpathSync(dir);
  if (seen.has(real)) return out;
  seen.add(real);

  for (const entry of readdirSync(dir, { withFileTypes: true }).sort((a, b) => (a.name < b.name ? -1 : 1))) {
    const rel = prefix ? `${prefix}/${entry.name}` : entry.name;
    const abs = path.join(dir, entry.name);

    let isDir = entry.isDirectory();
    let isFile = entry.isFile();
    if (entry.isSymbolicLink()) {
      let stat;
      try {
        stat = statSync(abs);
      } catch {
        continue;
      }
      isDir = stat.isDirectory();
      isFile = stat.isFile();
    }

    if (isDir) {
      if (EXCLUDED_PATHS.has(rel)) continue;
      out.push(...listEngineSources(abs, rel, seen));
    } else if (isFile && SOURCE_EXTENSIONS.some((ext) => entry.name.endsWith(ext))) {
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
//   * a module that can name neither 'state.json' NOR cursorPath() cannot
//     obtain the cursor's path at all — it must go through readCursor()/
//     writeCursor()
//   * likewise 'state-machine.json' / machinePath() for the machine loader
//   * a module that never names CLAUDE_PROJECT_DIR, `.agents`, `skills/vibe`,
//     `.spec`/`.git`, `process.cwd`, `import.meta`, `argv[1]`, an ascent off
//     its own `__dirname`, or resolveProjectCursorDir() has no leg of
//     resolveRoot()/resolveVibeDir()/resolveProjectCursorDir() left to
//     re-derive
//   * a module that never writes the `<!--` marker grammar cannot
//     re-implement extractBlock()
//
// That covers all five primitives R1 names (root resolver, cursor reader,
// cursor writer, machine loader, block extractor).
//
// The helper clauses are load-bearing, and were the round-1 fix's own defect
// (re-review Finding 1). Banning only the LITERAL is sound exactly while the
// literal is the only way to reach the primitive's path — and round 1 broke
// that premise itself, by exporting cursorPath()/machinePath() so a caller
// needing only the path would not have to spell the filename.
// `readJson(cursorPath(vibeDir))` was then a complete duplicate cursor reader
// naming nothing banned. The generalized invariant, which is what the
// ingredient list now encodes: a module may not obtain a primitive's location
// by ANY means unless it is an allowlisted consumer, pinned line by line.
//
// KNOWN LIMITS (stated, not papered over):
//
//   1. A whole-file literal ban cannot see through ANY source-level split or
//      re-spelling of a literal. Not just concatenation (`'state' + '.json'`)
//      — also a line continuation inside the string (`'state\` + newline +
//      `.json'`), a computed name (`['state','json'].join('.')`), a unicode
//      escape (`'state.json'`), or a value built at runtime
//      (`Buffer.from(...)`, `new Function(...)`). The boundary is "the
//      ingredient appears verbatim in the source text", not "no
//      concatenation". Closing it needs an AST plus constant folding, which
//      needs a dependency this engine will not take (R5).
//   2. Laundering through an ALLOWLISTED consumer's own name is closed for
//      re-exports (unwaivable, see `noReexport`) and for any second use of
//      the helper (the allowlist is occurrence-counted), but a consumer that
//      returned the path from an existing, differently-named export would
//      still pass. That takes a deliberate edit to an allowlisted file, which
//      is the narrowest surface this can be reduced to without dataflow.
//   3. Reaching a primitive's file by enumeration rather than by name
//      (readdirSync + a filter) is not banned, because doctor.mjs enumerates
//      directories legitimately. It re-derives nothing on its own; it would
//      still have to identify which file it found.
//   4. ASCENDING from a value a sanctioned resolver returned is textually
//      indistinguishable from any other `path.dirname(someVariable)` — and
//      doctor.mjs's rootForReport() is exactly that shape, reviewed and
//      sanctioned (round-1 Finding 3). The single-expression spelling
//      (`path.dirname(path.dirname(x))`) IS caught by `self-relative-ascent`;
//      the same walk split across two statements with an intermediate
//      variable is not. Telling the two apart needs dataflow, not a wider
//      regex — a blunter rule would red-flag json.mjs's ordinary
//      `path.dirname(filePath)`.
//
// This is the strongest sound subset, not a complete one.
//
// OWNERS vs WAIVERS. An ingredient's `owners` are the modules that define the
// primitive — unrestricted there, by design (machinePath() must build the
// machine path inside machine.mjs; resolveProjectCursorDir() must know the
// layout inside root.mjs). Everything else needs a WAIVER: an exact,
// individually-reasoned source line WITH an occurrence count. Editing a waived
// line un-waives it, and so does adding a second copy of it — byte-identical
// or not (re-review Finding 2: content-only matching made copy-paste free,
// which is the likeliest way a second re-derivation actually lands). A waiver
// whose observed count drifts from its declared count in either direction
// fails, so the list can neither rot into a blanket exemption nor quietly
// under-report.
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
    // `.agents` alone misses the PLUGIN leg of the layout, which is one level
    // shallower and never names `.agents` at all: pluginVibeDir()'s
    // `<PLUGIN_ROOT>/skills/vibe`. Both the joined-literal and the
    // slash-separated spelling of that partial path count (re-review
    // Finding 4, E8).
    re: /\\?\.agents|['"`]skills['"`]\s*,\s*['"`]vibe['"`]|skills\/vibe/,
    owners: ['root.mjs'],
    why: 'hardcodes the `.agents/skills/vibe` (or plugin `skills/vibe`) layout — resolveVibeDir()/resolveProjectCursorDir() (root.mjs) single-source it',
  },
  {
    id: 'root-markers',
    re: /(^|[^A-Za-z0-9_$.])\\?\.(spec|git)\b/,
    owners: ['root.mjs'],
    why: "re-derives the upward `.spec`/`.git` marker search — resolveRoot() (root.mjs) owns it",
  },
  {
    id: 'cwd-fallback',
    // Every route to process.cwd that needs no string surgery: member access,
    // computed access, a destructured `cwd`, and an aliased `process` object
    // (the last two are the shapes M10/M11 already cover for process.env —
    // re-review Finding 4 is that the sibling ingredients never got them).
    re: /process\s*(?:\.\s*cwd\b|\[\s*['"`]\s*cwd)|\{[^}]*\bcwd\b[^}]*\}\s*=\s*process\b|=\s*process\s*[;,)]/,
    owners: ['root.mjs'],
    why: "re-derives resolveRoot()'s cwd fallback leg (however spelled — member, computed, destructured, or through an aliased `process`) — root.mjs owns it",
  },
  {
    id: 'self-location',
    // `import.meta`, not `import.meta.url`: destructuring (`const { url } =
    // import.meta`) and aliasing (`const m = import.meta`) reach the same
    // ingredient without the `.url` suffix (re-review Finding 4, E5/E6).
    // `argv[0]`/`argv[1]` is the OTHER self-location ingredient — the entry
    // script's own path — including the array-destructuring spelling that
    // never writes a bracket index. `process.argv.slice(2)` (cli.mjs's real,
    // unrelated use: reading the user's arguments) matches neither.
    re: /import\s*\.\s*meta\b|\bargv\s*(?:\[\s*[01]\s*\]|\.\s*at\s*\(\s*[01]\s*\))|\[[^\]]*\]\s*=\s*process\s*\.\s*argv\b/,
    owners: ['root.mjs'],
    why: "self-locates from the module's own URL or the entry script's path — that is resolveRoot()/resolveVibeDir()'s ingredient (root.mjs)",
  },
  {
    id: 'self-relative-ascent',
    // Walking UP from an already-self-located directory. cli.mjs holds a
    // legitimate `__dirname` for its own commands/ dispatch, so a smuggled
    // root resolver there needs no NEW ingredient at all — it just ascends
    // from the one the file already has. A pure dot-dot path literal is the
    // same move spelled through path.resolve/path.join — `'..'`, `'../'`,
    // `'../..'`, `'/..'` all count, while an ordinary relative import
    // specifier (`'../root.mjs'`) deliberately does not.
    re: /path\s*\.\s*dirname\s*\(\s*(?:__dirname|__filename|path\s*\.\s*dirname)\b|['"`][/\\]?(?:\.\.[/\\])*\.\.[/\\]?['"`]/,
    owners: ['root.mjs'],
    why: 'ascends from a self-located directory toward a project root — that ascent is resolveRoot()/resolveVibeDir()/selfRelativeRoot() (root.mjs)',
  },
  {
    id: 'marker-grammar',
    re: /<!--|-->/,
    owners: ['blocks.mjs'],
    why: 'writes the marker-block grammar — extractBlock() (blocks.mjs) is the one grammar',
  },
  {
    id: 'tests-import',
    // tests/ is exempt because fixtures legitimately name every ingredient.
    // That exemption is only sound while nothing SHIPPED imports out of it:
    // otherwise "put the duplicate reader in tests/helpers.mjs and import it"
    // is a one-line evasion of the entire scan. No engine module has ever
    // imported from tests/, so this owns nothing and waives nothing.
    re: /\bfrom\s*['"][^'"]*\btests\//,
    owners: [],
    why: 'imports from the exempt tests/ directory — nothing shipped may depend on fixture code, because fixture code is not scanned',
  },

  // --- PRIMITIVE PATH HELPERS (re-review Finding 1).
  //
  // Banning the filename LITERAL is only sound while naming the literal is
  // the only way to obtain the primitive's path. Fix round 1 broke that
  // premise: it exported cursorPath()/machinePath() so a caller needing just
  // the path would not have to spell 'state.json' — which turned the helper
  // into a second legal spelling of the ingredient it was meant to protect.
  // `readJson(cursorPath(vibeDir))` is a complete duplicate cursor reader
  // that names nothing banned.
  //
  // So the helper NAMES are ingredients too, and the rule generalizes: a
  // module may not obtain a primitive's location by ANY means unless it is
  // an allowlisted consumer, pinned line-by-line like every other waiver.
  // Banning the identifier (rather than the import statement) is what makes
  // this hold for a namespace import (`mod.cursorPath(v)`), a computed member
  // (`mod['cursorPath']`), an aliased import, and a re-export — all of which
  // still have to write the name somewhere.
  //
  // resolveVibeDir()/resolveRoot()/resolveSkillsDir() are deliberately NOT in
  // this class. They hand out a DIRECTORY, which every command legitimately
  // needs and which is useless on its own: turning one into a cursor read
  // still requires either the banned literal or one of these helpers, both of
  // which are covered here. Gating them would be a large allowlist that closes
  // nothing.
  {
    id: 'cursor-path-helper',
    re: /\bcursorPath\b/,
    owners: ['cursor.mjs'],
    noReexport: true,
    why: "obtains the cursor file's PATH without naming it — read/write the cursor through readCursor()/writeCursor() (cursor.mjs); cursorPath() has a counted, per-line consumer allowlist",
  },
  {
    id: 'machine-path-helper',
    re: /\bmachinePath\b/,
    owners: ['machine.mjs'],
    noReexport: true,
    why: "obtains the state-machine file's PATH without naming it — read it through loadMachine() (machine.mjs); machinePath() has a counted, per-line consumer allowlist",
  },
  {
    id: 'project-cursor-dir-helper',
    re: /\bresolveProjectCursorDir\b/,
    owners: ['root.mjs'],
    noReexport: true,
    why: "consumes doctrine's CLAUDE_PROJECT_DIR-gated cursor-dir rule — one consumer only (doctrine.mjs), pinned per line, so the precedence rule cannot sprout a second caller silently",
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
    "mirrors the ORIGINAL .sh hook's own $ROOT-relative literal for the warnings log / evidence receipts (hook.mjs header; js-core/7 review). This IS a real layout literal — the waiver allows exactly ONE occurrence (see `count`), so a SECOND copy, byte-identical or not, is a violation.",
  'layout-name-probe':
    'a path.basename() equality check on a directory NAME, walking structure off an already-resolved primitive — no path is constructed and no new resolution algorithm exists (doctor.mjs rootForReport(), round-1 Finding 3).',
  'marker-presence-probe':
    "an opener-only substring probe, deliberately NOT extractBlock(): the oracle's `grep -q '<!-- vibe:doctrine -->'` succeeds on an opener with no closer, where extractBlock() correctly returns undefined. Routing it through blocks.mjs would change behaviour and break parity.",
  'cli-self-dispatch':
    'cli.mjs locates its own commands/ directory for module dispatch — one level DOWN from its own file, never up toward a repo root. Waived per LINE and per OCCURRENCE COUNT rather than as an owner entry: a second self-location line here, a byte-identical copy of this one, or any ascent off the __dirname it already holds, all trip the scan (self-location / self-relative-ascent).',
  'primitive-path-consumer':
    "the single allowlisted consumer of a primitive's PATH helper: it needs the path only for an existence/type probe (doctor.mjs) or for one documented precedence rule (doctrine.mjs), never to read or write the file — that stays with readCursor()/writeCursor()/loadMachine(). Pinned to the exact import line and the exact use line, one occurrence each, so a SECOND use in the same file (`readJson(cursorPath(v))`) is a violation and a re-export under a new name is unwaivable.",
  'spec-sanctioned-exemption':
    "tech.md's Contract defines resolveProjectCursorDir() as doctrine's CLAUDE_PROJECT_DIR-first cursor rule 'gated on that state.json existing' — knowing the filename is its documented job. NOT routed through cursorPath() because root.mjs is the self-location primitive and must stay importable alone (root.test.mjs copies this single file into synthetic install layouts); importing cursor.mjs would drag json.mjs in behind it.",
};

// Exact-line, occurrence-COUNTED waivers. `line` is matched against the
// TRIMMED source line after comment-stripping and `count` (default 1) is how
// many times that exact line may appear in that file. Both halves matter:
// matching on content alone made a byte-identical copy-paste of a waived line
// inside the same file silently free (re-review Finding 2), which is the
// likeliest way a second layout re-derivation actually lands. Editing a waived
// line, adding a differently-spelled one, and duplicating one are now all
// violations; a waiver whose observed count drifts from `count` in EITHER
// direction fails too (see the count test), so the list cannot rot into a
// blanket exemption or quietly under-report.
const WAIVERS = [
  { file: 'cli.mjs', id: 'self-location', reason: 'cli-self-dispatch', line: 'const __filename = fileURLToPath(import.meta.url);' },
  { file: 'cli.mjs', id: 'self-relative-ascent', reason: 'cli-self-dispatch', line: 'const __dirname = path.dirname(__filename);' },

  { file: 'root.mjs', id: 'cursor-file', reason: 'spec-sanctioned-exemption', line: "return fs.existsSync(path.join(candidate, 'state.json')) ? candidate : undefined;" },

  { file: 'commands/doctor.mjs', id: 'machine-file', reason: 'oracle-text', line: 'return warn(\'machine\', `state-machine.json missing at ${p} — flow harness incomplete`);' },
  { file: 'commands/doctor.mjs', id: 'machine-file', reason: 'oracle-text', line: "return warn('machine', 'state-machine.json is present but not valid JSON');" },
  { file: 'commands/doctor.mjs', id: 'machine-file', reason: 'oracle-text', line: "return ok('machine', 'state-machine.json present');" },
  { file: 'commands/doctor.mjs', id: 'marker-grammar', reason: 'marker-presence-probe', line: "doctrineBlock = fs.readFileSync(skillMdPath, 'utf8').includes('<!-- vibe:doctrine -->');" },
  { file: 'commands/doctor.mjs', id: 'marker-grammar', reason: 'oracle-text', line: "'no doctrine coverage — no <!-- vibe:doctrine --> block, no wired SessionStart hook, no per-user plugin; run install.sh (--local or --global) / setup.apply'," },
  { file: 'commands/doctor.mjs', id: 'vibe-layout', reason: 'layout-name-probe', line: "if (path.basename(agentsDir) === '.agents') {" },
  { file: 'commands/doctor.mjs', id: 'cursor-path-helper', reason: 'primitive-path-consumer', line: "import { readCursor, cursorPath } from '../cursor.mjs';" },
  { file: 'commands/doctor.mjs', id: 'cursor-path-helper', reason: 'primitive-path-consumer', line: "const statePath = typeof vibeDir === 'string' ? cursorPath(vibeDir) : undefined;" },
  { file: 'commands/doctor.mjs', id: 'machine-path-helper', reason: 'primitive-path-consumer', line: "import { loadMachine, machinePath } from '../machine.mjs';" },
  { file: 'commands/doctor.mjs', id: 'machine-path-helper', reason: 'primitive-path-consumer', line: "const p = typeof vibeDir === 'string' ? machinePath(vibeDir) : undefined;" },

  { file: 'commands/doctrine.mjs', id: 'project-cursor-dir-helper', reason: 'primitive-path-consumer', line: "import { resolveVibeDir, resolveSkillsDir, resolveProjectCursorDir } from '../root.mjs';" },
  { file: 'commands/doctrine.mjs', id: 'project-cursor-dir-helper', reason: 'primitive-path-consumer', line: 'return resolveProjectCursorDir() ?? vibeDir;' },

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

// How many occurrences of the waived line this waiver allows. Absent means
// exactly one — the only count any waiver has needed so far, and the safe
// default: a new copy of a waived line has to be declared, not inherited.
function waiverCount(w) {
  return w.count ?? 1;
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

    // Per-file waiver budget, spent as occurrences are met. Once a waiver's
    // allowance is exhausted, every further copy of that line is a violation.
    const budget = new Map();
    for (const w of WAIVERS) {
      if (w.file === rel) budget.set(waiverKey(w), waiverCount(w));
    }

    for (const ing of INGREDIENTS) {
      if (ing.owners.includes(rel)) continue;
      for (let i = 0; i < lines.length; i += 1) {
        if (!ing.re.test(lines[i])) continue;
        const trimmed = lines[i].trim();

        // Re-exporting a primitive-path helper out of an allowlisted consumer
        // would hand every other module a fresh, unbanned name for it — the
        // round-1 hole one indirection further out. Unwaivable by design.
        if (ing.noReexport && /\bexport\b/.test(lines[i])) {
          violations.push(
            `${rel}:${i + 1}: [${ing.id}] re-exports a primitive path helper — only its owner may export it\n    | ${trimmed}`,
          );
          continue;
        }

        const key = `${rel}|${ing.id}|${trimmed}`;
        const left = budget.get(key) ?? 0;
        if (left > 0) {
          budget.set(key, left - 1);
          continue;
        }
        violations.push(`${rel}:${i + 1}: [${ing.id}] ${ing.why}\n    | ${trimmed}`);
      }
    }
  }
  return violations.sort();
}

// How many times each waiver's line actually occurs in `dir`, keyed by
// waiverKey(). Zero means a stale waiver; more than the declared `count`
// means a new re-derivation was pasted in beside a blessed one. Both are
// failures — see the two tests below.
export function waiverOccurrences(dir) {
  const counts = new Map();
  for (const rel of listEngineSources(dir)) {
    const src = stripComments(readFileSync(path.join(dir, rel), 'utf8'));
    for (const line of src.split('\n')) {
      const trimmed = line.trim();
      for (const w of WAIVERS) {
        if (w.file !== rel || w.line !== trimmed) continue;
        const ing = INGREDIENTS.find((x) => x.id === w.id);
        if (ing && ing.re.test(line)) counts.set(waiverKey(w), (counts.get(waiverKey(w)) ?? 0) + 1);
      }
    }
  }
  return counts;
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
  const used = waiverOccurrences(ENGINE_DIR);
  const stale = WAIVERS.filter((w) => !used.get(waiverKey(w))).map(waiverKey);
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

  // --- fix-round-1 re-review, Finding 1: the HELPER-REUSE family. Round 1
  // exported cursorPath()/machinePath() so a caller needing only the PATH
  // would not have to name the file — which made the helper itself a legal
  // spelling of the banned ingredient. Every one of these named NO banned
  // token before the helper ingredients were added.
  {
    id: 'E1 helper reuse — readJson(cursorPath(vibeDir)) (re-review Finding 1, planted live in orders.mjs)',
    primitive: 'cursor reader',
    code: "import { cursorPath as _cp } from '../cursor.mjs';\nimport { readJson as _rj } from '../json.mjs';\nexport function dupReadCursor(vibeDir) {\n  const raw = _rj(_cp(vibeDir));\n  return { flow: raw.flow, phase: raw.phase, feature: raw.feature ?? null };\n}\n",
  },
  {
    id: 'E2 helper reuse — JSON.parse(readFileSync(cursorPath(v)))',
    primitive: 'cursor reader',
    code: "import { cursorPath as _cp2 } from '../cursor.mjs';\nimport { readFileSync as _rfs3 } from 'node:fs';\nfunction dupReadCursor2(vibeDir) {\n  return JSON.parse(_rfs3(_cp2(vibeDir), 'utf8'));\n}\n",
  },
  {
    id: 'E3 helper reuse — duplicate cursor WRITER via writeFileSync(cursorPath(v))',
    primitive: 'cursor writer',
    code: "import { cursorPath as _cp3 } from '../cursor.mjs';\nimport { writeFileSync as _wfs2 } from 'node:fs';\nfunction dupCursorWrite2(vibeDir, body) {\n  _wfs2(_cp3(vibeDir), `${JSON.stringify(body, null, 2)}\\n`);\n}\n",
  },
  {
    id: 'E4 helper reuse — duplicate machine loader via readJson(machinePath(v))',
    primitive: 'machine loader',
    code: "import { machinePath as _mp } from '../machine.mjs';\nimport { readJson as _rj2 } from '../json.mjs';\nfunction dupLoadMachine(vibeDir) {\n  return _rj2(_mp(vibeDir));\n}\n",
  },
  {
    id: 'E14 helper reuse — path in a variable, then hand-rolled field validation',
    primitive: 'cursor reader',
    code: "import { cursorPath as _cp4 } from '../cursor.mjs';\nimport { readJson as _rj3 } from '../json.mjs';\nfunction dupReadCursor3(vibeDir) {\n  const statePath = _cp4(vibeDir);\n  const raw = _rj3(statePath);\n  return typeof raw.flow === 'string' ? raw : { flow: 'idle', phase: 'idle' };\n}\n",
  },
  {
    id: 'E15 helper reuse via a NAMESPACE import (no named binding to grep for)',
    primitive: 'cursor reader',
    code: "import * as _cursorMod from '../cursor.mjs';\nimport { readJson as _rj4 } from '../json.mjs';\nfunction dupReadCursor4(vibeDir) {\n  return _rj4(_cursorMod.cursorPath(vibeDir));\n}\n",
  },
  {
    id: "E16 helper reuse via computed member access (_mod['cursorPath'])",
    primitive: 'cursor reader',
    code: "import * as _cursorMod2 from '../cursor.mjs';\nimport { readJson as _rj5 } from '../json.mjs';\nfunction dupReadCursor5(vibeDir) {\n  return _rj5(_cursorMod2['cursorPath'](vibeDir));\n}\n",
  },
  {
    id: 'E17 second consumer of resolveProjectCursorDir (doctrine precedence re-derived)',
    primitive: 'root resolver',
    code: "import { resolveProjectCursorDir as _rpcd } from '../root.mjs';\nfunction dupDoctrineCursorDir(vibeDir) {\n  return _rpcd() ?? vibeDir;\n}\n",
  },

  // --- Finding 4: the process.env destructure/alias trick, applied to the
  // ingredients whose siblings never got the same treatment.
  {
    id: 'E5 destructured import.meta (const { url: _u } = import.meta)',
    primitive: 'root resolver',
    code: "const { url: _u } = import.meta;\nconst _selfDir = path.dirname(fileURLToPath(_u));\nfunction dupSelfRoot2() {\n  return path.dirname(path.dirname(_selfDir));\n}\n",
  },
  {
    id: 'E6 aliased import.meta (const _meta = import.meta)',
    primitive: 'root resolver',
    code: "const _meta = import.meta;\nconst _selfDir2 = path.dirname(fileURLToPath(_meta.url));\nfunction dupSelfRoot3() {\n  return path.dirname(path.dirname(_selfDir2));\n}\n",
  },
  {
    id: "E7 bracket-notation process['cwd']()",
    primitive: 'root resolver',
    code: "function dupCwdRoot() {\n  return process['cwd']();\n}\n",
  },
  {
    id: 'E7b destructured cwd off process',
    primitive: 'root resolver',
    code: 'const { cwd: _cwd } = process;\nfunction dupCwdRoot2() {\n  return _cwd();\n}\n',
  },
  {
    id: 'E7c aliased process object, then .cwd()',
    primitive: 'root resolver',
    code: 'const _proc = process;\nfunction dupCwdRoot3() {\n  return _proc.cwd();\n}\n',
  },
  {
    id: "E8 partial-path join — path.join(root, 'skills', 'vibe') (pluginVibeDir's leg, never names .agents)",
    primitive: 'root resolver',
    code: "function dupPluginVibeDir(root) {\n  return path.join(root, 'skills', 'vibe');\n}\n",
  },
  {
    id: 'E8b partial-path join as a template literal (`${root}/skills/vibe`)',
    primitive: 'root resolver',
    code: 'function dupPluginVibeDir2(root) {\n  return `${root}/skills/vibe`;\n}\n',
  },
  {
    id: 'E18 self-location off the entry script (process.argv[1])',
    primitive: 'root resolver',
    code: 'function dupEntryRoot() {\n  return path.dirname(path.dirname(process.argv[1]));\n}\n',
  },
  {
    id: 'E19 self-location off a DESTRUCTURED process.argv (no brackets to grep)',
    primitive: 'root resolver',
    code: 'const [, _script] = process.argv;\nfunction dupEntryRoot2() {\n  return path.dirname(_script);\n}\n',
  },
  {
    id: 'E21 the re-derivation moved INTO the exempt tests/ directory and imported back out',
    primitive: 'cursor reader',
    code: "import { dupCursorRead as _dcr } from '../tests/helpers.mjs';\nexport const readIt = (vibeDir) => _dcr(vibeDir);\n",
  },
];

// cli.mjs-ONLY mutants. cli.mjs legitimately holds `__filename`/`__dirname`
// for its own commands/ dispatch, so a smuggled root resolver there needs no
// new import.meta line at all — it just walks UP from the dirname the file
// already has. These cannot be planted in the other two sites, where no
// self-location exists to reuse without naming a banned ingredient.
export const CLI_ASCENT_MUTANTS = [
  {
    id: 'E9 smuggled root resolver reusing cli.mjs\'s own __dirname (no new ingredient named)',
    code: 'function smuggledRoot() {\n  return path.dirname(path.dirname(__dirname));\n}\n',
  },
  {
    id: 'E10 smuggled root resolver via path.resolve(__dirname, ..., ...)',
    code: "const _smuggledRoot = path.resolve(__dirname, '..', '..');\n",
  },
  {
    id: "E13 smuggled root resolver via a MULTI-segment dot-dot literal ('../..')",
    code: "const _smuggledRoot2 = path.join(__dirname, '../..');\n",
  },
];

// The scan's own SCOPE, attacked. A violation the scan never reads is a
// violation the scan cannot catch, and both of these are reachable by adding
// one ordinary-looking file — no unusual spelling anywhere in it.
const SCOPE_MUTANTS = [
  {
    id: 'E11 a duplicate cursor reader in a NESTED tests/ directory',
    // EXCLUDED_DIRS matched by name at any depth, so `commands/tests/` was
    // exempt too — a directory anyone may create, exempting anything in it.
    site: 'commands/tests/dup.mjs',
  },
  {
    id: 'E12 a duplicate cursor reader in a .js module',
    // package.json declares "type": "module", so a bare .js file under
    // flow/engine/ is an ordinary ESM module the engine can import — but the
    // scan only enumerated *.mjs.
    site: 'commands/dup.js',
  },
];

for (const mutant of SCOPE_MUTANTS) {
  test(`discriminating: cursor reader — ${mutant.id} (${mutant.site}) — is in scope`, () => {
    const dir = makeScratchEngine();
    try {
      assertEqual(scanEngineTree(dir), [], 'sanity: an untouched scratch copy must start clean');
      plantMutant(dir, mutant.site, MUTANTS[0].code);
      const violations = scanEngineTree(dir);
      assert(
        violations.some((v) => v.startsWith(`${mutant.site}:`)),
        `EVADED: ${mutant.id} was outside the scan's scope.\nscan reported: ${JSON.stringify(violations)}`,
      );
    } finally {
      rmSync(dir, { recursive: true, force: true });
    }
  });
}

test('discriminating: cursor reader — E20 a SYMLINKED module under flow/engine/ — is in scope', () => {
  // readdirSync(withFileTypes) reports a symlink as neither isFile() nor
  // isDirectory(), so an enumerator that tests those two predicates skips it
  // silently — while Node imports it perfectly well. This repo already ships
  // `.agents/skills/vibe -> flow`, so symlinks into the engine tree are not a
  // hypothetical shape here.
  const dir = makeScratchEngine();
  const outside = mkTempRoot('vibe-primscan-outside-');
  try {
    assertEqual(scanEngineTree(dir), [], 'sanity: an untouched scratch copy must start clean');
    const realTarget = path.join(outside, 'dup.mjs');
    writeFileSync(realTarget, NEW_FILE_PREAMBLE + MUTANTS[0].code);
    symlinkSync(realTarget, path.join(dir, 'commands', 'linked.mjs'));
    const violations = scanEngineTree(dir);
    assert(
      violations.some((v) => v.startsWith('commands/linked.mjs:')),
      `EVADED: a symlinked engine module was never read.\nscan reported: ${JSON.stringify(violations)}`,
    );
  } finally {
    rmSync(dir, { recursive: true, force: true });
    rmSync(outside, { recursive: true, force: true });
  }
});

test('primitive scan: every file under flow/engine/ (tests/ aside) has an extension the scan enumerates', () => {
  // The scan can only ban ingredients in files it reads. Rather than trust
  // that the extension list stays complete, fail the moment a source file
  // with an unenumerated extension appears — adding a `.ts`/`.mts` engine
  // module then has to update SOURCE_EXTENSIONS instead of silently landing
  // outside the scan.
  const unscanned = [];
  const walk = (abs, prefix = '') => {
    for (const entry of readdirSync(abs, { withFileTypes: true })) {
      const rel = prefix ? `${prefix}/${entry.name}` : entry.name;
      if (EXCLUDED_PATHS.has(rel)) continue;
      if (entry.isDirectory()) {
        walk(path.join(abs, entry.name), rel);
      } else if (!SOURCE_EXTENSIONS.some((ext) => entry.name.endsWith(ext))) {
        unscanned.push(rel);
      }
    }
  };
  walk(ENGINE_DIR);
  assertEqual(
    unscanned,
    [],
    `files under flow/engine/ the primitive scan never reads:\n${unscanned.join('\n')}`,
  );
});

test("primitive scan: the top-level tests/ exemption is by PATH, not by directory NAME anywhere", () => {
  const files = listEngineSources(ENGINE_DIR);
  assert(
    !files.some((f) => f === 'tests' || f.startsWith('tests/')),
    `the real tests/ directory must still be excluded, got: ${files.join(', ')}`,
  );
});

// Where each mutant is planted. `commands/state.mjs` is an ordinary command
// module; `cli.mjs` and `commands/sub/dup.mjs` are the two scope holes round
// 1 demonstrated (M12/M13 and M14).
export const PLANT_SITES = ['commands/state.mjs', 'cli.mjs', 'commands/sub/dup.mjs'];

const NEW_FILE_PREAMBLE =
  "import fs from 'node:fs';\nimport path from 'node:path';\nimport { fileURLToPath } from 'node:url';\n\n";

// A scratch copy of flow/engine/ WITHOUT tests/ (the scan skips it anyway,
// and copying ~8k lines of fixtures per mutant would be pure waste).
export function makeScratchEngine() {
  const dir = mkTempRoot('vibe-primscan-');
  // Skip THE tests directory, by path — not any path segment named "tests".
  // A segment test also drops a hypothetical commands/tests/ (which the scan
  // now reads, so the copy must contain it), and would empty the whole copy
  // if the checkout itself ever sat under a directory called "tests".
  const excluded = path.join(ENGINE_DIR, 'tests');
  cpSync(ENGINE_DIR, dir, {
    recursive: true,
    filter: (src) => src !== excluded && !src.startsWith(excluded + path.sep),
  });
  return dir;
}

export function plantMutant(dir, site, code) {
  const target = path.join(dir, site);
  if (!existsSync(target)) {
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

for (const mutant of CLI_ASCENT_MUTANTS) {
  test(`discriminating: root resolver — ${mutant.id} — is caught in cli.mjs`, () => {
    const dir = makeScratchEngine();
    try {
      assertEqual(scanEngineTree(dir), [], 'sanity: an untouched scratch copy must start clean');
      plantMutant(dir, 'cli.mjs', mutant.code);
      const violations = scanEngineTree(dir);
      assert(
        violations.some((v) => v.startsWith('cli.mjs:')),
        `EVADED: ${mutant.id} planted in cli.mjs was not caught.\nscan reported: ${JSON.stringify(violations)}`,
      );
    } finally {
      rmSync(dir, { recursive: true, force: true });
    }
  });
}

// Fix-round-1 re-review, Finding 2: a waiver is an allowance for a COUNTED
// set of occurrences, not a content pattern. Copy-pasting a waived line
// inside its own file was the likeliest way a second layout re-derivation
// lands, and it used to be free. This is the general form of the reviewer's
// W1/W3/W4/W5 and T13 — every waiver in the list, not a hand-picked one.
for (const w of WAIVERS) {
  test(`discriminating: a byte-identical duplicate of the waived ${w.file} [${w.id}] line is caught`, () => {
    const dir = makeScratchEngine();
    try {
      assertEqual(scanEngineTree(dir), [], 'sanity: an untouched scratch copy must start clean');
      const target = path.join(dir, w.file);
      writeFileSync(target, `${readFileSync(target, 'utf8')}\n${w.line}\n`);
      const violations = scanEngineTree(dir);
      assert(
        violations.some((v) => v.startsWith(`${w.file}:`) && v.includes(`[${w.id}]`)),
        `EVADED: a second byte-identical copy of a waived line was silently waived.\nline: ${w.line}\nscan reported: ${JSON.stringify(violations)}`,
      );
    } finally {
      rmSync(dir, { recursive: true, force: true });
    }
  });
}

test('primitive scan: every waiver occurrence count matches the declared count exactly', () => {
  const observed = waiverOccurrences(ENGINE_DIR);
  const mismatches = WAIVERS.filter((w) => (observed.get(waiverKey(w)) ?? 0) !== waiverCount(w)).map(
    (w) => `${waiverKey(w)}: declared ${waiverCount(w)}, observed ${observed.get(waiverKey(w)) ?? 0}`,
  );
  assertEqual(
    mismatches,
    [],
    `a waiver's declared occurrence count no longer matches the tree — a second copy of a waived line is a NEW re-derivation, and a vanished one is a stale waiver:\n${mismatches.join('\n')}`,
  );
});

// A primitive-path helper must not be laundered out of an allowlisted
// consumer under a fresh name — that would hand every other module a legal
// spelling again, one indirection further out.
test('discriminating: an allowlisted helper consumer may not re-export the helper under a new name', () => {
  const dir = makeScratchEngine();
  try {
    assertEqual(scanEngineTree(dir), [], 'sanity: an untouched scratch copy must start clean');
    plantMutant(dir, 'commands/doctor.mjs', 'export const cp = cursorPath;\n');
    const violations = scanEngineTree(dir);
    assert(
      violations.some((v) => v.startsWith('commands/doctor.mjs:') && v.includes('cursor-path-helper')),
      `EVADED: an allowlisted consumer re-exported the helper.\nscan reported: ${JSON.stringify(violations)}`,
    );
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

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
