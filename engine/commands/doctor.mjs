// engine/commands/doctor.mjs — `vibe doctor`, ported from
// flow/scripts/doctor.sh (the oracle; read closely, never edited). Byte
// parity with the bash script is the acceptance test — see
// engine/tests/doctor.test.mjs. Warn-only, read-only, ALWAYS {code: 0} —
// preserved exactly, including on missing files, broken symlinks, and
// unreadable JSON. Never crosses "warn" into "fail": a broken install must
// still be able to report on itself.
//
// jq-presence note (per the task brief): doctor.sh's tool.jq check is REAL
// — it spawns `jq --version` — even though the ENGINE itself never needs
// jq for any of its own JSON reads (readJson/readCursor are pure JS, no
// shelling out). The line is kept ONLY for byte parity with the bash
// oracle's tool.jq check; removing it belongs to a later feature
// (plugin-runtime). Do not "clean this up" here. The jq PRESENCE also
// gates several other checks below exactly like the oracle: when jq is
// missing, the machine/deps JSON-validity checks and the cursor
// validate-state.sh check all degrade to their no-jq branch, which is not
// simply "skip validation" — it is a specific, sometimes more lenient,
// documented shortcut. Reproduce the shortcut, do not "improve" it.
//
// No CLI positional ROOT override (doctor.sh's `doctor.sh [<repo-root>]`):
// when the oracle is given an explicit root it joins VIBE_SKILL/SPEC_SKILL
// off it literally, bypassing self-location entirely. Reproducing that
// would mean re-deriving the `.agents/skills/vibe` join in this file, which
// the task brief forbids — root.mjs is the one place that constant lives.
// This port only serves the no-arg path (`vibe doctor`), the only path
// unit 7's hook shims and the CLI actually exercise.
//
// Per repo convention: never re-derive root/vibeDir/skillsDir, never parse
// cursor/machine/manifest JSON directly here — resolveRoot/resolveVibeDir/
// resolveSkillsDir (root.mjs), readCursor (cursor.mjs), readJson (json.mjs)
// are the only primitives for that.

import fs from 'node:fs';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import { resolveRoot, resolveVibeDir, resolveSkillsDir } from '../root.mjs';
import { readCursor } from '../cursor.mjs';
import { readJson } from '../json.mjs';

const HOOK_SCRIPTS = [
  'session-start-doctrine.sh',
  'user-prompt-submit-inject.sh',
  'pre-tool-use-guard.sh',
  'stop-gate.sh',
];

// ---------------------------------------------------------------------------
// Output formatting — mirrors the oracle's `ok()`/`warn()` printf helpers
// exactly (5-char-wide prefix so the two verdicts column-align).
// ---------------------------------------------------------------------------

function ok(id, msg) {
  return `ok   ${id} ${msg}\n`;
}

function warn(id, msg) {
  return `warn ${id} ${msg}\n`;
}

// ---------------------------------------------------------------------------
// Small guarded filesystem helpers. Every one degrades to "not present" on
// a non-string path (a caller passing a bad root/vibeDir/skillsDir) or any
// fs error — never throws, matching the never-throws contract shared by
// every command since orders.mjs/state.mjs's review lessons.
// ---------------------------------------------------------------------------

function joinMaybe(base, ...parts) {
  return typeof base === 'string' ? path.join(base, ...parts) : undefined;
}

function isRegularFile(p) {
  if (typeof p !== 'string') return false;
  try {
    return fs.statSync(p).isFile();
  } catch {
    return false;
  }
}

function isDirectory(p) {
  if (typeof p !== 'string') return false;
  try {
    return fs.statSync(p).isDirectory();
  } catch {
    return false;
  }
}

function isExecutable(p) {
  if (typeof p !== 'string') return false;
  try {
    fs.accessSync(p, fs.constants.X_OK);
    return true;
  } catch {
    return false;
  }
}

// ---------------------------------------------------------------------------
// core.spec / core.vibe — ok when P is a real dir, or a symlink that
// resolves; warn when broken/absent. Mirrors doctor.sh's
// check_link_or_dir() exactly, including its quirk of calling a same-named
// regular FILE "absent" too (bash `-d` is false for a file, same as for
// nothing there at all — never "fixed" here, byte parity is the contract).
// ---------------------------------------------------------------------------

function checkLinkOrDir(id, p) {
  if (typeof p !== 'string') return warn(id, `${p} is absent`);
  let lst;
  try {
    lst = fs.lstatSync(p);
  } catch {
    return warn(id, `${p} is absent`);
  }
  if (lst.isSymbolicLink()) {
    if (fs.existsSync(p)) {
      return ok(id, `${p} -> ${fs.readlinkSync(p)} (symlink resolves)`);
    }
    return warn(id, `${p} -> ${fs.readlinkSync(p)} is a BROKEN symlink`);
  }
  if (lst.isDirectory()) {
    return ok(id, `${p} is a real directory`);
  }
  return warn(id, `${p} is absent`);
}

