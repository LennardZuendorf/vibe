#!/usr/bin/env bash
# user-prompt-submit-inject.sh — vibe flow inject hook (platform-adapters/1).
#
# Event: UserPromptSubmit. The daily driver — every turn it injects the current
# flow state's "orders" so the human is no longer the inject mechanism (D12).
#
# Thin shell: all logic lives in .agents/skills/vibe/scripts/orders.sh, which resolves
# the cursor state, follows its `skill` link, and emits that skill's per-state
# orders block (idle falls back to the machine's inline string). stdout on exit 0
# is added to the prompt context. Static-content discipline: orders.sh emits a
# byte-stable block (only <feature> interpolates) so the prompt cache holds.
#
# Drift-first nudge (flow-legibility/6): before the orders, this hook asks
# detect-context.sh whether working-tree activity contradicts the cursor and, only
# then, prepends a single `vibe-drift:` correction line — so the byte-stable orders
# block is preserved on every no-drift turn.
#
# Warnings relay (the model-visible end): the guard and gate hooks queue warn-only
# smells into .agents/skills/vibe/warnings.log (their stderr never reaches the
# model). After the orders, this hook drains that log to stdout — prefixing each
# line "vibe-warn:" — and truncates it, so every queued warn surfaces exactly once.
#
# Graceful degrade (R9): missing project dir / script -> exit 0, inject nothing,
# never break the session. An unreadable/unwritable relay log is a silent no-op.

set -euo pipefail

cat >/dev/null 2>&1 || true   # consume stdin; we don't need it

ROOT="${CLAUDE_PROJECT_DIR:-$PWD}"
ORDERS="$ROOT/.agents/skills/vibe/scripts/orders.sh"
DETECT="$ROOT/.agents/skills/vibe/scripts/detect-context.sh"
WARN_LOG="$ROOT/.agents/skills/vibe/warnings.log"

[[ -f "$ORDERS" ]] || exit 0

# Drift-first: prepend a single correction line ONLY when detect-context.sh infers
# that working-tree activity contradicts the cursor. No drift -> nothing prepended,
# so the orders block below stays the byte-stable first content.
if [[ -f "$DETECT" ]]; then
  drift="$(bash "$DETECT" infer 2>/dev/null || true)"
  [[ -n "$drift" ]] && printf 'vibe-drift: %s\n' "${drift#drift:*:}"
fi

# orders.sh always exits 0 and self-degrades; guard anyway.
bash "$ORDERS" 2>/dev/null || true

# Drain the warnings relay to stdout (the injected stream), then truncate so each
# warn shows once. Non-empty lines only; failures never end the session.
if [[ -f "$WARN_LOG" ]]; then
  while IFS= read -r line || [[ -n "$line" ]]; do
    [[ -n "$line" ]] || continue
    printf 'vibe-warn: %s\n' "$line"
  done < "$WARN_LOG"
  : > "$WARN_LOG" 2>/dev/null || rm -f "$WARN_LOG" 2>/dev/null || true
fi

exit 0
