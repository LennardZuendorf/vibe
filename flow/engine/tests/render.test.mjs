// engine/tests/render.test.mjs — `vibe render`, the content layer's CLI.
//
// Split from content.test.mjs on purpose: that file owns RESOLUTION (what a
// channel composes to), this one owns the COMMAND CONTRACT — exit codes, the
// document write, and idempotence. The document write is the part with teeth:
// it edits a file that holds user prose outside the managed block, so "never
// clobber, never rewrite when unchanged, refuse rather than write garbage" are
// assertions, not intentions.

import { mkdirSync, writeFileSync, readFileSync, existsSync, readdirSync, symlinkSync, lstatSync, readlinkSync, rmSync } from 'node:fs';
import path from 'node:path';
import { test, assert, assertEqual, assertIncludes, assertMatch, makeHookSandbox, mkTempRoot, runCli } from './run.mjs';
import { runRender } from '../commands/render.mjs';

function makeRenderSandbox({ defaults, project, blocks = {}, cursor } = {}) {
  const sb = makeHookSandbox({ cursor });
  const contentDir = path.join(sb.vibeDir, 'content');
  if (defaults !== undefined) {
    mkdirSync(contentDir, { recursive: true });
    writeFileSync(path.join(contentDir, 'vibe.default.json'), `${JSON.stringify(defaults, null, 2)}\n`);
  }
  for (const [rel, body] of Object.entries(blocks)) {
    const file = path.join(contentDir, 'blocks', rel);
    mkdirSync(path.dirname(file), { recursive: true });
    writeFileSync(file, body);
  }
  if (project !== undefined) {
    writeFileSync(path.join(sb.dir, 'vibe.json'), `${JSON.stringify(project, null, 2)}\n`);
  }
  return { ...sb, ctx: { root: sb.dir, vibeDir: sb.vibeDir, skillsDir: sb.skillsDir } };
}

const BLOCK = (id, summary, body) =>
  `---\nid: ${id}\ntitle: ${id} title\nchannels: [doc]\n---\n<!-- vibe:summary -->\n${summary}\n<!-- /vibe:summary -->\n\n${body ?? `${id} prose`}\n`;

const DOC_DEFAULTS = {
  version: 1,
  channels: {
    doc: {
      render: 'body',
      budget: 40,
      blocks: ['a.one'],
      write: { file: 'AGENTS.md', block: 'vibe:rules', note: '_managed_' },
    },
  },
};

// ---------------------------------------------------------------------------
// Reporting modes
// ---------------------------------------------------------------------------

test('render --list: names both config sources, the block count, and each channel’s blocks', () => {
  const sb = makeRenderSandbox({ defaults: DOC_DEFAULTS, blocks: { 'a/one.md': BLOCK('a.one', 'S') } });
  try {
    const { code, stdout } = runRender(sb.ctx, ['--list']);
    assertEqual(code, 0);
    assertIncludes(stdout, 'vibe.default.json');
    assertIncludes(stdout, '(absent — shipped defaults only)');
    assertIncludes(stdout, 'blocks:   1');
    assertIncludes(stdout, 'doc (render=body, trigger=level, budget=40)');
    assertIncludes(stdout, 'a.one');
  } finally {
    sb.cleanup();
  }
});

test('render --list: a referenced-but-missing block is shown as MISSING rather than omitted', () => {
  const sb = makeRenderSandbox({ defaults: DOC_DEFAULTS });
  try {
    assertIncludes(runRender(sb.ctx, ['--list']).stdout, 'a.one — MISSING');
  } finally {
    sb.cleanup();
  }
});

test('render --check: a clean tree exits 0 and reports what it examined', () => {
  const sb = makeRenderSandbox({ defaults: DOC_DEFAULTS, blocks: { 'a/one.md': BLOCK('a.one', 'S') } });
  try {
    const { code, stdout } = runRender(sb.ctx, ['--check']);
    assertEqual(code, 0);
    assertMatch(stdout, /checked 1 blocks across 1 channels — 0 error\(s\)/);
  } finally {
    sb.cleanup();
  }
});

test('render --check: a broken tree exits 1 with the error on stderr', () => {
  const sb = makeRenderSandbox({ defaults: DOC_DEFAULTS });
  try {
    const { code, stderr } = runRender(sb.ctx, ['--check']);
    assertEqual(code, 1);
    assertIncludes(stderr, "no such block 'a.one'");
  } finally {
    sb.cleanup();
  }
});

