#!/usr/bin/env bash
# set-state.sh — the only sanctioned entry point for writing the flow cursor.
#
# Usage:
#   set-state.sh <flow.phase> [feature] [--confirm]
#   set-state.sh idle
#
# Examples:
#   set-state.sh strategy.brainstorm
#   set-state.sh feature.design auth-tokens
#   set-state.sh feature.impl              # preserves the current feature
#   set-state.sh feature.impl --confirm    # crosses a human gate
#   set-state.sh idle
#
# Behaviour:
#   - With `vibe-flow` on PATH it execs `vibe-flow state set` with the same argv.
#     The binary is the one gate (D27); this script is a caller, not a second
#     copy of the rules. `--root`/`--vibe-dir` pin the layout resolved here, so
#     the caller's working directory cannot redirect the write. The binary's
#     refusal and its `-> <state>` line pass through verbatim.
#   - Without the binary — every install target that has no Rust build — it
#     keeps the ungated writer below and says on stderr that the gate was not
#     enforced. Installs stay functional; they are never silently gated shut.
#   - The fallback writer validates the target state exists in
#     state-machine.json, splits <flow>.<phase> into flow/phase, preserves
#     `feature` unless a new one is given (idle clears it), and writes atomically
#     (temp file + mv). It touches nothing turn-varying beyond `updated`, which
#     lives in the cursor only — never in any injected text.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SKILL_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
MACHINE="$SKILL_DIR/state-machine.json"
STATE="$SKILL_DIR/state.json"
if [[ -f "$SKILL_DIR/scripts/runtime-state.sh" ]]; then
  # shellcheck source=flow/scripts/runtime-state.sh
  source "$SKILL_DIR/scripts/runtime-state.sh"
  STATE="$(runtime_state_path "$SKILL_DIR")"
fi

err() { echo "set-state: $1" >&2; }

# One gate, one writer (D27). When the binary is on PATH it owns the write and
# this script's whole job is to hand it the same argv — including `--confirm`,
# which the binary parses as a flag. The root and data home are pinned to what
# this script already resolved: a caller's working directory must not be able to
# point the cursor at another project.
if command -v vibe-flow >/dev/null 2>&1; then
  DELEGATE_ROOT=""
  if declare -F runtime_project_root >/dev/null; then
    DELEGATE_ROOT="$(runtime_project_root "$SKILL_DIR")" || DELEGATE_ROOT=""
  fi
  if [[ -n "$DELEGATE_ROOT" ]]; then
    exec vibe-flow --root "$DELEGATE_ROOT" --vibe-dir "$SKILL_DIR" state set "$@"
  fi
  err "WARN: no project root resolved — the transition gate was not enforced."
else
  err "WARN: vibe-flow is not on PATH — the transition gate was not enforced."
fi

# jq is recommended, not required. Without it the cursor is still written (via
# printf, byte-identical to the jq path — the cursor is a flat 4-key JSON) and the
# target state is validated by a text match instead of a JSON query.
HAVE_JQ=0
command -v jq >/dev/null 2>&1 && HAVE_JQ=1

if [[ ! -f "$MACHINE" ]]; then
  err "ERROR: state machine not found at $MACHINE"
  exit 1
fi

TARGET=""
NEW_FEATURE=""
# `--confirm` may appear anywhere after the target (the /flow contract). The
# ungated writer below has no gate to confirm, so the flag is dropped there
# rather than read as a feature name.
POSITIONAL=0
for arg in "$@"; do
  if [[ "$arg" == "--confirm" ]]; then
    continue
  fi
  POSITIONAL=$((POSITIONAL + 1))
  case "$POSITIONAL" in
    1) TARGET="$arg" ;;
    2) NEW_FEATURE="$arg" ;;
  esac
done
# An empty feature argument means "no feature argument" (carry forward / clear),
# exactly as in the engine writer (`vibe state set`).
FEATURE_GIVEN=0
[[ -n "$NEW_FEATURE" ]] && FEATURE_GIVEN=1

# valid_feature VALUE — the charset both cursor writers accept (the engine's
# validFeature in engine/cursor.mjs is the twin): non-empty, no control byte, no
# double quote, no backslash. Slug-shaped and spaced names pass; anything that
# would need JSON escaping (or break the cursor re-read without jq) does not.
# LC_ALL=C pins [[:cntrl:]] to C0 and DEL: under a UTF-8 locale it also matches
# C1 code points, which the engine's check (and JSON) treat as ordinary text.
valid_feature() {
  local LC_ALL=C
  [[ -n "$1" ]] || return 1
  case "$1" in
    *[[:cntrl:]]*|*\"*|*\\*) return 1 ;;
  esac
  return 0
}

