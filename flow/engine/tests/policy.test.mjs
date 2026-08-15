// engine/tests/policy.test.mjs — engine/policy.mjs (loadPolicy/decide/
// renderInvariants) and its CLI surface, engine/commands/policy.mjs
// (inject-triggers/1, fix round 1).
//
// Four things are under test:
//   1. THE SHIPPED SIX ARMS — content/policy.json must model all six guarded
//      arms of detect-context.sh's decide() (three block, three warn), each
//      reproducing the oracle's own states AND reason string (including its
//      `(current: $state)` suffix, interpolated) element for element.
//   2. BASH-FAITHFUL MATCHING — a literal or glob pattern matches on any
//      path-boundary suffix of the queried path (an absolute path, a
//      `../`-prefixed one, or one nested under an extra directory), and a
//      wildcard crosses `/` — reproducing bash case-pattern semantics, not a
//      minimatch-style "stays in one directory" dialect.
//   3. DEGRADE — an absent, malformed, or unrecognized-version policy.json
//      must never throw, must answer 'allow' for everything, and must report
//      through `errors`.
//   4. THE CLI — `vibe policy decide` always exits 0, `list`/`render`
//      surface the same data loadPolicy/decide/renderInvariants already
//      prove correct.

import { mkdirSync, writeFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { test, assert, assertEqual, assertIncludes, assertMatch, makeHookSandbox, runCli } from './run.mjs';
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

// A rule with exactly two arms: allow inside `allowStates`, otherwise a
// fixed verdict+reason — the shape most of the shipped block rules share.
const RULE = (id, match, allowStates, verdict = 'block', reason = `${id} reason (current: {state})`) => ({
  id,
  match,
  arms: [
    { states: allowStates, verdict: 'allow', reason: '' },
    { states: '*', verdict, reason },
  ],
});

// A rule with a single catch-all arm — always the same verdict, regardless
// of state (the shape the shipped `cursor` and `generated-docs` rules use).
const ALWAYS_RULE = (id, match, verdict, reason = `${id} always`) => ({
  id,
  match,
  arms: [{ states: '*', verdict, reason }],
});

// A rule built from a caller-supplied arms list, for cases neither shape above covers.
const ARMS_RULE = (id, match, arms) => ({ id, match, arms });

// ---------------------------------------------------------------------------
// The shipped policy.json — all six guarded arms, byte-faithful to the oracle.
// ---------------------------------------------------------------------------

test('shipped: content/policy.json loads with no errors and exactly six rules', () => {
  const { rules, errors } = loadPolicy(REPO_VIBE_DIR);
  assertEqual(errors, []);
  assertEqual(rules.length, 6, `expected 6 shipped rules, got ${rules.length}`);
});

test('shipped: state.json always blocks — no state admits a direct edit', () => {
  const { rules } = loadPolicy(REPO_VIBE_DIR);
  const policy = { rules };
  for (const state of ['idle', 'feature.impl', 'setup.apply', 'strategy.spec']) {
    const result = decide(policy, '.agents/skills/vibe/state.json', state);
    assertEqual(result.verdict, 'block');
    assertEqual(result.reason, 'state.json is written only via set-state.sh, never by direct edit');
  }
});

test('shipped: lessons.md blocks outside its states with the oracle\'s exact reason, allows inside them', () => {
  const { rules } = loadPolicy(REPO_VIBE_DIR);
  const policy = { rules };
  const blocked = decide(policy, '.spec/lessons.md', 'idle');
  assertEqual(blocked.verdict, 'block');
  assertEqual(
    blocked.reason,
    '.spec/lessons.md is writable only during feature.compound, setup.apply, strategy.spec, or quick.verify (current: idle)',
  );
  for (const state of ['feature.compound', 'setup.apply', 'strategy.spec', 'quick.verify']) {
    assertEqual(decide(policy, '.spec/lessons.md', state).verdict, 'allow', `expected allow in ${state}`);
  }
});

test('shipped: root .spec specs block outside their states with the oracle\'s exact reason', () => {
  const { rules } = loadPolicy(REPO_VIBE_DIR);
  const policy = { rules };
  for (const file of ['.spec/product.md', '.spec/tech.md', '.spec/design.md', '.spec/plan.md']) {
    const blocked = decide(policy, file, 'feature.impl');
    assertEqual(blocked.verdict, 'block', `${file} should block in feature.impl`);
    assertEqual(
      blocked.reason,
      'root .spec specs are writable only during strategy.spec, feature.compound, or setup.apply (current: feature.impl)',
    );
    for (const state of ['strategy.spec', 'feature.compound', 'setup.apply']) {
      assertEqual(decide(policy, file, state).verdict, 'allow', `${file} should allow in ${state}`);
    }
  }
});

test('shipped: .spec/features/* warns during impl/fix (nested paths too), allows otherwise', () => {
  const { rules } = loadPolicy(REPO_VIBE_DIR);
  const policy = { rules };
  for (const file of ['.spec/features/x/plan.md', '.spec/features/x/y/z.md']) {
    const warned = decide(policy, file, 'feature.impl');
    assertEqual(warned.verdict, 'warn');
    assertEqual(
      warned.reason,
      '.spec/features edits are frozen during impl/fix — route back to feature.design/plan to change scope (current: feature.impl)',
    );
    assertEqual(decide(policy, file, 'quick.fix').verdict, 'warn');
    assertEqual(decide(policy, file, 'feature.design').verdict, 'allow');
    assertEqual(decide(policy, file, 'idle').verdict, 'allow');
  }
});

test('shipped: CLAUDE.md/AGENTS.md warns unconditionally, in every state', () => {
  const { rules } = loadPolicy(REPO_VIBE_DIR);
  const policy = { rules };
  for (const file of ['CLAUDE.md', 'AGENTS.md']) {
    for (const state of ['idle', 'feature.impl', 'strategy.spec']) {
      const result = decide(policy, file, state);
      assertEqual(result.verdict, 'warn');
      assertEqual(
        result.reason,
        'CLAUDE.md/AGENTS.md active-rules block is generated by regen-active-rules.sh; edits inside the markers are overwritten next compound',
      );
    }
  }
});

test('shipped: src/*|tests/* has three distinct warn reasons plus an allow band', () => {
  const { rules } = loadPolicy(REPO_VIBE_DIR);
  const policy = { rules };
  for (const file of ['src/x.mjs', 'tests/x.test.mjs']) {
    const verify = decide(policy, file, 'feature.verify');
    assertEqual(verify.verdict, 'warn');
    assertEqual(verify.reason, 'verify writes no src — route findings back to impl (set-state.sh feature.impl)');

    const quickVerify = decide(policy, file, 'quick.verify');
    assertEqual(quickVerify.verdict, 'warn');
    assertEqual(quickVerify.reason, 'verify writes no src — route findings back to fix (set-state.sh quick.fix)');

    for (const state of ['feature.impl', 'quick.fix', 'setup.apply']) {
      assertEqual(decide(policy, file, state).verdict, 'allow', `${file} should allow in ${state}`);
    }

    const other = decide(policy, file, 'idle');
    assertEqual(other.verdict, 'warn');
    assertEqual(other.reason, 'source/test edits outside an impl/fix state (current: idle)');
  }
});

test('shipped: an unmatched path is allow, with no rule id', () => {
  const { rules } = loadPolicy(REPO_VIBE_DIR);
  const result = decide({ rules }, 'README.md', 'idle');
  assertEqual(result.verdict, 'allow');
  assertEqual(result.ruleId, null);
});

// ---------------------------------------------------------------------------
// Bash-faithful matching — path-boundary suffixes and slash-crossing globs.
// ---------------------------------------------------------------------------

test('bash-fidelity: an ABSOLUTE path still hits a literal rule (the finding-2 regression)', () => {
  const { rules } = loadPolicy(REPO_VIBE_DIR);
  const result = decide({ rules }, '/home/user/vibe/.spec/lessons.md', 'idle');
  assertEqual(result.verdict, 'block', 'an absolute path must match the same as the bare relative one');
  assertIncludes(result.reason, 'writable only during');
});

test('bash-fidelity: a "../"-prefixed path still hits a literal rule', () => {
  const { rules } = loadPolicy(REPO_VIBE_DIR);
  const result = decide({ rules }, '../.spec/lessons.md', 'idle');
  assertEqual(result.verdict, 'block');
});

test('bash-fidelity: a path nested under an extra directory still hits a literal rule', () => {
  const { rules } = loadPolicy(REPO_VIBE_DIR);
  const result = decide({ rules }, 'worktree/.agents/skills/vibe/state.json', 'idle');
  assertEqual(result.verdict, 'block');
});

test('bash-fidelity: a literal pattern does NOT match a coincidental substring off a path boundary', () => {
  const policy = { rules: [RULE('r1', 'lessons.md', [])] };
  assertEqual(decide(policy, 'not-lessons.md', 'idle').verdict, 'allow', 'no "/" boundary before the match — must not fire');
  assertEqual(decide(policy, 'x/lessons.md', 'idle').verdict, 'block', 'a real "/" boundary — must fire');
});

test('bash-fidelity: `*` crosses `/`, matching nested paths, same as bash case-pattern `*`', () => {
  const policy = { rules: [RULE('r1', 'docs/*', [])] };
  assertEqual(decide(policy, 'docs/a.md', 'idle').verdict, 'block');
  assertEqual(decide(policy, 'docs/sub/a.md', 'idle').verdict, 'block', '`*` must cross `/`, unlike a minimatch-style single-segment star');
  assertEqual(decide(policy, 'docs/', 'idle').verdict, 'block', '`*` matches an empty remainder too');
});

test('bash-fidelity: `?` matches exactly one character, including `/`', () => {
  const policy = { rules: [RULE('r1', 'a?b', [])] };
  assertEqual(decide(policy, 'axb', 'idle').verdict, 'block');
  assertEqual(decide(policy, 'a/b', 'idle').verdict, 'block', '`?` matches `/` too, same as bash');
  assertEqual(decide(policy, 'ab', 'idle').verdict, 'allow', '`?` requires exactly one character');
  assertEqual(decide(policy, 'axxb', 'idle').verdict, 'allow');
});

test('decide: `match` may be an array of exact paths — any one matches', () => {
  const policy = { rules: [RULE('r1', ['a.md', 'b.md'], [])] };
  assertEqual(decide(policy, 'a.md', 'idle').verdict, 'block');
  assertEqual(decide(policy, 'b.md', 'idle').verdict, 'block');
  assertEqual(decide(policy, 'c.md', 'idle').verdict, 'allow');
});

test('decide: an exact rule outranks a broader glob rule even when the glob rule is listed first', () => {
  const policy = {
    rules: [ALWAYS_RULE('broad', '.spec/*', 'warn', 'broad'), ALWAYS_RULE('narrow', '.spec/lessons.md', 'block', 'narrow')],
  };
  const result = decide(policy, '.spec/lessons.md', 'idle');
  assertEqual(result.ruleId, 'narrow', 'exact match must win over an earlier, broader glob rule');
  assertEqual(result.verdict, 'block');
});

test('decide: among same-specificity rules, the first listed wins', () => {
  const policy = { rules: [ALWAYS_RULE('first', 'a.md', 'warn'), ALWAYS_RULE('second', 'a.md', 'block')] };
  assertEqual(decide(policy, 'a.md', 'idle').ruleId, 'first');
});

test('decide: a rule with a single catch-all arm is never allowed by state', () => {
  const policy = { rules: [ALWAYS_RULE('r1', 'a.md', 'block')] };
  for (const state of ['idle', 'feature.impl', 'a.md']) {
    assertEqual(decide(policy, 'a.md', state).verdict, 'block');
  }
});

test('decide: verdict "warn" carries its interpolated reason the same way "block" does', () => {
  const policy = { rules: [ALWAYS_RULE('r1', 'a.md', 'warn', 'careful in {state}')] };
  const result = decide(policy, 'a.md', 'feature.plan');
  assertEqual(result.verdict, 'warn');
  assertEqual(result.reason, 'careful in feature.plan');
});

test('decide: an empty rule set is allow for everything (same shape as no policy at all)', () => {
  assertEqual(decide({ rules: [] }, 'anything', 'idle').verdict, 'allow');
  assertEqual(decide(undefined, 'anything', 'idle').verdict, 'allow');
});

test('decide: a rule with no arm admitting the queried state degrades to allow, not a throw', () => {
  const policy = { rules: [ARMS_RULE('r1', 'a.md', [{ states: ['only-this-state'], verdict: 'block', reason: 'x' }])] };
  assertEqual(decide(policy, 'a.md', 'idle').verdict, 'allow');
  assertEqual(decide(policy, 'a.md', 'only-this-state').verdict, 'block');
});

// ---------------------------------------------------------------------------
// Adversarial — prototype keys as rule ids, match paths, or the queried path.
// ---------------------------------------------------------------------------

test('adversarial: "__proto__"/"constructor" as a rule id or match path behave like any other string', () => {
  const policy = {
    rules: [ALWAYS_RULE('__proto__', '__proto__', 'block'), ALWAYS_RULE('constructor', 'constructor', 'block')],
  };
  const protoResult = decide(policy, '__proto__', 'idle');
  assertEqual(protoResult.ruleId, '__proto__');
  assertEqual(protoResult.verdict, 'block');
  const ctorResult = decide(policy, 'constructor', 'idle');
  assertEqual(ctorResult.ruleId, 'constructor');
  assertEqual(ctorResult.verdict, 'block');
  assertEqual({}.polluted, undefined);
});

test('adversarial: "__proto__"/"constructor" as the QUERIED path never crash or bypass an unrelated rule', () => {
  const policy = { rules: [ALWAYS_RULE('r1', 'a.md', 'block')] };
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

test('adversarial: a glob pattern shaped like a prototype key never crashes compileGlob\'s cache', () => {
  // Not reachable via a bare '__proto__'/'constructor' state or id (neither
  // contains a glob char) — this exercises the cache with a PATTERN that
  // does, since that is compileGlob's actual key space.
  const policy = { rules: [ALWAYS_RULE('r1', '__proto__*', 'block')] };
  assertEqual(decide(policy, '__proto__x', 'idle').verdict, 'block');
  assertEqual({}.polluted, undefined);
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

test('degrade: an unrecognized `version` is refused and reported, never silently treated as v1', () => {
  const sb = makePolicySandbox({ version: 2, rules: [{ id: 'r1', match: 'a.md', arms: [{ states: '*', verdict: 'block' }] }] });
  try {
    const { rules, errors } = loadPolicy(sb.vibeDir);
    assertEqual(rules, []);
    assertIncludes(errors.join('\n'), 'version 2');
  } finally {
    sb.cleanup();
  }
});

test('degrade: an ABSENT `version` is not an error — loads normally', () => {
  const sb = makePolicySandbox({ rules: [{ id: 'r1', match: 'a.md', arms: [{ states: '*', verdict: 'block' }] }] });
  try {
    const { rules, errors } = loadPolicy(sb.vibeDir);
    assertEqual(errors, []);
    assertEqual(rules.length, 1);
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
    rules: [{ match: 'no-id.md', arms: [] }, { id: 'good', match: 'good.md', arms: [{ states: '*', verdict: 'block', reason: 'x' }] }],
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

test('degrade: a rule with no `arms` array (or an empty one) is dropped and reported', () => {
  const sb = makePolicySandbox({
    rules: [
      { id: 'no-arms', match: 'a.md' },
      { id: 'empty-arms', match: 'b.md', arms: [] },
      { id: 'good', match: 'c.md', arms: [{ states: '*', verdict: 'block', reason: 'x' }] },
    ],
  });
  try {
    const { rules, errors } = loadPolicy(sb.vibeDir);
    assertEqual(rules.length, 1);
    assertEqual(rules[0].id, 'good');
    assertIncludes(errors.join('\n'), "rule 'no-arms' has no 'arms' array");
    assertIncludes(errors.join('\n'), "rule 'empty-arms' has no usable 'arms'");
  } finally {
    sb.cleanup();
  }
});

test('degrade: an unknown verdict string on an arm degrades to "block" and is reported', () => {
  const sb = makePolicySandbox({
    rules: [{ id: 'r1', match: 'a.md', arms: [{ states: '*', verdict: 'nope', reason: 'x' }] }],
  });
  try {
    const { rules, errors } = loadPolicy(sb.vibeDir);
    assertEqual(rules[0].arms[0].verdict, 'block');
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

test('renderInvariants: one clause per arm, naming its states and interpolated-as-prose reason', () => {
  const policy = { rules: [RULE('r1', 'a.md', ['idle'], 'block', 'because {state} reasons')] };
  const text = renderInvariants(policy);
  assertEqual(text, '- `a.md` — `idle`: allow; otherwise: block — because the current state reasons');
});

test('renderInvariants: an allow arm never prints a reason clause, even if one is set', () => {
  const policy = { rules: [ARMS_RULE('r1', 'a.md', [{ states: '*', verdict: 'allow', reason: 'ignored' }])] };
  assertEqual(renderInvariants(policy), '- `a.md` — otherwise: allow');
});

test('renderInvariants: an empty policy renders an empty string', () => {
  assertEqual(renderInvariants({ rules: [] }), '');
  assertEqual(renderInvariants(undefined), '');
});

test('shipped: renderInvariants on content/policy.json produces one line per rule, in order', () => {
  const { rules } = loadPolicy(REPO_VIBE_DIR);
  const lines = renderInvariants({ rules }).split('\n');
  assertEqual(lines.length, 6);
  assertIncludes(lines[0], 'state.json');
  assertIncludes(lines[1], 'lessons.md');
  assertIncludes(lines[5], 'src/*');
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

test('cli: `vibe policy decide` prints "block:<reason>" for a block, interpolated, and still exits 0', () => {
  const sb = makePolicySandbox({ rules: [ALWAYS_RULE('r1', 'a.md', 'block', 'no direct edits in {state}')] });
  try {
    const result = runDecide(sb.vibeDir, ['a.md', 'idle']);
    assertEqual(result.code, 0);
    assertEqual(result.stdout, 'block:no direct edits in idle\n');
  } finally {
    sb.cleanup();
  }
});

test('cli: `vibe policy decide` prints "warn:<reason>" for a warn, and still exits 0', () => {
  const sb = makePolicySandbox({ rules: [ALWAYS_RULE('r1', 'a.md', 'warn', 'take care')] });
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

test('cli: `vibe policy list` prints the rule count and one line per rule, arms included', () => {
  const sb = makePolicySandbox({
    rules: [RULE('a', 'a.md', ['idle']), ALWAYS_RULE('b', 'b.md', 'block')],
  });
  try {
    const result = runList(sb.vibeDir);
    assertEqual(result.code, 0);
    assertIncludes(result.stdout, '2 rule(s)');
    assertIncludes(result.stdout, 'a  match=a.md  arms=[idle->allow, *->block]');
    assertIncludes(result.stdout, 'b  match=b.md  arms=[*->block]');
  } finally {
    sb.cleanup();
  }
});

test('cli: `vibe policy render` prints renderInvariants output', () => {
  const sb = makePolicySandbox({ rules: [RULE('a', 'a.md', ['idle'], 'block', 'why')] });
  try {
    const result = runRenderInvariants(sb.vibeDir);
    assertEqual(result.code, 0);
    assertEqual(result.stdout, '- `a.md` — `idle`: allow; otherwise: block — why\n');
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

test('cli end-to-end: `vibe policy list` against the real repo prints six rules', () => {
  const result = runCli(['policy', 'list']);
  assertEqual(result.code, 0, `stderr: ${result.stderr}`);
  assertIncludes(result.stdout, '6 rule(s)');
  for (const id of ['cursor', 'lessons', 'root-specs', 'features-frozen', 'generated-docs', 'src-tests']) {
    assertIncludes(result.stdout, id);
  }
});

test('cli end-to-end: `vibe policy decide` against the real repo always exits 0', () => {
  const allow = runCli(['policy', 'decide', 'src/index.mjs', 'feature.impl']);
  assertEqual(allow.code, 0);
  assertEqual(allow.stdout, 'allow\n');

  const blocked = runCli(['policy', 'decide', '.agents/skills/vibe/state.json', 'idle']);
  assertEqual(blocked.code, 0, 'decide must exit 0 even for a block verdict');
  assertMatch(blocked.stdout, /^block:/);
});

test('cli end-to-end: `vibe policy decide` against the real repo reproduces the absolute-path finding', () => {
  const result = runCli(['policy', 'decide', '/home/user/vibe/.spec/lessons.md', 'idle']);
  assertEqual(result.code, 0);
  assertMatch(result.stdout, /^block:/);
});
