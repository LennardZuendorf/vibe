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

if [[ ! -x "$BIN" ]] && command -v cargo >/dev/null 2>&1; then
  (cd "$ROOT" && cargo build -q -p vibe-spec)
fi
if [[ ! -x "$BIN" ]]; then
  echo "hooks.sh: $BIN missing and cargo could not build it" >&2
  exit 1
fi

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
    out="$(env -u VIBE_SPEC_BIN PATH="/usr/bin:/bin" CLAUDE_PROJECT_DIR="$EMPTY" bash "$SHIM" "$ev" < <(payload "$fx") 2>&1 >/dev/null)"; rc=$?
    check HOOK-MISSING "$ev exits 0 with one stderr line" "0|$MISSING_MSG" "$rc|$out"
  done
}

test_pass_through() {
  local out rc
  out="$(VIBE_SPEC_BIN="$BIN" CLAUDE_PROJECT_DIR="$PROJ" bash "$SHIM" guard < <(payload main-write.json) 2>&1 >/dev/null)"; rc=$?
  check HOOK-BLOCK "unskilled write exits 2 with the block line" "2|blocked: invoke Skill(spec) before editing docs/spec/a.md" "$rc|$out"
}

test_crash() {
  local crash out rc
  crash="$SCRATCH/crash.sh"
  printf '#!/usr/bin/env bash\nexit 5\n' > "$crash"; chmod +x "$crash"
  out="$(VIBE_SPEC_BIN="$crash" CLAUDE_PROJECT_DIR="$PROJ" bash "$SHIM" guard < <(payload main-write.json) 2>&1 >/dev/null)"; rc=$?
  check HOOK-CRASH "crash allows with message" "0|vibe-spec: hook guard failed (exit 5) — allowing" "$rc|$out"
}

test_nonexistent_binary() {
  local out rc
  out="$(VIBE_SPEC_BIN="$SCRATCH/nope" CLAUDE_PROJECT_DIR="$PROJ" bash "$SHIM" guard < <(payload main-write.json) 2>&1 >/dev/null)"; rc=$?
  check HOOK-NOBIN "nonexistent VIBE_SPEC_BIN exits 0" "0" "$rc"
  if [[ "$out" == *"vibe-spec: hook guard failed (exit 127) — allowing"* ]]; then
    pass HOOK-NOBIN "stderr names exit 127"; else fail HOOK-NOBIN "stderr names exit 127"; echo "        actual: $out"; fi
}

# fake <name> <exit> <stderr-printf-format> — an executable fake binary.
fake() {
  printf '#!/usr/bin/env bash\ncat >/dev/null\nprintf %s >&2\nexit %s\n' "'$3'" "$2" > "$SCRATCH/$1"
  chmod +x "$SCRATCH/$1"
}

test_usage_errors() {
  local ev fx out rc
  fake usage2.sh 2 'error: unrecognized subcommand hook\nUsage: vibe-spec\n'
  for pair in session-start:session-start.json guard:main-write.json validate:post-write.json; do
    ev="${pair%%:*}"; fx="${pair##*:}"
    out="$(VIBE_SPEC_BIN="$SCRATCH/usage2.sh" CLAUDE_PROJECT_DIR="$PROJ" bash "$SHIM" "$ev" < <(payload "$fx") 2>&1 >/dev/null)"; rc=$?
    check HOOK-USAGE "$ev: exit 2 with usage stderr fails open" "0" "$rc"
  done
  out="$(VIBE_SPEC_BIN="$SCRATCH/usage2.sh" CLAUDE_PROJECT_DIR="$PROJ" bash "$SHIM" Stop < <(payload main-write.json) 2>&1 >/dev/null)"; rc=$?
  check HOOK-USAGE "Stop: exit 2 fails open" "0" "$rc"
  fake block1.sh 2 'blocked: x\n'
  out="$(VIBE_SPEC_BIN="$SCRATCH/block1.sh" CLAUDE_PROJECT_DIR="$PROJ" bash "$SHIM" guard < <(payload main-write.json) 2>&1 >/dev/null)"; rc=$?
  check HOOK-USAGE "guard: single blocked line exits 2" "2|blocked: x" "$rc|$out"
  fake block2.sh 2 'blocked: x\nextra\n'
  out="$(VIBE_SPEC_BIN="$SCRATCH/block2.sh" CLAUDE_PROJECT_DIR="$PROJ" bash "$SHIM" guard < <(payload main-write.json) 2>&1 >/dev/null)"; rc=$?
  check HOOK-USAGE "guard: two stderr lines fail open" "0" "$rc"
  # a PATH binary that lacks the hook subcommand
  mkdir -p "$SCRATCH/pathbin"
  cp "$SCRATCH/usage2.sh" "$SCRATCH/pathbin/vibe-spec"
  out="$(env -u VIBE_SPEC_BIN PATH="$SCRATCH/pathbin:/usr/bin:/bin" CLAUDE_PROJECT_DIR="$EMPTY" bash "$SHIM" guard < <(payload main-write.json) 2>&1 >/dev/null)"; rc=$?
  check HOOK-USAGE "PATH binary without hook subcommand fails open" "0" "$rc"
}

test_newer_binary() {
  local proj="$SCRATCH/proj-bins" out
  mkdir -p "$proj/target/release" "$proj/target/debug"
  printf '#!/usr/bin/env bash\necho release >&2\nexit 0\n' > "$proj/target/release/vibe-spec"
  printf '#!/usr/bin/env bash\necho debug >&2\nexit 0\n' > "$proj/target/debug/vibe-spec"
  chmod +x "$proj/target/release/vibe-spec" "$proj/target/debug/vibe-spec"
  touch -t 202001010000 "$proj/target/release/vibe-spec"
  out="$(env -u VIBE_SPEC_BIN PATH="/usr/bin:/bin" CLAUDE_PROJECT_DIR="$proj" bash "$SHIM" guard < <(payload main-write.json) 2>&1 >/dev/null)"
  check HOOK-NEWER "newer debug binary beats older release" "debug" "$out"
}

test_generated_hooks() {
  local f="$ROOT/plugin/hooks/hooks.json" v
  v="$(jq -r '.hooks.SessionStart[] | select(.matcher != null) | .matcher' "$f" 2>/dev/null)"
  check HOOK-GEN "SessionStart matcher" "startup|clear|compact" "$v"
  v="$(jq -r '.hooks.PreToolUse[0].matcher' "$f" 2>/dev/null)"
  check HOOK-GEN "PreToolUse matcher" "Edit|Write|MultiEdit|NotebookEdit|Bash" "$v"
  v="$(jq -r '.hooks.PostToolUse[0].matcher' "$f" 2>/dev/null)"
  check HOOK-GEN "PostToolUse matcher" "Edit|Write|MultiEdit" "$v"
  v="$(jq -r '.hooks.SessionStart[] | select(.matcher == null) | .hooks[0].timeout' "$f" 2>/dev/null)"
  check HOOK-GEN "doctrine SessionStart timeout" "10" "$v"
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
test_nonexistent_binary
test_usage_errors
test_newer_binary
test_generated_hooks
test_drift

echo ""
echo "=== hooks: $PASS passed, $FAIL failed ==="
[[ $FAIL -eq 0 ]]
