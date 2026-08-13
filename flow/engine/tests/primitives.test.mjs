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
import { test, assert, assertEqual, assertIncludes, mkTempRoot } from './run.mjs';

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
// file — that is what lets a violation report its real line number. Fix round 3
// is what finally CONSUMES that property: the scan matches whole files and
// derives the line from the match offset (see scanEngineTree).
//
// The same pass also records every STRING and TEMPLATE literal with its offset,
// because the module-reach rule (Critical 1) has to resolve module specifiers
// and must not confuse a specifier with a regex literal or a comment. One
// tokenizer, two consumers — a second hand-rolled literal finder is exactly the
// kind of near-duplicate that drifts.
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

// Cooks the source-level escape sequences inside a literal's body. Without
// this, `'./x.mjs'` and a backslash-newline line continuation are two
// more spellings of a module specifier that the reach rule would never resolve.
// This is not constant folding — it decodes ONE literal, no concatenation, no
// dataflow — but it is free, and it moves the documented boundary from "the
// literal is written plainly" to "the specifier is one literal".
function cookEscapes(raw) {
  let out = '';
  for (let i = 0; i < raw.length; i += 1) {
    const c = raw[i];
    if (c !== '\\') {
      out += c;
      continue;
    }
    const n = raw[i + 1];
    i += 1;
    if (n === undefined) break;
    if (n === '\n') continue; // line continuation — contributes nothing
    if (n === 'n') { out += '\n'; continue; }
    if (n === 't') { out += '\t'; continue; }
    if (n === 'r') { out += '\r'; continue; }
    if (n === 'b') { out += '\b'; continue; }
    if (n === 'f') { out += '\f'; continue; }
    if (n === 'v') { out += '\v'; continue; }
    if (n === 'x') {
      const code = parseInt(raw.slice(i + 1, i + 3), 16);
      if (Number.isFinite(code)) { out += String.fromCharCode(code); i += 2; continue; }
      out += n;
      continue;
    }
    if (n === 'u') {
      if (raw[i + 1] === '{') {
        const close = raw.indexOf('}', i + 2);
        const code = close === -1 ? NaN : parseInt(raw.slice(i + 2, close), 16);
        if (Number.isFinite(code)) { out += String.fromCodePoint(code); i = close; continue; }
      } else {
        const code = parseInt(raw.slice(i + 1, i + 5), 16);
        if (Number.isFinite(code)) { out += String.fromCharCode(code); i += 4; continue; }
      }
      out += n;
      continue;
    }
    out += n;
  }
  return out;
}

// The placeholder a `${...}` substitution leaves in a template's recorded text.
// It can never occur in real source, so a text carrying it is provably dynamic.
const SUBSTITUTION = '\u0000';

// One pass over the source producing BOTH products the scan needs:
//   stripped — comments blanked, length- and newline-preserving
//   literals — every string/template literal as {start, end, text, dynamic}
//              with `text` cooked and `${...}` replaced by SUBSTITUTION
export function tokenize(src) {
  let out = '';
  let i = 0;
  let prevSig = ''; // last significant CODE character emitted
  let prevSigIdx = -1;
  const literals = [];
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
        ctx.raw += src.slice(i, i + 2);
        i += 2;
        continue;
      }
      if (c === '`') {
        emit(c);
        i += 1;
        stack.pop();
        literals.push({
          start: ctx.start,
          end: i,
          dynamic: ctx.dynamic,
          text: cookEscapes(ctx.raw),
        });
        prevSig = '`';
        prevSigIdx = i;
        continue;
      }
      if (c === '$' && d === '{') {
        emit('${');
        ctx.dynamic = true;
        ctx.raw += SUBSTITUTION;
        i += 2;
        stack.push({ kind: 'code', brace: 0 });
        prevSig = '';
        prevSigIdx = -1;
        continue;
      }
      emit(c);
      ctx.raw += c;
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
      // A properly closed literal ends ON its quote; an unterminated one
      // (endOfQuoted degraded at the newline) has no closer to trim.
      const closed = j > i + 1 && src[j - 1] === c;
      literals.push({
        start: i,
        end: j,
        dynamic: false,
        text: cookEscapes(src.slice(i + 1, closed ? j - 1 : j)),
      });
      prevSig = c;
      prevSigIdx = j;
      i = j;
      continue;
    }

    if (c === '`') {
      emit(c);
      i += 1;
      stack.push({ kind: 'template', start: i - 1, dynamic: false, raw: '' });
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

  return { stripped: out, literals };
}

export function stripComments(src) {
  return tokenize(src).stripped;
}

