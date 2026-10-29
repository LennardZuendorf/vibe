#!/usr/bin/env bash
# spec/tests/agents.sh — the two registered spec agents (P4-R9), the retired
# agent names and the captured subagent payload fixture. Needs jq.
#
# Usage: bash spec/tests/agents.sh
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd -P)"
AGENTS="$ROOT/spec/agents"
FIX="$ROOT/crates/vibe-spec/tests/fixtures/hooks/agent-spec-health.json"
PASS=0
FAIL=0
pass() { echo "  PASS [$1] $2"; PASS=$((PASS + 1)); }
fail() { echo "  FAIL [$1] $2"; FAIL=$((FAIL + 1)); }
ok() { if "${@:3}"; then pass "$1" "$2"; else fail "$1" "$2"; fi; }

# shellcheck disable=SC2016 # literal backticks
MISSING='If `vibe-spec` is not on PATH, reply "vibe-spec binary missing" and stop.'
check_agent() { # name model rule-line
  local n="$1" m="$2" rule="$3" f="$AGENTS/$1.md" fm
  [[ -f "$f" ]] || { fail "agents/$n" "$n.md exists"; return; }
  pass "agents/$n" "$n.md exists"
  ok "agents/$n" "at most 40 lines" test "$(wc -l < "$f" | tr -d ' ')" -le 40
  fm="$(awk 'NR==1&&$0!="---"{exit} NR>1&&$0=="---"{exit} NR>1{print}' "$f")"
  ok "agents/$n" "frontmatter name" grep -qx "name: $n" <<<"$fm"
  ok "agents/$n" "frontmatter description" grep -q '^description: .\+' <<<"$fm"
  ok "agents/$n" "frontmatter tools" grep -q '^tools: .\+' <<<"$fm"
  ok "agents/$n" "frontmatter model $m" grep -qx "model: $m" <<<"$fm"
  ok "agents/$n" "exact rule line" grep -qxF "$rule" "$f"
  ok "agents/$n" "exact binary-missing line" grep -qxF "$MISSING" "$f"
}
# shellcheck disable=SC2016 # literal backticks
check_agent spec-promoter sonnet 'Write only under the directory printed by `vibe-spec root`.'
check_agent spec-health haiku 'Read-only: never write files.'

# shellcheck disable=SC2016 # literal backticks
ok agents/spec-promoter "bare apply command on its own line" \
  grep -qE '^ *`vibe-spec promote <feature>`$' "$AGENTS/spec-promoter.md"
ok agents/spec-promoter "tools include Write" grep -qE '^tools: .*Write' "$AGENTS/spec-promoter.md"

# skill-revision/6: non-interactive agents with a status line and real fields.
P="$AGENTS/spec-promoter.md" H="$AGENTS/spec-health.md"
ok agents/spec-promoter "STATUS line with the closed enum" \
  grep -qF 'STATUS: DONE | DONE_WITH_CONCERNS | NEEDS_CONTEXT | BLOCKED' "$P"
ok agents/spec-promoter "applies only on 'apply: yes'" grep -qF 'apply: yes' "$P"
# shellcheck disable=SC2016 # literal backticks
ok agents/spec-promoter "never runs --done" grep -qF 'Never run `--done`' "$P"
ok agents/spec-promoter "never waits for an answer" test -z "$(grep -F 'Wait for' "$P")"
ok agents/spec-promoter "never asks the user" test -z "$(grep -F 'Ask the user' "$P")"
for fld in check file message severity; do
  # shellcheck disable=SC2016 # literal backticks
  ok agents/spec-health "names the JSON field \`$fld\`" grep -qF "\`$fld\`" "$H"
done
# shellcheck disable=SC2016 # literal backticks
ok agents/spec-health "ends with the health count line" grep -qF 'health: <n> error, <n> warning' "$H"
ok agents/spec-health "names no CRITICAL level" test -z "$(grep -F 'CRITICAL' "$H")"

ok agents "no old SKILL.md-format agent dirs" test -z "$(find "$AGENTS" -mindepth 1 -type d)"

hits="$(grep -rIl -e spec-interviewer -e spec-tracer "$ROOT/spec" "$ROOT/.claude" "$ROOT/plugin" "$ROOT/flow" 2>/dev/null \
  | grep -v '/spec/tests/agents.sh$' || true)"
ok agents "no spec-interviewer or spec-tracer under spec/, .claude/, plugin/, flow/" test -z "$hits"
[[ -z "$hits" ]] || echo "$hits"

for n in spec-promoter spec-health; do
  ok agents ".claude/agents/$n.md is a symlink to the spec agent" \
    test "$(readlink "$ROOT/.claude/agents/$n.md")" = "../../spec/agents/$n.md"
  ok agents "plugin/agents resolves $n.md" test -f "$ROOT/plugin/agents/$n.md"
done

ok fixture "agent-spec-health.json has agent_type spec-health" \
  jq -e '.agent_type == "spec-health"' "$FIX" >/dev/null
ok fixture "fixture holds no home path" test -z "$(grep -l lennarddib "$FIX" 2>/dev/null)"

echo "agents.sh: $PASS passed, $FAIL failed"
[[ $FAIL -eq 0 ]]
