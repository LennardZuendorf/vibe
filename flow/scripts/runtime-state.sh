#!/usr/bin/env bash
# Migration helper: prefer the new cursor once it exists; never erase the old one.
set -euo pipefail

runtime_project_root() {
  local d="${CLAUDE_PROJECT_DIR:-$1}"
  if [[ -n "${CLAUDE_PROJECT_DIR:-}" ]]; then printf '%s\n' "$d"; return; fi
  while [[ "$d" != / ]]; do
    if [[ -d "$d/docs/spec" || -d "$d/.spec" || -e "$d/.git" || -d "$d/.vibe" ]]; then # literal
      printf '%s\n' "$d"; return
    fi
    d="$(dirname "$d")"
  done
  return 1
}

# 0: regular entry (readable unless metadata mode); 1: absent; 2: invalid chain.
# Check each parent before interpreting a failed child probe as absence.
# Keep this body identical to the standalone permanent Stop classifier.
runtime_cursor_status() {
  local target="$1" access="${2:-read}" current=/ rest part
  [[ "$target" == /* ]] || target="$PWD/$target"
  rest="${target#/}"
  [[ -d "$current" && -x "$current" ]] || return 2
  while [[ "$rest" == */* ]]; do
    part="${rest%%/*}"; rest="${rest#*/}"
    [[ -n "$part" ]] || continue
    current="${current%/}/$part"
    if [[ -e "$current" || -L "$current" ]]; then
      [[ -d "$current" && -x "$current" ]] || return 2
    else
      return 1
    fi
  done
  current="${current%/}/$rest"
  if [[ -e "$current" || -L "$current" ]]; then
    [[ -f "$current" ]] || return 2
    [[ "$access" == metadata || -r "$current" ]] || return 2
    return 0
  fi
  return 1
}

runtime_state_path() {
  local legacy="$1/state.json" root="${2:-}" candidate
  [[ -n "$root" ]] || root="$(runtime_project_root "$1")" || { printf '%s\n' "$legacy"; return; }
  candidate="$root/.vibe/run/flow/state.json"
  local status=0
  runtime_cursor_status "$candidate" || status=$?
  if [[ "$status" == 1 ]]; then printf '%s\n' "$legacy"
  else
    [[ "$status" != 2 ]] || printf 'cursor: WARN — present but invalid: %s\n' "$candidate" >&2
    printf '%s\n' "$candidate"
  fi
}

runtime_check_write() {
  local root="$1" target="$2" resolved parent count=0
  root="$(cd "$root" && pwd -P)" || return 1
  while :; do
    count=$((count + 1)); [[ "$count" -le 40 ]] || return 1
    [[ ! -d "$target" ]] || return 1
    parent="$(cd "$(dirname "$target")" && pwd -P)" || return 1
    case "$parent/" in "$root/"*) ;; *) return 1 ;; esac
    resolved="$parent/${target##*/}"
    [[ -L "$resolved" ]] || return 0
    target="$(readlink "$resolved")" || return 1
    [[ "$target" == /* ]] || target="$parent/$target"
    [[ -e "$target" || -L "$target" ]] || return 1
  done
}

# Exactly one JSON object is required to classify persisted cursor metadata.
# Unknown integrity/parser availability never proves an unadopted cursor.
runtime_cursor_kind() {
  local file="$1" kind=""
  if [[ ! -e "$file" && ! -L "$file" ]]; then printf 'absent\n'; return; fi
  if [[ ! -f "$file" || -L "$file" ]]; then printf 'invalid\n'; return; fi
  if command -v node >/dev/null 2>&1; then
    kind="$(node -e 'const fs=require("node:fs");let v;try{v=JSON.parse(fs.readFileSync(process.argv[1],"utf8"));}catch{console.log("invalid");process.exit(0);}console.log(v===null||typeof v!=="object"||Array.isArray(v)?"invalid":Object.hasOwn(v,"v")?"versioned":"legacy");' "$file" 2>/dev/null)" || kind=invalid
  elif command -v jq >/dev/null 2>&1; then
    kind="$(jq -sr 'if length != 1 or (.[0] | type) != "object" then "invalid" elif (.[0] | has("v")) then "versioned" else "legacy" end' "$file" 2>/dev/null)" || kind=invalid
  else
    kind=unavailable
  fi
  case "$kind" in absent|invalid|versioned|legacy|unavailable) printf '%s\n' "$kind" ;; *) printf 'invalid\n' ;; esac
}
