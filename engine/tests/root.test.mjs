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
