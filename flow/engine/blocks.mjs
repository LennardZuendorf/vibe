// engine/blocks.mjs — extractBlock(): the one marker-block grammar.
//
// Markers look like:
//   <!-- <id> -->
//   ...content...
//   <!-- /<id> -->
//
// During js-core a legacy asymmetric closer is also accepted: for an id with
// more than two `:`-separated segments (e.g. "vibe:orders:setup.detect"),
// the closer may be truncated to the first two segments
// ("<!-- /vibe:orders -->") — this is what orders.sh's oracle actually
// emits, and ported output must stay byte-identical to it. content-layer
// migrates authors to the single symmetric form; this module keeps accepting
// both until then.
//
// If the opener is present but no matching closer is found before EOF, this
// returns undefined — NOT the file tail. That is the fix for a real bash
// bug: `sed -n '/open/,/close/p'` with no matching close address prints
// every line through EOF instead of nothing, so a missing or misspelled
// closer silently leaks the rest of the file as "block content".

function openerLine(id) {
  return `<!-- ${id} -->`;
}

function closerCandidates(id) {
  const exact = `<!-- /${id} -->`;
  const parts = id.split(':');
  if (parts.length > 2) {
    const legacy = `<!-- /${parts.slice(0, 2).join(':')} -->`;
    return [exact, legacy];
  }
  return [exact];
}

// The inclusive line span of a block, or undefined when the pair is absent
// (opener missing, or opener present with no closer before EOF — the same
// refusal extractBlock() makes, for the same reason).
function blockSpan(lines, id) {
  const opener = openerLine(id);
  const openIdx = lines.findIndex((line) => line === opener);
  if (openIdx === -1) return undefined;

  const closers = closerCandidates(id);
  for (let i = openIdx + 1; i < lines.length; i += 1) {
    if (closers.includes(lines[i])) return { openIdx, closeIdx: i };
  }
  return undefined;
}

export function extractBlock(text, id) {
  const lines = text.split('\n');
  const span = blockSpan(lines, id);
  if (!span) return undefined;
  return lines.slice(span.openIdx + 1, span.closeIdx).join('\n');
}

// The write half of the same grammar, so no other module has to spell the
// markers (the duplicate-primitive scan bans that outright — blocks.mjs is
// the one grammar, for reading AND writing).

// Removes the inclusive marker region; an absent or unclosed pair leaves the
// text byte-identical.
export function stripBlock(text, id) {
  const lines = text.split('\n');
  const span = blockSpan(lines, id);
  if (!span) return text;
  return [...lines.slice(0, span.openIdx), ...lines.slice(span.closeIdx + 1)].join('\n');
}

// A whole managed block: opener, body, symmetric closer. No trailing newline —
// the caller owns how it joins its surroundings.
export function renderBlock(id, body) {
  const inner = String(body).replace(/\n+$/, '');
  return `${openerLine(id)}\n${inner}\n<!-- /${id} -->`;
}

// Replace the block in place, or append it when absent. Returns the new text
// plus what happened, so a caller can stay idempotent (`changed: false` means
// the file already holds exactly this block and must not be rewritten).
export function upsertBlock(text, id, body) {
  const rendered = renderBlock(id, body);
  const lines = text.split('\n');
  const span = blockSpan(lines, id);
  if (span) {
    const next = [
      ...lines.slice(0, span.openIdx),
      ...rendered.split('\n'),
      ...lines.slice(span.closeIdx + 1),
    ].join('\n');
    return { text: next, changed: next !== text, action: 'replaced' };
  }
  const base = text.replace(/\n+$/, '');
  const next = base ? `${base}\n\n${rendered}\n` : `${rendered}\n`;
  return { text: next, changed: true, action: 'appended' };
}
