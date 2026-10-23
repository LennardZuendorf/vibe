// engine/commands/policy.mjs — `vibe policy`, the CLI surface for the write
// invariants (policy.mjs).
//
//   vibe policy decide <path> [state]   allow|warn|block for writing <path>
//                                        now (state defaults to the cursor)
//   vibe policy list                    the loaded rules, one per line
//   vibe policy render                  {{invariants}} prose (renderInvariants)
//
// A LEADING `--vibe-dir <dir>` (or `--vibe-dir=<dir>`) pins which install's
// policy data answers, instead of letting the usual self-relative/marker
// resolution pick. flow/scripts/detect-context.sh passes the directory it
// already self-located into, which is what makes the delegation hermetic: the
// enforcer and the engine read the same policy.json by construction, whatever
// the cwd or the ambient environment happens to be. Parsed LEADING-only, so a
// path argument spelled `--vibe-dir` is still addressable as `decide`'s own
// operand and can never be swallowed as an option.
//
// Exit codes. The verdict is NEVER the exit code — a hook translates the
// stdout line to its own convention (block -> a non-zero exit), this command
// does not do that translation itself. So every ANSWERED decision, allow or
// warn or block alike, exits 0.
//
//   0   an answered verdict, printed on stdout
//   1   usage error (no path)
//   2   REFUSED: the policy could not be loaded as a usable rule set — the
//       file reported errors, or it loaded to zero rules. Nothing on stdout.
//
// Exit 2 is a correctness tooth, not tidiness (inject-triggers/2 review,
// Critical). loadPolicy() degrades EVERY defect to `rules: []` — truncated
// JSON, a version this engine does not understand, a missing `rules` key, a
// zero-length file, an unreadable path, or an explicit empty list — and
// `decide` over zero rules answers `allow` for every path on earth. Reported
// as exit 0 with `allow` on stdout, that turns a partial write or an
// engine/data version skew into the silent disappearance of every hard block,
// for a caller that has no way to tell the two apart. Refusing instead lets
// flow/scripts/detect-context.sh fall back to its own bash branch, which
// carries the same policy hardcoded and cannot be corrupted by a data file.
//
// `list`/`render` exit 1 only when the policy file itself failed to load;
// they are diagnostics, and printing what loaded is the useful answer there.

import { resolveVibeDir } from '../root.mjs';
import { readCursor } from '../cursor.mjs';
import { loadPolicy, decide, renderInvariants } from '../policy.mjs';

const USAGE = [
  'usage: vibe policy [--vibe-dir <dir>] decide <path> [state] | list | render',
  '',
  '  decide <path> [state]   print allow | warn:<reason> | block:<reason> for',
  '                           writing <path> now (state defaults to the cursor)',
  '  list                    print the loaded rules',
  '  render                  print the {{invariants}} prose',
  '  --vibe-dir <dir>        read the policy from <dir>/content/policy.json',
].join('\n');

const VIBE_DIR_FLAG = '--vibe-dir';

// Pulls a leading `--vibe-dir <dir>` / `--vibe-dir=<dir>` off argv. Returns
// the remaining argv and the directory (undefined when the flag is absent, so
// the caller's normal resolution still applies). A flag with no value is
// dropped rather than treated as an error: this command's contract is to
// answer a verdict, and falling back to normal resolution is the recoverable
// direction.
export function parseLeadingOptions(argv) {
  const rest = [...argv];
  let vibeDir;
  while (rest.length > 0) {
    const head = rest[0];
    if (head === VIBE_DIR_FLAG) {
      rest.shift();
      const value = rest.shift();
      if (typeof value === 'string' && value) vibeDir = value;
      continue;
    }
    if (typeof head === 'string' && head.startsWith(`${VIBE_DIR_FLAG}=`)) {
      rest.shift();
      const value = head.slice(VIBE_DIR_FLAG.length + 1);
      if (value) vibeDir = value;
      continue;
    }
    break;
  }
  return { argv: rest, vibeDir };
}

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

export const DECIDE_REFUSED = 2;

export function runDecide(vibeDir, args) {
  const [target, stateArg] = args;
  if (!target) {
    // A usage error, not a verdict — nothing to print on stdout.
    return { code: 1, stdout: '', stderr: line('vibe policy decide: usage: vibe policy decide <path> [state]') };
  }
  const { rules, errors } = loadPolicy(vibeDir);
  let stderr = '';
  for (const err of errors) stderr += line(`vibe policy: WARN — ${err}`);

  // See the exit-code block in this file's header. An unusable policy is
  // refused, never answered — and with NOTHING on stdout, so a caller that
  // ignores exit codes still cannot read an `allow` out of it.
  if (errors.length > 0 || rules.length === 0) {
    stderr += line(
      `vibe policy decide: refusing to answer — no usable rules loaded${errors.length ? '' : ' (the policy loaded to an empty rule set)'}`,
    );
    return { code: DECIDE_REFUSED, stdout: '', stderr };
  }

  const state = typeof stateArg === 'string' && stateArg ? stateArg : currentState(vibeDir);
  const result = decide({ rules }, target, state);
  return { code: 0, stdout: line(verdictLine(result)), stderr };
}

function armSummary(arm) {
  const states = arm.states === '*' ? '*' : arm.states.join('|') || '(none)';
  return `${states}->${arm.verdict}`;
}

function ruleLine(rule) {
  const match = rule.match.join(', ');
  const arms = rule.arms.map(armSummary).join(', ');
  return `${rule.id}  match=${match}  arms=[${arms}]`;
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
  const { argv: argv0, vibeDir: pinnedVibeDir } = parseLeadingOptions(Array.isArray(argv) ? argv : []);
  const [sub, ...rest] = argv0;
  const vibeDir = resolveVibeDir(pinnedVibeDir ? { ...opts, vibeDir: pinnedVibeDir } : opts);

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
