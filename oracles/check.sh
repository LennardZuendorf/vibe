#!/usr/bin/env bash
# Verifies the frozen oracles against MANIFEST.sha256. Fails on any byte
# change, missing file, unlisted file, or a population below the floor.
# Usage: bash oracles/check.sh [oracles-dir]
set -euo pipefail
FLOOR=43
dir="${1:-$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)}"
cd "$dir"
[[ -f MANIFEST.sha256 ]] || { echo "oracles: no MANIFEST.sha256 in $dir" >&2; exit 1; }
listed="$(awk '{ print $2 }' MANIFEST.sha256 | LC_ALL=C sort)"
actual="$(find bash js -type f 2>/dev/null | LC_ALL=C sort)"
if [[ "$listed" != "$actual" ]]; then
  echo "oracles: the file set differs from MANIFEST.sha256 (< listed, > on disk):" >&2
  diff <(printf '%s\n' "$listed") <(printf '%s\n' "$actual") >&2 || true
  exit 1
fi
count="$(printf '%s\n' "$listed" | grep -c .)"
if [[ "$count" -lt "$FLOOR" ]]; then
  echo "oracles: $count files listed, floor $FLOOR" >&2
  exit 1
fi
if ! out="$(shasum -a 256 -c MANIFEST.sha256 2>&1)"; then
  printf '%s\n' "$out" | grep -v ': OK$' >&2
  echo "oracles: hash mismatch — a frozen oracle changed" >&2
  exit 1
fi
echo "oracles: $count files match MANIFEST.sha256"
