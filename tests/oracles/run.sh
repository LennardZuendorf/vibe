#!/usr/bin/env bash
# Discriminating tests for oracles/check.sh: the real tree and a clean copy
# pass; a changed byte, a missing file, an unlisted file, a missing manifest
# and a population below the floor each fail. The manifest itself is pinned
# by hash here, outside oracles/, so a changed oracle with a regenerated
# manifest still fails.
set -uo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
CHECK="$ROOT/oracles/check.sh"
# The hash of oracles/MANIFEST.sha256. D30: oracles never change. Changing
# this constant is an explicit, reviewed act, never a side effect of a fix.
MANIFEST_SHA256=4e123ebb72ffe28ded21a1f439ae454a60fab8ed9e15625b04331290e9922aaf
pass=0
fail=0

expect() { # name want-rc [dir]
  local name="$1" want="$2" rc=0 output
  shift 2
  output="$(bash "$CHECK" "$@" 2>&1)" || rc=$?
  if [[ "$rc" == "$want" ]]; then
    pass=$((pass + 1)); echo "  PASS [oracles] $name"
  else
    fail=$((fail + 1)); echo "  FAIL [oracles] $name (rc=$rc, want $want)"
    [[ -z "$output" ]] || printf '%s\n' "$output"
  fi
}

pin_ok() { # manifest-path; true when its hash equals MANIFEST_SHA256
  local got
  got="$(shasum -a 256 "$1" 2>/dev/null | awk '{ print $1 }')"
  [[ -n "$got" && "$got" == "$MANIFEST_SHA256" ]]
}

record() { # name ok(0|1) [detail]
  if [[ "$2" == 0 ]]; then
    pass=$((pass + 1)); echo "  PASS [oracles] $1"
  else
    fail=$((fail + 1)); echo "  FAIL [oracles] $1${3:+ ($3)}"
  fi
}

copy() { # prints a fresh copy of oracles/
  local d
  d="$(mktemp -d)"
  cp -R "$ROOT/oracles/." "$d/"
  printf '%s\n' "$d"
}

echo "=== oracles: manifest check ==="
expect "the real tree passes" 0

d="$(copy)"; expect "a clean copy passes" 0 "$d"; rm -rf "$d"

d="$(copy)"; printf ' ' >>"$d/bash/spec/scripts/validate.sh"
expect "one appended byte fails" 1 "$d"; rm -rf "$d"

d="$(copy)"; sed 's/exit 1/exit 9/' "$d/bash/spec/scripts/promote.sh" >"$d/p" && mv "$d/p" "$d/bash/spec/scripts/promote.sh"
expect "a same-path content change fails" 1 "$d"; rm -rf "$d"

d="$(copy)"; rm "$d/js/cli.mjs"
expect "a missing file fails" 1 "$d"; rm -rf "$d"

d="$(copy)"; : >"$d/bash/extra.sh"
expect "an unlisted file fails" 1 "$d"; rm -rf "$d"

d="$(copy)"; rm "$d/MANIFEST.sha256"
expect "a missing manifest fails" 1 "$d"; rm -rf "$d"

d="$(copy)"; rm "$d/js/json.mjs"; grep -v ' js/json.mjs$' "$d/MANIFEST.sha256" >"$d/m" && mv "$d/m" "$d/MANIFEST.sha256"
expect "a population below the floor fails" 1 "$d"; rm -rf "$d"

rc=0; pin_ok "$ROOT/oracles/MANIFEST.sha256" || rc=1
record "the manifest matches its pinned hash" "$rc"

# The hole the pin closes: change an oracle and regenerate the manifest, and
# check.sh alone passes. The pin must still fail on that manifest.
d="$(copy)"; printf ' ' >>"$d/bash/spec/scripts/validate.sh"
(cd "$d" && find bash js -type f | LC_ALL=C sort | xargs shasum -a 256 >MANIFEST.sha256)
crc=0; bash "$CHECK" "$d" >/dev/null 2>&1 || crc=$?
got="$(shasum -a 256 "$d/MANIFEST.sha256" 2>/dev/null | awk '{ print $1 }')"
if [[ "$crc" != 0 ]]; then
  record "a regenerated manifest over a changed oracle fails the pin" 1 "check.sh rc=$crc, want 0 on the regenerated copy"
elif [[ -z "$got" ]]; then
  record "a regenerated manifest over a changed oracle fails the pin" 1 "no hash for the copy's manifest"
elif pin_ok "$d/MANIFEST.sha256"; then
  record "a regenerated manifest over a changed oracle fails the pin" 1 "the pin accepted $got"
else
  record "a regenerated manifest over a changed oracle fails the pin" 0
fi
rm -rf "$d"

