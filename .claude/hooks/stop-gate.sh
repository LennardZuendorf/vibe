#!/usr/bin/env bash
# stop-gate.sh — vibe flow gate hook (js-core/7).
#
# Event: Stop. Node-first: execs into the JS engine's `hook stop-gate`
# command, which reproduces this hook's prior behaviour (re-entry guard on
# stop_hook_active; warn-only TDD nudge; the one promoted blocking tooth — a
# *.verify state needs a fresh evidence receipt; warn-only stuck-phase
# nudge) via engine/commands/hook.mjs. `exec` replaces the shell so the
# engine's exit code (notably the block path's exit 2) propagates unchanged.
#
# STATE/FEATURE/NEXT resolution is pure JS (readCursor/loadMachine — the same
# primitives orders/doctrine use), not a shell-out to detect-context.sh: it
# is jq-independent by construction, so there is no no-jq degrade branch to
# reproduce here the way detect-context.sh's own sed fallback needs one.
#
# Graceful degrade (R4): no `node` on PATH -> exit 0 silently, NEVER exit 2.
# Enforcement is lost, never inverted into a spurious block.

set -euo pipefail

command -v node >/dev/null 2>&1 || exit 0

ROOT="${CLAUDE_PROJECT_DIR:-$PWD}"
ENGINE="${VIBE_ENGINE:-$ROOT/.agents/skills/vibe/engine}"

# Graceful degrade (R4, review round 1 Finding 2): every old per-command
# resolver guarded its own script (`[[ -f "$DOCTRINE" ]] || exit 0` etc) --
# the shim must guard the engine the same way, or a target installed before
# the engine shipped (or a moved/broken symlink) gets a raw Node
# MODULE_NOT_FOUND stack trace on every prompt/tool call instead of a silent
# no-op.
[[ -f "$ENGINE/cli.mjs" ]] || exit 0

exec node "$ENGINE/cli.mjs" hook stop-gate
