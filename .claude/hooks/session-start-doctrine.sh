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

exec node "$ENGINE/cli.mjs" hook session-start-doctrine
