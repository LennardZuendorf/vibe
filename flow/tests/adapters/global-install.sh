#!/usr/bin/env bash
# vibe-spec: literal-file
# flow/tests/adapters/global-install.sh — vibe#71 sub-gap 2, end to end.
#
# `install.sh --global` used to be Claude-only (`claude plugin install`), so a
# Codex, Droid or OpenCode user had to hand-copy skill trees. It now writes the
# two skills into each selected host's OWN user-level skills dir and maintains
# one shared user-level AGENTS.md block. This leg proves that against a throwaway
# HOME, per host, and proves the SECOND run changes no byte.
#
# Hermetic by construction: every leg runs with its own temp HOME, from a temp
# working dir (so the per-repo seed cannot fire), with XDG/CODEX/FACTORY/CLAUDE
# overrides scrubbed and a PATH farm that contains no host CLI unless the leg
# installs a stub. The ambient machine never decides a result.
#
#   bash flow/tests/adapters/global-install.sh

# pass/fail always return 0, so `cond && pass || fail` cannot misfire.
# shellcheck disable=SC2015
set -uo pipefail

_find_repo_root() {
  local d; d="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"
  while [[ "$d" != "/" ]]; do
    [[ -d "$d/docs/spec" || -d "$d/.spec" || -e "$d/.git" ]] && { printf '%s\n' "$d"; return 0; }
    d="$(dirname "$d")"
  done
  return 1
}
REPO_ROOT="$(_find_repo_root)" || { echo "cannot locate repo root" >&2; exit 1; }
INSTALL="$REPO_ROOT/install.sh"
TAG="global-install/71"

PASS=0 FAIL=0
pass() { echo "  PASS [$1] $2"; PASS=$((PASS + 1)); }
fail() { echo "  FAIL [$1] $2"; FAIL=$((FAIL + 1)); }
assert_contains()     { if [[ "$3" == *"$4"* ]]; then pass "$1" "$2"; else fail "$1" "$2"; echo "        want contains: $4"; fi; }
assert_not_contains() { if [[ "$3" != *"$4"* ]]; then pass "$1" "$2"; else fail "$1" "$2"; echo "        want NOT contains: $4"; fi; }
assert_eq()           { if [[ "$3" == "$4" ]]; then pass "$1" "$2"; else fail "$1" "$2"; echo "        want: $4"; echo "        got:  $3"; fi; }

tmp() { mktemp -d "${TMPDIR:-/tmp}/vibe-global.XXXXXX"; }

# PATH farm: a broad POSIX toolset, no host CLI. A leg that needs a host adds a
# stub with mkhost. $TMPDIR is stripped of its trailing slash (macOS sets one).
mkfarm() {
  local dir; dir="$(tmp)"
  local t p
  for t in bash sh mkdir dirname basename date mktemp mv cp rm rmdir sed grep head tail cat env awk find readlink ln chmod cmp diff sort cksum node git id uname touch; do
    p="$(command -v "$t" 2>/dev/null)" && ln -s "$p" "$dir/$t"
  done
  printf '%s\n' "$dir"
}
mkhost() {
  local dir="$1" h="$2"
  printf '#!/bin/sh\nexit 0\n' > "$dir/$h"
  chmod +x "$dir/$h"
  printf '%s\n' "$dir"
}

# content_hash DIR — a stable content hash of every file under DIR: relative
# path + cksum. mtimes are deliberately absent, because the contract is
# "a second run changes no file", not "touches nothing".
content_hash() {
  local dir="$1" f
  ( cd "$dir" && find . -type f | LC_ALL=C sort | while IFS= read -r f; do
      printf '%s %s\n' "$(cksum < "$f")" "$f"
    done )
}

# run_global HOME PATHSTR HOSTARG... — invoke the installer like a user would,
# with every ambient host override removed so the HOME under test is the only
# state in play. Prints combined output; returns the installer's status.
run_global() {
  local home="$1" pathstr="$2"; shift 2
  local work; work="$(tmp)"
  ( cd "$work" && env -u CLAUDE_CONFIG_DIR -u CODEX_HOME -u FACTORY_HOME \
      -u XDG_CONFIG_HOME -u CLAUDE_PROJECT_DIR \
      HOME="$home" PATH="$pathstr" bash "$INSTALL" "$@" 2>&1 )
  local rc=$?
  rm -rf "$work"
  return $rc
}

