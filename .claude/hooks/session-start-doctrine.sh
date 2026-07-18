#!/usr/bin/env bash
# session-start-doctrine.sh — vibe flow SessionStart hook (flow-legibility/5).
#
# Event: SessionStart (all sources, incl. `compact` re-inject). Emits the vibe
# working-model doctrine + a live cursor summary so the agent gets the flow
# contract every session — making the AGENTS.md managed block an optional adapter
# rather than the only carrier of the doctrine.
#
# Thin shell: all content lives in .agents/skills/vibe/scripts/doctrine.sh, which
# single-sources the doctrine from the vibe skill's SKILL.md. stdout on exit 0 is
# added to the session context.
#
# Graceful degrade: missing project dir / resolver -> exit 0, inject nothing,
# never break the session.

set -euo pipefail

cat >/dev/null 2>&1 || true   # consume stdin; we don't need it

ROOT="${CLAUDE_PROJECT_DIR:-$PWD}"
DOCTRINE="$ROOT/.agents/skills/vibe/scripts/doctrine.sh"

[[ -f "$DOCTRINE" ]] || exit 0

# doctrine.sh always exits 0 and self-degrades; guard anyway.
bash "$DOCTRINE" 2>/dev/null || true

exit 0
