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

export function extractBlock(text, id) {
  const lines = text.split('\n');
  const opener = openerLine(id);
  const openIdx = lines.findIndex((line) => line === opener);
  if (openIdx === -1) return undefined;

  const closers = closerCandidates(id);
  for (let i = openIdx + 1; i < lines.length; i += 1) {
    if (closers.includes(lines[i])) {
      return lines.slice(openIdx + 1, i).join('\n');
    }
  }
  return undefined; // opener found, no closer before EOF — never leak the tail
}
