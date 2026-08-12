#!/usr/bin/env bash
# pre-tool-use-guard.sh — vibe flow guard hook (js-core/7).
#
# Event: PreToolUse, matcher Edit|Write|NotebookEdit|Bash. Node-first: execs
# into the JS engine's `hook pre-tool-use-guard` command, which reproduces
# this hook's prior behaviour (Bash write-shaped sniffer, warn-only; file
# tools routed through bash detect-context.sh decide, verdict translated to
# block/warn/allow) via engine/commands/hook.mjs. `exec` replaces the shell
# so the engine's exit code (notably the block path's exit 2) propagates
# unchanged.
#
# STILL DELEGATES POLICY TO BASH: detect-context.sh is NOT ported in this
# feature (its policy becomes policy.json in content-layer; porting it twice
# is waste) — the engine's guard handler shells out to the same
# .agents/skills/vibe/scripts/detect-context.sh this hook always called for
# its `decide` verdict. Only the orchestration around that call (stdin
# parsing, the Bash sniffer, warnings relay, exit-code translation) moved to
# JS.
#
# Graceful degrade (R4): no `node` on PATH -> exit 0 silently, NEVER exit 2.
# Enforcement is lost, never inverted into a spurious block.

set -euo pipefail

command -v node >/dev/null 2>&1 || exit 0

ROOT="${CLAUDE_PROJECT_DIR:-$PWD}"
ENGINE="${VIBE_ENGINE:-$ROOT/.agents/skills/vibe/engine}"

exec node "$ENGINE/cli.mjs" hook pre-tool-use-guard
