// engine/commands/hook.mjs — `vibe hook <name>`, the orchestration layer the
// four `.claude/hooks/*.sh` shims exec into once Node is present (js-core/7).
//
// This is NOT a port of a single bash oracle the way state/orders/doctrine/
// doctor are — it is the JS reimplementation of what each hook SCRIPT itself
// did around its ported command (drift-first nudge, warnings relay, guard
// verdict translation, the evidence-receipt gate). Per the feature plan,
// detect-context.sh's DECISION POLICY stays bash — `decide` and `infer` are
// still invoked via `spawnSync('bash', [...])`, exactly like doctor.mjs
// already shells out to validate-state.sh rather than reimplementing it.
// STATE/FEATURE/NEXT resolution, by contrast, is pure JS via the existing
// cursor.mjs/machine.mjs primitives (jq-independent by construction — no
// no-jq branch is needed here the way detect-context.sh's own sed fallback
// needs one).
//
// Every exported run*Hook() is pure over its inputs (root/vibeDir/skillsDir/
// stdin text, plus injectable spawn* functions for hermetic tests) and NEVER
// throws, returning {code, stdout, stderr} like every other command — the
// default export below is the only place that touches real stdin/stdout.
//
// Root-relative, not vibeDir-relative: the detect-context.sh path, the
// warnings relay log, and the evidence receipts all mirror the ORIGINAL
// hooks' own `$ROOT/.agents/skills/vibe/...` literals exactly (never
// vibeDir-derived) — these are the real .sh scripts' own paths, not a vibe
// primitive's resolution, so there is nothing to single-source them against.
// orders/doctrine calls use vibeDir/skillsDir, matching those commands' own
// contracts.

import fs from 'node:fs';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import { resolveRoot, resolveVibeDir, resolveSkillsDir } from '../root.mjs';
import { readCursor } from '../cursor.mjs';
import { loadMachine, stateOf } from '../machine.mjs';
import { runDoctrine } from './doctrine.mjs';
import { runOrders } from './orders.mjs';
import { renderChannelSafe } from '../content.mjs';

function line(s) {
  return `${s}\n`;
}

// ---------------------------------------------------------------------------
// Shared root-relative paths (mirror each hook's own literal exactly).
// ---------------------------------------------------------------------------

function vibeLogDir(root) {
  return path.join(root, '.agents', 'skills', 'vibe');
}

function detectScriptPath(root) {
  return path.join(vibeLogDir(root), 'scripts', 'detect-context.sh');
}

function warnLogPath(root) {
  return path.join(vibeLogDir(root), 'warnings.log');
}

function isDir(p) {
  try {
    return fs.statSync(p).isDirectory();
  } catch {
    return false;
  }
}

// ---------------------------------------------------------------------------
// Warnings relay — a warn on stderr with exit 0 is invisible to the model, so
// guard/gate also append it here; the inject hook drains + truncates it once
// per turn. Every failure mode (missing dir, unwritable log) is a silent
// no-op, matching the bash originals.
// ---------------------------------------------------------------------------

function appendWarnLog(root, msg) {
  if (!isDir(vibeLogDir(root))) return;
  try {
    fs.appendFileSync(warnLogPath(root), line(msg));
  } catch {
    // unwritable log — never fail the hook
  }
}

function drainWarnLog(root) {
  const p = warnLogPath(root);
  let text;
  try {
    text = fs.readFileSync(p, 'utf8');
  } catch {
    return '';
  }
  let out = '';
  for (const raw of text.split('\n')) {
    if (!raw) continue;
    out += line(`vibe-warn: ${raw}`);
  }
  try {
    fs.writeFileSync(p, '');
  } catch {
    try {
      fs.unlinkSync(p);
    } catch {
      // unwritable/unremovable log — never fail the hook
    }
  }
  return out;
}

// ---------------------------------------------------------------------------
// session-start-doctrine — a thin pass-through to the already-ported
// doctrine command. Always exit 0, matching both doctrine.mjs's own
// contract and the bash shim's `bash "$DOCTRINE" 2>/dev/null || true; exit 0`.
// ---------------------------------------------------------------------------

