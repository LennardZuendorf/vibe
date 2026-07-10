#!/usr/bin/env bash
# merge-settings.sh — wire (or unwire) the vibe flow hooks in a target repo's
# .claude/settings.json, so the hooks fire natively without a plugin.
#
#   merge-settings.sh merge   <target-root>   # add the three vibe hook entries
#   merge-settings.sh unmerge <target-root>   # remove only the vibe hook entries
#
# Contract (byte-exact — install.sh, doctor, and the docs describe this shape):
#   .claude/settings.json gains a "hooks" object with three vibe entries, each a
#   command of the form  bash "$CLAUDE_PROJECT_DIR/.claude/hooks/<script>.sh":
#     UserPromptSubmit -> user-prompt-submit-inject.sh   (no matcher, timeout 10)
#     PreToolUse       -> pre-tool-use-guard.sh           (matcher Edit|Write|NotebookEdit)
#     Stop             -> stop-gate.sh                     (no matcher, timeout 10)
#
# Merge is idempotent: vibe's own groups (detected by the .claude/hooks/<script>.sh
# command path) are stripped before re-adding, so re-running never duplicates.
# User settings and other hooks are preserved. Unmerge removes only vibe's three
# groups; it drops the file only if that leaves it empty ({}).
#
# Graceful degrade: no jq -> warn and print the exact snippet to paste by hand;
# invalid existing settings.json -> warn and leave it untouched. Never hard-fail.

set -euo pipefail

warn() { echo "merge-settings: $1" >&2; }

# The three vibe hook groups, keyed by event. $CLAUDE_PROJECT_DIR stays literal:
# Claude Code expands it per-project at hook time (single quotes protect it here).
# shellcheck disable=SC2016  # intentional: keep $CLAUDE_PROJECT_DIR literal in the JSON
VIBE_HOOKS='{
  "UserPromptSubmit": {
    "hooks": [
      { "type": "command", "command": "bash \"$CLAUDE_PROJECT_DIR/.claude/hooks/user-prompt-submit-inject.sh\"", "timeout": 10 }
    ]
  },
  "PreToolUse": {
    "matcher": "Edit|Write|NotebookEdit",
    "hooks": [
      { "type": "command", "command": "bash \"$CLAUDE_PROJECT_DIR/.claude/hooks/pre-tool-use-guard.sh\"", "timeout": 10 }
    ]
  },
  "Stop": {
    "hooks": [
      { "type": "command", "command": "bash \"$CLAUDE_PROJECT_DIR/.claude/hooks/stop-gate.sh\"", "timeout": 10 }
    ]
  }
}'

# A jq predicate: a hook group belongs to vibe iff any of its commands points at
# one of vibe's three shipped hook scripts under .claude/hooks/.
VIBE_MATCH='\.claude/hooks/(user-prompt-submit-inject|pre-tool-use-guard|stop-gate)\.sh'

# print_snippet — emit the JSON the user should merge into .claude/settings.json
# by hand when jq is unavailable.
print_snippet() {
  echo "merge-settings: add these hooks to $1/.claude/settings.json manually:" >&2
  printf '%s\n' "{ \"hooks\": $(jq_free_snippet) }" >&2
}

# jq_free_snippet — the three groups wrapped as an event->[group] map, formatted
# without jq (VIBE_HOOKS is already valid JSON; wrap each value in an array).
jq_free_snippet() {
  cat <<'EOF'
{
    "UserPromptSubmit": [ { "hooks": [ { "type": "command", "command": "bash \"$CLAUDE_PROJECT_DIR/.claude/hooks/user-prompt-submit-inject.sh\"", "timeout": 10 } ] } ],
    "PreToolUse": [ { "matcher": "Edit|Write|NotebookEdit", "hooks": [ { "type": "command", "command": "bash \"$CLAUDE_PROJECT_DIR/.claude/hooks/pre-tool-use-guard.sh\"", "timeout": 10 } ] } ],
    "Stop": [ { "hooks": [ { "type": "command", "command": "bash \"$CLAUDE_PROJECT_DIR/.claude/hooks/stop-gate.sh\"", "timeout": 10 } ] } ]
}
EOF
}

do_merge() {
  local target="$1" settings="$1/.claude/settings.json" tmp
  if ! command -v jq >/dev/null 2>&1; then
    warn "jq not found — cannot auto-wire hooks."
    print_snippet "$target"
    return 0
  fi
  mkdir -p "$target/.claude"
  [[ -f "$settings" ]] || echo '{}' >"$settings"

  tmp="$(mktemp)"
  if ! jq --argjson vibe "$VIBE_HOOKS" --arg m "$VIBE_MATCH" '
        def strip_vibe(g): (g // []) | map(
          select(([.hooks[]?.command // empty] | any(test($m))) | not));
        .hooks = (.hooks // {})
        | .hooks.UserPromptSubmit = (strip_vibe(.hooks.UserPromptSubmit) + [$vibe.UserPromptSubmit])
        | .hooks.PreToolUse       = (strip_vibe(.hooks.PreToolUse)       + [$vibe.PreToolUse])
        | .hooks.Stop             = (strip_vibe(.hooks.Stop)             + [$vibe.Stop])
      ' "$settings" >"$tmp" 2>/dev/null; then
    rm -f "$tmp"
    warn "existing $settings is not valid JSON — left untouched."
    print_snippet "$target"
    return 0
  fi
  mv -f "$tmp" "$settings"
}

do_unmerge() {
  local target="$1" settings="$1/.claude/settings.json" tmp
  [[ -f "$settings" ]] || return 0
  if ! command -v jq >/dev/null 2>&1; then
    warn "jq not found — cannot unwire the vibe hooks from $settings; remove them by hand."
    return 1
  fi
  tmp="$(mktemp)"
  if ! jq --arg m "$VIBE_MATCH" '
        def strip_vibe(g): (g // []) | map(
          select(([.hooks[]?.command // empty] | any(test($m))) | not));
        if .hooks then
          .hooks.UserPromptSubmit = strip_vibe(.hooks.UserPromptSubmit)
          | .hooks.PreToolUse     = strip_vibe(.hooks.PreToolUse)
          | .hooks.Stop           = strip_vibe(.hooks.Stop)
          | .hooks |= with_entries(select(.value | length > 0))
          | if (.hooks | length) == 0 then del(.hooks) else . end
        else . end
      ' "$settings" >"$tmp" 2>/dev/null; then
    rm -f "$tmp"
    warn "existing $settings is not valid JSON — left untouched."
    return 0
  fi
  if [[ "$(jq -c . "$tmp" 2>/dev/null)" == "{}" ]]; then
    rm -f "$tmp" "$settings"
  else
    mv -f "$tmp" "$settings"
  fi
}

case "${1:-}" in
  merge)   [[ $# -ge 2 ]] || { warn "usage: merge-settings.sh merge <target-root>"; exit 1; }
           do_merge "$2" ;;
  unmerge) [[ $# -ge 2 ]] || { warn "usage: merge-settings.sh unmerge <target-root>"; exit 1; }
           do_unmerge "$2" ;;
  *) warn "usage: merge-settings.sh {merge|unmerge} <target-root>"; exit 1 ;;
esac
