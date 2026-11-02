#!/usr/bin/env bash
# vibe-spec: literal-file
# flow/tests/adapters/plugin-install.sh — vibe#71 end-to-end host install.
#
# `codex plugin add` DROPS a symlinked skill dir from its per-user cache and
# `droid plugin install` leaves skills/ empty, so the Codex and Factory packages
# ship REAL skill trees materialized by build-plugin.sh. This leg proves the
# consequence end to end rather than by inspection: install every package into a
# throwaway HOME and read the HOST's own cache.
#
# The source is a CLONE, never the worktree: what a user installs is the
# committed payload, so a tree built but not committed must fail here.
#
# Skips loudly when a host CLI is absent (CI installs `claude` and `codex` for
# this leg and asserts they were used with --require-host; `droid` refuses every
# marketplace source for the CI-less sandbox's credential — see the SKIP below).
#
#   bash flow/tests/adapters/plugin-install.sh                  # skip-aware
#   bash flow/tests/adapters/plugin-install.sh --require-host    # a skip is a failure
#
# shellcheck disable=SC2015

set -uo pipefail

_find_repo_root() {
  local d; d="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"
  while [[ "$d" != "/" ]]; do
    [[ -d "$d/.spec" || -e "$d/.git" ]] && { printf '%s\n' "$d"; return 0; }
    d="$(dirname "$d")"
  done
  return 1
}
REPO_ROOT="$(_find_repo_root)" || { echo "cannot locate repo root (.spec/.git)" >&2; exit 1; }

PASS=0 FAIL=0 SKIP=0
pass() { echo "  PASS [$1] $2"; PASS=$((PASS + 1)); }
fail() { echo "  FAIL [$1] $2"; FAIL=$((FAIL + 1)); }
skip() { echo "  SKIP [$1] $2"; SKIP=$((SKIP + 1)); }
assert_eq() { if [[ "$3" == "$4" ]]; then pass "$1" "$2"; else fail "$1" "$2"; echo "        want: $4"; echo "        got: $3"; fi; }

REQUIRE_HOST=0
[[ "${1:-}" == "--require-host" ]] && REQUIRE_HOST=1
TAG="plugin-install/71"

echo "=== $TAG — a clean clone installs real skills into each host cache ==="

VER="$(sed -n 's/.*"version"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' "$REPO_ROOT/package.json" | head -n1)"
assert_eq "$TAG" "precondition: package.json declares a version" "$([[ -n "$VER" ]] && echo yes || echo no)" "yes"

WORK="$(mktemp -d "${TMPDIR:-/tmp}/vibe-plugin-install.XXXXXX")" || exit 1
trap 'rm -rf "$WORK"' EXIT
CLONE="$WORK/clone"

if git clone -q "$REPO_ROOT" "$CLONE" 2>"$WORK/clone.err"; then
  pass "$TAG" "a clean clone of the repo under test"
else
  fail "$TAG" "a clean clone of the repo under test"
  echo "        $(tail -n1 "$WORK/clone.err")"
  echo ""
  echo "=== results: $PASS passed, $FAIL failed, $SKIP skipped ==="
  exit 1
fi
# A CI checkout is a detached HEAD, whose clone has no branch to land on. Pin the
# clone to the exact commit under test so the installed payload is that commit.
git -C "$CLONE" -c advice.detachedHead=false checkout -q --detach "$(git -C "$REPO_ROOT" rev-parse HEAD)" 2>/dev/null || true

# The payload property both drops depend on: real skill dirs, zero links.
PI_LINKS="$(find "$CLONE/plugins" "$CLONE/factory-plugin" -type l 2>/dev/null || true)"
assert_eq "$TAG" "the clone's shipped packages carry no symlink at all" "$PI_LINKS" ""
for _pkg in plugins/vibe factory-plugin; do
  _ok=1
  for _s in spec vibe; do
    [[ -f "$CLONE/$_pkg/skills/$_s/SKILL.md" && ! -L "$CLONE/$_pkg/skills/$_s" ]] || _ok=0
  done
  assert_eq "$TAG" "$_pkg/skills/{spec,vibe} are real dirs carrying SKILL.md" "$_ok" "1"
done

