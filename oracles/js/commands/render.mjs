// engine/commands/render.mjs — `vibe render`, the content layer's one CLI
// surface (content-layer feature).
//
//   vibe render <channel>              compose a channel to stdout
//   vibe render <channel> --write      write it into that channel's managed
//                                      document block (config: channels.<c>.write)
//   vibe render --list                 channels, their blocks, and where each came from
//   vibe render --check                lint the whole content tree (CI tooth)
//
// The hooks never shell out to this command — they call renderChannel()
// directly (content.mjs) — so this stays the human/CI entry point: authoring
// feedback, the AGENTS.md sync, and the lint that fails a build.
//
// Exit codes: 0 fine; 1 a lint error, an unknown channel, or a failed write.
// Rendering itself never throws — a broken content tree renders empty and
// reports, so a bad block can never take a session down.

import fs from 'node:fs';
import path from 'node:path';
import { resolveRoot, resolveVibeDir, resolveSkillsDir } from '../root.mjs';
import { upsertBlock } from '../blocks.mjs';
import { loadContent, renderChannel, checkContent, confinePath } from '../content.mjs';

const USAGE = [
  'usage: vibe render <channel> [--write] | --list | --check',
  '',
  '  <channel>   compose the channel and print it',
  '  --write     write the channel into its configured document block',
  '  --list      list channels, their composed blocks, and the config sources',
  '  --check     lint the content tree (unknown blocks, budgets, placeholders)',
].join('\n');

function listReport(content) {
  const lines = [];
  lines.push(`defaults: ${content.origins.defaults ?? '(none)'}`);
  lines.push(
    `project:  ${content.origins.project ?? '(none)'}${content.origins.hasProjectConfig ? '' : ' (absent — shipped defaults only)'}`,
  );
  lines.push(`blocks:   ${content.blocks.size}`);
  for (const name of Object.keys(content.channels)) {
    const channel = content.channels[name];
    // The trigger is what decides whether a channel is injected on THIS turn,
    // so `--list` — the "what does each channel do" view — has to show it, not
    // just what each channel composes.
    const flags = [`render=${channel.render}`, `trigger=${channel.trigger}`];
    if (channel.budget > 0) flags.push(`budget=${channel.budget}`);
    if (!channel.enabled) flags.push('disabled');
    lines.push(`\n${name} (${flags.join(', ')})`);
    if (channel.blocks.length === 0) lines.push('  (no blocks)');
    for (const id of channel.blocks) {
      const block = content.blocks.get(id);
      if (!block) lines.push(`  ${id} — MISSING`);
      else lines.push(`  ${id}${block.enabled ? '' : ' (disabled)'} — ${block.source ?? 'vibe.json'}`);
    }
  }
  return `${lines.join('\n')}\n`;
}

// `channels.<name>.write` names the document and the managed block a channel
// syncs into — data, so a project can point `agents-md` at CLAUDE.md, or add
// its own document channel, without touching the engine.
function writeTarget(channel, root) {
  const spec = channel.write;
  if (!spec || typeof spec !== 'object') return undefined;
  if (typeof spec.file !== 'string' || !spec.file) return undefined;
  if (typeof spec.block !== 'string' || !spec.block) return undefined;
  // Confined to the repo: `write.file` is project config, and `--write` creates
  // the file it names. Unconfined, a `vibe.json` shipped in a cloned repo could
  // write anywhere the user can.
  const file = confinePath(root, spec.file);
  if (file === undefined) return { escaped: spec.file };
  return {
    file,
    block: spec.block,
    note: typeof spec.note === 'string' ? spec.note : '',
  };
}

