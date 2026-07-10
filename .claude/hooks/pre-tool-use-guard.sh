#!/usr/bin/env bash
# pre-tool-use-guard.sh — vibe flow guard hook (platform-adapters/2).
#
# Event: PreToolUse, matcher Edit|Write|NotebookEdit|Bash. For the file tools it
# reads the target path from stdin and asks the shared decision policy whether the
# write is allowed in the current flow state. NO path policy lives here (R8) — it
# all lives once in .agents/skills/vibe/scripts/detect-context.sh decide.
#
# Bash is a WARN-ONLY sniffer, never a block: the three hard blocks intercept
# file-tool calls only, so a raw `echo >> .spec/lessons.md` would otherwise slip
# past them undocumented. When tool_name is Bash the command is scanned for a
# write-shaped operation (>, >>, tee, sed -i, truncate, mv/cp/rm) aimed at one of
# the three guarded path classes; a hit emits a WARN through the relay and exits 0.
# False positives are certain (it is a text scan), so it can only warn — a command
# merely reading a guarded path (grep, cat) has no write op and does NOT warn, and
# a command driving set-state.sh is never warned about state.json.
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

have_jq() { command -v jq >/dev/null 2>&1; }

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

# warn_bash — relay a warn-only Bash-write smell (stderr + the relay log). Never blocks.
warn_bash() {
  echo "vibe-guard: warn — a bash command looks like it writes $1 (a guarded path); prefer the flow's write surface over a raw shell redirect. (warn-only)" >&2
  log_warn "bash command appears to write $1 (bypasses the file-tool guard) (warn-only)"
}

# sniff_bash — text-scan a Bash command for a write-shaped op aimed at a guarded
# path class, and warn (never block) on the first hit. A command with no write op
# (a pure read like grep/cat) never warns; set-state.sh is the sanctioned state.json
# writer, so it is never warned about state.json.
sniff_bash() {
  local cmd="$1"
  # A write-shaped operator anywhere in the command (redirect, tee, in-place sed,
  # truncate, or a destructive mv/cp/rm). No write op -> nothing to warn about.
  grep -Eq '(>>?|(^|[[:space:]])(tee|truncate|mv|cp|rm)([[:space:]]|$)|(^|[[:space:]])sed([[:space:]]|$).*-i)' <<<"$cmd" || return 0

  if grep -Eq '(^|[^[:alnum:]_])\.spec/lessons\.md' <<<"$cmd"; then
    warn_bash ".spec/lessons.md"; return 0
  fi
  if grep -Eq '(^|[^[:alnum:]_])\.spec/(product|tech|design|plan)\.md' <<<"$cmd"; then
    warn_bash "a root .spec/{product,tech,design,plan}.md doc"; return 0
  fi
  if grep -Eq '(\.agents/skills/vibe/state\.json|(^|[^[:alnum:]_])flow/state\.json)' <<<"$cmd"; then
    grep -q 'set-state\.sh' <<<"$cmd" && return 0   # sanctioned writer: never warn
    warn_bash ".agents/skills/vibe/state.json (use set-state.sh)"; return 0
  fi
}

# tool_name routes the handler: Bash -> warn-only sniffer; file tools -> path policy.
if have_jq; then
  TOOL="$(printf '%s' "$INPUT" | jq -r '.tool_name // empty' 2>/dev/null || true)"
else
  TOOL="$(printf '%s' "$INPUT" | sed -n 's/.*"tool_name"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' | head -n1 || true)"
fi

if [[ "$TOOL" == "Bash" ]]; then
  if have_jq; then
    CMD="$(printf '%s' "$INPUT" | jq -r '.tool_input.command // empty' 2>/dev/null || true)"
  else
    CMD="$(printf '%s' "$INPUT" | sed -n 's/.*"command"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' | head -n1 || true)"
  fi
  [[ -n "$CMD" ]] || exit 0
  sniff_bash "$CMD"
  exit 0
fi

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
