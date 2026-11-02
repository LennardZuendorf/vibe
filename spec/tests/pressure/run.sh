#!/usr/bin/env bash
# vibe-spec: literal-file
# Pressure-tests the spec skill: fresh headless sessions with no CLAUDE.md, no user
# settings and no user plugins (empty CLAUDE_CONFIG_DIR). Org-managed plugins and the
# hooks of loaded plugins still run. A should-trigger row passes only if the first
# skill invoked — before any write — is in its accept set. A near-miss row (accept
# `none`) passes only if that first skill is neither `spec` nor `vibe`.
# The sandbox is seeded by `vibe-spec init` (binary from $VIBE_SPEC_BIN, else built
# with cargo); `--root legacy` seeds the flat legacy root with the frozen setup.sh.
# The model is $PRESSURE_MODEL (default claude-sonnet-5-5).
# Transcripts and stderr go to a timestamped folder under $PRESSURE_LOG_DIR
# (default .superpowers/p1/pressure).
#
# Usage: run.sh [--sp present|absent|both] [--root default|legacy]
#        run.sh --score <accept> <sp> <first-skill>   (scorer only; exit 0 PASS, 1 FAIL)
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO="$(cd "$HERE/../../.." && pwd)"

# score ACCEPT SP FIRST — prints PASS or FAIL; returns 0 on PASS.
score() {
  local accept="$1" sp="$2" first="$3"
  if [[ "$accept" == none ]]; then
    case "${first##*:}" in spec|vibe) echo FAIL; return 1 ;; esac
    echo PASS; return 0
  fi
  [[ "$sp" == absent ]] && accept="spec"
  if [[ "|$accept|" == *"|$first|"* ]]; then echo PASS; return 0; fi
  echo FAIL; return 1
}

if [[ "${1:-}" == "--score" ]]; then
  [[ $# -eq 4 ]] || { echo "pressure: --score needs <accept> <sp> <first-skill>" >&2; exit 2; }
  score "$2" "$3" "$4" && exit 0
  exit 1
fi

mode="both" root="default"
while [[ $# -gt 0 ]]; do
  case "$1" in
    --sp) [[ $# -ge 2 ]] || { echo "pressure: --sp needs present|absent|both" >&2; exit 2; }; mode="$2"; shift 2 ;;
    --root) [[ $# -ge 2 ]] || { echo "pressure: --root needs default|legacy" >&2; exit 2; }; root="$2"; shift 2 ;;
    *) echo "pressure: unknown argument '$1'" >&2; exit 2 ;;
  esac
done
case "$mode" in present|absent|both) ;; *) echo "pressure: bad --sp value '$mode'" >&2; exit 2 ;; esac
case "$root" in default|legacy) ;; *) echo "pressure: bad --root value '$root' (default|legacy)" >&2; exit 2 ;; esac
[[ -n "${ANTHROPIC_API_KEY:-}${CLAUDE_CODE_OAUTH_TOKEN:-}" ]] || { echo "pressure: set ANTHROPIC_API_KEY or CLAUDE_CODE_OAUTH_TOKEN (claude setup-token); not run" >&2; exit 2; }
command -v claude >/dev/null 2>&1 || { echo "pressure: claude CLI not found on PATH; not run" >&2; exit 2; }

VSB="${VIBE_SPEC_BIN:-}"
if [[ -z "$VSB" ]]; then
  (cd "$REPO" && cargo build -q -p vibe-spec) || { echo "pressure: cargo build -p vibe-spec failed; set VIBE_SPEC_BIN; not run" >&2; exit 2; }
  VSB="$REPO/target/debug/vibe-spec"
fi
[[ -x "$VSB" ]] || { echo "pressure: vibe-spec binary '$VSB' is not executable; not run" >&2; exit 2; }

SP_DIR="${SUPERPOWERS_DIR:-}"
if [[ "$mode" != absent ]]; then
  if [[ -z "$SP_DIR" ]]; then
    SP_DIR="$(printf '%s\n' "$HOME"/.claude/plugins/cache/claude-plugins-official/superpowers/*/ | sort -V | tail -1)"
  fi
  [[ -d "$SP_DIR" ]] || { echo "pressure: superpowers plugin dir not found ('$SP_DIR'); set SUPERPOWERS_DIR or use --sp absent; not run" >&2; exit 2; }
  echo "pressure: superpowers dir: $SP_DIR"
fi

MODEL="${PRESSURE_MODEL:-claude-sonnet-5-5}"
LOG_DIR="${PRESSURE_LOG_DIR:-$REPO/.superpowers/p1/pressure}/$(date +%Y%m%dT%H%M%S)-$root-$mode"
if ! mkdir -p "$LOG_DIR"; then
  echo "pressure: cannot create log dir '$LOG_DIR'; set PRESSURE_LOG_DIR; not run" >&2; exit 2
fi
LOG_DIR="$(cd "$LOG_DIR" && pwd)"
echo "pressure: model $MODEL; root $root; logs $LOG_DIR"

SANDBOX=""
trap '[[ -z "$SANDBOX" ]] || rm -rf "$SANDBOX"' EXIT
trap 'exit 130' INT TERM

make_sandbox() { # populates dir $1: proj/ plugin/ config/
  local d="$1"
  mkdir -p "$d/proj/src" "$d/plugin/.claude-plugin" "$d/plugin/skills" "$d/config"
  printf '#!/usr/bin/env bash\necho "Helo"\n' >"$d/proj/src/cli.sh"
  printf '{"name":"spec-under-test","version":"0.0.0"}\n' >"$d/plugin/.claude-plugin/plugin.json"
  ln -s "$REPO/spec" "$d/plugin/skills/spec"
  (cd "$d/proj" && git init -q .) || true
  case "$root" in
    default) (cd "$d/proj" && "$VSB" init >/dev/null 2>&1) || true ;;
    legacy) (cd "$d/proj" && bash "$REPO/spec/scripts/setup.sh" >/dev/null 2>&1) || true ;;
  esac
  (cd "$d/proj" && "$VSB" root >/dev/null 2>&1) || { echo "pressure: sandbox has no spec root after the $root seed (vibe-spec root failed); not run" >&2; exit 2; }
}
claude_iso() { # cfg plugin-args... prompt
  local cfg="$1"; shift
  CLAUDE_CONFIG_DIR="$cfg" claude -p "$@" --model "$MODEL" --permission-mode plan --output-format stream-json --verbose </dev/null
}

