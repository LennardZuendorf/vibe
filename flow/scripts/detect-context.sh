#!/usr/bin/env bash
# detect-context.sh — single source of truth for "where are we" and "is this
# write allowed here". Read-only. Two modes:
#
#   detect-context.sh                  # emit a JSON snapshot of the current state
#   detect-context.sh decide <path>    # allow|warn|block for writing <path> now
#   detect-context.sh decide <path> <state>   # ... as if in <state> (testing)
#   detect-context.sh infer [<porcelain>] [<state>]  # drift:<state>:<reason> when
#                                      working-tree activity contradicts the cursor
#
# The decision policy is DATA (content/policy.json), read by the engine's
# `vibe policy decide`. This script is the one entry point every adapter's hook
# calls, and it translates nothing: the verdict is stdout, the exit code stays
# 0. `decide` delegates to the engine when node is available and answers from
# its own bash branch when it is not.
#
# The bash branch is PERMANENT, not a migration stop-gap (inject-triggers/2,
# plan decision 1). It backs a HARD BLOCK, and a target without node must still
# be enforced — losing the guard to a missing runtime is not acceptable. The
# differential matrix in flow/engine/tests/policy.test.mjs drives every guarded
# path x all 13 machine states through BOTH branches and asserts byte-identical
# stdout and exit code, so the duplication cannot drift silently.
#
# The three hard blocks (everything else is allow/warn):
#   1. .spec/lessons.md            — only during feature.compound, setup.apply,
#                                    strategy.spec, or quick.verify (the flow-end
#                                    states where the conditional lesson step lives)
#   2. root .spec/{product,tech,design,plan}.md
#                                  — only during strategy.spec, feature.compound, or setup.apply
#   3. .agents/skills/vibe/state.json — never by direct edit; only via set-state.sh

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SKILL_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
MACHINE="$SKILL_DIR/state-machine.json"
STATE="$SKILL_DIR/state.json"
ENGINE_CLI="$SKILL_DIR/engine/cli.mjs"
POLICY_JSON="$SKILL_DIR/content/policy.json"

have_jq() { command -v jq >/dev/null 2>&1; }
have_node() { command -v node >/dev/null 2>&1; }

# Locate the repo/install root by upward marker search (never fixed hops) — used
# only by `infer` to run `git status` from the right place when no porcelain is
# passed. CLAUDE_PROJECT_DIR wins when the hook sets it.
find_root() {
  local d="$1"
  while [[ -n "$d" && "$d" != "/" ]]; do
    if [[ -d "$d/.spec" || -e "$d/.git" ]]; then printf '%s\n' "$d"; return 0; fi
    d="$(dirname "$d")"
  done
  return 1
}

# Resolve the current compound state key from the cursor (default: idle).
# Without jq, fall back to sed — the cursor is machine-written flat JSON
# (set-state.sh), so the hard blocks stay state-aware instead of collapsing
# every state onto idle's stricter policy on jq-less targets.
current_state() {
  local flow="" phase=""
  if have_jq && [[ -f "$STATE" ]] && jq -e . "$STATE" >/dev/null 2>&1; then
    flow=$(jq -r '.flow // "idle"' "$STATE")
    phase=$(jq -r '.phase // "idle"' "$STATE")
  elif ! have_jq && [[ -f "$STATE" ]]; then
    flow=$(sed -n 's/.*"flow"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' "$STATE" | head -n1)
    phase=$(sed -n 's/.*"phase"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' "$STATE" | head -n1)
  fi
  [[ -n "$flow" && "$flow" != "null" ]] || flow="idle"
  [[ -n "$phase" && "$phase" != "null" ]] || phase="idle"
  if [[ "$flow" == "$phase" ]]; then echo "$flow"; else echo "$flow.$phase"; fi
}

