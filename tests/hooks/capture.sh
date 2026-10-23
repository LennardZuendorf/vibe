#!/usr/bin/env bash
# Re-captures the hook payload and transcript fixtures in
# crates/vibe-spec/tests/fixtures/hooks/ from a real Claude Code run.
# Not part of CI: it needs the `claude` CLI, jq, and network access.
#
# Usage: CLAUDE_CODE_OAUTH_TOKEN=... bash tests/hooks/capture.sh [out-dir]
# The token comes from the environment only. This script never prints it.
set -euo pipefail

: "${CLAUDE_CODE_OAUTH_TOKEN:?set CLAUDE_CODE_OAUTH_TOKEN in the environment}"
export CLAUDE_CODE_OAUTH_TOKEN
for tool in claude jq; do
  command -v "$tool" >/dev/null || { echo "capture.sh: $tool not found" >&2; exit 1; }
done

here="$(cd "$(dirname "$0")" && pwd)"
out="${1:-$here/../../crates/vibe-spec/tests/fixtures/hooks}"
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
project="$(cd "$work" && pwd -P)/project"
home="$work/home"
mkdir -p "$project/.claude/skills/spec" "$project/log" "$project/docs/spec" "$home" "$out"
out="$(cd "$out" && pwd -P)"

# A project skill named `spec`, so Skill(spec) and /spec resolve.
cat > "$project/.claude/skills/spec/SKILL.md" <<'SKILL'
---
name: spec
description: Test skill. Loading it unlocks edits under docs/spec.
---
Spec skill loaded. You may now edit files under docs/spec.
SKILL

# Dumps each hook's stdin to log/<event>-<n>.json.
cat > "$project/.claude/dump.sh" <<'DUMP'
#!/usr/bin/env bash
d="$(cd "$(dirname "$0")/.." && pwd)/log"
n=$(find "$d" -type f | wc -l | tr -d ' ')
cat > "$d/$1-$n.json"
exit 0
DUMP
chmod +x "$project/.claude/dump.sh"

# shellcheck disable=SC2016 # the hook command expands $CLAUDE_PROJECT_DIR itself
dump() { printf '$CLAUDE_PROJECT_DIR/.claude/dump.sh %s' "$1"; }
cat > "$project/.claude/settings.json" <<JSON
{
  "hooks": {
    "SessionStart": [{"hooks": [{"type": "command", "command": "$(dump SessionStart)"}]}],
    "PreToolUse": [{"matcher": "Edit|Write|MultiEdit|NotebookEdit|Skill|Task|Agent", "hooks": [{"type": "command", "command": "$(dump PreToolUse)"}]}],
    "PostToolUse": [{"matcher": "Edit|Write|MultiEdit", "hooks": [{"type": "command", "command": "$(dump PostToolUse)"}]}],
    "Stop": [{"hooks": [{"type": "command", "command": "$(dump Stop)"}]}]
  }
}
JSON

run() { # prompt
  (cd "$project" && HOME="$home" claude -p --model haiku --setting-sources project \
    --permission-mode acceptEdits "$1" >/dev/null)
}

# Rewrites the temp project and transcript dirs to the fixed fixture paths.
san() {
  sed -e "s#$project#/work/project#g" \
      -e "s#$home/.claude/projects/[^/\"]*#/home/u/.claude/projects/p#g"
}
# Keeps only user and assistant records and shrinks their content.
slim='select(.type=="user" or .type=="assistant") | {type, isSidechain, message: {role: .message.role, content: (.message.content | if type=="array" then map(if .type=="tool_use" then {type, name, input: (.input | del(.content))} elif .type=="tool_result" then {type, tool_use_id} else {type, text: (.text // "" | .[0:300])} end) else .[0:300] end)}}'

newest() { find "$home/.claude/projects" -name '*.jsonl' -not -path '*/subagents/*' -print0 \
  | xargs -0 ls -t | head -n 1; }
last() { find "$project/log" -name "$1-*.json" -print0 | xargs -0 ls -t | head -n 1; }

# 1. Main session: Skill(spec) then Write.
run 'First invoke the Skill tool with skill "spec". Then use the Write tool to create docs/spec/a.md containing the single line: hello. Then stop.'
main="$(newest)"
jq -c "$slim" "$main" | san > "$out/transcript-main-skill.jsonl"
san < "$(last Stop)" > "$out/stop.json"
san < "$(last SessionStart)" > "$out/session-start.json"
san < "$(last PostToolUse)" | jq 'del(.tool_input.content)' > "$out/post-write.json"
pre="$(find "$project/log" -name 'PreToolUse-*.json' -print0 | xargs -0 grep -l '"tool_name": *"Write"' | head -n 1)"
san < "$pre" | jq 'del(.tool_input.content)' > "$out/main-write.json"
skillpre="$(find "$project/log" -name 'PreToolUse-*.json' -print0 | xargs -0 grep -l '"tool_name": *"Skill"' | head -n 1)"
san < "$skillpre" > "$out/main-skill.json"

# 2. Slash command.
run '/spec create docs/spec/d.md containing the single line: slash'
jq -c "$slim" "$(newest)" | san > "$out/transcript-slash-command.jsonl"

# 3. Subagents: one relies on the parent's skill load, one loads its own.
rm -rf "$project/log"/*
run 'Step 1: invoke the Skill tool with skill "spec". Step 2: use the Agent tool to launch a general-purpose subagent with model haiku and this exact prompt: "Use the Write tool to create docs/spec/b.md containing the single line: sub. Do not invoke any skill. Then reply done." Step 3: launch a second general-purpose subagent with this prompt: "The spec skill rules apply. Use the Write tool to create docs/spec/c.md containing: sub2. Then reply done." Then stop.'
parent="$(newest)"
jq -c "$slim" "$parent" | san > "$out/transcript-parent-of-subagents.jsonl"
agents="${parent%.jsonl}/subagents"
for f in $(find "$project/log" -name 'PreToolUse-*.json' -print0 | xargs -0 grep -l '"agent_id"'); do
  id="$(jq -r .agent_id "$f")"
  if jq -e 'select(.type=="assistant") | .message.content[]? | select(.type=="tool_use" and .name=="Skill")' \
    "$agents/agent-$id.jsonl" >/dev/null 2>&1; then
    payload=subagent-write-own-skill.json
    tfile=transcript-subagent-own-skill.jsonl
  else
    payload=subagent-write-parent-skill.json
    tfile=transcript-subagent-no-skill.jsonl
  fi
  if [ "$(jq -r .tool_name "$f")" = Write ]; then
    san < "$f" | jq 'del(.tool_input.content)' > "$out/$payload"
    jq -c "$slim" "$agents/agent-$id.jsonl" | san > "$out/$tfile"
    cp "$agents/agent-$id.meta.json" "$out/subagent-meta.json"
  fi
done
echo "capture.sh: fixtures written to $out"
