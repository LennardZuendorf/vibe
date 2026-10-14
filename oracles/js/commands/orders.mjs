// engine/commands/orders.mjs — `vibe orders`, ported from
// flow/scripts/orders.sh (the oracle; read closely, never edited). Byte
// parity with the bash script is the acceptance test — see
// engine/tests/orders.test.mjs.
//
// D12 resolution chain (mirrors the oracle exactly): cursor -> the state's
// `skill` link in the machine -> the `<!-- vibe:orders:<state> -->` block in
// that skill's SKILL.md -> the machine's inline `inject` string (idle's only
// carrier) -> a hardcoded generic one-liner. `<feature>` is the only
// interpolation. Always exits 0 — read-only, never fails.
//
// The oracle's no-jq degrade path hardcodes "vibe" as the skill for any
// non-idle state and sed-grabs the first quoted "inject" string in the
// machine file — both documented shortcuts that happen to hold for THIS
// repo's current machine.json but are not general. Per the task brief, the
// jq path is authoritative: this port always does the real field lookups
// (stateOf/loadMachine), never those shortcuts.
//
// Per repo convention: never re-derive vibeDir/skillsDir, never parse
// cursor/machine JSON directly here — resolveVibeDir/resolveSkillsDir
// (root.mjs), readCursor (cursor.mjs), loadMachine/stateOf (machine.mjs),
// extractBlock (blocks.mjs) are the only primitives for that.

import fs from 'node:fs';
import path from 'node:path';
import { resolveVibeDir, resolveSkillsDir } from '../root.mjs';
import { readCursor } from '../cursor.mjs';
import { loadMachine, stateOf } from '../machine.mjs';
import { extractBlock } from '../blocks.mjs';

const GENERIC_FALLBACK =
  'state=unknown · read .agents/skills/vibe/state-machine.json and pick the matching vibe phase · transition via set-state.sh';

function line(s) {
  return `${s}\n`;
}

// Mirrors bash `$(...)` command substitution, which strips ALL trailing
// newlines from a captured value before it is ever assigned. Both
// extract_block()'s and machine_inject()'s bash equivalents are captured via
// `$(...)` before interpolate()/printf run, so a block whose content ends in
// a blank line (or an inject string with a stray trailing newline) loses
// that trailing whitespace on the oracle side. extractBlock() itself must
// not do this trimming (other consumers may want the raw block), so it is
// applied here, once, on the two call sites that are analogous to a bash
// command substitution.
function stripTrailingNewlines(s) {
  return s.replace(/\n+$/, '');
}

// Cursor -> {state, feature}. An ABSENT cursor legitimately means idle
// (readCursor's own documented contract). A PRESENT-but-corrupt cursor also
// degrades to idle/empty-feature here — this is orders-specific, matching
// the oracle's `jq -e .` gate: an unparseable state.json fails that check,
// so current_state()'s flow/phase stay unset and default to "idle", and the
// unguarded `jq -r '.feature // empty' 2>/dev/null || true` on the same bad
// file also yields empty. This deliberately diverges from readers like
// `vibe state get`, whose whole job is to surface a corrupt cursor as an
// error rather than paper over it — orders.sh itself papers over it, so the
// port must too, to stay byte-identical.
function cursorStateAndFeature(vibeDir) {
  try {
    const cursor = readCursor(vibeDir);
    return { state: cursor.state, feature: cursor.feature ?? null };
  } catch {
    return { state: 'idle', feature: null };
  }
}

