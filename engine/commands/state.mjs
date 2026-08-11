// engine/commands/state.mjs — `vibe state get|set`, ported from
// flow/scripts/set-state.sh (the oracle for `set`; read closely, never
// edited). Byte parity with the bash writer is the acceptance test for
// `set` — see engine/tests/state.test.mjs.
//
// Scope: writer only, per js-core/3. `set` validates only that the target
// state NAME exists in the machine, exactly like the oracle — it does not
// check `next` membership and does not require a confirm token on gated
// edges. Gate enforcement is machine-teeth's job; adding it here would
// silently diverge from the oracle, which does not gate either.
//
// `get` has no bash equivalent (set-state.sh is a writer, not a reader) —
// there is nothing to reproduce byte-for-byte. It prints the resolved
// cursor (the exact shape readCursor() already returns) as pretty JSON, so
// scripts/tests get the same {flow, phase, feature, updated, state} this
// module itself works from.
//
// Per repo convention: never re-derive vibeDir, never parse cursor/machine
// JSON directly here — resolveVibeDir/readCursor/writeCursor/loadMachine
// (unit 2) are the only primitives for that.

import { resolveVibeDir } from '../root.mjs';
import { readCursor, writeCursor } from '../cursor.mjs';
import { loadMachine, stateOf } from '../machine.mjs';

// Splits "<flow>.<phase>" on the FIRST dot only, matching the oracle's
// `${TARGET%%.*}` / `${TARGET#*.}`. A single-token target (e.g. "idle")
// collapses flow===phase, same as the oracle's else-branch.
function splitTarget(target) {
  const dot = target.indexOf('.');
  if (dot === -1) return { flow: target, phase: target };
  return { flow: target.slice(0, dot), phase: target.slice(dot + 1) };
}

function line(s) {
  return `${s}\n`;
}

// Pure: takes the vibeDir and raw `set` args, returns {code, stdout,
// stderr} — never touches process.std{out,err} itself, so it is directly
// unit-testable without spawning a process.
export function runSet(vibeDir, args) {
  const [target, newFeature] = args;

  if (!target) {
    return {
      code: 1,
      stdout: '',
      stderr: line(
        'vibe state: ERROR: no target state given. Usage: vibe state set <flow.phase> [feature]',
      ),
    };
  }

  const machine = loadMachine(vibeDir);
  if (!stateOf(machine, target)) {
    const legal = Object.keys(machine.states ?? {}).join(', ');
    return {
      code: 1,
      stdout: '',
      stderr:
        line(`vibe state: ERROR: '${target}' is not a known state.`) +
        line(`vibe state: Known states: ${legal}`),
    };
  }

  const { flow, phase } = splitTarget(target);

  // Feature carry-forward — mirrors the oracle's precedence exactly: a new
  // feature argument wins; moving to idle clears it; otherwise the current
  // cursor's feature is preserved (readCursor already normalizes an absent
  // cursor's feature to null, matching the oracle's `// "null"` default).
  const curFeature = readCursor(vibeDir).feature;
  let feature;
  if (newFeature) {
    feature = newFeature;
  } else if (flow === 'idle') {
    feature = null;
  } else if (curFeature !== null && curFeature !== undefined && curFeature !== '') {
    feature = curFeature;
  } else {
    feature = null;
  }

  let stderr = '';
  if (flow === 'feature' && feature === null) {
    stderr += line(
      `vibe state: WARN: entering '${target}' with no feature set. Pass one: vibe state set ${target} <feature>`,
    );
  }

  writeCursor(vibeDir, { flow, phase, feature });

  let stdout = line(`-> ${target}`);
  const next = stateOf(machine, target).next;
  if (Array.isArray(next) && next.length > 0) {
    stdout += line(`   next: ${next.join(', ')}`);
  }

  return { code: 0, stdout, stderr };
}

// No bash oracle — see header. Prints the resolved cursor as pretty JSON.
export function runGet(vibeDir) {
  const cursor = readCursor(vibeDir);
  return { code: 0, stdout: `${JSON.stringify(cursor, null, 2)}\n`, stderr: '' };
}

export default async function run(argv, opts = {}) {
  const [sub, ...rest] = argv;
  const vibeDir = resolveVibeDir(opts);

  let result;
  try {
    if (sub === 'get') {
      result = runGet(vibeDir);
    } else if (sub === 'set') {
      result = runSet(vibeDir, rest);
    } else {
      result = {
        code: 1,
        stdout: '',
        stderr: line(
          `vibe state: ERROR: unknown subcommand '${sub ?? ''}'. Usage: vibe state <get|set> ...`,
        ),
      };
    }
  } catch (err) {
    result = {
      code: 1,
      stdout: '',
      stderr: line(`vibe state: ERROR: ${err && err.message ? err.message : err}`),
    };
  }

  if (result.stdout) process.stdout.write(result.stdout);
  if (result.stderr) process.stderr.write(result.stderr);
  return result.code;
}
