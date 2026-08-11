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
    return { state: cursor.state, feature: cursor.feature ?? '' };
  } catch {
    return { state: 'idle', feature: '' };
  }
}

// <feature> is the only interpolation; a feature-less cursor leaves the
// literal placeholder in place (still valid guidance, still byte-stable).
function interpolate(text, feature) {
  return feature ? text.split('<feature>').join(feature) : text;
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
// straight through, matching state.mjs's runSet/runGet contract.
export function runOrders(vibeDir, skillsDir, args = []) {
  const { state: cursorState, feature } = cursorStateAndFeature(vibeDir);
  const stateKey = args[0] || cursorState;

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
    if (skill) {
      const skillFile = path.join(skillsDir, skill, 'SKILL.md');
      let text;
      try {
        text = fs.readFileSync(skillFile, 'utf8');
      } catch {
        text = undefined; // missing skill file — fall through, never fail
      }
      if (text !== undefined) {
        const block = extractBlock(text, `vibe:orders:${stateKey}`);
        if (block) {
          return { code: 0, stdout: line(interpolate(block, feature)), stderr: '' };
        }
      }
    }

    // 2. Fall back to the machine's inline inject (idle, or a skill with no
    // block yet).
    const inline = machineInject(machine, stateKey);
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
