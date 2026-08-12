#!/usr/bin/env bash
# session-start-doctrine.sh — vibe flow SessionStart hook (js-core/7).
#
# Event: SessionStart (all sources, incl. `compact` re-inject). Node-first:
# execs into the JS engine's `hook session-start-doctrine` command, which
# reproduces this hook's prior behaviour (emit the vibe doctrine block + a
# live cursor summary) via engine/commands/doctrine.mjs. `exec` replaces the
# shell so the engine's exit code and stdout propagate unchanged.
#
# Graceful degrade (R4): no `node` on PATH -> exit 0 silently, inject
# nothing. Enforcement/inject is lost, never inverted into a spurious block —
# this hook's contract was already "never break the session" and stays that
# way with Node absent.

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

exec node "$ENGINE/cli.mjs" hook session-start-doctrine
