#!/usr/bin/env bash
# Re-freezes every oracle from the live source it mirrors, then regenerates
# MANIFEST.sha256 and proves the result. A freeze is all-or-nothing: pass 1
# resolves every listed source and proves it exists, pass 2 copies. Nothing is
# written until every source is known to be there, so a refusal leaves the
# oracles byte-identical — which is what "nothing written" claims.
# Usage: bash oracles/freeze.sh [--dry-run]
set -euo pipefail
dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root="$(cd "$dir/.." && pwd)"
[[ -f "$dir/MANIFEST.sha256" ]] || { echo "freeze: no MANIFEST.sha256 in $dir" >&2; exit 1; }
if [[ ! -d "$root/flow/engine" || ! -d "$root/spec/scripts" ]]; then
  echo "freeze: ${root} is not a checkout of the sources" >&2
  exit 1
fi
dry=0
if [[ "${1:-}" == "--dry-run" ]]; then
  dry=1
elif [[ $# -gt 0 ]]; then
  echo "freeze: unknown argument $1 (usage: bash oracles/freeze.sh [--dry-run])" >&2
  exit 1
fi
# Same rule as check.sh: `bash/<path>` is `<path>` at the repo root,
# `js/<path>` is `flow/engine/<path>`. One rule, shared by both passes.
source_of() { # <oracle-path>; prints its live source, or fails
  case "$1" in
    js/*) printf '%s\n' "$root/flow/engine/${1#js/}" ;;
    bash/*) printf '%s\n' "$root/${1#bash/}" ;;
    *) return 1 ;;
  esac
}
listed="$(awk '{ print $2 }' "$dir/MANIFEST.sha256")"

# Pass 1 — resolve and prove. No copying happens here at all, so each refusal
# below leaves the freeze exactly as it was.
missing=0
while IFS= read -r path; do
  [[ -n "$path" ]] || continue
  if ! src="$(source_of "$path")"; then
    echo "freeze: $path has no live-source rule (expected a bash/ or js/ prefix)" >&2
    missing=$((missing + 1))
    continue
  fi
  if [[ ! -f "$src" ]]; then
    echo "freeze: the live source ${src#"$root"/} is missing — $path cannot be frozen from it" >&2
    missing=$((missing + 1))
  fi
done <<<"$listed"
if [[ "$missing" -ne 0 ]]; then
  echo "freeze: $missing oracles have no live source; nothing written" >&2
  exit 1
fi

# Pass 2 — copy. Pass 1 proved every source in this list exists, so this loop
# cannot stop half-way over a missing input.
copied=0
while IFS= read -r path; do
  [[ -n "$path" ]] || continue
  src="$(source_of "$path")"
  if cmp -s "$dir/$path" "$src"; then
    continue
  fi
  copied=$((copied + 1))
  if [[ "$dry" -eq 0 ]]; then
    cp "$src" "$dir/$path"
  fi
  echo "freeze: ${path} <- ${src#"$root"/}"
done <<<"$listed"

if [[ "$dry" -eq 1 ]]; then
  echo "freeze: dry run — $copied oracles would be re-copied"
  exit 0
fi
(
  cd "$dir"
  find bash js -type f | LC_ALL=C sort | xargs shasum -a 256
) >"$dir/MANIFEST.sha256"
bash "$dir/check.sh"
echo "freeze: $copied oracles re-copied from $root"
echo "freeze: MANIFEST.sha256 pin for tests/oracles/run.sh: $(shasum -a 256 "$dir/MANIFEST.sha256" | awk '{ print $1 }')"
