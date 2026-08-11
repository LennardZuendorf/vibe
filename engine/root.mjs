// engine/root.mjs — the self-relative/marker resolution family: resolveRoot(),
// resolveVibeDir(), resolveSkillsDir().
//
// Order for each: an explicit override -> CLAUDE_PROJECT_DIR (root only) ->
// self-relative (import.meta.url) -> upward .spec/.git marker search -> a
// final fallback. Self-relative deliberately precedes the marker search: a
// fresh install target has neither `.git` nor `.spec` (see the "stranger
// eval" lesson in .spec/lessons.md), so a resolver that only searched for
// markers silently broke there while looking fine in every privileged
// in-repo test.
//
// vibeDir and skillsDir are exposed here, not re-derived by their callers
// (cursor.mjs, machine.mjs, and later orders/doctrine commands), because
// they are variants of the exact same self-relative reasoning root
// resolution already needed: root.mjs is the one place that knows how an
// installed engine relates to its surrounding directories.

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

const MARKERS = ['.spec', '.git'];

function hasMarker(dir) {
  return MARKERS.some((marker) => fs.existsSync(path.join(dir, marker)));
}

// An installed engine lives at <ROOT>/.agents/skills/vibe/engine (see the
// hook shim in tech.md: VIBE_ENGINE defaults to that path) — engine/'s own
// parent IS the vibe skill dir (where state.json/state-machine.json live).
// Validate the full chain of directory names, not just "go up and hope" —
// that is what lets the dogfood repo (engine/ sitting at the repo top
// level, not nested under .agents/skills/vibe/) correctly fall through to
// the next strategy instead of resolving to some unrelated directory.
function selfRelativeVibeDir() {
  const vibeDir = path.dirname(__dirname);
  const skillsDir = path.dirname(vibeDir);
  const agentsDir = path.dirname(skillsDir);
  const isInstalledLayout =
    path.basename(vibeDir) === 'vibe' &&
    path.basename(skillsDir) === 'skills' &&
    path.basename(agentsDir) === '.agents';
  return isInstalledLayout ? vibeDir : undefined;
}

// The project root is three levels above the self-relative vibe dir
// (vibe -> skills -> .agents -> ROOT) when that chain validates.
function selfRelativeRoot() {
  const vibeDir = selfRelativeVibeDir();
  return vibeDir ? path.dirname(path.dirname(path.dirname(vibeDir))) : undefined;
}

function markerSearchRoot(startDir) {
  let dir = path.resolve(startDir);
  for (;;) {
    if (hasMarker(dir)) return dir;
    const parent = path.dirname(dir);
    if (parent === dir) return undefined; // hit the filesystem root
    dir = parent;
  }
}

export function resolveRoot(opts = {}) {
  const cwd = opts.cwd ?? process.cwd();

  const envRoot = process.env.CLAUDE_PROJECT_DIR;
  if (envRoot) return envRoot;

  const selfRelative = selfRelativeRoot();
  if (selfRelative) return selfRelative;

  const marker = markerSearchRoot(cwd);
  if (marker) return marker;

  return cwd;
}

// The vibe skill's own directory — where state.json and state-machine.json
// actually live. In an installed target this is engine/'s immediate parent.
// This source repo does not nest engine/ under a skill dir at all (it sits
// at the repo top level; flow/ is the skill dir, symlinked at
// .agents/skills/vibe), so the self-relative leg legitimately does not
// apply here and this falls through to root + the fixed
// `.agents/skills/vibe` suffix — which resolves correctly via that symlink.
export function resolveVibeDir(opts = {}) {
  if (opts.vibeDir) return opts.vibeDir;

  const selfRelative = selfRelativeVibeDir();
  if (selfRelative) return selfRelative;

  return path.join(resolveRoot(opts), '.agents', 'skills', 'vibe');
}

// The directory containing sibling skills (vibe, spec, ...). Mirrors
// orders.sh's SKILLS_DIR resolution exactly: probe whether vibeDir's parent
// itself looks like a skills directory (has a `vibe/SKILL.md` inside it —
// true for an installed target, where vibeDir IS already `.agents/skills/vibe`
// so its parent is `.agents/skills`); otherwise fall back to
// root + `.agents/skills`.
export function resolveSkillsDir(opts = {}) {
  if (opts.skillsDir) return opts.skillsDir;

  const vibeDir = resolveVibeDir(opts);
  const skillParent = path.dirname(vibeDir);
  if (fs.existsSync(path.join(skillParent, 'vibe', 'SKILL.md'))) {
    return skillParent;
  }

  return path.join(resolveRoot(opts), '.agents', 'skills');
}
