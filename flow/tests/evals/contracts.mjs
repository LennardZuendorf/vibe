import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../../..');
const read = p => fs.readFileSync(path.join(root, p), 'utf8');
const name = p => /^name: (.+)$/m.exec(read(p))?.[1];
assert.equal(name('skills/vibe/SKILL.md'), 'vibe', 'Task 2: toolkit identity');
assert.equal(name('flow/SKILL.md'), 'flow', 'Task 3: workflow identity');
for (const pkg of ['plugins/vibe', 'factory-plugin']) {
  for (const skill of ['vibe', 'flow', 'spec']) {
    assert.equal(name(`${pkg}/skills/${skill}/SKILL.md`), skill, `${pkg}: directory-derived identity`);
  }
  assert.equal(fs.existsSync(path.join(root, pkg, 'skills/using-spec/SKILL.md')), false, 'retire owned bootstrap');
  for (const kind of ['feature','quick','setup','strategy']) {
    const text = read(`${pkg}/skills/flow/procedures/${kind}.md`);
    for (const section of ['Inputs', 'Prerequisites', 'Artifacts', 'Checks', 'Recovery', 'Example']) assert.ok(text.includes(`## ${section}`), `${kind}: ${section}`);
  }
}
const selection = JSON.parse(read('flow/tests/evals/selection.json'));
assert.ok(selection.queries.length >= 10);
for (const id of ['orientation','direct-flow','direct-spec','no-root','missing-component','near-miss','delegated-worker']) assert.ok(selection.queries.some(q => q.id === id), id);
const scenarios = JSON.parse(read('flow/tests/evals/scenarios.json'));
for (const id of ['unapproved-coding','plan-edit','scope-drift','premature-finish','resume','candidate-change','missing-reviewer']) assert.ok(scenarios.scenarios.some(q => q.id === id), id);
console.log('PASS: Tasks 1–4 discovery/resource contracts (structural, not model behavior)');
