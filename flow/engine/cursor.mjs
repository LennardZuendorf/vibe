// engine/cursor.mjs — readCursor()/writeCursor(): the only cursor JSON I/O.
//
// Both take the resolved flow data directory. Once a project has a migrated
// .vibe runtime cursor, all reads and writes prefer it. Legacy-only targets
// keep their existing cursor until the Rust writer migrates them; old bytes
// remain available as an absence-only fallback.
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
import fs from 'node:fs';
import { resolveCursorRoot, flowRuntimeDir, assertCursorWithinRoot, cursorEntryAbsent } from './root.mjs';
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
export function cursorPath(vibeDir, rootOverride) {
  const root = rootOverride ?? resolveCursorRoot(vibeDir);
  const current = root && path.join(flowRuntimeDir(root), 'state.json');
  if (current) {
    try { if (!cursorEntryAbsent(current)) return current; }
    catch { return current; } // Invalid new layout remains authoritative.
  }
  return path.join(vibeDir, 'state.json');
}

function stateKey(flow, phase) {
  return flow === phase ? flow : `${flow}.${phase}`;
}

export function readCursor(vibeDir, rootOverride) {
  const filePath = cursorPath(vibeDir, rootOverride);

  let raw;
  try {
    if (cursorEntryAbsent(filePath)) {
      return { flow: 'idle', phase: 'idle', feature: null, updated: null, state: 'idle' };
    }
    raw = readJson(filePath);
  } catch (err) {
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

// validFeature(value) — the feature charset both cursor writers accept, the twin
// of set-state.sh's valid_feature: a non-empty string with no C0 control byte,
// no DEL, no double quote and no backslash.
export function validFeature(value) {
  return typeof value === 'string' && value !== '' && !/[\u0000-\u001f\u007f"\\]/.test(value);
}

// Atomic; preserves the cursor's shape (key order flow, phase, feature,
// updated) so byte parity with the bash writer's hand-rolled printf holds.
function lockedCursor(vibeDir, operation) {
  const root = resolveCursorRoot(vibeDir);
  const lockDir = root ? flowRuntimeDir(root) : vibeDir;
  assertCursorWithinRoot(root, lockDir);
  fs.mkdirSync(lockDir, { recursive: true });
  const lockPath = path.join(lockDir, 'runtime.lock');
  const fd = fs.openSync(lockPath, 'wx', 0o600);
  const owned = fs.fstatSync(fd);
  try {
    if (fs.existsSync(path.join(lockDir, 'runs'))) {
      throw new Error('adopted runtime requires the Rust Flow runtime; legacy writer refused');
    }
    // Re-resolve only after acquiring the same exclusive file as Rust upgrade.
    const filePath = cursorPath(vibeDir);
    assertCursorWithinRoot(root, filePath);
    let current;
    try { current = readJson(filePath); } catch (err) {
      if (err.code !== 'ENOENT' && !(err instanceof SyntaxError)) throw err;
    }
    if (current && Object.hasOwn(current, 'v')) {
      throw new Error('versioned cursor requires the Rust Flow runtime; legacy writer refused');
    }
    return operation(filePath);
  } finally {
    fs.closeSync(fd);
    const current = fs.lstatSync(lockPath, { throwIfNoEntry: false });
    if (current && current.dev === owned.dev && current.ino === owned.ino) fs.unlinkSync(lockPath);
  }
}

// The selector reads carried state under the same lock as the final write.
export function mutateCursor(vibeDir, select) {
  return lockedCursor(vibeDir, filePath => {
    const { flow, phase, feature = null } = select();
    const updated = new Date().toISOString().replace(/\.\d{3}Z$/, 'Z');
    const body = { flow, phase, feature: feature ?? null, updated };
    writeJsonAtomic(filePath, body);
    return { ...body, state: stateKey(flow, phase) };
  });
}

export function writeCursor(vibeDir, body) {
  return mutateCursor(vibeDir, () => body);
}

// Legacy adapters cannot evaluate upgraded readiness or grant semantics.
export function legacyAdapterUnavailable(vibeDir, rootOverride) {
  const root = rootOverride ?? resolveCursorRoot(vibeDir);
  if (root) {
    try { fs.lstatSync(path.join(flowRuntimeDir(root), 'runs')); return true; }
    catch (err) { if (err.code !== 'ENOENT') return true; }
  }
  try {
    const raw = readJson(cursorPath(vibeDir, rootOverride));
    return raw !== null && typeof raw === 'object' && Object.hasOwn(raw, 'v');
  } catch { return false; }
}
