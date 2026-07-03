#!/usr/bin/env bash
# tests/cli/parity-shell-vs-cli.sh — one-time system comparison of the retiring
# bash flow scripts vs the vibe CLI.
#
# Sweeps the write-policy guard (`detect-context.sh decide` vs `vibe check`) over
# a state x path matrix, and the D12 orders (`orders.sh` vs `vibe orders`) over
# every state, asserting the CLI is byte-identical to the bash origin. This is the
# system-level counterpart to the per-module parity unit tests — broader coverage,
# a single human-readable report.
#
# Transitional: it exists only while the bash origins and the CLI coexist, so it
# is intentionally NOT wired into tests/run.sh; it retires with the bash scripts.
# Assumes the repo cursor is idle (no carried feature) so orders resolve the bare
# <feature> placeholder on both sides. Run directly. Exit 0 = full parity.

set -uo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
SCRIPTS="$REPO_ROOT/.agents/skills/vibe/scripts"
MACHINE="$REPO_ROOT/.agents/skills/vibe/state-machine.json"
VIBE="$REPO_ROOT/cli/.venv/bin/vibe"
EMPTY="$(mktemp -d)"
trap 'rm -rf "$EMPTY"' EXIT

command -v jq >/dev/null 2>&1 || { echo "jq required"; exit 1; }
[[ -x "$VIBE" ]] || { echo "cli not built at $VIBE — run: (cd cli && uv sync)"; exit 1; }

STATES=()
while IFS= read -r s; do STATES+=("$s"); done < <(jq -r '.states | keys[]' "$MACHINE")

PATHS=(
  ".spec/lessons.md"
  ".spec/product.md"
  ".spec/tech.md"
  ".spec/design.md"
  ".spec/plan.md"
  ".agents/skills/vibe/state.json"
  "src/foo.py"
  "tests/foo_test.py"
  ".spec/features/x/product.md"
  ".spec/quick/slug.md"
  "README.md"
  "docs/note.md"
)

# strip trailing whitespace per line so rich's newline handling can't cause a
# spurious mismatch; the verdict/orders bodies must still be byte-identical.
trim() { printf '%s' "$1" | sed -e 's/[[:space:]]*$//'; }

echo "=== shell vs CLI parity eval (one-time) ==="
echo "states: ${#STATES[@]}   guard paths: ${#PATHS[@]}"
echo ""

# --- guard / decide ----------------------------------------------------------
g_total=0; g_match=0; g_mismatch=0
for st in "${STATES[@]}"; do
  for p in "${PATHS[@]}"; do
    g_total=$((g_total + 1))
    b="$(trim "$(bash "$SCRIPTS/detect-context.sh" decide "$p" "$st" 2>/dev/null)")"
    c="$(trim "$("$VIBE" check "$p" --state "$st" --root "$REPO_ROOT" 2>/dev/null)")"
    if [[ "$b" == "$c" ]]; then
      g_match=$((g_match + 1))
    else
      g_mismatch=$((g_mismatch + 1))
      echo "  MISMATCH guard [$st] $p"
      echo "    bash: $b"
      echo "    cli : $c"
    fi
  done
done
echo "guard/decide: $g_total comparisons — $g_match match, $g_mismatch mismatch"

# --- orders ------------------------------------------------------------------
o_total=0; o_match=0; o_mismatch=0
for st in "${STATES[@]}"; do
  o_total=$((o_total + 1))
  b="$(trim "$(bash "$SCRIPTS/orders.sh" "$st" 2>/dev/null)")"
  c="$(trim "$("$VIBE" orders --state "$st" --root "$EMPTY" 2>/dev/null)")"
  if [[ "$b" == "$c" ]]; then
    o_match=$((o_match + 1))
  else
    o_mismatch=$((o_mismatch + 1))
    echo "  MISMATCH orders [$st]"
    echo "    bash: $b"
    echo "    cli : $c"
  fi
done
echo "orders: $o_total comparisons — $o_match match, $o_mismatch mismatch"

# --- summary ------------------------------------------------------------------
total=$((g_total + o_total))
mism=$((g_mismatch + o_mismatch))
echo ""
echo "========================================"
echo "TOTAL: $total comparisons, $((total - mism)) match, $mism mismatch"
if [[ "$mism" -eq 0 ]]; then
  echo "=== SHELL/CLI PARITY: PASS ==="
else
  echo "=== SHELL/CLI PARITY: FAIL ==="
fi
echo "========================================"
[[ "$mism" -eq 0 ]]
