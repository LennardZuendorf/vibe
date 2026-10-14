#!/usr/bin/env bash
# Discriminating tests for oracles/check.sh: the real tree and a clean copy
# pass; a changed byte, a missing file, an unlisted file, a missing manifest
# and a population below the floor each fail.
set -uo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
CHECK="$ROOT/oracles/check.sh"
pass=0
fail=0

expect() { # name want-rc [dir]
  local name="$1" want="$2" rc=0
  shift 2
  bash "$CHECK" "$@" >/dev/null 2>&1 || rc=$?
  if [[ "$rc" == "$want" ]]; then
    pass=$((pass + 1)); echo "  PASS [oracles] $name"
  else
    fail=$((fail + 1)); echo "  FAIL [oracles] $name (rc=$rc, want $want)"
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

attr="$(git -C "$ROOT" check-attr text -- oracles/bash/spec/scripts/validate.sh 2>/dev/null)"
if [[ "$attr" == *": text: unset" ]]; then
  pass=$((pass + 1)); echo "  PASS [oracles] .gitattributes disables EOL conversion under oracles/"
else
  fail=$((fail + 1)); echo "  FAIL [oracles] .gitattributes disables EOL conversion under oracles/ (got: $attr)"
fi

echo ""
echo "=== results: $pass passed, $fail failed ==="
[[ "$fail" -eq 0 ]]