# validate_args — reject a malformed state or feature before anything is read or
# written, so the cursor is never rewritten to invalid JSON.
validate_args() {
  if [[ -n "$TARGET" && ! "$TARGET" =~ ^[a-z]+(\.[a-z]+)?$ ]]; then
    err "ERROR: invalid state '$TARGET' (expected <flow>.<phase> or a single lowercase word)."
    exit 1
  fi
  if [[ "$FEATURE_GIVEN" -eq 1 ]] && ! valid_feature "$NEW_FEATURE"; then
    err "ERROR: invalid feature name (no control character, double quote or backslash allowed)."
    exit 1
  fi
}

if [[ -z "$TARGET" ]]; then
  err "ERROR: no target state given. Usage: set-state.sh <flow.phase> [feature] [--confirm]"
  exit 1
fi

validate_args

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
# (full escaping); without jq, escapes backslash and double-quote — enough because
# validate_args already rejects a feature with a control byte, `"` or `\`.
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

# read_cur_feature — classify the existing cursor's feature, as the engine
# writer does (engine/commands/state.mjs). Prints `n` for a missing, null or
# empty-string feature (nothing to carry, no WARN), `x` for any other non-string
# value (number, boolean, object, array), or `s<value>.` for a string. The
# trailing `.` keeps a trailing newline in the value from being stripped by the
# command substitution. An unreadable cursor reads as `n`, as in the engine.
read_cur_feature() {
  local line val
  if [[ "$HAVE_JQ" -eq 1 ]]; then
    jq -r 'if .feature == null or .feature == "" then "n"
           elif (.feature | type) == "string" then "s" + .feature + "."
           else "x" end' "$STATE" 2>/dev/null || printf 'n'
    return 0
  fi
  # No jq: the flat cursor holds the feature on one `"feature": <value>` line.
  line="$(sed -n '/^[[:space:]]*"feature"[[:space:]]*:/p' "$STATE" | head -n1 || true)"
  val="${line#*:}"
  val="${val#"${val%%[![:space:]]*}"}"
  case "$val" in
    '' | null | null,) printf 'n' ;;
    \"*)
      val="$(printf '%s\n' "$line" | sed -n 's/^[[:space:]]*"feature"[[:space:]]*:[[:space:]]*"\(.*\)".*/\1/p')"
      if [[ -n "$val" ]]; then printf 's%s.' "$val"; else printf 'n'; fi
      ;;
    *) printf 'x' ;;
  esac
}

# Determine the feature to carry forward. CUR_FEATURE stays empty when there is
# none. A corrupt cursor must not be re-emitted: reuse only a string feature
# that passes the same check as a new one; drop any other value with a WARN.
CUR_FEATURE=""
if [[ -f "$STATE" ]]; then
  CUR_RAW="$(read_cur_feature)"
  case "$CUR_RAW" in
    s*)
      CUR_FEATURE="${CUR_RAW#s}"
      CUR_FEATURE="${CUR_FEATURE%.}"
      if ! valid_feature "$CUR_FEATURE"; then
        err "WARN: ignoring an invalid feature in the existing cursor."
        CUR_FEATURE=""
      fi
      ;;
    x) err "WARN: ignoring an invalid feature in the existing cursor." ;;
  esac
fi

if [[ "$FEATURE_GIVEN" -eq 1 ]]; then
  FEATURE_JSON=$(json_string "$NEW_FEATURE")
elif [[ "$FLOW" == "idle" ]]; then
  FEATURE_JSON="null"   # leaving all flows clears the feature pointer
elif [[ -n "$CUR_FEATURE" ]]; then
  FEATURE_JSON=$(json_string "$CUR_FEATURE")
else
  FEATURE_JSON="null"
fi

# Warn (do not block) when a feature.* state has no feature attached.
if [[ "$FLOW" == "feature" && "$FEATURE_JSON" == "null" ]]; then
  err "WARN: entering '$TARGET' with no feature set. Pass one: set-state.sh $TARGET <feature>"
fi

if declare -F runtime_check_write >/dev/null; then
  runtime_root="$(runtime_project_root "$SKILL_DIR")" || runtime_root="$SKILL_DIR"
  if ! runtime_check_write "$runtime_root" "$STATE"; then
    err "ERROR: unsafe cursor path (outside project, unresolved, or a directory): $STATE"
    exit 1
  fi
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
