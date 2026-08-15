// engine/policy.mjs — loadPolicy()/decide()/renderInvariants(): the write
// invariants as data (content/policy.json), replacing the three hardcoded
// blocks a bash guard would otherwise hand-maintain. A later unit wires that
// guard to delegate here; this module only has to be correct on its own.
//
// Rule shape (content/policy.json):
//   { id, match, states, verdict, reason }
// `match` is a relative path, or an array of them; each entry is either an
// EXACT path or a GLOB (`*` — any run of characters other than `/`; `**` —
// any run including `/`). `states` lists the states in which the rule's
// verdict is always 'allow'; the rule's own `verdict` field is what applies
// OUTSIDE that list. Rules are tried in file order, exact matches (across
// every rule) before glob matches (across every rule) — so a narrow exact
// rule always outranks a broader glob rule regardless of which comes first
// in the file — and the first rule that matches wins.
//
// Contract, matching every other engine module: NEVER throws. An absent or
// malformed policy.json degrades to "no rules" — which makes `decide` answer
// 'allow' for every path — and reports through `errors`. Failing shut (no
// write is restricted) is recoverable; a hook that throws wedges the session.

import path from 'node:path';
import { readJson } from './json.mjs';

export const POLICY_RELPATH = path.join('content', 'policy.json');

const VERDICTS = new Set(['allow', 'warn', 'block']);

// ---------------------------------------------------------------------------
// loadPolicy — read + validate. Every per-rule defect is reported and that
// ONE rule is dropped; one bad rule must never take the rest of the file
// down with it.
// ---------------------------------------------------------------------------

function normalizeMatchField(value) {
  const list = Array.isArray(value) ? value : [value];
  return list.filter((entry) => typeof entry === 'string' && entry.length > 0);
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
    const states = Array.isArray(entry.states) ? entry.states.filter((s) => typeof s === 'string') : [];
    const verdict = VERDICTS.has(entry.verdict) ? entry.verdict : 'block';
    if (entry.verdict !== undefined && !VERDICTS.has(entry.verdict)) {
      errors.push(`policy: rule '${id}' has an unknown verdict '${entry.verdict}' — defaulting to 'block'`);
    }
    const reason = typeof entry.reason === 'string' ? entry.reason : '';
    rules.push({ id, match, states, verdict, reason });
  });

  return { rules, errors };
}

// ---------------------------------------------------------------------------
// Matching — glob support kept intentionally small (no dependency): `*`
// stands for any run of characters other than `/`, `**` for any run
// including `/`. Every other character is matched literally.
// ---------------------------------------------------------------------------

const GLOB_CHAR_RE = /[*?]/;

function hasGlobChars(pattern) {
  return GLOB_CHAR_RE.test(pattern);
}

const REGEXP_ESCAPE_RE = /[.+^${}()|[\]\\]/g;

function globToRegExp(pattern) {
  let body = '';
  for (let i = 0; i < pattern.length; i += 1) {
    const c = pattern[i];
    if (c === '*' && pattern[i + 1] === '*') {
      body += '.*';
      i += 1;
    } else if (c === '*') {
      body += '[^/]*';
    } else if (c === '?') {
      body += '[^/]';
    } else {
      body += c.replace(REGEXP_ESCAPE_RE, '\\$&');
    }
  }
  return new RegExp(`^${body}$`);
}

// Compiled-pattern cache. Keyed by the pattern STRING through a Map, never a
// plain object — a plain object indexed by attacker-shaped input (a rule's
// own `match` entry, ultimately policy.json data) would resolve inherited
// members for a key like '__proto__' or 'constructor' instead of a cached
// regex. Map has no prototype-chain lookup hazard at all.
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

function ruleMatchesExact(rule, relPath) {
  return matchList(rule).some((pattern) => pattern === relPath);
}

function ruleMatchesGlob(rule, relPath) {
  return matchList(rule).some((pattern) => hasGlobChars(pattern) && compileGlob(pattern).test(relPath));
}

// `state` a rule allows through as 'allow' regardless of its own verdict.
// Plain `.includes()` over an array — never a keyed lookup — so a state (or
// a rule id, or a match path) spelled '__proto__'/'constructor' is just a
// string being compared, never a property access that could resolve
// something inherited.
function ruleVerdict(rule, state) {
  const states = Array.isArray(rule.states) ? rule.states : [];
  const ruleId = typeof rule.id === 'string' ? rule.id : null;
  if (states.includes(state)) return { verdict: 'allow', reason: '', ruleId };
  const verdict = VERDICTS.has(rule.verdict) ? rule.verdict : 'block';
  const reason = typeof rule.reason === 'string' ? rule.reason : '';
  return { verdict, reason, ruleId };
}

// ---------------------------------------------------------------------------
// decide — the one entry point. Pure: no I/O, never throws.
// ---------------------------------------------------------------------------

const ALLOW_UNMATCHED = { verdict: 'allow', reason: '', ruleId: null };

export function decide(policy, relPath, state) {
  const rules = policy && Array.isArray(policy.rules) ? policy.rules : [];
  if (rules.length === 0) return ALLOW_UNMATCHED;

  const normalizedPath = typeof relPath === 'string' ? relPath.replace(/^\.\/+/, '') : '';
  const stateKey = typeof state === 'string' && state ? state : 'idle';

  // Exact matches, across EVERY rule, before any glob match — a narrow exact
  // rule always outranks a broader glob rule irrespective of file order.
  for (const rule of rules) {
    if (ruleMatchesExact(rule, normalizedPath)) return ruleVerdict(rule, stateKey);
  }
  for (const rule of rules) {
    if (ruleMatchesGlob(rule, normalizedPath)) return ruleVerdict(rule, stateKey);
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

function statesDisplay(states) {
  const list = Array.isArray(states) ? states : [];
  return list.length ? list.map((s) => `\`${s}\``).join(', ') : '(no state)';
}

export function renderInvariants(policy) {
  const rules = policy && Array.isArray(policy.rules) ? policy.rules : [];
  return rules
    .map((rule) => {
      const verdict = VERDICTS.has(rule.verdict) ? rule.verdict : 'block';
      const reason = typeof rule.reason === 'string' && rule.reason ? ` — ${rule.reason}` : '';
      return `- ${matchDisplay(rule.match)}: ${verdict} outside ${statesDisplay(rule.states)}${reason}`;
    })
    .join('\n');
}
