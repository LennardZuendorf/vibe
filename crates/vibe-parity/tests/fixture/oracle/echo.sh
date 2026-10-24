#!/usr/bin/env bash
# Parity fixture oracle: echoes its arguments and writes out.txt.
set -euo pipefail
printf 'args:%s\n' "$*"
printf '{"updated": "2026-01-01T00:00:00Z"}\n' >out.txt