#!/bin/sh
# bin/hook.sh <event> — flow's hook shim (D26, FV-R5): five tiers, never a download.
# $VIBE_FLOW_BIN -> $CLAUDE_PLUGIN_ROOT/bin/<os>-<arch>/vibe-flow -> `vibe-flow` on
# PATH -> no binary: the JS engine while it still ships (FV-R9 — deleted at F23),
# then bin/fallback/<event>.sh (frozen bash) -> exit 0 with one stderr line. A tier
# that crashes degrades to the frozen bash, never to nothing.
# A hook reports by exit code, so the shim decides from the child's OUTPUT: exit 2
# for a line that proves a block (`block:<reason>`, or the shipped
# `vibe-guard|vibe-gate: BLOCKED — <reason>` line), allow for everything else — a
# crash, a stack trace, a banner. An unknown event reads no stdin and exits 0.
set -u

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
elif [ -n "$fb" ]; then set -- "${BASH:-bash}" "$fb"
else
  echo "vibe: no vibe-flow binary, engine, or fallback for '$event' — flow hook not enforced" >&2
  exit 0
fi
payload=$(cat 2>/dev/null || true)
rc=0
out=$(printf '%s' "$payload" | "$@" 2>&1) || rc=$?
if [ "$rc" -ne 0 ] && [ "$rc" -ne 2 ] && [ -n "$fb" ]; then out=$(printf '%s' "$payload" | "${BASH:-bash}" "$fb" 2>&1); rc=$?; fi
if printf '%s\n' "$out" | grep -Eq '^(block:.+|vibe-(guard|gate): BLOCKED — .+)$'; then printf '%s\n' "$out" >&2; exit 2; fi
if [ "$rc" -eq 0 ] || [ "$rc" -eq 2 ]; then [ -z "$out" ] || printf '%s\n' "$out" >&2; fi
exit 0
