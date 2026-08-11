// engine/commands/doctrine.mjs — `vibe doctrine`, ported from
// flow/scripts/doctrine.sh (the oracle; read closely, never edited). Byte
// parity with the bash script is the acceptance test — see
// engine/tests/doctrine.test.mjs.
//
// Emits the vibe skill's `<!-- vibe:doctrine -->` block (marker lines
// excluded) verbatim, followed by a one-line `Cursor: <state>[
// (feature=<f>)].` summary. Read-only, always {code: 0} — a missing
// block/skill/SKILL.md degrades to no output, never a failure.
//
// KNOWN BUG — DELIBERATELY REPRODUCED, DO NOT FIX HERE: this is a
// SessionStart hook's output. Claude Code REPLAYS a SessionStart hook's
// saved output verbatim on `--resume` rather than re-running the hook, so
// the `Cursor:` line below is a snapshot from whenever the session first
// started and goes stale the moment the flow cursor moves in a resumed
// session. That is a real bug in the oracle, not a bash-vs-JS difference —
// reproducing it here is required for this unit's byte-parity acceptance
// test. The fix (stop appending live state to output that gets replayed;
// move it to something that re-runs every turn) belongs to a later
// feature, inject-triggers. Do not "helpfully" fix it in this file.
//
// CURSOR PRECEDENCE — settled here (carried forward from unit 2's review):
// the oracle hardcodes `$CLAUDE_PROJECT_DIR/.agents/skills/vibe/state.json`
// as its first-choice cursor location when that file exists, falling back
// to the skill-local state.json next to doctrine.sh itself otherwise. That
// is a rule specific to THIS command: a SessionStart hook wants the cursor
// for the CURRENT PROJECT, even in a per-user-plugin layout where the
// engine (and its own skill-local state.json) lives outside the project
// under `${CLAUDE_PLUGIN_ROOT}`. This is a DIFFERENT contract from
// resolveVibeDir() (root.mjs), which deliberately IGNORES
// CLAUDE_PROJECT_DIR once the installed self-relative chain validates —
// resolveVibeDir answers "where does this engine's own state live", which
// must stay stable no matter which project invoked it, and other commands
// (state, orders) rely on that stability. The two rules are allowed to
// disagree because they answer different questions. resolveVibeDir's own
// contract is left untouched; this command layers the oracle's
// CLAUDE_PROJECT_DIR-first rule on top, locally, only for the cursor it
// reads to print the summary line.
//
// Per repo convention: never re-derive vibeDir/skillsDir resolution logic,
// never parse cursor/machine JSON directly here — resolveVibeDir/
// resolveSkillsDir (root.mjs), readCursor (cursor.mjs), extractBlock
// (blocks.mjs) are the only primitives for that. The CLAUDE_PROJECT_DIR
// literal below is not a re-derivation of resolveVibeDir's self-relative
// logic — it is this command's own, narrower rule, matching the oracle's
// own hardcoded path.

import fs from 'node:fs';
import path from 'node:path';
import { resolveVibeDir, resolveSkillsDir } from '../root.mjs';
import { readCursor } from '../cursor.mjs';
import { extractBlock } from '../blocks.mjs';

// Mirrors bash `$(...)`, which strips ALL trailing newlines from a captured
// value before `printf '%s\n'` re-adds exactly one. extractBlock() itself
// never trims (other consumers may want the raw block); applied once here,
// at the call site analogous to the oracle's `DOCTRINE="$(extract_doctrine
// || true)"`. Same pattern as orders.mjs's identical helper.
function stripTrailingNewlines(s) {
  return s.replace(/\n+$/, '');
}

// The oracle's own CLAUDE_PROJECT_DIR-first cursor rule (see header) — a
// literal path exactly as doctrine.sh writes it, never derived via
// resolveVibeDir/resolveRoot.
function projectVibeDir() {
  const projectDir = process.env.CLAUDE_PROJECT_DIR;
  return projectDir ? path.join(projectDir, '.agents', 'skills', 'vibe') : undefined;
}

// Only redirect to the project's cursor when it actually exists, so a
// vendored/dogfood run (no CLAUDE_PROJECT_DIR, or a project with no cursor
// of its own yet) is byte-for-byte unchanged and falls through to the
// skill-local `vibeDir`.
function cursorDir(vibeDir) {
  const candidate = projectVibeDir();
  if (candidate && fs.existsSync(path.join(candidate, 'state.json'))) {
    return candidate;
  }
  return vibeDir;
}

// A present-but-corrupt cursor degrades to idle/no-feature here, matching
// the oracle's `jq -e .` gate: current_state()/current_feature() both go
// quiet on an unparseable file and default to idle. This is
// doctrine-specific — like orders.mjs's identical divergence from readers
// such as `vibe state get`, whose job is to surface a corrupt cursor, not
// paper over it.
function cursorStateAndFeature(dir) {
  try {
    const cursor = readCursor(dir);
    return { state: cursor.state, feature: cursor.feature ?? '' };
  } catch {
    return { state: 'idle', feature: '' };
  }
}

const SILENT = { code: 0, stdout: '', stderr: '' };

// Pure: takes the resolved vibeDir + skillsDir, returns {code, stdout,
// stderr} — NEVER throws, matching orders.mjs/state.mjs's contract, so
// unit 7's SessionStart hook shim can call it straight through. Guards
// non-string shapes for both arguments (review lesson from unit 4): a
// non-string skillsDir can't be joined into a path, so it degrades to
// silent success rather than throwing; a non-string vibeDir simply skips
// the cursor lookup and reports idle/no-feature, which is exactly what an
// absent cursor already means.
export function runDoctrine(vibeDir, skillsDir) {
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

  let stdout = `${doctrine}\n`;

  const { state, feature } =
    typeof vibeDir === 'string'
      ? cursorStateAndFeature(cursorDir(vibeDir))
      : { state: 'idle', feature: '' };

  // DELIBERATE BUG — see header. Do not fix here; inject-triggers owns it.
  stdout += feature ? `Cursor: ${state} (feature=${feature}).\n` : `Cursor: ${state}.\n`;

  return { code: 0, stdout, stderr: '' };
}

export default async function run(argv, opts = {}) {
  const vibeDir = resolveVibeDir(opts);
  const skillsDir = resolveSkillsDir(opts);

  const result = runDoctrine(vibeDir, skillsDir);

  if (result.stdout) process.stdout.write(result.stdout);
  if (result.stderr) process.stderr.write(result.stderr);
  return result.code;
}
