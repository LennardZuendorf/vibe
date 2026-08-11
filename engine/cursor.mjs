// engine/cursor.mjs — readCursor()/writeCursor(): the only cursor JSON I/O.
//
// readCursor(root):
//   - absent file            -> {state: "idle", flow: "idle", phase: "idle",
//                                 feature: null, updated: null}
//   - present, valid object  -> per-field defaults mirror the bash readers'
//                                `// "idle"` / `// empty` fallbacks
//   - present, does not parse (or is not a JSON object) -> throws
//                                CursorParseError, never silently "idle"
//
// This last case is the fix for a real bash bug: all five bash cursor
// readers treat an unparseable state.json exactly like an absent one (jq -e
// . fails -> flow/phase stay empty -> both default to "idle"), so a
// corrupted cursor reads as a healthy idle flow instead of surfacing. Only
// an *absent* cursor may mean idle.

import path from 'node:path';
import { readJson, writeJsonAtomic } from './json.mjs';

export class CursorParseError extends Error {
  constructor(filePath, cause) {
    const reason = cause && cause.message ? cause.message : String(cause);
    super(`cursor at ${filePath} is malformed: ${reason}`);
    this.name = 'CursorParseError';
    this.filePath = filePath;
    this.cause = cause;
  }
}

function cursorPath(root) {
  return path.join(root, 'flow', 'state.json');
}

function stateKey(flow, phase) {
  return flow === phase ? flow : `${flow}.${phase}`;
}

export function readCursor(root) {
  const filePath = cursorPath(root);

  let raw;
  try {
    raw = readJson(filePath);
  } catch (err) {
    if (err && err.code === 'ENOENT') {
      return { flow: 'idle', phase: 'idle', feature: null, updated: null, state: 'idle' };
    }
    // Present but unreadable-as-JSON (parse error, permissions error other
    // than ENOENT, etc.) — a real malformed cursor. Throw, don't guess idle.
    throw new CursorParseError(filePath, err);
  }

  if (raw === null || typeof raw !== 'object' || Array.isArray(raw)) {
    throw new CursorParseError(filePath, new Error('top-level JSON value is not an object'));
  }

  const flow = typeof raw.flow === 'string' && raw.flow ? raw.flow : 'idle';
  const phase = typeof raw.phase === 'string' && raw.phase ? raw.phase : 'idle';
  const feature = raw.feature ?? null;
  const updated = raw.updated ?? null;

  return { flow, phase, feature, updated, state: stateKey(flow, phase) };
}

// Atomic; preserves the cursor's shape (key order flow, phase, feature,
// updated) so byte parity with the bash writer's hand-rolled printf holds.
export function writeCursor(root, { flow, phase, feature = null }) {
  const filePath = cursorPath(root);
  const updated = new Date().toISOString().replace(/\.\d{3}Z$/, 'Z');
  const body = { flow, phase, feature: feature ?? null, updated };
  writeJsonAtomic(filePath, body);
  return { ...body, state: stateKey(flow, phase) };
}
