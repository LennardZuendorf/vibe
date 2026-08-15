// engine/content.mjs — the content & injection layer (content-layer feature).
//
// Every injected sentence is authored ONCE as a block, and composed into a
// CHANNEL. A channel is an injection surface (per-turn prompt, session start,
// the AGENTS.md managed rules block); a block is a rule with two verbosities
// (a terse `summary` for prompt channels, a prose `body` for document
// channels). What ships is data — `content/vibe.default.json` plus
// `content/blocks/**.md` — and every project may override, extend, reorder, or
// disable any of it from a single root `vibe.json`.
//
// Resolution order (later wins, never mutates the earlier layer):
//   1. shipped   <vibeDir>/content/vibe.default.json + <vibeDir>/content/blocks/**.md
//   2. project   <root>/.vibe/blocks/**.md          (block overrides by id)
//   3. project   <root>/vibe.json                    (channels, block fields, placeholders)
//
// Contract, matching every other engine module: NEVER throws. A missing,
// unreadable, or malformed layer degrades to "that layer contributed nothing"
// and is reported through `errors`, which `vibe render --check` turns into a
// non-zero exit while the hooks keep going. Injection failing shut (no rules)
// is recoverable; a hook that throws wedges the session.

import fs from 'node:fs';
import path from 'node:path';
import { extractBlock, stripBlock } from './blocks.mjs';
import { readJson as readJsonFile } from './json.mjs';
import { readCursor } from './cursor.mjs';
import { loadMachine, stateOf } from './machine.mjs';
import { loadPolicy, renderInvariants } from './policy.mjs';
import { runOrders } from './commands/orders.mjs';

export const CONFIG_BASENAME = 'vibe.json';
export const DEFAULT_CONFIG_RELPATH = path.join('content', 'vibe.default.json');
export const SHIPPED_BLOCKS_RELPATH = path.join('content', 'blocks');
export const USER_BLOCKS_RELPATH = path.join('.vibe', 'blocks');
export const SUMMARY_BLOCK_ID = 'vibe:summary';

// Render modes. `summary` is for prompt channels — terse lines, no headings,
// one newline between blocks. `body` is for document channels — prose with a
// `## <title>` heading and a blank line between blocks.
const MODES = ['summary', 'body'];
const DEFAULT_MODE = 'summary';

// ---------------------------------------------------------------------------
// Small file helpers — every one of them fails soft.
// ---------------------------------------------------------------------------

function readText(file) {
  try {
    return fs.readFileSync(file, 'utf8');
  } catch {
    return undefined;
  }
}

// Config layers go through json.mjs's readJson (the one JSON reader) and are
// classified here: an ABSENT layer is normal — a project with no vibe.json is
// the common case — while a present-but-malformed one is an error the lint
// surfaces and the hooks step over.
function readConfig(file, errors, label) {
  let value;
  try {
    value = readJsonFile(file);
  } catch (err) {
    if (err && err.code === 'ENOENT') return undefined;
    errors.push(`${label}: ${file} is not readable as JSON (${err && err.message ? err.message : err})`);
    return undefined;
  }
  if (!value || typeof value !== 'object' || Array.isArray(value)) {
    errors.push(`${label}: ${file} is not a JSON object`);
    return undefined;
  }
  return value;
}

// Deterministic depth-first walk, sorted at every level, so composition order
// and `--list` output never depend on filesystem iteration order.
//
// statSync (not the dirent) so a symlinked directory is followed like a real
// one — which is exactly why the depth cap exists: a symlink cycle inside a
// blocks tree would otherwise spin forever, and this walk runs inside a hook.
// Losing blocks nested 8 deep is recoverable; a hung hook is not.
const MAX_BLOCK_DEPTH = 8;