skill_dir() { # host -> expected absolute skills root under $1 (HOME)
  local home="$1" host="$2"
  case "$host" in
    claude)   printf '%s' "$home/.claude/skills" ;;
    codex)    printf '%s' "$home/.codex/skills" ;;
    droid)    printf '%s' "$home/.factory/skills" ;;
    opencode) printf '%s' "$home/.config/opencode/skills" ;;
  esac
}

echo "=== $TAG — --global installs each host's own skills, idempotently ==="

FARM="$(mkfarm)"; trap 'rm -rf "$FARM"' EXIT

# ── 1. one host at a time, one fresh HOME each ────────────────────────────────
for host in claude codex droid opencode; do
  H="$(tmp)"; dir="$(skill_dir "$H" "$host")"
  out="$(run_global "$H" "$FARM" --global --host "$host")"
  assert_eq "$TAG" "$host: install exits 0" "$?" "0"
  [[ -f "$dir/spec/SKILL.md" ]] && pass "$TAG" "$host: skills/spec/SKILL.md is installed" \
    || fail "$TAG" "$host: skills/spec/SKILL.md is installed ($dir)"
  [[ -f "$dir/vibe/SKILL.md" ]] && pass "$TAG" "$host: skills/vibe/SKILL.md is installed" \
    || fail "$TAG" "$host: skills/vibe/SKILL.md is installed ($dir)"
  # Exactly one host tree: --host must not fan out to the others.
  others="$(cd "$H" && find . -mindepth 1 -maxdepth 1 -type d | LC_ALL=C sort | tr '\n' ' ')"
  case "$host" in
    claude)   want="./.agents ./.claude " ;;
    codex)    want="./.agents ./.codex " ;;
    droid)    want="./.agents ./.factory " ;;
    opencode) want="./.agents ./.config " ;;
  esac
  assert_eq "$TAG" "$host: only that host's tree is written" "$others" "$want"
  # The shipped tree is the payload, not the contributor tree: no tests, no
  # half's own AGENTS.md, and never the source checkout's cursor.
  [[ ! -e "$dir/spec/tests" && ! -e "$dir/vibe/tests" && ! -e "$dir/vibe/engine/tests" ]] \
    && pass "$TAG" "$host: no tests/ dir rides along" || fail "$TAG" "$host: no tests/ dir rides along"
  [[ ! -e "$dir/vibe/state.json" ]] && pass "$TAG" "$host: the source cursor is not shipped" \
    || fail "$TAG" "$host: the source cursor is not shipped"
  rm -rf "$H"
done

# ── 2. idempotent: a second run changes no byte, and says so ──────────────────
H="$(tmp)"
run_global "$H" "$FARM" --global --host all >/dev/null 2>&1
before="$(content_hash "$H")"
out2="$(run_global "$H" "$FARM" --global --host all)"; rc2=$?
after="$(content_hash "$H")"
assert_eq "$TAG" "second --global run exits 0" "$rc2" "0"
assert_eq "$TAG" "second run changes no file content (hash of every file)" "$after" "$before"
assert_contains "$TAG" "second run reports 'already installed' for a skill" "$out2" "already installed: spec ->"
assert_contains "$TAG" "second run reports the block already installed" "$out2" "already installed: shared AGENTS.md block"
# ...and a third run is still stable (no drift from rewriting the block).
run_global "$H" "$FARM" --global --host all >/dev/null 2>&1
assert_eq "$TAG" "third run is byte-identical too" "$(content_hash "$H")" "$before"

# ── 3. the shared block: once, between markers, user prose untouched ─────────
AG="$H/.agents/AGENTS.md"
assert_eq "$TAG" "the shared AGENTS.md block appears exactly once" "$(grep -cF 'vibe:global:start' "$AG")" "1"
assert_eq "$TAG" "the block is closed" "$(grep -cF 'vibe:global:end' "$AG")" "1"
printf '# my own notes\nmine\n' > "$AG.prose"
cat "$AG.prose" "$AG" > "$AG.mixed" && mv -f "$AG.mixed" "$AG"
run_global "$H" "$FARM" --global --host claude >/dev/null 2>&1
assert_eq "$TAG" "user prose before the block survives a re-run" \
  "$(head -n 2 "$AG" | tr '\n' '|')" "# my own notes|mine|"
