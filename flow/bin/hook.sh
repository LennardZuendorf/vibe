#!/bin/sh
# flow/bin/hook.sh <event> — five-tier discovery; forward exit 2 only for a proved block line.
set -u
event="${1:-}"
tmp=""
trap '[ -z "$tmp" ] || rm -f "$tmp"' EXIT
run() {
  tmp="$(mktemp "${TMPDIR:-/tmp}/vibe-flow-hook.XXXXXX" 2>/dev/null || true)"
  [ -n "$tmp" ] || { echo "vibe-flow: hook $event cannot buffer output — allowing" >&2; exit 0; }
  "$@" >"$tmp" 2>&1
  rc=$?
  first=""
  IFS= read -r first <"$tmp" 2>/dev/null || true
  if [ "$rc" -eq 0 ]; then cat "$tmp" >&2; exit 0; fi
  case "$first" in
    "vibe-guard: BLOCKED"*|"vibe-gate: BLOCKED"*)
      if [ "$rc" -eq 2 ]; then cat "$tmp" >&2; exit 2; fi ;;
  esac
  echo "vibe-flow: hook $event failed (exit $rc) — allowing" >&2
  exit 0
}
bin=""
if [ -n "${VIBE_FLOW_BIN:-}" ] && [ -x "$VIBE_FLOW_BIN" ]; then bin="$VIBE_FLOW_BIN"
elif [ -n "${CLAUDE_PLUGIN_ROOT:-}" ]; then
  p="$CLAUDE_PLUGIN_ROOT/bin/$(uname -s | tr '[:upper:]' '[:lower:]')-$(uname -m | tr '[:upper:]' '[:lower:]')/vibe-flow"
  [ -x "$p" ] && bin="$p"
fi
[ -n "$bin" ] || bin="$(command -v vibe-flow 2>/dev/null || true)"
[ -n "$bin" ] && run "$bin" hook "$event"
case "$event" in
  pre-tool-use-guard|stop-gate)
    cpd="${CLAUDE_PROJECT_DIR:-}"
    for f in "$(dirname "$0")/../hooks-fallback/$event.sh" \
      "${cpd:-/nonexistent}/.agents/flow/hooks-fallback/$event.sh" \
      "${cpd:-/nonexistent}/.agents/skills/vibe/hooks-fallback/$event.sh"; do
      [ -f "$f" ] && run bash "$f"
    done ;;
esac
echo "vibe-flow: no vibe-flow binary or fallback for hook $event — allowing" >&2
exit 0
