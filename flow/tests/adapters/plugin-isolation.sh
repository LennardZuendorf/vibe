#!/usr/bin/env bash
# vibe-spec: literal-file
# plugin-install/71: inherited host config paths must never escape test homes.
# These CLI doubles prove environment isolation; plugin-install.sh still owns
# the separate real-host cache and committed-package assertions.
set -euo pipefail

_find_repo_root() {
  local d; d="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"
  while [[ "$d" != / ]]; do
    [[ -e "$d/.git" ]] && { printf '%s\n' "$d"; return 0; }
    d="$(dirname "$d")"
  done
  return 1
}
ROOT="$(_find_repo_root)"
WORK="$(mktemp -d "${TMPDIR:-/tmp}/vibe-plugin-isolation.XXXXXX")"
trap 'rm -rf "$WORK"' EXIT
PASS=0 FAIL=0
check() {
  if "$@"; then PASS=$((PASS + 1)); else
    echo "  FAIL [plugin-install/71] $*"; FAIL=$((FAIL + 1))
  fi
}

# A hermetic PATH cannot accidentally select a real host, even on failure legs.
for scenario in success marketplace-failure install-failure missing bad-cache; do
  probe="$WORK/$scenario"; mkdir -p "$probe/bin" "$probe/outside"
  for tool in bash sh env git mkdir rm sed grep head tail find sort cmp tr dirname basename mktemp cat cp touch; do
    ln -s "$(command -v "$tool")" "$probe/bin/$tool"
  done
  printf '%s\n' "$scenario" > "$probe/scenario"
  printf 'untouched\n' > "$probe/outside/sentinel"
  if [[ "$scenario" != missing ]]; then
    cat > "$probe/bin/host" <<'SHIM'
#!/usr/bin/env bash
set -euo pipefail
probe="$(cd "$(dirname "$0")/.." && pwd -P)"
host="$(basename "$0")"
scenario="$(cat "$probe/scenario")"
printf '%s %s\n' "$host" "$*" >> "$probe/calls"
env | sort > "$probe/env-$host-$3"
case "$host" in
  claude) config="${CLAUDE_CONFIG_DIR:-$HOME/.claude}" ;;
  codex) config="${CODEX_HOME:-$HOME/.codex}" ;;
  droid) config="${FACTORY_HOME_OVERRIDE:-$HOME}/.factory" ;;
esac
# Model a host's real write before checking the boundary: the RED run must
# expose an external side effect, rather than merely assert its own fixture.
[[ ! -f "$config/sentinel" ]] || cat "$config/sentinel" >> "$probe/reads"
mkdir -p "$config"
touch "$config/probe-touched"
case "$config" in "$HOME"/*) ;; *) echo 'isolation violation: inherited host config' >&2; exit 1 ;; esac
for name in CODEX_HOME CLAUDE_CONFIG_DIR FACTORY_CONFIG_HOME XDG_CONFIG_HOME XDG_DATA_HOME XDG_STATE_HOME XDG_CACHE_HOME TMPDIR; do
  value="${!name:-}"
  case "$value" in "$HOME"/*) ;; *) echo "isolation violation: $name" >&2; exit 1 ;; esac
done
[[ "${FACTORY_HOME_OVERRIDE:-$HOME}" == "$HOME" ]] || { echo 'isolation violation: FACTORY_HOME_OVERRIDE' >&2; exit 1; }
[[ "$PWD" == "$HOME" ]] || { echo 'isolation violation: cwd' >&2; exit 1; }
if env | grep -Eq '^(VIBE_|CLAUDE_PROJECT_DIR=|CLAUDE_PLUGIN_ROOT=|CODEX_CONFIG=|FACTORY_API_KEY=|FACTORY_PROFILE_DIR=|FACTORY_RUNTIME_SETTINGS_PATH=|ANTHROPIC_API_KEY=|OPENAI_API_KEY=|GIT_CONFIG_GLOBAL=)'; then
  echo 'isolation violation: inherited override or credential' >&2; exit 1
fi
if [[ "$2" == marketplace ]]; then
  [[ "$3" == add && "$#" == 4 ]] || exit 2
  [[ "$scenario" != marketplace-failure ]] || { echo 'Error: test marketplace refusal' >&2; exit 1; }
  printf '%s\n' "$4" > "$probe/source-$host"
  # Droid names a local marketplace after its source directory, regardless
  # of the manifest name. Its later install must use that registered name.
  [[ "$host" != droid ]] || basename "$4" > "$probe/droid-marketplace"
else
  [[ "$3" == vibe@vibe ]] || exit 2
  case "$host:$2:$#" in claude:install:3|codex:add:3|droid:install:5) ;; *) exit 2 ;; esac
  if [[ "$host" == droid && "$(cat "$probe/droid-marketplace")" != vibe ]]; then
    echo 'Error: Marketplace "vibe" not found' >&2; exit 1
  fi
  [[ "$scenario" != install-failure ]] || { echo 'Error: test install refusal' >&2; exit 1; }
  source="$(cat "$probe/source-$host")"
  version="$(sed -n 's/.*"version"[[:space:]]*:[[:space:]]*"\([^" ]*\)".*/\1/p' "$source/package.json" | head -n1)"
  for pair in spec:spec vibe:flow; do
    skill="${pair%%:*}"; canon="${pair#*:}"
    if [[ "$host" == droid ]]; then
      revision="$(git -C "$source" rev-parse --short=12 HEAD)"
      cache="$config/plugins/cache/vibe-fixture/vibe-fixture/$revision/skills/$skill"
    else
      cache="$config/plugins/cache/vibe/vibe/$version/skills/$skill"
    fi
    mkdir -p "$cache"
    cp "$source/$canon/SKILL.md" "$cache/SKILL.md"
    [[ "$scenario" != bad-cache ]] || printf 'corrupt\n' > "$cache/SKILL.md"
  done
