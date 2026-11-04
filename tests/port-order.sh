#!/usr/bin/env bash
# tests/port-order.sh — the port-order gate (F6, docs/spec/tech/rust/rust-parity-pins.md).
#
# Port order is blast-radius first: the guard and the stop gate are the two hard
# blocks, so nothing downstream may exist before they are parity-green. The TUI
# and the signals crate are the first code that would be built ON that parity, so
# they are what this gate watches: if `crates/vibe-flow/src/tui/` or
# `crates/vibe-signals/` exists while `tests/parity/GREEN.hard` is absent, the
# build fails. The marker is written only when the guard/stop corpus is green.
#
# Usage: bash tests/port-order.sh [root]   (root defaults to this checkout)
# Exit 0 = ordered; 1 = code ahead of the marker.
set -euo pipefail

root="${1:-$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)}"
marker="tests/parity/GREEN.hard"

ahead=0
for path in crates/vibe-flow/src/tui crates/vibe-signals; do
  [ -e "$root/$path" ] || continue
  [ -f "$root/$marker" ] && continue
  echo "port-order: $path exists while $marker is absent — the TUI and signals cannot get ahead of the guard/stop hard blocks" >&2
  ahead=1
done

if [ "$ahead" -ne 0 ]; then
  exit 1
fi
echo "port-order: ok"
