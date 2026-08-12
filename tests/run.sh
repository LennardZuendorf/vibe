#!/usr/bin/env bash
# tests/run.sh — combined test runner: spec + flow + adapters + engine suites.
#
# This is a reporter/aggregator: it runs ALL suites even when one fails, prints a
# per-suite PASS/FAIL line plus a final summary, and exits non-zero if any suite
# failed. Pure bash; no bats.
#
# Usage: bash tests/run.sh
# Exit 0 = every suite passed; 1 = at least one suite failed.
#
# Bash-3.2 compatible (js-core/8): stock macOS ships bash 3.2, which has no
# `declare -A` — two parallel INDEXED arrays are used instead, kept in
# lockstep by position (SUITE_INTERPRETERS[i]/SUITE_SCRIPTS[i]/SUITE_NAMES[i]
# all describe the SAME suite). Do not reorder one without the other two.

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

# Suites live inside their monorepo half: spec/tests, flow/tests (with
# adapters folded under flow/tests/adapters), and the JS engine suite under
# flow/engine/tests. `engine` runs under `node`, not `bash` — everything else
# does — so interpreter is its own parallel array rather than assumed.
SUITE_NAMES=(spec flow adapters engine)
SUITE_INTERPRETERS=(bash bash bash node)
SUITE_SCRIPTS=(
  "spec/tests/run.sh"
  "flow/tests/run.sh"
  "flow/tests/adapters/run.sh"
  "flow/engine/tests/run.mjs"
)

overall=0
results=()

i=0
while [[ "$i" -lt "${#SUITE_NAMES[@]}" ]]; do
  suite="${SUITE_NAMES[$i]}"
  interpreter="${SUITE_INTERPRETERS[$i]}"
  script="${SUITE_SCRIPTS[$i]}"

  echo "########################################"
  echo "### suite: $suite"
  echo "########################################"

  if ! command -v "$interpreter" >/dev/null 2>&1; then
    echo "  '$interpreter' not found on PATH — cannot run this suite" >&2
    rc=127
  elif "$interpreter" "$ROOT/$script"; then
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

  i=$((i + 1))
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