# verify_cache HOST HOMEDIR — the host's cache must hold both SKILL.md files as
# real files byte-identical to the clone's canonical half. Matching on
# "/vibe/$VER/" is what pins the manifest version: a version-less Codex manifest
# makes the cache dir read 1.0.0 and this finds nothing.
verify_cache() {
  local host="$1" home="$2" name canon got all found
  all="$(find "$home" -type f -path '*/plugins/cache/*' -name SKILL.md 2>/dev/null | sort)"
  for pair in "spec:spec" "vibe:flow"; do
    name="${pair%%:*}" canon="${pair#*:}"
    got="$(printf '%s\n' "$all" | grep -E "/skills/$name/SKILL\.md$" | grep -F "/vibe/$VER/" | head -n1)"
    found="$got"
    [[ -n "$found" && ! -L "$found" ]] && cmp -s "$found" "$CLONE/$canon/SKILL.md" || found=""
    if [[ -n "$found" ]]; then
      pass "$TAG" "$host cache holds skills/$name/SKILL.md (v$VER, byte-identical to $canon/)"
    else
      fail "$TAG" "$host cache holds skills/$name/SKILL.md (v$VER, byte-identical to $canon/)"
      echo "        got: ${got:-<no skills/$name/SKILL.md under $home>}"
    fi
  done
}

RAN=0
# ── Claude: the host that already dereferenced symlinks — the control ────────
if ! command -v claude >/dev/null 2>&1; then
  skip "$TAG" "claude CLI not on PATH — cache not exercised here"
else
  H="$WORK/home-claude"; mkdir -p "$H"
  if HOME="$H" claude plugin marketplace add "$CLONE" >"$WORK/claude.log" 2>&1 \
     && HOME="$H" claude plugin install vibe@vibe >>"$WORK/claude.log" 2>&1; then
    RAN=$((RAN + 1)); verify_cache claude "$H"
  else
    fail "$TAG" "claude plugin install vibe@vibe"
    echo "        $(tail -n2 "$WORK/claude.log" | tr '\n' ' ')"
  fi
fi

# ── Codex: the host that dropped the symlink (the bug) ───────────────────────
if ! command -v codex >/dev/null 2>&1; then
  skip "$TAG" "codex CLI not on PATH — cache not exercised here"
else
  H="$WORK/home-codex"; mkdir -p "$H"
  if HOME="$H" codex plugin marketplace add "$CLONE" >"$WORK/codex.log" 2>&1 \
     && HOME="$H" codex plugin add vibe@vibe >>"$WORK/codex.log" 2>&1; then
    RAN=$((RAN + 1)); verify_cache codex "$H"
  else
    fail "$TAG" "codex plugin add vibe@vibe"
    echo "        $(grep -v 'PATH aliases' "$WORK/codex.log" | tail -n2 | tr '\n' ' ')"
  fi
fi

# ── Factory Droid: same drop, but its marketplace refuses this credential ────
if ! command -v droid >/dev/null 2>&1; then
  skip "$TAG" "droid CLI not on PATH — cache not exercised here"
elif ! HOME="$WORK/home-droid" droid plugin marketplace add "$CLONE" >"$WORK/droid.log" 2>&1; then
  skip "$TAG" "droid refuses every marketplace source here ($(grep -m1 -E 'Error|error' "$WORK/droid.log" || head -n1 "$WORK/droid.log")) — payload property asserted above instead"
else
  H="$WORK/home-droid"; mkdir -p "$H"
  if HOME="$H" droid plugin install vibe@vibe --scope user >"$WORK/droid.log" 2>&1; then
    RAN=$((RAN + 1)); verify_cache droid "$H"
  else
    fail "$TAG" "droid plugin install vibe@vibe --scope user"
    echo "        $(tail -n2 "$WORK/droid.log" | tr '\n' ' ')"
  fi
fi

# A leg that installed nothing proves nothing, so CI says so out loud.
if [[ "$RAN" -eq 0 ]]; then
  if [[ "$REQUIRE_HOST" -eq 1 ]]; then
    fail "$TAG" "at least one host CLI really installed the plugin (--require-host)"
  else
    skip "$TAG" "no host cache exercised on this machine (pass --require-host to make that fatal)"
  fi
fi

echo ""
echo "=== results: $PASS passed, $FAIL failed, $SKIP skipped ==="
[[ $FAIL -eq 0 ]]
