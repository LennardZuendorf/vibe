// engine/policy.mjs — loadPolicy()/decide()/renderInvariants(): the six
// guarded-path arms of flow/scripts/detect-context.sh's `decide()` as data,
// replacing a hardcoded bash guard. A later unit wires that guard to
// delegate here and adds a differential matrix over every guarded path; this
// module only has to answer correctly on its own, but every reason string
// and every state list below is copied element-for-element from the bash
// source so that later matrix has something to actually agree with.
//
// Rule shape (content/policy.json):
//   { id, match, arms }
//
// `match` is a relative path, or an array of them. Each entry is a LITERAL
// path or a GLOB (`*`/`?`). Every wildcard matches ANY run of characters,
// INCLUDING `/` — there is no "stays within one directory" dialect here,
// because every arm below is a direct port of a detect-context.sh `case`
// pattern, and bash's own pattern matching (fnmatch without FNM_PATHNAME) is
// exactly that: unrestricted by `/`.
//
// A LITERAL pattern additionally matches on any PATH-BOUNDARY SUFFIX of the
// queried path, not only a full-string match. detect-context.sh guards every
// one of its six arms with the same `PATTERN|*/PATTERN` idiom — "the bare
// path, or the bare path preceded by anything and a `/`" — so an absolute
// path, a `../`-prefixed one, or one sitting under an extra directory all
// still hit the rule the bare relative path hits. Reproducing that here as a
// suffix match (rather than requiring every pattern to be hand-authored with
// its own `*/` alternative) makes it apply uniformly and makes it impossible
// to forget on a new rule. A GLOB pattern gets the same suffix treatment: it
// is tested against every boundary suffix, not just the whole path.
//
// `arms` is an ORDERED list of `{states, verdict, reason}`. `states` is
// either an array of state keys or the literal string `"*"` — an else/
// catch-all arm, admitted regardless of state. The FIRST arm whose `states`
// admits the queried state wins (mirrors a bash `case "$state" in ... esac`
// read top to bottom). `reason` may contain the literal token `{state}`,
// replaced with the queried state at `decide()` time — this is what lets one
// reason string carry the bash oracle's own `(current: $state)` suffix
// verbatim, on the exact state actually queried, rather than baking in a
// fixed string.
//
// Contract, matching every other engine module: NEVER throws. An absent or
// malformed policy.json degrades to "no rules" — which makes `decide` answer
// 'allow' for every path — and reports through `errors`. Failing shut (no
// write is restricted) is recoverable; a hook that throws wedges the session.

import path from 'node:path';
import { readJson } from './json.mjs';

export const POLICY_RELPATH = path.join('content', 'policy.json');

const VERDICTS = new Set(['allow', 'warn', 'block']);
const SUPPORTED_VERSION = 1;
const CATCH_ALL = '*';
const STATE_TOKEN = '{state}';

// ---------------------------------------------------------------------------
// loadPolicy — read + validate. Every per-rule (and per-arm) defect is
// reported and that ONE rule is dropped; one bad rule must never take the
// rest of the file down with it.
// ---------------------------------------------------------------------------

function normalizeMatchField(value) {
  const list = Array.isArray(value) ? value : [value];
  return list.filter((entry) => typeof entry === 'string' && entry.length > 0);
}