test('render --check: an EMPTY tree exits 1 (a check with no population is not a pass)', () => {
  const sb = makeRenderSandbox({});
  try {
    const { code, stderr } = runRender(sb.ctx, ['--check']);
    assertEqual(code, 1);
    assertIncludes(stderr, 'no content blocks found');
  } finally {
    sb.cleanup();
  }
});

test('render <channel>: composes to stdout; an unknown channel exits 1', () => {
  const sb = makeRenderSandbox({ defaults: DOC_DEFAULTS, blocks: { 'a/one.md': BLOCK('a.one', 'S', 'BODY') } });
  try {
    const ok = runRender(sb.ctx, ['doc']);
    assertEqual(ok.code, 0);
    assertEqual(ok.stdout, '## a.one title\n\nBODY\n');

    const bad = runRender(sb.ctx, ['nope']);
    assertEqual(bad.code, 1);
    assertIncludes(bad.stderr, "unknown channel 'nope'");
  } finally {
    sb.cleanup();
  }
});

// Both spellings of help, in one case, on purpose: `--help` is the control that
// proves the help branch is reachable at all, so a red `-h` leg is about `-h`
// and not about the branch having moved. `-h` used to fall through to the
// channel lookup and exit 1 with `unknown channel '-h'`, because only `--`
// arguments were collected as flags.
test('render: -h and --help both print usage and exit 0', () => {
  const sb = makeRenderSandbox({ defaults: DOC_DEFAULTS });
  try {
    const long = runRender(sb.ctx, ['--help']);
    assertEqual(long.code, 0, `--help: ${long.stderr}`);
    assertIncludes(long.stdout, 'usage: vibe render');

    const short = runRender(sb.ctx, ['-h']);
    assertEqual(short.code, 0, `-h: ${short.stderr}`);
    assertIncludes(short.stdout, 'usage: vibe render');
    assertEqual(short.stdout, long.stdout, '-h and --help must print the same usage text');

    // `-h` is the ONLY short flag that changed classification: a lone `-x` is
    // still an ordinary positional, so this fix cannot have swallowed anyone
    // else's argument.
    assertEqual(runRender(sb.ctx, ['-x']).code, 1, "a non-help short argument is still looked up as a channel");
    assertIncludes(runRender(sb.ctx, ['-x']).stderr, "unknown channel '-x'");
  } finally {
    sb.cleanup();
  }
});

test('render: no channel and no flag prints usage and exits 1', () => {
  const sb = makeRenderSandbox({ defaults: DOC_DEFAULTS });
  try {
    const { code, stderr } = runRender(sb.ctx, []);
    assertEqual(code, 1);
    assertIncludes(stderr, 'usage: vibe render');
  } finally {
    sb.cleanup();
  }
});

// ---------------------------------------------------------------------------
// The document write
// ---------------------------------------------------------------------------

test('render --write: appends the managed block, then is a NO-OP on an unchanged re-run', () => {
  const sb = makeRenderSandbox({ defaults: DOC_DEFAULTS, blocks: { 'a/one.md': BLOCK('a.one', 'S', 'BODY') } });
  const target = path.join(sb.dir, 'AGENTS.md');
  try {
    writeFileSync(target, '# AGENTS.md\n\nuser prose\n');
    const first = runRender(sb.ctx, ['doc', '--write']);
    assertEqual(first.code, 0);
    assertIncludes(first.stdout, 'appended the vibe:rules block');

    const text = readFileSync(target, 'utf8');
    assertIncludes(text, 'user prose', 'user content outside the markers is never touched');
    assertIncludes(text, '<!-- vibe:rules -->\n_managed_\n\n## a.one title\n\nBODY\n<!-- /vibe:rules -->');

    const second = runRender(sb.ctx, ['doc', '--write']);
    assertIncludes(second.stdout, 'no change');
    assertEqual(readFileSync(target, 'utf8'), text, 'an unchanged render must not rewrite the file');
  } finally {
    sb.cleanup();
  }
});