export function stringLiterals(src) {
  return tokenize(src).literals;
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
// Matching: WHOLE FILE, never line by line (re-review round 2, Critical 2).
//
// Round 2 widened four ingredients from single tokens to multi-token patterns
// whose `\s*` / `[^…]*` classes tolerate newlines — and then kept testing them
// one line at a time, so the per-line loop was the only thing stopping them
// from matching. `prettier` at a narrow print width, `eslint --fix`, or a hand
// wrap past 100 columns silently disarmed all four:
//
//     return path.join(          const [                  const _u = import
//       root,                      ,                        .meta.url;
//       'skills',                  entryScript,
//       'vibe',                  ] = process.argv;
//     );
//
// All three are ordinary valid JS, all three are complete re-derivations, and
// all three passed. The ingredient DID appear verbatim in the source text, so
// the file's own stated boundary was false as well.
//
// The fix is structural rather than another regex: match the whole
// comment-stripped file and derive the line from the match offset. The
// stripper has been length- and newline-preserving from the start precisely so
// offsets address the original file; this is what finally uses that.
//
// Two properties are preserved deliberately:
//   * `m` is added along with `g`, so `^` still means START OF LINE. Without it
//     `root-markers`' `(^|[^A-Za-z0-9_$.])` would fall through to the class,
//     swallow the preceding newline, and every line-initial `.spec` would look
//     like a multi-line match.
//   * results are collapsed to ONE hit per (rule, start line), exactly what a
//     per-line boolean produced. Waiver occurrence counts therefore mean the
//     same thing they meant before, and no waiver needed re-counting.
// Scanning resumes at match.index + 1 rather than at the match end, so a long
// match can never hide a second one that starts inside it.
//
// A match that SPANS lines has no single line to waive, so it is an unwaivable
// violation reported as `file:start-end`. Nothing in the real tree spans (this
// is asserted below, not assumed), so the change costs zero waiver churn.
// ---------------------------------------------------------------------------

function compileWholeFile(re) {
  const flags = new Set([...re.flags, 'g', 'm']);
  return new RegExp(re.source, [...flags].join(''));
}

function regexHits(src, re) {
  const g = compileWholeFile(re);
  const hits = [];
  let m;
  while ((m = g.exec(src)) !== null) {
    hits.push({ start: m.index, end: m.index + m[0].length });
    g.lastIndex = m.index + 1;
  }
  return hits;
}

// offset -> 1-based line number, by binary search over the line starts.
function makeLineOf(src) {
  const starts = [0];
  for (let i = 0; i < src.length; i += 1) if (src[i] === '\n') starts.push(i + 1);
  return (offset) => {
    let lo = 0;
    let hi = starts.length - 1;
    while (lo < hi) {
      const mid = (lo + hi + 1) >> 1;
      if (starts[mid] <= offset) lo = mid;
      else hi = mid - 1;
    }
    return lo + 1;
  };
}

// ---------------------------------------------------------------------------
// The module-reach rule's resolver.
// ---------------------------------------------------------------------------

// A literal is treated as a module specifier when it is shaped like one. Both
// halves matter: `'../tests/helpers.mjs'` and `'./x'` are caught by the prefix,
// `'/abs/helpers.mjs'` and a bare `'helpers.mjs'` by the extension. Node
// builtins (`'node:fs'`) match neither, which is why they need no exemption.
//
// This deliberately OVER-approximates: a `'../..'` used as an ordinary path
// segment rather than as a specifier is checked too, and would need a waiver if
// it ever resolved outside the tree. That is the right direction to be wrong in
// — a literal a shipped module holds that points out of flow/engine/ is worth a
// sentence of justification whichever loader eventually reads it — and it costs
// nothing today: every `./`/`../` literal in the real engine IS an import
// specifier, and every one resolves into the scanned set.
const SPECIFIER_PREFIX_RE = /^\.\.?[/\\]/;
const SPECIFIER_EXT_RE = /\.(mjs|js|cjs)$/;

function looksLikeSpecifier(text) {
  return SPECIFIER_PREFIX_RE.test(text) || SPECIFIER_EXT_RE.test(text);
}

function moduleReachHits({ dir, rel, literals, sourceSet }) {
  const hits = [];
  const baseDir = path.dirname(path.join(dir, rel));
  for (const lit of literals) {
    // For a dynamic template the static head is what a reader can still see;
    // either it or the whole text being specifier-shaped is enough to ask.
    const head = lit.text.split(SUBSTITUTION)[0];
    if (!looksLikeSpecifier(lit.text) && !looksLikeSpecifier(head)) continue;

    if (lit.dynamic) {
      hits.push({
        start: lit.start,
        end: lit.end,
        detail:
          'the specifier is built from a substitution, so it cannot be shown to resolve inside the scanned set at all',
      });
      continue;
    }

    const resolved = path.resolve(baseDir, lit.text);
    const relResolved = path.relative(dir, resolved).split(path.sep).join('/');
    if (relResolved !== '' && !relResolved.startsWith('../') && sourceSet.has(relResolved)) continue;
    hits.push({
      start: lit.start,
      end: lit.end,
      detail: `it resolves to ${relResolved === '' ? '.' : relResolved}, which flow/engine/ does not ship as a scanned source`,
    });
  }
  return hits;
}

function ruleHits(rule, ctx) {
  return rule.find ? rule.find(ctx) : regexHits(ctx.src, rule.re);
}

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
// KNOWN LIMITS (stated, not papered over — and each one PINNED BY A TEST at the
// bottom of this file, so a limit that is narrower or wider than claimed shows
// up as a failure rather than as the next reviewer's finding. Round 1 wrote a
// limit narrower than reality — it said "concatenation" when line continuation
// evaded too — which is why these are now executable rather than prose.)
//
//   1. The INGREDIENT rules match the comment-stripped SOURCE TEXT, so any
//      source-level split or re-spelling of an ingredient defeats them:
//      concatenation (`'state' + '.json'`), a line continuation inside the
//      string, a computed name (`['state','json'].join('.')`), a unicode escape
//      (`'state.json'`), or a value built at runtime (`Buffer.from(...)`,
//      `new Function(...)`). The boundary is exactly "the ingredient appears
//      verbatim in the comment-stripped source text" — and, since fix round 3,
//      that text is the WHOLE FILE, so line breaks anywhere inside the pattern
//      no longer help. Closing the rest needs an AST plus constant folding,
//      which needs a dependency this engine will not take (R5).
//   2. `module-reach` resolves ONE LITERAL at a time. It does cook that
//      literal's own escapes, so a unicode-escaped separator and a
//      backslash-newline continuation inside the specifier are both resolved
//      rather than evasions. It does NOT fold concatenation or variables — but
//      note the asymmetry that makes this narrower than it sounds: the rule
//      fires if ANY fragment is specifier-shaped, so `'../tests/' + 'x.mjs'`
//      and `d + '/helpers.mjs'` are both caught. Evading requires EVERY
//      fragment to be inert — splitting the dot from the extension and the
//      `..` from its slash — which is not a shape anyone writes by accident.
//      That is the real boundary, and it is pinned below.
//   3. Laundering through an ALLOWLISTED consumer's own name is closed for
//      re-exports (unwaivable, see `noReexport`) and for any second use of
//      the helper (the allowlist is occurrence-counted), but a consumer that
//      returned the path from an existing, differently-named export would
//      still pass. That takes a deliberate edit to an allowlisted file, which
//      is the narrowest surface this can be reduced to without dataflow.
//   4. Reaching a primitive's file by enumeration rather than by name
//      (readdirSync + a filter) is not banned, because doctor.mjs enumerates
//      directories legitimately. It re-derives nothing on its own; it would
//      still have to identify which file it found.
//   5. ASCENDING from a value a sanctioned resolver returned is textually
//      indistinguishable from any other `path.dirname(someVariable)` — and
//      doctor.mjs's rootForReport() is exactly that shape, reviewed and
//      sanctioned (round-1 Finding 3). The single-expression spelling
//      (`path.dirname(path.dirname(x))`) IS caught by `self-relative-ascent`,
//      wrapped or not; the same walk split across two statements with an
//      intermediate variable is not. Telling the two apart needs dataflow, not
//      a wider regex — a blunter rule would red-flag json.mjs's ordinary
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
    //
    // `PWD` is banned as a bare NAME, the same way `CLAUDE_PROJECT_DIR` is:
    // process.env.PWD is the shell-idiomatic route to the same value and
    // matched nothing here (re-review round 2, Minor 4). Banning the token
    // rather than the access shape covers member, computed, destructured and
    // aliased in one, which is the whole point of banning ingredients. No
    // engine module has ever named it, so this owns nothing and waives nothing.
    re: /process\s*(?:\.\s*cwd\b|\[\s*['"`]\s*cwd)|\{[^}]*\bcwd\b[^}]*\}\s*=\s*process\b|=\s*process\s*[;,)]|\bPWD\b/,
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
  // --- REACHABILITY, not import syntax (re-review round 2, Critical 1).
  //
  // tests/ is exempt because fixtures legitimately name every ingredient. That
  // exemption is sound only while nothing SHIPPED can reach into it: otherwise
  // "put the duplicate reader in tests/helpers.mjs and load it" evades the
  // entire scan in one line, and the scan's own comment tells the next reviewer
  // it cannot happen.
  //
  // Round 2's rule was `\bfrom\s*['"][^'"]*\btests\//` — the static `from`
  // clause, a SPELLING. `await import('../tests/helpers.mjs')` has no `from`;
  // neither does `require()` in a `.cjs` (which IS in SOURCE_EXTENSIONS), nor
  // `createRequire`, nor whatever loader lands next. Adding `import\s*\(` would
  // have been the fourth instance-patch in a row.
  //
  // So the rule is inverted the same way the ingredients were. Every loader has
  // to WRITE THE PATH, so the path is what gets checked: every string or
  // template literal shaped like a module specifier — one that starts `./`/`../`
  // or ends `.mjs`/`.js`/`.cjs` — is resolved against its own module's directory
  // and must land on a member of listEngineSources(). That set is exactly the
  // set this scan reads, so the scanned tree is closed under module resolution
  // by induction: a shipped module can only reach files that are themselves
  // scanned. tests/, and everything else outside flow/engine/, is unreachable
  // by construction rather than by enumerating the syntax of the day.
  //
  // A specifier that cannot be resolved statically (a template with a `${}`
  // substitution) is a violation for the same reason: unresolvable means
  // unprovable. cli.mjs's command dispatch is the engine's ONE such specifier
  // and carries a counted, per-line waiver like everything else.
  {
    id: 'module-reach',
    owners: [],
    find: moduleReachHits,
    why: 'reaches a module outside the scanned engine source set — fixture and out-of-tree code is never scanned, so nothing shipped may load it',
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
  'cli-command-dispatch':
    "the engine's ONE module specifier that is not a plain literal: cli.mjs joins `commands/<name>.mjs` where <name> is already constrained to the COMMANDS array three lines above it, and dispatch cannot be a static import without importing every command on every invocation. Waived per LINE and per OCCURRENCE COUNT like everything else — a second unresolvable specifier anywhere in the engine, or a byte-identical copy of this one, is a violation.",
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
  { file: 'cli.mjs', id: 'module-reach', reason: 'cli-command-dispatch', line: "const modulePath = path.join(__dirname, 'commands', `${name}.mjs`);" },

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

// One analysis pass per file, shared by scanEngineTree() and
// waiverOccurrences() so the two can never disagree about what a hit is.
// Returns Map(ruleId -> [{startLine, endLine, trimmed, line, detail}]),
// collapsed to one hit per (rule, start line).
export function analyzeFile(dir, rel, sourceSet) {
  const { stripped, literals } = tokenize(readFileSync(path.join(dir, rel), 'utf8'));
  const lines = stripped.split('\n');
  const lineOf = makeLineOf(stripped);
  const ctx = { dir, rel, src: stripped, literals, sourceSet };

  const byRule = new Map();
  for (const rule of INGREDIENTS) {
    const byLine = new Map();
    for (const h of ruleHits(rule, ctx)) {
      const startLine = lineOf(h.start);
      const endLine = lineOf(Math.max(h.start, h.end - 1));
      const prev = byLine.get(startLine);
      // Prefer the widest hit on a line: a spanning match must never be masked
      // by a same-line single-line one, because spanning is the unwaivable case.
      if (prev && prev.endLine >= endLine) continue;
      const line = lines[startLine - 1] ?? '';
      byLine.set(startLine, { startLine, endLine, line, trimmed: line.trim(), detail: h.detail });
    }
    byRule.set(rule.id, [...byLine.values()].sort((a, b) => a.startLine - b.startLine));
  }
  return byRule;
}

// Returns a flat, sorted array of violation strings — empty when the tree is
// clean. `dir` is a flow/engine/ root (the real one, or a scratch copy).
export function scanEngineTree(dir) {
  const sources = listEngineSources(dir);
  const sourceSet = new Set(sources);
  const violations = [];

  for (const rel of sources) {
    const byRule = analyzeFile(dir, rel, sourceSet);

    // Per-file waiver budget, spent as occurrences are met. Once a waiver's
    // allowance is exhausted, every further copy of that line is a violation.
    const budget = new Map();
    for (const w of WAIVERS) {
      if (w.file === rel) budget.set(waiverKey(w), waiverCount(w));
    }

    for (const ing of INGREDIENTS) {
      if (ing.owners.includes(rel)) continue;
      for (const hit of byRule.get(ing.id) ?? []) {
        const why = hit.detail ? `${ing.why} — ${hit.detail}` : ing.why;

        // Re-exporting a primitive-path helper out of an allowlisted consumer
        // would hand every other module a fresh, unbanned name for it — the
        // round-1 hole one indirection further out. Unwaivable by design.
        if (ing.noReexport && /\bexport\b/.test(hit.line)) {
          violations.push(
            `${rel}:${hit.startLine}: [${ing.id}] re-exports a primitive path helper — only its owner may export it\n    | ${hit.trimmed}`,
          );
          continue;
        }

        // A match that crosses a newline has no single source line to pin a
        // waiver to, so it is never waivable — which is the correct outcome:
        // the waiver mechanism describes lines, and this is not one.
        if (hit.endLine !== hit.startLine) {
          violations.push(
            `${rel}:${hit.startLine}-${hit.endLine}: [${ing.id}] ${why} (the match spans lines ${hit.startLine}-${hit.endLine}, so there is no single line to waive)\n    | ${hit.trimmed}`,
          );
          continue;
        }

        const key = `${rel}|${ing.id}|${hit.trimmed}`;
        const left = budget.get(key) ?? 0;
        if (left > 0) {
          budget.set(key, left - 1);
          continue;
        }
        violations.push(`${rel}:${hit.startLine}: [${ing.id}] ${why}\n    | ${hit.trimmed}`);
      }
    }
  }
  return violations.sort();
}

// How many times each waiver's line actually occurs in `dir`, keyed by
// waiverKey(). Zero means a stale waiver; more than the declared `count`
// means a new re-derivation was pasted in beside a blessed one. Both are
// failures — see the two tests below. Counted off the SAME hits the scan
// spends budget against (a spanning hit is not waivable, so it is not counted).
export function waiverOccurrences(dir) {
  const counts = new Map();
  const sources = listEngineSources(dir);
  const sourceSet = new Set(sources);
  for (const rel of sources) {
    const relevant = WAIVERS.filter((w) => w.file === rel);
    if (relevant.length === 0) continue;
    const byRule = analyzeFile(dir, rel, sourceSet);
    for (const w of relevant) {
      for (const hit of byRule.get(w.id) ?? []) {
        if (hit.endLine !== hit.startLine) continue;
        if (hit.trimmed !== w.line) continue;
        counts.set(waiverKey(w), (counts.get(waiverKey(w)) ?? 0) + 1);
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
    expect: ['cursor-file'],
    primitive: 'cursor reader',
    code: "function dupCursorRead(vibeDir) {\n  return JSON.parse(fs.readFileSync(path.join(vibeDir, 'state.json'), 'utf8'));\n}\n",
  },
  {
    id: 'M2 split read-then-parse (json.mjs\'s own house style)',
    expect: ['cursor-file'],
    primitive: 'cursor reader',
    code: "function dupCursorRead(vibeDir) {\n  const raw = fs.readFileSync(path.join(vibeDir, 'state.json'), 'utf8');\n  return JSON.parse(raw);\n}\n",
  },
  {
    id: 'M3 aliased readFileSync import',
    expect: ['cursor-file'],
    primitive: 'cursor reader',
    code: "import { readFileSync as _rfs } from 'node:fs';\nfunction dupCursorRead(vibeDir) {\n  return JSON.parse(_rfs(path.join(vibeDir, 'state.json'), 'utf8'));\n}\n",
  },
  {
    id: 'M4 path held in a variable',
    expect: ['cursor-file'],
    primitive: 'cursor reader',
    code: "function dupCursorRead(vibeDir) {\n  const cursorFile = path.join(vibeDir, 'state.json');\n  return JSON.parse(fs.readFileSync(cursorFile, 'utf8'));\n}\n",
  },
  {
    id: "M15 the engine's own readJson() against the cursor path",
    expect: ['cursor-file'],
    primitive: 'cursor reader',
    code: "import { readJson as _rj } from '../json.mjs';\nfunction dupCursorRead(vibeDir) {\n  return _rj(path.join(vibeDir, 'state.json'));\n}\n",
  },
  {
    id: 'M16 alias + filename constant + template-literal path',
    expect: ['cursor-file'],
    primitive: 'cursor reader',
    code: "import { readFileSync as _rfs2 } from 'node:fs';\nconst CURSOR_FILE = 'state.json';\nfunction dupCursorRead(vibeDir) {\n  return JSON.parse(_rfs2(`${vibeDir}/${CURSOR_FILE}`, 'utf8'));\n}\n",
  },
  {
    id: 'M17 duplicate cursor WRITER',
    expect: ['cursor-file'],
    primitive: 'cursor writer',
    code: "import { writeFileSync as _wfs } from 'node:fs';\nfunction dupCursorWrite(vibeDir, body) {\n  _wfs(path.join(vibeDir, 'state.json'), `${JSON.stringify(body, null, 2)}\\n`);\n}\n",
  },
  {
    id: 'M5 raw path.join for the machine file',
    expect: ['machine-file'],
    primitive: 'machine loader',
    code: "function dupMachinePath(vibeDir) {\n  return path.join(vibeDir, 'state-machine.json');\n}\n",
  },
  {
    id: 'M6 nested call inside the join arguments',
    expect: ['machine-file'],
    primitive: 'machine loader',
    code: "function dupMachinePath(vibeDir) {\n  return path.join(path.resolve(vibeDir), 'state-machine.json');\n}\n",
  },
  {
    id: 'M7 template-literal machine path',
    expect: ['machine-file'],
    primitive: 'machine loader',
    code: 'function dupMachinePath(vibeDir) {\n  return `${vibeDir}/state-machine.json`;\n}\n',
  },
  {
    id: 'M8 path.resolve instead of path.join',
    expect: ['machine-file'],
    primitive: 'machine loader',
    code: "function dupMachinePath(vibeDir) {\n  return path.resolve(vibeDir, 'state-machine.json');\n}\n",
  },
  {
    id: 'M9 direct process.env member read',
    expect: ['project-dir-env'],
    primitive: 'root resolver',
    code: 'function dupProjectDir() {\n  return process.env.CLAUDE_PROJECT_DIR;\n}\n',
  },
  {
    id: 'M10 destructured process.env',
    expect: ['project-dir-env'],
    primitive: 'root resolver',
    code: 'function dupProjectDir() {\n  const { CLAUDE_PROJECT_DIR } = process.env;\n  return CLAUDE_PROJECT_DIR;\n}\n',
  },
  {
    id: 'M11 aliased process.env',
    expect: ['project-dir-env'],
    primitive: 'root resolver',
    code: 'const _env = process.env;\nfunction dupProjectDir() {\n  return _env.CLAUDE_PROJECT_DIR;\n}\n',
  },
  {
    id: 'M18b duplicate root resolver (upward marker search)',
    expect: ['root-markers'],
    primitive: 'root resolver',
    code: "function dupResolveRoot(start) {\n  let dir = start;\n  for (;;) {\n    if (fs.existsSync(path.join(dir, '.spec')) || fs.existsSync(path.join(dir, '.git'))) return dir;\n    const up = path.dirname(dir);\n    if (up === dir) return undefined;\n    dir = up;\n  }\n}\n",
  },
  {
    id: 'M19 re-derived `.agents/skills/vibe` layout join',
    expect: ['vibe-layout'],
    primitive: 'root resolver',
    code: "function dupVibeDir(root) {\n  return path.join(root, '.agents', 'skills', 'vibe');\n}\n",
  },
  {
    id: 'M20 self-relative resolution from the module\'s own URL',
    expect: ['self-location'],
    primitive: 'root resolver',
    code: 'const _here = path.dirname(fileURLToPath(import.meta.url));\nfunction dupSelfRoot() {\n  return path.dirname(path.dirname(_here));\n}\n',
  },
  {
    id: 'M18a duplicate block extractor (marker grammar re-implementation)',
    expect: ['marker-grammar'],
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
    expect: ['cursor-path-helper'],
    primitive: 'cursor reader',
    code: "import { cursorPath as _cp } from '../cursor.mjs';\nimport { readJson as _rj } from '../json.mjs';\nexport function dupReadCursor(vibeDir) {\n  const raw = _rj(_cp(vibeDir));\n  return { flow: raw.flow, phase: raw.phase, feature: raw.feature ?? null };\n}\n",
  },
  {
    id: 'E2 helper reuse — JSON.parse(readFileSync(cursorPath(v)))',
    expect: ['cursor-path-helper'],
    primitive: 'cursor reader',
    code: "import { cursorPath as _cp2 } from '../cursor.mjs';\nimport { readFileSync as _rfs3 } from 'node:fs';\nfunction dupReadCursor2(vibeDir) {\n  return JSON.parse(_rfs3(_cp2(vibeDir), 'utf8'));\n}\n",
  },
  {
    id: 'E3 helper reuse — duplicate cursor WRITER via writeFileSync(cursorPath(v))',
    expect: ['cursor-path-helper'],
    primitive: 'cursor writer',
    code: "import { cursorPath as _cp3 } from '../cursor.mjs';\nimport { writeFileSync as _wfs2 } from 'node:fs';\nfunction dupCursorWrite2(vibeDir, body) {\n  _wfs2(_cp3(vibeDir), `${JSON.stringify(body, null, 2)}\\n`);\n}\n",
  },
  {
    id: 'E4 helper reuse — duplicate machine loader via readJson(machinePath(v))',
    expect: ['machine-path-helper'],
    primitive: 'machine loader',
    code: "import { machinePath as _mp } from '../machine.mjs';\nimport { readJson as _rj2 } from '../json.mjs';\nfunction dupLoadMachine(vibeDir) {\n  return _rj2(_mp(vibeDir));\n}\n",
  },
  {
    id: 'E14 helper reuse — path in a variable, then hand-rolled field validation',
    expect: ['cursor-path-helper'],
    primitive: 'cursor reader',
    code: "import { cursorPath as _cp4 } from '../cursor.mjs';\nimport { readJson as _rj3 } from '../json.mjs';\nfunction dupReadCursor3(vibeDir) {\n  const statePath = _cp4(vibeDir);\n  const raw = _rj3(statePath);\n  return typeof raw.flow === 'string' ? raw : { flow: 'idle', phase: 'idle' };\n}\n",
  },
  {
    id: 'E15 helper reuse via a NAMESPACE import (no named binding to grep for)',
    expect: ['cursor-path-helper'],
    primitive: 'cursor reader',
    code: "import * as _cursorMod from '../cursor.mjs';\nimport { readJson as _rj4 } from '../json.mjs';\nfunction dupReadCursor4(vibeDir) {\n  return _rj4(_cursorMod.cursorPath(vibeDir));\n}\n",
  },
  {
    id: "E16 helper reuse via computed member access (_mod['cursorPath'])",
    expect: ['cursor-path-helper'],
    primitive: 'cursor reader',
    code: "import * as _cursorMod2 from '../cursor.mjs';\nimport { readJson as _rj5 } from '../json.mjs';\nfunction dupReadCursor5(vibeDir) {\n  return _rj5(_cursorMod2['cursorPath'](vibeDir));\n}\n",
  },
  {
    id: 'E17 second consumer of resolveProjectCursorDir (doctrine precedence re-derived)',
    expect: ['project-cursor-dir-helper'],
    primitive: 'root resolver',
    code: "import { resolveProjectCursorDir as _rpcd } from '../root.mjs';\nfunction dupDoctrineCursorDir(vibeDir) {\n  return _rpcd() ?? vibeDir;\n}\n",
  },

  // --- Finding 4: the process.env destructure/alias trick, applied to the
  // ingredients whose siblings never got the same treatment.
  {
    id: 'E5 destructured import.meta (const { url: _u } = import.meta)',
    expect: ['self-location'],
    primitive: 'root resolver',
    code: "const { url: _u } = import.meta;\nconst _selfDir = path.dirname(fileURLToPath(_u));\nfunction dupSelfRoot2() {\n  return path.dirname(path.dirname(_selfDir));\n}\n",
  },
  {
    id: 'E6 aliased import.meta (const _meta = import.meta)',
    expect: ['self-location'],
    primitive: 'root resolver',
    code: "const _meta = import.meta;\nconst _selfDir2 = path.dirname(fileURLToPath(_meta.url));\nfunction dupSelfRoot3() {\n  return path.dirname(path.dirname(_selfDir2));\n}\n",
  },
  {
    id: "E7 bracket-notation process['cwd']()",
    expect: ['cwd-fallback'],
    primitive: 'root resolver',
    code: "function dupCwdRoot() {\n  return process['cwd']();\n}\n",
  },
  {
    id: 'E7b destructured cwd off process',
    expect: ['cwd-fallback'],
    primitive: 'root resolver',
    code: 'const { cwd: _cwd } = process;\nfunction dupCwdRoot2() {\n  return _cwd();\n}\n',
  },
  {
    id: 'E7c aliased process object, then .cwd()',
    expect: ['cwd-fallback'],
    primitive: 'root resolver',
    code: 'const _proc = process;\nfunction dupCwdRoot3() {\n  return _proc.cwd();\n}\n',
  },
  {
    id: 'E7d process.env.PWD — the shell-idiomatic route to the same value (re-review Minor 4)',
    expect: ['cwd-fallback'],
    primitive: 'root resolver',
    code: 'function dupCwdRoot5() {\n  return process.env.PWD;\n}\n',
  },
  {
    id: 'E7e destructured PWD off process.env',
    expect: ['cwd-fallback'],
    primitive: 'root resolver',
    code: 'const { PWD: _pwd } = process.env;\nfunction dupCwdRoot6() {\n  return _pwd;\n}\n',
  },
  {
    id: "E8 partial-path join — path.join(root, 'skills', 'vibe') (pluginVibeDir's leg, never names .agents)",
    expect: ['vibe-layout'],
    primitive: 'root resolver',
    code: "function dupPluginVibeDir(root) {\n  return path.join(root, 'skills', 'vibe');\n}\n",
  },
  {
    id: 'E8b partial-path join as a template literal (`${root}/skills/vibe`)',
    expect: ['vibe-layout'],
    primitive: 'root resolver',
    code: 'function dupPluginVibeDir2(root) {\n  return `${root}/skills/vibe`;\n}\n',
  },
  {
    id: 'E18 self-location off the entry script (process.argv[1])',
    expect: ['self-location'],
    primitive: 'root resolver',
    code: 'function dupEntryRoot() {\n  return path.dirname(path.dirname(process.argv[1]));\n}\n',
  },
  {
    id: 'E19 self-location off a DESTRUCTURED process.argv (no brackets to grep)',
    expect: ['self-location'],
    primitive: 'root resolver',
    code: 'const [, _script] = process.argv;\nfunction dupEntryRoot2() {\n  return path.dirname(_script);\n}\n',
  },
  {
    id: 'E21 the re-derivation moved INTO the exempt tests/ directory and imported back out',
    expect: ['module-reach'],
    primitive: 'cursor reader',
    code: "import { dupCursorRead as _dcr } from '../tests/helpers.mjs';\nexport const readIt = (vibeDir) => _dcr(vibeDir);\n",
  },

  // --- fix-round-2 re-review, CRITICAL 1: reaching the unscanned tests/ tree by
  // any loader that is not a static `from` clause. Every one of these was a
  // working duplicate cursor reader against round 2's scan; the reviewer ran
  // E22 in the real tree and got the live cursor back with all four suites
  // green. They are here as a FAMILY rather than as the one spelling reported,
  // because the point of the module-reach rule is that the loader is irrelevant.
  {
    id: 'E22 dynamic import() of the exempt tests/ directory (no `from` clause to match)',
    expect: ['module-reach'],
    primitive: 'cursor reader',
    code: "export async function readCursorAgain(vibeDir) {\n  const m = await import('../tests/helpers.mjs');\n  return m.dupCursorRead(vibeDir);\n}\n",
  },
  {
    id: 'E23 require() of the exempt tests/ directory (.cjs is in SOURCE_EXTENSIONS)',
    expect: ['module-reach'],
    primitive: 'cursor reader',
    code: "const _h = require('../tests/helpers.js');\nexport const readIt2 = (vibeDir) => _h.dupCursorRead(vibeDir);\n",
  },
  {
    id: 'E24 createRequire() of the exempt tests/ directory',
    expect: ['module-reach'],
    primitive: 'cursor reader',
    code: "import { createRequire as _cr } from 'node:module';\nconst _req = _cr('file:///x');\nexport const readIt3 = (vibeDir) => _req('../tests/helpers.js').dupCursorRead(vibeDir);\n",
  },
  {
    id: 'E25 dynamic import() whose specifier is CONCATENATED (no whole specifier literal)',
    expect: ['module-reach'],
    primitive: 'cursor reader',
    code: "export async function readIt4(vibeDir) {\n  const m = await import('../tests/' + 'helpers.mjs');\n  return m.dupCursorRead(vibeDir);\n}\n",
  },
  {
    id: 'E26 dynamic import() through a template SUBSTITUTION (unresolvable, so unprovable)',
    expect: ['module-reach'],
    primitive: 'cursor reader',
    code: "export async function readIt5(vibeDir, n) {\n  const m = await import(`../tests/${n}.mjs`);\n  return m.dupCursorRead(vibeDir);\n}\n",
  },
  {
    id: 'E27 reaching OUT of flow/engine/ entirely (a sibling tree nothing scans either)',
    expect: ['module-reach'],
    primitive: 'cursor reader',
    code: "import { dupCursorRead as _dcr2 } from '../../scripts/helpers.mjs';\nexport const readIt6 = (vibeDir) => _dcr2(vibeDir);\n",
  },

  // --- fix-round-2 re-review, CRITICAL 2: ordinary line WRAPPING. Every one of
  // these is the single-line mutant above it with a newline inserted where
  // prettier, `eslint --fix`, or a hand wrap past 100 columns would put one.
  // Round 2's scanner matched `ing.re.test(lines[i])`, so all of them passed
  // while their unwrapped twins failed — the same semantics, different
  // formatting, opposite verdicts. They are pinned as a family because the
  // defect was the MATCHER, not these four patterns.
  {
    id: "F1 E8 wrapped by prettier — path.join(root,\\n 'skills',\\n 'vibe')",
    expect: ['vibe-layout'],
    primitive: 'root resolver',
    code: "function dupPluginVibeDir3(pluginRootDirectory) {\n  return path.join(\n    pluginRootDirectory,\n    'skills',\n    'vibe',\n  );\n}\n",
  },
  {
    id: 'F2 a complete self-locating resolver with ONE newline inside `import.meta`',
    expect: ['self-location'],
    primitive: 'root resolver',
    code: 'const _selfUrl = import\n  .meta.url;\nfunction dupSelfRoot4() {\n  const here = path.dirname(fileURLToPath(_selfUrl));\n  const up = path.dirname(here);\n  return path.dirname(up);\n}\n',
  },
  {
    id: 'F3 E19 with the array destructure wrapped across four lines',
    expect: ['self-location'],
    primitive: 'root resolver',
    code: 'const [\n  ,\n  _entryScript,\n] = process.argv;\nfunction dupEntryRoot3() {\n  return path.dirname(_entryScript);\n}\n',
  },
  {
    id: 'F4 the dot-dot ascent wrapped — path.dirname(\\n path.dirname(x),\\n )',
    expect: ['self-relative-ascent'],
    primitive: 'root resolver',
    code: 'function dupEntryRoot4(entryScript) {\n  return path.dirname(\n    path.dirname(entryScript),\n  );\n}\n',
  },
  {
    id: 'F5 E7b with the destructure wrapped — const {\\n cwd: _c,\\n } = process',
    expect: ['cwd-fallback'],
    primitive: 'root resolver',
    code: 'const {\n  cwd: _cwd2,\n} = process;\nfunction dupCwdRoot4() {\n  return _cwd2();\n}\n',
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
    expect: ['self-relative-ascent'],
    code: 'function smuggledRoot() {\n  return path.dirname(path.dirname(__dirname));\n}\n',
  },
  {
    id: 'E10 smuggled root resolver via path.resolve(__dirname, ..., ...)',
    expect: ['self-relative-ascent'],
    code: "const _smuggledRoot = path.resolve(__dirname, '..', '..');\n",
  },
  {
    id: "E13 smuggled root resolver via a MULTI-segment dot-dot literal ('../..')",
    expect: ['self-relative-ascent'],
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
  {
    id: 'E28 a duplicate cursor reader in a .cjs module',
    // `.cjs` loads under a "type": "module" package too, and it is the one
    // extension where `require()` is the NATIVE loader — which is how the
    // round-2 `tests-import` regex (a `from`-clause matcher) was walked past.
    site: 'commands/dup.cjs',
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
        violations.some((v) => v.startsWith(`${mutant.site}:`) && v.includes('[cursor-file]')),
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

// Every mutant declares the RULE it is aimed at, and the assertion demands a
// violation carrying that rule's id — not merely "some violation at this site".
//
// That distinction became load-bearing in fix round 3. Several mutants carry a
// relative import (`'../cursor.mjs'`, `'../json.mjs'`) which resolves correctly
// from commands/ but NOT from cli.mjs or commands/sub/, where the new
// module-reach rule quite rightly flags it. Under the old "some violation"
// assertion, deleting `cursor-path-helper` outright would have left all seven
// helper-reuse mutants green in two of their three plant sites — a corpus
// passing for a reason unrelated to what it tests. This is the fix for that
// before it becomes the next round's finding.
function assertCaughtBy(mutant, site, violations) {
  const expected = mutant.expect ?? [];
  assert(expected.length > 0, `mutant ${mutant.id} declares no expected rule`);
  const atSite = violations.filter((v) => v.startsWith(`${site}:`));
  assert(
    atSite.some((v) => expected.some((id) => v.includes(`[${id}]`))),
    `EVADED: ${mutant.id} planted in ${site} was not caught by ${expected.join('/')}.\n` +
      `scan reported at this site: ${JSON.stringify(atSite)}`,
  );
}

test('primitive scan: every mutant declares an expected rule that actually exists', () => {
  // A corpus entry with no target, or a target that was renamed out from under
  // it, silently downgrades to "any violation will do" — which is exactly the
  // assertion this replaced.
  const known = new Set(INGREDIENTS.map((i) => i.id));
  const bad = [];
  for (const m of [...MUTANTS, ...CLI_ASCENT_MUTANTS]) {
    if (!Array.isArray(m.expect) || m.expect.length === 0) bad.push(`${m.id}: no expect[]`);
    for (const id of m.expect ?? []) if (!known.has(id)) bad.push(`${m.id}: unknown rule '${id}'`);
  }
  assertEqual(bad, [], `mutants with a missing or stale target rule:\n${bad.join('\n')}`);
});

for (const mutant of MUTANTS) {
  test(`discriminating: ${mutant.primitive} — ${mutant.id} — is caught in every plant site`, () => {
    for (const site of PLANT_SITES) {
      const dir = makeScratchEngine();
      try {
        assertEqual(scanEngineTree(dir), [], 'sanity: an untouched scratch copy must start clean');
        plantMutant(dir, site, mutant.code);
        assertCaughtBy(mutant, site, scanEngineTree(dir));
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
      assertCaughtBy(mutant, 'cli.mjs', scanEngineTree(dir));
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

// ---------------------------------------------------------------------------
// Fix round 3 — the matcher's OWN properties, and the stated limits, executable.
// ---------------------------------------------------------------------------

// Helper: plant `code` in commands/orders.mjs (an ordinary command module where
// the mutants' `'../…'` imports resolve correctly, so nothing fires by
// accident) and return the violations reported for that file.
function violationsFor(code, site = 'commands/orders.mjs') {
  const dir = makeScratchEngine();
  try {
    assertEqual(scanEngineTree(dir), [], 'sanity: an untouched scratch copy must start clean');
    plantMutant(dir, site, code);
    return scanEngineTree(dir).filter((v) => v.startsWith(`${site}:`));
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
}

test('matcher: no ingredient match in the REAL tree spans a newline', () => {
  // The property that made whole-file matching adoptable with zero waiver
  // churn, kept true rather than assumed. If a real engine line is ever wrapped
  // into a spanning match the clean-tree test goes red anyway — this one says
  // WHY, and it also documents the measurement the fix was justified by.
  const sources = listEngineSources(ENGINE_DIR);
  const sourceSet = new Set(sources);
  const spanning = [];
  for (const rel of sources) {
    const byRule = analyzeFile(ENGINE_DIR, rel, sourceSet);
    for (const [id, hits] of byRule) {
      for (const h of hits) {
        if (h.endLine !== h.startLine) spanning.push(`${rel}:${h.startLine}-${h.endLine} [${id}]`);
      }
    }
  }
  assertEqual(spanning, [], `matches spanning a newline in the real tree:\n${spanning.join('\n')}`);
});

test('matcher: a violation reports the line the match STARTS on, and the source of that line', () => {
  // A structural matcher that reports "somewhere in this file" is a matcher
  // nobody can act on. The offset must survive comment-stripping (which is
  // length-preserving precisely for this) and address the ORIGINAL file.
  const dir = makeScratchEngine();
  try {
    const target = path.join(dir, 'commands', 'orders.mjs');
    const before = readFileSync(target, 'utf8');
    const offending = "  return path.join(v, 'state.json');";
    writeFileSync(target, `${before}\nfunction dupA(v) {\n${offending}\n}\n`);
    // Ground truth read back off the file, not computed — an arithmetic pin
    // would only test the arithmetic.
    const lineNo = readFileSync(target, 'utf8').split('\n').indexOf(offending) + 1;
    assert(lineNo > 1, 'sanity: the planted line must be findable in the file');
    const v = scanEngineTree(dir).filter((x) => x.includes('[cursor-file]'));
    assertEqual(v.length, 1, `expected exactly one cursor-file violation, got ${JSON.stringify(v)}`);
    assertIncludes(v[0], `commands/orders.mjs:${lineNo}:`);
    assertIncludes(v[0], "| return path.join(v, 'state.json');");
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test('matcher: a match that spans lines is reported as a RANGE and is NOT waivable', () => {
  // The waiver mechanism describes single lines. A spanning match has no line
  // to pin, so it must be unwaivable — including when the waived line is the
  // one the match starts on. This is the property that stops "wrap the waived
  // line" from becoming the next free spelling.
  const v = violationsFor("function dupW(root) {\n  return path.join(root,\n    'skills',\n    'vibe');\n}\n");
  assert(
    v.some((x) => /commands\/orders\.mjs:\d+-\d+: \[vibe-layout\]/.test(x)),
    `a wrapped layout join must be reported as a line RANGE, got: ${JSON.stringify(v)}`,
  );
  assert(
    v.some((x) => x.includes('no single line to waive')),
    `the report must say why the match cannot be waived, got: ${JSON.stringify(v)}`,
  );
});

test('matcher: a waived line WRAPPED is no longer waived (the waiver is a line, not a shape)', () => {
  const dir = makeScratchEngine();
  try {
    const target = path.join(dir, 'commands', 'hook.mjs');
    const src = readFileSync(target, 'utf8');
    const waived = "return path.join(root, '.agents', 'skills', 'vibe');";
    assert(src.includes(waived), 'sanity: the waived hook.mjs layout line must still exist');
    writeFileSync(
      target,
      src.replace(waived, "return path.join(\n    root,\n    '.agents',\n    'skills',\n    'vibe',\n  );"),
    );
    const v = scanEngineTree(dir).filter((x) => x.startsWith('commands/hook.mjs:'));
    assert(
      v.some((x) => x.includes('[vibe-layout]')),
      `wrapping a waived line must un-waive it, got: ${JSON.stringify(v)}`,
    );
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test('module-reach: an out-of-set specifier is caught however the module is loaded', () => {
  // The loader is not part of the rule. These four are the syntaxes that exist
  // today; the rule never mentions any of them, which is the point.
  const loaders = [
    ["static import", "import { x as _x1 } from '../tests/helpers.mjs';\n"],
    ['export … from', "export { x as _x2 } from '../tests/helpers.mjs';\n"],
    ['dynamic import()', "const _p = import('../tests/helpers.mjs');\n"],
    ['require()', "const _r = require('../tests/helpers.js');\n"],
    ['a bare specifier held in a variable, loaded later', "const _spec = '../tests/helpers.mjs';\n"],
  ];
  for (const [label, code] of loaders) {
    const v = violationsFor(code);
    assert(
      v.some((x) => x.includes('[module-reach]')),
      `EVADED via ${label}: ${JSON.stringify(v)}`,
    );
  }
});

test('module-reach: an in-set specifier is NOT flagged (the rule is reachability, not "no relative imports")', () => {
  // The false-positive direction. Without this the rule could be "fixed" into a
  // blanket ban on relative specifiers and every test above would still pass.
  assertEqual(violationsFor("import { readJson as _rjx } from '../json.mjs';\n"), []);
  assertEqual(violationsFor("const _q = await import('./doctrine.mjs');\n"), []);
  assertEqual(violationsFor("import _fs2 from 'node:fs';\n"), []);
});

test('module-reach: the cli.mjs dispatch waiver is one LINE — a second dynamic specifier is a violation', () => {
  const v = violationsFor("const _other = await import(`./commands/${x}.mjs`);\n", 'cli.mjs');
  assert(
    v.some((x) => x.includes('[module-reach]')),
    `a second unresolvable specifier must not inherit cli.mjs's dispatch waiver, got: ${JSON.stringify(v)}`,
  );
});

// --- The KNOWN LIMITS, executable. Each of these asserts that something is
// NOT caught. They exist so the stated boundary is the ACTUAL boundary: if a
// later round widens a rule past one of these, the test goes red and the limit
// has to be rewritten instead of quietly outliving its truth.

test('KNOWN LIMIT 1: an ingredient split across two literals is not caught (needs constant folding)', () => {
  const v = violationsFor("const _f = 'state' + '.json';\nfunction dupL1(v2) { return path.join(v2, _f); }\n");
  assertEqual(
    v,
    [],
    'if this now FAILS the scan folds concatenation — excellent, but KNOWN LIMIT 1 above is stale and must be rewritten',
  );
});

test('KNOWN LIMIT 1: a unicode-escaped ingredient is not caught by the INGREDIENT rules', () => {
  const v = violationsFor("const _f2 = 'state\\u002ejson';\nfunction dupL1b(v2) { return path.join(v2, _f2); }\n");
  assertEqual(
    v,
    [],
    'if this now FAILS the ingredient rules cook escapes — rewrite KNOWN LIMIT 1',
  );
});

test('KNOWN LIMIT 2: module-reach DOES resolve escapes inside a single specifier literal', () => {
  // The positive half of limit 2, and the reason it is narrower than "no
  // constant folding" implies: the literal's own escapes are cooked, so a
  // unicode-escaped separator or a line continuation is not a spelling.
  const escaped = violationsFor("const _m1 = await import('..\\u002ftests/helpers.mjs');\n");
  assert(
    escaped.some((x) => x.includes('[module-reach]')),
    `a unicode-escaped separator must not hide a specifier, got: ${JSON.stringify(escaped)}`,
  );
  const continued = violationsFor("const _m2 = await import('../tests/hel\\\npers.mjs');\n");
  assert(
    continued.some((x) => x.includes('[module-reach]')),
    `a line continuation inside a specifier must not hide it, got: ${JSON.stringify(continued)}`,
  );
});

test('KNOWN LIMIT 2: a specifier whose every fragment is inert is not caught (needs constant folding)', () => {
  // The ACTUAL boundary, not the one it would be easy to claim. A partial split
  // is caught, because one fragment is still specifier-shaped:
  for (const partial of [
    "const _p1 = await import('../tests/' + 'helpers.mjs');\n",
    "const _p2 = await import('../tests/helpers' + '.mjs');\n",
    "const _p3 = await import(_base + '/helpers.mjs');\n",
  ]) {
    assert(
      violationsFor(partial).some((x) => x.includes('[module-reach]')),
      `a partially split specifier must still be caught: ${JSON.stringify(partial)}`,
    );
  }
  // Evading takes splitting EVERY fragment below the shape — the dot away from
  // the extension and the `..` away from its slash:
  const inert = violationsFor(
    "const _d = '.' + '.';\nconst _s = '/';\nconst _e = '.' + 'mjs';\nconst _m3 = await import(_d + _s + 'tests' + _s + 'helpers' + _e);\n",
  );
  assertEqual(
    inert,
    [],
    'if this now FAILS the reach rule folds concatenation — excellent, but KNOWN LIMIT 2 above is stale and must be rewritten',
  );
});

test('KNOWN LIMIT 5: an ascent split across statements is not caught, wrapped or not', () => {
  // Round 1 sanctioned doctor.mjs's rootForReport(), which is this shape. Both
  // spellings are stated as uncaught, so both are pinned as uncaught — the
  // wrapped one included, because "wrapped" is what round 3 just closed
  // everywhere else and the limit must not be assumed to have moved with it.
  assertEqual(violationsFor('function dupL5(p) {\n  const up = path.dirname(p);\n  return path.dirname(up);\n}\n'), []);
  assertEqual(
    violationsFor('function dupL5b(p) {\n  const up2 = path.dirname(\n    p,\n  );\n  return path.dirname(\n    up2,\n  );\n}\n'),
    [],
  );
});