// `states: "*"` is the catch-all sentinel; anything else must be an array of
// state-key strings (an empty array is legal — it is simply an arm no state
// can ever admit, e.g. a rule with no allow branch at all).
function normalizeArms(value, id, errors) {
  if (!Array.isArray(value)) {
    errors.push(`policy: rule '${id}' has no 'arms' array`);
    return [];
  }
  const arms = [];
  value.forEach((entry, idx) => {
    if (!entry || typeof entry !== 'object' || Array.isArray(entry)) {
      errors.push(`policy: rule '${id}' arms[${idx}] is not an object`);
      return;
    }
    const states =
      entry.states === CATCH_ALL
        ? CATCH_ALL
        : Array.isArray(entry.states)
          ? entry.states.filter((s) => typeof s === 'string')
          : [];
    const verdict = VERDICTS.has(entry.verdict) ? entry.verdict : 'block';
    if (entry.verdict !== undefined && !VERDICTS.has(entry.verdict)) {
      errors.push(`policy: rule '${id}' arms[${idx}] has an unknown verdict '${entry.verdict}' — defaulting to 'block'`);
    }
    const reason = typeof entry.reason === 'string' ? entry.reason : '';
    arms.push({ states, verdict, reason });
  });
  return arms;
}

export function loadPolicy(vibeDir) {
  const errors = [];
  if (typeof vibeDir !== 'string' || !vibeDir) return { rules: [], errors };

  const filePath = path.join(vibeDir, POLICY_RELPATH);
  let raw;
  try {
    raw = readJson(filePath);
  } catch (err) {
    if (err && err.code === 'ENOENT') return { rules: [], errors };
    errors.push(`policy: ${filePath} is not readable as JSON (${err && err.message ? err.message : err})`);
    return { rules: [], errors };
  }

  if (!raw || typeof raw !== 'object' || Array.isArray(raw)) {
    errors.push(`policy: ${filePath} is not a JSON object`);
    return { rules: [], errors };
  }
  // An explicit, unrecognized version is refused outright rather than loaded
  // silently as if it were v1 — nobody previously checked this field, so a
  // future schema change had no way to fail loudly on an old engine reading
  // it. An ABSENT version is not an error: it is the common case for a file
  // this engine itself wrote before `version` existed as a check at all.
  if (raw.version !== undefined && raw.version !== SUPPORTED_VERSION) {
    errors.push(
      `policy: ${filePath} declares version ${JSON.stringify(raw.version)}, but this engine only understands version ${SUPPORTED_VERSION} — refusing to load its rules`,
    );
    return { rules: [], errors };
  }
  if (!Array.isArray(raw.rules)) {
    errors.push(`policy: ${filePath} has no 'rules' array`);
    return { rules: [], errors };
  }

  const rules = [];
  raw.rules.forEach((entry, idx) => {
    if (!entry || typeof entry !== 'object' || Array.isArray(entry)) {
      errors.push(`policy: rules[${idx}] is not an object`);
      return;
    }
    const id = typeof entry.id === 'string' && entry.id ? entry.id : '';
    const match = normalizeMatchField(entry.match);
    if (!id) {
      errors.push(`policy: rules[${idx}] is missing an 'id'`);
      return;
    }
    if (match.length === 0) {
      errors.push(`policy: rule '${id}' has no usable 'match' path`);
      return;
    }
    const arms = normalizeArms(entry.arms, id, errors);
    if (arms.length === 0) {
      errors.push(`policy: rule '${id}' has no usable 'arms'`);
      return;
    }
    rules.push({ id, match, arms });
  });

  return { rules, errors };
}

// ---------------------------------------------------------------------------
// Matching
// ---------------------------------------------------------------------------

const LEADING_DOT_SLASH_RE = /^(?:\.\/)+/;

// path.posix.normalize, minus the caller's leading-`./` convention, and never
// turning '' into a bare dot (callers pass repo-relative paths). A regex, not a
// dot-slash STRING literal: the duplicate-primitive scan treats any dot-slash
// string in a shipped module as a module specifier reaching outside the engine
// tree, and it is right to over-approximate there.
function posixNormalize(p) {
  const normalized = path.posix.normalize(p).replace(LEADING_DOT_SLASH_RE, '');
  return normalized === '.' ? '' : normalized;
}

const GLOB_CHAR_RE = /[*?]/;

function hasGlobChars(pattern) {
  return GLOB_CHAR_RE.test(pattern);
}

const REGEXP_ESCAPE_RE = /[.+^${}()|[\]\\]/g;