// ---------------------------------------------------------------------------
// tool.jq — the one check that genuinely shells out. Presence is detected
// by actually running `jq --version`, exactly like the oracle's
// `command -v jq` + `jq --version 2>/dev/null` pair. Kept vestigial per the
// task brief.
// ---------------------------------------------------------------------------

function detectJq() {
  const res = spawnSync('jq', ['--version'], { encoding: 'utf8' });
  if (!res || res.error || res.status !== 0) return { present: false, version: '' };
  return { present: true, version: (res.stdout || '').trim() };
}

// Tests need to exercise both the jq-present and jq-absent branches inside
// one process without mutating the real PATH mid-run — opts.jqPresent (and
// opts.jqVersion for the message text) overrides real detection when a test
// explicitly supplies it; production callers (the CLI) never pass it, so
// `vibe doctor` always reports the machine's real jq status.
function jqStatus(opts) {
  if (opts && typeof opts.jqPresent === 'boolean') {
    return { present: opts.jqPresent, version: typeof opts.jqVersion === 'string' ? opts.jqVersion : '' };
  }
  return detectJq();
}

function checkToolJq(jq) {
  if (jq.present) {
    return ok('tool.jq', `jq present (${jq.version})`);
  }
  return warn(
    'tool.jq',
    'jq not installed (recommended, not required) — set-state writes the cursor via printf, the guard extracts paths via sed, state reads degrade to idle; cursor + manifest checks unverified',
  );
}

// ---------------------------------------------------------------------------
// machine — state-machine.json presence + (jq-gated) parse validity. jq's
// `-e` fails not only on invalid JSON but also on a top-level `null`/
// `false` document; reproduced by the explicit value check below, not just
// a try/catch. When jq is absent the oracle skips validation ENTIRELY and
// always reports "present" — not a smarter/safer check, a documented
// shortcut. Reproduce it as-is.
// ---------------------------------------------------------------------------

function checkMachine(vibeDir, jqPresent) {
  const machinePath = joinMaybe(vibeDir, 'state-machine.json');
  if (!isRegularFile(machinePath)) {
    return warn('machine', `state-machine.json missing at ${machinePath} — flow harness incomplete`);
  }
  if (jqPresent) {
    let value;
    try {
      value = readJson(machinePath);
    } catch {
      return warn('machine', 'state-machine.json is present but not valid JSON');
    }
    if (value === null || value === false) {
      return warn('machine', 'state-machine.json is present but not valid JSON');
    }
  }
  return ok('machine', 'state-machine.json present');
}

// ---------------------------------------------------------------------------
// cursor — absent is idle (ok); present-without-jq is an unverified ok
// (validate-state.sh itself needs jq, so the oracle cannot run it either);
// present-with-jq spawns the REAL validate-state.sh (never reimplemented
// here) and reports valid/invalid off its exit code.
// ---------------------------------------------------------------------------

function runValidateState(scriptPath) {
  const res = spawnSync('bash', [scriptPath], { stdio: 'ignore' });
  return !res.error && res.status === 0;
}

// Mirrors jq's `.feature // "none"` STRING INTERPOLATION (not a top-level
// `-r` value): a string interpolates raw, anything else uses jq's
// tostring conversion, which for objects/arrays is always compact (never
// pretty, unlike a bare `-r` document dump). null/false take the `// "none"`
// branch; only reachable via a hand-edited cursor, but worth getting right.
function jqFeatureOrNone(value) {
  if (value === null || value === undefined || value === false) return 'none';
  return typeof value === 'string' ? value : JSON.stringify(value);
}

function checkCursor(vibeDir, jqPresent) {
  const statePath = joinMaybe(vibeDir, 'state.json');
  if (!isRegularFile(statePath)) {
    return ok('cursor', 'no flow cursor (idle) — normal when not mid-flow');
  }
  if (!jqPresent) {
    return ok('cursor', 'flow cursor present (unverified — jq missing; validate-state.sh needs jq)');
  }

  const validateScript = joinMaybe(vibeDir, 'scripts', 'validate-state.sh');
  const valid = isExecutable(validateScript) && runValidateState(validateScript);
  if (!valid) {
    return warn('cursor', 'flow cursor present but invalid — run validate-state.sh (or reseed from state.example.json)');
  }

  let summary = 'present';
  try {
    const cursor = readCursor(vibeDir);
    summary = `${cursor.flow}.${cursor.phase} feature=${jqFeatureOrNone(cursor.feature)}`;
  } catch {
    summary = 'present';
  }
  return ok('cursor', `flow cursor valid (${summary})`);
}