function walkMarkdown(dir, depth = 0) {
  if (depth > MAX_BLOCK_DEPTH) return [];
  let entries;
  try {
    entries = fs.readdirSync(dir, { withFileTypes: true });
  } catch {
    return [];
  }
  const names = entries.map((e) => e.name).sort();
  const out = [];
  for (const name of names) {
    const full = path.join(dir, name);
    let stat;
    try {
      stat = fs.statSync(full);
    } catch {
      continue;
    }
    if (stat.isDirectory()) out.push(...walkMarkdown(full, depth + 1));
    else if (stat.isFile() && name.endsWith('.md')) out.push(full);
  }
  return out;
}

// ---------------------------------------------------------------------------
// Block files
// ---------------------------------------------------------------------------
//
//   ---
//   id: delegation.subagents          <- optional; defaults to the path-derived id
//   title: Delegating to sub-agents
//   channels: [agents-md]
//   ---
//   <!-- vibe:summary -->
//   terse form, used by prompt channels
//   <!-- /vibe:summary -->
//
//   prose form, used by document channels
//
// The summary block reuses blocks.mjs's marker grammar rather than inventing a
// second one (its missing-closer bug fix comes along for free). A file with no
// summary block falls back to its first paragraph; a file with no body falls
// back to its summary. One author point, two verbosities.

// A frontmatter subset, deliberately: `key: scalar`, `key: [a, b]`, and a
// `- item` list on the following lines. Enough for id/title/channels, and it
// keeps the zero-runtime-dependency contract (no YAML parser).
export function parseFrontmatter(text) {
  const lines = text.split('\n');
  if (lines[0] !== '---') return { data: {}, rest: text };
  const end = lines.indexOf('---', 1);
  if (end === -1) return { data: {}, rest: text };

  const data = {};
  let listKey;
  for (const raw of lines.slice(1, end)) {
    const itemMatch = /^\s*-\s+(.*)$/.exec(raw);
    if (itemMatch && listKey) {
      data[listKey].push(unquote(itemMatch[1].trim()));
      continue;
    }
    const kv = /^([A-Za-z0-9_-]+):\s*(.*)$/.exec(raw);
    if (!kv) continue;
    const [, key, rawValue] = kv;
    const value = rawValue.trim();
    if (value === '') {
      listKey = key;
      data[key] = [];
      continue;
    }
    listKey = undefined;
    if (value.startsWith('[') && value.endsWith(']')) {
      const inner = value.slice(1, -1).trim();
      data[key] = inner === '' ? [] : inner.split(',').map((s) => unquote(s.trim()));
    } else {
      data[key] = unquote(value);
    }
  }
  return { data, rest: lines.slice(end + 1).join('\n') };
}

function unquote(s) {
  if (s.length >= 2 && ((s[0] === '"' && s.endsWith('"')) || (s[0] === "'" && s.endsWith("'")))) {
    return s.slice(1, -1);
  }
  return s;
}

function firstParagraph(text) {
  const trimmed = text.trim();
  if (!trimmed) return '';
  const [para] = trimmed.split(/\n\s*\n/);
  return para.trim();
}

function idFromPath(file, blocksDir) {
  const rel = path.relative(blocksDir, file).replace(/\.md$/, '');
  return rel.split(path.sep).join('.');
}

export function parseBlockFile(text, { id: fallbackId, source } = {}) {
  const { data, rest } = parseFrontmatter(text);
  const summaryRaw = extractBlock(rest, SUMMARY_BLOCK_ID);
  // The body is everything the summary block is not. stripBlock leaves the
  // text untouched when the pair is absent or unclosed — which is right: an
  // unclosed opener is a mistake that should stay visible in the body rather
  // than swallow the rest of the file.
  const body = stripBlock(rest, SUMMARY_BLOCK_ID).trim();
  const summary = (summaryRaw === undefined ? firstParagraph(body) : summaryRaw).trim();
  const channels = Array.isArray(data.channels)
    ? data.channels
    : typeof data.channels === 'string' && data.channels
      ? [data.channels]
      : [];
  return {
    id: typeof data.id === 'string' && data.id ? data.id : fallbackId,
    title: typeof data.title === 'string' ? data.title : '',
    channels,
    summary: summary || body,
    body: body || summary,
    enabled: data.enabled !== 'false' && data.enabled !== false,
    source,
  };
}