fi
SHIM
    chmod +x "$probe/bin/host"
    for host in claude codex droid; do ln -s host "$probe/bin/$host"; done
  fi
  # Adversarial values remain inside this fixture so even the known-bad
  # implementation can be exercised safely before any live-host invocation.
  rc=0
  env -i PATH="$probe/bin" HOME="$probe/outside" TMPDIR="$WORK" \
    CODEX_HOME="$probe/outside" CLAUDE_CONFIG_DIR="$probe/outside" \
    FACTORY_CONFIG_HOME="$probe/outside" FACTORY_HOME_OVERRIDE="$probe/outside" \
    FACTORY_PROFILE_DIR="$probe/outside" FACTORY_RUNTIME_SETTINGS_PATH="$probe/outside" XDG_CONFIG_HOME="$probe/outside" \
    XDG_DATA_HOME="$probe/outside" XDG_STATE_HOME="$probe/outside" \
    XDG_CACHE_HOME="$probe/outside" VIBE_ENGINE=adversarial VIBE_SPEC_ROOT="$probe/outside" \
    CLAUDE_PROJECT_DIR="$probe/outside" CLAUDE_PLUGIN_ROOT="$probe/outside" \
    CODEX_CONFIG="$probe/outside" FACTORY_API_KEY=fake ANTHROPIC_API_KEY=fake \
    OPENAI_API_KEY=fake GIT_CONFIG_GLOBAL="$probe/outside/nonexistent" \
    bash "$ROOT/flow/tests/adapters/plugin-install.sh" --require-host > "$probe/output" 2>&1 || rc=$?
  check test ! -e "$probe/outside/probe-touched"
  check test ! -e "$probe/reads"
  check test "$(cat "$probe/outside/sentinel")" = untouched
  case "$scenario" in
    success)
      check test "$rc" = 0
      check grep -q '11 passed, 0 failed, 0 skipped' "$probe/output"
      ;;
    marketplace-failure)
      check test "$rc" = 1
      check grep -q '3 failed, 1 skipped' "$probe/output"
      ;;
    install-failure)
      check test "$rc" = 1
      check grep -q '4 failed, 0 skipped' "$probe/output"
      ;;
    missing)
      check test "$rc" = 1
      check grep -q '1 failed, 3 skipped' "$probe/output"
      ;;
    bad-cache)
      check test "$rc" = 1
      check grep -q '6 failed, 0 skipped' "$probe/output"
      ;;
  esac
  if [[ "$scenario" != missing ]]; then
    for host in claude codex droid; do check grep -q "^$host plugin marketplace add " "$probe/calls"; done
    if [[ "$scenario" != marketplace-failure ]]; then
      for host in claude codex droid; do check grep -Eq "^$host plugin (add|install) vibe@vibe" "$probe/calls"; done
    fi
  fi
  if [[ "$FAIL" -gt 0 ]]; then cat "$probe/output"; fi
  echo "  exercised $scenario"
done
printf '=== isolation: %s passed, %s failed ===\n' "$PASS" "$FAIL"
[[ "$FAIL" -eq 0 ]]
