// engine/commands/doctrine.mjs — `vibe doctrine`, ported from
// flow/scripts/doctrine.sh (the oracle). Byte parity with the bash script is
// the acceptance test — see engine/tests/doctrine.test.mjs.
//
// Emits the vibe skill's `<!-- vibe:doctrine -->` block (marker lines
// excluded) verbatim, and nothing else. Read-only, always {code: 0} — a
// missing block/skill/SKILL.md degrades to no output, never a failure.
//
// NO LIVE STATE RIDES THIS OUTPUT (inject-triggers, R4). Until this unit the
// command appended a one-line `Cursor: <state>[ (feature=<f>)].` summary. This
// is a SessionStart hook's output, and Claude Code REPLAYS a SessionStart
// hook's saved output verbatim on `--resume` rather than re-running the hook,
// so that line was a snapshot from whenever the session first started and went
// stale the moment the flow cursor moved in a resumed session. The cursor is
// now carried by the per-turn `user-prompt.level` channel, which re-runs every
// turn and names the state anyway — so the line was both stale AND duplicated.
// It is deleted here and in the oracle (flow/scripts/doctrine.sh) TOGETHER, so
// the parity matrix still compares two implementations of the same contract.
//
// Consequences of that deletion, stated because they are what shrank this
// file: no cursor is read, so `vibeDir`, resolveProjectCursorDir()'s
// CLAUDE_PROJECT_DIR-first precedence rule, readCursor(), and the jq `//`
// truthiness reproduction for a non-string `feature` are all gone with it.
// resolveProjectCursorDir() itself stays in root.mjs (it is that module's own
// documented contract); this command simply no longer has a cursor to resolve.
//
// Per repo convention: never re-derive skillsDir resolution logic —
// resolveSkillsDir (root.mjs) and extractBlock (blocks.mjs) are the only
// primitives this command needs.

import fs from 'node:fs';
import path from 'node:path';
import { resolveSkillsDir } from '../root.mjs';
import { extractBlock } from '../blocks.mjs';

// Mirrors bash `$(...)`, which strips ALL trailing newlines from a captured
// value before `printf '%s\n'` re-adds exactly one. extractBlock() itself
// never trims (other consumers may want the raw block); applied once here,
// at the call site analogous to the oracle's `DOCTRINE="$(extract_doctrine
// || true)"`. Same pattern as orders.mjs's identical helper.
function stripTrailingNewlines(s) {
  return s.replace(/\n+$/, '');
}

const SILENT = { code: 0, stdout: '', stderr: '' };

// Pure: takes the resolved skillsDir, returns {code, stdout, stderr} — NEVER
// throws, matching orders.mjs/state.mjs's contract. Guards a non-string
// skillsDir (review lesson from unit 4): it cannot be joined into a path, so
// it degrades to silent success rather than throwing.
export function runDoctrine(skillsDir) {
  if (typeof skillsDir !== 'string') return SILENT;

  const skillMdPath = path.join(skillsDir, 'vibe', 'SKILL.md');
  let text;
  try {
    text = fs.readFileSync(skillMdPath, 'utf8');
  } catch {
    return SILENT; // missing skill / SKILL.md — degrade, never fail
  }

  const rawBlock = extractBlock(text, 'vibe:doctrine');
  if (rawBlock === undefined) return SILENT; // no opener, or opener with no closer before EOF

  const doctrine = stripTrailingNewlines(rawBlock);
  if (!doctrine) return SILENT; // empty block — silent exit 0, matches the oracle

  return { code: 0, stdout: `${doctrine}\n`, stderr: '' };
}

export default async function run(argv, opts = {}) {
  const result = runDoctrine(resolveSkillsDir(opts));

  if (result.stdout) process.stdout.write(result.stdout);
  if (result.stderr) process.stderr.write(result.stderr);
  return result.code;
}
