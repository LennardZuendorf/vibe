// engine/root.mjs — the self-relative/marker resolution family: resolveRoot(),
// resolveVibeDir(), resolveSkillsDir().
//
// Order for each: an explicit override -> CLAUDE_PROJECT_DIR (root only) ->
// self-relative (import.meta.url) -> upward docs/spec/.spec/.git marker search -> a // literal
// final fallback. Self-relative deliberately precedes the marker search: a
// fresh install target has neither `.git` nor `.spec` (see the "stranger // literal
// eval" lesson in .spec/lessons.md), so a resolver that only searched for // literal
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

const MARKERS = ['docs/spec', '.spec', '.git']; // literal

function hasMarker(dir) {
  return MARKERS.some((marker) => fs.existsSync(path.join(dir, marker)));
}

// An installed engine lives at <ROOT>/.agents/skills/flow/engine (see the
// hook shim in tech.md: VIBE_ENGINE defaults to that path) — engine/'s own
// parent IS the vibe skill dir (where state.json/state-machine.json live).
// Validate the full chain of directory names, not just "go up and hope" —
// that is what lets the dogfood repo (engine/ sitting at the repo top
// level, not nested under .agents/skills/flow/) correctly fall through to
// the next strategy instead of resolving to some unrelated directory. This
// is the ONLY layout with a meaningful "project root" three levels up (see
// selfRelativeRoot below) — a per-user plugin (pluginVibeDir) has none.
function vendoredVibeDir() {
  const vibeDir = path.dirname(__dirname);
  const skillsDir = path.dirname(vibeDir);
  const agentsDir = path.dirname(skillsDir);
  const isVendoredLayout =
    path.basename(vibeDir) === 'flow' &&
    path.basename(skillsDir) === 'skills' &&
    path.basename(agentsDir) === '.agents';
  return isVendoredLayout ? vibeDir : undefined;
}

