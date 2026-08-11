// engine/tests/root.test.mjs — engine/root.mjs (js-core/2, R3).

import { mkdtempSync, mkdirSync, writeFileSync, copyFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { pathToFileURL } from 'node:url';
import { test, assertEqual } from './run.mjs';

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
