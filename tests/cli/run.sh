#!/usr/bin/env bash
# tests/cli/run.sh — system tests for the vibe-cli (feature vibe-cli).
#
# Two gates:
#   1. the CLI's own pytest suite (unit + byte-parity vs the bash origins), and
#   2. a real end-to-end lifecycle against the installed console_scripts —
#      `vibe init` into a fresh throwaway target, then status / guard exit codes /
#      doctor / surgical uninstall — the install-target reality the in-repo unit
#      suites cannot exercise (the "stranger" lesson).
#
# Pure bash; no bats. Uses a fake `claude` on PATH so `init`'s plugin step never
# mutates global Claude Code state. Exit 0 = all pass.

# shellcheck disable=SC2015
set -uo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
CLI="$REPO_ROOT/cli"
VENV="$CLI/.venv"
VIBE="$VENV/bin/vibe"
VIBEHOOK="$VENV/bin/vibe-hook"

PASS=0
FAIL=0
pass() { echo "  PASS [$1] $2"; PASS=$((PASS + 1)); }
fail() { echo "  FAIL [$1] $2"; FAIL=$((FAIL + 1)); }
assert_eq()       { if [[ "$3" == "$4" ]]; then pass "$1" "$2"; else fail "$1" "$2"; echo "        expected: $4"; echo "        got:      $3"; fi; }
assert_file()     { if [[ -e "$3" ]]; then pass "$1" "$2"; else fail "$1" "$2"; echo "        missing: $3"; fi; }
assert_no_file()  { if [[ ! -e "$3" ]]; then pass "$1" "$2"; else fail "$1" "$2"; echo "        present: $3"; fi; }
assert_grep()     { if grep -qF "$4" "$3" 2>/dev/null; then pass "$1" "$2"; else fail "$1" "$2"; echo "        '$4' not in $3"; fi; }

# --- ensure the editable install exists --------------------------------------
if [[ ! -x "$VIBE" ]]; then
  echo "=== cli venv missing — running uv sync ==="
  if command -v uv >/dev/null 2>&1; then
    ( cd "$CLI" && uv sync ) || { echo "  FAIL uv sync"; exit 1; }
  else
    echo "  SKIP: uv not on PATH; cannot build the cli venv"
    exit 0
  fi
fi

# --- gate 1: the pytest suite (unit + parity) --------------------------------
echo "=== cli/1 — pytest suite (unit + parity) ==="
if ( cd "$CLI" && uv run --no-sync pytest -q ) ; then
  pass "cli/1" "cli pytest suite green"
else
  fail "cli/1" "cli pytest suite"
fi

# --- gate 2: end-to-end install lifecycle ------------------------------------
echo "=== cli/2 — end-to-end install lifecycle (fresh target) ==="
TARGET="$(mktemp -d)"
FAKEBIN="$(mktemp -d)"
printf '#!/bin/sh\nexit 0\n' > "$FAKEBIN/claude"
chmod +x "$FAKEBIN/claude"
cleanup() { rm -rf "$TARGET" "$FAKEBIN"; }
trap cleanup EXIT

PATH="$FAKEBIN:$PATH" "$VIBE" init "$TARGET" --yes >/dev/null 2>&1
assert_file "cli/2" "init provisions the vibe skill tree"  "$TARGET/.claude/skills/vibe/SKILL.md"
assert_file "cli/2" "init provisions the spec skill tree"  "$TARGET/.claude/skills/spec/SKILL.md"
assert_grep "cli/2" "init wires the guard hook"            "$TARGET/.claude/settings.json" "vibe-hook guard"
assert_grep "cli/2" "init merges the AGENTS.md block"      "$TARGET/AGENTS.md" "vibe:instructions"
assert_file "cli/2" "init seeds the flow cursor"           "$TARGET/.agents/skills/vibe/state.json"
assert_grep "cli/2" "init gitignores the cursor"           "$TARGET/.gitignore" "state.json"

out="$( cd "$TARGET" && "$VIBE" status 2>&1 )"
case "$out" in *idle*) pass "cli/2" "status shows idle" ;; *) fail "cli/2" "status shows idle"; echo "        got: $out" ;; esac

# guard: block a hard-blocked path, allow a source path (from within the target)
echo '{"tool_input":{"file_path":".spec/lessons.md"}}' | ( cd "$TARGET" && CLAUDE_PROJECT_DIR="$TARGET" "$VIBEHOOK" guard >/dev/null 2>&1 )
assert_eq "cli/2" "guard blocks lessons.md at idle (exit 2)" "$?" "2"
echo '{"tool_input":{"file_path":"src/foo.py"}}' | ( cd "$TARGET" && CLAUDE_PROJECT_DIR="$TARGET" "$VIBEHOOK" guard >/dev/null 2>&1 )
assert_eq "cli/2" "guard allows src/ at idle (exit 0)" "$?" "0"

( cd "$TARGET" && "$VIBE" doctor >/dev/null 2>&1 )
assert_eq "cli/2" "doctor exits 0 (warn-only)" "$?" "0"

# surgical uninstall: a co-located user file must survive; shipped file must go
echo "keep me" > "$TARGET/.claude/skills/USER_NOTE.md"
"$VIBE" uninstall "$TARGET" >/dev/null 2>&1
assert_file    "cli/2" "uninstall preserves a co-located user file" "$TARGET/.claude/skills/USER_NOTE.md"
assert_no_file "cli/2" "uninstall removes the shipped skill file"   "$TARGET/.claude/skills/vibe/SKILL.md"
assert_file    "cli/2" "uninstall preserves the cursor (no --yes)"  "$TARGET/.agents/skills/vibe/state.json"

# --- summary ------------------------------------------------------------------
echo ""
echo "=== cli suite: $PASS passed, $FAIL failed ==="
[[ "$FAIL" -eq 0 ]]
