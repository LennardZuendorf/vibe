#!/usr/bin/env bash
# tests/run.sh — combined test runner: spec + flow + adapters behaviour suites.
#
# This is a reporter/aggregator: it runs ALL suites even when one fails, prints a
# per-suite PASS/FAIL line plus a final summary, and exits non-zero if any suite
# failed. Pure bash; no bats.
#
# Usage: bash tests/run.sh
# Exit 0 = every suite passed; 1 = at least one suite failed.

set -uo pipefail

# Repo root by upward marker search (.spec / .git) — depth- and symlink-agnostic:
# resolves the physical path so real and symlinked invocations converge.
_find_repo_root() {
  local d; d="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"
  while [[ "$d" != "/" ]]; do
    [[ -d "$d/.spec" || -e "$d/.git" ]] && { printf '%s\n' "$d"; return 0; }
    d="$(dirname "$d")"
  done
  return 1
}
ROOT="$(_find_repo_root)" || { echo "cannot locate repo root (.spec/.git)" >&2; exit 1; }

# Suites now live inside their monorepo half: spec/tests, flow/tests
# (with adapters folded under flow/tests/adapters).
declare -A SUITE_PATHS=(
  [spec]="spec/tests/run.sh"
  [flow]="flow/tests/run.sh"
  [adapters]="flow/tests/adapters/run.sh"
)
SUITES=(spec flow adapters)

overall=0
results=()

for suite in "${SUITES[@]}"; do
  echo "########################################"
  echo "### suite: $suite"
  echo "########################################"
  if bash "$ROOT/${SUITE_PATHS[$suite]}"; then
    rc=0
  else
    rc=$?
  fi
  if [[ "$rc" -eq 0 ]]; then
    results+=("PASS  $suite")
  else
    results+=("FAIL  $suite (exit $rc)")
    overall=1
  fi
  echo ""
done

echo "========================================"
echo "=== combined test summary ==="
for line in "${results[@]}"; do
  echo "  $line"
done
if [[ "$overall" -eq 0 ]]; then
  echo "=== ALL SUITES PASSED ==="
else
  echo "=== SUITE FAILURES DETECTED ==="
fi
echo "========================================"

exit "$overall"
