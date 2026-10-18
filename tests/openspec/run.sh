#!/usr/bin/env bash
# tests/openspec/run.sh — F18 gate (P3c-R17): OpenSpec's own validator
# accepts vibe-spec's export of the fixture and rejects a broken export.
# Needs node >= 20.19 and network: npx fetches the pinned CLI.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
OPENSPEC="@fission-ai/openspec@1.14.2"
# 1 capability (product-auth) + 2 changes (notes: skip_specs; passkeys).
# A run that examined fewer items is not evidence: validate prints
# "No items found" and exits 0.
EXPECTED_ITEMS=3

cargo build --locked -q -p vibe-spec --manifest-path "$REPO_ROOT/Cargo.toml"
BIN="$REPO_ROOT/target/debug/vibe-spec"
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

# validate <case>: openspec's JSON report on stdout; returns its exit code.
validate() {
  (cd "$work/$1/out" && OPENSPEC_TELEMETRY=0 npx -y "$OPENSPEC" validate --all --strict --no-interactive --json)
}

# 1. The fixture's export validates, and the run examined every item.
cp -R "$REPO_ROOT/tests/openspec/fixture" "$work/good"
(cd "$work/good" && "$BIN" export --openspec out >/dev/null)
if ! validate good >"$work/good.json"; then
  cat "$work/good.json"
  echo "openspec: the fixture's export failed validate --strict" >&2
  exit 1
fi
node "$REPO_ROOT/tests/openspec/check.mjs" count "$work/good.json" "$EXPECTED_ITEMS"

# 2. A requirement without SHALL/MUST must fail: the gate discriminates.
cp -R "$REPO_ROOT/tests/openspec/fixture" "$work/bad"
node "$REPO_ROOT/tests/openspec/check.mjs" mutate "$work/bad/.spec/product-auth.md" "The system SHALL end" "The system ends"
(cd "$work/bad" && "$BIN" export --openspec out >/dev/null)
if validate bad >"$work/bad.json"; then
  echo "openspec: a requirement without SHALL/MUST passed validate --strict" >&2
  exit 1
fi
# Non-zero alone is not evidence (a registry blip also exits non-zero):
# the report must show the one expected failure and its reason.
node "$REPO_ROOT/tests/openspec/check.mjs" fail "$work/bad.json" "$EXPECTED_ITEMS"

echo "openspec: the export validates ($EXPECTED_ITEMS items); a broken export fails"
