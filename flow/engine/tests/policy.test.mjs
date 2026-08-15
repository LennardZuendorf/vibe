// engine/tests/policy.test.mjs — engine/policy.mjs (loadPolicy/decide/
// renderInvariants) and its CLI surface, engine/commands/policy.mjs
// (inject-triggers/1).
//
// Three things are under test:
//   1. THE SHIPPED THREE RULES — content/policy.json must actually encode the
//      write invariants AGENTS.md documents: each blocks outside its states
//      and allows inside them, an unmatched path is allow.
//   2. DEGRADE — an absent or malformed policy.json must never throw, must
//      answer 'allow' for everything (fail OPEN — the write is not what is
//      unsafe here, a wedged hook is), and must report through `errors`.
//   3. THE CLI — `vibe policy decide` always exits 0 (the verdict lives on
//      stdout, never the exit code — the hook that consumes it does the
//      exit-code translation, not this command), `list` and `render` surface
//      the same data loadPolicy/decide/renderInvariants already prove correct.

import { mkdirSync, writeFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { test, assert, assertEqual, assertIncludes, makeHookSandbox, runCli } from './run.mjs';
import { loadPolicy, decide, renderInvariants } from '../policy.mjs';
import { runDecide, runList, runRenderInvariants } from '../commands/policy.mjs';

const REPO_ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..', '..', '..');
const REPO_VIBE_DIR = path.join(REPO_ROOT, 'flow');

function makePolicySandbox(policy) {
  const sb = makeHookSandbox();
  if (policy !== undefined) {
    const contentDir = path.join(sb.vibeDir, 'content');
    mkdirSync(contentDir, { recursive: true });
    writeFileSync(
      path.join(contentDir, 'policy.json'),
      typeof policy === 'string' ? policy : `${JSON.stringify(policy, null, 2)}\n`,
    );
  }
  return sb;
}

const RULE = (id, match, states, verdict = 'block', reason = `${id} reason`) => ({
  id,
  match,
  states,
  verdict,
  reason,
});

// ---------------------------------------------------------------------------
// The shipped policy.json — the three write invariants as data.
// ---------------------------------------------------------------------------

test('shipped: content/policy.json loads with no errors and exactly three rules', () => {
  const { rules, errors } = loadPolicy(REPO_VIBE_DIR);
  assertEqual(errors, []);
  assertEqual(rules.length, 3, `expected 3 shipped rules, got ${rules.length}`);
});

test('shipped: lessons.md blocks outside its states, allows inside them', () => {
  const { rules } = loadPolicy(REPO_VIBE_DIR);
  const policy = { rules };
  assertEqual(decide(policy, '.spec/lessons.md', 'idle').verdict, 'block');
  assertEqual(decide(policy, '.spec/lessons.md', 'feature.impl').verdict, 'block');
  for (const state of ['feature.compound', 'setup.apply', 'strategy.spec', 'quick.verify']) {
    assertEqual(decide(policy, '.spec/lessons.md', state).verdict, 'allow', `expected allow in ${state}`);
  }
});

test('shipped: root .spec specs block outside strategy.spec/feature.compound/setup.apply', () => {
  const { rules } = loadPolicy(REPO_VIBE_DIR);
  const policy = { rules };
  for (const file of ['.spec/product.md', '.spec/tech.md', '.spec/design.md', '.spec/plan.md']) {
    assertEqual(decide(policy, file, 'idle').verdict, 'block', `${file} should block in idle`);
    for (const state of ['strategy.spec', 'feature.compound', 'setup.apply']) {
      assertEqual(decide(policy, file, state).verdict, 'allow', `${file} should allow in ${state}`);
    }
  }
});

test('shipped: state.json always blocks — no state admits a direct edit', () => {
  const { rules } = loadPolicy(REPO_VIBE_DIR);
  const policy = { rules };
  for (const state of ['idle', 'feature.impl', 'setup.apply', 'strategy.spec']) {
    const result = decide(policy, '.agents/skills/vibe/state.json', state);
    assertEqual(result.verdict, 'block', `state.json must block in ${state}`);
    assertIncludes(result.reason, 'set-state.sh');
  }
});

test('shipped: an unmatched path is allow, with no rule id', () => {
  const { rules } = loadPolicy(REPO_VIBE_DIR);
  const result = decide({ rules }, 'src/index.mjs', 'idle');
  assertEqual(result.verdict, 'allow');
  assertEqual(result.ruleId, null);
});

// ---------------------------------------------------------------------------
// Matching — exact before glob, first match wins.
// ---------------------------------------------------------------------------

test('decide: an exact-path rule matches only that path', () => {
  const policy = { rules: [RULE('r1', 'a/b.md', [])] };
  assertEqual(decide(policy, 'a/b.md', 'idle').ruleId, 'r1');
  assertEqual(decide(policy, 'a/c.md', 'idle').ruleId, null);
});

test('decide: `match` may be an array of exact paths — any one matches', () => {
  const policy = { rules: [RULE('r1', ['a.md', 'b.md'], [])] };
  assertEqual(decide(policy, 'a.md', 'idle').ruleId, 'r1');
  assertEqual(decide(policy, 'b.md', 'idle').ruleId, 'r1');
  assertEqual(decide(policy, 'c.md', 'idle').ruleId, null);
});

test('decide: `*` matches within a path segment, not across `/`', () => {
  const policy = { rules: [RULE('r1', 'docs/*.md', [])] };
  assertEqual(decide(policy, 'docs/a.md', 'idle').ruleId, 'r1');
  assertEqual(decide(policy, 'docs/sub/a.md', 'idle').ruleId, null, '`*` must not cross a `/`');
});

test('decide: `**` matches any run of characters, including `/`', () => {
  const policy = { rules: [RULE('r1', '.spec/features/**', [])] };
  assertEqual(decide(policy, '.spec/features/x/plan.md', 'idle').ruleId, 'r1');
  assertEqual(decide(policy, '.spec/features/x/y/z.md', 'idle').ruleId, 'r1');
  assertEqual(decide(policy, '.spec/other.md', 'idle').ruleId, null);
});

test('decide: an exact rule outranks a broader glob rule even when the glob rule is listed first', () => {
  const policy = {
    rules: [RULE('broad', '.spec/**', [], 'warn'), RULE('narrow', '.spec/lessons.md', [], 'block')],
  };
  const result = decide(policy, '.spec/lessons.md', 'idle');
  assertEqual(result.ruleId, 'narrow', 'exact match must win over an earlier, broader glob rule');
  assertEqual(result.verdict, 'block');
});

test('decide: among same-specificity rules, the first listed wins', () => {
  const policy = { rules: [RULE('first', 'a.md', [], 'warn'), RULE('second', 'a.md', [], 'block')] };
  assertEqual(decide(policy, 'a.md', 'idle').ruleId, 'first');
});

test('decide: a rule with an empty `states` list is never allowed by state', () => {
  const policy = { rules: [RULE('r1', 'a.md', [])] };
  for (const state of ['idle', 'feature.impl', 'a.md']) {
    assertEqual(decide(policy, 'a.md', state).verdict, 'block');
  }
});

test('decide: a leading "./" is normalized before matching, like the bash oracle', () => {
  const policy = { rules: [RULE('r1', 'a.md', [])] };
  assertEqual(decide(policy, './a.md', 'idle').ruleId, 'r1');
});

test('decide: verdict "warn" carries its reason the same way "block" does', () => {
  const policy = { rules: [RULE('r1', 'a.md', [], 'warn', 'careful')] };
  const result = decide(policy, 'a.md', 'idle');
  assertEqual(result.verdict, 'warn');
  assertEqual(result.reason, 'careful');
});

test('decide: an empty rule set is allow for everything (same shape as no policy at all)', () => {
  assertEqual(decide({ rules: [] }, 'anything', 'idle').verdict, 'allow');
  assertEqual(decide(undefined, 'anything', 'idle').verdict, 'allow');
});

// ---------------------------------------------------------------------------
// Adversarial — prototype keys as rule ids, match paths, or the queried path.
// ---------------------------------------------------------------------------

test('adversarial: "__proto__"/"constructor" as a rule id or match path behave like any other string', () => {
  const policy = {
    rules: [RULE('__proto__', '__proto__', []), RULE('constructor', 'constructor', [])],
  };
  const protoResult = decide(policy, '__proto__', 'idle');
  assertEqual(protoResult.ruleId, '__proto__');
  assertEqual(protoResult.verdict, 'block');
  const ctorResult = decide(policy, 'constructor', 'idle');
  assertEqual(ctorResult.ruleId, 'constructor');
  assertEqual(ctorResult.verdict, 'block');
  // Neither must resolve to an inherited function or leak Object.prototype.
  assertEqual(typeof protoResult.ruleId, 'string');
  assertEqual({}.polluted, undefined);
});

test('adversarial: "__proto__"/"constructor" as the QUERIED path never crash or bypass an unrelated rule', () => {
  const policy = { rules: [RULE('r1', 'a.md', [])] };
  for (const hostile of ['__proto__', 'constructor', 'toString', 'hasOwnProperty']) {
    const result = decide(policy, hostile, 'idle');
    assertEqual(result.verdict, 'allow', `${hostile} must not accidentally match rule 'r1'`);
    assertEqual(result.ruleId, null);
  }
});

test('adversarial: a state named "__proto__" is compared as a plain string, matches only if listed', () => {
  const policy = { rules: [RULE('r1', 'a.md', ['__proto__'])] };
  assertEqual(decide(policy, 'a.md', '__proto__').verdict, 'allow');
  assertEqual(decide(policy, 'a.md', 'idle').verdict, 'block');
});

// ---------------------------------------------------------------------------
// Degrade — absent, malformed, or partially-malformed policy.json.
// ---------------------------------------------------------------------------

test('degrade: an ABSENT policy.json loads as zero rules, no errors, allow for everything', () => {
  const sb = makePolicySandbox();
  try {
    const { rules, errors } = loadPolicy(sb.vibeDir);
    assertEqual(rules, []);
    assertEqual(errors, []);
    assertEqual(decide({ rules }, '.spec/lessons.md', 'idle').verdict, 'allow');
  } finally {
    sb.cleanup();
  }
});

test('degrade: an unparseable policy.json reports and degrades to allow for everything', () => {
  const sb = makePolicySandbox('{ this is not json');
  try {
    const { rules, errors } = loadPolicy(sb.vibeDir);
    assertEqual(rules, []);
    assert(errors.length > 0);
    assertIncludes(errors[0], 'policy.json');
    assertEqual(decide({ rules }, '.spec/lessons.md', 'idle').verdict, 'allow');
  } finally {
    sb.cleanup();
  }
});

test('degrade: a policy.json that is a JSON array (not an object) is rejected, not merged', () => {
  const sb = makePolicySandbox('[1, 2, 3]');
  try {
    const { rules, errors } = loadPolicy(sb.vibeDir);
    assertEqual(rules, []);
    assertIncludes(errors.join('\n'), 'is not a JSON object');
  } finally {
    sb.cleanup();
  }
});

test('degrade: a policy.json with no `rules` array reports and yields zero rules', () => {
  const sb = makePolicySandbox({ version: 1 });
  try {
    const { rules, errors } = loadPolicy(sb.vibeDir);
    assertEqual(rules, []);
    assertIncludes(errors.join('\n'), "no 'rules' array");
  } finally {
    sb.cleanup();
  }
});

test('degrade: ONE malformed rule is dropped and reported; the rest of the file still loads', () => {
  const sb = makePolicySandbox({
    rules: [{ match: 'no-id.md', states: [] }, RULE('good', 'good.md', [])],
  });
  try {
    const { rules, errors } = loadPolicy(sb.vibeDir);
    assertEqual(rules.length, 1);
    assertEqual(rules[0].id, 'good');
    assertIncludes(errors.join('\n'), "missing an 'id'");
  } finally {
    sb.cleanup();
  }
});

test('degrade: an unknown verdict string degrades to "block" and is reported', () => {
  const sb = makePolicySandbox({ rules: [{ id: 'r1', match: 'a.md', states: [], verdict: 'nope' }] });
  try {
    const { rules, errors } = loadPolicy(sb.vibeDir);
    assertEqual(rules[0].verdict, 'block');
    assertIncludes(errors.join('\n'), "unknown verdict 'nope'");
  } finally {
    sb.cleanup();
  }
});

test('degrade: loadPolicy on a hostile vibeDir (non-string, empty) never throws', () => {
  for (const bad of [undefined, null, 12, '']) {
    const { rules, errors } = loadPolicy(bad);
    assertEqual(rules, []);
    assertEqual(errors, []);
  }
});

// ---------------------------------------------------------------------------
// renderInvariants — prose generated from the same data decide() enforces.
// ---------------------------------------------------------------------------

test('renderInvariants: one line per rule, naming its match, states, and reason', () => {
  const policy = { rules: [RULE('r1', 'a.md', ['idle'], 'block', 'because reasons')] };
  const text = renderInvariants(policy);
  assertEqual(text, '- `a.md`: block outside `idle` — because reasons');
});

test('renderInvariants: a rule with no states prints "(no state)"', () => {
  const policy = { rules: [RULE('r1', 'a.md', [], 'block', 'never')] };
  assertIncludes(renderInvariants(policy), '(no state)');
});

test('renderInvariants: an empty policy renders an empty string', () => {
  assertEqual(renderInvariants({ rules: [] }), '');
  assertEqual(renderInvariants(undefined), '');
});

test('shipped: renderInvariants on content/policy.json produces one line per rule, in order', () => {
  const { rules } = loadPolicy(REPO_VIBE_DIR);
  const lines = renderInvariants({ rules }).split('\n');
  assertEqual(lines.length, 3);
  assertIncludes(lines[0], 'lessons.md');
  assertIncludes(lines[2], 'state.json');
});

// ---------------------------------------------------------------------------
// The CLI — engine/commands/policy.mjs
// ---------------------------------------------------------------------------

test('cli: `vibe policy decide` prints the bare verdict for allow, and always exits 0', () => {
  const sb = makePolicySandbox({ rules: [] });
  try {
    const result = runDecide(sb.vibeDir, ['src/index.mjs']);
    assertEqual(result.code, 0);
    assertEqual(result.stdout, 'allow\n');
  } finally {
    sb.cleanup();
  }
});

test('cli: `vibe policy decide` prints "block:<reason>" for a block, and still exits 0', () => {
  const sb = makePolicySandbox({ rules: [RULE('r1', 'a.md', [], 'block', 'no direct edits')] });
  try {
    const result = runDecide(sb.vibeDir, ['a.md', 'idle']);
    assertEqual(result.code, 0);
    assertEqual(result.stdout, 'block:no direct edits\n');
  } finally {
    sb.cleanup();
  }
});

test('cli: `vibe policy decide` prints "warn:<reason>" for a warn, and still exits 0', () => {
  const sb = makePolicySandbox({ rules: [RULE('r1', 'a.md', [], 'warn', 'take care')] });
  try {
    const result = runDecide(sb.vibeDir, ['a.md', 'idle']);
    assertEqual(result.code, 0);
    assertEqual(result.stdout, 'warn:take care\n');
  } finally {
    sb.cleanup();
  }
});

test('cli: `vibe policy decide` with no state argument reads the cursor', () => {
  const sb = makeHookSandbox({ cursor: { flow: 'feature', phase: 'compound', feature: 'x', updated: null } });
  const contentDir = path.join(sb.vibeDir, 'content');
  mkdirSync(contentDir, { recursive: true });
  writeFileSync(
    path.join(contentDir, 'policy.json'),
    `${JSON.stringify({ rules: [RULE('lessons', '.spec/lessons.md', ['feature.compound'])] }, null, 2)}\n`,
  );
  try {
    assertEqual(runDecide(sb.vibeDir, ['.spec/lessons.md']).stdout, 'allow\n');
  } finally {
    sb.cleanup();
  }
});

test('cli: `vibe policy decide` with no path is a usage error, the one case that is not a verdict', () => {
  const sb = makePolicySandbox({ rules: [] });
  try {
    const result = runDecide(sb.vibeDir, []);
    assert(result.code !== 0);
    assertEqual(result.stdout, '');
  } finally {
    sb.cleanup();
  }
});

test('cli: `vibe policy list` prints the rule count and one line per rule', () => {
  const sb = makePolicySandbox({
    rules: [RULE('a', 'a.md', ['idle']), RULE('b', 'b.md', [])],
  });
  try {
    const result = runList(sb.vibeDir);
    assertEqual(result.code, 0);
    assertIncludes(result.stdout, '2 rule(s)');
    assertIncludes(result.stdout, 'a  match=a.md  states=idle  verdict=block');
    assertIncludes(result.stdout, 'b  match=b.md  states=(none)  verdict=block');
  } finally {
    sb.cleanup();
  }
});

test('cli: `vibe policy render` prints renderInvariants output', () => {
  const sb = makePolicySandbox({ rules: [RULE('a', 'a.md', ['idle'], 'block', 'why')] });
  try {
    const result = runRenderInvariants(sb.vibeDir);
    assertEqual(result.code, 0);
    assertEqual(result.stdout, '- `a.md`: block outside `idle` — why\n');
  } finally {
    sb.cleanup();
  }
});

test('cli: a malformed policy.json makes `list`/`render` exit 1 and still print what they can', () => {
  const sb = makePolicySandbox('{ nope');
  try {
    const listResult = runList(sb.vibeDir);
    assertEqual(listResult.code, 1);
    assertIncludes(listResult.stdout, '0 rule(s)');
    const renderResult = runRenderInvariants(sb.vibeDir);
    assertEqual(renderResult.code, 1);
  } finally {
    sb.cleanup();
  }
});

// ---------------------------------------------------------------------------
// End-to-end through the real CLI, against the shipped repo policy.
// ---------------------------------------------------------------------------

test('cli end-to-end: `vibe policy --help` exits 0 and shows usage', () => {
  const result = runCli(['policy', '--help']);
  assertEqual(result.code, 0);
  assertIncludes(result.stdout, 'usage: vibe policy');
});

test('cli end-to-end: `vibe policy list` against the real repo prints three rules', () => {
  const result = runCli(['policy', 'list']);
  assertEqual(result.code, 0, `stderr: ${result.stderr}`);
  assertIncludes(result.stdout, '3 rule(s)');
  assertIncludes(result.stdout, 'lessons');
  assertIncludes(result.stdout, 'root-specs');
  assertIncludes(result.stdout, 'cursor');
});

test('cli end-to-end: `vibe policy decide` against the real repo always exits 0', () => {
  const allow = runCli(['policy', 'decide', 'src/index.mjs', 'idle']);
  assertEqual(allow.code, 0);
  assertEqual(allow.stdout, 'allow\n');

  const blocked = runCli(['policy', 'decide', '.agents/skills/vibe/state.json', 'idle']);
  assertEqual(blocked.code, 0, 'decide must exit 0 even for a block verdict');
  assertMatchBlock(blocked.stdout);
});

function assertMatchBlock(stdout) {
  assert(stdout.startsWith('block:'), `expected a block verdict, got ${JSON.stringify(stdout)}`);
}
