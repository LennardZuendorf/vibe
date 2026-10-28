#!/usr/bin/env bash
# tests/skills/run.sh — lints for the shipped skills, agents and docs (skill-revision/1).
# Sources lib.sh, then every cases/*.sh in sorted order (one case file per track),
# then prints the results line. Pure bash; bash 3.2 safe.
#
# Usage: bash tests/skills/run.sh
# Exit 0 = every check passed; 1 = at least one failed.
set -uo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"
REPO_ROOT="$(cd "$HERE/../.." && pwd -P)"
export REPO_ROOT
cd "$REPO_ROOT" || exit 1

# shellcheck source=tests/skills/lib.sh
source "$HERE/lib.sh"

_count=0
for _case in "$HERE"/cases/*.sh; do
  [[ -f "$_case" ]] || continue
  _count=$((_count + 1))
  echo "--- case: $(basename "$_case")"
  # shellcheck source=/dev/null
  source "$_case"
done
assert_population "skill-revision/1" "case files loaded" "$_count" 1

echo ""
echo "=== results: $PASS passed, $FAIL failed ==="
[[ "$FAIL" -eq 0 ]]
