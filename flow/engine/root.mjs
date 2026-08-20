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
// the next strategy instead of resolving to some unrelated directory. This
// is the ONLY layout with a meaningful "project root" three levels up (see
// selfRelativeRoot below) — a per-user plugin (pluginVibeDir) has none.
function vendoredVibeDir() {
  const vibeDir = path.dirname(__dirname);
  const skillsDir = path.dirname(vibeDir);
  const agentsDir = path.dirname(skillsDir);
  const isVendoredLayout =
    path.basename(vibeDir) === 'vibe' &&
    path.basename(skillsDir) === 'skills' &&
    path.basename(agentsDir) === '.agents';
  return isVendoredLayout ? vibeDir : undefined;
}

// A per-user PLUGIN engine lives at <PLUGIN_ROOT>/skills/vibe/engine — ONE
// level shallower than the vendored layout (no `.agents/` wrapper), because
// build-plugin.sh ships `skills/vibe` as a top-level symlink to flow/, not
// nested under `.agents/skills/`; see the real bash plugin hook's own
// self-location, `${CLAUDE_PLUGIN_ROOT}/skills/vibe/scripts/doctrine.sh`
// (plugin/hooks/session-start.sh). Recognized by the SAME sibling-SKILL.md
// probe resolveSkillsDir() already uses for the vendored layout's fallback,
// so the two checks never disagree about what counts as "the skill is
// really here". Without this leg, a plugin-installed engine falls through
// resolveVibeDir()'s CLAUDE_PROJECT_DIR-honouring fallback below and reads
// the WRONG project's skill instead of its own — the exact plugin-layout
// gap review round 1 (js-core/5, Finding 2) found and this closes.
function pluginVibeDir() {
  const vibeDir = path.dirname(__dirname);
  const skillsDir = path.dirname(vibeDir);
  const isPluginLayout =
    path.basename(vibeDir) === 'vibe' &&
    path.basename(skillsDir) === 'skills' &&
    fs.existsSync(path.join(vibeDir, 'SKILL.md'));
  return isPluginLayout ? vibeDir : undefined;
}

function selfRelativeVibeDir() {
  return vendoredVibeDir() ?? pluginVibeDir();
}

// The project root is three levels above the self-relative vibe dir
// (vibe -> skills -> .agents -> ROOT) when that STRICT vendored chain
// validates. Deliberately uses vendoredVibeDir(), not selfRelativeVibeDir():
// a per-user plugin's engine has no project root of its own to report (its
// three-levels-up would land outside the plugin entirely) — callers that
// need the actual project root from a plugin context use CLAUDE_PROJECT_DIR
// (resolveRoot's own next leg), not this.
function selfRelativeRoot() {
  const vibeDir = vendoredVibeDir();
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
// engine/ lives at flow/engine/ in THIS source repo (js-core/7 review round
// 1, Finding 4) so install.sh's existing `.agents/skills/vibe` copy (a
// dereferenced copy of flow/) ships it automatically — but that placement
// alone does NOT make the self-relative leg fire here: vendoredVibeDir()'s
// chain requires the skill dir to be literally named "vibe", and this
// repo's is named "flow". So resolveVibeDir() legitimately falls through to
// root + the fixed `.agents/skills/vibe` suffix even in this repo — which
// still resolves correctly via that symlink. The self-relative leg matches
// only in a real installed target, where the copy actually IS named
// `.agents/skills/vibe/engine`.
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

// DOCTRINE-ONLY. Not a general vibeDir/root resolver — implements exactly
// one thing: flow/scripts/doctrine.sh's own narrow precedence rule (its
// STATE variable, lines 48-52), which prefers a PROJECT's own cursor over
// the skill-local one, but ONLY when CLAUDE_PROJECT_DIR is set AND that
// project's `.agents/skills/vibe/state.json` actually exists. Deliberately
// does not consult self-relative/marker resolution at all — the oracle
// never does either for this one lookup, it is a literal env-gated path
// check. Kept here, not duplicated as a literal in commands/doctrine.mjs,
// so the `.agents/skills/vibe` layout constant is defined exactly once and
// the duplicate-primitive scan (js-core/8) needs no exemption for it
// (review round 1, Finding 3). Returns undefined — never a guess — when
// there is nothing to redirect to; the caller (doctrine.mjs) decides the
// fallback, matching the oracle's own STATE assignment shape.
export function resolveProjectCursorDir(opts = {}) {
  const projectDir = opts.projectDir ?? process.env.CLAUDE_PROJECT_DIR;
  if (!projectDir) return undefined;

  const candidate = path.join(projectDir, '.agents', 'skills', 'vibe');
  return fs.existsSync(path.join(candidate, 'state.json')) ? candidate : undefined;
}