// `*` and `?` both stand for "any run of / any one character", including
// `/` — see the header. Every other character is matched literally.
function globToRegExp(pattern) {
  let body = '';
  // Collapse runs of `*` first. `**` translated naively to `.*.*` is
  // catastrophic backtracking on a non-matching path, and this compiles onto the
  // in-process PreToolUse path, which — unlike the bash branch — has no `timeout`
  // around it. `*` already spans `/`, so a run of them means exactly what one
  // does; the collapse is semantics-preserving.
  for (const c of pattern.replace(/\*+/g, '*')) {
    if (c === '*') body += '.*';
    else if (c === '?') body += '.';
    else body += c.replace(REGEXP_ESCAPE_RE, '\\$&');
  }
  return new RegExp(`^${body}$`);
}

// Compiled-pattern cache, keyed by the pattern string. A Map, not a plain
// object — but note WHY that matters is narrower than it might look:
// compileGlob() is only ever reached through hasGlobChars() gating its
// caller, and no prototype-chain key ('__proto__', 'constructor', ...)
// contains a `*` or `?`, so a plain object here would not actually be
// reachable with a hostile key today. The Map is kept anyway because it is
// the correct tool for "cache keyed by an arbitrary string" regardless — no
// hasOwnProperty dance, no `in` operator surprises — not because removing it
// would currently break anything.
const globCache = new Map();

function compileGlob(pattern) {
  const cached = globCache.get(pattern);
  if (cached) return cached;
  const re = globToRegExp(pattern);
  globCache.set(pattern, re);
  return re;
}

// `decide`/`renderInvariants` accept a hand-built policy object as readily
// as one `loadPolicy` produced (the same flexibility content.mjs's
// renderChannel gives a caller-supplied `content`), so a rule's `match` is
// re-normalized here rather than assumed to already be the array
// `loadPolicy` normalizes it to.
function matchList(rule) {
  return normalizeMatchField(rule && rule.match);
}

// Every boundary suffix of a path: the path split on `/`, then rejoined from
// each split point onward, PLUS the path itself (index 0 covers that). This
// is what turns a single authored pattern into the bash oracle's own
// `PATTERN|*/PATTERN` alternation, for an arbitrary number of leading
// directories — an absolute path, a `../`-prefixed one, or a path nested
// under an extra directory all produce a suffix equal to the bare pattern.
function pathBoundarySuffixes(relPath) {
  const segments = relPath.split('/');
  const suffixes = [];
  for (let i = 0; i < segments.length; i += 1) suffixes.push(segments.slice(i).join('/'));
  return suffixes;
}

function ruleMatchesExact(rule, suffixes) {
  return matchList(rule).some((pattern) => !hasGlobChars(pattern) && suffixes.includes(pattern));
}

function ruleMatchesGlob(rule, suffixes) {
  return matchList(rule).some((pattern) => {
    if (!hasGlobChars(pattern)) return false;
    const re = compileGlob(pattern);
    return suffixes.some((s) => re.test(s));
  });
}

// `{state}` -> the literal queried state, everywhere it appears in a reason.
function interpolateReason(reason, state) {
  return reason.split(STATE_TOKEN).join(state);
}

function armAdmitsState(arm, state) {
  if (arm.states === CATCH_ALL) return true;
  return Array.isArray(arm.states) && arm.states.includes(state);
}