test('render --write: replaces an existing block in place, preserving prose on both sides', () => {
  const sb = makeRenderSandbox({ defaults: DOC_DEFAULTS, blocks: { 'a/one.md': BLOCK('a.one', 'S', 'NEW') } });
  const target = path.join(sb.dir, 'AGENTS.md');
  try {
    writeFileSync(target, 'above\n<!-- vibe:rules -->\nstale\n<!-- /vibe:rules -->\nbelow\n');
    const result = runRender(sb.ctx, ['doc', '--write']);
    assertIncludes(result.stdout, 'replaced the vibe:rules block');
    const text = readFileSync(target, 'utf8');
    assertMatch(text, /^above\n/);
    assertMatch(text, /\nbelow\n$/);
    assertIncludes(text, 'NEW');
    assert(!text.includes('stale'), 'the previous rendered content is gone');
  } finally {
    sb.cleanup();
  }
});

test('render --write: creates the document when it does not exist yet', () => {
  const sb = makeRenderSandbox({ defaults: DOC_DEFAULTS, blocks: { 'a/one.md': BLOCK('a.one', 'S', 'BODY') } });
  try {
    assertEqual(runRender(sb.ctx, ['doc', '--write']).code, 0);
    assertIncludes(readFileSync(path.join(sb.dir, 'AGENTS.md'), 'utf8'), 'BODY');
  } finally {
    sb.cleanup();
  }
});

test('render --write: a channel with no write target is refused, not guessed', () => {
  const sb = makeRenderSandbox({
    defaults: { version: 1, channels: { doc: { render: 'body', blocks: ['a.one'] } } },
    blocks: { 'a/one.md': BLOCK('a.one', 'S', 'BODY') },
  });
  try {
    const { code, stderr } = runRender(sb.ctx, ['doc', '--write']);
    assertEqual(code, 1);
    assertIncludes(stderr, 'has no write target');
  } finally {
    sb.cleanup();
  }
});

test('render --write: an empty channel is REFUSED — never blanks a managed block by accident', () => {
  const sb = makeRenderSandbox({
    defaults: { version: 1, channels: { doc: { render: 'body', blocks: [], write: { file: 'AGENTS.md', block: 'vibe:rules' } } } },
    blocks: { 'a/one.md': BLOCK('a.one', 'S', 'BODY') },
  });
  const target = path.join(sb.dir, 'AGENTS.md');
  try {
    writeFileSync(target, 'above\n<!-- vibe:rules -->\nkeep me\n<!-- /vibe:rules -->\n');
    const { code, stderr } = runRender(sb.ctx, ['doc', '--write']);
    assertEqual(code, 1);
    assertIncludes(stderr, 'refusing to write an empty block');
    assertIncludes(readFileSync(target, 'utf8'), 'keep me');
  } finally {
    sb.cleanup();
  }
});

test('render --write: a failed write leaves no temp file behind', () => {
  const sb = makeRenderSandbox({
    defaults: {
      version: 1,
      channels: { doc: { render: 'body', blocks: ['a.one'], write: { file: 'nodir/AGENTS.md', block: 'vibe:rules' } } },
    },
    blocks: { 'a/one.md': BLOCK('a.one', 'S', 'BODY') },
  });
  try {
    const { code, stderr } = runRender(sb.ctx, ['doc', '--write']);
    assertEqual(code, 1);
    assertIncludes(stderr, 'cannot write');
    assert(!existsSync(path.join(sb.dir, 'nodir')), 'no directory is created for a failed write');
    assertEqual(
      readdirSync(sb.dir).filter((f) => f.includes('.tmp')),
      [],
      'a failed write must not strand a temp file',
    );
  } finally {
    sb.cleanup();
  }
});

test('render --write: a channel whose render errors still reports a non-zero exit', () => {
  const sb = makeRenderSandbox({
    defaults: {
      version: 1,
      channels: {
        doc: { render: 'body', blocks: ['a.one', 'a.missing'], write: { file: 'AGENTS.md', block: 'vibe:rules' } },
      },
    },
    blocks: { 'a/one.md': BLOCK('a.one', 'S', 'BODY') },
  });
  try {
    const { code, stderr } = runRender(sb.ctx, ['doc', '--write']);
    assertEqual(code, 1, 'the block is written, but a broken channel never reports success');
    assertIncludes(stderr, "no such block 'a.missing'");
    assertIncludes(readFileSync(path.join(sb.dir, 'AGENTS.md'), 'utf8'), 'BODY');
  } finally {
    sb.cleanup();
  }
});

// ---------------------------------------------------------------------------
// End to end through the real CLI
// ---------------------------------------------------------------------------