# --- freshness: integrity cannot see a freeze that has fallen behind the
# sources it mirrors (a generation-1 freeze is self-consistent), so check.sh
# also compares every oracle with its live source. A fixture root — oracles/
# beside the sources — is what makes that leg testable in isolation.
fixture() { # prints a fresh root: oracles/ beside the live sources they mirror
  local r
  r="$(mktemp -d)"
  mkdir -p "$r/oracles" "$r/flow" "$r/spec" "$r/.claude"
  cp -R "$ROOT/oracles/." "$r/oracles/"
  cp -R "$ROOT/oracles/js" "$r/flow/engine"
  cp -R "$ROOT/oracles/bash/flow/scripts" "$r/flow/scripts"
  cp -R "$ROOT/oracles/bash/flow/hooks-fallback" "$r/flow/hooks-fallback"
  cp -R "$ROOT/oracles/bash/spec/scripts" "$r/spec/scripts"
  cp -R "$ROOT/oracles/bash/spec/reference" "$r/spec/reference"
  cp -R "$ROOT/oracles/bash/.claude/hooks" "$r/.claude/hooks"
  printf '%s\n' "$r"
}

r="$(fixture)"; expect "a freeze that matches the live sources passes" 0 "$r/oracles"; rm -rf "$r"

# flow-v2/0.4: one missing live source directory is a partial checkout.
for missing in flow/engine spec/scripts .claude/hooks; do
  r="$(fixture)"; rm -rf "${r:?}/$missing"
  expect "a partial checkout missing $missing fails freshness" 1 "$r/oracles"
  rm -rf "$r"
done

r="$(fixture)"; printf ' ' >>"$r/flow/engine/cli.mjs"
expect "a live source that moved on since the freeze fails" 1 "$r/oracles"; rm -rf "$r"

r="$(fixture)"; rm "$r/spec/scripts/promote.sh"
expect "a missing live source fails freshness" 1 "$r/oracles"; rm -rf "$r"

r="$(fixture)"; printf '\n' >>"$r/flow/scripts/orders.sh"
out="$(bash "$CHECK" "$r/oracles" 2>&1)"; rc=$?
if [[ "$rc" == 1 && "$out" == *"stale: bash/flow/scripts/orders.sh"* && "$out" == *"1 of 43 frozen files are stale"* ]]; then
  record "a stale oracle is named, with its count" 0
else
  record "a stale oracle is named, with its count" 1 "rc=$rc, want 1; the output must keep the stale line and its count"
fi
rm -rf "$r"

r="$(fixture)"; printf '\n' >>"$r/flow/scripts/orders.sh"
bash "$r/oracles/freeze.sh" --dry-run >/dev/null 2>&1
expect "freeze.sh --dry-run writes nothing" 1 "$r/oracles"
bash "$r/oracles/freeze.sh" >/dev/null 2>&1
expect "freeze.sh re-freezes a drifted source and the check goes green" 0 "$r/oracles"
rm -rf "$r"

# A freeze refuses as a whole. The missing-source path used to copy whatever it
# had already visited and then abort while claiming "nothing written" — so the
# evidence has to be the filesystem *after* the failure, not the exit code.
# Drift one source, delete a different one: the drifted oracle must be
# byte-identical, and no copy may be announced.
r="$(fixture)"; printf '\n' >>"$r/flow/scripts/orders.sh"; rm "$r/spec/scripts/promote.sh"
drifted="$r/oracles/bash/flow/scripts/orders.sh"
before="$(shasum -a 256 "$drifted" 2>/dev/null | awk '{ print $1 }')"
out="$(bash "$r/oracles/freeze.sh" 2>&1)"; rc=$?
after="$(shasum -a 256 "$drifted" 2>/dev/null | awk '{ print $1 }')"
if [[ "$rc" == 1 && -n "$before" && "$after" == "$before" && "$out" == *"nothing written"* && "$out" != *"<-"* ]]; then
  record "freeze.sh writes nothing when a live source is missing" 0
else
  record "freeze.sh writes nothing when a live source is missing" 1 \
    "rc=$rc, want 1; drifted oracle $before -> $after; a refusal must copy nothing"
fi
rm -rf "$r"