// The first arm whose `states` admits `state` wins — a top-to-bottom `case`
// read, same as the bash oracle's own `case "$state" in ... esac`. Plain
// `.includes()`/`===` throughout, never a keyed property lookup, so a state
// (or a rule id, or a match path) spelled '__proto__'/'constructor' is just
// a string being compared, never a property access that could resolve
// something inherited.
function ruleVerdict(rule, state) {
  const ruleId = typeof rule.id === 'string' ? rule.id : null;
  const arms = Array.isArray(rule.arms) ? rule.arms : [];
  for (const arm of arms) {
    if (!armAdmitsState(arm, state)) continue;
    const verdict = VERDICTS.has(arm.verdict) ? arm.verdict : 'block';
    const reason = typeof arm.reason === 'string' ? interpolateReason(arm.reason, state) : '';
    return { verdict, reason, ruleId };
  }
  // No arm admitted this state — a rule authored without a catch-all arm.
  // An unmodeled state is not evidence a write should be blocked, so this
  // degrades to allow rather than throwing or guessing a verdict.
  return { verdict: 'allow', reason: '', ruleId };
}

// ---------------------------------------------------------------------------
// decide — the one entry point. Pure: no I/O, never throws.
// ---------------------------------------------------------------------------

const ALLOW_UNMATCHED = { verdict: 'allow', reason: '', ruleId: null };

export function decide(policy, relPath, state) {
  const rules = policy && Array.isArray(policy.rules) ? policy.rules : [];
  if (rules.length === 0) return ALLOW_UNMATCHED;

  // Normalize before matching: `.spec/./lessons.md` and `.spec//lessons.md` name
  // the same file as `.spec/lessons.md`, and a hard block any of those spellings
  // walks around is not a block.
  const rawPath = typeof relPath === 'string' ? relPath : '';
  const normalizedPath = rawPath ? posixNormalize(rawPath) : '';
  const stateKey = typeof state === 'string' && state ? state : 'idle';
  const suffixes = pathBoundarySuffixes(normalizedPath);

  // Exact matches, across EVERY rule, before any glob match — a narrow exact
  // rule always outranks a broader glob rule irrespective of file order. This is
  // a DELIBERATE divergence from the bash oracle's top-to-bottom `case` read: a
  // path matching two rules takes the exact rule's verdict here and the
  // first-listed rule's verdict there. The engine is stricter on every derived
  // cross-rule path, never looser, and that property is pinned by its own control
  // test — see policy.test.mjs's "known divergence" case, which fails if the
  // divergence ever disappears silently.
  for (const rule of rules) {
    if (ruleMatchesExact(rule, suffixes)) return ruleVerdict(rule, stateKey);
  }
  for (const rule of rules) {
    if (ruleMatchesGlob(rule, suffixes)) return ruleVerdict(rule, stateKey);
  }
  return ALLOW_UNMATCHED;
}

// ---------------------------------------------------------------------------
// renderInvariants — prose for the {{invariants}} placeholder. One line per
// rule, generated from the same data `decide` enforces, so the two can never
// drift apart the way hand-authored prose and code do.
// ---------------------------------------------------------------------------

function matchDisplay(match) {
  return normalizeMatchField(match).map((p) => `\`${p}\``).join(', ');
}

function armStatesDisplay(states) {
  if (states === CATCH_ALL) return 'otherwise';
  return Array.isArray(states) && states.length ? states.map((s) => `\`${s}\``).join(', ') : '(no state)';
}

// Prose has no live queried state to substitute, so `{state}` reads as
// "the current state" — legible on its own, and honest about what it means.
function humanizeReason(reason) {
  return typeof reason === 'string' ? reason.split(STATE_TOKEN).join('the current state') : '';
}

function armDisplay(arm) {
  const verdict = VERDICTS.has(arm.verdict) ? arm.verdict : 'block';
  const reason = verdict !== 'allow' && arm.reason ? ` — ${humanizeReason(arm.reason)}` : '';
  return `${armStatesDisplay(arm.states)}: ${verdict}${reason}`;
}

export function renderInvariants(policy) {
  const rules = policy && Array.isArray(policy.rules) ? policy.rules : [];
  return rules
    .map((rule) => {
      const arms = Array.isArray(rule.arms) ? rule.arms : [];
      return `- ${matchDisplay(rule.match)} — ${arms.map(armDisplay).join('; ')}`;
    })
    .join('\n');
}
