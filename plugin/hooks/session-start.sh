#!/usr/bin/env bash
# vibe plugin SessionStart hook (per-user). Self-detects a vibe-enabled repo and,
# when found, emits the working-model doctrine. Silent
# (exit 0) in non-vibe repos so a per-user install never adds noise elsewhere.
set -euo pipefail
cat >/dev/null 2>&1 || true   # consume stdin; unused

ROOT="${CLAUDE_PROJECT_DIR:-}"
if [[ -z "$ROOT" ]]; then
  # No CLAUDE_PROJECT_DIR (a manual/non-Claude run): find the enclosing repo root by
  # upward marker search so detection works from a subdirectory; fall back to cwd.
  ROOT="$PWD"; d="$PWD"
  while [[ "$d" != "/" ]]; do
    if [[ -d "$d/docs/spec" || -d "$d/.spec" || -e "$d/.git" ]]; then ROOT="$d"; break; fi # literal
    d="$(dirname "$d")"
  done
fi
# vibe-enabled = the project carries a docs/spec/ (or legacy spec) tree or a flow cursor.
[[ -d "$ROOT/docs/spec" || -d "$ROOT/.spec" || -f "$ROOT/.agents/skills/vibe/state.json" ]] || exit 0 # literal

DOCTRINE="${CLAUDE_PLUGIN_ROOT:-}/skills/vibe/scripts/doctrine.sh"
[[ -f "$DOCTRINE" ]] || exit 0
# doctrine.sh reads the project cursor via CLAUDE_PROJECT_DIR, self-degrades, exit 0.
bash "$DOCTRINE" 2>/dev/null || true
exit 0