function loadBlocksDir(dir, blocks) {
  for (const file of walkMarkdown(dir)) {
    const text = readText(file);
    if (text === undefined) continue;
    const block = parseBlockFile(text, { id: idFromPath(file, dir), source: file });
    if (!block.id) continue;
    // Later layers override earlier ones wholesale, by id.
    blocks.set(block.id, block);
  }
}

// A `blocks` entry in vibe.json patches (or defines) a block by id. `file` is
// read relative to the config's own directory, so a project can keep long
// rules in files without adopting the .vibe/blocks/ convention.
function applyConfigBlocks(configBlocks, blocks, configDir, errors) {
  if (!configBlocks || typeof configBlocks !== 'object' || Array.isArray(configBlocks)) return;
  for (const id of Object.keys(configBlocks).sort()) {
    const entry = configBlocks[id];
    if (!entry || typeof entry !== 'object' || Array.isArray(entry)) {
      errors.push(`blocks.${id}: expected an object`);
      continue;
    }
    const base = blocks.get(id) ?? {
      id,
      title: '',
      channels: [],
      summary: '',
      body: '',
      enabled: true,
      source: undefined,
    };
    let fromFile;
    if (typeof entry.file === 'string' && entry.file) {
      const file = path.resolve(configDir, entry.file);
      const text = readText(file);
      if (text === undefined) errors.push(`blocks.${id}: file not readable: ${file}`);
      else fromFile = parseBlockFile(text, { id, source: file });
    }
    const merged = { ...base, ...(fromFile ?? {}) };
    if (typeof entry.title === 'string') merged.title = entry.title;
    if (Array.isArray(entry.channels)) merged.channels = entry.channels;
    if (typeof entry.summary === 'string') merged.summary = entry.summary.trim();
    if (typeof entry.body === 'string') merged.body = entry.body.trim();
    if (typeof entry.enabled === 'boolean') merged.enabled = entry.enabled;
    if (!merged.summary) merged.summary = merged.body;
    if (!merged.body) merged.body = merged.summary;
    merged.id = id;
    blocks.set(id, merged);
  }
}

// ---------------------------------------------------------------------------
// Channels
// ---------------------------------------------------------------------------
//
// A project channel entry either REPLACES the shipped block list (`blocks`) or
// edits it (`add` / `remove`) — the two compose in that order, so
// `{"blocks": [...], "add": [...]}` is meaningful rather than ambiguous.

function mergeChannel(base = {}, over = {}, name, errors) {
  let ids = Array.isArray(over.blocks)
    ? [...over.blocks]
    : Array.isArray(base.blocks)
      ? [...base.blocks]
      : [];
  if (over.blocks !== undefined && !Array.isArray(over.blocks)) {
    errors.push(`channels.${name}.blocks: expected an array of block ids`);
  }
  if (Array.isArray(over.add)) {
    for (const id of over.add) if (!ids.includes(id)) ids.push(id);
  }
  if (Array.isArray(over.remove)) {
    ids = ids.filter((id) => !over.remove.includes(id));
  }
  const mode = over.render ?? base.render ?? DEFAULT_MODE;
  if (!MODES.includes(mode)) {
    errors.push(`channels.${name}.render: expected one of ${MODES.join(' | ')}, got '${mode}'`);
  }
  const budget = Number.isInteger(over.budget) ? over.budget : Number.isInteger(base.budget) ? base.budget : 0;
  return {
    name,
    blocks: ids,
    render: MODES.includes(mode) ? mode : DEFAULT_MODE,
    budget, // 0 = unbudgeted
    headings: over.headings ?? base.headings ?? undefined,
    // {file, block} — the document + managed block `vibe render <c> --write`
    // syncs into. Data, so a project can retarget it (or add its own document
    // channel) without an engine change.
    write: over.write ?? base.write ?? undefined,
    enabled: over.enabled !== false && base.enabled !== false,
  };
}