pass=0 total=0
run_one() { # id accept prompt sp
  local id="$1" accept="$2" prompt="$3" sp="$4" d first log verdict args=()
  d="$(mktemp -d)"; SANDBOX="$d"; make_sandbox "$d"
  args=(--plugin-dir "$d/plugin")
  [[ "$sp" == present ]] && args+=(--plugin-dir "$SP_DIR")
  log="$LOG_DIR/$id-$sp"
  (cd "$d/proj" && claude_iso "$d/config" ${args[@]+"${args[@]}"} "$prompt" >"$log.jsonl" 2>"$log.err") ||
    echo "pressure: claude exited non-zero for scenario $id ($sp); see $log.err" >&2
  if ! first="$(node "$HERE/parse.mjs" <"$log.jsonl")"; then
    echo "pressure: parse failed for scenario $id ($sp); counting first-skill=none" >&2
    first="none"
  fi
  rm -rf "$d"; SANDBOX=""; total=$((total + 1))
  if verdict="$(score "$accept" "$sp" "$first")"; then pass=$((pass + 1)); fi
  if [[ "$accept" == none ]]; then echo "$verdict $id $sp first-skill=$first (near-miss)"
  elif [[ "$verdict" == PASS ]]; then echo "PASS $id $sp first-skill=$first"
  else echo "FAIL $id $sp first-skill=$first (accept: $accept)"; fi
}

sps=(present absent); [[ "$mode" != both ]] && sps=("$mode")
while IFS=$'\t' read -r id accept prompt; do
  [[ "$id" == id ]] && continue
  for sp in ${sps[@]+"${sps[@]}"}; do run_one "$id" "$accept" "$prompt" "$sp"; done
done <"$HERE/scenarios.tsv"
[[ $total -ge 6 ]] || { echo "pressure: only $total scenarios ran (floor 6)"; exit 1; }
echo "pressure: $pass/$total passed"
[[ $pass -eq $total ]]
