// engine/root.mjs — resolveRoot(): the one project-root resolver.
//
// Order: CLAUDE_PROJECT_DIR -> self-relative (import.meta.url) -> upward
// .spec/.git marker search -> cwd. Self-relative deliberately precedes the
// marker search: a fresh install target has neither `.git` nor `.spec` (see
// the "stranger eval" lesson in .spec/lessons.md), so a resolver that only
// searched for markers silently broke there while looking fine in every
// privileged in-repo test.

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
// hook shim in tech.md: VIBE_ENGINE defaults to that path). Validate the
// full chain of directory names, not just "go up four and hope" — that is
// what lets the dogfood repo (engine/ sitting at the repo top level, not
// nested under .agents/skills/vibe/) correctly fall through to the marker
// search instead of resolving to some unrelated directory four levels above
// itself.
function selfRelativeRoot() {
  const vibeDir = path.dirname(__dirname);
  const skillsDir = path.dirname(vibeDir);
  const agentsDir = path.dirname(skillsDir);
  const isInstalledLayout =
    path.basename(vibeDir) === 'vibe' &&
    path.basename(skillsDir) === 'skills' &&
    path.basename(agentsDir) === '.agents';
  return isInstalledLayout ? path.dirname(agentsDir) : undefined;
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