// A null-prototype map, deliberately: config comes from JSON, and
// `JSON.parse('{"__proto__": …}')` yields an OWN "__proto__" key. Assigning that
// key onto an ordinary object walks the setter and mutates Object.prototype;
// reading `channels.constructor` on one finds an inherited function and treats
// it as a channel. Both are closed by giving the map no prototype at all —
// the same class of defect state.mjs's prototype-key rejection already covers.
function mergeChannels(defaults, project, errors) {
  const out = Object.create(null);
  const names = new Set([
    ...Object.keys(defaults.channels ?? {}),
    ...Object.keys(project.channels ?? {}),
  ]);
  for (const name of [...names].sort()) {
    out[name] = mergeChannel(
      (defaults.channels ?? {})[name],
      (project.channels ?? {})[name],
      name,
      errors,
    );
  }
  return out;
}

// ---------------------------------------------------------------------------
// loadContent — the one entry point that assembles all three layers.
// ---------------------------------------------------------------------------

export function loadContent(root, vibeDir) {
  const errors = [];
  const shippedDir = typeof vibeDir === 'string' ? path.join(vibeDir, 'content') : undefined;
  const defaultConfigPath =
    typeof vibeDir === 'string' ? path.join(vibeDir, DEFAULT_CONFIG_RELPATH) : undefined;
  const projectConfigPath =
    typeof root === 'string' ? path.join(root, CONFIG_BASENAME) : undefined;

  const defaults = (defaultConfigPath && readConfig(defaultConfigPath, errors, 'defaults')) || {};
  const project = (projectConfigPath && readConfig(projectConfigPath, errors, CONFIG_BASENAME)) || {};

  const blocks = new Map();
  if (typeof vibeDir === 'string') loadBlocksDir(path.join(vibeDir, SHIPPED_BLOCKS_RELPATH), blocks);
  if (typeof root === 'string') loadBlocksDir(path.join(root, USER_BLOCKS_RELPATH), blocks);
  applyConfigBlocks(defaults.blocks, blocks, shippedDir ?? '.', errors);
  applyConfigBlocks(project.blocks, blocks, root ?? '.', errors);

  const placeholders = Object.assign(
    Object.create(null),
    defaults.placeholders && typeof defaults.placeholders === 'object' ? defaults.placeholders : {},
    project.placeholders && typeof project.placeholders === 'object' ? project.placeholders : {},
  );

  const defaultSources = defaults.sources && typeof defaults.sources === 'object' ? defaults.sources : {};
  const projectSources = project.sources && typeof project.sources === 'object' ? project.sources : {};

  return {
    channels: mergeChannels(defaults, project, errors),
    blocks,
    placeholders,
    // `sources` is CONFIG DATA (paths the content layer reads, e.g. lessons);
    // `origins` is where the config itself came from. Two maps, because a
    // project's own `sources` key must never collide with the loader's
    // bookkeeping.
    sources: Object.assign(Object.create(null), defaultSources, projectSources),
    origins: {
      defaults: defaultConfigPath,
      project: projectConfigPath,
      hasProjectConfig: Boolean(projectConfigPath && fs.existsSync(projectConfigPath)),
    },
    errors,
  };
}

// ---------------------------------------------------------------------------
// Placeholders — `{{name}}`, typed and resolved from the live machine fields.
// ---------------------------------------------------------------------------
//
// Built-ins win over config `placeholders`, so a project cannot shadow `state`
// with a constant and silently freeze the injected cursor. Unknown names are
// left LITERAL (never blanked) and reported: a visible `{{typo}}` in the inject
// is debuggable, a silent empty string is not.

function joinList(value) {
  return Array.isArray(value) ? value.join(', ') : typeof value === 'string' ? value : '';
}

