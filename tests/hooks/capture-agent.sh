#!/usr/bin/env bash
# Re-captures crates/vibe-spec/tests/fixtures/hooks/agent-spec-health.json: the
# PreToolUse payload of a tool call made inside the spec-health agent.
# Not part of CI: it needs the `claude` CLI, jq, and network access.
#
# Usage: CLAUDE_CODE_OAUTH_TOKEN=... bash tests/hooks/capture-agent.sh [out-file]
# The token comes from the environment only. This script never prints it.
set -euo pipefail

: "${CLAUDE_CODE_OAUTH_TOKEN:?set CLAUDE_CODE_OAUTH_TOKEN in the environment}"
export CLAUDE_CODE_OAUTH_TOKEN
for tool in claude jq; do
  command -v "$tool" >/dev/null || { echo "capture-agent.sh: $tool not found" >&2; exit 1; }
done

here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../.." && pwd -P)"
out="${1:-$repo/crates/vibe-spec/tests/fixtures/hooks/agent-spec-health.json}"
mkdir -p "$(dirname "$out")"
out="$(cd "$(dirname "$out")" && pwd -P)/$(basename "$out")"
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
project="$(cd "$work" && pwd -P)/project"
home="$work/home"
mkdir -p "$project/.claude/agents" "$project/log" "$project/docs/spec" "$home"
cp "$repo/spec/agents/spec-health.md" "$project/.claude/agents/spec-health.md"

# Dumps each hook's stdin to log/<event>-<n>.json.
cat > "$project/.claude/dump.sh" <<'DUMP'
#!/usr/bin/env bash
d="$(cd "$(dirname "$0")/.." && pwd)/log"
n=$(find "$d" -type f | wc -l | tr -d ' ')
cat > "$d/$1-$n.json"
exit 0
DUMP
chmod +x "$project/.claude/dump.sh"

cat > "$project/.claude/settings.json" <<'JSON'
{
  "hooks": {
    "PreToolUse": [{"matcher": "Read|Bash", "hooks": [{"type": "command", "command": "$CLAUDE_PROJECT_DIR/.claude/dump.sh PreToolUse"}]}]
  }
}
JSON

attempt() {
  (cd "$project" && HOME="$home" claude -p "Use the spec-health agent to check this project." \
    --model haiku --setting-sources project --permission-mode acceptEdits \
    --allowedTools "Agent,Task,Read,Bash" </dev/null >/dev/null) || true
  grep -l '"agent_type": *"spec-health"' "$project"/log/PreToolUse-*.json 2>/dev/null | head -n 1 || true
}

hit=""
for _ in 1 2; do
  hit="$(attempt)"
  [ -n "$hit" ] && break
done
[ -n "$hit" ] || { echo "capture-agent.sh: no spec-health agent payload captured" >&2; exit 1; }

sed -e "s#$project#/work/project#g" \
    -e "s#$home/.claude/projects/[^/\"]*#/home/u/.claude/projects/p#g" "$hit" | jq . > "$out"
echo "capture-agent.sh: fixture written to $out"
