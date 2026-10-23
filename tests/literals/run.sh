#!/usr/bin/env bash
# The zero-literal sweep (P4-R16a). The logic lives in Rust, so the sweep and
# `vibe-spec migrate` share one regex source (literal::find):
# crates/vibe-spec/tests/literal_sweep.rs. The path exemptions, each with its
# reason, are listed there. LITERALS_REPORT_ONLY=1 prints the findings and passes.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"
command -v cargo >/dev/null 2>&1 || { echo "literals: cargo not found on PATH" >&2; exit 1; }
cargo test -q -p vibe-spec --test literal_sweep -- --ignored --nocapture