// ---------------------------------------------------------------------------
// Claude adapter wiring — hook scripts present under .claude/hooks, then
// whether .claude/settings.json actually wires all four by name (a plain
// substring/grep -F check, not JSON-aware — matches the oracle).
// ---------------------------------------------------------------------------

function checkAdapter(root) {
  let lines = '';
  let allPresent = true;

  for (const hs of HOOK_SCRIPTS) {
    const p = joinMaybe(root, '.claude', 'hooks', hs);
    if (isRegularFile(p)) {
      lines += ok(`adapter.script.${hs}`, `.claude/hooks/${hs} present`);
    } else {
      lines += warn(`adapter.script.${hs}`, `.claude/hooks/${hs} missing — re-run install.sh`);
      allPresent = false;
    }
  }

  const settingsPath = joinMaybe(root, '.claude', 'settings.json');
  const settingsPresent = isRegularFile(settingsPath);
  let settingsText;
  if (settingsPresent) {
    try {
      settingsText = fs.readFileSync(settingsPath, 'utf8');
    } catch {
      settingsText = '';
    }
  }

  if (settingsPresent) {
    const unwired = HOOK_SCRIPTS.filter((hs) => !settingsText.includes(hs));
    if (unwired.length === 0) {
      lines += ok('adapter.activation', `all ${HOOK_SCRIPTS.length} vibe hooks wired in .claude/settings.json`);
    } else {
      lines += warn(
        'adapter.activation',
        `hooks present but NOT wired in .claude/settings.json (issue #12 gap: ${unwired.join(' ')}) — re-run install.sh`,
      );
    }
  } else if (allPresent) {
    lines += warn(
      'adapter.activation',
      '.claude/settings.json absent — hooks not activated (issue #12 gap) — re-run install.sh',
    );
  } else {
    lines += warn('adapter.activation', '.claude/settings.json absent and hook scripts missing — re-run install.sh');
  }

  return { lines, allPresent, settingsPresent, settingsText };
}

// ---------------------------------------------------------------------------
// instruction.coverage — three independent carriers; ok if any fire. The
// plugin-installed probe and dep_present() below both read real
// filesystem state under $HOME, exactly like the oracle — `home` is
// threaded through explicitly (default process.env.HOME) so tests can
// fixture it without touching the real machine's ~/.claude.
// ---------------------------------------------------------------------------

// find "$dir" -maxdepth N -type f -name plugin.json -path '*vibe*': files
// only, exact (case-sensitive) basename, substring match on the FULL path
// as constructed from `startDir` (mirrors find's own path text, which
// always carries the starting-point prefix).
function findPluginJson(startDir, maxDepth) {
  function visit(dir, level) {
    let entries;
    try {
      entries = fs.readdirSync(dir, { withFileTypes: true });
    } catch {
      return false;
    }
    const childLevel = level + 1;
    for (const e of entries) {
      const full = path.join(dir, e.name);
      if (e.isFile() && e.name === 'plugin.json' && full.includes('vibe')) return true;
      if (e.isDirectory() && childLevel < maxDepth) {
        if (visit(full, childLevel)) return true;
      }
    }
    return false;
  }
  return visit(startDir, 0);
}

// find "$dir" -maxdepth N -iname "$name": any type, case-insensitive
// basename match, including the starting dir itself.
function findInameBelow(startDir, name, maxDepth) {
  const target = name.toLowerCase();
  if (path.basename(startDir).toLowerCase() === target) return true;
  function visit(dir, level) {
    let entries;
    try {
      entries = fs.readdirSync(dir, { withFileTypes: true });
    } catch {
      return false;
    }
    const childLevel = level + 1;
    for (const e of entries) {
      if (e.name.toLowerCase() === target) return true;
      if (e.isDirectory() && childLevel < maxDepth) {
        if (visit(path.join(dir, e.name), childLevel)) return true;
      }
    }
    return false;
  }
  return visit(startDir, 0);
}

// dep_present(): mirrors the oracle's own string-concatenation exactly
// (`"$HOME/.claude/skills/$n"`), including the edge case of an
// empty/unset HOME collapsing to an absolute `/.claude/...` path — plain
// template interpolation, not path.join, so that edge case matches byte
// for byte instead of being silently normalized away.
function depPresent(home, name) {
  const h = typeof home === 'string' ? home : '';
  if (isDirectory(`${h}/.claude/skills/${name}`)) return true;
  const pluginsDir = `${h}/.claude/plugins`;
  if (!isDirectory(pluginsDir)) return false;
  return findInameBelow(pluginsDir, name, 5);
}

