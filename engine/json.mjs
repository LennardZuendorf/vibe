// engine/json.mjs — readJson()/writeJsonAtomic(): the only JSON I/O.
//
// readJson lets fs/JSON errors propagate as-is (ENOENT for absent files,
// SyntaxError for malformed content) — callers with error-taxonomy needs
// (e.g. cursor.mjs's CursorParseError) wrap them; this module stays a thin,
// honest pass-through.

import fs from 'node:fs';
import path from 'node:path';

export function readJson(filePath) {
  const raw = fs.readFileSync(filePath, 'utf8');
  return JSON.parse(raw);
}

// Writes `data` (JSON.stringify'd, 2-space indent, trailing newline) to a
// sibling temp file in the target's own directory, then renames it over the
// target. Same-directory temp file keeps the rename on one filesystem, so it
// is atomic — matching the bash `mktemp "$FILE.XXXXXX"` + `mv -f` behaviour
// a crash mid-write can never truncate.
export function writeJsonAtomic(filePath, data) {
  const dir = path.dirname(filePath);
  const base = path.basename(filePath);
  const tmpPath = path.join(dir, `.${base}.${process.pid}.${Date.now()}.tmp`);
  const body = `${JSON.stringify(data, null, 2)}\n`;
  fs.writeFileSync(tmpPath, body, 'utf8');
  fs.renameSync(tmpPath, filePath);
}
