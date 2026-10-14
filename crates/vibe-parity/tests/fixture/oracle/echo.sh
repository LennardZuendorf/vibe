#!/usr/bin/env bash
# Parity fixture oracle: echoes its arguments and writes out.txt.
set -euo pipefail
printf 'args:%s\n' "$*"
printf 'x\n' >out.txt