// `{{lessons:TAG}}` reads the durable-memory lessons file named by
// `sources.lessons` in the config — the path is DATA, not an engine literal,
// so the layout stays owned by root.mjs/the config and a project that keeps
// its lessons elsewhere only edits vibe.json.
function lessonsFor(root, relPath, tag) {
  if (typeof relPath !== 'string' || !relPath) return '';
  const text = readText(path.resolve(root ?? '.', relPath));
  if (text === undefined) return '';
  const wanted = String(tag).toLowerCase();
  const out = [];
  let title = '';
  for (const line of text.split('\n')) {
    const heading = /^###\s+(.*)$/.exec(line);
    if (heading) {
      title = heading[1].trim();
      continue;
    }
    const tags = /^\*\*Tags:\*\*\s*(.*)$/.exec(line);
    if (tags && title) {
      const list = tags[1].split(',').map((s) => s.trim().toLowerCase());
      if (list.includes(wanted)) out.push(`- ${title}`);
    }
  }
  return out.join('\n');
}

export function buildResolver(ctx, content) {
  const { root, vibeDir, skillsDir } = ctx;
  const cache = new Map();

  function cursorInfo() {
    if (cache.has('#cursor')) return cache.get('#cursor');
    let info = { state: 'idle', feature: '' };
    try {
      const cursor = readCursor(vibeDir);
      info = { state: cursor.state || 'idle', feature: cursor.feature || '' };
    } catch {
      // absent or corrupt cursor legitimately means idle — same degrade as orders
    }
    cache.set('#cursor', info);
    return info;
  }

  function machineState() {
    if (cache.has('#state')) return cache.get('#state');
    let value;
    try {
      value = stateOf(loadMachine(vibeDir), cursorInfo().state);
    } catch {
      value = undefined;
    }
    cache.set('#state', value);
    return value;
  }

  const builtins = {
    state: () => cursorInfo().state,
    flow: () => cursorInfo().state.split('.')[0],
    phase: () => cursorInfo().state.split('.').slice(1).join('.') || cursorInfo().state,
    feature: () => cursorInfo().feature || '<feature>',
    next: () => joinList(machineState()?.next),
    writes: () => joinList(machineState()?.writes),
    reads: () => joinList(machineState()?.reads),
    delegates: () => joinList(machineState()?.delegates),
    exit: () => machineState()?.exit ?? '',
    orders: () => (runOrders(vibeDir, skillsDir, []).stdout || '').trim(),
    // The write invariants as prose, generated from the SAME rules the
    // enforcer decides against (content/policy.json). Hand-authored copies of
    // this text used to be kept in step with the code by a test that could
    // only notice disagreement after the fact; generated text cannot disagree.
    // Degrades to '' — an absent or malformed policy contributes no sentence,
    // it never breaks the turn.
    invariants: () => {
      try {
        return renderInvariants({ rules: loadPolicy(vibeDir).rules }).trim();
      } catch {
        return '';
      }
    },
    doctrine: () => {
      const text = readText(path.join(skillsDir ?? '', 'vibe', 'SKILL.md'));
      if (text === undefined) return '';
      return (extractBlock(text, 'vibe:doctrine') ?? '').trim();
    },
  };

  return function resolve(name) {
    if (cache.has(name)) return cache.get(name);
    let value;
    const lessonsMatch = /^lessons:(.+)$/.exec(name);
    if (lessonsMatch) value = lessonsFor(root, content.sources.lessons, lessonsMatch[1]);
    else if (Object.prototype.hasOwnProperty.call(builtins, name)) value = builtins[name]();
    else if (typeof content.placeholders[name] === 'string') value = content.placeholders[name];
    else value = undefined;
    cache.set(name, value);
    return value;
  };
}

const PLACEHOLDER_RE = /\{\{([A-Za-z0-9_.:-]+)\}\}/g;

export function interpolate(text, resolve, unresolved) {
  return text.replace(PLACEHOLDER_RE, (literal, name) => {
    const value = resolve(name);
    if (value === undefined) {
      if (unresolved && !unresolved.includes(name)) unresolved.push(name);
      return literal;
    }
    return value;
  });
}

// ---------------------------------------------------------------------------
// renderChannel — compose one channel's blocks into injectable text.
// ---------------------------------------------------------------------------

