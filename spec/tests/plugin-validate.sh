#!/usr/bin/env bash
# spec/tests/plugin-validate.sh — `claude plugin validate --strict` on a
# dereferenced copy of plugin/ (the validator rejects symlinked dirs).
# Skips with one line when `claude` is not installed, so CI skips it.
#
# Usage: bash spec/tests/plugin-validate.sh
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd -P)"
if ! command -v claude >/dev/null 2>&1; then
  echo "  SKIP [plugin-validate] claude not installed"
  exit 0
fi
tmp="$(mktemp -d "${TMPDIR:-/tmp}/plugin-validate.XXXXXX")"
trap 'rm -rf "$tmp"' EXIT
cp -RL "$ROOT/plugin" "$tmp/vibe"
out="$(claude plugin validate --strict "$tmp/vibe" 2>&1)"; rc=$?
printf '%s\n' "$out"
if [[ $rc -eq 0 ]] && grep -q 'Validation passed' <<<"$out"; then
  echo "  PASS [plugin-validate] claude plugin validate --strict"
else
  echo "  FAIL [plugin-validate] claude plugin validate --strict"
  exit 1
fi
