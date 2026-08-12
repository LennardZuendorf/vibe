#!/usr/bin/env bash
# user-prompt-submit-inject.sh — vibe flow inject hook (js-core/7).
#
# Event: UserPromptSubmit. Node-first: execs into the JS engine's
# `hook user-prompt-submit-inject` command, which reproduces this hook's
# prior behaviour (drift-first nudge via bash detect-context.sh infer, then
# the cursor state's orders via engine/commands/orders.mjs, then the
# warnings-relay drain+truncate) via engine/commands/hook.mjs. `exec`
# replaces the shell so the engine's exit code and stdout propagate
# unchanged.
#
# Graceful degrade (R4): no `node` on PATH -> exit 0 silently, inject
# nothing. Enforcement/inject is lost, never inverted into a spurious block.

set -euo pipefail

command -v node >/dev/null 2>&1 || exit 0

ROOT="${CLAUDE_PROJECT_DIR:-$PWD}"
ENGINE="${VIBE_ENGINE:-$ROOT/.agents/skills/vibe/engine}"

exec node "$ENGINE/cli.mjs" hook user-prompt-submit-inject