# flow-v2/0.4: exact reviewed divergence pairs; no file-level exemptions.
accept_pair() {
  local r="$1" oracle="$2" source="$3"
  mkdir -p "$r/tests/oracles"
  printf '%s\t%s\t%s\t%s\n' "$oracle" \
    "$(shasum -a 256 "$r/oracles/$oracle" | awk '{print $1}')" \
    "$(shasum -a 256 "$r/$source" | awk '{print $1}')" \
    'Intentional reviewed control: live source differs from frozen generation.' \
    > "$r/tests/oracles/accepted-divergences.tsv"
}
r="$(fixture)"; printf '\n' >> "$r/flow/scripts/orders.sh"
expect "an unreviewed source mutation remains stale" 1 "$r/oracles"
accept_pair "$r" bash/flow/scripts/orders.sh flow/scripts/orders.sh
expect "an exact reviewed oracle/live hash pair passes" 0 "$r/oracles"
out="$(bash "$CHECK" "$r/oracles" 2>&1)"; rc=$?
if [[ "$rc" == 0 && "$out" == *"42/43 live sources are byte-identical; 1 exact reviewed divergences; 0 unreviewed stale files"* ]]; then
  record "byte-identical and accepted-divergence counts stay separate" 0
else
  record "byte-identical and accepted-divergence counts stay separate" 1 "$out"
fi

printf '\n' >> "$r/flow/engine/cli.mjs"
expect "unlisted drift still fails alongside an accepted pair" 1 "$r/oracles"
cp "$r/oracles/js/cli.mjs" "$r/flow/engine/cli.mjs"
printf ' ' >> "$r/flow/scripts/orders.sh"
expect "live bytes changed after acceptance fail the hash pair" 1 "$r/oracles"
cp "$r/oracles/bash/flow/scripts/orders.sh" "$r/flow/scripts/orders.sh"
printf '\n' >> "$r/flow/scripts/orders.sh"
for column in 2 3; do
  cp "$r/tests/oracles/accepted-divergences.tsv" "$r/pair"
  awk -F '\t' -v OFS='\t' -v c="$column" '{$c="0000000000000000000000000000000000000000000000000000000000000000"; print}' "$r/pair" > "$r/tests/oracles/accepted-divergences.tsv"
  expect "a mutated accepted hash column $column fails" 1 "$r/oracles"
  mv "$r/pair" "$r/tests/oracles/accepted-divergences.tsv"
done
for malformed in hash reason extra; do
  cp "$r/tests/oracles/accepted-divergences.tsv" "$r/pair"
  case "$malformed" in
    hash) awk -F '\t' -v OFS='\t' '{$2="not-hex"; print}' "$r/pair" > "$r/tests/oracles/accepted-divergences.tsv" ;;
    reason) awk -F '\t' -v OFS='\t' '{$4=""; print}' "$r/pair" > "$r/tests/oracles/accepted-divergences.tsv" ;;
    extra) sed $'s/$/\tunexpected/' "$r/pair" > "$r/tests/oracles/accepted-divergences.tsv" ;;
  esac
  expect "malformed accepted $malformed fails" 1 "$r/oracles"
  mv "$r/pair" "$r/tests/oracles/accepted-divergences.tsv"
done
cp "$r/tests/oracles/accepted-divergences.tsv" "$r/pair"
cat "$r/pair" >> "$r/tests/oracles/accepted-divergences.tsv"
expect "duplicate accepted paths fail" 1 "$r/oracles"
mv "$r/pair" "$r/tests/oracles/accepted-divergences.tsv"
printf 'not-a-valid-row\n' > "$r/tests/oracles/accepted-divergences.tsv"
expect "malformed accepted rows fail" 1 "$r/oracles"
accept_pair "$r" bash/flow/scripts/orders.sh flow/scripts/orders.sh
sed 's@bash/flow/scripts/orders.sh@bash/not-in-manifest.sh@' "$r/tests/oracles/accepted-divergences.tsv" > "$r/pair"
mv "$r/pair" "$r/tests/oracles/accepted-divergences.tsv"
expect "accepted paths outside the frozen manifest fail" 1 "$r/oracles"
accept_pair "$r" bash/flow/scripts/orders.sh flow/scripts/orders.sh
rm "$r/flow/scripts/orders.sh"
expect "an accepted missing live source fails" 1 "$r/oracles"
cp "$r/oracles/bash/flow/scripts/orders.sh" "$r/flow/scripts/orders.sh"
expect "a stale acceptance row fails even when source equals oracle" 1 "$r/oracles"
accept_pair "$r" bash/flow/scripts/orders.sh flow/scripts/orders.sh
expect "an exact equal-byte row is unnecessary and fails" 1 "$r/oracles"
rm -rf "$r"

attr="$(git -C "$ROOT" check-attr text -- oracles/bash/spec/scripts/validate.sh 2>/dev/null)"
if [[ "$attr" == *": text: unset" ]]; then
  pass=$((pass + 1)); echo "  PASS [oracles] .gitattributes disables EOL conversion under oracles/"
else
  fail=$((fail + 1)); echo "  FAIL [oracles] .gitattributes disables EOL conversion under oracles/ (got: $attr)"
fi

echo ""
echo "=== results: $pass passed, $fail failed ==="
[[ "$fail" -eq 0 ]]