# ── snapshot mode ────────────────────────────────────────────────────────────
snapshot() {
  local key; key="$(current_state)"
  if ! have_jq; then
    printf '{"state":"%s","jq":false,"note":"jq missing; degraded"}\n' "$key"
    return 0
  fi
  if [[ ! -f "$MACHINE" ]]; then
    printf '{"state":"%s","jq":true,"note":"state-machine.json missing; degraded"}\n' "$key"
    return 0
  fi
  local feature="null"
  [[ -f "$STATE" ]] && feature=$(jq -r '.feature // "null"' "$STATE" 2>/dev/null || echo null)
  jq -n \
    --arg state "$key" \
    --argjson entry "$(jq --arg s "$key" '.states[$s] // {}' "$MACHINE")" \
    --arg feature "$feature" \
    '{
       state: $state,
       feature: (if $feature == "null" then null else $feature end),
       skill: ($entry.skill // null),
       delegates: ($entry.delegates // []),
       reads: ($entry.reads // []),
       writes: ($entry.writes // []),
       inject: ($entry.inject // null),
       next: ($entry.next // []),
       exit: ($entry.exit // null)
     }'
}

# ── the engine branch ────────────────────────────────────────────────────────
# Runs `vibe policy decide` and prints its stdout. Returns non-zero for every
# "the engine did not answer" outcome, which the caller reads as "delegation
# unavailable" and falls back to the bash branch for.
#
# Three hardening points, all of them load-bearing rather than defensive
# habit (inject-triggers/2 review):
#
#   * `</dev/null` — the PreToolUse hook feeds this script's caller a JSON
#     event on stdin. Without the redirect the child node inherits that pipe;
#     a node that decides to read stdin then blocks forever and wedges the
#     hook, which is the one failure mode this harness must never have.
#   * a TIMEOUT when one is available. GNU coreutils `timeout` is present on
#     Linux and on any macOS with coreutils installed, but NOT on a stock
#     macOS (where it is `gtimeout`, if at all). It is used when found and
#     simply not used when absent — bounding the call is not worth a
#     dependency, and faking a bound with a background kill would add a race
#     to a hook path. On a target without `timeout`, a wedged node is
#     unbounded here; the `</dev/null` above removes the only cause this
#     script can control.
#   * a strict verdict shape on the way out (is_verdict). The hooks read any
#     line that is not `block:`/`warn:` as an allow, so forwarding whatever
#     the engine happened to print would turn a diagnostic, a warning banner,
#     or a partial line into a silent allow.
engine_decide() {
  local path="$1" state="$2"
  if command -v timeout >/dev/null 2>&1; then
    timeout 10 node "$ENGINE_CLI" policy --vibe-dir "$SKILL_DIR" decide "$path" "$state" 2>/dev/null </dev/null
  else
    node "$ENGINE_CLI" policy --vibe-dir "$SKILL_DIR" decide "$path" "$state" 2>/dev/null </dev/null
  fi
}

# Exactly the three shapes `decide` is contracted to emit, on ONE line: bare
# `allow`, or `warn:`/`block:` with a non-empty reason. Anything else — empty,
# multi-line, a bare `block:`, a stack trace — is not a verdict.
is_verdict() {
  local out="$1"
  [[ "$out" != *$'\n'* ]] || return 1
  case "$out" in
    allow) return 0 ;;
    warn:?*) return 0 ;;
    block:?*) return 0 ;;
    *) return 1 ;;
  esac
}

# ── decision mode ──────────────────────────────────────────────────────────—
# Emits one of: allow | warn:<reason> | block:<reason>
#
# The engine answers only when node can run it, the policy data is there, the
# engine EXITS 0, and what it printed is a well-formed verdict. Every other
# outcome falls back to the bash branch below, which carries the same policy
# hardcoded and so cannot be corrupted by a data file. The engine refuses
# (exit 2, empty stdout) whenever its rule set failed to load or loaded empty
# — a truncated policy.json, a version this engine does not understand, a
# missing `rules` key, an unreadable file — because `decide` over zero rules
# answers `allow` for every path, and reporting that as a verdict would make
# every hard block disappear silently.
#
# The state is resolved HERE, once, and passed explicitly to whichever branch
# answers — the engine is never left to read the cursor itself. Two branches
# that each resolve "where are we" would be two chances to disagree about it,
# and the differential matrix could then not tell a policy divergence from a
# cursor-resolution one.
decide() {
  local path="$1"
  local state="${2:-}"
  [[ -n "$state" ]] || state="$(current_state)"

  # Normalise a leading ./ before either branch sees it.
  path="${path#./}"

  local out
  if have_node && [[ -f "$ENGINE_CLI" && -f "$POLICY_JSON" ]] \
    && out="$(engine_decide "$path" "$state")" \
    && is_verdict "$out"; then
    printf '%s\n' "$out"
    return 0
  fi

  decide_bash "$path" "$state"
}

# The bash branch. Takes an ALREADY-normalised path and an explicit state (see
# decide above) — it is never called with either left to default.
decide_bash() {
  local path="$1"
  local state="$2"

  # Block 3: state.json is writer-only.
  case "$path" in
    .agents/skills/vibe/state.json|*/.agents/skills/vibe/state.json)
      echo "block:state.json is written only via set-state.sh, never by direct edit"
      return 0
      ;;
  esac

  # Block 1: lessons.md only during the flow-end states that carry the lesson step.
  case "$path" in
    .spec/lessons.md|*/.spec/lessons.md)
      case "$state" in
        feature.compound|setup.apply|strategy.spec|quick.verify) echo "allow" ;;
        *) echo "block:.spec/lessons.md is writable only during feature.compound, setup.apply, strategy.spec, or quick.verify (current: $state)" ;;
      esac
      return 0
      ;;
  esac

  # Block 2: root specs only during strategy.spec or feature.compound.
  case "$path" in
    .spec/product.md|.spec/tech.md|.spec/design.md|.spec/plan.md|\
