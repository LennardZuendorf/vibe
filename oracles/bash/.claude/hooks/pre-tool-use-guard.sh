#!/usr/bin/env bash
# pre-tool-use-guard.sh — vibe flow guard hook (Claude Code PreToolUse adapter).
#
# Event: PreToolUse, matcher Edit|Write|NotebookEdit|Bash. This file is ONLY the
# adapter: the work happens in `flow/bin/hook.sh pre-tool-use-guard`, flow's
# five-tier discovery shim (D26, F6). That shim resolves, in order, $VIBE_FLOW_BIN,
# a bundled `bin/<os>-<arch>/vibe-flow`, a `vibe-flow` on PATH, and finally the
# frozen bash fallback at `bin/fallback/pre-tool-use-guard.sh`.
#
# Why the flip matters: this hook carries a HARD BLOCK (the three write-policy
# classes), so it must not depend on any one runtime. Routing it through the shim
# means enforcement degrades binary -> bash, never to nothing; the shim maps a
# crashed child to allow and a proved block line to exit 2, so a lost runtime can
# never manufacture a spurious block either.
#
# Path policy is not here and not in this file's caller — the decision lives once
# in the port's `policy::decide` (with `detect-context.sh decide` as its own
# fallback), reading content/policy.json.
set -euo pipefail

HOOKS_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"
ROOT="${CLAUDE_PROJECT_DIR:-$PWD}"

# Two layouts, matching install.sh: the installed skill at
# `.agents/skills/vibe/bin/hook.sh`, then this repo's own `flow/bin/hook.sh`.
SHIM="${VIBE_FLOW_SHIM:-$ROOT/.agents/skills/vibe/bin/hook.sh}"
[[ -f "$SHIM" ]] || SHIM="$HOOKS_DIR/../../flow/bin/hook.sh"
if [[ ! -f "$SHIM" ]]; then
  echo "vibe-guard: flow hook shim not found ($SHIM) — the guard is NOT enforced" >&2
  exit 0
fi

# `${BASH}` — the shell already running this script — not a bare `sh`/`bash`,
# which resolves through PATH. A minimal PATH can lack both, and then the hook
# dies with `exec: bash: not found` and exit 127 instead of answering.
exec "${BASH:-bash}" "$SHIM" pre-tool-use-guard
