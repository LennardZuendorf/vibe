#!/usr/bin/env bash
# check-drift.sh — compound-enforcement drift gate for the dogfood repo.
#
# A merged feature must leave a compounded trail: a row in the root plan and no
# units still marked NOT STARTED. Nothing used to force that, so flow-mvp shipped
# (PR #14) with an all-NOT-STARTED plan, no root row, and no archive move — caught
# only weeks later by an audit. This gate makes that state fail CI.
#
# Checks:
#   (a) every directory under .spec/features/ is named somewhere in the root
#       .spec/plan.md            -> ERROR when missing (merged feature not recorded)
#   (b) any "NOT STARTED" unit in .spec/features/*/plan.md
#                                -> WARN, listed (a live feature plan that lags reality)
#   (d) a live feature folder whose root-plan row already says DONE
#                                -> ERROR (compound promoted the facts but never
#                                   archived the folder — half a compound), and
#                                   every live feature's row status is PRINTED so
#                                   a row that lags reality is visible in CI
#   (c) hand-written assertion counts in README.md or .spec/** (never the frozen
#       archive)                 -> ERROR (counts must come from the test runner,
#                                   not be typed by hand — they rot silently)
#
# Self-locates by upward marker search (.spec/.git) from its own path, so it works
# whether invoked via spec/ or the .agents/skills/spec compat alias. The archive is
# frozen history and is never scanned. Deterministic, idempotent, graceful-degrade:
# a missing .spec/ or plan warns and exits 0. Exit 1 only on ERROR.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Walk up for a .spec/.git marker rather than counting `..` hops: this script sits
# under spec/scripts/ on the canonical path and under .agents/skills/spec/scripts/
# via the compat symlink — both land on the same repo root.
find_repo_root() {
  local d="$1"
  while [[ -n "$d" && "$d" != "/" ]]; do
    if [[ -d "$d/.spec" || -e "$d/.git" ]]; then printf '%s\n' "$d"; return 0; fi
    d="$(dirname "$d")"
  done
  return 1
}
REPO_ROOT="$(find_repo_root "$SCRIPT_DIR")" || REPO_ROOT="$(pwd)"

SPEC_DIR="$REPO_ROOT/.spec"
PLAN="$SPEC_DIR/plan.md"
README="$REPO_ROOT/README.md"

errors=0
warnings=0
warn() { echo "check-drift: WARN: $1" >&2; warnings=$((warnings + 1)); }
err()  { echo "check-drift: ERROR: $1" >&2; errors=$((errors + 1)); }

if [[ ! -d "$SPEC_DIR" ]]; then
  echo "check-drift: no .spec/ under $REPO_ROOT; nothing to check." >&2
  exit 0
fi

# (a) every live feature folder must be named in the root plan.
if [[ -f "$PLAN" ]]; then
  for dir in "$SPEC_DIR"/features/*/; do
    [[ -d "$dir" ]] || continue
    feat="$(basename "$dir")"
    if ! grep -qF "$feat" "$PLAN"; then
      err "feature '$feat' has no row in .spec/plan.md — a merged feature must be compounded into the root plan"
    fi
  done
else
  warn "root plan .spec/plan.md not found; skipping plan-row check."
fi

# (d) status of each live feature's row in the root plan.
#
# Check (a) only asks that the NAME appears somewhere in plan.md, which a stale
# `NOT STARTED` row satisfies just as well as a correct one — that is how two
# fully shipped, fully tested features (content-layer, inject-triggers) passed
# this gate while misreporting themselves. A script cannot know "shipped", but it
# CAN catch the other half of the same mistake and make the first half visible:
# a DONE row with the folder still present is a compound that promoted the facts
# and forgot to archive, and printing every live row's status puts a lagging one
# in front of a human on every run.
if [[ -f "$PLAN" ]]; then
  for dir in "$SPEC_DIR"/features/*/; do
    [[ -d "$dir" ]] || continue
    feat="$(basename "$dir")"
    row="$(grep -F "| $feat |" "$PLAN" 2>/dev/null | head -n1 || true)"
    [[ -n "$row" ]] || row="$(grep -F "**$feat**" "$PLAN" 2>/dev/null | head -n1 || true)"
    if [[ -z "$row" ]]; then
      echo "check-drift: note: feature '$feat' has no Feature Sequence row in .spec/plan.md." >&2
      continue
    fi
    # Read the status FIELD, not the row text. Two ways to get this wrong, and the
    # first spelling managed both: matching the literal `| DONE |` missed
    # `| **DONE** |` or `| DONE ✅ |`, so the ERROR below could never fire (a check
    # that examines nothing); matching a bare `DONE` anywhere in the row would hit
    # the "Starts when" cell, which legitimately reads `js-core DONE`. So: split on
    # `|`, reduce each cell to its letters, and require the WHOLE cell to be the
    # status word.
    status="NOT STARTED"
    rest="$row"
    while [[ "$rest" == *"|"* ]]; do
      cell="${rest%%|*}"
      rest="${rest#*|}"
      letters="$(printf '%s' "$cell" | tr -cd 'A-Za-z')"
      case "$letters" in
        DONE) status=DONE; break ;;
        INPROGRESS) status="IN PROGRESS" ;;
      esac
    done
    echo "check-drift: live feature '$feat' — root plan says: $status" >&2
    if [[ "$status" == "DONE" ]]; then
      err "feature '$feat' is DONE in .spec/plan.md but .spec/features/$feat/ still exists — compound promotes AND archives; move it to .spec/archive/"
    fi
  done
fi

# (b) NOT STARTED units left in a live feature plan (archive is excluded — it is
# not under features/).
for dir in "$SPEC_DIR"/features/*/; do
  [[ -d "$dir" ]] || continue
  plan="${dir%/}/plan.md"
  [[ -f "$plan" ]] || continue
  while IFS= read -r hit; do
    [[ -n "$hit" ]] || continue
    warn "NOT STARTED unit in ${plan#"$REPO_ROOT"/}:$hit"
  done < <(grep -n 'NOT STARTED' "$plan" 2>/dev/null || true)
done

# (c) hand-written assertion counts anywhere in README.md or .spec/** except the
# frozen archive.
count_re='[0-9]+ (passing|passed|assertions|green)'
scan_targets=()
[[ -f "$README" ]] && scan_targets+=("$README")
while IFS= read -r f; do
  scan_targets+=("$f")
done < <(find "$SPEC_DIR" -type f -name '*.md' -not -path "$SPEC_DIR/archive/*" 2>/dev/null | sort)

if [[ ${#scan_targets[@]} -gt 0 ]]; then
  for f in ${scan_targets[@]+"${scan_targets[@]}"}; do
    while IFS= read -r hit; do
      [[ -n "$hit" ]] || continue
      err "hand-written test count in ${f#"$REPO_ROOT"/}:$hit"
    done < <(grep -nE "$count_re" "$f" 2>/dev/null || true)
  done
fi

if [[ $errors -gt 0 ]]; then
  echo "check-drift: FAIL — $errors error(s), $warnings warning(s)." >&2
  exit 1
fi
echo "check-drift: OK — 0 errors, $warnings warning(s)."
exit 0
