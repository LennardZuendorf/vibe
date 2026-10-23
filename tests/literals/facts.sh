#!/usr/bin/env bash
# The fact set of the spec tree (P4-R18): every D<nn>, F<nn> and SF<nn> id and
# every backticked token under the spec root, one per line, sorted. The
# restructure must keep the set: before ⊆ after (Task 14 compares with comm -23).
# The root is what `vibe-spec root` names, else docs/spec, else the legacy tree.
set -euo pipefail
export LC_ALL=C
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"
SPEC=""
if command -v vibe-spec >/dev/null 2>&1; then
  SPEC="$(vibe-spec root 2>/dev/null || true)"
elif [[ -x target/debug/vibe-spec ]]; then
  SPEC="$(target/debug/vibe-spec root 2>/dev/null || true)"
fi
if [[ -z "$SPEC" ]]; then
  if [[ -d docs/spec ]]; then SPEC="docs/spec"; else SPEC=".spec"; fi # literal
fi
[[ -d "$SPEC" ]] || { echo "facts: no spec root at $SPEC" >&2; exit 1; }
{
  find "$SPEC" -type f -name '*.md' -print0 | xargs -0 grep -hoE '(^|[^A-Za-z0-9_])(D[0-9]+|F[0-9]+|SF[0-9]+)([^A-Za-z0-9_]|$)' |
    grep -oE '(SF|D|F)[0-9]+' || true
  # shellcheck disable=SC2016
  find "$SPEC" -type f -name '*.md' -print0 | xargs -0 grep -hoE '`[^`]+`' || true
} | sort -u
