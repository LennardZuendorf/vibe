#!/usr/bin/env bash
# validate-state.sh — check that state.json is well-formed and consistent with
# the state machine. Read-only. Exit 0 = valid, exit 1 = invalid/corrupt.
#
# Checks:
#   - state.json exists and is valid JSON (else: hint to copy state.example.json)
#   - required fields present: flow, phase, feature, updated
#   - the compound <flow>.<phase> resolves to a known state in the machine
#   - flow/phase are drawn from the machine's declared vocabularies

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SKILL_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
MACHINE="$SKILL_DIR/state-machine.json"
STATE="$SKILL_DIR/state.json"

ok() { echo "validate-state: OK — $1"; }
bad() { echo "validate-state: FAIL — $1" >&2; }

if ! command -v jq >/dev/null 2>&1; then
  bad "jq is required but not installed."
  exit 1
fi

if [[ ! -f "$MACHINE" ]]; then
  bad "state machine not found at $MACHINE"
  exit 1
fi

if [[ ! -f "$STATE" ]]; then
  bad "state.json not found. Copy the template: cp $SKILL_DIR/state.example.json $STATE"
  exit 1
fi

if ! jq -e . "$STATE" >/dev/null 2>&1; then
  bad "state.json is not valid JSON."
  exit 1
fi

for field in flow phase feature updated; do
  if ! jq -e "has(\"$field\")" "$STATE" >/dev/null 2>&1; then
    bad "state.json missing required field: $field"
    exit 1
  fi
done

FLOW=$(jq -r '.flow' "$STATE")
PHASE=$(jq -r '.phase' "$STATE")

# Resolve the compound key the same way set-state.sh builds it.
if [[ "$FLOW" == "$PHASE" ]]; then
  KEY="$FLOW"
else
  KEY="$FLOW.$PHASE"
fi

if ! jq -e --arg s "$KEY" '.states[$s]' "$MACHINE" >/dev/null 2>&1; then
  bad "'$KEY' is not a known state in the machine."
  exit 1
fi

if ! jq -e --arg f "$FLOW" '.flows | index($f)' "$MACHINE" >/dev/null 2>&1; then
  bad "flow '$FLOW' is not in the machine's declared flows."
  exit 1
fi

if ! jq -e --arg p "$PHASE" '.phases | index($p)' "$MACHINE" >/dev/null 2>&1; then
  bad "phase '$PHASE' is not in the machine's declared phases."
  exit 1
fi

ok "state=$KEY feature=$(jq -r '.feature // "none"' "$STATE")"
