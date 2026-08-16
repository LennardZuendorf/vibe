// engine/commands/hook.mjs — `vibe hook <name>`, the orchestration layer the
// four `.claude/hooks/*.sh` shims exec into once Node is present (js-core/7).
//
// This is NOT a port of a single bash oracle the way state/orders/doctrine/
// doctor are — it is the JS reimplementation of what each hook SCRIPT itself
// did around its ported command (drift-first nudge, warnings relay, guard
// verdict translation, the evidence-receipt gate). `infer` is still invoked
// via `spawnSync('bash', [...])`, exactly like doctor.mjs already shells out
// to validate-state.sh rather than reimplementing it; `decide` is now answered
// IN-PROCESS from policy.mjs, with the bash spawn kept as the fallback for an
// unusable policy — see inProcessVerdict() for the ruling and its reasons.
// STATE/FEATURE resolution is pure JS via the existing cursor.mjs primitives
// (jq-independent by construction — no no-jq branch is needed here the way
// detect-context.sh's own sed fallback needs one).
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
import { loadPolicy, decide } from '../policy.mjs';
import { runDoctrine } from './doctrine.mjs';
import { runOrders } from './orders.mjs';
import {
  renderChannelSafe,
  loadContent,
  renderChannel,
  channelTrigger,
  cursorChangedSince,
  recordInject,
  VIBE_DIR_RELPATH,
  LAST_INJECT_RELPATH,
} from '../content.mjs';

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

// git speaks POSIX separators in `status --porcelain` whatever the platform;
// path.join does not. One normalizer so the two halves compare like for like.
function toPosix(p) {
  return typeof p === 'string' ? p.split(path.sep).join('/') : '';
}

// ---------------------------------------------------------------------------
// Vibe's OWN runtime state — the files this harness writes on the very turns
// it is gating. The receipt-staleness scan (predicate 2) must skip them, or
// the harness stales its own receipt and blocks a `*.verify` state over a path
// the human never touched.
//
// Observed, not theorized (inject-triggers/6): a fresh install, one inject
// turn, and the gate reported `changed after it was written: .vibe/` — the
// collapsed untracked-directory entry git prints for the edge-detection marker
// `recordInject()` writes every turn. That wedges the only blocking tooth in
// the repo, with no exit but `set-state.sh idle`.
//
// Fixed belt AND braces: install.sh gitignores `.vibe/last-inject` alongside
// the cursor, the evidence dir and the warnings log (step 5), so on a current
// install git never reports it at all — and this exclusion catches the targets
// that gitignore cannot: an install made before the marker existed, or one
// whose `.gitignore` a project edited.
//
// Deliberately NARROW — the tooth keeps every bite it had. Only three things
// are excluded: the evidence directory (as before), the warnings relay log,
// and the marker — by its exact path, plus the `.vibe/` directory entry in the
// one spelling git uses when the WHOLE directory is untracked and it has
// nothing finer to report. A tracked file under `.vibe/` (a project's own
// authored blocks, `.vibe/blocks/**`) is reported individually by git, is not
// matched here, and still stales the receipt.
function isOwnRuntimeState(root, p) {
  const evidRel = '.agents/skills/vibe/evidence';
  if (p === evidRel || p.startsWith(`${evidRel}/`)) return true;
  if (p === toPosix(path.relative(root, warnLogPath(root)))) return true;
  if (p === toPosix(LAST_INJECT_RELPATH)) return true;
  return p === `${toPosix(VIBE_DIR_RELPATH)}/`;
}

// ---------------------------------------------------------------------------
// Warnings relay — a warn on stderr with exit 0 is invisible to the model, so
// guard/gate also append it here; the inject hook drains + truncates it once
// per turn. Every failure mode (missing dir, unwritable log) is a silent
// no-op, matching the bash originals.
//
// BOUNDED AND DEDUPLICATED (inject-triggers, R5). The drain writes into the
// PROMPT, which is append-only and re-read on every subsequent turn — so an
// unbounded drain is the one surface here that can grow without limit. A loop
// that trips the same guard on every tool call queued one line per occurrence
// and spent one transcript line per occurrence, for the rest of the session.
// Identical lines now collapse to one carrying a count, and a single turn
// emits at most RELAY_MAX_LINES of them plus a `+N more` trailer when the cap
// truncates. The log is truncated afterwards exactly as before: what the cap
// dropped is dropped for good rather than re-emitted next turn — the relay is
// a nudge, not a ledger, and a warn worth blocking on is a gate, not a line.
// ---------------------------------------------------------------------------