*/.spec/product.md|*/.spec/tech.md|*/.spec/design.md|*/.spec/plan.md)
      case "$state" in
        strategy.spec|feature.compound) echo "allow" ;;
        setup.apply) echo "allow" ;;
        *) echo "block:root .spec specs are writable only during strategy.spec, feature.compound, or setup.apply (current: $state)" ;;
      esac
      return 0
      ;;
  esac

  # Warning: feature specs are frozen once implementation begins — feature.impl and
  # quick.fix write code, not spec. feature.design/plan (and setup) still author them.
  case "$path" in
    .spec/features/*|*/.spec/features/*)
      case "$state" in
        feature.impl|quick.fix) echo "warn:.spec/features edits are frozen during impl/fix — route back to feature.design/plan to change scope (current: $state)" ;;
        *) echo "allow" ;;
      esac
      return 0
      ;;
  esac

  # Warning (not a block): the managed active-rules block is generated output.
  case "$path" in
    CLAUDE.md|AGENTS.md|*/CLAUDE.md|*/AGENTS.md)
      echo "warn:CLAUDE.md/AGENTS.md active-rules block is generated by regen-active-rules.sh; edits inside the markers are overwritten next compound"
      return 0
      ;;
  esac

  # Warning: source edits outside an implementation/fix state; verify writes no src
  # — findings route back to the fix state, they are never applied in verify.
  case "$path" in
    src/*|tests/*|*/src/*|*/tests/*)
      case "$state" in
        feature.verify) echo "warn:verify writes no src — route findings back to impl (set-state.sh feature.impl)" ;;
        quick.verify)   echo "warn:verify writes no src — route findings back to fix (set-state.sh quick.fix)" ;;
        feature.impl|quick.fix|setup.apply) echo "allow" ;;
        *) echo "warn:source/test edits outside an impl/fix state (current: $state)" ;;
      esac
      return 0
      ;;
  esac

  echo "allow"
}

# ── drift inference ──────────────────────────────────────────────────────────
# infer [<porcelain>] [<state>] — emit `drift:<suggested-state>:<reason>` when
# working-tree activity contradicts the cursor, else nothing. Read-only and
# warn-only by construction: a false positive costs one advisory line, never a
# block. Porcelain omitted (__git__) => read `git status --porcelain` from
# CLAUDE_PROJECT_DIR (or a marker-located root); a passed porcelain string keeps it
# hermetic for tests. `state` defaults to the cursor.
infer() {
  local porcelain="$1" state="${2:-}"
  [[ -n "$state" ]] || state="$(current_state)"
  if [[ "$porcelain" == "__git__" ]]; then
    local root="${CLAUDE_PROJECT_DIR:-}"
    [[ -n "$root" ]] || root="$(find_root "$SCRIPT_DIR")" || root="$PWD"
    porcelain="$(git -C "$root" status --porcelain 2>/dev/null || true)"
  fi
  # States that legitimately churn src/tests — activity there is not drift.
  case "$state" in
    feature.impl|quick.fix|feature.verify|quick.verify|setup.apply) return 0 ;;
  esac
  # A porcelain line whose path starts with src/ or tests/ (after the 2-col status
  # + space) at a non-building state is likely cursor drift. `tests/` (plural)
  # matches what `decide` treats as code, so the two agree. The fix is stated as
  # `set-state.sh <state>` (the writer, legal from any state) — NOT `/flow`, which
  # enforces edge legality and would reject the hop from idle. Warn-only: a wrong
  # guess costs one advisory line, and the agent picks the state it is really in.
  if printf '%s\n' "$porcelain" | grep -qE '^.{3}(src|tests)/'; then
    case "$state" in
      feature.design|feature.plan)
        printf 'drift:feature.impl:src edits in %s — set-state.sh feature.impl to match your work, or set-state.sh idle to stop\n' "$state" ;;
      quick.triage)
        printf 'drift:quick.fix:src edits in quick.triage — set-state.sh quick.fix, or set-state.sh idle\n' ;;
      *)
        printf 'drift:feature.impl:src/tests edits at %s — set the cursor to match: set-state.sh feature.impl or quick.fix (else set-state.sh idle)\n' "$state" ;;
    esac
  fi
  return 0
}

case "${1:-}" in
  decide)
    if [[ -z "${2:-}" ]]; then
      echo "usage: detect-context.sh decide <path> [state]" >&2
      exit 1
    fi
    decide "$2" "${3:-}"
    ;;
  infer)
    if [[ $# -ge 2 ]]; then infer "$2" "${3:-}"; else infer "__git__" ""; fi
    ;;
  ""|snapshot)
    snapshot
    ;;
  *)
    echo "usage: detect-context.sh [snapshot | decide <path> [state] | infer [porcelain] [state]]" >&2
    exit 1
    ;;
esac
