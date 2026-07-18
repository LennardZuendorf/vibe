#!/usr/bin/env bash
# doctrine.sh — emit the vibe working-model doctrine for the SessionStart hook.
#
#   doctrine.sh    # prints the doctrine block + a one-line cursor summary
#
# The doctrine is single-sourced from the vibe skill's SKILL.md, between the
#   <!-- vibe:doctrine -->
#   ...working model, two gates, write invariants...
#   <!-- /vibe:doctrine -->
# markers — the same single-source pattern orders.sh uses for per-turn orders, so
# the SessionStart hook stays a thin shell and the AGENTS.md managed block becomes
# an optional adapter rather than the only carrier of the doctrine.
#
# Read-only. Always exits 0: a missing block / skill / SKILL.md degrades to no
# output — never a session-ending failure. jq is recommended, not required; the
# cursor read falls back to sed and is byte-identical to the jq path.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SKILL_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

# Self-locate the skills dir from the script's own path (never a repo-root marker a
# fresh install may lack) — identical logic to orders.sh so the two resolvers agree.
find_repo_root() {
  local d="$1"
  while [[ -n "$d" && "$d" != "/" ]]; do
    if [[ -d "$d/.spec" || -e "$d/.git" ]]; then printf '%s\n' "$d"; return 0; fi
    d="$(dirname "$d")"
  done
  return 1
}
SKILL_PARENT="$(cd "$SKILL_DIR/.." && pwd)"
if [[ -f "$SKILL_PARENT/vibe/SKILL.md" ]]; then
  SKILLS_DIR="$SKILL_PARENT"
else
  REPO_ROOT="$(find_repo_root "$SCRIPT_DIR")" || REPO_ROOT="$SKILL_PARENT"
  SKILLS_DIR="$REPO_ROOT/.agents/skills"
fi
SKILL_MD="$SKILLS_DIR/vibe/SKILL.md"
STATE="$SKILL_DIR/state.json"

have_jq() { command -v jq >/dev/null 2>&1; }

# Cursor reads mirror orders.sh exactly (jq path, else sed over the flat cursor) so
# the summary line is byte-identical with or without jq.
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

current_feature() {
  [[ -f "$STATE" ]] || return 0
  if have_jq; then
    jq -r '.feature // empty' "$STATE" 2>/dev/null || true
  else
    sed -n 's/^[[:space:]]*"feature"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' "$STATE" | head -n1 || true
  fi
}

# Extract the doctrine block (marker lines excluded). Pure sed — no jq, no awk — so
# the no-jq degrade path is byte-identical, matching orders.sh's extract_block.
extract_doctrine() {
  [[ -f "$SKILL_MD" ]] || return 1
  sed -n '\|^<!-- vibe:doctrine -->$|,\|^<!-- /vibe:doctrine -->$|p' "$SKILL_MD" \
    | sed '1d;$d'
}

DOCTRINE="$(extract_doctrine || true)"
[[ -n "$DOCTRINE" ]] || exit 0

printf '%s\n' "$DOCTRINE"

ST="$(current_state)"
FT="$(current_feature)"
if [[ -n "$FT" ]]; then
  printf 'Cursor: %s (feature=%s).\n' "$ST" "$FT"
else
  printf 'Cursor: %s.\n' "$ST"
fi
exit 0
