// engine/machine.mjs — loadMachine()/stateOf(): the only state-machine I/O.

import path from 'node:path';
import { readJson } from './json.mjs';

export function loadMachine(root) {
  const filePath = path.join(root, 'flow', 'state-machine.json');
  const raw = readJson(filePath);
  const { states, flows, phases, gates, initial, version, style } = raw;
  return { states, flows, phases, gates, initial, version, style };
}

export function stateOf(machine, key) {
  return machine && machine.states ? machine.states[key] : undefined;
}
