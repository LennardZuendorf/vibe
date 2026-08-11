// engine/tests/blocks.test.mjs — engine/blocks.mjs (js-core/2).
//
// The load-bearing scenario: a missing/misspelled closer must return
// undefined, never the rest of the file — the bash `sed -n '/open/,/close/p'`
// range prints to EOF in that case, and extractBlock must not reproduce it.

import { test, assert, assertEqual } from './run.mjs';
import { extractBlock } from '../blocks.mjs';

test('extractBlock: extracts a well-formed symmetric block (doctrine grammar)', () => {
  const text = [
    'before',
    '<!-- vibe:doctrine -->',
    'line one',
    'line two',
    '<!-- /vibe:doctrine -->',
    'after',
  ].join('\n');
  assertEqual(extractBlock(text, 'vibe:doctrine'), 'line one\nline two');
});

test('extractBlock: accepts the legacy asymmetric closer (orders grammar)', () => {
  const text = [
    '<!-- vibe:orders:setup.detect -->',
    'skill=vibe · do the thing · next: setup.apply, idle',
    '<!-- /vibe:orders -->',
  ].join('\n');
  assertEqual(
    extractBlock(text, 'vibe:orders:setup.detect'),
    'skill=vibe · do the thing · next: setup.apply, idle',
  );
});

test('extractBlock: a state key containing a literal "." does not act as a regex wildcard', () => {
  // Oracle parity: orders.sh escapes '.' for its sed address. A block for
  // "feature.impl" must not spuriously match an opener spelled
  // "featureXimpl" (or any other single-char substitution for the dot).
  const text = [
    '<!-- vibe:orders:featureXimpl -->',
    'wrong block',
    '<!-- /vibe:orders -->',
    '<!-- vibe:orders:feature.impl -->',
    'right block',
    '<!-- /vibe:orders -->',
  ].join('\n');
  assertEqual(extractBlock(text, 'vibe:orders:feature.impl'), 'right block');
});

test('extractBlock: MISSING closer returns undefined, never the file tail', () => {
  const text = [
    '<!-- vibe:orders:setup.detect -->',
    'line one',
    'line two',
    '-- no closer anywhere below --',
    'trailing content that must never leak out',
  ].join('\n');
  assertEqual(extractBlock(text, 'vibe:orders:setup.detect'), undefined);
});

test('extractBlock: MISSPELLED closer (matches neither exact nor legacy form) returns undefined', () => {
  const text = [
    '<!-- vibe:doctrine -->',
    'content',
    '<!-- /vibe:doctrin -->', // typo'd closer
    'trailing content that must never leak out',
  ].join('\n');
  assertEqual(extractBlock(text, 'vibe:doctrine'), undefined);
});

test('extractBlock: absent opener returns undefined', () => {
  const text = 'nothing here at all';
  assertEqual(extractBlock(text, 'vibe:doctrine'), undefined);
});

test('extractBlock: an empty block (opener immediately followed by closer) returns an empty string, not undefined', () => {
  const text = ['<!-- vibe:doctrine -->', '<!-- /vibe:doctrine -->'].join('\n');
  assertEqual(extractBlock(text, 'vibe:doctrine'), '');
});

// js-core/2 review, Finding 4 — on a duplicated id, this correctly returns
// the first block's content in isolation. That is NOT what the sed oracle
// does and this must not be read as a parity claim: `sed -n '/o/,/c/p'`
// re-activates its range on the second opener too, so raw sed prints all
// six lines, and the outer `sed '1d;$d'` (which strips only the first/last
// line of that whole run) leaves a garbled four-line blob — "first", the
// first block's closer, the second opener, and "second" — verified by hand
// against `sed -n '/^<!-- vibe:doctrine -->$/,/^<!-- \/vibe:doctrine -->$/p' | sed '1d;$d'`.
// Real fixtures never duplicate a state id, so this divergence is not on
// unit 8's parity matrix; the JS behaviour here is simply better than the
// oracle's, deliberately kept rather than reproduced.
test('extractBlock: a duplicated id returns the first block cleanly (deliberate improvement over the sed oracle, not parity)', () => {
  const text = [
    '<!-- vibe:doctrine -->',
    'first',
    '<!-- /vibe:doctrine -->',
    '<!-- vibe:doctrine -->',
    'second',
    '<!-- /vibe:doctrine -->',
  ].join('\n');
  assertEqual(extractBlock(text, 'vibe:doctrine'), 'first');
});

// Oracle parity — extract the real doctrine block straight out of flow/SKILL.md
// (the actual oracle file orders.sh/doctrine.sh read) and sanity-check its shape.
test('extractBlock: pulls a real, non-trivial block out of flow/SKILL.md', async () => {
  const { readFileSync } = await import('node:fs');
  const path = await import('node:path');
  const skillMd = readFileSync(
    path.join(path.dirname(new URL(import.meta.url).pathname), '..', '..', 'flow', 'SKILL.md'),
    'utf8',
  );
  const block = extractBlock(skillMd, 'vibe:doctrine');
  assert(typeof block === 'string' && block.length > 0, 'expected a non-empty doctrine block');
  assert(!block.includes('<!-- vibe:doctrine -->'), 'markers themselves must be excluded');
  assert(!block.includes('<!-- /vibe:doctrine -->'), 'markers themselves must be excluded');
});
