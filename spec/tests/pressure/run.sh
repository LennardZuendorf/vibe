#!/usr/bin/env bash
# Pressure-tests the spec skill: fresh headless sessions with no CLAUDE.md,
# no user settings, no hooks (empty CLAUDE_CONFIG_DIR). Passes only if the first skill
# invoked — before any write — is in the scenario's accept set.
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO="$(cd "$HERE/../../.." && pwd)"
mode="both"
if [[ "${1:-}" == "--sp" ]]; then
  [[ $# -ge 2 ]] || { echo "pressure: --sp needs present|absent|both" >&2; exit 2; }
  mode="$2"
fi
case "$mode" in present|absent|both) ;; *) echo "pressure: bad --sp value '$mode'" >&2; exit 2 ;; esac
[[ -n "${ANTHROPIC_API_KEY:-}${CLAUDE_CODE_OAUTH_TOKEN:-}" ]] || { echo "pressure: set ANTHROPIC_API_KEY or CLAUDE_CODE_OAUTH_TOKEN (claude setup-token); not run" >&2; exit 2; }
command -v claude >/dev/null 2>&1 || { echo "pressure: claude CLI not found on PATH; not run" >&2; exit 2; }

SP_DIR="${SUPERPOWERS_DIR:-}"
if [[ "$mode" != absent ]]; then
  if [[ -z "$SP_DIR" ]]; then
    SP_DIR="$(printf '%s\n' "$HOME"/.claude/plugins/cache/claude-plugins-official/superpowers/*/ | sort -V | tail -1)"
  fi
  [[ -d "$SP_DIR" ]] || { echo "pressure: superpowers plugin dir not found ('$SP_DIR'); set SUPERPOWERS_DIR or use --sp absent; not run" >&2; exit 2; }
  echo "pressure: superpowers dir: $SP_DIR"
fi

SANDBOX=""
trap '[[ -z "$SANDBOX" ]] || rm -rf "$SANDBOX"' EXIT

make_sandbox() { # populates dir $1: proj/ plugin/ config/
  local d="$1"
  mkdir -p "$d/proj/src" "$d/plugin/.claude-plugin" "$d/plugin/skills" "$d/config"
  printf '#!/usr/bin/env bash\necho "Helo"\n' >"$d/proj/src/cli.sh"
  printf '{"name":"spec-under-test","version":"0.0.0"}\n' >"$d/plugin/.claude-plugin/plugin.json"
  ln -s "$REPO/spec" "$d/plugin/skills/spec"
  (cd "$d/proj" && git init -q . && bash "$REPO/spec/scripts/setup.sh" >/dev/null 2>&1) || true
  [[ -d "$d/proj/.spec" ]] || { echo "pressure: sandbox setup (spec/scripts/setup.sh) did not create .spec/; not run" >&2; exit 2; }
}
claude_iso() { # cfg plugin-args... prompt
  local cfg="$1"; shift
  CLAUDE_CONFIG_DIR="$cfg" claude -p "$@" --permission-mode plan --output-format stream-json --verbose
}

pass=0 total=0
run_one() { # id accept prompt sp
  local id="$1" accept="$2" prompt="$3" sp="$4" d first args=()
  [[ "$sp" == absent ]] && accept="spec"
  d="$(mktemp -d)"; SANDBOX="$d"; make_sandbox "$d"
  args=(--plugin-dir "$d/plugin")
  [[ "$sp" == present ]] && args+=(--plugin-dir "$SP_DIR")
  if ! first="$(cd "$d/proj" && claude_iso "$d/config" "${args[@]}" "$prompt" 2>/dev/null | node "$HERE/parse.mjs")"; then
    echo "pressure: claude/parse failed for scenario $id ($sp); counting first-skill=none" >&2
    first="none"
  fi
  rm -rf "$d"; SANDBOX=""; total=$((total + 1))
  if [[ "|$accept|" == *"|$first|"* ]]; then pass=$((pass + 1)); echo "PASS $id $sp first-skill=$first"
  else echo "FAIL $id $sp first-skill=$first (accept: $accept)"; fi
}

sps=(present absent); [[ "$mode" != both ]] && sps=("$mode")
while IFS=$'\t' read -r id accept prompt; do
  [[ "$id" == id ]] && continue
  for sp in "${sps[@]}"; do run_one "$id" "$accept" "$prompt" "$sp"; done
done <"$HERE/scenarios.tsv"
[[ $total -ge 6 ]] || { echo "pressure: only $total scenarios ran (floor 6)"; exit 1; }
echo "pressure: $pass/$total passed"
[[ $pass -eq $total ]]
