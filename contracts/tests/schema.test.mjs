// Unit tests for contracts/lib/schema.mjs. Run: node contracts/tests/schema.test.mjs
import assert from 'node:assert/strict';
import { checkSchema, validate } from '../lib/schema.mjs';

let passed = 0;
let failed = 0;
function test(name, fn) {
  try { fn(); passed += 1; console.log(`  PASS ${name}`); }
  catch (err) { failed += 1; console.log(`  FAIL ${name}\n    ${err.message.split('\n').join('\n    ')}`); }
}

const root = {
  $defs: {
    item: { type: 'object', required: ['id'], properties: { id: { type: 'string', minLength: 1 } } },
  },
  type: 'object',
  required: ['v', 'items'],
  properties: {
    v: { const: 1 },
    kind: { enum: ['a', 'b'] },
    count: { type: 'integer', minimum: 0 },
    date: { type: 'string', pattern: '^[0-9]{4}-[0-9]{2}-[0-9]{2}$' },
    items: { type: 'array', minItems: 1, items: { $ref: '#/$defs/item' } },
  },
};
const ok = (value) => validate(root, root, value);

test('a conforming value has no errors', () => {
  assert.deepEqual(ok({ v: 1, kind: 'a', count: 0, date: '2026-10-14', items: [{ id: 'x' }] }), []);
});
test('unknown fields are ignored', () => {
  assert.deepEqual(ok({ v: 1, items: [{ id: 'x', extra: true }], future: { any: 1 } }), []);
});
test('a missing required field fails', () => {
  assert.match(ok({ v: 1 }).join('\n'), /missing required "items"/);
});
test('a wrong const fails', () => assert.match(ok({ v: 2, items: [{ id: 'x' }] }).join('\n'), /const 1/));
test('a value outside enum fails', () => assert.match(ok({ v: 1, kind: 'c', items: [{ id: 'x' }] }).join('\n'), /not one of/));
test('a wrong type fails', () => assert.match(ok({ v: 1, items: 'x' }).join('\n'), /expected array/));
test('a float is not an integer', () => assert.match(ok({ v: 1, count: 1.5, items: [{ id: 'x' }] }).join('\n'), /expected integer/));
test('below minimum fails', () => assert.match(ok({ v: 1, count: -1, items: [{ id: 'x' }] }).join('\n'), /minimum/));
test('a pattern mismatch fails', () => assert.match(ok({ v: 1, date: '14.10.2026', items: [{ id: 'x' }] }).join('\n'), /does not match/));
test('too few items fails', () => assert.match(ok({ v: 1, items: [] }).join('\n'), /fewer than 1/));
test('$ref is followed into items', () => assert.match(ok({ v: 1, items: [{ id: '' }] }).join('\n'), /\$\.items\[0\]\.id: shorter than 1/));
test('an unresolved $ref is an error', () => {
  assert.match(validate({}, { $ref: '#/$defs/none' }, 1).join('\n'), /unresolved/);
});
test('checkSchema accepts the test schema', () => assert.deepEqual(checkSchema(root), []));
test('checkSchema rejects an unsupported keyword', () => {
  assert.match(checkSchema({ type: 'object', oneOf: [] }).join('\n'), /unsupported keyword "oneOf"/);
});
test('checkSchema rejects additionalProperties false, nested too', () => {
  const s = { properties: { a: { type: 'object', additionalProperties: false } } };
  assert.match(checkSchema(s).join('\n'), /#\/properties\/a: additionalProperties/);
});
test('checkSchema rejects a non-local $ref', () => {
  assert.match(checkSchema({ $ref: 'https://example.com/x.json' }).join('\n'), /\$ref must be/);
});

console.log(`\n=== results: ${passed} passed, ${failed} failed ===`);
process.exit(failed === 0 ? 0 : 1);
