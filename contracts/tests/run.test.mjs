// Discriminating tests for contracts/run.mjs: a clean copy passes; each
// planted defect fails with its own message. Run: node contracts/tests/run.test.mjs
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { cpSync, mkdtempSync, readFileSync, rmSync, unlinkSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const CONTRACTS = join(dirname(fileURLToPath(import.meta.url)), '..');
let passed = 0;
let failed = 0;

function check(name, mutate, wantExit, wantText) {
  const dir = mkdtempSync(join(tmpdir(), 'contracts-'));
  try {
    cpSync(CONTRACTS, dir, { recursive: true });
    mutate(dir);
    const r = spawnSync(process.execPath, [join(CONTRACTS, 'run.mjs'), '--dir', dir], { encoding: 'utf8' });
    assert.equal(r.status, wantExit, `exit ${r.status}\n${r.stdout}${r.stderr}`);
    if (wantText) assert.match(r.stdout + r.stderr, wantText);
    passed += 1;
    console.log(`  PASS ${name}`);
  } catch (err) {
    failed += 1;
    console.log(`  FAIL ${name}\n    ${err.message.split('\n').join('\n    ')}`);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
}

const edit = (dir, rel, fn) => {
  const p = join(dir, rel);
  writeFileSync(p, `${JSON.stringify(fn(JSON.parse(readFileSync(p, 'utf8'))), null, 2)}\n`);
};
const SPEC = 'spec-json.schema.json';
const FX = 'fixtures/spec-json';

check('a clean copy passes', () => {}, 0, /OK$/m);
check('a valid golden missing "v" fails', (d) => edit(d, `${FX}/root/valid/dot-spec.json`, ({ v, ...rest }) => rest), 1, /missing required "v"/);
check('additionalProperties false fails', (d) => edit(d, SPEC, (s) => { s.$defs.plan.additionalProperties = false; return s; }), 1, /additionalProperties must be absent or true/);
check('an unsupported keyword fails', (d) => edit(d, SPEC, (s) => { s.$defs.root.oneOf = []; return s; }), 1, /unsupported keyword "oneOf"/);
check('a missing compat statement fails', (d) => edit(d, 'exit-codes.schema.json', ({ ['x-vibe-compat']: _, ...rest }) => rest), 1, /x-vibe-compat must be/);
check('an invalid golden that validates fails', (d) => writeFileSync(join(d, `${FX}/plan/invalid/row-without-status.json`), '{ "v": 1, "feature": "auth", "rows": [] }\n'), 1, /validates, but must fail/);
check('too few invalid goldens fails the floor', (d) => unlinkSync(join(d, `${FX}/feature/invalid/wrong-major.json`)), 1, /feature: 1 invalid golden\(s\) \(floor 2\)/);
check('an unknown-field golden without one fails', (d) => writeFileSync(join(d, `${FX}/plan/valid/unknown-field.json`), '{ "v": 1, "feature": "auth", "rows": [] }\n'), 1, /holds no field outside the schema/);
check('a reordered root.source enum fails', (d) => edit(d, SPEC, (s) => { s.$defs.root.properties.source.enum.reverse(); return s; }), 1, /discovery order/);
check('an extra cli exit code fails', (d) => edit(d, 'exit-codes.json', (c) => { c.contexts.cli['4'] = 'new'; return c; }), 1, /cli has/);
check('a missing query def fails', (d) => edit(d, SPEC, (s) => { delete s.$defs.feature; return s; }), 1, /\$defs\.feature missing/);
check('unparseable JSON fails', (d) => writeFileSync(join(d, `${FX}/root/valid/dot-spec.json`), '{ nope'), 1, /dot-spec\.json/);

console.log(`\n=== results: ${passed} passed, ${failed} failed ===`);
process.exit(failed === 0 ? 0 : 1);
