// engine/commands/policy.mjs — `vibe policy`, the CLI surface for the write
// invariants (policy.mjs).
//
//   vibe policy decide <path> [state]   allow|warn|block for writing <path>
//                                        now (state defaults to the cursor)
//   vibe policy list                    the loaded rules, one per line
//   vibe policy render                  {{invariants}} prose (renderInvariants)
//
// A later unit points flow/scripts/detect-context.sh's `decide` at this
// command; that hand-off is NOT this file's job — this only has to answer
// correctly on its own.
//
// Exit codes: `decide` ALWAYS exits 0 — the verdict is the stdout line, never
// the exit code, so a hook translates it to its own convention (block -> a
// non-zero exit) rather than this command doing that translation itself.
// `list`/`render` exit 1 only when the policy file itself failed to load.

import { resolveVibeDir } from '../root.mjs';
import { readCursor } from '../cursor.mjs';
import { loadPolicy, decide, renderInvariants } from '../policy.mjs';

const USAGE = [
  'usage: vibe policy decide <path> [state] | list | render',
  '',
  '  decide <path> [state]   print allow | warn:<reason> | block:<reason> for',
  '                           writing <path> now (state defaults to the cursor)',
  '  list                    print the loaded rules',
  '  render                  print the {{invariants}} prose',
].join('\n');

function line(s) {
  return `${s}\n`;
}

// The current cursor's state key, degrading to 'idle' on any failure — the
// same degrade orders.mjs's cursorStateAndFeature() applies, for the same
// reason: a missing or corrupt cursor legitimately reads as idle here, this
// command's job is to answer a decision, not to diagnose the cursor.
function currentState(vibeDir) {
  try {
    return readCursor(vibeDir).state;
  } catch {
    return 'idle';
  }
}

// verdict + reason -> the oracle-shaped stdout line: bare 'allow', or
// 'warn:<reason>' / 'block:<reason>'.
function verdictLine(result) {
  if (result.verdict === 'allow') return 'allow';
  return `${result.verdict}:${result.reason}`;
}

export function runDecide(vibeDir, args) {
  const [target, stateArg] = args;
  if (!target) {
    // A usage error, not a verdict — this is the one case with nothing to
    // print on stdout, so it is the one case allowed a non-zero exit.
    return { code: 1, stdout: '', stderr: line('vibe policy decide: usage: vibe policy decide <path> [state]') };
  }
  const { rules, errors } = loadPolicy(vibeDir);
  const state = typeof stateArg === 'string' && stateArg ? stateArg : currentState(vibeDir);
  const result = decide({ rules }, target, state);
  let stderr = '';
  for (const err of errors) stderr += line(`vibe policy: WARN — ${err}`);
  return { code: 0, stdout: line(verdictLine(result)), stderr };
}

function ruleLine(rule) {
  const match = rule.match.join(', ');
  const states = rule.states.length ? rule.states.join(', ') : '(none)';
  return `${rule.id}  match=${match}  states=${states}  verdict=${rule.verdict}`;
}

export function runList(vibeDir) {
  const { rules, errors } = loadPolicy(vibeDir);
  let stderr = '';
  for (const err of errors) stderr += line(`vibe policy: WARN — ${err}`);
  let stdout = line(`policy: ${rules.length} rule(s)`);
  for (const rule of rules) stdout += line(ruleLine(rule));
  return { code: errors.length ? 1 : 0, stdout, stderr };
}

export function runRenderInvariants(vibeDir) {
  const { rules, errors } = loadPolicy(vibeDir);
  let stderr = '';
  for (const err of errors) stderr += line(`vibe policy: WARN — ${err}`);
  const prose = renderInvariants({ rules });
  return { code: errors.length ? 1 : 0, stdout: prose ? `${prose}\n` : '', stderr };
}

export default async function run(argv, opts = {}) {
  const argv0 = Array.isArray(argv) ? argv : [];
  const [sub, ...rest] = argv0;
  const vibeDir = resolveVibeDir(opts);

  let result;
  if (sub === 'decide') {
    result = runDecide(vibeDir, rest);
  } else if (sub === 'list') {
    result = runList(vibeDir);
  } else if (sub === 'render') {
    result = runRenderInvariants(vibeDir);
  } else if (sub === '--help' || sub === '-h' || sub === undefined) {
    result = { code: 0, stdout: `${USAGE}\n`, stderr: '' };
  } else {
    result = { code: 1, stdout: '', stderr: `${USAGE}\n` };
  }

  if (result.stdout) process.stdout.write(result.stdout);
  if (result.stderr) process.stderr.write(result.stderr);
  return result.code;
}
