// engine/machine.mjs — loadMachine()/stateOf(): the only state-machine I/O.
//
// loadMachine takes `vibeDir` — the vibe skill's own directory, where
// state-machine.json lives — not a project root. See cursor.mjs's header
// for why: callers resolve it once via `resolveVibeDir()` in root.mjs and
// pass it in explicitly, so resolution logic lives in exactly one place.

import path from 'node:path';
import { readJson } from './json.mjs';

export function loadMachine(vibeDir) {
  const filePath = path.join(vibeDir, 'state-machine.json');
  const raw = readJson(filePath);
  const { states, flows, phases, gates, initial, version, style } = raw;
  return { states, flows, phases, gates, initial, version, style };
}

export function stateOf(machine, key) {
  if (!machine || !machine.states) return undefined;
  // Own-property check only — `key` is untrusted CLI input, and a bare `[]`
  // lookup resolves inherited Object.prototype members (constructor,
  // toString, valueOf, hasOwnProperty, __proto__, ...) as if they were real
  // states. Every consumer (stateOf is the ONLY state lookup primitive)
  // inherits this fix; do not re-check in callers.
  return Object.prototype.hasOwnProperty.call(machine.states, key)
    ? machine.states[key]
    : undefined;
}
