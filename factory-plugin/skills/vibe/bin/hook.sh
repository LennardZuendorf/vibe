#!/bin/sh
# Five-tier hook discovery; complete output proves a block, crashes allow.
set -u
case ${BASH:-} in */bash) shell=$BASH ;; *) shell=bash ;; esac
event=${1:-}
case $event in
  pre-tool-use-guard|stop-gate) ;;
  *) exit 0 ;;
esac
here=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd -P) || exit 0
bin=${VIBE_FLOW_BIN:-}
engine=${VIBE_ENGINE:-${CLAUDE_PROJECT_DIR:-$PWD}/.agents/skills/vibe/engine}
fb=$here/fallback/$event.sh
[ -f "$fb" ] || fb=
bundled=${CLAUDE_PLUGIN_ROOT:-$here/..}/bin/$(uname -s 2>/dev/null | tr '[:upper:]' '[:lower:]')-$(uname -m 2>/dev/null)/vibe-flow
if [ -n "$bin" ] && [ ! -x "$bin" ]; then bin=; fi
if [ -z "$bin" ] && [ -x "$bundled" ]; then bin=$bundled; fi
if [ -z "$bin" ]; then bin=$(command -v vibe-flow 2>/dev/null); fi
if [ -n "$bin" ]; then set -- "$bin" hook "$event"
elif [ -f "$engine/cli.mjs" ] && command -v node >/dev/null 2>&1; then set -- node "$engine/cli.mjs" hook "$event"
elif [ -n "$fb" ]; then set -- "$shell" "$fb"
else
  echo "vibe: no vibe-flow binary, engine, or fallback for '$event' — flow hook not enforced" >&2
  exit 0
fi
payload=$(cat 2>/dev/null || true)
rc=0
out=$(printf '%s' "$payload" | "$@" 2>&1) || rc=$?
if [ "$rc" -ne 0 ] && [ "$rc" -ne 2 ] && [ -n "$fb" ]; then out=$(printf '%s' "$payload" | "$shell" "$fb" 2>&1); rc=$?; fi
if { [ "$rc" -eq 0 ] || [ "$rc" -eq 2 ]; } && printf '%s\n' "$out" | {
  IFS= read -r first || exit 1
  case $first in
    block:?*) ! IFS= read -r rest ;;
    "vibe-guard: BLOCKED — "?*) if IFS= read -r rest; then [ "$rest" = "vibe-guard: transition with set-state.sh, or edit within the current state's write rules." ] && ! IFS= read -r rest; fi ;;
    "vibe-gate: BLOCKED — "?*) while IFS= read -r rest; do case $rest in "  "?*) ;; *) exit 1 ;; esac; done ;;
    *) exit 1 ;;
  esac
}; then printf '%s\n' "$out" >&2; exit 2; fi
if [ "$rc" -eq 0 ] || [ "$rc" -eq 2 ]; then [ -z "$out" ] || printf '%s\n' "$out" >&2; fi
exit 0