test('CLI: `vibe render <channel>` resolves the project from CLAUDE_PROJECT_DIR', () => {
  const sb = makeRenderSandbox({ defaults: DOC_DEFAULTS, blocks: { 'a/one.md': BLOCK('a.one', 'S', 'BODY') } });
  try {
    const result = runCli(['render', 'doc'], { env: { CLAUDE_PROJECT_DIR: sb.dir } });
    assertEqual(result.code, 0);
    assertIncludes(result.stdout, 'BODY');
  } finally {
    sb.cleanup();
  }
});

test('CLI: `vibe render --check` exits 1 on a broken tree (the CI tooth, end to end)', () => {
  const sb = makeRenderSandbox({ defaults: DOC_DEFAULTS });
  try {
    const result = runCli(['render', '--check'], { env: { CLAUDE_PROJECT_DIR: sb.dir } });
    assertEqual(result.code, 1);
    assertIncludes(result.stderr, "no such block 'a.one'");
  } finally {
    sb.cleanup();
  }
});

test('CLI: `vibe --help` lists render among the commands', () => {
  assertIncludes(runCli(['--help']).stdout, 'render');
});

test('CLI: `vibe render --check` on THIS repo is clean', () => {
  const result = runCli(['render', '--check'], { unsetEnv: ['CLAUDE_PROJECT_DIR'] });
  assertEqual(result.code, 0, `${result.stdout}${result.stderr}`);
  assertMatch(result.stdout, /0 error\(s\)/);
});

// ---------------------------------------------------------------------------
// review-hardening/12 — `render --write` resolves its target physically. A
// symlinked AGENTS.md (CLAUDE.md -> AGENTS.md is the house shape) is written
// THROUGH, so the link survives; a link that leaves the repo, dangles, or sits
// under an outside-linked directory is refused. Symlink targets are absolute.
// ---------------------------------------------------------------------------

const OUTSIDE_BYTES = '# outside\n\nuntouched\n';

function makeOutside() {
  const dir = mkTempRoot('vibe-render-outside-');
  const file = path.join(dir, 'outside.md');
  writeFileSync(file, OUTSIDE_BYTES);
  return { dir, file, cleanup: () => rmSync(dir, { recursive: true, force: true }) };
}

function docSandbox(file) {
  return makeRenderSandbox({
    defaults: { version: 1, channels: { doc: { render: 'body', blocks: ['a.one'], write: { file, block: 'vibe:rules' } } } },
    blocks: { 'a/one.md': BLOCK('a.one', 'S', 'BODY') },
  });
}

test('review-hardening/12: --write writes THROUGH an in-repo symlink and keeps the link', () => {
  const sb = docSandbox('AGENTS.md');
  try {
    mkdirSync(path.join(sb.dir, 'docs'));
    const real = path.join(sb.dir, 'docs', 'real.md');
    writeFileSync(real, '# real\n\nuser prose\n');
    const link = path.join(sb.dir, 'AGENTS.md');
    symlinkSync(real, link);
    const result = runRender(sb.ctx, ['doc', '--write']);
    assertEqual(result.code, 0, result.stderr);
    assert(lstatSync(link).isSymbolicLink(), 'the symlink was replaced by a regular file');
    assertEqual(readlinkSync(link), real);
    const text = readFileSync(real, 'utf8');
    assertIncludes(text, 'user prose');
    assertIncludes(text, 'BODY');
    assertEqual(readdirSync(path.join(sb.dir, 'docs')).filter((n) => n.endsWith('.tmp')), [], 'no temp file left');
  } finally {
    sb.cleanup();
  }
});

test('review-hardening/12: --write refuses a symlink to an outside file and leaves it unchanged', () => {
  const out = makeOutside();
  const sb = docSandbox('AGENTS.md');
  try {
    symlinkSync(out.file, path.join(sb.dir, 'AGENTS.md'));
    const result = runRender(sb.ctx, ['doc', '--write']);
    assertEqual(result.code, 1);
    assertIncludes(result.stderr, 'escapes');
    assertEqual(readFileSync(out.file, 'utf8'), OUTSIDE_BYTES, 'outside bytes changed');
    assertEqual(readdirSync(out.dir), ['outside.md'], 'something was created outside the repo');
  } finally {
    sb.cleanup();
    out.cleanup();
  }
});

