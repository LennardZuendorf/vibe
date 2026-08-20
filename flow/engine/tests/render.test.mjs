// engine/tests/render.test.mjs — `vibe render`, the content layer's CLI.
//
// Split from content.test.mjs on purpose: that file owns RESOLUTION (what a
// channel composes to), this one owns the COMMAND CONTRACT — exit codes, the
// document write, and idempotence. The document write is the part with teeth:
// it edits a file that holds user prose outside the managed block, so "never
// clobber, never rewrite when unchanged, refuse rather than write garbage" are
// assertions, not intentions.

import { mkdirSync, writeFileSync, readFileSync, existsSync, readdirSync } from 'node:fs';
import path from 'node:path';
import { test, assert, assertEqual, assertIncludes, assertMatch, makeHookSandbox, runCli } from './run.mjs';
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
