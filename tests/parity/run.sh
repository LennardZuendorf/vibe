#!/usr/bin/env bash
# Spec + flow parity: the frozen bash oracles against themselves and against
# the vibe-spec candidate over the generated corpus, and the frozen JS flow
# oracles against themselves and against the vibe-flow candidate over the
# generated flow corpus. The spec "live" leg (the candidate against the
# oracles on this repo's own spec tree) is retired (P4-R17): the oracle bash
# scripts hard-code the legacy root and cannot read docs/spec. vibe-spec checks
# this repo's tree itself (tests/run.sh, CI). The flow legs spawn node
# (oracles/js). Needs cargo. Exit 0 only when all pass.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"
command -v cargo >/dev/null 2>&1 || { echo "parity: cargo not found on PATH" >&2; exit 1; }
# The flow oracles read the cursor with jq (see detect-context.sh / stop-gate.sh
# and their documented no-jq branches), so without it the ORACLE's own answer
# changes and the run reports stale pins — a missing tool dressed up as a real
# divergence. Refuse up front instead.
command -v jq >/dev/null 2>&1 || { echo "parity: jq not found on PATH" >&2; exit 1; }
cargo build --locked -q -p vibe-spec -p vibe-parity -p vibe-flow
BIN="${CARGO_TARGET_DIR:-$ROOT/target}/debug"
flow_scratch="$(mktemp -d)"
trap 'rm -rf "$flow_scratch"' EXIT
"$BIN/vibe-parity" gen-flow-corpus "$ROOT/flow" "$flow_scratch/corpus"
node - "$ROOT/tests/parity/flow.json" "$flow_scratch" "$ROOT/oracles" <<'NODE'
const fs = require('fs');
const [suitePath, scratch, oracles] = process.argv.slice(2);
const suite = JSON.parse(fs.readFileSync(suitePath, 'utf8'));
suite.corpus = `${scratch}/corpus`;
suite.oracle_root = oracles;
fs.writeFileSync(`${scratch}/flow.json`, JSON.stringify(suite));
NODE

echo "=== spec self-check (oracle vs oracle) ==="
"$BIN/vibe-parity" run tests/parity/spec.json --self | grep -v '^PASS '
echo "=== spec candidate (oracle vs vibe-spec) ==="
"$BIN/vibe-parity" run tests/parity/spec.json --candidate "$BIN/vibe-spec" | grep -v '^PASS '

echo "=== flow self-check (oracle vs oracle) ==="
"$BIN/vibe-parity" run "$flow_scratch/flow.json" --self | grep -v '^PASS '
echo "=== flow candidate (oracle vs vibe-flow) ==="
"$BIN/vibe-parity" run "$flow_scratch/flow.json" --candidate "$BIN/vibe-flow" | grep -v '^PASS '

echo "parity: self-check and candidate green"
