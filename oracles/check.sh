#!/usr/bin/env bash
# Verifies the frozen oracles against MANIFEST.sha256, and — inside a checkout
# of the sources they were frozen from — against those live sources. Fails on
# any byte change, missing file, unlisted file, a population below the floor,
# or a live source that has moved on since the freeze (staleness).
# Usage: bash oracles/check.sh [oracles-dir]
set -euo pipefail
FLOOR=43
dir="$(cd "${1:-$(dirname "${BASH_SOURCE[0]}")}" && pwd)"
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

# Freshness. Integrity alone cannot tell a current freeze from a generation-1
# one: a stale oracle is self-consistent, so this check used to pass while the
# whole freeze lagged the sources. The mapping is a rule, not a second list, so
# it cannot drift from the manifest: `bash/<path>` is `<path>` at the repo
# root, `js/<path>` is `flow/engine/<path>`.
root="$(cd "$dir/.." && pwd)"
if [[ ! -e "$root/flow" && ! -L "$root/flow" && ! -e "$root/spec" && ! -L "$root/spec" && ! -e "$root/.claude" && ! -L "$root/.claude" && ! -e "$root/.git" && ! -L "$root/.git" && ! -e "$root/Cargo.toml" && ! -L "$root/Cargo.toml" ]]; then
  echo "oracles: freshness not checked — ${root} is not a checkout of the sources"
  exit 0
fi
# Exact pairs preserve the frozen generation while pinning reviewed live repairs.
# Validate every row before comparing bytes, including rows that ceased to diverge.
acceptance="$root/tests/oracles/accepted-divergences.tsv"
accepted_paths=""
source_path() {
  case "$1" in
    js/*) printf '%s/flow/engine/%s\n' "$root" "${1#js/}" ;;
    bash/*) printf '%s/%s\n' "$root" "${1#bash/}" ;;
    *) return 1 ;;
  esac
}
refuse_pair() { echo "oracles: invalid accepted divergence — $1" >&2; exit 1; }
if [[ -e "$acceptance" || -L "$acceptance" ]]; then
  [[ -f "$acceptance" ]] || refuse_pair "not a regular table: $acceptance"
  while IFS= read -r row || [[ -n "$row" ]]; do
    [[ -n "$row" && "$row" != \#* ]] || continue
    [[ "$row" == *$'\t'* ]] || refuse_pair "malformed row: $row"
    pair_path="${row%%$'\t'*}"; fields="${row#*$'\t'}"
    [[ "$fields" == *$'\t'* ]] || refuse_pair "malformed row: $pair_path"
    frozen_hash="${fields%%$'\t'*}"; fields="${fields#*$'\t'}"
    [[ "$fields" == *$'\t'* ]] || refuse_pair "malformed row: $pair_path"
    live_hash="${fields%%$'\t'*}"; reason="${fields#*$'\t'}"
    [[ "$reason" != *$'\t'* && "$reason" == *[[:alnum:]]* && "${#reason}" -ge 16 ]] || refuse_pair "missing reason: $pair_path"
    [[ "$frozen_hash" =~ ^[0-9a-f]{64}$ && "$live_hash" =~ ^[0-9a-f]{64}$ ]] || refuse_pair "malformed hashes: $pair_path"
    printf '%s\n' "$listed" | grep -Fxq -- "$pair_path" || refuse_pair "path outside manifest: $pair_path"
    if printf '%s\n' "$accepted_paths" | grep -Fxq -- "$pair_path"; then refuse_pair "duplicate path: $pair_path"; fi
    pair_source="$(source_path "$pair_path")" || refuse_pair "no source rule: $pair_path"
    [[ -f "$pair_source" ]] || refuse_pair "live source missing: $pair_path"
    actual_frozen="$(shasum -a 256 "$pair_path" | awk '{print $1}')"
    actual_live="$(shasum -a 256 "$pair_source" | awk '{print $1}')"
    [[ "$frozen_hash" == "$actual_frozen" && "$live_hash" == "$actual_live" ]] || refuse_pair "hash pair drifted: $pair_path"
    [[ "$frozen_hash" != "$live_hash" ]] || refuse_pair "row no longer diverges: $pair_path"
    accepted_paths="${accepted_paths}${pair_path}"$'\n'
  done < "$acceptance"
fi
accepted=0
stale=0
while IFS= read -r path; do
  [[ -n "$path" ]] || continue
  case "$path" in
    js/*) src="$root/flow/engine/${path#js/}" ;;
    bash/*) src="$root/${path#bash/}" ;;
    *)
      echo "stale: $path — path has no live-source rule (expected a bash/ or js/ prefix)" >&2
      stale=$((stale + 1))
      continue
      ;;
  esac
  if [[ ! -f "$src" ]]; then
    echo "stale: $path — the live source ${src#"$root"/} is missing" >&2
    stale=$((stale + 1))
  elif ! cmp -s "$path" "$src"; then
    if printf '%s\n' "$accepted_paths" | grep -Fxq -- "$path"; then
      accepted=$((accepted + 1))
      continue
    fi
    echo "stale: $path — differs from the live source ${src#"$root"/}" >&2
    stale=$((stale + 1))
  fi
done <<<"$listed"
if [[ "$stale" -ne 0 ]]; then
  echo "oracles: $stale of $count frozen files are stale against the live sources" >&2
  echo "oracles: re-freeze them (see oracles/README.md) — never port against a stale freeze" >&2
  exit 1
fi
echo "oracles: $((count - accepted))/$count live sources are byte-identical; $accepted exact reviewed divergences; 0 unreviewed stale files"
