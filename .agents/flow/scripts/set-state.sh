#!/usr/bin/env bash
# set-state.sh — the ONLY sanctioned writer of .agents/flow/state.json.
#
# Usage:
#   set-state.sh <flow.phase> [feature]
#   set-state.sh idle
#
# Examples:
#   set-state.sh strategy.brainstorm
#   set-state.sh feature.design auth-tokens
#   set-state.sh feature.impl            # preserves the current feature
#   set-state.sh idle
#
# Behaviour:
#   - Validates the target state exists in state-machine.json.
#   - Splits <flow>.<phase> into the cursor's flow/phase fields.
#   - Preserves `feature` unless a new one is given (or moving to idle clears it).
#   - Writes atomically (temp file + mv). Never touches anything turn-varying
#     beyond the `updated` timestamp, which lives in the cursor only — never in
#     any injected text.
#
# This script does NOT enforce transition legality; the /flow command and the
# vibe-* skills check `next` before calling it. This is the writer, not the gate.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
FLOW_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
MACHINE="$FLOW_DIR/state-machine.json"
STATE="$FLOW_DIR/state.json"

err() { echo "set-state: $1" >&2; }

if ! command -v jq >/dev/null 2>&1; then
  err "ERROR: jq is required but not installed."
  exit 1
fi

if [[ ! -f "$MACHINE" ]]; then
  err "ERROR: state machine not found at $MACHINE"
  exit 1
fi

TARGET="${1:-}"
NEW_FEATURE="${2:-}"

if [[ -z "$TARGET" ]]; then
  err "ERROR: no target state given. Usage: set-state.sh <flow.phase> [feature]"
  exit 1
fi

# amend is a modifier, not a stored cursor state — it edits scope and returns.
if [[ "$TARGET" == "amend" ]]; then
  err "amend is a modifier, not a cursor state. Run vibe-amend for a targeted"
  err "scope edit, then continue in your current state. The cursor is unchanged."
  exit 1
fi

# Validate target exists in the machine.
if ! jq -e --arg s "$TARGET" '.states[$s]' "$MACHINE" >/dev/null 2>&1; then
  LEGAL=$(jq -r '.states | keys | map(select(. != "amend")) | join(", ")' "$MACHINE")
  err "ERROR: '$TARGET' is not a known state."
  err "Known states: $LEGAL"
  exit 1
fi

# Split <flow>.<phase>. Single-token states (idle) map flow==phase.
if [[ "$TARGET" == *.* ]]; then
  FLOW="${TARGET%%.*}"
  PHASE="${TARGET#*.}"
else
  FLOW="$TARGET"
  PHASE="$TARGET"
fi

# Determine the feature to carry forward.
CUR_FEATURE="null"
if [[ -f "$STATE" ]]; then
  CUR_FEATURE=$(jq -r '.feature // "null"' "$STATE" 2>/dev/null || echo "null")
fi

if [[ -n "$NEW_FEATURE" ]]; then
  FEATURE_JSON=$(jq -n --arg f "$NEW_FEATURE" '$f')
elif [[ "$FLOW" == "idle" ]]; then
  FEATURE_JSON="null"   # leaving all flows clears the feature pointer
elif [[ "$CUR_FEATURE" != "null" && -n "$CUR_FEATURE" ]]; then
  FEATURE_JSON=$(jq -n --arg f "$CUR_FEATURE" '$f')
else
  FEATURE_JSON="null"
fi

# Warn (do not block) when a feature.* state has no feature attached.
if [[ "$FLOW" == "feature" && "$FEATURE_JSON" == "null" ]]; then
  err "WARN: entering '$TARGET' with no feature set. Pass one: set-state.sh $TARGET <feature>"
fi

NOW="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
TMP="$(mktemp "${STATE}.XXXXXX")"
trap 'rm -f "$TMP"' EXIT

jq -n \
  --arg flow "$FLOW" \
  --arg phase "$PHASE" \
  --argjson feature "$FEATURE_JSON" \
  --arg updated "$NOW" \
  '{flow: $flow, phase: $phase, feature: $feature, updated: $updated}' > "$TMP"

mv -f "$TMP" "$STATE"
trap - EXIT

# Echo the new state and its legal next states for the caller.
NEXT=$(jq -r --arg s "$TARGET" '.states[$s].next | join(", ")' "$MACHINE")
echo "-> $TARGET"
if [[ -n "$NEXT" ]]; then
  echo "   next: $NEXT"
fi
