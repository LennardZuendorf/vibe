// engine/tests/policy.test.mjs — engine/policy.mjs (loadPolicy/decide/
// renderInvariants) and its CLI surface, engine/commands/policy.mjs
// (inject-triggers/1, fix round 1; inject-triggers/2 adds the last three
// sections).
//
// Seven things are under test — the first four are unit 1's, the last three
// are unit 2's: 5. THE DIFFERENTIAL MATRIX — every guarded path x all 13
// machine states driven through detect-context.sh with node present (the
// engine answers) and with node stripped from PATH (its permanent bash branch
// answers), asserted byte-identical on stdout AND exit code, with the matrix
// asserting its own population; 6. DELEGATION IS LIVE — a rule planted in
// policy.json that only the engine can see; 7. GENERATED PROSE —
// `{{invariants}}` and the block that uses it, asserted against the rule set
// rather than a hand-copied sentence.
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

import { cpSync, mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import {
  test,
  assert,
  assertEqual,
  assertIncludes,
  assertMatch,
  makeHookSandbox,
  mkTempRoot,
  runCli,
  runCommand,
} from './run.mjs';
import { loadPolicy, decide, renderInvariants } from '../policy.mjs';
import { runDecide, runList, runRenderInvariants, parseLeadingOptions } from '../commands/policy.mjs';
import { buildResolver, loadContent, renderChannel } from '../content.mjs';

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

// ---------------------------------------------------------------------------
// `--vibe-dir` — the pin detect-context.sh delegates with (inject-triggers/2).
// ---------------------------------------------------------------------------

test('option: `--vibe-dir <dir>` is parsed off the front and leaves the subcommand argv intact', () => {
  assertEqual(parseLeadingOptions(['--vibe-dir', '/x', 'decide', 'a.md', 'idle']), {
    argv: ['decide', 'a.md', 'idle'],
    vibeDir: '/x',
  });
  assertEqual(parseLeadingOptions(['--vibe-dir=/x', 'list']), { argv: ['list'], vibeDir: '/x' });
  assertEqual(parseLeadingOptions(['decide', 'a.md']), { argv: ['decide', 'a.md'], vibeDir: undefined });
});

test('option: `--vibe-dir` is LEADING-only — a path operand spelled like the flag is still an operand', () => {
  // The flag is stripped only while it is at the head of argv, so
  // `decide --vibe-dir` asks about a file NAMED `--vibe-dir` rather than
  // silently eating the one argument `decide` needs.
  assertEqual(parseLeadingOptions(['decide', '--vibe-dir', 'idle']), {
    argv: ['decide', '--vibe-dir', 'idle'],
    vibeDir: undefined,
  });
});

test('option: a `--vibe-dir` with no value falls back to normal resolution rather than erroring', () => {
  assertEqual(parseLeadingOptions(['--vibe-dir']), { argv: [], vibeDir: undefined });
});

test('cli end-to-end: `--vibe-dir` pins WHICH install answers, whatever the cwd', () => {
  // Run from a directory that is not the repo, with the flag pointing at the
  // repo's own flow/ — the verdict must still come from this repo's policy.
  const elsewhere = mkTempRoot('vibe-policy-cwd-');
  try {
    const pinned = runCli(['policy', '--vibe-dir', REPO_VIBE_DIR, 'decide', '.spec/lessons.md', 'idle'], {
      cwd: elsewhere,
      unsetEnv: ['CLAUDE_PROJECT_DIR'],
    });
    assertEqual(pinned.code, 0, `stderr: ${pinned.stderr}`);
    assertMatch(pinned.stdout, /^block:/);

    // Discriminating control: the SAME invocation without the pin, from the
    // same foreign cwd, resolves no policy at all and answers allow — which is
    // exactly why detect-context.sh passes the pin and refuses to delegate
    // when the policy file is missing.
    const unpinned = runCli(['policy', 'decide', '.spec/lessons.md', 'idle'], {
      cwd: elsewhere,
      unsetEnv: ['CLAUDE_PROJECT_DIR'],
    });
    assertEqual(unpinned.stdout, 'allow\n');
  } finally {
    rmSync(elsewhere, { recursive: true, force: true });
  }
});

// ---------------------------------------------------------------------------
// THE DIFFERENTIAL MATRIX (inject-triggers/2).
//
// `detect-context.sh decide` has TWO branches — the engine (`vibe policy
// decide`) when node is available, and its own bash `case` ladder when it is
// not. The bash branch is permanent, not transitional: it backs a hard block
// and a target without node must still be enforced. Two implementations of one
// policy is exactly the shape that drifts, so every guarded path x all 13
// machine states is driven through BOTH branches here and asserted
// byte-identical on stdout AND exit code.
//
// The no-node leg is a PATH shim that deliberately omits `node` (and asserts
// that postcondition before running anything through it) — a leg that quietly
// still had node would be comparing the engine against itself, which is the
// vacuous green this repo has a recorded lesson about. The matrix also asserts
// its own population: a floor on comparisons actually run, that all 13 states
// and every shipped rule are represented, and that the verdicts observed are
// not all the same value.
// ---------------------------------------------------------------------------

const DETECT_SH = path.join(REPO_VIBE_DIR, 'scripts', 'detect-context.sh');
const MACHINE_STATES = Object.keys(
  JSON.parse(readFileSync(path.join(REPO_VIBE_DIR, 'state-machine.json'), 'utf8')).states,
).sort();

// One bash process per leg, not one per cell: the driver loops the matrix
// itself and prints `path \t state \t rc \t stdout`. 13 states x ~26 paths is
// ~340 cells per leg; spawning each from Node would dominate the suite runtime
// for no extra fidelity.
const MATRIX_DRIVER = [
  'set -u',
  "printf '%s\\n' \"$VIBE_PATHS\" | while IFS= read -r p; do",
  "  printf '%s\\n' \"$VIBE_STATES\" | while IFS= read -r s; do",
  '    out="$(bash "$VIBE_DETECT" decide "$p" "$s" 2>/dev/null)" && rc=0 || rc=$?',
  "    printf '%s\\t%s\\t%s\\t%s\\n' \"$p\" \"$s\" \"$rc\" \"$out\"",
  '  done',
  'done',
  '',
].join('\n');

// A PATH carrying the tools the bash branch needs and NOT node. The tool list
// mirrors flow/tests/adapters/run.sh's own mkshim(); `node` is absent by
// deliberate omission, which the caller then asserts rather than assumes.
const SHIM_BUILDER = [
  'set -eu',
  'dir="$1"',
  'mkdir -p "$dir"',
  'for t in bash sh mkdir dirname basename date mktemp mv cp rm rmdir sed grep head tail cat env awk find readlink ln chmod cmp diff sort cksum jq git; do',
  '  p="$(command -v "$t" 2>/dev/null)" && ln -sf "$p" "$dir/$t" || true',
  'done',
  '',
].join('\n');

function makeMatrixHarness() {
  const dir = mkTempRoot('vibe-policy-matrix-');
  const driverPath = path.join(dir, 'driver.sh');
  writeFileSync(driverPath, MATRIX_DRIVER);
  const builderPath = path.join(dir, 'shim.sh');
  writeFileSync(builderPath, SHIM_BUILDER);
  const shimDir = path.join(dir, 'no-node-path');
  const built = runCommand('bash', [builderPath, shimDir]);
  assert(built.code === 0, `no-node PATH shim failed to build: ${built.stderr}`);

  // Postconditions, asserted rather than assumed (a shim that still exposed
  // node would make the whole matrix compare the engine against itself).
  const nodeGone = runCommand('bash', ['-c', 'command -v node'], { env: { PATH: shimDir } });
  assert(nodeGone.code !== 0, `the no-node shim still exposes node at ${nodeGone.stdout.trim()}`);
  const bashThere = runCommand('bash', ['-c', 'command -v bash'], { env: { PATH: shimDir } });
  assertEqual(bashThere.code, 0, 'the no-node shim must still carry bash');
  const nodeHere = runCommand('bash', ['-c', 'command -v node']);
  assertEqual(nodeHere.code, 0, 'the node leg needs node on the ambient PATH');

  function runLeg(paths, states, { noNode }) {
    const env = {
      VIBE_DETECT: DETECT_SH,
      VIBE_PATHS: paths.join('\n'),
      VIBE_STATES: states.join('\n'),
    };
    if (noNode) env.PATH = shimDir;
    const result = runCommand('bash', [driverPath], { env, unsetEnv: ['CLAUDE_PROJECT_DIR'] });
    assertEqual(result.code, 0, `matrix driver failed: ${result.stderr}`);
    const rows = result.stdout.split('\n').filter((l) => l !== '');
    assertEqual(
      rows.length,
      paths.length * states.length,
      `driver produced ${rows.length} rows, expected ${paths.length * states.length} — a verdict spanning lines would corrupt the comparison`,
    );
    return rows;
  }

  return { dir, shimDir, runLeg, cleanup: () => rmSync(dir, { recursive: true, force: true }) };
}

// The guarded paths, DERIVED from the shipped rules rather than hand-listed —
// a rule added to policy.json widens this matrix on its own. Each rule
// contributes its own patterns only: a literal pattern plus the three
// path-boundary spellings detect-context.sh's `PATTERN|*/PATTERN` idiom
// accepts, and a glob pattern instantiated at one and two extra segments.
function guardedPathsFor(rules) {
  const out = [];
  for (const rule of rules) {
    for (const pattern of rule.match) {
      if (/[*?]/.test(pattern)) {
        out.push(pattern.replace(/\*/g, 'sample.md'), pattern.replace(/\*/g, 'nested/dir/sample.md'));
      } else {
        out.push(pattern, `/abs/root/${pattern}`, `worktree/${pattern}`, `./${pattern}`);
      }
    }
  }
  return out;
}

// Unmatched controls: a matrix of guarded paths alone cannot tell "the two
// branches agree" from "both branches block everything".
const CONTROL_PATHS = ['README.md', 'docs/x.md', 'flow/engine/policy.mjs', 'srcx/a.js'];

test('differential matrix: every guarded path x all 13 states is byte-identical through the engine and the bash branch', () => {
  const { rules } = loadPolicy(REPO_VIBE_DIR);
  const paths = [...guardedPathsFor(rules), ...CONTROL_PATHS];
  const harness = makeMatrixHarness();
  try {
    const engineRows = harness.runLeg(paths, MACHINE_STATES, { noNode: false });
    const bashRows = harness.runLeg(paths, MACHINE_STATES, { noNode: true });

    const mismatches = [];
    for (let i = 0; i < engineRows.length; i += 1) {
      if (engineRows[i] !== bashRows[i]) mismatches.push(`engine: ${engineRows[i]}\n  bash: ${bashRows[i]}`);
    }
    assertEqual(mismatches, [], `branches diverged on ${mismatches.length} cell(s):\n${mismatches.join('\n')}`);

    // --- the matrix asserts its own population -----------------------------
    assertEqual(MACHINE_STATES.length, 13, 'the machine no longer has 13 states — update the matrix, do not shrink it');
    assert(paths.length >= 20, `path axis collapsed to ${paths.length} paths`);
    assert(engineRows.length >= 250, `only ${engineRows.length} comparisons ran — the matrix is not covering the surface`);

    // Every state actually appears in a row, and every shipped rule is the
    // deciding rule for at least one cell: a matrix whose paths all fell
    // through to the same rule would pass the count floor while proving
    // nothing about the rest of the table.
    const seenStates = new Set(engineRows.map((row) => row.split('\t')[1]));
    assertEqual([...seenStates].sort(), MACHINE_STATES);
    const decidingRules = new Set();
    const verdictKinds = new Set();
    for (const row of engineRows) {
      const [p, s, rc, out] = row.split('\t');
      assertEqual(rc, '0', `decide must exit 0 for every verdict (${p} @ ${s})`);
      decidingRules.add(decide({ rules }, p, s).ruleId);
      verdictKinds.add(out === 'allow' ? 'allow' : out.split(':')[0]);
    }
    for (const rule of rules) {
      assert(decidingRules.has(rule.id), `no matrix cell was decided by rule '${rule.id}'`);
    }
    assertEqual([...verdictKinds].sort(), ['allow', 'block', 'warn'], 'the matrix must observe all three verdicts');
  } finally {
    harness.cleanup();
  }
});

test('differential matrix: the EMPTY-path row is asserted explicitly, not normalized away', () => {
  // The one row the matrix cannot express as a verdict comparison. Through
  // detect-context.sh the two branches are still byte-identical, because the
  // usage guard sits UPSTREAM of the delegation — the empty path never reaches
  // either branch. The engine CLI called directly answers the same situation
  // with its OWN usage text on stderr and exit 1, which is a different string
  // for a different entry point; it is asserted here rather than normalized so
  // that nobody later "fixes" the divergence by making the two texts equal and
  // silently changes the enforcer's stdout contract.
  const harness = makeMatrixHarness();
  try {
    const legs = [
      ['engine leg', {}],
      ['no-node leg', { PATH: harness.shimDir }],
    ];
    for (const [label, env] of legs) {
      const result = runCommand('bash', [DETECT_SH, 'decide'], { env, unsetEnv: ['CLAUDE_PROJECT_DIR'] });
      assertEqual(result.code, 1, `${label}: empty path must exit 1`);
      assertEqual(result.stdout, '', `${label}: nothing on stdout`);
      assertEqual(result.stderr, 'usage: detect-context.sh decide <path> [state]\n', `${label}: the oracle's usage text`);
    }
  } finally {
    harness.cleanup();
  }

  const direct = runCli(['policy', 'decide']);
  assertEqual(direct.code, 1);
  assertEqual(direct.stdout, '');
  assertIncludes(direct.stderr, 'usage: vibe policy decide <path> [state]');
});

test('differential matrix: node absent changes nothing about the exit-code contract', () => {
  const harness = makeMatrixHarness();
  try {
    for (const env of [{}, { PATH: harness.shimDir }]) {
      for (const [target, state, expected] of [
        ['.spec/lessons.md', 'idle', /^block:/],
        ['.spec/lessons.md', 'feature.compound', /^allow$/],
        ['src/a.js', 'feature.verify', /^warn:/],
      ]) {
        const result = runCommand('bash', [DETECT_SH, 'decide', target, state], { env, unsetEnv: ['CLAUDE_PROJECT_DIR'] });
        assertEqual(result.code, 0, `${target} @ ${state}: decide always exits 0`);
        assertMatch(result.stdout.replace(/\n$/, ''), expected);
      }
    }
  } finally {
    harness.cleanup();
  }
});

// ---------------------------------------------------------------------------
// KNOWN DIVERGENCE — cross-rule paths (a path matched by BOTH an exact rule
// and an earlier glob rule).
//
// detect-context.sh's bash branch reads its `case` ladder top to bottom, so
// the FIRST arm wins; the engine ranks every exact match above every glob
// match irrespective of file order (policy.mjs, pinned by its own tests). The
// two orders only ever disagree on a path that two rules both claim —
// `.spec/features/AGENTS.md` is the only such shape the shipped six produce.
//
// This is pinned, not papered over, and the direction is what makes it
// acceptable: the engine's verdict is never LESS strict than bash's. The
// property is asserted over the whole cross-rule set, so a future rule that
// made the engine the LOOSER branch fails here rather than in a session.
// ---------------------------------------------------------------------------

const SEVERITY = { allow: 0, warn: 1, block: 2 };

function verdictKind(line) {
  return line === 'allow' ? 'allow' : line.split(':')[0];
}

const CROSS_RULE_PATHS = ['.spec/features/AGENTS.md', '.spec/features/x/CLAUDE.md'];

test('known divergence: on a cross-rule path the engine is stricter than bash, never looser', () => {
  const harness = makeMatrixHarness();
  try {
    const engineRows = harness.runLeg(CROSS_RULE_PATHS, MACHINE_STATES, { noNode: false });
    const bashRows = harness.runLeg(CROSS_RULE_PATHS, MACHINE_STATES, { noNode: true });
    let differed = 0;
    for (let i = 0; i < engineRows.length; i += 1) {
      const engineOut = engineRows[i].split('\t')[3];
      const bashOut = bashRows[i].split('\t')[3];
      if (engineOut !== bashOut) differed += 1;
      assert(
        SEVERITY[verdictKind(engineOut)] >= SEVERITY[verdictKind(bashOut)],
        `engine is LOOSER than bash on ${engineRows[i]} (bash: ${bashOut})`,
      );
    }
    // The control that keeps this test honest: if the two branches stopped
    // diverging here at all, this file would be asserting a property of an
    // empty population and the comment above would be stale.
    assert(differed > 0, 'the cross-rule divergence is gone — delete this test and its comment, do not leave it asserting nothing');
  } finally {
    harness.cleanup();
  }
});

// ---------------------------------------------------------------------------
// Delegation is LIVE — a mutant proves which branch answered.
//
// Every assertion above compares two branches that are supposed to agree, so
// none of them can tell "the engine answered" from "the delegation silently
// fell through to bash". This plants a rule that exists ONLY in the engine's
// data and checks that the enforcer reports it.
// ---------------------------------------------------------------------------

function makeInstallSandbox() {
  const dir = mkTempRoot('vibe-policy-install-');
  const vibeDir = path.join(dir, '.agents', 'skills', 'vibe');
  mkdirSync(vibeDir, { recursive: true });
  // Everything the delegation needs: the script, the engine, the data.
  cpSync(path.join(REPO_VIBE_DIR, 'scripts'), path.join(vibeDir, 'scripts'), { recursive: true });
  cpSync(path.join(REPO_VIBE_DIR, 'engine'), path.join(vibeDir, 'engine'), {
    recursive: true,
    filter: (src) => path.basename(src) !== 'tests',
  });
  cpSync(path.join(REPO_VIBE_DIR, 'content'), path.join(vibeDir, 'content'), { recursive: true });
  cpSync(path.join(REPO_VIBE_DIR, 'state-machine.json'), path.join(vibeDir, 'state-machine.json'));
  const policyPath = path.join(vibeDir, 'content', 'policy.json');
  const detectPath = path.join(vibeDir, 'scripts', 'detect-context.sh');
  return { dir, vibeDir, policyPath, detectPath, cleanup: () => rmSync(dir, { recursive: true, force: true }) };
}

test('delegation is live: a rule planted in policy.json only reaches the verdict through the engine', () => {
  const sb = makeInstallSandbox();
  const sentinel = 'sentinel-mutant.md';
  try {
    const baseline = runCommand('bash', [sb.detectPath, 'decide', sentinel, 'idle'], { unsetEnv: ['CLAUDE_PROJECT_DIR'] });
    assertEqual(baseline.stdout, 'allow\n', 'the sentinel path must be unguarded before the mutation');

    const policy = JSON.parse(readFileSync(sb.policyPath, 'utf8'));
    policy.rules.push({
      id: 'sentinel',
      match: sentinel,
      arms: [{ states: '*', verdict: 'block', reason: 'planted by the differential test' }],
    });
    writeFileSync(sb.policyPath, `${JSON.stringify(policy, null, 2)}\n`);
    // Confirm the mutation actually landed before believing either result.
    assertIncludes(readFileSync(sb.policyPath, 'utf8'), 'planted by the differential test');

    const withNode = runCommand('bash', [sb.detectPath, 'decide', sentinel, 'idle'], { unsetEnv: ['CLAUDE_PROJECT_DIR'] });
    assertEqual(
      withNode.stdout,
      'block:planted by the differential test\n',
      'with node present the verdict must come from policy.json — it did not, so decide never reached the engine',
    );

    const harness = makeMatrixHarness();
    try {
      const withoutNode = runCommand('bash', [sb.detectPath, 'decide', sentinel, 'idle'], {
        env: { PATH: harness.shimDir },
        unsetEnv: ['CLAUDE_PROJECT_DIR'],
      });
      assertEqual(withoutNode.stdout, 'allow\n', 'without node the bash branch answers, and it has never heard of the planted rule');
    } finally {
      harness.cleanup();
    }
  } finally {
    sb.cleanup();
  }
});

test('delegation refuses to answer from an EMPTY policy: no data means the bash branch decides', () => {
  // loadPolicy degrades an absent policy.json to "no rules", which reads as
  // allow for every path. Delegating in that state would turn a missing data
  // file into a silent unblock of the hard blocks, so detect-context.sh
  // requires the file to exist before it hands over.
  const sb = makeInstallSandbox();
  try {
    rmSync(sb.policyPath, { force: true });
    const result = runCommand('bash', [sb.detectPath, 'decide', '.spec/lessons.md', 'idle'], {
      unsetEnv: ['CLAUDE_PROJECT_DIR'],
    });
    assertEqual(result.code, 0);
    assertMatch(result.stdout, /^block:/, 'a missing policy.json must NOT downgrade a hard block to allow');
  } finally {
    sb.cleanup();
  }
});

// ---------------------------------------------------------------------------
// GENERATED INVARIANT PROSE — {{invariants}} and the block that uses it.
//
// Asserted against the RULE SET, never against a hand-copied sentence: a
// sentence copied into a test is the same defect as a sentence copied into
// prose, one indirection further out.
// ---------------------------------------------------------------------------

const REPO_CTX = {
  root: REPO_ROOT,
  vibeDir: REPO_VIBE_DIR,
  skillsDir: path.join(REPO_ROOT, '.agents', 'skills'),
};

test('{{invariants}}: resolves to renderInvariants over the shipped policy', () => {
  const content = loadContent(REPO_CTX.root, REPO_CTX.vibeDir);
  const resolve = buildResolver(REPO_CTX, content);
  const { rules } = loadPolicy(REPO_VIBE_DIR);
  assertEqual(resolve('invariants'), renderInvariants({ rules }).trim());
  assertEqual(resolve('invariants').split('\n').length, rules.length);
});

test('{{invariants}}: an absent policy degrades to an empty string, never a throw', () => {
  const sb = makeHookSandbox();
  try {
    const content = loadContent(sb.root, sb.vibeDir);
    const resolve = buildResolver({ root: sb.root, vibeDir: sb.vibeDir, skillsDir: sb.skillsDir }, content);
    assertEqual(resolve('invariants'), '');
  } finally {
    sb.cleanup();
  }
});

test('shipped block flow.invariants renders every rule and every arm state from the data', () => {
  const content = loadContent(REPO_CTX.root, REPO_CTX.vibeDir);
  const block = content.blocks.get('flow.invariants');
  assert(block !== undefined, 'flow/content/blocks/flow/invariants.md did not load as block id flow.invariants');
  assertIncludes(block.body, '{{invariants}}');

  // Compose it through a channel of its own, so the placeholder is resolved by
  // the real renderChannel path rather than by hand.
  const synthetic = {
    ...content,
    channels: {
      probe: { name: 'probe', blocks: ['flow.invariants'], render: 'body', budget: 0, enabled: true },
    },
  };
  const result = renderChannel('probe', REPO_CTX, synthetic);
  assertEqual(result.errors, []);
  assertEqual(result.unresolved, []);

  const { rules } = loadPolicy(REPO_VIBE_DIR);
  assert(rules.length > 0, 'no shipped rules — this assertion would be vacuous');
  for (const rule of rules) {
    for (const pattern of rule.match) {
      assertIncludes(result.text, `\`${pattern}\``, `rendered prose omits the guarded path ${pattern}`);
    }
    for (const arm of rule.arms) {
      if (arm.states === '*') continue;
      for (const state of arm.states) {
        assertIncludes(result.text, `\`${state}\``, `rendered prose omits state ${state} of rule ${rule.id}`);
      }
    }
  }
  assert(!result.text.includes('{{'), 'an unresolved placeholder survived into the rendered prose');
});

test('shipped block flow.invariants states the same writable-state set the enforcer applies', () => {
  // The parity that flow/tests/run.sh used to assert against hand-authored
  // doctrine prose (inject-triggers/2 retires it there): for each rule, the
  // states named in the generated prose are exactly the states `decide` lets
  // through — checked through decide(), not by re-reading policy.json.
  const { rules } = loadPolicy(REPO_VIBE_DIR);
  const content = loadContent(REPO_CTX.root, REPO_CTX.vibeDir);
  const resolve = buildResolver(REPO_CTX, content);
  const lines = resolve('invariants').split('\n');
  assertEqual(lines.length, rules.length);

  let checked = 0;
  rules.forEach((rule, idx) => {
    const allowArm = rule.arms.find((arm) => arm.verdict === 'allow' && arm.states !== '*');
    if (!allowArm) return;
    const enforced = MACHINE_STATES.filter(
      (state) => decide({ rules }, rule.match[0], state).verdict === 'allow',
    );
    assertEqual(allowArm.states.slice().sort(), enforced.slice().sort(), `rule ${rule.id}`);
    for (const state of enforced) assertIncludes(lines[idx], `\`${state}\``);
    checked += 1;
  });
  assert(checked >= 3, `only ${checked} rule(s) carried a state-gated allow arm — the parity check examined almost nothing`);
});