// `root` is optional and trails the ported arguments deliberately: the
// doctrine port itself never needed a project root, and the content layer must
// not change that call's shape. With no root (or no content tree) the
// session-start channel contributes nothing and the output is byte-identical
// to the pre-content-layer hook.
export function runDoctrineHook(vibeDir, skillsDir, root) {
  const result = runDoctrine(vibeDir, skillsDir);
  const content = renderChannelSafe('session-start', { root, vibeDir, skillsDir });
  return { code: 0, stdout: (result.stdout || '') + content, stderr: '' };
}

// ---------------------------------------------------------------------------
// user-prompt-submit-inject — drift-first nudge (delegates to bash
// detect-context.sh infer) + orders.mjs's output + the warnings relay drain.
// ---------------------------------------------------------------------------

// Mirrors bash `${drift#drift:*:}` — shortest-match removal of a
// "drift:<state>:" prefix. If the pattern does not match, bash leaves the
// value UNCHANGED (not emptied), so a non-matching string is returned as-is.
function stripDriftPrefix(s) {
  const m = /^drift:[^:]*:/.exec(s);
  return m ? s.slice(m[0].length) : s;
}

export function runInjectHook(root, vibeDir, skillsDir, opts = {}) {
  let stdout = '';

  const detectPath = detectScriptPath(root);
  if (fs.existsSync(detectPath)) {
    const spawnInfer =
      opts.spawnInfer || (() => spawnSync('bash', [detectPath, 'infer'], { encoding: 'utf8' }));
    let res;
    try {
      res = spawnInfer();
    } catch {
      res = undefined;
    }
    const drift = res && !res.error && typeof res.stdout === 'string' ? res.stdout.trim() : '';
    if (drift) stdout += line(`vibe-drift: ${stripDriftPrefix(drift)}`);
  }

  const ordersResult = runOrders(vibeDir, skillsDir, []);
  stdout += ordersResult.stdout || '';

  // Standing rules from the content layer ride AFTER the state's orders (which
  // are the turn's imperative) and BEFORE the warnings drain (which is
  // event-only). Byte-stable by construction, so the per-turn prompt cache
  // still holds; a missing/broken content tree adds nothing at all.
  stdout += renderChannelSafe('user-prompt', { root, vibeDir, skillsDir });

  stdout += drainWarnLog(root);

  return { code: 0, stdout, stderr: '' };
}

// ---------------------------------------------------------------------------
// pre-tool-use-guard — Bash commands get a warn-only text sniff; Edit/Write/
// NotebookEdit get their target path routed through bash detect-context.sh
// decide, whose verdict (allow|warn:<reason>|block:<reason>) is translated to
// this hook's exit-code convention (block -> stderr + exit 2, warn -> stderr
// + relay + exit 0, allow -> exit 0).
// ---------------------------------------------------------------------------

const BASH_WRITE_OP_RE = /(>>?|(^|\s)(tee|truncate|mv|cp|rm)(\s|$)|(^|\s)sed(\s|$).*-i)/;
const LESSONS_RE = /(^|[^A-Za-z0-9_])\.spec\/lessons\.md/;
const ROOT_SPEC_RE = /(^|[^A-Za-z0-9_])\.spec\/(product|tech|design|plan)\.md/;
const STATE_JSON_RE = /(\.agents\/skills\/vibe\/state\.json|(^|[^A-Za-z0-9_])flow\/state\.json)/;
const SET_STATE_RE = /set-state\.sh/;

// Returns the guarded-path-class label to warn about, or undefined for "no
// hit" — mirrors sniff_bash()'s three case arms, checked in the same order.
function sniffBash(cmd) {
  if (!BASH_WRITE_OP_RE.test(cmd)) return undefined;
  if (LESSONS_RE.test(cmd)) return '.spec/lessons.md';
  if (ROOT_SPEC_RE.test(cmd)) return 'a root .spec/{product,tech,design,plan}.md doc';
  if (STATE_JSON_RE.test(cmd)) {
    if (SET_STATE_RE.test(cmd)) return undefined; // sanctioned writer: never warn
    return '.agents/skills/vibe/state.json (use set-state.sh)';
  }
  return undefined;
}

