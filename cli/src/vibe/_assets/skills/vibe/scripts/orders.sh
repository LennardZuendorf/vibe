#!/usr/bin/env bash
# orders.sh — resolve the per-turn "orders" for the current (or a given) state.
#
#   orders.sh                 # orders for the current cursor state (default idle)
#   orders.sh <flow.phase>    # orders for an explicit state (testing)
#
# D12: the orders live in the linked vibe skill (the state's phase file) as a
#   <!-- vibe:orders:<state> -->
#   skill=... · ... · next: ...
#   <!-- /vibe:orders -->
# block. state-machine.json carries inject:null for skill-owning states and the
# `skill` field links the state to its shim. This script follows that link and
# prints the matching block verbatim, so the UserPromptSubmit inject hook stays a
# thin shell. `idle` (and any skill-less state) falls back to the machine's inline
# `inject` string. `<feature>` is the only interpolation — keeping the output
# byte-stable per state so the prompt cache holds.
#
# Read-only. Always exits 0: missing jq / skill / block degrade to the machine
# inject, then to a one-line generic fallback — never a session-ending failure.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SKILL_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

# Resolve the skills dir (where sibling skills like vibe + spec live) from the
# script's own location — never from a repo-root marker, which a freshly-installed
# target may not have yet (no .git, no .spec). In an installed target and via the
# .agents/skills/vibe symlink alias, the skill's own parent IS the skills dir. Only
# the source repo's canonical flow/ path is different (SKILL_DIR is flow/, whose
# parent is the repo root) — detected because its parent does not contain vibe/ —
# and there the .spec/.git marker is always present, so fall back to a marker walk.
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
MACHINE="$SKILL_DIR/state-machine.json"
STATE="$SKILL_DIR/state.json"
MACHINE="$SKILL_DIR/state-machine.json"
STATE="$SKILL_DIR/state.json"

GENERIC_FALLBACK="state=unknown · read .agents/skills/vibe/state-machine.json and pick the matching vibe phase · transition via set-state.sh"

have_jq() { command -v jq >/dev/null 2>&1; }

# Resolve the compound state key from the cursor (matches detect-context.sh).
current_state() {
  if have_jq && [[ -f "$STATE" ]] && jq -e . "$STATE" >/dev/null 2>&1; then
    local flow phase
    flow=$(jq -r '.flow // "idle"' "$STATE")
    phase=$(jq -r '.phase // "idle"' "$STATE")
    if [[ "$flow" == "$phase" ]]; then echo "$flow"; else echo "$flow.$phase"; fi
  else
    echo "idle"
  fi
}

current_feature() {
  if have_jq && [[ -f "$STATE" ]]; then
    jq -r '.feature // empty' "$STATE" 2>/dev/null || true
  fi
}

# Interpolate <feature> when the cursor carries one; otherwise leave the literal
# placeholder (still valid guidance, still byte-stable within a feature session).
interpolate() {
  local text="$1" feature="$2"
  if [[ -n "$feature" ]]; then
    printf '%s\n' "${text//<feature>/$feature}"
  else
    printf '%s\n' "$text"
  fi
}

# Extract the orders block for <state> from a skill's SKILL.md (between the
# `<!-- vibe:orders:<state> -->` and `<!-- /vibe:orders -->` markers).
extract_block() {
  local file="$1" state="$2"
  [[ -f "$file" ]] || return 1
  awk -v want="<!-- vibe:orders:${state} -->" '
    $0 == want { grab = 1; next }
    grab && $0 == "<!-- /vibe:orders -->" { exit }
    grab { print }
  ' "$file"
}

machine_inject() {
  local state="$1"
  have_jq || return 1
  [[ -f "$MACHINE" ]] || return 1
  jq -r --arg s "$state" '.states[$s].inject // empty' "$MACHINE" 2>/dev/null
}

machine_skill() {
  local state="$1"
  have_jq || return 1
  [[ -f "$MACHINE" ]] || return 1
  jq -r --arg s "$state" '.states[$s].skill // empty' "$MACHINE" 2>/dev/null
}

STATE_KEY="${1:-$(current_state)}"
FEATURE="$(current_feature)"

# 1. Prefer the linked skill's orders block (D12).
SKILL="$(machine_skill "$STATE_KEY" || true)"
if [[ -n "$SKILL" ]]; then
  BLOCK="$(extract_block "$SKILLS_DIR/$SKILL/SKILL.md" "$STATE_KEY" || true)"
  if [[ -n "$BLOCK" ]]; then
    interpolate "$BLOCK" "$FEATURE"
    exit 0
  fi
fi

# 2. Fall back to the machine's inline inject (idle, or a skill with no block yet).
INLINE="$(machine_inject "$STATE_KEY" || true)"
if [[ -n "$INLINE" ]]; then
  interpolate "$INLINE" "$FEATURE"
  exit 0
fi

# 3. Last resort: a generic one-liner. Never fail.
printf '%s\n' "$GENERIC_FALLBACK"
exit 0