// A per-user PLUGIN engine lives at <PLUGIN_ROOT>/skills/flow/engine — ONE
// level shallower than the vendored layout (no `.agents/` wrapper), because
// build-plugin.sh ships `skills/vibe` as a top-level symlink to flow/, not
// nested under `.agents/skills/`; see the real bash plugin hook's own
// self-location, `${CLAUDE_PLUGIN_ROOT}/skills/flow/scripts/doctrine.sh`
// (plugin/hooks/session-start.sh). Recognized by a sibling-SKILL.md probe
// like the one resolveSkillsDir() uses — but NOT the same probe, and the two
// CAN disagree (js-core/5 re-review). resolveSkillsDir() (and orders.sh's own
// SKILLS_DIR resolution, its oracle) is name-agnostic: it accepts any parent
// directory holding a `vibe/SKILL.md`. This function additionally requires
// that parent to be named literally `skills`, which is strictly narrower.
// Demonstrated divergence: an engine at `<X>/mySkills/vibe/engine` fails this
// probe, so resolveVibeDir() falls through to the CLAUDE_PROJECT_DIR-honouring
// fallback and the engine prints the PROJECT's doctrine, while the oracle —
// which only needs `$SKILL_PARENT/flow/SKILL.md` — prints the plugin's own.
// Every layout actually shipped (`skills/`, `.agents/skills/`,
// `.claude/skills/`) is named `skills`, so no shipped configuration reaches
// that gap and the behaviour stays as-is; the narrowing is simply not the
// equivalence an earlier revision of this comment claimed it was. Without
// this leg at all, a plugin-installed engine falls through
// resolveVibeDir()'s CLAUDE_PROJECT_DIR-honouring fallback below and reads
// the WRONG project's skill instead of its own — the exact plugin-layout
// gap review round 1 (js-core/5, Finding 2) found and this closes.
function pluginVibeDir() {
  const vibeDir = path.dirname(__dirname);
  const skillsDir = path.dirname(vibeDir);
  const isPluginLayout =
    path.basename(vibeDir) === 'flow' &&
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
// 1, Finding 4) so install.sh's existing `.agents/skills/flow` copy (a
// dereferenced copy of flow/) ships it automatically — but that placement
// alone does NOT make the self-relative leg fire here: vendoredVibeDir()'s
// chain requires the skill dir to be literally named "vibe", and this
// repo's is named "flow". So resolveVibeDir() legitimately falls through to
// root + the fixed `.agents/skills/flow` suffix even in this repo — which
// still resolves correctly via that symlink. The self-relative leg matches
// only in a real installed target, where the copy actually IS named
// `.agents/skills/flow/engine`.
export function resolveVibeDir(opts = {}) {
  if (opts.vibeDir) return opts.vibeDir;

  const selfRelative = selfRelativeVibeDir();
  if (selfRelative) return selfRelative;

  return path.join(resolveRoot(opts), '.agents', 'skills', 'flow');
}

// The directory containing sibling skills (vibe, spec, ...). Mirrors
// orders.sh's SKILLS_DIR resolution exactly: probe whether vibeDir's parent
// itself looks like a skills directory (has a `vibe/SKILL.md` inside it —
// true for an installed target, where vibeDir IS already `.agents/skills/flow`
// so its parent is `.agents/skills`); otherwise fall back to
// root + `.agents/skills`.
export function resolveSkillsDir(opts = {}) {
  if (opts.skillsDir) return opts.skillsDir;

  const vibeDir = resolveVibeDir(opts);
  const skillParent = path.dirname(vibeDir);
  if (fs.existsSync(path.join(skillParent, 'flow', 'SKILL.md'))) {
    return skillParent;
  }

  return path.join(resolveRoot(opts), '.agents', 'skills');
}

// DOCTRINE-ONLY. Not a general vibeDir/root resolver — implements exactly
// one thing: flow/scripts/doctrine.sh's own narrow precedence rule (its
// STATE variable, lines 48-52), which prefers a PROJECT's own cursor over
// the skill-local one, but ONLY when CLAUDE_PROJECT_DIR is set AND that
// project's `.agents/skills/flow/state.json` actually exists. Deliberately
// does not consult self-relative/marker resolution at all — the oracle
// never does either for this one lookup, it is a literal env-gated path
// check. Kept here, not duplicated as a literal in commands/doctrine.mjs,
// so the `.agents/skills/flow` layout constant is defined exactly once and
// the duplicate-primitive scan (js-core/8) needs no exemption for it
// (review round 1, Finding 3). Returns undefined — never a guess — when
// there is nothing to redirect to; the caller (doctrine.mjs) decides the
// fallback, matching the oracle's own STATE assignment shape.
export function resolveProjectCursorDir(opts = {}) {
  const projectDir = opts.projectDir ?? process.env.CLAUDE_PROJECT_DIR;
  if (!projectDir) return undefined;

  const candidate = path.join(projectDir, '.agents', 'skills', 'flow');
  // Deliberately NOT routed through cursorPath() (cursor.mjs): root.mjs is
  // the self-location primitive and must stay importable on its own — the
  // fixtures in root.test.mjs copy this single file into synthetic install
  // layouts, and an import of cursor.mjs would drag json.mjs along with it.
  // The duplicate-primitive scan carries this as an exact one-LINE waiver
  // instead, so a SECOND cursor-file literal in root.mjs still fails.
  return fs.existsSync(path.join(candidate, 'state.json')) ? candidate : undefined;
}

// Runtime layout and physical write anchors belong with root resolution.
export function flowRuntimeDir(root) {
  return path.join(root, '.vibe', 'run', 'flow');
}

export function resolveCursorRoot(vibeDir) {
  if (process.env.CLAUDE_PROJECT_DIR) return process.env.CLAUDE_PROJECT_DIR;
  const dir = path.resolve(vibeDir);
  if (path.basename(dir) === 'flow' && path.basename(path.dirname(dir)) === 'skills'
      && path.basename(path.dirname(path.dirname(dir))) === '.agents') {
    return path.dirname(path.dirname(path.dirname(dir)));
  }
  if (path.basename(dir) === 'flow') return path.basename(path.dirname(dir)) === '.agents'
    ? path.dirname(path.dirname(dir)) : path.dirname(dir);
  return undefined;
}

export function assertCursorWithinRoot(root, target) {
  if (!root) return;
  const realRoot = fs.realpathSync(root);
  let existing = target;
  for (;;) {
    try { fs.lstatSync(existing); break; }
    catch (err) {
      if (err.code !== 'ENOENT') throw err;
      const parent = path.dirname(existing);
      if (parent === existing) throw err;
      existing = parent;
    }
  }
  const rel = path.relative(realRoot, fs.realpathSync(existing));
  if (rel === '..' || rel.startsWith(`..${path.sep}`) || path.isAbsolute(rel)) {
    throw new Error(`cursor path ${target} escapes project ${realRoot}`);
  }
}

// Only a missing child below established searchable directories proves absence.
export function cursorEntryAbsent(target) {
  const absolute=path.isAbsolute(target) ? target : `${process.cwd()}${path.sep}${target}`;
  let current=path.parse(absolute).root;
  const parts=absolute.slice(current.length).split(path.sep === '\\' ? /[\\/]/ : path.sep).filter(Boolean);
  fs.accessSync(current,fs.constants.X_OK);
  for (let i=0; i<parts.length; i+=1) {
    current=`${current.endsWith(path.sep) ? current : current+path.sep}${parts[i]}`;
    try { fs.lstatSync(current); }
    catch (err) { if (err.code === 'ENOENT') return true; throw err; }
    const stat=fs.statSync(current);
    if (i === parts.length-1) {
      if (!stat.isFile()) throw new Error(`cursor entry ${current} is not a regular file`);
    } else {
      if (!stat.isDirectory()) throw new Error(`cursor ancestor ${current} is not a directory`);
      fs.accessSync(current,fs.constants.X_OK);
    }
  }
  return false;
}