function checkInstructionCoverage(vibeDir, adapter, home) {
  let doctrineBlock = false;
  const skillMdPath = joinMaybe(vibeDir, 'SKILL.md');
  if (isRegularFile(skillMdPath)) {
    try {
      doctrineBlock = fs.readFileSync(skillMdPath, 'utf8').includes('<!-- vibe:doctrine -->');
    } catch {
      doctrineBlock = false;
    }
  }

  const sessionStartWired =
    adapter.settingsPresent &&
    typeof adapter.settingsText === 'string' &&
    adapter.settingsText.includes('session-start-doctrine.sh');

  const h = typeof home === 'string' ? home : '';
  const pluginsDir = `${h}/.claude/plugins`;
  const pluginInstalled = isDirectory(pluginsDir) && findPluginJson(pluginsDir, 6);

  if (doctrineBlock || sessionStartWired || pluginInstalled) {
    const carriers = [];
    if (doctrineBlock) carriers.push('doctrine block');
    if (sessionStartWired) carriers.push('SessionStart hook');
    if (pluginInstalled) carriers.push('per-user plugin');
    return ok('instruction.coverage', `doctrine reaches the agent via: ${carriers.join(', ')}`);
  }
  return warn(
    'instruction.coverage',
    'no doctrine coverage — no <!-- vibe:doctrine --> block, no wired SessionStart hook, no per-user plugin; run install.sh (--local or --global) / setup.apply',
  );
}

// ---------------------------------------------------------------------------
// deps.manifest + per-dep presence.
// ---------------------------------------------------------------------------

function checkDeps(vibeDir, jqPresent, home) {
  const depsPath = joinMaybe(vibeDir, 'reference', 'deps.json');
  if (!isRegularFile(depsPath)) {
    return warn('deps.manifest', `deps.json missing at ${depsPath}`);
  }
  if (!jqPresent) {
    return warn('deps.manifest', 'deps.json present but jq unavailable — cannot read dependency list');
  }

  let raw;
  try {
    raw = readJson(depsPath);
  } catch {
    return warn('deps.manifest', 'deps.json is not valid JSON');
  }
  if (raw === null || raw === false) {
    return warn('deps.manifest', 'deps.json is not valid JSON');
  }

  let lines = ok('deps.manifest', `dependency manifest valid (${depsPath})`);
  const deps = raw && typeof raw === 'object' && Array.isArray(raw.deps) ? raw.deps : [];
  for (const dep of deps) {
    if (!dep || typeof dep !== 'object') continue;
    const name = typeof dep.name === 'string' ? dep.name : '';
    if (!name) continue; // matches the oracle's `[[ -n "$name" ]] || continue`
    const kind = typeof dep.kind === 'string' ? dep.kind : '';
    const degrade = typeof dep.degrade === 'string' ? dep.degrade : '';
    if (depPresent(home, name)) {
      lines += ok(`dep.${name}`, `${kind} '${name}' present on disk`);
    } else {
      lines += warn(`dep.${name}`, `${kind} '${name}' not found — degrade: ${degrade}`);
    }
  }
  return lines;
}

// ---------------------------------------------------------------------------
// Pure: takes the already-resolved root/vibeDir/skillsDir, returns
// {code, stdout, stderr} — NEVER throws, matching every other command's
// contract, so unit 7's hook shims can call it straight through. Guards
// non-string root/vibeDir/skillsDir (review lesson from units 4/5): every
// check function above degrades to its own "absent"/warn branch on a
// non-string path rather than throwing on a bad path.join.
// ---------------------------------------------------------------------------

export function runDoctor(root, vibeDir, skillsDir, opts = {}) {
  const home = opts && typeof opts.home === 'string' ? opts.home : process.env.HOME;
  const jq = jqStatus(opts);

  let stdout = `# vibe doctor — ${root}\n`;
  stdout += checkToolJq(jq);
  stdout += checkLinkOrDir('core.spec', joinMaybe(skillsDir, 'spec'));
  stdout += checkLinkOrDir('core.vibe', vibeDir);
  stdout += checkMachine(vibeDir, jq.present);
  stdout += checkCursor(vibeDir, jq.present);

  const adapter = checkAdapter(root);
  stdout += adapter.lines;
  stdout += checkInstructionCoverage(vibeDir, adapter, home);
  stdout += checkDeps(vibeDir, jq.present, home);

  return { code: 0, stdout, stderr: '' };
}

export default async function run(argv, opts = {}) {
  const root = resolveRoot(opts);
  const vibeDir = resolveVibeDir(opts);
  const skillsDir = resolveSkillsDir(opts);

  const result = runDoctor(root, vibeDir, skillsDir, opts);

  if (result.stdout) process.stdout.write(result.stdout);
  if (result.stderr) process.stderr.write(result.stderr);
  return result.code;
}
