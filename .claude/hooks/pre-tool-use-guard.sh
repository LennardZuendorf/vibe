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
#   warn:<reason>  -> reason to stderr, exit 0 (non-blocking guidance)
#   allow          -> exit 0
#
# The three hard blocks (lessons.md outside compound; root specs outside
# strategy.spec/feature.compound; direct state.json edits) are coded in
# detect-context.sh, shipped warn-first elsewhere ("earn the teeth").
#
# Graceful degrade (R9): missing jq / detect-context.sh / empty or unparseable
# stdin -> exit 0. Never end a session.

set -euo pipefail

ROOT="${CLAUDE_PROJECT_DIR:-$PWD}"
DETECT="$ROOT/.agents/skills/vibe/scripts/detect-context.sh"

command -v jq >/dev/null 2>&1 || exit 0
[[ -f "$DETECT" ]] || exit 0

INPUT="$(cat 2>/dev/null || true)"
[[ -n "$INPUT" ]] || exit 0

# Edit/Write carry tool_input.file_path; NotebookEdit carries notebook_path.
PATH_IN="$(printf '%s' "$INPUT" | jq -r '.tool_input.file_path // .tool_input.notebook_path // empty' 2>/dev/null || true)"
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
    echo "vibe-guard: transition with set-state.sh, or amend within the current state's write rules." >&2
    exit 2
    ;;
  warn:*)
    echo "vibe-guard: warn — ${VERDICT#warn:}" >&2
    exit 0
    ;;
  *)
    exit 0
    ;;
esac
