#!/usr/bin/env bash
# Spec parity: the frozen bash oracles against themselves and against the
# vibe-spec candidate over the generated corpus. The "live" leg (the candidate
# against the oracles on this repo's own spec tree) is retired (P4-R17): the
# oracle bash scripts hard-code the legacy root and cannot read docs/spec.
# vibe-spec checks this repo's tree itself (tests/run.sh, CI). Needs cargo.
# Exit 0 only when all pass.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"
command -v cargo >/dev/null 2>&1 || { echo "parity: cargo not found on PATH" >&2; exit 1; }
cargo build --locked -q -p vibe-spec -p vibe-parity
BIN="${CARGO_TARGET_DIR:-$ROOT/target}/debug"

echo "=== self-check (oracle vs oracle) ==="
"$BIN/vibe-parity" run tests/parity/spec.json --self | grep -v '^PASS '
echo "=== candidate (oracle vs vibe-spec) ==="
"$BIN/vibe-parity" run tests/parity/spec.json --candidate "$BIN/vibe-spec" | grep -v '^PASS '

echo "parity: self-check and candidate green"