function extractTargetPath(parsed) {
  const ti = parsed && typeof parsed === 'object' ? parsed.tool_input : undefined;
  if (!ti || typeof ti !== 'object') return undefined;
  if (typeof ti.file_path === 'string' && ti.file_path) return ti.file_path;
  if (typeof ti.notebook_path === 'string' && ti.notebook_path) return ti.notebook_path;
  return undefined;
}

function stripLeadingRoot(p, root) {
  if (typeof p !== 'string' || typeof root !== 'string') return p;
  if (p === root) return p;
  const prefix = root.endsWith(path.sep) ? root : `${root}${path.sep}`;
  return p.startsWith(prefix) ? p.slice(prefix.length) : p;
}

export function runGuardHook(root, stdinText, opts = {}) {
  const detectPath = detectScriptPath(root);
  if (!fs.existsSync(detectPath)) return { code: 0, stdout: '', stderr: '' };
  if (!stdinText) return { code: 0, stdout: '', stderr: '' };

  let parsed;
  try {
    parsed = JSON.parse(stdinText);
  } catch {
    parsed = {};
  }

  const toolName = parsed && typeof parsed.tool_name === 'string' ? parsed.tool_name : '';

  if (toolName === 'Bash') {
    const cmd = parsed && parsed.tool_input && typeof parsed.tool_input.command === 'string'
      ? parsed.tool_input.command
      : '';
    if (!cmd) return { code: 0, stdout: '', stderr: '' };
    const hit = sniffBash(cmd);
    if (!hit) return { code: 0, stdout: '', stderr: '' };
    appendWarnLog(root, `guard: bash command appears to write ${hit} (bypasses the file-tool guard) (warn-only)`);
    return {
      code: 0,
      stdout: '',
      stderr: line(
        `vibe-guard: warn — a bash command looks like it writes ${hit} (a guarded path); prefer the flow's write surface over a raw shell redirect. (warn-only)`,
      ),
    };
  }

  let pathIn = extractTargetPath(parsed);
  if (!pathIn) return { code: 0, stdout: '', stderr: '' };
  pathIn = stripLeadingRoot(pathIn, root);

  const spawnDecide =
    opts.spawnDecide || ((args) => spawnSync('bash', args, { encoding: 'utf8' }));
  let res;
  try {
    res = spawnDecide([detectPath, 'decide', pathIn]);
  } catch {
    res = undefined;
  }
  const verdict =
    res && !res.error && typeof res.stdout === 'string' && res.stdout.trim() ? res.stdout.trim() : 'allow';

  if (verdict.startsWith('block:')) {
    const reason = verdict.slice('block:'.length);
    return {
      code: 2,
      stdout: '',
      stderr:
        line(`vibe-guard: BLOCKED — ${reason}`) +
        line(`vibe-guard: transition with set-state.sh, or edit within the current state's write rules.`),
    };
  }
  if (verdict.startsWith('warn:')) {
    const reason = verdict.slice('warn:'.length);
    appendWarnLog(root, `guard: ${reason}`);
    return { code: 0, stdout: '', stderr: line(`vibe-guard: warn — ${reason}`) };
  }
  return { code: 0, stdout: '', stderr: '' };
}

// ---------------------------------------------------------------------------
// stop-gate — the re-entry guard, three predicates (TDD warn / the one
// promoted evidence-receipt block / stuck-phase nudge). STATE, FEATURE, NEXT
// come from readCursor()/loadMachine()/stateOf() directly (jq-independent by
// construction), not from shelling out to detect-context.sh snapshot.
// ---------------------------------------------------------------------------

// FAIL-SAFE, and a deliberate choice between two disagreeing bash legs
// (js-core/8 final review, M4). The oracle's jq leg is
// `jq -r '.stop_hook_active // false'` compared against the STRING "true", so a
// JSON string "true" short-circuits there exactly like a boolean; its sed leg
// matches only a bare `true`/`false` token and treats the string as false. The
// port required `=== true` and so matched the sed leg — into the blocking
// tooth.
//
// Claude Code sends a JSON boolean today, so nothing observed this. But the
// asymmetry of the two failure modes is total: reading a truthy value as
// "already re-entered" loses ONE turn of gate enforcement, while reading it as
// "first entry" makes the Stop hook block its own re-invocation — a block loop,
// the single failure mode this guard exists to prevent, and a wedged session
// the user cannot exit. So the engine matches the jq leg: any value jq -r would
// render as exactly `true`.
function readStopHookActive(stdinText) {
  if (!stdinText) return false;
  try {
    const parsed = JSON.parse(stdinText);
    if (!parsed) return false;
    const raw = parsed.stop_hook_active;
    return raw === true || raw === 'true';
  } catch {
    return false;
  }
}