const RELAY_MAX_LINES = 10;

function appendWarnLog(root, msg) {
  if (!isDir(vibeLogDir(root))) return;
  try {
    fs.appendFileSync(warnLogPath(root), line(msg));
  } catch {
    // unwritable log — never fail the hook
  }
}

// Queued lines -> the lines to emit: identical text collapsed to one entry
// carrying `(xN)`, in FIRST-SEEN order (a Map preserves insertion order), so
// the reader sees the same sequence a plain drain would have shown, minus the
// repeats. Exported for the tests that pin the collapse and the cap.
export function collapseWarnLines(text) {
  if (typeof text !== 'string') return [];
  const counts = new Map();
  for (const raw of text.split('\n')) {
    if (!raw) continue;
    counts.set(raw, (counts.get(raw) ?? 0) + 1);
  }
  return [...counts].map(([msg, n]) => (n > 1 ? `${msg} (x${n})` : msg));
}

function drainWarnLog(root) {
  const p = warnLogPath(root);
  let text;
  try {
    text = fs.readFileSync(p, 'utf8');
  } catch {
    return '';
  }
  const collapsed = collapseWarnLines(text);
  let out = '';
  for (const msg of collapsed.slice(0, RELAY_MAX_LINES)) out += line(`vibe-warn: ${msg}`);
  if (collapsed.length > RELAY_MAX_LINES) {
    out += line(`vibe-warn: +${collapsed.length - RELAY_MAX_LINES} more`);
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
  const result = runDoctrine(skillsDir);
  const content = renderChannelSafe('session-start', { root, vibeDir, skillsDir });
  return { code: 0, stdout: (result.stdout || '') + content, stderr: '' };
}

// ---------------------------------------------------------------------------
// user-prompt-submit-inject — drift-first nudge (delegates to bash
// detect-context.sh infer) + the trigger-classed prompt payload + the warnings
// relay drain.
//
// The payload is classed by CADENCE, because this stream is append-only and
// every line of it is re-read on every turn for the rest of the session:
//
//   level  every turn        the cursor line — two lines, byte-stable
//   edge   cursor moved      the full orders for the state it moved into
//   event  something happened  text that is worth nothing on a quiet turn
//
// Byte-stability is the property that matters and the reason the split exists:
// two consecutive turns in the same state, with no drift and no warnings, emit
// IDENTICAL bytes, so the prompt cache still holds. The edge payload is what
// used to break that — it was re-injected verbatim on every turn.
// ---------------------------------------------------------------------------

// Mirrors bash `${drift#drift:*:}` — shortest-match removal of a
// "drift:<state>:" prefix. If the pattern does not match, bash leaves the
// value UNCHANGED (not emptied), so a non-matching string is returned as-is.
function stripDriftPrefix(s) {
  const m = /^drift:[^:]*:/.exec(s);
  return m ? s.slice(m[0].length) : s;
}

// The per-turn prompt surface is the `user-prompt` channel and anything under
// it (`user-prompt.level`, `.edge`, `.event`, plus whatever a project adds).
// Every other channel belongs to a different surface — `session-start` rides
// the session hook, `agents-md` is a document — and none of them may be
// injected here just because they exist.
const PROMPT_CHANNEL = 'user-prompt';

function isPromptChannel(name) {
  return name === PROMPT_CHANNEL || name.startsWith(`${PROMPT_CHANNEL}.`);
}

// Deterministic emission order, so the payload is a function of the cursor and
// nothing else: the flow's own three cadence channels lead, in cadence order,
// and every other prompt channel follows by name. The cursor line has to be
// the first thing after a drift nudge and the orders have to precede the
// standing rules a project composes — the orders are the turn's imperative,
// the rules are context for carrying it out.
const CHANNEL_LEAD = ['user-prompt.level', 'user-prompt.edge', 'user-prompt.event'];

// The slot the raw orders occupy when no edge channel delivers them.
const EDGE_CHANNEL = 'user-prompt.edge';

function compareChannels(a, b) {
  const ra = CHANNEL_LEAD.indexOf(a);
  const rb = CHANNEL_LEAD.indexOf(b);
  const ka = ra === -1 ? CHANNEL_LEAD.length : ra;
  const kb = rb === -1 ? CHANNEL_LEAD.length : rb;
  if (ka !== kb) return ka - kb;
  return a < b ? -1 : a > b ? 1 : 0;
}

// Every enabled prompt channel, in emission order, each tagged with the
// cadence that decides whether THIS turn carries it.
function promptChannels(content) {
  const out = [];
  for (const name of Object.keys(content.channels ?? {})) {
    if (!isPromptChannel(name)) continue;
    const channel = content.channels[name];
    if (!channel || typeof channel !== 'object' || channel.enabled === false) continue;
    out.push({ name, trigger: channelTrigger(channel), blocks: channel.blocks ?? [] });
  }
  return out.sort((a, b) => compareChannels(a.name, b.name));
}

// The cursor key the edge detector compares turns against: state plus feature,
// one line, built from the cursor reader this module already uses for the stop
// gate. Never re-derived from a file — cursor.mjs is the only cursor reader.
function injectKey(vibeDir) {
  const { state, feature } = cursorStateFeature(vibeDir);
  return `${state} ${feature ?? ''}`;
}

// Composes the whole prompt payload for this turn. Returns `undefined` if
// ANYTHING goes wrong — the caller then falls back to the pre-content-layer
// payload (the raw orders) and records nothing, so a crash in here can never
// silently swallow an edge payload: the next turn still sees the cursor as
// moved and re-emits it.
//
// `orders` is the raw orders text for this turn, passed in by the caller so it
// is resolved exactly once: it is both the fallback payload and the EVIDENCE
// the take-over below is decided on.
function composePromptPayload(ctx, hadEvent, orders) {
  try {
    const content = loadContent(ctx.root, ctx.vibeDir);
    const channels = promptChannels(content);

    // Does the content layer OWN the orders this turn?
    //
    // Decided on EVIDENCE, never on a declaration (fix round 1, Critical). The
    // first cut asked whether an edge-classed channel DECLARED any blocks,
    // which is a promise about config, not a fact about output — and it had two
    // silent-loss paths, both ending in the turn's imperative disappearing from
    // every turn forever, at exit 0, with nothing to notice it:
    //
    //   (a) the composed block is missing (a partial install, or the file
    //       deleted). The declared list is still non-empty; the channel renders
    //       nothing at all.
    //   (b) a project repoints the channel at its own block that carries no
    //       `{{orders}}`. The channel renders fine and simply does not contain
    //       them.
    //
    // So the channel is composed FIRST and the take-over is granted only if
    // that composition actually produced the orders. This is the repo's own
    // "a check that examines nothing must fail loudly" rule applied to a
    // handover: losing a line to duplication is recoverable, losing the turn's
    // imperative is not. `render --check` carries the same rule as a lint, so
    // an author hears about it at config time rather than at inject time.
    //
    // Composed on EVERY turn, not only when the cursor moved: the evidence has
    // to exist before the decision, and the decision is due every turn. Only
    // the EMISSION is gated on the cadence, below.
    const edgeText = channels
      .filter((c) => c.trigger === 'edge')
      .map((c) => renderChannel(c.name, ctx, content).text || '')
      .join('');
    const ownsOrders = Boolean(orders) && edgeText.includes(orders);

    const key = injectKey(ctx.vibeDir);
    // Fail-open by contract: an unreadable or absent marker reads as "moved",
    // which costs one extra edge payload and never loses one.
    let moved = true;
    try {
      moved = cursorChangedSince(ctx.root, key);
    } catch {
      moved = true;
    }

    const due = { level: true, edge: moved, event: hadEvent };

    // Split at the edge SLOT so the caller can splice the raw orders into it
    // when no edge channel delivered them (fix carried over from unit 4's
    // review): the documented order is level -> edge/orders -> event -> the
    // standing rules a project composes, and the fallback used to PREPEND the
    // orders ahead of the level line instead. The orders are the turn's
    // imperative and the rules are context for carrying it out, so they must
    // still precede everything that follows the level channels.
    //
    // Split by SORT POSITION, not by trigger: the edge slot exists in the
    // emission order even on a turn carrying no edge payload, and a project's
    // standing-rules channel is level-classed too but sorts after it — keying
    // on the trigger would let the orders land behind the rules.
    let levelText = '';
    let restText = '';
    for (const channel of channels) {
      if (!due[channel.trigger]) continue;
      const rendered = renderChannel(channel.name, ctx, content).text || '';
      if (compareChannels(channel.name, EDGE_CHANNEL) < 0) levelText += rendered;
      else restText += rendered;
    }

    // Recorded on ANY turn the payload was composed, not only when an edge
    // channel produced text (fix carried over from unit 4's review): gating the
    // record on `edgeText !== ''` meant an install whose edge channel renders
    // nothing never wrote the marker, so cursorChangedSince() read "moved"
    // forever. Harmless while nothing consumed it beyond the edge cadence, but
    // it made the marker a lie the moment anything else did.
    return { levelText, restText, ownsOrders, key, record: true };
  } catch {
    return undefined;
  }
}

export function runInjectHook(root, vibeDir, skillsDir, opts = {}) {
  let stdout = '';

  let drift = '';
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
    drift = res && !res.error && typeof res.stdout === 'string' ? res.stdout.trim() : '';
    if (drift) stdout += line(`vibe-drift: ${stripDriftPrefix(drift)}`);
  }

  // Drained HERE, emitted LAST. The event class needs to know whether this
  // turn carried an event before the payload is composed, and the relay is the
  // other half of that answer; draining early changes no output order, only
  // when the read happens.
  const warns = drainWarnLog(root);

  const ctx = { root, vibeDir, skillsDir };
  // Resolved once and used twice: as the fallback payload below, and as the
  // evidence composePromptPayload weighs the edge channel's output against.
  // `{{orders}}` interpolates the TRIMMED text (content.mjs), so the evidence
  // is trimmed and the emitted line keeps its newline.
  const orders = runOrders(vibeDir, skillsDir, []).stdout || '';
  const payload = composePromptPayload(ctx, Boolean(drift) || Boolean(warns), orders.trim());

  if (payload) {
    // level -> orders (the edge slot) -> everything after it. The raw orders
    // ride only when no edge channel actually delivered them, exactly as
    // before this feature; what changed is that they no longer jump ahead of
    // the level line.
    stdout += payload.levelText;
    if (!payload.ownsOrders) stdout += orders;
    stdout += payload.restText;
    // Recorded ONCE per turn, after composing succeeded — never before, so a
    // failure mid-compose leaves the cursor looking unchanged and the next
    // turn re-emits the edge payload.
    if (payload.record) recordInject(root, payload.key);
  } else {
    // No content layer at all: the state's orders are the turn's imperative
    // and ride every turn, ahead of the legacy channel's standing rules.
    stdout += orders;
    // Compose failed outright. The legacy channel is still worth trying
    // through its own never-throws wrapper, so a broken new channel cannot
    // cost a project the standing rules it had before.
    stdout += renderChannelSafe(PROMPT_CHANNEL, ctx);
  }

  stdout += warns;

  return { code: 0, stdout, stderr: '' };
}

