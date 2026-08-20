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

# -e is deliberately absent: this is a reporter/aggregator (see contract
# above) that MUST keep running every remaining suite after one fails, then
# exit non-zero as a summary. `errexit` would abort at the first failing
# suite and break that contract.
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

# The whole tree must run on the interpreter THIS script was started with.
# The macOS CI leg invokes `/bin/bash tests/run.sh` precisely to exercise stock
# bash 3.2 — but dispatching sub-suites through a PATH-resolved `bash` handed
# them whatever the runner's PATH points at (GitHub's macOS images also carry a
# Homebrew bash 5), so the one leg built to catch a bash-3.2 regression ran the
# three bash suites under a shell that cannot see one.
#
# Two halves, because the suites nest: (1) dispatch on $BASH explicitly, and
# (2) prepend a shim dir whose `bash` IS $BASH, so the ~200 nested
# `bash "$SOME_SCRIPT"` calls inside the suites (install.sh, the hooks, the
# flow scripts) inherit the same shell instead of re-resolving through PATH.
SELF_BASH="${BASH:-}"
if [[ -z "$SELF_BASH" || "$SELF_BASH" != /* ]]; then
  SELF_BASH="$(command -v bash 2>/dev/null || true)"
fi
if [[ -z "$SELF_BASH" || ! -x "$SELF_BASH" ]]; then
  SELF_BASH=bash
else
  BASH_SHIM="$(mktemp -d 2>/dev/null || true)"
  if [[ -n "$BASH_SHIM" && -d "$BASH_SHIM" ]] && ln -s "$SELF_BASH" "$BASH_SHIM/bash" 2>/dev/null; then
    trap 'rm -rf "$BASH_SHIM"' EXIT
    PATH="$BASH_SHIM:$PATH"
    export PATH
  else
    echo "  note: could not shim 'bash' on PATH — nested calls may use a different shell" >&2
  fi
fi

# Suites live inside their monorepo half: spec/tests, flow/tests (with
# adapters folded under flow/tests/adapters), and the JS engine suite under
# flow/engine/tests. `engine` runs under `node`, not `bash` — everything else
# does — so interpreter is its own parallel array rather than assumed.
SUITE_NAMES=(spec flow adapters engine)
SUITE_INTERPRETERS=("$SELF_BASH" "$SELF_BASH" "$SELF_BASH" node)
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
for line in ${results[@]+"${results[@]}"}; do
  echo "  $line"
done
if [[ "$overall" -eq 0 ]]; then
  echo "=== ALL SUITES PASSED ==="
else
  echo "=== SUITE FAILURES DETECTED ==="
fi
echo "========================================"

exit "$overall"
