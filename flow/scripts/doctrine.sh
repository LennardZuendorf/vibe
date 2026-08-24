#!/usr/bin/env bash
# doctrine.sh — emit the vibe working-model doctrine for the SessionStart hook.
#
#   doctrine.sh    # prints the doctrine block, and nothing else
#
# The doctrine is single-sourced from the vibe skill's SKILL.md, between the
#   <!-- vibe:doctrine -->
#   ...working model, two gates, write invariants...
#   <!-- /vibe:doctrine -->
# markers — the same single-source pattern orders.sh uses for per-turn orders, so
# the SessionStart hook stays a thin shell and the AGENTS.md managed block becomes
# an optional adapter rather than the only carrier of the doctrine.
#
# NO LIVE STATE RIDES THIS OUTPUT (inject-triggers, R4). This script used to
# append a one-line `Cursor: <state>[ (feature=<f>)].` summary. Claude Code
# REPLAYS a SessionStart hook's saved output verbatim on `--resume` instead of
# re-running the hook, so that line went stale the moment the cursor moved in a
# resumed session — and the per-turn `user-prompt.level` channel now names the
# state every turn anyway. Deleted here and in the engine port
# (flow/engine/commands/doctrine.mjs) together, so the byte-parity matrix keeps
# comparing two implementations of the same contract. With the cursor read gone
# this script no longer reads state.json, CLAUDE_PROJECT_DIR, or jq at all.
#
# Read-only. Always exits 0: a missing block / skill / SKILL.md degrades to no
# output — never a session-ending failure.

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
exit 0