// ---------------------------------------------------------------------------
// pre-tool-use-guard — Bash commands get a warn-only text sniff; Edit/Write/
// NotebookEdit get their target path decided against the write-invariant
// policy, whose verdict (allow|warn:<reason>|block:<reason>) is translated to
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

// The write-invariant decision, answered IN-PROCESS (inject-triggers/5,
// carrying over unit 2's controller ruling). Unit 2 made detect-context.sh
// delegate `decide` to `vibe policy decide`, so a hook that reached the verdict
// by spawning that script was spawning node again: node -> bash -> node on
// every guarded Edit, on a PreToolUse path the user waits behind. policy.mjs is
// pure, jq-independent and never throws, so the same answer is one call away.
//
// The bash spawn stays as the FALLBACK rather than being deleted, because the
// two branches answer different failure modes. `undefined` here means "the
// policy DATA is unusable" — absent, unreadable, a version this engine does not
// understand, or a rule set that loaded empty — which is exactly what
// `vibe policy decide` refuses with exit 2 so detect-context.sh can fall
// through to its own hardcoded copy of the same policy. Answering `allow` from
// an empty rule set would make every hard block vanish silently; falling back
// keeps the block. A reasonless warn/block is treated the same way, mirroring
// detect-context.sh's is_verdict() shape check.
//
// WHICH install answers: vibeLogDir(root) — the same directory whose
// scripts/detect-context.sh this hook would otherwise spawn, and the same
// directory that script self-locates as its own SKILL_DIR and passes to the
// engine as `--vibe-dir`. Both branches therefore read one policy.json by
// construction, whatever the cwd or the ambient environment. The state comes
// from that directory's cursor for the same reason: two resolvers would be two
// chances to disagree about where we are.
function inProcessVerdict(root, relPath) {
  try {
    const dir = vibeLogDir(root);
    const { rules, errors } = loadPolicy(dir);
    if (errors.length > 0 || rules.length === 0) return undefined;
    const { state } = cursorStateFeature(dir);
    const { verdict, reason } = decide({ rules }, relPath, state);
    if (verdict === 'allow') return 'allow';
    if (!reason) return undefined;
    return `${verdict}:${reason}`;
  } catch {
    return undefined;
  }
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
  // Mirrors detect-context.sh's own `path="${path#./}"`, applied before either
  // branch sees the path so the two cannot disagree about `./x` vs `x`. Spelled
  // as a regex, not a string literal: a leading-dot-slash STRING is
  // specifier-shaped, and primitives.test.mjs's module-reach rule (rightly)
  // refuses to let a shipped module hold one it cannot resolve.
  pathIn = pathIn.replace(/^\.\//, '');

  let verdict = inProcessVerdict(root, pathIn);
  if (verdict === undefined) {
    const spawnDecide =
      opts.spawnDecide || ((args) => spawnSync('bash', args, { encoding: 'utf8' }));
    let res;
    try {
      res = spawnDecide([detectPath, 'decide', pathIn]);
    } catch {
      res = undefined;
    }
    verdict =
      res && !res.error && typeof res.stdout === 'string' && res.stdout.trim() ? res.stdout.trim() : 'allow';
  }

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
// stop-gate — the re-entry guard and TWO predicates (TDD warn / the one
// promoted evidence-receipt block). STATE and FEATURE come from readCursor()
// directly (jq-independent by construction), not from shelling out to
// detect-context.sh snapshot.
//
// PREDICATE 3 IS DELETED (inject-triggers, R6). It warned `still in <state> —
// ... (next: ...)` on every Stop in a non-idle state with legal next states,
// which is a fact the per-turn `user-prompt.level` channel now states on EVERY
// turn. Worse than merely redundant: each firing queued a line into the
// warnings relay, which the next inject drained into the prompt — so a
// duplicate of a line already present became a permanent extra line in the
// transcript, once per Stop. Nothing else moved with it: predicate 2 is the one
// blocking tooth in the repo and is untouched below.
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

  for (const raw of changed.split('\n')) {
    if (!raw) continue;
    const xy = raw.slice(0, 2);
    let p = raw.slice(3);
    if (xy.includes('D')) continue; // skip deletions
    if (xy[0] === 'R' || xy[0] === 'C') {
      const idx = p.lastIndexOf(' -> ');
      if (idx !== -1) p = p.slice(idx + 4); // rename/copy -> new path
    }
    if (isOwnRuntimeState(root, p)) continue; // this harness's own writes

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
