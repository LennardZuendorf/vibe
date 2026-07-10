#!/usr/bin/env bash
# pre-tool-use-guard.sh — vibe flow guard hook (platform-adapters/2).
#
# Event: PreToolUse, matcher Edit|Write|NotebookEdit. Reads the target path from
# stdin and asks the shared decision policy whether the write is allowed in the
# current flow state. NO policy logic lives here (R8) — it all lives once in
# .agents/skills/vibe/scripts/detect-context.sh decide.
#
# Verdict translation (Claude Code PreToolUse convention):
#   block:<reason> -> reason to stderr, exit 2 (deny, fed back to Claude)
#   warn:<reason>  -> reason to stderr (exit 0) AND queued to the warnings relay
#   allow          -> exit 0
#
# Warnings relay: a warn on stderr with exit 0 is invisible to the model (Claude
# Code surfaces stderr only on a block/exit 2). So each warn is also appended to
# .agents/skills/vibe/warnings.log; the UserPromptSubmit inject hook drains that
# log to stdout (which IS injected) next turn, then truncates it.
#
# The three hard blocks are coded in detect-context.sh (single source of truth),
# shipped warn-first elsewhere ("earn the teeth"):
#   1. .spec/lessons.md    — allowed only in feature.compound | setup.apply |
#                            strategy.spec | quick.verify
#   2. root .spec/{product,tech,design,plan}.md
#                          — allowed only in strategy.spec | feature.compound | setup.apply
#   3. .agents/skills/vibe/state.json — never by direct edit (set-state.sh only)
#
# Graceful degrade (R9): missing detect-context.sh / empty or unparseable stdin ->
# exit 0. jq is preferred for reading the path; without it a best-effort sed
# extraction runs, and the three hard blocks still fire (detect-context.sh's
# decision policy is pure bash). An unwritable relay log never fails the hook.

set -euo pipefail

ROOT="${CLAUDE_PROJECT_DIR:-$PWD}"
DETECT="$ROOT/.agents/skills/vibe/scripts/detect-context.sh"
WARN_LOG="$ROOT/.agents/skills/vibe/warnings.log"

[[ -f "$DETECT" ]] || exit 0

INPUT="$(cat 2>/dev/null || true)"
[[ -n "$INPUT" ]] || exit 0

# Append a one-line warning to the relay log for the inject hook to surface.
# Graceful: no vibe dir or an unwritable log is a silent no-op (never fail).
log_warn() {
  local dir="$ROOT/.agents/skills/vibe"
  [[ -d "$dir" ]] || return 0
  printf 'guard: %s\n' "$1" >> "$WARN_LOG" 2>/dev/null || true
}

# Edit/Write carry tool_input.file_path; NotebookEdit carries notebook_path.
# With jq, read them exactly; without it, a best-effort sed on the flat JSON.
if command -v jq >/dev/null 2>&1; then
  PATH_IN="$(printf '%s' "$INPUT" | jq -r '.tool_input.file_path // .tool_input.notebook_path // empty' 2>/dev/null || true)"
else
  PATH_IN="$(printf '%s' "$INPUT" | sed -n 's/.*"file_path"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' | head -n1 || true)"
  [[ -n "$PATH_IN" ]] || PATH_IN="$(printf '%s' "$INPUT" | sed -n 's/.*"notebook_path"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' | head -n1 || true)"
fi
[[ -n "$PATH_IN" ]] || exit 0

# Make repo-relative when possible so the policy's leading-anchored patterns match
# (it also handles */ forms, so an absolute path still matches — this is belt-and-braces).
case "$PATH_IN" in
  "$ROOT"/*) PATH_IN="${PATH_IN#"$ROOT"/}" ;;
esac

VERDICT="$(bash "$DETECT" decide "$PATH_IN" 2>/dev/null || echo allow)"

case "$VERDICT" in
  block:*)
    echo "vibe-guard: BLOCKED — ${VERDICT#block:}" >&2
    echo "vibe-guard: transition with set-state.sh, or edit within the current state's write rules." >&2
    exit 2
    ;;
  warn:*)
    echo "vibe-guard: warn — ${VERDICT#warn:}" >&2
    log_warn "${VERDICT#warn:}"
    exit 0
    ;;
  *)
    exit 0
    ;;
esac