// Mirrors jq's `// empty` + `-r` for an arbitrary JSON value (review round
// 1, Finding 4, shared with doctrine.mjs's identical helper): jq's
// alternative operator treats EVERYTHING except `false` and `null` as
// truthy — including `0` and `""` — which a plain JS `feature ? ... : ...`
// gets wrong for `0` (JS-falsy, jq-truthy: the oracle's `current_feature()`
// captures jq's raw "0" and `[[ -n "0" ]]` is true, so it DOES interpolate).
// `-r` prints a raw string as-is; anything else (numbers, booleans,
// objects, arrays) prints as normal jq JSON output, which for non-scalars
// is PRETTY (2-space indent) by default, not compact — reproduced with
// `JSON.stringify(value, null, 2)`.
//
// SCOPE — what this does NOT reproduce (js-core/5 re-review; the earlier
// claim of exact jq reproduction was an overclaim). It reproduces jq's `//`
// truthiness and its 2-space pretty-print, not the oracle's full capture
// pipeline. JSON.stringify RE-SERIALIZES numbers, so `1.0` prints as `1`,
// `1e2` as `100`, `-0` as `0`, and integers beyond 2^53 lose precision, where
// jq's own number handling differs; integer-like OBJECT KEYS are reordered to
// the front by JS property ordering, where jq preserves document order; and a
// string `feature` ending in newlines keeps them here, where bash `$(...)`
// would strip them. All of these are pre-existing or improved relative to the
// pre-fix behaviour, and all are reachable only via a hand-edited cursor
// (readCursor/writeCursor never produce a non-string/non-null feature) — but
// they are divergences, not reproductions, and should be named as such.
function jqAltRaw(value) {
  if (value === null || value === undefined || value === false) return '';
  return typeof value === 'string' ? value : JSON.stringify(value, null, 2);
}

// <feature> is the only interpolation; a feature-less cursor leaves the
// literal placeholder in place (still valid guidance, still byte-stable).
function interpolate(text, feature) {
  const featureText = jqAltRaw(feature);
  return featureText ? text.split('<feature>').join(featureText) : text;
}

function machineSkill(machine, stateKey) {
  const state = stateOf(machine, stateKey);
  return state && typeof state.skill === 'string' && state.skill ? state.skill : '';
}

function machineInject(machine, stateKey) {
  const state = stateOf(machine, stateKey);
  return state && typeof state.inject === 'string' && state.inject ? state.inject : '';
}

// Pure: takes the resolved vibeDir + skillsDir and raw CLI args, returns
// {code, stdout, stderr} — NEVER throws, so unit 7's hook shims can call it
// straight through, matching state.mjs's runSet/runGet contract. That
// contract holds for any argument shape, not just the well-formed ones: a
// non-string skillsDir (undefined, a nonexistent path) and a non-array args
// (null, a plain default only covers `undefined`) must degrade, never throw
// — review round 1, Finding 3.
export function runOrders(vibeDir, skillsDir, args) {
  const safeArgs = Array.isArray(args) ? args : [];
  const { state: cursorState, feature } = cursorStateAndFeature(vibeDir);
  const stateKey = safeArgs[0] || cursorState;

  let machine;
  try {
    machine = loadMachine(vibeDir);
  } catch {
    // Missing/corrupt machine — degrade straight to the generic fallback,
    // matching the oracle's `[[ -f "$MACHINE" ]] || return 1` guard shared
    // by both machine_skill() and machine_inject().
    machine = undefined;
  }

  if (machine) {
    // 1. Prefer the linked skill's orders block (D12).
    const skill = machineSkill(machine, stateKey);
    if (skill && typeof skillsDir === 'string') {
      const skillFile = path.join(skillsDir, skill, 'SKILL.md');
      let text;
      try {
        text = fs.readFileSync(skillFile, 'utf8');
      } catch {
        text = undefined; // missing skill file — fall through, never fail
      }
      if (text !== undefined) {
        const rawBlock = extractBlock(text, `vibe:orders:${stateKey}`);
        if (rawBlock !== undefined) {
          // Command-substitution parity (Finding 2): strip trailing
          // newlines before the truthiness check AND before interpolating,
          // so a block ending in a blank line collapses exactly like the
          // oracle's `BLOCK="$(extract_block ...)"` does.
          const block = stripTrailingNewlines(rawBlock);
          if (block) {
            return { code: 0, stdout: line(interpolate(block, feature)), stderr: '' };
          }
        }
      }
    }

    // 2. Fall back to the machine's inline inject (idle, or a skill with no
    // block yet).
    const inline = stripTrailingNewlines(machineInject(machine, stateKey));
    if (inline) {
      return { code: 0, stdout: line(interpolate(inline, feature)), stderr: '' };
    }
  }

  // 3. Last resort: a generic one-liner. Never fail.
  return { code: 0, stdout: line(GENERIC_FALLBACK), stderr: '' };
}

export default async function run(argv, opts = {}) {
  const vibeDir = resolveVibeDir(opts);
  const skillsDir = resolveSkillsDir(opts);

  const result = runOrders(vibeDir, skillsDir, argv);

  if (result.stdout) process.stdout.write(result.stdout);
  if (result.stderr) process.stderr.write(result.stderr);
  return result.code;
}