assert_eq "$TAG" "the block is still not duplicated" "$(grep -cF 'vibe:global:start' "$AG")" "1"
assert_eq "$TAG" "the block keeps its position after the user's prose" \
  "$(awk '/vibe:global:start/ {print NR; exit}' "$AG")" "3"
# Everything under the temp HOME belongs to the invoking user, never to root.
assert_eq "$TAG" "every installed file is owned by the invoking user" \
  "$(find "$H" ! -user "$(id -u)" | wc -l | tr -d ' ')" "0"
rm -rf "$H"

# ── 4. --host all in ONE home, and nothing written into the source checkout ───
H="$(tmp)"
git_before="$(git -C "$REPO_ROOT" status --porcelain 2>/dev/null; git -C "$REPO_ROOT" ls-files -o --exclude-standard 2>/dev/null)"
run_global "$H" "$FARM" --global --host all >/dev/null 2>&1
for host in claude codex droid opencode; do
  dir="$(skill_dir "$H" "$host")"
  [[ -f "$dir/spec/SKILL.md" && -f "$dir/vibe/SKILL.md" ]] \
    && pass "$TAG" "--host all installs $host" || fail "$TAG" "--host all installs $host"
done
git_after="$(git -C "$REPO_ROOT" status --porcelain 2>/dev/null; git -C "$REPO_ROOT" ls-files -o --exclude-standard 2>/dev/null)"
assert_eq "$TAG" "a global install writes nothing into the source checkout" "$git_after" "$git_before"
rm -rf "$H"

# ── 5. default detection: CLI on PATH, else nothing; never guessed ───────────
H="$(tmp)"
out="$(run_global "$H" "$FARM" --global)"
assert_contains "$TAG" "no host found: the plan says so instead of inventing one" "$out" "for no host"
[[ ! -d "$H/.claude" && ! -d "$H/.codex" && ! -d "$H/.factory" && ! -d "$H/.config" ]] \
  && pass "$TAG" "no host found: no host skills dir is created" || fail "$TAG" "no host found: no host skills dir is created"
[[ -f "$H/.agents/AGENTS.md" ]] \
  && pass "$TAG" "the shared block is written even with no host selected" \
  || fail "$TAG" "the shared block is written even with no host selected"
rm -rf "$H"

FARM_C="$(mkhost "$(mkfarm)" codex)"
H="$(tmp)"
run_global "$H" "$FARM_C" --global >/dev/null 2>&1
[[ -f "$H/.codex/skills/spec/SKILL.md" && -f "$H/.codex/skills/vibe/SKILL.md" ]] \
  && pass "$TAG" "detection: a codex CLI on PATH installs codex" \
  || fail "$TAG" "detection: a codex CLI on PATH installs codex"
[[ ! -d "$H/.claude" && ! -d "$H/.factory" && ! -d "$H/.config/opencode" ]] \
  && pass "$TAG" "detection: absent hosts are not installed" \
  || fail "$TAG" "detection: absent hosts are not installed"
rm -rf "$H" "$FARM_C"

# ── 6. dry-run writes nothing; the root guard is present ─────────────────────
H="$(tmp)"
out="$(run_global "$H" "$FARM" --global --host all --dry-run)"
assert_contains "$TAG" "--global --dry-run states the plan" "$out" "would install the spec skill"
assert_eq "$TAG" "--global --dry-run writes nothing under HOME" "$(find "$H" -mindepth 1 | wc -l | tr -d ' ')" "0"
rm -rf "$H"
grep -qF 'must not run as root' "$INSTALL" \
  && pass "$TAG" "the installer refuses a root --global (no root-owned files in \$HOME)" \
  || fail "$TAG" "the installer refuses a root --global (no root-owned files in \$HOME)"

echo ""
echo "=== results: $PASS passed, $FAIL failed ==="
[[ $FAIL -eq 0 ]]
