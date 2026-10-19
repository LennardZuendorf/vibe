#!/usr/bin/env bash
# spec/tests/hooks.sh — the spec hook shim, the generated plugin hooks and the
# committed payload fixtures. Needs jq and target/debug/vibe-spec.
#
# Usage: bash spec/tests/hooks.sh
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd -P)"
SHIM="$ROOT/spec/hooks/run-hook.sh"
FIX="$ROOT/crates/vibe-spec/tests/fixtures/hooks"
BIN="$ROOT/target/debug/vibe-spec"
PASS=0
FAIL=0
pass() { echo "  PASS [$1] $2"; PASS=$((PASS + 1)); }
fail() { echo "  FAIL [$1] $2"; FAIL=$((FAIL + 1)); }
check() { # id desc expected actual
  if [[ "$3" == "$4" ]]; then pass "$1" "$2"; else
    fail "$1" "$2"; echo "        expected: $3"; echo "        actual:   $4"; fi
}

SCRATCH="$(mktemp -d "${TMPDIR:-/tmp}/spec-hooks-test.XXXXXX")"
trap 'rm -rf "$SCRATCH"' EXIT
PROJ="$SCRATCH/proj"
mkdir -p "$PROJ/docs/spec"
: > "$SCRATCH/transcript.jsonl"

# payload <fixture> — fixture with cwd, paths and transcript rewritten into PROJ.
payload() {
  jq -c --arg p "$PROJ" --arg t "$SCRATCH/transcript.jsonl" '
    .cwd = $p | .transcript_path = $t
    | if .tool_input.file_path then .tool_input.file_path = ($p + "/docs/spec/a.md") else . end
  ' "$FIX/$1"
}

MISSING_MSG='vibe-spec: binary missing — spec hooks skipped (build with: cargo build -p vibe-spec)'
EMPTY="$SCRATCH/empty"
mkdir -p "$EMPTY"

test_missing_binary() {
  local ev fx out rc
  for pair in session-start:session-start.json guard:main-write.json validate:post-write.json; do
    ev="${pair%%:*}"; fx="${pair##*:}"
    out="$(payload "$fx" | env -u VIBE_SPEC_BIN PATH="/usr/bin:/bin" CLAUDE_PROJECT_DIR="$EMPTY" bash "$SHIM" "$ev" 2>&1 >/dev/null)"; rc=$?
    check HOOK-MISSING "$ev exits 0 with one stderr line" "0|$MISSING_MSG" "$rc|$out"
  done
}

test_pass_through() {
  local out rc
  out="$(payload main-write.json | VIBE_SPEC_BIN="$BIN" CLAUDE_PROJECT_DIR="$PROJ" bash "$SHIM" guard 2>&1 >/dev/null)"; rc=$?
  check HOOK-BLOCK "unskilled write exits 2" "2" "$rc"
  if [[ -n "$out" ]]; then pass HOOK-BLOCK "block line printed"; else fail HOOK-BLOCK "block line printed"; fi
}

test_crash() {
  local crash out rc
  crash="$SCRATCH/crash.sh"
  printf '#!/usr/bin/env bash\nexit 5\n' > "$crash"; chmod +x "$crash"
  out="$(payload main-write.json | VIBE_SPEC_BIN="$crash" CLAUDE_PROJECT_DIR="$PROJ" bash "$SHIM" guard 2>&1 >/dev/null)"; rc=$?
  check HOOK-CRASH "crash allows with message" "0|vibe-spec: hook guard failed (exit 5) — allowing" "$rc|$out"
}

test_generated_hooks() {
  local f="$ROOT/plugin/hooks/hooks.json" v
  v="$(jq -r '.hooks.SessionStart[] | select(.matcher != null) | .matcher' "$f" 2>/dev/null)"
  check HOOK-GEN "SessionStart matcher" "startup|clear|compact" "$v"
  v="$(jq -r '.hooks.PreToolUse[0].matcher' "$f" 2>/dev/null)"
  check HOOK-GEN "PreToolUse matcher" "Edit|Write|MultiEdit|NotebookEdit|Bash" "$v"
  v="$(jq -r '.hooks.PostToolUse[0].matcher' "$f" 2>/dev/null)"
  check HOOK-GEN "PostToolUse matcher" "Edit|Write|MultiEdit" "$v"
  v="$(jq -r '.hooks.Stop | length' "$f" 2>/dev/null)"
  check HOOK-GEN "Stop entry" "1" "$v"
}

test_drift() {
  local rc
  ( cd "$ROOT" && bash build-plugin.sh --check >/dev/null 2>&1 ); rc=$?
  check HOOK-DRIFT "build-plugin.sh --check exits 0" "0" "$rc"
}

test_missing_binary
test_pass_through
test_crash
test_generated_hooks
test_drift

echo ""
echo "=== hooks: $PASS passed, $FAIL failed ==="
[[ $FAIL -eq 0 ]]
