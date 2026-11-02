#!/usr/bin/env bash
# vibe plugin SessionStart hook (per-user). Self-detects a repo with the flow
# installed and, when found, emits the working-model doctrine (no live state rides
# it). Silent (exit 0) in every other repo, a spec-only docs/spec repo included, so
# a per-user install never adds noise elsewhere; and silent in a repo whose
# settings wire its own session-start-doctrine hook and whose engine can run it,
# so the doctrine is emitted once, not twice or never.
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
# A repo-local install (install.sh) already emits the doctrine via its own hook:
# defer to it only when settings.json/settings.local.json wire it AND it can
# answer. A copied but unwired script (no jq, invalid settings.json) never fires,
# and the wired one runs the Node engine with no bash fallback, so without node,
# without the engine, with a node too old for it or with a broken engine it
# prints nothing; none of those is a signal to stay silent. "Can answer" is
# probed, not inferred: the same engine command the repo hook runs must print.
ENGINE="${VIBE_ENGINE:-$ROOT/.agents/skills/vibe/engine}"
if command -v node >/dev/null 2>&1 && [[ -f "$ENGINE/cli.mjs" ]]; then
  for f in "$ROOT/.claude/settings.json" "$ROOT/.claude/settings.local.json"; do
    if [[ -f "$f" ]] && grep -qF 'session-start-doctrine.sh' "$f" 2>/dev/null; then
      probe="$(CLAUDE_PROJECT_DIR="$ROOT" node "$ENGINE/cli.mjs" hook session-start-doctrine </dev/null 2>/dev/null || true)"
      if [[ -n "$probe" ]]; then exit 0; fi
      break
    fi
  done
fi
# flow installed = the project carries the flow's state machine. A docs/spec tree
# alone is a spec repo, not a flow repo: the doctrine is flow doctrine.
[[ -f "$ROOT/.agents/skills/vibe/state-machine.json" ]] || exit 0

DOCTRINE="${CLAUDE_PLUGIN_ROOT:-}/skills/vibe/scripts/doctrine.sh"
[[ -f "$DOCTRINE" ]] || exit 0
# doctrine.sh reads no project state, self-degrades, exit 0.
bash "$DOCTRINE" 2>/dev/null || true
exit 0
