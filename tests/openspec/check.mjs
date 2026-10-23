// tests/openspec/check.mjs — helpers for tests/openspec/run.sh (F18).
//   node check.mjs count <report.json> <n>   fail unless n items ran and n passed
//   node check.mjs fail <report.json> <n>    fail unless n items ran, exactly 1 failed,
//                                            and an issue says "SHALL or MUST"
//   node check.mjs mutate <file> <from> <to> replace <from> once; fail if absent
import { readFileSync, writeFileSync } from 'node:fs';

const [cmd, file, a, b] = process.argv.slice(2);
if (cmd === 'count') {
  const t = JSON.parse(readFileSync(file, 'utf8')).summary.totals;
  const want = Number(a);
  if (t.items !== want || t.passed !== want) {
    console.error(`openspec: expected ${want} items passed, got ${t.items} items, ${t.passed} passed`);
    process.exit(1);
  }
} else if (cmd === 'fail') {
  const r = JSON.parse(readFileSync(file, 'utf8'));
  const t = r.summary.totals;
  const want = Number(a);
  const why = (r.items ?? []).some((i) => (i.issues ?? []).some((x) => String(x.message).includes('SHALL or MUST')));
  if (t.items !== want || t.failed !== 1 || !why) {
    console.error(`openspec: mutant must fail once over ${want} items for "SHALL or MUST"; got ${t.items} items, ${t.failed} failed, reason found: ${why}`);
    process.exit(1);
  }
} else if (cmd === 'mutate') {
  const text = readFileSync(file, 'utf8');
  if (!text.includes(a)) {
    console.error(`openspec: ${file} does not contain ${JSON.stringify(a)}`);
    process.exit(1);
  }
  writeFileSync(file, text.replace(a, b));
} else {
  console.error('usage: node check.mjs count <report> <n> | fail <report> <n> | mutate <file> <from> <to>');
  process.exit(2);
}