export function runRender(ctx, args) {
  const argv = Array.isArray(args) ? args : [];
  // `-h` is the one short flag this command answers to; everything else is
  // classified by the `--` prefix exactly as before. Collecting only `--`
  // arguments left `-h` in `positional`, which made the `flags.has('-h')` test
  // below dead code and sent `vibe render -h` into the channel lookup to exit 1
  // with `unknown channel '-h'`.
  const isFlag = (a) => a.startsWith('--') || a === '-h';
  const flags = new Set(argv.filter(isFlag));
  const positional = argv.filter((a) => !isFlag(a));

  if (flags.has('--help') || flags.has('-h')) return { code: 0, stdout: `${USAGE}\n`, stderr: '' };

  const content = loadContent(ctx.root, ctx.vibeDir);

  if (flags.has('--list')) return { code: 0, stdout: listReport(content), stderr: '' };

  if (flags.has('--check')) {
    const { errors, warnings, blockCount } = checkContent(ctx, content);
    let stderr = '';
    for (const w of warnings) stderr += `render: WARN — ${w}\n`;
    for (const e of errors) stderr += `render: ERROR — ${e}\n`;
    const channelCount = Object.keys(content.channels).length;
    const stdout = `render: checked ${blockCount} blocks across ${channelCount} channels — ${errors.length} error(s), ${warnings.length} warning(s)\n`;
    return { code: errors.length ? 1 : 0, stdout, stderr };
  }

  const name = positional[0];
  if (!name) return { code: 1, stdout: '', stderr: `${USAGE}\n` };

  const result = renderChannel(name, ctx, content);
  let stderr = '';
  for (const w of result.warnings) stderr += `render: WARN — ${w}\n`;
  for (const e of result.errors) stderr += `render: ERROR — ${e}\n`;
  const channel = content.channels[name];
  if (!channel) return { code: 1, stdout: '', stderr };

  if (!flags.has('--write')) return { code: result.errors.length ? 1 : 0, stdout: result.text, stderr };

  const target = writeTarget(channel, ctx.root);
  if (target && target.escaped !== undefined) {
    return {
      code: 1,
      stdout: '',
      stderr: `${stderr}render: ERROR — channel '${name}' write target '${target.escaped}' resolves outside the repo; refusing to write\n`,
    };
  }
  if (!target) {
    return {
      code: 1,
      stdout: '',
      stderr: `${stderr}render: ERROR — channel '${name}' has no write target (set channels.${name}.write = {"file": ..., "block": ...})\n`,
    };
  }

  let existing = '';
  try {
    existing = fs.readFileSync(target.file, 'utf8');
  } catch (err) {
    if (!err || err.code !== 'ENOENT') {
      return { code: 1, stdout: '', stderr: `${stderr}render: ERROR — cannot read ${target.file}: ${err && err.message}\n` };
    }
  }

  const composed = result.text.replace(/\n+$/, '');
  // The note is data (channels.<c>.write.note), so the document says who owns
  // the region without the engine hardcoding a sentence about itself.
  const body = composed && target.note ? `${target.note}\n\n${composed}` : composed;
  if (!body) {
    return { code: 1, stdout: '', stderr: `${stderr}render: ERROR — channel '${name}' composed no content; refusing to write an empty block\n` };
  }
  const next = upsertBlock(existing, target.block, body);
  if (!next.changed) {
    return { code: result.errors.length ? 1 : 0, stdout: `render: no change — ${target.file} already carries the current ${target.block} block\n`, stderr };
  }

  // Temp + rename in the target's own directory: a crash mid-write can never
  // truncate a document that holds user prose outside the managed block.
  const tmp = path.join(path.dirname(target.file), `.${path.basename(target.file)}.${process.pid}.tmp`);
  try {
    fs.writeFileSync(tmp, next.text, 'utf8');
    fs.renameSync(tmp, target.file);
  } catch (err) {
    try {
      fs.unlinkSync(tmp);
    } catch {
      // best-effort cleanup; the original error is what matters
    }
    return { code: 1, stdout: '', stderr: `${stderr}render: ERROR — cannot write ${target.file}: ${err && err.message}\n` };
  }

  return {
    code: result.errors.length ? 1 : 0,
    stdout: `render: ${next.action} the ${target.block} block in ${target.file} (${result.blocks.length} block(s))\n`,
    stderr,
  };
}

export default async function run(argv, opts = {}) {
  const ctx = {
    root: resolveRoot(opts),
    vibeDir: resolveVibeDir(opts),
    skillsDir: resolveSkillsDir(opts),
  };
  const result = runRender(ctx, argv);
  if (result.stdout) process.stdout.write(result.stdout);
  if (result.stderr) process.stderr.write(result.stderr);
  return result.code;
}