function cursorStateFeature(vibeDir) {
  try {
    const cursor = readCursor(vibeDir);
    return { state: cursor.state, feature: cursor.feature ?? null };
  } catch {
    return { state: 'idle', feature: null };
  }
}

// DELIBERATE DIVERGENCE (js-core/7 review round 1, Finding 3): the bash
// oracle resolved NEXT via detect-context.sh's jq-gated `snapshot` — without
// jq it left NEXT="" and predicate 3 (the stuck-phase nudge, below) silently
// never fired. This is pure JS (loadMachine/stateOf), jq-independent by
// construction, so it now fires predicate 3 even when jq is absent. Warn-only,
// cannot block, arguably more correct — kept as-is rather than reproducing
// the oracle's jq-gate, and pinned by a dedicated no-jq test in
// flow/tests/run.sh so the divergence stays visible, not silent.
function nextStates(vibeDir, stateKey) {
  try {
    const machine = loadMachine(vibeDir);
    const entry = stateOf(machine, stateKey);
    return Array.isArray(entry && entry.next) ? entry.next : [];
  } catch {
    return [];
  }
}

function gitPorcelain(root, opts) {
  const spawnGit = opts.spawnGit || ((args) => spawnSync('git', args, { encoding: 'utf8' }));
  try {
    const check = spawnGit(['-C', root, 'rev-parse', '--is-inside-work-tree']);
    if (!check || check.error || check.status !== 0) return '';
    const res = spawnGit(['-C', root, 'status', '--porcelain']);
    if (!res || res.error || res.status !== 0) return '';
    return typeof res.stdout === 'string' ? res.stdout.replace(/\n+$/, '') : '';
  } catch {
    return '';
  }
}

