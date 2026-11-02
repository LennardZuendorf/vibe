#!/usr/bin/env bash
# spec/tests/plugin-validate.sh — `claude plugin validate --strict` on a
# dereferenced copy of plugin/ (the validator rejects symlinked dirs).
#
# Reach decision (2026-10-28): a stock CI runner has no `claude`, so under
# `spec/tests/run.sh` this check was a SKIP dressed as a gate. It now has two
# modes: the default stays local-friendly (a developer without Claude Code gets
# one SKIP line and exit 0), and VIBE_PLUGIN_VALIDATE=require makes a missing CLI
# a FAILURE. The CI job `plugin — claude plugin validate --strict (pinned CLI)`
# runs the require mode, so the gate can actually go red — there and nowhere
# else, because that is the only place the CLI is guaranteed.
#
# Usage: bash spec/tests/plugin-validate.sh
#        VIBE_PLUGIN_VALIDATE=require bash spec/tests/plugin-validate.sh
set -uo pipefail

# The presence check runs before ANY external command: the skip and the
# fail-closed path must work on a host (or fixture) with nothing else on PATH.
if ! command -v claude >/dev/null 2>&1; then
  if [[ "${VIBE_PLUGIN_VALIDATE:-}" == "require" ]]; then
    echo "  FAIL [plugin-validate] claude not installed — VIBE_PLUGIN_VALIDATE=require makes this a gate, so it cannot pass without running"
    exit 1
  fi
  echo "  SKIP [plugin-validate] claude not installed (not a gate here — set VIBE_PLUGIN_VALIDATE=require to fail instead of skip)"
  exit 0
fi
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd -P)"
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
