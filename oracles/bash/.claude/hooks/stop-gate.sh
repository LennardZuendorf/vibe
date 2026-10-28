#!/usr/bin/env bash
# stop-gate.sh — vibe flow gate hook (Claude Code Stop adapter).
#
# Event: Stop. This file is ONLY the adapter: the work happens in
# `flow/bin/hook.sh stop-gate`, flow's five-tier discovery shim (D26, F6). The
# shim resolves, in order, $VIBE_FLOW_BIN, a bundled `bin/<os>-<arch>/vibe-flow`,
# a `vibe-flow` on PATH, and finally the frozen bash fallback at
# `bin/fallback/stop-gate.sh`.
#
# What the event owns: the re-entry guard on stop_hook_active, the warn-only TDD
# nudge, and the one promoted blocking tooth — a `*.verify` state needs a fresh
# evidence receipt. That tooth is a HARD BLOCK the docs promise, so it must not
# depend on any one runtime: it degrades binary -> bash, never to nothing.
#
# Policy lives with the decision, not here: the port's receipt check (with the
# frozen bash fallback as the last tier).
set -euo pipefail

HOOKS_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"
ROOT="${CLAUDE_PROJECT_DIR:-$PWD}"

# Two layouts, matching install.sh: the installed skill at
# `.agents/skills/vibe/bin/hook.sh`, then this repo's own `flow/bin/hook.sh`.
SHIM="${VIBE_FLOW_SHIM:-$ROOT/.agents/skills/vibe/bin/hook.sh}"
[[ -f "$SHIM" ]] || SHIM="$HOOKS_DIR/../../flow/bin/hook.sh"
if [[ ! -f "$SHIM" ]]; then
  echo "vibe-gate: flow hook shim not found ($SHIM) — the evidence tooth is NOT enforced" >&2
  exit 0
fi

# `${BASH}` — the shell already running this script — not a bare `sh`/`bash`,
# which resolves through PATH. A minimal PATH can lack both, and then the hook
# dies with `exec: bash: not found` and exit 127 instead of answering.
exec "${BASH:-bash}" "$SHIM" stop-gate
