#!/usr/bin/env bash
# set-state.sh — the ONLY sanctioned writer of .agents/skills/vibe/state.json.
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
SKILL_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
MACHINE="$SKILL_DIR/state-machine.json"
STATE="$SKILL_DIR/state.json"

err() { echo "set-state: $1" >&2; }

# jq is recommended, not required. Without it the cursor is still written (via
# printf, byte-identical to the jq path — the cursor is a flat 4-key JSON) and the
# target state is validated by a text match instead of a JSON query.
HAVE_JQ=0
command -v jq >/dev/null 2>&1 && HAVE_JQ=1

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

# Validate target exists in the machine.
if [[ "$HAVE_JQ" -eq 1 ]]; then
  if ! jq -e --arg s "$TARGET" '.states[$s]' "$MACHINE" >/dev/null 2>&1; then
    LEGAL=$(jq -r '.states | keys | join(", ")' "$MACHINE")
    err "ERROR: '$TARGET' is not a known state."
    err "Known states: $LEGAL"
    exit 1
  fi
else
  # No jq: best-effort validation. State entries are objects at EXACTLY 4-space
  # indentation in the checked-in machine ("<name>": {); meta keys
  # ("style"/"version"/"initial"/"states"…) sit at 2 spaces, nested state fields
  # ("skill"/"reads"…) at 6+, and the 4-space `gates` keys carry string values (not
  # `{`). Anchoring on `^    "<name>": {` therefore matches only real states, so a
  # meta key like `style` or `version` is rejected. Relying on the fixed indent is
  # safe: state-machine.json is a versioned, checked-in file with stable formatting.
  if ! grep -qE "^    \"${TARGET//./\\.}\"[[:space:]]*:[[:space:]]*\{" "$MACHINE"; then
    err "ERROR: '$TARGET' is not a known state (jq absent; validated by text match)."
    exit 1
  fi
fi

# Split <flow>.<phase>. Single-token states (idle) map flow==phase.
if [[ "$TARGET" == *.* ]]; then
  FLOW="${TARGET%%.*}"
  PHASE="${TARGET#*.}"
else
  FLOW="$TARGET"
  PHASE="$TARGET"
fi

# json_string VALUE — emit VALUE as a JSON string literal. Uses jq when present
# (full escaping); without jq, escapes backslash and double-quote — enough for the
# slug-shaped feature names the cursor carries.
json_string() {
  if [[ "$HAVE_JQ" -eq 1 ]]; then
    jq -n --arg f "$1" '$f'
  else
    local s="$1"
    s="${s//\\/\\\\}"
    s="${s//\"/\\\"}"
    printf '"%s"' "$s"
  fi
}

# Determine the feature to carry forward.
CUR_FEATURE="null"
if [[ -f "$STATE" ]]; then
  if [[ "$HAVE_JQ" -eq 1 ]]; then
    CUR_FEATURE=$(jq -r '.feature // "null"' "$STATE" 2>/dev/null || echo "null")
  else
    # Flat cursor: read the feature string; a null (unquoted) value yields empty.
    CUR_FEATURE="$(sed -n 's/^[[:space:]]*"feature"[[:space:]]*:[[:space:]]*"\(.*\)".*/\1/p' "$STATE" | head -n1 || true)"
    [[ -n "$CUR_FEATURE" ]] || CUR_FEATURE="null"
  fi
fi

if [[ -n "$NEW_FEATURE" ]]; then
  FEATURE_JSON=$(json_string "$NEW_FEATURE")
elif [[ "$FLOW" == "idle" ]]; then
  FEATURE_JSON="null"   # leaving all flows clears the feature pointer
elif [[ "$CUR_FEATURE" != "null" && -n "$CUR_FEATURE" ]]; then
  FEATURE_JSON=$(json_string "$CUR_FEATURE")
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

if [[ "$HAVE_JQ" -eq 1 ]]; then
  jq -n \
    --arg flow "$FLOW" \
    --arg phase "$PHASE" \
    --argjson feature "$FEATURE_JSON" \
    --arg updated "$NOW" \
    '{flow: $flow, phase: $phase, feature: $feature, updated: $updated}' > "$TMP"
else
  # Byte-identical to jq's pretty-print: 2-space indent, this key order, trailing
  # newline. FEATURE_JSON is already a JSON value (a quoted string or null).
  printf '{\n  "flow": "%s",\n  "phase": "%s",\n  "feature": %s,\n  "updated": "%s"\n}\n' \
    "$FLOW" "$PHASE" "$FEATURE_JSON" "$NOW" > "$TMP"
fi

mv -f "$TMP" "$STATE"
trap - EXIT

# Echo the new state and its legal next states for the caller (jq-only hint).
echo "-> $TARGET"
if [[ "$HAVE_JQ" -eq 1 ]]; then
  NEXT=$(jq -r --arg s "$TARGET" '.states[$s].next | join(", ")' "$MACHINE")
  if [[ -n "$NEXT" ]]; then
    echo "   next: $NEXT"
  fi
fi
