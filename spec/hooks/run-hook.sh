#!/usr/bin/env bash
# spec/hooks/run-hook.sh <event> — fail-open shim for `vibe-spec hook <event>`.
# Binary lookup: $VIBE_SPEC_BIN, vibe-spec on PATH, then target/release and
# target/debug under $CLAUDE_PROJECT_DIR. Exit 0 and 2 pass through; a missing
# binary or any other exit code allows the action with one stderr line.
set -uo pipefail

event="${1:-}"
bin=""
if [[ -n "${VIBE_SPEC_BIN:-}" ]]; then
  bin="$VIBE_SPEC_BIN"
elif command -v vibe-spec >/dev/null 2>&1; then
  bin="$(command -v vibe-spec)"
elif [[ -n "${CLAUDE_PROJECT_DIR:-}" && -x "$CLAUDE_PROJECT_DIR/target/release/vibe-spec" ]]; then
  bin="$CLAUDE_PROJECT_DIR/target/release/vibe-spec"
elif [[ -n "${CLAUDE_PROJECT_DIR:-}" && -x "$CLAUDE_PROJECT_DIR/target/debug/vibe-spec" ]]; then
  bin="$CLAUDE_PROJECT_DIR/target/debug/vibe-spec"
fi

if [[ -z "$bin" ]]; then
  echo "vibe-spec: binary missing — spec hooks skipped (build with: cargo build -p vibe-spec)" >&2
  exit 0
fi

"$bin" hook "$event"
rc=$?
case "$rc" in
  0|2) exit "$rc" ;;
  *)
    echo "vibe-spec: hook $event failed (exit $rc) — allowing" >&2
    exit 0
    ;;
esac
