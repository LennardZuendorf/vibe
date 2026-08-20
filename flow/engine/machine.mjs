// engine/machine.mjs — loadMachine()/stateOf()/machinePath(): the only
// state-machine I/O and path.
//
// loadMachine takes `vibeDir` — the vibe skill's own directory, where
// state-machine.json lives — not a project root. See cursor.mjs's header
// for why: callers resolve it once via `resolveVibeDir()` in root.mjs and
// pass it in explicitly, so resolution logic lives in exactly one place.

import path from 'node:path';
import { readJson } from './json.mjs';

// The one place `state-machine.json`'s path is joined. A caller that needs
// only the PATH (e.g. doctor.mjs's own existence/regular-file check, which
// must distinguish "missing" from "present but unparseable" before it knows
// whether reading is even worth attempting) calls this instead of
// re-deriving `path.join(vibeDir, 'state-machine.json')` itself — mirrors
// root.mjs's resolveProjectCursorDir() single-sourcing its own layout
// constant for doctrine.mjs (js-core/6 review round 2, Finding: machine
// re-derivation).
export function machinePath(vibeDir) {
  return path.join(vibeDir, 'state-machine.json');
}

export function loadMachine(vibeDir) {
  const filePath = machinePath(vibeDir);
  const raw = readJson(filePath);
  // Every consumer's bash counterpart gates on jq's `-e`, which fails not
  // only on invalid JSON but also on a top-level `null` or `false`
  // document. `null` already throws below via the destructure ("Cannot
  // destructure property of null"); `false` would NOT (primitives
  // auto-box, so `const {x} = false` silently gives `x: undefined`) without
  // this explicit check. Reproduced once, here, in the primitive that owns
  // the read — not re-derived per caller (js-core/6 review round 2).
  if (raw === null || raw === false) {
    throw new TypeError(`state-machine.json at ${filePath} is not a valid machine document (top-level ${raw})`);
  }
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
