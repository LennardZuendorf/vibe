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
# The decision policy lives HERE, once, so every adapter's hook is a thin shell
# that calls this and translates the verdict to its own exit-code convention.
# This houses the future PreToolUse decision fn (Stage 2); in Stage 1 it is the
# canonical reference the skills consult.
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

have_jq() { command -v jq >/dev/null 2>&1; }

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

# ── decision mode ──────────────────────────────────────────────────────────—
# Emits one of: allow | warn:<reason> | block:<reason>
decide() {
  local path="$1"
  local state="${2:-$(current_state)}"

  # Normalise a leading ./
  path="${path#./}"

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
  # + space) at a non-building state is likely cursor drift. Route to the build
  # state that matches the CURRENT flow — feature states point at feature.impl, the
  # quick flow points at quick.fix, and ambiguous hubs (idle/strategy/setup) name
  # both. Warn-only: a wrong guess costs one advisory line.
  if printf '%s\n' "$porcelain" | grep -qE '^.{3}(src|tests?)/'; then
    case "$state" in
      feature.design|feature.plan)
        printf 'drift:feature.impl:src edits in %s before the impl gate — advance to feature.impl (after the plan gate) or set-state.sh idle\n' "$state" ;;
      quick.triage)
        printf 'drift:quick.fix:src edits in quick.triage — run set-state.sh quick.fix or set-state.sh idle\n' ;;
      *)
        printf 'drift:feature.impl:src/tests edits at %s — run /flow feature.impl or quick.fix (or set-state.sh idle)\n' "$state" ;;
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
