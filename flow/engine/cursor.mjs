// engine/cursor.mjs — readCursor()/writeCursor(): the only cursor JSON I/O.
//
// Both take `vibeDir` — the vibe skill's own directory, where state.json
// lives — not a project root. Callers resolve that directory once via
// `resolveVibeDir()` in root.mjs (which knows the self-relative/
// installed-vs-dogfood layout distinction) and pass it in explicitly; this
// module never re-derives it, so there is exactly one place that owns that
// resolution (the duplicate-primitive scan's whole point). Tests pass
// whatever directory their fixture uses (e.g. sandbox.flowDir) directly.
//
// readCursor(vibeDir):
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

// The one place `state.json`'s path is joined. Exported for the same reason
// machine.mjs exports machinePath(): a caller that needs only the PATH — the
// existence check in doctor.mjs's checkCursor() — calls this instead of
// re-deriving the join, and so never has to name the cursor file itself.
//
// Exporting it does NOT make it a free spelling of the file. On its own it
// would have been the opposite: `readJson(cursorPath(vibeDir))` is a complete
// duplicate cursor reader that names no banned literal, and it walked through
// the scan when this export was introduced (js-core/8 fix round 1, re-review
// Finding 1). So the scan bans the identifier `cursorPath` outside this module
// too, with a counted, per-line consumer allowlist — doctor.mjs's import line
// and its one use, and nothing else. A second use anywhere, including inside
// doctor.mjs, and any re-export under another name, are violations.
// root.mjs's resolveProjectCursorDir() deliberately does NOT call this (it
// must stay importable without dragging json.mjs in); it carries its own
// one-line waiver for the literal instead.
export function cursorPath(vibeDir) {
  return path.join(vibeDir, 'state.json');
}

function stateKey(flow, phase) {
  return flow === phase ? flow : `${flow}.${phase}`;
}

export function readCursor(vibeDir) {
  const filePath = cursorPath(vibeDir);

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
export function writeCursor(vibeDir, { flow, phase, feature = null }) {
  const filePath = cursorPath(vibeDir);
  const updated = new Date().toISOString().replace(/\.\d{3}Z$/, 'Z');
  const body = { flow, phase, feature: feature ?? null, updated };
  writeJsonAtomic(filePath, body);
  return { ...body, state: stateKey(flow, phase) };
}
