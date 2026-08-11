// engine/tests/root.test.mjs — engine/root.mjs (js-core/2, R3).

import { mkdtempSync, mkdirSync, writeFileSync, copyFileSync, readFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { pathToFileURL } from 'node:url';
import { test, assert, assertEqual } from './run.mjs';

const ROOT_MJS = path.resolve(path.dirname(new URL(import.meta.url).pathname), '..', 'root.mjs');

test('resolveRoot: CLAUDE_PROJECT_DIR wins over everything else when set', async () => {
  const { resolveRoot } = await import(pathToFileURL(ROOT_MJS).href + '?a');
  const prev = process.env.CLAUDE_PROJECT_DIR;
  process.env.CLAUDE_PROJECT_DIR = '/some/explicit/project/dir';
  try {
    assertEqual(resolveRoot({ cwd: '/somewhere/else' }), '/some/explicit/project/dir');
  } finally {
    if (prev === undefined) delete process.env.CLAUDE_PROJECT_DIR;
    else process.env.CLAUDE_PROJECT_DIR = prev;
  }
});

test('resolveRoot: a bare mktemp -d with no .git/.spec and no CLAUDE_PROJECT_DIR resolves without throwing', async () => {
  const { resolveRoot } = await import(pathToFileURL(ROOT_MJS).href + '?b');
  const prev = process.env.CLAUDE_PROJECT_DIR;
  delete process.env.CLAUDE_PROJECT_DIR;
  const bareDir = mkdtempSync(path.join(tmpdir(), 'vibe-root-bare-'));
  try {
    // No .git, no .spec anywhere under bareDir's own tree. Self-relative
    // fails too (this module's real __dirname isn't nested under
    // .agents/skills/vibe), so this exercises the final cwd fallback — the
    // exact "install target has neither marker" scenario from the lesson.
    const resolved = resolveRoot({ cwd: bareDir });
    assertEqual(resolved, bareDir);
  } finally {
    rmSync(bareDir, { recursive: true, force: true });
    if (prev !== undefined) process.env.CLAUDE_PROJECT_DIR = prev;
  }
});

test('resolveRoot: marker search walks upward from cwd to find .spec', async () => {
  const { resolveRoot } = await import(pathToFileURL(ROOT_MJS).href + '?c');
  const prev = process.env.CLAUDE_PROJECT_DIR;
  delete process.env.CLAUDE_PROJECT_DIR;
  const projectDir = mkdtempSync(path.join(tmpdir(), 'vibe-root-marker-'));
  const nested = path.join(projectDir, 'a', 'b', 'c');
  mkdirSync(nested, { recursive: true });
  mkdirSync(path.join(projectDir, '.spec'), { recursive: true });
  try {
    assertEqual(resolveRoot({ cwd: nested }), projectDir);
  } finally {
    rmSync(projectDir, { recursive: true, force: true });
    if (prev !== undefined) process.env.CLAUDE_PROJECT_DIR = prev;
  }
});

test('resolveRoot: self-relative resolves an installed-layout engine (.agents/skills/vibe/engine) with no markers present', async () => {
  const prev = process.env.CLAUDE_PROJECT_DIR;
  delete process.env.CLAUDE_PROJECT_DIR;

  // Build a fresh, marker-less "install target" and copy root.mjs into the
  // exact nested layout the hook shim expects: <ROOT>/.agents/skills/vibe/engine.
  const installRoot = mkdtempSync(path.join(tmpdir(), 'vibe-root-install-'));
  const engineDir = path.join(installRoot, '.agents', 'skills', 'vibe', 'engine');
  mkdirSync(engineDir, { recursive: true });
  const copiedRootMjs = path.join(engineDir, 'root.mjs');
  copyFileSync(ROOT_MJS, copiedRootMjs);

  // A directory the resolver must NOT wander into via marker search or cwd —
  // proves the installed root came from self-relative structure, not luck.
  const unrelatedCwd = mkdtempSync(path.join(tmpdir(), 'vibe-root-unrelated-'));

  try {
    const { resolveRoot } = await import(pathToFileURL(copiedRootMjs).href);
    assertEqual(resolveRoot({ cwd: unrelatedCwd }), installRoot);
  } finally {
    rmSync(installRoot, { recursive: true, force: true });
    rmSync(unrelatedCwd, { recursive: true, force: true });
    if (prev !== undefined) process.env.CLAUDE_PROJECT_DIR = prev;
  }
});

test('resolveRoot: dogfood-style layout (engine/ at repo top level) is not mistaken for the installed layout', async () => {
  // Sanity check on the real, in-repo engine/root.mjs: this repo's engine/
  // sits at the top level, so self-relative must NOT fire here — only the
  // marker search (this repo has both .git and .spec) should resolve it.
  const { resolveRoot } = await import(pathToFileURL(ROOT_MJS).href + '?d');
  const prev = process.env.CLAUDE_PROJECT_DIR;
  delete process.env.CLAUDE_PROJECT_DIR;
  try {
    const repoRoot = path.resolve(path.dirname(ROOT_MJS), '..');
    const resolved = resolveRoot({ cwd: repoRoot });
    assertEqual(resolved, repoRoot);
  } finally {
    if (prev !== undefined) process.env.CLAUDE_PROJECT_DIR = prev;
  }
});

// ---------------------------------------------------------------------------
// js-core/2 review, Finding 1 (CRITICAL) — resolveVibeDir().
// ---------------------------------------------------------------------------

test('resolveVibeDir: opts.vibeDir override short-circuits resolution', async () => {
  const { resolveVibeDir } = await import(pathToFileURL(ROOT_MJS).href + '?e');
  assertEqual(resolveVibeDir({ vibeDir: '/explicit/vibe/dir' }), '/explicit/vibe/dir');
});

test('resolveVibeDir: self-relative resolves an installed layout to the vibe dir itself, not the root', async () => {
  const prev = process.env.CLAUDE_PROJECT_DIR;
  delete process.env.CLAUDE_PROJECT_DIR;

  const installRoot = mkdtempSync(path.join(tmpdir(), 'vibe-vibedir-install-'));
  const vibeDir = path.join(installRoot, '.agents', 'skills', 'vibe');
  const engineDir = path.join(vibeDir, 'engine');
  mkdirSync(engineDir, { recursive: true });
  const copiedRootMjs = path.join(engineDir, 'root.mjs');
  copyFileSync(ROOT_MJS, copiedRootMjs);

  const unrelatedCwd = mkdtempSync(path.join(tmpdir(), 'vibe-vibedir-unrelated-'));

  try {
    const { resolveVibeDir } = await import(pathToFileURL(copiedRootMjs).href);
    const resolved = resolveVibeDir({ cwd: unrelatedCwd });
    assertEqual(resolved, vibeDir);
    assert(resolved !== installRoot, 'vibeDir must not be the project root itself');
  } finally {
    rmSync(installRoot, { recursive: true, force: true });
    rmSync(unrelatedCwd, { recursive: true, force: true });
    if (prev !== undefined) process.env.CLAUDE_PROJECT_DIR = prev;
  }
});

test('resolveVibeDir: dogfood layout falls back to root + .agents/skills/vibe, which resolves through the flow/ symlink', async () => {
  // Sanity check against the real, in-repo engine/root.mjs: self-relative
  // fails here (engine/ is top-level, not nested), so this exercises the
  // fallback. In THIS repo .agents/skills/vibe is a real symlink to flow/,
  // so reading through the fallback path must land on the same
  // state-machine.json content as flow/state-machine.json — proving the
  // fallback isn't just string-equal to the right answer by luck.
  const { resolveVibeDir } = await import(pathToFileURL(ROOT_MJS).href + '?f');
  const prev = process.env.CLAUDE_PROJECT_DIR;
  delete process.env.CLAUDE_PROJECT_DIR;
  try {
    const repoRoot = path.resolve(path.dirname(ROOT_MJS), '..');
    const resolved = resolveVibeDir({ cwd: repoRoot });
    assertEqual(resolved, path.join(repoRoot, '.agents', 'skills', 'vibe'));

    const viaFallback = readFileSync(path.join(resolved, 'state-machine.json'), 'utf8');
    const viaFlow = readFileSync(path.join(repoRoot, 'flow', 'state-machine.json'), 'utf8');
    assertEqual(viaFallback, viaFlow, 'fallback path must actually reach the real machine file');
  } finally {
    if (prev !== undefined) process.env.CLAUDE_PROJECT_DIR = prev;
  }
});

// ---------------------------------------------------------------------------
// js-core/2 review, Finding 2 (Important) — resolveSkillsDir().
// ---------------------------------------------------------------------------

test('resolveSkillsDir: opts.skillsDir override short-circuits resolution', async () => {
  const { resolveSkillsDir } = await import(pathToFileURL(ROOT_MJS).href + '?g');
  assertEqual(resolveSkillsDir({ skillsDir: '/explicit/skills/dir' }), '/explicit/skills/dir');
});

test('resolveSkillsDir: installed layout resolves via the sibling vibe/SKILL.md probe', async () => {
  const prev = process.env.CLAUDE_PROJECT_DIR;
  delete process.env.CLAUDE_PROJECT_DIR;

  const installRoot = mkdtempSync(path.join(tmpdir(), 'vibe-skillsdir-install-'));
  const skillsDir = path.join(installRoot, '.agents', 'skills');
  const vibeDir = path.join(skillsDir, 'vibe');
  const engineDir = path.join(vibeDir, 'engine');
  mkdirSync(engineDir, { recursive: true });
  writeFileSync(path.join(vibeDir, 'SKILL.md'), '# vibe\n');
  copyFileSync(ROOT_MJS, path.join(engineDir, 'root.mjs'));

  const unrelatedCwd = mkdtempSync(path.join(tmpdir(), 'vibe-skillsdir-unrelated-'));

  try {
    const { resolveSkillsDir } = await import(pathToFileURL(path.join(engineDir, 'root.mjs')).href);
    assertEqual(resolveSkillsDir({ cwd: unrelatedCwd }), skillsDir);
  } finally {
    rmSync(installRoot, { recursive: true, force: true });
    rmSync(unrelatedCwd, { recursive: true, force: true });
    if (prev !== undefined) process.env.CLAUDE_PROJECT_DIR = prev;
  }
});

test('resolveSkillsDir: no sibling vibe/SKILL.md falls back to root + .agents/skills', async () => {
  const { resolveSkillsDir } = await import(pathToFileURL(ROOT_MJS).href + '?h');
  const prev = process.env.CLAUDE_PROJECT_DIR;
  delete process.env.CLAUDE_PROJECT_DIR;

  const projectDir = mkdtempSync(path.join(tmpdir(), 'vibe-skillsdir-fallback-'));
  mkdirSync(path.join(projectDir, '.spec'), { recursive: true });
  // vibeDir override points somewhere with no sibling SKILL.md at all.
  const lonelyVibeDir = path.join(projectDir, 'somewhere', 'vibe');
  mkdirSync(lonelyVibeDir, { recursive: true });

  try {
    const resolved = resolveSkillsDir({ cwd: projectDir, vibeDir: lonelyVibeDir });
    assertEqual(resolved, path.join(projectDir, '.agents', 'skills'));
  } finally {
    rmSync(projectDir, { recursive: true, force: true });
    if (prev !== undefined) process.env.CLAUDE_PROJECT_DIR = prev;
  }
});

// ---------------------------------------------------------------------------
// js-core/5 review round 1, Finding 2 — the per-user PLUGIN layout
// (<PLUGIN_ROOT>/skills/vibe/engine, no `.agents/` wrapper — see
// build-plugin.sh and plugin/hooks/session-start.sh's own
// `${CLAUDE_PLUGIN_ROOT}/skills/vibe/scripts/doctrine.sh`). Before this fix,
// resolveVibeDir()/resolveSkillsDir() only recognized the vendored 3-level
// `.agents/skills/vibe` chain, so a plugin-shaped engine fell through to
// the CLAUDE_PROJECT_DIR-honouring fallback and resolved to the WRONG
// (project's, not plugin's) skill dir.
// ---------------------------------------------------------------------------

function buildPluginFixture() {
  const pluginRoot = mkdtempSync(path.join(tmpdir(), 'vibe-plugin-'));
  const vibeDir = path.join(pluginRoot, 'skills', 'vibe');
  const engineDir = path.join(vibeDir, 'engine');
  mkdirSync(engineDir, { recursive: true });
  writeFileSync(path.join(vibeDir, 'SKILL.md'), '# vibe (plugin)\n');
  const copiedRootMjs = path.join(engineDir, 'root.mjs');
  copyFileSync(ROOT_MJS, copiedRootMjs);
  return { pluginRoot, vibeDir, copiedRootMjs };
}

test('resolveVibeDir: recognizes the per-user plugin layout (skills/vibe/engine, no .agents/ wrapper)', async () => {
  const prev = process.env.CLAUDE_PROJECT_DIR;
  delete process.env.CLAUDE_PROJECT_DIR;

  const { pluginRoot, vibeDir, copiedRootMjs } = buildPluginFixture();
  const unrelatedCwd = mkdtempSync(path.join(tmpdir(), 'vibe-plugin-unrelated-'));

  try {
    const { resolveVibeDir } = await import(pathToFileURL(copiedRootMjs).href);
    assertEqual(resolveVibeDir({ cwd: unrelatedCwd }), vibeDir);
  } finally {
    rmSync(pluginRoot, { recursive: true, force: true });
    rmSync(unrelatedCwd, { recursive: true, force: true });
    if (prev !== undefined) process.env.CLAUDE_PROJECT_DIR = prev;
  }
});

test('resolveVibeDir: the plugin layout is NOT hijacked by an ambient CLAUDE_PROJECT_DIR (the actual regression)', async () => {
  const { pluginRoot, vibeDir, copiedRootMjs } = buildPluginFixture();

  // A DIFFERENT project, with its own real .agents/skills/vibe — the exact
  // shape that used to win via resolveVibeDir()'s old CLAUDE_PROJECT_DIR
  // fallback. If the plugin layout is recognized correctly, this must be
  // completely ignored for the SKILL location (doctrine.mjs's own,
  // separate resolveProjectCursorDir() is the only thing allowed to look
  // at CLAUDE_PROJECT_DIR, and only for the cursor).
  const projectDir = mkdtempSync(path.join(tmpdir(), 'vibe-plugin-project-'));
  mkdirSync(path.join(projectDir, '.agents', 'skills', 'vibe'), { recursive: true });

  const prev = process.env.CLAUDE_PROJECT_DIR;
  process.env.CLAUDE_PROJECT_DIR = projectDir;

  try {
    const { resolveVibeDir } = await import(pathToFileURL(copiedRootMjs).href);
    const resolved = resolveVibeDir({ cwd: projectDir });
    assertEqual(resolved, vibeDir, 'the plugin must resolve to its OWN skill dir, not the project one');
    assert(resolved !== path.join(projectDir, '.agents', 'skills', 'vibe'), 'must not be hijacked by CLAUDE_PROJECT_DIR');
  } finally {
    rmSync(pluginRoot, { recursive: true, force: true });
    rmSync(projectDir, { recursive: true, force: true });
    if (prev === undefined) delete process.env.CLAUDE_PROJECT_DIR;
    else process.env.CLAUDE_PROJECT_DIR = prev;
  }
});

test('resolveSkillsDir: the plugin layout resolves to <PLUGIN_ROOT>/skills via the same sibling-SKILL.md probe', async () => {
  const prev = process.env.CLAUDE_PROJECT_DIR;
  delete process.env.CLAUDE_PROJECT_DIR;

  const { pluginRoot, copiedRootMjs } = buildPluginFixture();
  const unrelatedCwd = mkdtempSync(path.join(tmpdir(), 'vibe-plugin-skillsdir-unrelated-'));

  try {
    const { resolveSkillsDir } = await import(pathToFileURL(copiedRootMjs).href);
    assertEqual(resolveSkillsDir({ cwd: unrelatedCwd }), path.join(pluginRoot, 'skills'));
  } finally {
    rmSync(pluginRoot, { recursive: true, force: true });
    rmSync(unrelatedCwd, { recursive: true, force: true });
    if (prev !== undefined) process.env.CLAUDE_PROJECT_DIR = prev;
  }
});

test('resolveRoot: the plugin layout has no project root of its own — self-relative does not fire for it', async () => {
  // Discriminates vendoredVibeDir() (used by selfRelativeRoot) from the
  // broader selfRelativeVibeDir(): a plugin engine's "three levels up" from
  // skills/vibe would land OUTSIDE the plugin entirely and mean nothing, so
  // resolveRoot() must fall through past self-relative to marker search /
  // cwd instead of fabricating a bogus root.
  const prev = process.env.CLAUDE_PROJECT_DIR;
  delete process.env.CLAUDE_PROJECT_DIR;

  const { pluginRoot, copiedRootMjs } = buildPluginFixture();
  const bareCwd = mkdtempSync(path.join(tmpdir(), 'vibe-plugin-root-bare-'));

  try {
    const { resolveRoot } = await import(pathToFileURL(copiedRootMjs).href);
    const resolved = resolveRoot({ cwd: bareCwd });
    assertEqual(resolved, bareCwd, 'falls through to the cwd fallback, not a fabricated plugin-relative root');
    assert(resolved !== pluginRoot, 'must not treat the plugin root as a project root');
  } finally {
    rmSync(pluginRoot, { recursive: true, force: true });
    rmSync(bareCwd, { recursive: true, force: true });
    if (prev !== undefined) process.env.CLAUDE_PROJECT_DIR = prev;
  }
});

// ---------------------------------------------------------------------------
// resolveProjectCursorDir() — doctrine-only, review round 1 Finding 3.
// ---------------------------------------------------------------------------

test('resolveProjectCursorDir: undefined when CLAUDE_PROJECT_DIR is unset', async () => {
  const { resolveProjectCursorDir } = await import(pathToFileURL(ROOT_MJS).href + '?i');
  const prev = process.env.CLAUDE_PROJECT_DIR;
  delete process.env.CLAUDE_PROJECT_DIR;
  try {
    assertEqual(resolveProjectCursorDir(), undefined);
  } finally {
    if (prev !== undefined) process.env.CLAUDE_PROJECT_DIR = prev;
  }
});

test('resolveProjectCursorDir: undefined when CLAUDE_PROJECT_DIR is set but has no cursor', async () => {
  const { resolveProjectCursorDir } = await import(pathToFileURL(ROOT_MJS).href + '?j');
  const projectDir = mkdtempSync(path.join(tmpdir(), 'vibe-projectcursor-empty-'));
  try {
    assertEqual(resolveProjectCursorDir({ projectDir }), undefined);
  } finally {
    rmSync(projectDir, { recursive: true, force: true });
  }
});

test('resolveProjectCursorDir: the project vibe dir when its state.json exists', async () => {
  const { resolveProjectCursorDir } = await import(pathToFileURL(ROOT_MJS).href + '?k');
  const projectDir = mkdtempSync(path.join(tmpdir(), 'vibe-projectcursor-present-'));
  const projectVibeDir = path.join(projectDir, '.agents', 'skills', 'vibe');
  mkdirSync(projectVibeDir, { recursive: true });
  writeFileSync(path.join(projectVibeDir, 'state.json'), '{}\n');
  try {
    assertEqual(resolveProjectCursorDir({ projectDir }), projectVibeDir);
  } finally {
    rmSync(projectDir, { recursive: true, force: true });
  }
});
