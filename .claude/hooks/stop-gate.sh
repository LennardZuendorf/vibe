#!/usr/bin/env bash
# stop-gate.sh — vibe flow gate hook (platform-adapters/3).
#
# Event: Stop. End-of-turn smell checks over the flow state. WARN-ONLY: every
# predicate prints to stderr and the hook ALWAYS exits 0. Promote an individual
# predicate to blocking (decision:block) only after dogfooding proves it is
# crossed by accident, not on purpose ("earn the teeth"). Each predicate below
# carries a TODO marking it promotion-eligible.
#
# Thin shell: state comes from .agents/skills/vibe/scripts/detect-context.sh snapshot;
# no flow policy is duplicated here.
#
# Graceful degrade (R9): missing jq / detect-context.sh / not a git repo -> the
# affected check is skipped; the hook still exits 0.

set -euo pipefail

cat >/dev/null 2>&1 || true   # consume stdin

ROOT="${CLAUDE_PROJECT_DIR:-$PWD}"
DETECT="$ROOT/.agents/skills/vibe/scripts/detect-context.sh"

command -v jq >/dev/null 2>&1 || exit 0
[[ -f "$DETECT" ]] || exit 0

SNAP="$(bash "$DETECT" snapshot 2>/dev/null || echo '{}')"
STATE="$(printf '%s' "$SNAP" | jq -r '.state // "idle"' 2>/dev/null || echo idle)"
NEXT="$(printf '%s' "$SNAP" | jq -r '(.next // []) | join(", ")' 2>/dev/null || echo "")"

warn() { echo "vibe-gate: $1" >&2; }

git_changed() {
  command -v git >/dev/null 2>&1 || return 1
  git -C "$ROOT" rev-parse --is-inside-work-tree >/dev/null 2>&1 || return 1
  git -C "$ROOT" status --porcelain 2>/dev/null
}

# Predicate 1 — impl touched src/** but no tests/**.
# TODO(earn-the-teeth): promote to blocking once dogfood shows it only fires on
# genuine TDD misses, never on doc-only or refactor turns.
if [[ "$STATE" == "feature.impl" || "$STATE" == "quick.fix" ]]; then
  CH="$(git_changed || true)"
  if [[ -n "$CH" ]]; then
    if grep -qE '(^|/)src/' <<<"$CH" && ! grep -qE '(^|/)tests?/' <<<"$CH"; then
      warn "in $STATE, src changed with no test changes — TDD expects a reproducing/covering test. (warn-only)"
    fi
  fi
fi

# Predicate 2 — verify state but no review artifact noticed.
# TODO(earn-the-teeth): no reliable signal from a snapshot; left as a reminder
# until a review marker exists to check.
if [[ "$STATE" == "feature.verify" || "$STATE" == "quick.verify" ]]; then
  warn "in $STATE — confirm real evidence per unit ID and a code review before shipping. (warn-only)"
fi

# Predicate 3 — stuck phase / forgotten set-state.sh.
# TODO(earn-the-teeth): needs a turn counter, which the cursor deliberately omits
# (prompt-cache discipline). Surface the next legal states as a nudge for now.
if [[ "$STATE" != "idle" && -n "$NEXT" ]]; then
  warn "still in $STATE — when this phase's exit is met, advance with set-state.sh (next: $NEXT). (warn-only)"
fi

exit 0