// Predicate 1 — impl touched src/** but no tests/**. Warn-only.
function predicateTdd(state, changed) {
  if (state !== 'feature.impl' && state !== 'quick.fix') return undefined;
  if (!changed) return undefined;
  const changedLines = changed.split('\n').filter(Boolean);
  const touchesSrc = changedLines.some((l) => /(^|\/)src\//.test(l));
  const touchesTests = changedLines.some((l) => /(^|\/)tests?\//.test(l));
  if (touchesSrc && !touchesTests) {
    return `in ${state}, src changed with no test changes — TDD expects a reproducing/covering test. (warn-only)`;
  }
  return undefined;
}

// Predicate 2 — the one promoted blocking tooth: a *.verify state needs a
// fresh evidence receipt. Returns {stderr} to block (exit 2), or undefined to
// continue. `warn` is a callback so the "no feature named" degrade can queue
// its own warn-only line without this function owning stderr accumulation.
function evidenceReceiptCheck(root, state, feature, changed, warn) {
  if (state !== 'feature.verify' && state !== 'quick.verify') return undefined;

  let receipt;
  if (state === 'feature.verify') {
    if (!feature) {
      warn(
        'in feature.verify but the cursor names no feature — cannot resolve the evidence receipt; skipping the gate. (warn-only)',
      );
      return undefined;
    }
    receipt = path.join(vibeLogDir(root), 'evidence', `feature-${feature}.md`);
  } else {
    receipt = path.join(vibeLogDir(root), 'evidence', 'quick.md');
  }

  let receiptStat;
  try {
    receiptStat = fs.statSync(receipt);
  } catch {
    receiptStat = undefined;
  }
  if (!receiptStat || !receiptStat.isFile()) {
    return {
      stderr:
        line(`vibe-gate: BLOCKED — ${state} needs an evidence receipt before 'done'.`) +
        line(`  expected: ${receipt}`) +
        line(`  it must record the commands you ran and their observed output (per unit ID for a feature).`) +
        line(`  not verifying? abort with: bash .agents/skills/vibe/scripts/set-state.sh idle`),
    };
  }

  if (!changed) return undefined;

  const evidRel = '.agents/skills/vibe/evidence';
  for (const raw of changed.split('\n')) {
    if (!raw) continue;
    const xy = raw.slice(0, 2);
    let p = raw.slice(3);
    if (xy.includes('D')) continue; // skip deletions
    if (xy[0] === 'R' || xy[0] === 'C') {
      const idx = p.lastIndexOf(' -> ');
      if (idx !== -1) p = p.slice(idx + 4); // rename/copy -> new path
    }
    if (p === evidRel || p.startsWith(`${evidRel}/`)) continue; // exclude the evidence dir

    const f = path.join(root, p);
    let st;
    try {
      st = fs.statSync(f);
    } catch {
      continue; // matches bash's `-e "$f"` guard
    }
    if (st.mtimeMs > receiptStat.mtimeMs) {
      return {
        stderr:
          line('vibe-gate: BLOCKED — the evidence receipt is stale.') +
          line(`  receipt: ${receipt}`) +
          line(`  changed after it was written: ${p}`) +
          line('  re-run verification and rewrite the receipt with fresh commands + output.') +
          line('  not verifying? abort with: bash .agents/skills/vibe/scripts/set-state.sh idle'),
      };
    }
  }
  return undefined;
}

export function runGateHook(root, vibeDir, stdinText, opts = {}) {
  if (readStopHookActive(stdinText)) return { code: 0, stdout: '', stderr: '' };

  const detectPath = detectScriptPath(root);
  if (!fs.existsSync(detectPath)) return { code: 0, stdout: '', stderr: '' };

  const { state, feature } = cursorStateFeature(vibeDir);
  const next = nextStates(vibeDir, state);
  const changed = gitPorcelain(root, opts);

  let stderr = '';
  const warn = (msg) => {
    stderr += line(`vibe-gate: ${msg}`);
    appendWarnLog(root, `gate: ${msg}`);
  };

  const p1 = predicateTdd(state, changed);
  if (p1) warn(p1);

  const blocked = evidenceReceiptCheck(root, state, feature, changed, warn);
  if (blocked) {
    return { code: 2, stdout: '', stderr: stderr + blocked.stderr };
  }

  if (state !== 'idle' && next.length > 0) {
    warn(
      `still in ${state} — when this phase's exit is met, advance with set-state.sh (next: ${next.join(', ')}). (warn-only)`,
    );
  }

  return { code: 0, stdout: '', stderr };
}

// ---------------------------------------------------------------------------
// CLI entry — `vibe hook <name>`. Reads real stdin once (mirrors each bash
// shim's own single `cat`/stdin read), dispatches by name, writes stdout/
// stderr, returns the exit code. An unrecognized name degrades to a silent
// no-op (exit 0) rather than failing — never breaks the session.
// ---------------------------------------------------------------------------

function readStdinSync() {
  try {
    return fs.readFileSync(0, 'utf8');
  } catch {
    return '';
  }
}

export default async function run(argv, opts = {}) {
  const [name] = argv;
  const root = resolveRoot(opts);
  const vibeDir = resolveVibeDir(opts);
  const skillsDir = resolveSkillsDir(opts);

  let result;
  switch (name) {
    case 'session-start-doctrine':
      readStdinSync();
      result = runDoctrineHook(vibeDir, skillsDir, root);
      break;
    case 'user-prompt-submit-inject':
      readStdinSync();
      result = runInjectHook(root, vibeDir, skillsDir, opts);
      break;
    case 'pre-tool-use-guard':
      result = runGuardHook(root, readStdinSync(), opts);
      break;
    case 'stop-gate':
      result = runGateHook(root, vibeDir, readStdinSync(), opts);
      break;
    default:
      readStdinSync();
      result = { code: 0, stdout: '', stderr: '' };
      break;
  }

  if (result.stdout) process.stdout.write(result.stdout);
  if (result.stderr) process.stderr.write(result.stderr);
  return result.code;
}
