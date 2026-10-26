#!/usr/bin/env bash
# Re-freezes every oracle from the live source it mirrors, then regenerates
# MANIFEST.sha256 and proves the result. A freeze is all-or-nothing: this
# copies every path the manifest lists, so a half-freeze — flow+js re-copied
# while the spec half stays generation-1 — is not expressible.
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
# `js/<path>` is `flow/engine/<path>`.
listed="$(awk '{ print $2 }' "$dir/MANIFEST.sha256")"
copied=0
missing=0
while IFS= read -r path; do
  [[ -n "$path" ]] || continue
  case "$path" in
    js/*) src="$root/flow/engine/${path#js/}" ;;
    bash/*) src="$root/${path#bash/}" ;;
    *)
      echo "freeze: $path has no live-source rule (expected a bash/ or js/ prefix)" >&2
      exit 1
      ;;
  esac
  if [[ ! -f "$src" ]]; then
    echo "freeze: the live source ${src#"$root"/} is missing — $path cannot be frozen from it" >&2
    missing=$((missing + 1))
    continue
  fi
  if cmp -s "$dir/$path" "$src"; then
    continue
  fi
  copied=$((copied + 1))
  if [[ "$dry" -eq 0 ]]; then
    cp "$src" "$dir/$path"
  fi
  echo "freeze: ${path} <- ${src#"$root"/}"
done <<<"$listed"
if [[ "$missing" -ne 0 ]]; then
  echo "freeze: $missing oracles have no live source; nothing written" >&2
  exit 1
fi
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