export function renderChannel(name, ctx, content = loadContent(ctx.root, ctx.vibeDir)) {
  const warnings = [];
  const errors = [];
  const unresolved = [];
  // hasOwnProperty even though mergeChannels hands back a null-prototype map:
  // renderChannel also accepts a caller-supplied `content`, and a plain-object
  // channels map would resolve 'constructor' / 'toString' to inherited
  // functions and then crash on `channel.blocks`.
  const channel = Object.prototype.hasOwnProperty.call(content.channels ?? {}, name)
    ? content.channels[name]
    : undefined;
  if (!channel || typeof channel !== 'object') {
    return { text: '', blocks: [], warnings, errors: [`unknown channel '${name}'`], unresolved };
  }
  if (!channel.enabled) return { text: '', blocks: [], warnings, errors, unresolved };

  const resolve = buildResolver(ctx, content);
  const rendered = [];
  const used = [];
  for (const id of Array.isArray(channel.blocks) ? channel.blocks : []) {
    const block = content.blocks.get(id);
    if (!block) {
      errors.push(`channels.${name}: no such block '${id}'`);
      continue;
    }
    if (!block.enabled) continue;
    const useBody = channel.render === 'body';
    const raw = useBody ? block.body : block.summary;
    if (!raw || !raw.trim()) {
      warnings.push(`channels.${name}: block '${id}' has no ${channel.render} content`);
      continue;
    }
    const wantHeading = channel.headings ?? useBody;
    const text = interpolate(raw.trim(), resolve, unresolved);
    rendered.push(wantHeading && block.title ? `## ${block.title}\n\n${text}` : text);
    used.push(id);
  }

  if (rendered.length === 0) return { text: '', blocks: used, warnings, errors, unresolved };

  const separator = channel.render === 'body' ? '\n\n' : '\n';
  const text = `${rendered.join(separator)}\n`;
  const lines = text.replace(/\n$/, '').split('\n').length;
  if (channel.budget > 0 && lines > channel.budget) {
    errors.push(`channels.${name}: ${lines} lines exceeds the ${channel.budget}-line budget`);
  }
  for (const unknown of unresolved) {
    errors.push(`channels.${name}: unresolved placeholder {{${unknown}}}`);
  }
  return { text, blocks: used, warnings, errors, unresolved };
}

// Hook-facing wrapper: whatever happens, return a string. A channel that is
// absent, empty, or broken contributes nothing to the turn.
export function renderChannelSafe(name, ctx) {
  try {
    return renderChannel(name, ctx).text || '';
  } catch {
    return '';
  }
}

// ---------------------------------------------------------------------------
// Lints (`vibe render --check`) — the CI tooth for authored content.
// ---------------------------------------------------------------------------

export function checkContent(ctx, content = loadContent(ctx.root, ctx.vibeDir)) {
  const errors = [...content.errors];
  const warnings = [];
  const seen = new Map(); // block id -> channels it is composed into

  for (const name of Object.keys(content.channels)) {
    const channel = content.channels[name];
    const result = renderChannel(name, ctx, content);
    errors.push(...result.errors);
    warnings.push(...result.warnings);
    for (const id of Array.isArray(channel.blocks) ? channel.blocks : []) {
      const block = content.blocks.get(id);
      if (!block) continue; // renderChannel already errored
      if (block.channels.length && !block.channels.includes(name)) {
        warnings.push(
          `blocks.${id}: composed into '${name}' but its frontmatter declares channels [${block.channels.join(', ')}]`,
        );
      }
      seen.set(id, [...(seen.get(id) ?? []), name]);
    }
  }

  for (const [id, channels] of seen) {
    if (channels.length > 1) {
      warnings.push(`blocks.${id}: composed into ${channels.length} channels (${channels.join(', ')}) — the same text is injected twice`);
    }
  }

  // A check that examines nothing must fail loudly (see .spec/lessons.md):
  // "no errors" is only evidence when there was content to examine.
  if (content.blocks.size === 0) {
    errors.push('no content blocks found — the shipped content/blocks tree is missing or unreadable');
  }
  if (Object.keys(content.channels).length === 0) {
    errors.push('no channels configured — content/vibe.default.json is missing or unreadable');
  }

  return { errors, warnings, blockCount: content.blocks.size };
}
