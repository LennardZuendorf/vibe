#!/usr/bin/env bash
# spec/hooks/run-hook.sh <event> — fail-open shim for `vibe-spec hook <event>`.
# Binary lookup: $VIBE_SPEC_BIN, vibe-spec on PATH, then the newer (by mtime) of
# target/release and target/debug under $CLAUDE_PROJECT_DIR. Only `guard` may
# exit 2, and only for one stderr line `blocked: ...`; every other exit code,
# event or failure allows the action (exit 0), with one stderr line.
set -uo pipefail

event="${1:-}"
bin=""
if [[ -n "${VIBE_SPEC_BIN:-}" ]]; then
  bin="$VIBE_SPEC_BIN"
elif command -v vibe-spec >/dev/null 2>&1; then
  bin="$(command -v vibe-spec)"
elif [[ -n "${CLAUDE_PROJECT_DIR:-}" ]]; then
  rel="$CLAUDE_PROJECT_DIR/target/release/vibe-spec"
  dbg="$CLAUDE_PROJECT_DIR/target/debug/vibe-spec"
  if [[ -x "$rel" && -x "$dbg" ]]; then
    if [[ "$dbg" -nt "$rel" ]]; then bin="$dbg"; else bin="$rel"; fi
  elif [[ -x "$rel" ]]; then
    bin="$rel"
  elif [[ -x "$dbg" ]]; then
    bin="$dbg"
  fi
fi

if [[ -z "$bin" ]]; then
  echo "vibe-spec: binary missing — spec hooks skipped (build with: cargo build -p vibe-spec)" >&2
  exit 0
fi

errf="$(mktemp "${TMPDIR:-/tmp}/vibe-spec-hook.XXXXXX" 2>/dev/null)" || errf=""
if [[ -n "$errf" ]]; then
  "$bin" hook "$event" 2>"$errf"
else
  "$bin" hook "$event"
fi
rc=$?
err=""
if [[ -n "$errf" ]]; then
  err="$(cat "$errf" 2>/dev/null)"
  rm -f "$errf"
fi

if [[ "$rc" -eq 0 ]]; then
  [[ -n "$err" ]] && printf '%s\n' "$err" >&2
  exit 0
fi
if [[ "$event" == "guard" && "$rc" -eq 2 && "$err" == blocked:\ * && "$err" != *$'\n'* ]]; then
  printf '%s\n' "$err" >&2
  exit 2
fi
echo "vibe-spec: hook $event failed (exit $rc) — allowing" >&2
exit 0