test('review-hardening/12: --write never follows a link planted at a temp-file name', () => {
  // The temp name used to be `.<base>.<pid>.tmp`, opened with flag 'w': a repo
  // shipping that name as a link to an outside file had the outside file
  // truncated and rewritten. The planted link must survive and point where it did.
  const out = makeOutside();
  const sb = docSandbox('AGENTS.md');
  try {
    writeFileSync(path.join(sb.dir, 'AGENTS.md'), '# AGENTS.md\n\nuser prose\n');
    const planted = path.join(sb.dir, `.AGENTS.md.${process.pid}.tmp`);
    symlinkSync(out.file, planted);
    const result = runRender(sb.ctx, ['doc', '--write']);
    assertEqual(result.code, 0, result.stderr);
    assertEqual(readFileSync(out.file, 'utf8'), OUTSIDE_BYTES, 'outside bytes changed through the planted link');
    assert(lstatSync(planted).isSymbolicLink(), 'the planted link was consumed');
    assertEqual(readlinkSync(planted), out.file);
    const agents = path.join(sb.dir, 'AGENTS.md');
    assert(!lstatSync(agents).isSymbolicLink(), 'AGENTS.md became the planted link');
    assertIncludes(readFileSync(agents, 'utf8'), 'BODY');
    assertIncludes(readFileSync(agents, 'utf8'), 'user prose');
  } finally {
    sb.cleanup();
    out.cleanup();
  }
});

test('review-hardening/12: --write refuses a dangling symlink and creates nothing', () => {
  const out = makeOutside();
  const sb = docSandbox('AGENTS.md');
  try {
    const missing = path.join(out.dir, 'not-yet.md');
    symlinkSync(missing, path.join(sb.dir, 'AGENTS.md'));
    const result = runRender(sb.ctx, ['doc', '--write']);
    assertEqual(result.code, 1);
    assertIncludes(result.stderr, 'dangling');
    assert(!existsSync(missing), 'the dangling link target was created');
    assert(lstatSync(path.join(sb.dir, 'AGENTS.md')).isSymbolicLink(), 'the dangling link was replaced');
  } finally {
    sb.cleanup();
    out.cleanup();
  }
});

test('review-hardening/12: --write refuses a target under an in-repo directory linked outside', () => {
  const out = makeOutside();
  const sb = docSandbox('linked/AGENTS.md');
  try {
    symlinkSync(out.dir, path.join(sb.dir, 'linked'));
    const result = runRender(sb.ctx, ['doc', '--write']);
    assertEqual(result.code, 1);
    assertIncludes(result.stderr, 'escapes');
    assertEqual(readdirSync(out.dir), ['outside.md'], 'a file was written outside the repo');
  } finally {
    sb.cleanup();
    out.cleanup();
  }
});

test('review-hardening/12 control: a regular file and a missing file in an existing in-repo dir are written', () => {
  const sb = docSandbox('docs/NEW.md');
  try {
    mkdirSync(path.join(sb.dir, 'docs'));
    assertEqual(runRender(sb.ctx, ['doc', '--write']).code, 0);
    const file = path.join(sb.dir, 'docs', 'NEW.md');
    assert(!lstatSync(file).isSymbolicLink());
    assertIncludes(readFileSync(file, 'utf8'), 'BODY');
    writeFileSync(file, 'above\n<!-- vibe:rules -->\nstale\n<!-- /vibe:rules -->\n');
    assertEqual(runRender(sb.ctx, ['doc', '--write']).code, 0);
    assertIncludes(readFileSync(file, 'utf8'), 'BODY');
  } finally {
    sb.cleanup();
  }
});

test('review-hardening/12 control: a root reached through a symlinked temp dir writes normally', () => {
  const sb = docSandbox('AGENTS.md');
  const aliasDir = mkTempRoot('vibe-render-alias-');
  const alias = path.join(aliasDir, 'root');
  try {
    symlinkSync(sb.dir, alias);
    const vibeDir = path.join(alias, path.relative(sb.dir, sb.vibeDir));
    const ctx = { root: alias, vibeDir, skillsDir: path.dirname(vibeDir) };
    const result = runRender(ctx, ['doc', '--write']);
    assertEqual(result.code, 0, result.stderr);
    assertIncludes(readFileSync(path.join(sb.dir, 'AGENTS.md'), 'utf8'), 'BODY');
  } finally {
    rmSync(aliasDir, { recursive: true, force: true });
    sb.cleanup();
  }
});
