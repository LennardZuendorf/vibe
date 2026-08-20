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

function errMsg(err) {
  return err && err.message ? err.message : String(err);
}

// Pure: takes the vibeDir and raw `set` args, returns {code, stdout,
// stderr} — NEVER throws, so it is directly unit-testable without spawning
// a process and safe for unit 7's hook shims to call straight through
// without wrapping it in their own try/catch. Every failure path below
// (missing/corrupt machine, corrupt cursor, a failed write) converts to a
// returned result instead of propagating.
//
// "Never throws" has to survive the ARGUMENTS too, or the claim is only about
// the happy shape: destructuring `args` threw a TypeError on null and on any
// non-iterable before a single failure path could run, so a shim calling
// straight through got the crash the comment promised it would not. Anything
// that is not an array is no arguments at all, and a target that is not a
// string is no target — which also keeps splitTarget() (string-only) off a
// non-string. Pinned in state.test.mjs.
export function runSet(vibeDir, args) {
  const list = Array.isArray(args) ? args : [];
  const [rawTarget, newFeature] = list;
  const target = typeof rawTarget === 'string' ? rawTarget : '';

  if (!target) {
    return {
      code: 1,
      stdout: '',
      stderr: line(
        'vibe state: ERROR: no target state given. Usage: vibe state set <flow.phase> [feature]',
      ),
    };
  }

  let machine;
  try {
    machine = loadMachine(vibeDir);
  } catch (err) {
    return {
      code: 1,
      stdout: '',
      stderr: line(`vibe state: ERROR: cannot read state machine: ${errMsg(err)}`),
    };
  }

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
  // cursor's feature is preserved.
  //
  // Writer-only degrade (review round 1, Finding 2): readCursor()'s
  // throwing contract is correct for READERS (a malformed cursor is a real
  // problem `get` must surface) but wrong here — the writer only needs the
  // previous `feature` value, and `vibe state set idle` is the only CLI
  // path that can ever fix a corrupt cursor. The oracle's own jq path
  // already recovers silently (`jq -r '.feature // "null"' 2>/dev/null ||
  // echo "null"`) and goes on to write a fresh, valid cursor; degrading to
  // null here and proceeding matches that, instead of bricking the one
  // recovery path bash has.
  let curFeature = null;
  try {
    curFeature = readCursor(vibeDir).feature;
  } catch {
    curFeature = null;
  }

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

  try {
    writeCursor(vibeDir, { flow, phase, feature });
  } catch (err) {
    return {
      code: 1,
      stdout: '',
      stderr: line(`vibe state: ERROR: failed to write cursor: ${errMsg(err)}`),
    };
  }

  let stdout = line(`-> ${target}`);
  const next = stateOf(machine, target).next;
  if (Array.isArray(next) && next.length > 0) {
    stdout += line(`   next: ${next.join(', ')}`);
  }

  return { code: 0, stdout, stderr };
}

// No bash oracle — see header. Prints the resolved cursor as pretty JSON.
// Unlike `set`, a corrupt cursor here is NOT degraded: `get`'s only job is
// reporting what the cursor says, so silently answering "idle" for a
// present-but-malformed file would be a wrong answer, not a recovery.
// Still never throws — converts to a {code:1,...} result like `set` does.
export function runGet(vibeDir) {
  try {
    const cursor = readCursor(vibeDir);
    return { code: 0, stdout: `${JSON.stringify(cursor, null, 2)}\n`, stderr: '' };
  } catch (err) {
    return { code: 1, stdout: '', stderr: line(`vibe state: ERROR: ${errMsg(err)}`) };
  }
}

export default async function run(argv, opts = {}) {
  // Same reason as runSet's: this destructuring sits OUTSIDE the try below, so
  // a non-array argv threw past the named-error result instead of producing it.
  const [sub, ...rest] = Array.isArray(argv) ? argv : [];
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
