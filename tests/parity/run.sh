#!/usr/bin/env bash
# Spec parity: the frozen bash oracles against themselves, against the
# vibe-spec candidate over the generated corpus, and against the candidate on
# this repo's own spec tree ("live"). validate leaves live parity at F17: its
# F17 checks are off only in the corpus (P3b-R2). Needs cargo. Exit 0 only
# when all pass.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"
command -v cargo >/dev/null 2>&1 || { echo "parity: cargo not found on PATH" >&2; exit 1; }
cargo build --locked -q -p vibe-spec -p vibe-parity
BIN="${CARGO_TARGET_DIR:-$ROOT/target}/debug"
SCRATCH="$(mktemp -d)"
trap 'rm -rf "$SCRATCH"' EXIT

echo "=== self-check (oracle vs oracle) ==="
"$BIN/vibe-parity" run tests/parity/spec.json --self | grep -v '^PASS '
echo "=== candidate (oracle vs vibe-spec) ==="
"$BIN/vibe-parity" run tests/parity/spec.json --candidate "$BIN/vibe-spec" | grep -v '^PASS '

echo "=== live (this repo's spec tree) ==="
fails=0
compared=0
live() { # <vibe-spec command> <oracle script> [args...]
  local command="$1" script="$2" orc=0 crc=0
  shift 2
  env -i PATH="$PATH" HOME="$HOME" LC_ALL=C TZ=UTC \
    bash "oracles/bash/spec/scripts/$script" "$@" >"$SCRATCH/oracle" 2>/dev/null || orc=$?
  if [[ "$(uname)" == Darwin ]]; then
    # BSD wc pads validate's SF15 count (P3a-R2); the port prints it unpadded.
    sed -E 's/(SF15: [^ ]+ is) +/\1 /' "$SCRATCH/oracle" >"$SCRATCH/oracle.n"
    mv "$SCRATCH/oracle.n" "$SCRATCH/oracle"
  fi
  env -i PATH="$PATH" HOME="$HOME" LC_ALL=C TZ=UTC \
    "$BIN/vibe-spec" "$command" "$@" >"$SCRATCH/candidate" 2>/dev/null || crc=$?
  compared=$((compared + 1))
  if [[ "$orc" == "$crc" ]] && cmp -s "$SCRATCH/oracle" "$SCRATCH/candidate"; then
    echo "PASS  live $command $*"
  else
    echo "FAIL  live $command $* (exit: oracle $orc, candidate $crc)"
    diff "$SCRATCH/oracle" "$SCRATCH/candidate" | head -20 || true
    fails=$((fails + 1))
  fi
}
live list-specs list-specs.sh
live lessons-for lessons-for.sh spec
live lessons-for lessons-for.sh --format inject bash testing
live scan-merges scan-merges.sh
[[ "$compared" -ge 1 ]] || { echo "parity: live compared nothing" >&2; exit 1; }
[[ "$fails" -eq 0 ]] || { echo "parity: $fails live mismatch(es)" >&2; exit 1; }
echo "parity: self-check, candidate and live ($compared) green"
