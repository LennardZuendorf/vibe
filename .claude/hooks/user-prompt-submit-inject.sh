#!/usr/bin/env bash
# user-prompt-submit-inject.sh — vibe flow inject hook (platform-adapters/1).
#
# Event: UserPromptSubmit. The daily driver — every turn it injects the current
# flow state's "orders" so the human is no longer the inject mechanism (D12).
#
# Thin shell: all logic lives in .agents/flow/scripts/orders.sh, which resolves
# the cursor state, follows its `skill` link, and emits that skill's per-state
# orders block (idle falls back to the machine's inline string). stdout on exit 0
# is added to the prompt context. Static-content discipline: orders.sh emits a
# byte-stable block (only <feature> interpolates) so the prompt cache holds.
#
# Graceful degrade (R9): missing project dir / script -> exit 0, inject nothing,
# never break the session.

set -euo pipefail

cat >/dev/null 2>&1 || true   # consume stdin; we don't need it

ROOT="${CLAUDE_PROJECT_DIR:-$PWD}"
ORDERS="$ROOT/.agents/flow/scripts/orders.sh"

[[ -f "$ORDERS" ]] || exit 0

# orders.sh always exits 0 and self-degrades; guard anyway.
bash "$ORDERS" 2>/dev/null || true
exit 0
