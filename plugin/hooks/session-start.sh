#!/usr/bin/env bash
# vibe plugin SessionStart hook (per-user). Self-detects a vibe-enabled repo and,
# when found, emits the working-model doctrine + this project's cursor. Silent
# (exit 0) in non-vibe repos so a per-user install never adds noise elsewhere.
set -euo pipefail
cat >/dev/null 2>&1 || true   # consume stdin; unused

ROOT="${CLAUDE_PROJECT_DIR:-$PWD}"
# vibe-enabled = the project carries a .spec/ tree or a flow cursor.
[[ -d "$ROOT/.spec" || -f "$ROOT/.agents/skills/vibe/state.json" ]] || exit 0

DOCTRINE="${CLAUDE_PLUGIN_ROOT:-}/skills/vibe/scripts/doctrine.sh"
[[ -f "$DOCTRINE" ]] || exit 0
# doctrine.sh reads the project cursor via CLAUDE_PROJECT_DIR, self-degrades, exit 0.
bash "$DOCTRINE" 2>/dev/null || true
exit 0
