// engine/tests/content.test.mjs — the content & injection layer (content.mjs)
// plus the write half of the marker grammar it leans on (blocks.mjs).
//
// Three things are under test, in this order of importance:
//   1. RESOLUTION — shipped defaults, .vibe/blocks overrides, and vibe.json all
//      compose in the documented order, and a project can add, remove, reorder,
//      redefine, or silence anything without editing what ships.
//   2. DEGRADE — every layer may be absent, unreadable, or malformed. The hook
//      path (renderChannelSafe) must then contribute NOTHING and never throw:
//      injection failing shut is recoverable, a hook that throws wedges the
//      session.
//   3. THE LINT'S OWN POPULATION — `--check` on an empty tree must FAIL, not
//      pass quietly (.spec/lessons.md: "a check that examines nothing must fail
//      loudly"). A green that cannot tell "checked and clean" from "checked
//      nothing" is not a green, so the shipped tree is asserted to be non-empty
//      by structure, not by a hand-written count.

import { mkdirSync, writeFileSync, readFileSync, symlinkSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { test, assert, assertEqual, assertIncludes, assertMatch, makeHookSandbox } from './run.mjs';
import { extractBlock, stripBlock, renderBlock, upsertBlock } from '../blocks.mjs';
import {
  parseFrontmatter,
  parseBlockFile,
  loadContent,
  renderChannel,
  renderChannelSafe,
  checkContent,
} from '../content.mjs';

const REPO_ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..', '..', '..');
const REPO_VIBE_DIR = path.join(REPO_ROOT, 'flow');
const REPO_SKILLS_DIR = path.join(REPO_ROOT, '.agents', 'skills');

// A hook-shaped sandbox (root + installed vibeDir) plus a content tree. Every
// argument is optional, so a test can build exactly the layer it is about —
// including the "no content at all" case, which is the degrade baseline.
function makeContentSandbox({ defaults, project, blocks = {}, userBlocks = {}, cursor } = {}) {
  const sb = makeHookSandbox({ cursor });
  const contentDir = path.join(sb.vibeDir, 'content');
  if (defaults !== undefined) {
    mkdirSync(contentDir, { recursive: true });
    writeFileSync(
      path.join(contentDir, 'vibe.default.json'),
      typeof defaults === 'string' ? defaults : `${JSON.stringify(defaults, null, 2)}\n`,
    );
  }
  for (const [rel, body] of Object.entries(blocks)) {
    const file = path.join(contentDir, 'blocks', rel);
    mkdirSync(path.dirname(file), { recursive: true });
    writeFileSync(file, body);
  }
  for (const [rel, body] of Object.entries(userBlocks)) {
    const file = path.join(sb.dir, '.vibe', 'blocks', rel);
    mkdirSync(path.dirname(file), { recursive: true });
    writeFileSync(file, body);
  }
  if (project !== undefined) {
    writeFileSync(
      path.join(sb.dir, 'vibe.json'),
      typeof project === 'string' ? project : `${JSON.stringify(project, null, 2)}\n`,
    );
  }
  return { ...sb, ctx: { root: sb.dir, vibeDir: sb.vibeDir, skillsDir: sb.skillsDir } };
}

const BLOCK = (id, summary, body) =>
  `---\nid: ${id}\ntitle: ${id} title\nchannels: [user-prompt]\n---\n<!-- vibe:summary -->\n${summary}\n<!-- /vibe:summary -->\n\n${body ?? `${id} body prose`}\n`;

const DEFAULTS = (blocks, extra = {}) => ({
  version: 1,
  channels: { 'user-prompt': { render: 'summary', budget: 6, blocks }, ...extra },
});

// ---------------------------------------------------------------------------
// blocks.mjs — the write half of the one marker grammar
// ---------------------------------------------------------------------------

test('blocks: renderBlock emits a symmetric opener/closer pair around the body', () => {
  assertEqual(renderBlock('vibe:rules', 'one\ntwo'), '<!-- vibe:rules -->\none\ntwo\n<!-- /vibe:rules -->');
});

test('blocks: stripBlock removes the inclusive region and leaves the rest byte-identical', () => {
  const text = 'top\n<!-- x -->\nin\n<!-- /x -->\nbottom\n';
  assertEqual(stripBlock(text, 'x'), 'top\nbottom\n');
});

test('blocks: stripBlock on an UNCLOSED opener changes nothing (never swallows the tail)', () => {
  const text = 'top\n<!-- x -->\nin\nbottom\n';
  assertEqual(stripBlock(text, 'x'), text);
  assertEqual(extractBlock(text, 'x'), undefined, 'the reader refuses it too — same rule, one grammar');
});

test('blocks: upsertBlock appends when absent, replaces in place when present, and is idempotent', () => {
  const first = upsertBlock('# Doc\n\nuser prose\n', 'vibe:rules', 'A');
  assertEqual(first.action, 'appended');
  assert(first.changed);
  assertIncludes(first.text, '# Doc\n\nuser prose\n\n<!-- vibe:rules -->\nA\n<!-- /vibe:rules -->\n');

  const second = upsertBlock(first.text, 'vibe:rules', 'B');
  assertEqual(second.action, 'replaced');
  assertIncludes(second.text, 'user prose');
  assertIncludes(second.text, '<!-- vibe:rules -->\nB\n<!-- /vibe:rules -->');

  const third = upsertBlock(second.text, 'vibe:rules', 'B');
  assertEqual(third.changed, false, 're-rendering identical content must not rewrite the file');
});

test('blocks: upsertBlock preserves content on BOTH sides of an existing block', () => {
  const doc = 'above\n<!-- vibe:rules -->\nold\n<!-- /vibe:rules -->\nbelow\n';
  assertEqual(upsertBlock(doc, 'vibe:rules', 'new').text, 'above\n<!-- vibe:rules -->\nnew\n<!-- /vibe:rules -->\nbelow\n');
});

// ---------------------------------------------------------------------------
// Block authoring format
// ---------------------------------------------------------------------------

test('content: parseFrontmatter reads scalars, inline lists, and dash lists', () => {
  const { data, rest } = parseFrontmatter(
    '---\nid: a.b\ntitle: "Quoted Title"\nchannels: [x, y]\ntags:\n  - one\n  - two\n---\nbody\n',
  );
  assertEqual(data.id, 'a.b');
  assertEqual(data.title, 'Quoted Title');
  assertEqual(data.channels, ['x', 'y']);
  assertEqual(data.tags, ['one', 'two']);
  assertEqual(rest, 'body\n');
});

test('content: parseFrontmatter leaves a file with no frontmatter untouched', () => {
  const { data, rest } = parseFrontmatter('plain body\n');
  assertEqual(data, {});
  assertEqual(rest, 'plain body\n');
});

test('content: a block file carries two verbosities — summary block and the remaining body', () => {
  const block = parseBlockFile(BLOCK('a.b', 'terse line', 'long prose\n\nsecond para'));
  assertEqual(block.id, 'a.b');
  assertEqual(block.summary, 'terse line');
  assertEqual(block.body, 'long prose\n\nsecond para');
  assertEqual(block.channels, ['user-prompt']);
});

test('content: no summary block -> the first paragraph becomes the summary', () => {
  const block = parseBlockFile('---\nid: a.b\n---\nfirst para line\nstill first\n\nsecond para\n');
  assertEqual(block.summary, 'first para line\nstill first');
  assertIncludes(block.body, 'second para');
});

test('content: a summary-only block falls back to the summary for its body (and vice versa)', () => {
  const summaryOnly = parseBlockFile('---\nid: a\n---\n<!-- vibe:summary -->\nonly\n<!-- /vibe:summary -->\n');
  assertEqual(summaryOnly.body, 'only');
  const bodyOnly = parseBlockFile('---\nid: b\n---\njust prose\n');
  assertEqual(bodyOnly.summary, 'just prose');
});

// ---------------------------------------------------------------------------
// Resolution: shipped -> .vibe/blocks -> vibe.json
// ---------------------------------------------------------------------------

test('content: shipped defaults alone compose a channel', () => {
  const sb = makeContentSandbox({ defaults: DEFAULTS(['a.one']), blocks: { 'a/one.md': BLOCK('a.one', 'A summary') } });
  try {
    const result = renderChannel('user-prompt', sb.ctx);
    assertEqual(result.text, 'A summary\n');
    assertEqual(result.blocks, ['a.one']);
    assertEqual(result.errors, []);
  } finally {
    sb.cleanup();
  }
});

test('content: vibe.json `blocks` REPLACES the shipped list; `add`/`remove` edit it', () => {
  const blocks = {
    'a/one.md': BLOCK('a.one', 'A'),
    'a/two.md': BLOCK('a.two', 'B'),
    'a/three.md': BLOCK('a.three', 'C'),
  };
  const replaced = makeContentSandbox({
    defaults: DEFAULTS(['a.one', 'a.two']),
    blocks,
    project: { channels: { 'user-prompt': { blocks: ['a.three'] } } },
  });
  try {
    assertEqual(renderChannel('user-prompt', replaced.ctx).text, 'C\n');
  } finally {
    replaced.cleanup();
  }

  const edited = makeContentSandbox({
    defaults: DEFAULTS(['a.one', 'a.two']),
    blocks,
    project: { channels: { 'user-prompt': { add: ['a.three'], remove: ['a.one'] } } },
  });
  try {
    const result = renderChannel('user-prompt', edited.ctx);
    assertEqual(result.blocks, ['a.two', 'a.three'], 'order follows the resolved list, not the file tree');
    assertEqual(result.text, 'B\nC\n');
  } finally {
    edited.cleanup();
  }
});

test('content: `enabled: false` silences a shipped block without removing it from any channel', () => {
  const sb = makeContentSandbox({
    defaults: DEFAULTS(['a.one', 'a.two']),
    blocks: { 'a/one.md': BLOCK('a.one', 'A'), 'a/two.md': BLOCK('a.two', 'B') },
    project: { blocks: { 'a.one': { enabled: false } } },
  });
  try {
    const result = renderChannel('user-prompt', sb.ctx);
    assertEqual(result.text, 'B\n');
    assertEqual(result.errors, [], 'a deliberately disabled block is not an error');
  } finally {
    sb.cleanup();
  }
});

test('content: vibe.json can DEFINE a block inline and compose it', () => {
  const sb = makeContentSandbox({
    defaults: DEFAULTS([]),
    project: {
      channels: { 'user-prompt': { add: ['team.rule'] } },
      blocks: { 'team.rule': { title: 'Team rule', summary: 'never force-push main' } },
    },
  });
  try {
    assertEqual(renderChannel('user-prompt', sb.ctx).text, 'never force-push main\n');
  } finally {
    sb.cleanup();
  }
});

test('content: a .vibe/blocks file overrides a shipped block of the same id', () => {
  const sb = makeContentSandbox({
    defaults: DEFAULTS(['a.one']),
    blocks: { 'a/one.md': BLOCK('a.one', 'shipped text') },
    userBlocks: { 'a/one.md': BLOCK('a.one', 'project text') },
  });
  try {
    assertEqual(renderChannel('user-prompt', sb.ctx).text, 'project text\n');
  } finally {
    sb.cleanup();
  }
});

test('content: a vibe.json block may point at a file, resolved relative to the project root', () => {
  const sb = makeContentSandbox({
    defaults: DEFAULTS([]),
    project: {
      channels: { 'user-prompt': { add: ['team.rule'] } },
      blocks: { 'team.rule': { file: 'rules/team.md' } },
    },
  });
  try {
    const file = path.join(sb.dir, 'rules', 'team.md');
    mkdirSync(path.dirname(file), { recursive: true });
    writeFileSync(file, BLOCK('team.rule', 'from a file'));
    assertEqual(renderChannel('user-prompt', sb.ctx).text, 'from a file\n');
  } finally {
    sb.cleanup();
  }
});

test('content: a project may declare a channel the shipped defaults never define', () => {
  const sb = makeContentSandbox({
    defaults: DEFAULTS([]),
    project: {
      channels: { 'code-review': { render: 'body', blocks: ['team.rule'] } },
      blocks: { 'team.rule': { title: 'Team rule', body: 'no silent catch' } },
    },
  });
  try {
    assertEqual(renderChannel('code-review', sb.ctx).text, '## Team rule\n\nno silent catch\n');
  } finally {
    sb.cleanup();
  }
});

// ---------------------------------------------------------------------------
// Render modes and budgets
// ---------------------------------------------------------------------------

test('content: body mode adds the title heading and a blank line between blocks; summary mode does neither', () => {
  const blocks = { 'a/one.md': BLOCK('a.one', 'S1', 'B1'), 'a/two.md': BLOCK('a.two', 'S2', 'B2') };
  const sb = makeContentSandbox({
    defaults: DEFAULTS(['a.one', 'a.two'], { doc: { render: 'body', blocks: ['a.one', 'a.two'] } }),
    blocks,
  });
  try {
    assertEqual(renderChannel('user-prompt', sb.ctx).text, 'S1\nS2\n');
    assertEqual(
      renderChannel('doc', sb.ctx).text,
      '## a.one title\n\nB1\n\n## a.two title\n\nB2\n',
    );
  } finally {
    sb.cleanup();
  }
});

test('content: a channel over its line budget is an ERROR (the transcript pays that rent every turn)', () => {
  const sb = makeContentSandbox({
    defaults: { version: 1, channels: { 'user-prompt': { render: 'summary', budget: 2, blocks: ['a.one'] } } },
    blocks: { 'a/one.md': BLOCK('a.one', 'l1\nl2\nl3') },
  });
  try {
    const result = renderChannel('user-prompt', sb.ctx);
    assertEqual(result.errors.length, 1);
    assertIncludes(result.errors[0], 'exceeds the 2-line budget');
    assertIncludes(result.text, 'l3', 'the text still renders — the budget reports, it does not truncate');
  } finally {
    sb.cleanup();
  }
});

test('content: an unknown block id in a channel is an error, and the rest of the channel still renders', () => {
  const sb = makeContentSandbox({
    defaults: DEFAULTS(['a.missing', 'a.one']),
    blocks: { 'a/one.md': BLOCK('a.one', 'A') },
  });
  try {
    const result = renderChannel('user-prompt', sb.ctx);
    assertEqual(result.text, 'A\n');
    assertEqual(result.errors.length, 1);
    assertIncludes(result.errors[0], "no such block 'a.missing'");
  } finally {
    sb.cleanup();
  }
});

test('content: an unknown channel reports, and renders nothing', () => {
  const sb = makeContentSandbox({ defaults: DEFAULTS([]) });
  try {
    const result = renderChannel('nope', sb.ctx);
    assertEqual(result.text, '');
    assertIncludes(result.errors[0], "unknown channel 'nope'");
  } finally {
    sb.cleanup();
  }
});

test('content: a disabled channel renders nothing at all', () => {
  const sb = makeContentSandbox({
    defaults: DEFAULTS(['a.one']),
    blocks: { 'a/one.md': BLOCK('a.one', 'A') },
    project: { channels: { 'user-prompt': { enabled: false } } },
  });
  try {
    assertEqual(renderChannel('user-prompt', sb.ctx).text, '');
  } finally {
    sb.cleanup();
  }
});

// ---------------------------------------------------------------------------
// Placeholders
// ---------------------------------------------------------------------------

test('content: machine placeholders interpolate from the live cursor and state machine', () => {
  const sb = makeContentSandbox({
    cursor: { flow: 'feature', phase: 'impl', feature: 'js-core', updated: '2026-01-01T00:00:00Z' },
    defaults: DEFAULTS(['a.one']),
    blocks: { 'a/one.md': BLOCK('a.one', 'state={{state}} feature={{feature}} next={{next}} writes={{writes}}') },
  });
  try {
    const result = renderChannel('user-prompt', sb.ctx);
    assertEqual(result.errors, []);
    assertEqual(result.text, 'state=feature.impl feature=js-core next=feature.verify, idle writes=src/**, tests/**\n');
  } finally {
    sb.cleanup();
  }
});

test('content: {{feature}} with no feature in the cursor keeps the literal <feature> placeholder (orders parity)', () => {
  const sb = makeContentSandbox({
    defaults: DEFAULTS(['a.one']),
    blocks: { 'a/one.md': BLOCK('a.one', 'f={{feature}}') },
  });
  try {
    assertEqual(renderChannel('user-prompt', sb.ctx).text, 'f=<feature>\n');
  } finally {
    sb.cleanup();
  }
});

test('content: {{orders}} composes the flow orders the inject hook already resolves', () => {
  const sb = makeContentSandbox({
    defaults: DEFAULTS(['a.one']),
    blocks: { 'a/one.md': BLOCK('a.one', 'orders: {{orders}}') },
  });
  try {
    assertMatch(renderChannel('user-prompt', sb.ctx).text, /^orders: state=idle/);
  } finally {
    sb.cleanup();
  }
});

test('content: {{lessons:TAG}} pulls tagged lesson titles from the configured lessons file', () => {
  const sb = makeContentSandbox({
    defaults: {
      version: 1,
      sources: { lessons: '.spec/lessons.md' },
      channels: { 'user-prompt': { render: 'summary', blocks: ['a.one'] } },
    },
    blocks: { 'a/one.md': BLOCK('a.one', '{{lessons:inject}}') },
  });
  try {
    writeFileSync(
      path.join(sb.dir, '.spec', 'lessons.md'),
      '### First lesson\n**Tags:** inject, other\n\n### Second lesson\n**Tags:** unrelated\n',
    );
    assertEqual(renderChannel('user-prompt', sb.ctx).text, '- First lesson\n');
  } finally {
    sb.cleanup();
  }
});

test('content: a custom placeholder resolves from vibe.json; a built-in cannot be shadowed', () => {
  const sb = makeContentSandbox({
    defaults: DEFAULTS(['a.one']),
    blocks: { 'a/one.md': BLOCK('a.one', 'team={{team}} state={{state}}') },
    project: { placeholders: { team: 'platform', state: 'SHADOWED' } },
  });
  try {
    assertEqual(renderChannel('user-prompt', sb.ctx).text, 'team=platform state=idle\n');
  } finally {
    sb.cleanup();
  }
});

test('content: an unknown placeholder stays LITERAL and is reported (a typo must be visible, not blank)', () => {
  const sb = makeContentSandbox({
    defaults: DEFAULTS(['a.one']),
    blocks: { 'a/one.md': BLOCK('a.one', 'x={{nope}}') },
  });
  try {
    const result = renderChannel('user-prompt', sb.ctx);
    assertEqual(result.text, 'x={{nope}}\n');
    assertIncludes(result.errors.join('\n'), 'unresolved placeholder {{nope}}');
  } finally {
    sb.cleanup();
  }
});

// ---------------------------------------------------------------------------
// Degrade — every layer may be missing or broken
// ---------------------------------------------------------------------------

test('content: NO content tree at all -> no channels, no throw, empty render', () => {
  const sb = makeContentSandbox({});
  try {
    const content = loadContent(sb.dir, sb.vibeDir);
    assertEqual(Object.keys(content.channels), []);
    assertEqual(content.blocks.size, 0);
    assertEqual(renderChannelSafe('user-prompt', sb.ctx), '');
  } finally {
    sb.cleanup();
  }
});

test('content: a MALFORMED vibe.json is reported but never fatal — shipped defaults still render', () => {
  const sb = makeContentSandbox({
    defaults: DEFAULTS(['a.one']),
    blocks: { 'a/one.md': BLOCK('a.one', 'A') },
    project: '{ this is not json',
  });
  try {
    const content = loadContent(sb.dir, sb.vibeDir);
    assertEqual(content.errors.length, 1);
    assertIncludes(content.errors[0], 'vibe.json');
    assertEqual(renderChannelSafe('user-prompt', sb.ctx), 'A\n', 'a broken user layer must not silence the shipped one');
  } finally {
    sb.cleanup();
  }
});

test('content: a vibe.json that is an ARRAY (not an object) is rejected, not merged', () => {
  const sb = makeContentSandbox({ defaults: DEFAULTS([]), project: '[1, 2]' });
  try {
    const content = loadContent(sb.dir, sb.vibeDir);
    assertIncludes(content.errors.join('\n'), 'is not a JSON object');
  } finally {
    sb.cleanup();
  }
});

test('adversarial: a prototype key is not a channel — no inherited function is rendered', () => {
  const sb = makeContentSandbox({ defaults: DEFAULTS(['a.one']), blocks: { 'a/one.md': BLOCK('a.one', 'A') } });
  try {
    for (const key of ['constructor', 'toString', 'hasOwnProperty', '__proto__']) {
      const result = renderChannel(key, sb.ctx);
      assertEqual(result.text, '', `${key} must not resolve to a channel`);
      assertIncludes(result.errors.join('\n'), `unknown channel '${key}'`);
    }
  } finally {
    sb.cleanup();
  }
});

test('adversarial: a CALLER-SUPPLIED plain-object channels map still rejects prototype keys', () => {
  // renderChannel(name, ctx, content) is a supported signature (checkContent and
  // render.mjs both pass a preloaded content object), so the null-prototype map
  // loadContent builds is not the only input this has to survive. Without the
  // hasOwnProperty guard this resolves `constructor` to a Function and crashes
  // on `channel.blocks`.
  const content = { channels: { doc: { render: 'summary', blocks: [] } }, blocks: new Map(), placeholders: {}, sources: {}, errors: [] };
  for (const key of ['constructor', 'toString', 'valueOf']) {
    const result = renderChannel(key, { root: undefined, vibeDir: undefined, skillsDir: undefined }, content);
    assertEqual(result.text, '');
    assertIncludes(result.errors.join('\n'), `unknown channel '${key}'`);
  }
});

test('adversarial: a "__proto__" channel is stored as data, not applied as a prototype', () => {
  // JSON.parse yields an OWN "__proto__" key, and assigning that onto an
  // ordinary object REPLACES the object's prototype instead of storing a
  // channel: the channel silently vanishes and every unrelated lookup
  // (`channels.blocks`) starts resolving against the attacker's object. The
  // channels map has no prototype for exactly this reason.
  const sb = makeContentSandbox({
    defaults: DEFAULTS([]),
    project: '{ "channels": { "__proto__": { "blocks": ["a.one"], "render": "summary" } } }',
  });
  try {
    const content = loadContent(sb.dir, sb.vibeDir);
    assert(
      Object.prototype.hasOwnProperty.call(content.channels, '__proto__'),
      'the key must be stored as ordinary channel data',
    );
    assertEqual(content.channels.blocks, undefined, 'no unrelated lookup may resolve through it');
    assertEqual({}.blocks, undefined, 'and Object.prototype is untouched');
  } finally {
    sb.cleanup();
  }
});

test('adversarial: a project `sources` key cannot shadow the loader’s own bookkeeping', () => {
  const sb = makeContentSandbox({
    defaults: DEFAULTS([]),
    project: { sources: { defaults: 'HIJACKED', project: 'HIJACKED' } },
  });
  try {
    const content = loadContent(sb.dir, sb.vibeDir);
    assertIncludes(content.origins.defaults, 'vibe.default.json');
    assertIncludes(content.origins.project, 'vibe.json');
    assertEqual(content.sources.defaults, 'HIJACKED', 'config data is still readable, just not authoritative');
  } finally {
    sb.cleanup();
  }
});

test('adversarial: a symlink cycle inside a blocks tree terminates instead of hanging the hook', () => {
  const sb = makeContentSandbox({ defaults: DEFAULTS(['a.one']), blocks: { 'a/one.md': BLOCK('a.one', 'A') } });
  try {
    const loop = path.join(sb.vibeDir, 'content', 'blocks', 'a', 'loop');
    symlinkSync(path.join(sb.vibeDir, 'content', 'blocks'), loop, 'dir');
    const content = loadContent(sb.dir, sb.vibeDir);
    assert(content.blocks.has('a.one'), 'the real block is still found');
    assertEqual(renderChannel('user-prompt', sb.ctx, content).text, 'A\n');
  } finally {
    sb.cleanup();
  }
});

test('content: renderChannelSafe swallows a hostile ctx (no root, no vibeDir) and returns empty', () => {
  assertEqual(renderChannelSafe('user-prompt', {}), '');
  assertEqual(renderChannelSafe('user-prompt', { root: 12, vibeDir: null }), '');
});

test('content: a channel whose blocks all resolve empty renders nothing (no stray blank line)', () => {
  const sb = makeContentSandbox({
    defaults: DEFAULTS(['a.one']),
    blocks: { 'a/one.md': '---\nid: a.one\n---\n' },
  });
  try {
    const result = renderChannel('user-prompt', sb.ctx);
    assertEqual(result.text, '');
    assertIncludes(result.warnings.join('\n'), 'has no summary content');
  } finally {
    sb.cleanup();
  }
});

// ---------------------------------------------------------------------------
// The lint, and its own population
// ---------------------------------------------------------------------------

test('checkContent: an EMPTY content tree FAILS — absence of findings is not a pass', () => {
  const sb = makeContentSandbox({});
  try {
    const { errors } = checkContent(sb.ctx);
    assertIncludes(errors.join('\n'), 'no content blocks found');
    assertIncludes(errors.join('\n'), 'no channels configured');
  } finally {
    sb.cleanup();
  }
});

test('checkContent: warns when a block is composed into a channel its frontmatter does not declare', () => {
  const sb = makeContentSandbox({
    defaults: DEFAULTS(['a.one'], { doc: { render: 'body', blocks: ['a.one'] } }),
    blocks: { 'a/one.md': BLOCK('a.one', 'A') }, // declares channels: [user-prompt]
  });
  try {
    const { warnings } = checkContent(sb.ctx);
    assertIncludes(warnings.join('\n'), "composed into 'doc'");
    assertIncludes(warnings.join('\n'), 'composed into 2 channels');
  } finally {
    sb.cleanup();
  }
});

// ---------------------------------------------------------------------------
// The shipped tree itself — the defaults this repo ships must actually work
// ---------------------------------------------------------------------------

const repoCtx = { root: REPO_ROOT, vibeDir: REPO_VIBE_DIR, skillsDir: REPO_SKILLS_DIR };

test('shipped: the repo content tree lints clean (this IS the CI tooth, run as a test)', () => {
  const { errors, blockCount } = checkContent(repoCtx);
  assertEqual(errors, [], errors.join('\n'));
  assert(blockCount >= 3, `expected the shipped block library to be non-empty, got ${blockCount}`);
});

test('shipped: the three default rulesets are composed into their channels', () => {
  const content = loadContent(REPO_ROOT, REPO_VIBE_DIR);
  assert(content.channels['user-prompt'].blocks.includes('style.ste100'), 'per-turn style rule');
  assert(content.channels['agents-md'].blocks.includes('delegation.subagents'), 'subagent tiers');
  assert(content.channels['agents-md'].blocks.includes('delegation.workflows'), 'dynamic workflows');
});

test('shipped: every channel renders inside its own budget', () => {
  const content = loadContent(REPO_ROOT, REPO_VIBE_DIR);
  for (const name of Object.keys(content.channels)) {
    const result = renderChannel(name, repoCtx, content);
    assertEqual(result.errors, [], `${name}: ${result.errors.join('; ')}`);
  }
});

test("shipped: AGENTS.md's vibe:rules block is in sync with the agents-md channel", () => {
  const rendered = renderChannel('agents-md', repoCtx).text.replace(/\n+$/, '');
  const inFile = extractBlock(readFileSync(path.join(REPO_ROOT, 'AGENTS.md'), 'utf8'), 'vibe:rules');
  assert(inFile !== undefined, 'AGENTS.md carries no vibe:rules block — run: vibe render agents-md --write');
  assertIncludes(inFile, rendered, 'AGENTS.md is stale — run: vibe render agents-md --write');
});

// ---------------------------------------------------------------------------
// Hook integration — the two channels that ride on hooks
// ---------------------------------------------------------------------------

test('hooks: the per-turn channel is injected AFTER the orders, and only when content exists', async () => {
  const { runInjectHook } = await import('../commands/hook.mjs');

  // Baseline: an install with no content tree emits exactly what the
  // pre-content-layer hook emitted — the degrade that keeps every parity
  // assertion in hook.test.mjs honest.
  const bare = makeHookSandbox();
  let baseline;
  try {
    baseline = runInjectHook(bare.dir, bare.vibeDir, bare.skillsDir, { spawnInfer: () => ({ error: null, stdout: '' }) });
    assertMatch(baseline.stdout, /^state=idle/);
    assertEqual(baseline.stdout.split('\n').filter(Boolean).length, 1, 'no content tree -> orders only');
  } finally {
    bare.cleanup();
  }

  const sb = makeContentSandbox({
    defaults: DEFAULTS(['a.one']),
    blocks: { 'a/one.md': BLOCK('a.one', 'STYLE RULE') },
  });
  try {
    const result = runInjectHook(sb.dir, sb.vibeDir, sb.skillsDir, { spawnInfer: () => ({ error: null, stdout: '' }) });
    const lines = result.stdout.split('\n').filter(Boolean);
    assertMatch(lines[0], /^state=idle/, 'orders stay first — the turn imperative outranks standing rules');
    assertEqual(lines[1], 'STYLE RULE');
  } finally {
    sb.cleanup();
  }
});

test('hooks: the per-turn channel lands BEFORE the warnings drain (rules are standing, warns are events)', async () => {
  const { runInjectHook } = await import('../commands/hook.mjs');
  const sb = makeContentSandbox({
    defaults: DEFAULTS(['a.one']),
    blocks: { 'a/one.md': BLOCK('a.one', 'STYLE RULE') },
  });
  try {
    writeFileSync(sb.warnLogPath, 'guard: something (warn-only)\n');
    const result = runInjectHook(sb.dir, sb.vibeDir, sb.skillsDir, { spawnInfer: () => ({ error: null, stdout: '' }) });
    const lines = result.stdout.split('\n').filter(Boolean);
    assertEqual(lines[1], 'STYLE RULE');
    assertMatch(lines[2], /^vibe-warn: /);
  } finally {
    sb.cleanup();
  }
});

test('hooks: the session-start channel is appended after the doctrine block', async () => {
  const { runDoctrineHook } = await import('../commands/hook.mjs');
  const sb = makeContentSandbox({
    defaults: { version: 1, channels: { 'session-start': { render: 'summary', blocks: ['a.one'] } } },
    blocks: { 'a/one.md': BLOCK('a.one', 'SESSION RULE') },
  });
  try {
    const result = runDoctrineHook(sb.vibeDir, sb.skillsDir, sb.dir);
    assertEqual(result.code, 0);
    assertMatch(result.stdout, /^vibe flow — working model/, 'the doctrine block still comes first');
    assertEqual(result.stdout.trimEnd().split('\n').pop(), 'SESSION RULE');
  } finally {
    sb.cleanup();
  }
});

test('hooks: a doctrine call with NO content tree is byte-identical to the ported command', async () => {
  const { runDoctrineHook } = await import('../commands/hook.mjs');
  const { runDoctrine } = await import('../commands/doctrine.mjs');
  const sb = makeHookSandbox();
  try {
    assertEqual(
      runDoctrineHook(sb.vibeDir, sb.skillsDir, sb.dir).stdout,
      runDoctrine(sb.vibeDir, sb.skillsDir).stdout,
    );
  } finally {
    sb.cleanup();
  }
});
