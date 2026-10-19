// Validates contracts/: every schema is usable and states the compatibility
// rule; every Spec JSON golden validates (valid/) or fails (invalid/); the
// exit-code sets are exact; population floors hold.
// Usage: node contracts/run.mjs [--dir <contracts dir>]
import { existsSync, readdirSync, readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { checkSchema, validate } from './lib/schema.mjs';

const COMPAT = 'additive-within-major; readers-ignore-unknown-fields';
const QUERIES = ['root', 'lessons-for', 'plan', 'feature', 'validate', 'trace', 'check-drift', 'promote', 'openspec', 'setup', 'init', 'migrate'];
// .spec/tech-spec.md "Root and discovery", steps 1-5; step 6 repeats 4-5 per parent (`depth`).
const ROOT_SOURCES = ['flag', 'env', 'config', 'docs-spec', 'dot-spec'];
// Required sets are part of the v1 contract: dropping one breaks readers (additive-only).
const REQUIRED = {
  root: ['root', 'source', 'v'],
  'lessons-for': ['lessons', 'tags', 'v'],
  lesson: ['rule', 'tags', 'title'],
  plan: ['feature', 'rows', 'v'],
  'plan-row': ['id', 'status'],
  feature: ['docs', 'feature', 'layer', 'v'],
  trace: ['cited', 'history', 'orphans', 'uncited', 'units', 'v'],
  orphan: ['source', 'unit'],
  'check-drift': ['errors', 'findings', 'pre_merge', 'root', 'v', 'warnings'],
  'drift-feature': ['name', 'status'],
  'drift-finding': ['message', 'severity'],
  promote: ['artifacts', 'deleted', 'dry_run', 'feature', 'v'],
  'promote-change': ['applied', 'name', 'op'],
  'promote-item': ['target', 'what'],
  openspec: ['dir', 'direction', 'v', 'written'],
  setup: ['skipped', 'v', 'written'],
  init: ['created', 'root', 'skipped', 'v'],
  migrate: ['created', 'dry_run', 'from', 'markers', 'moves', 'rewrites', 'to', 'v'],
  'migrate-move': ['from', 'to'],
  'migrate-rewrite': ['file', 'line', 'new', 'old'],
  doc: ['kind', 'path'],
  validate: ['checks', 'errors', 'examined', 'findings', 'layout', 'root', 'strict', 'v', 'warnings'],
  'check-level': ['id', 'level'],
  finding: ['check', 'message', 'severity'],
};
const EXIT_CODES = { cli: ['0', '1', '2'], 'vibe-spec-query': ['3'], hook: ['0', '2'] };
const FLOOR = { schemas: 2, validPerQuery: 2, invalidPerQuery: 2 };

const at = process.argv.indexOf('--dir');
const dir = at > 0 ? process.argv[at + 1] : dirname(fileURLToPath(import.meta.url));
const errors = [];
const readJson = (p) => {
  try { return JSON.parse(readFileSync(p, 'utf8')); } catch (err) { errors.push(`${p}: ${err.message}`); return undefined; }
};
const jsonFiles = (p) => (existsSync(p) ? readdirSync(p).filter((f) => f.endsWith('.json')).sort() : []);

const schemaFiles = jsonFiles(dir).filter((f) => f.endsWith('.schema.json'));
if (schemaFiles.length < FLOOR.schemas) errors.push(`only ${schemaFiles.length} schema(s) (floor ${FLOOR.schemas})`);
const schemas = {};
for (const f of schemaFiles) {
  const s = readJson(join(dir, f));
  if (s === undefined) continue;
  schemas[f] = s;
  errors.push(...checkSchema(s).map((e) => `${f}: ${e}`));
  if (s['x-vibe-compat'] !== COMPAT) errors.push(`${f}: x-vibe-compat must be "${COMPAT}"`);
}

let valid = 0;
let invalid = 0;
const spec = schemas['spec-json.schema.json'];
if (!spec) errors.push('spec-json.schema.json missing');
for (const q of spec ? QUERIES : []) {
  const def = spec.$defs?.[q];
  if (!def) { errors.push(`spec-json: $defs.${q} missing`); continue; }
  if (def.properties?.v?.const !== 1 || !def.required?.includes('v')) errors.push(`spec-json: ${q} must require "v" with const 1`);
  const base = join(dir, 'fixtures', 'spec-json', q);
  const goods = jsonFiles(join(base, 'valid'));
  const bads = jsonFiles(join(base, 'invalid'));
  if (goods.length < FLOOR.validPerQuery) errors.push(`${q}: ${goods.length} valid golden(s) (floor ${FLOOR.validPerQuery})`);
  if (bads.length < FLOOR.invalidPerQuery) errors.push(`${q}: ${bads.length} invalid golden(s) (floor ${FLOOR.invalidPerQuery})`);
  if (!goods.includes('unknown-field.json')) errors.push(`${q}: valid/unknown-field.json missing`);
  for (const f of goods) {
    const v = readJson(join(base, 'valid', f));
    if (v === undefined) continue;
    valid += 1;
    const e = validate(spec, def, v);
    if (e.length) errors.push(`${q}/valid/${f}: ${e.join('; ')}`);
    if (f === 'unknown-field.json' && !Object.keys(v ?? {}).some((k) => !(k in (def.properties ?? {})))) {
      errors.push(`${q}/valid/unknown-field.json: holds no field outside the schema`);
    }
  }
  for (const f of bads) {
    const v = readJson(join(base, 'invalid', f));
    if (v === undefined) continue;
    invalid += 1;
    if (validate(spec, def, v).length === 0) errors.push(`${q}/invalid/${f}: validates, but must fail`);
  }
}
const sources = spec?.$defs?.root?.properties?.source?.enum;
if (spec && JSON.stringify(sources) !== JSON.stringify(ROOT_SOURCES)) {
  errors.push(`spec-json: root.source is ${JSON.stringify(sources)}, discovery order is ${JSON.stringify(ROOT_SOURCES)}`);
}

for (const [name, want] of Object.entries(REQUIRED)) {
  if (!spec) break;
  const def = spec.$defs?.[name];
  if (!def) { if (!QUERIES.includes(name)) errors.push(`spec-json: $defs.${name} missing`); continue; }
  const got = [...(def.required ?? [])].sort();
  if (JSON.stringify(got) !== JSON.stringify(want)) errors.push(`spec-json: ${name} requires ${JSON.stringify(got)}, contract pins ${JSON.stringify(want)}`);
}

const exitSchema = schemas['exit-codes.schema.json'];
const codes = existsSync(join(dir, 'exit-codes.json')) ? readJson(join(dir, 'exit-codes.json')) : undefined;
if (!exitSchema || codes === undefined) errors.push('exit-codes.json or exit-codes.schema.json missing');
else {
  errors.push(...validate(exitSchema, exitSchema, codes).map((e) => `exit-codes.json: ${e}`));
  for (const [ctx, want] of Object.entries(EXIT_CODES)) {
    const got = Object.keys(codes.contexts?.[ctx] ?? {}).sort();
    if (JSON.stringify(got) !== JSON.stringify(want)) errors.push(`exit-codes.json: ${ctx} has ${JSON.stringify(got)}, want ${JSON.stringify(want)}`);
  }
}

for (const e of errors) console.error(`contracts: ${e}`);
console.log(`contracts: ${schemaFiles.length} schemas, ${valid} valid + ${invalid} invalid goldens — ${errors.length ? 'FAIL' : 'OK'}`);
process.exit(errors.length ? 1 : 0);
