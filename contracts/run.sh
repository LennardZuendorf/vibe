#!/usr/bin/env bash
# Validates contracts/: the schema validator's own tests, the discriminating
# run tests, then the contracts themselves. Needs Node >= 18 and no tool crate.
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
command -v node >/dev/null 2>&1 || { echo "contracts: node not found on PATH" >&2; exit 2; }
node "$HERE/tests/schema.test.mjs"
node "$HERE/tests/run.test.mjs"
node "$HERE/run.mjs"
