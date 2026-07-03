#!/usr/bin/env bash
# doctor.sh — one-command install health report (install-tooling/4).
#
#   doctor.sh [<repo-root>]   # default: the repo root resolved from this script
#
# Prints one line per check as "ok <check-id> <message>" or "warn <check-id>
# <message>" and ALWAYS exits 0 — a broken install must still be able to report
# on itself. Checks: core skills present; symlink (source) / real-dir (target)
# integrity; state machine parses; flow cursor validity (delegates
# validate-state.sh); Claude adapter wiring; every deps.json entry present on
# disk (warn + degrade text when absent); jq availability.
#
# Warn-only by design, so this drops -e (a failing check must not abort the run)
# and ends with an explicit exit 0.

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Resolve the repo root by upward marker search (never fixed hop counts — the
# script is reachable via the .agents/skills/vibe symlink and the canonical
# flow/ path; both must agree). A repo/install is marked by .spec, .git, or the
# .agents/skills tree an install lays down.
find_root() {
  local d="$1"
  while [[ -n "$d" && "$d" != "/" ]]; do
    if [[ -d "$d/.spec" || -e "$d/.git" || -d "$d/.agents/skills" ]]; then
      printf '%s\n' "$d"; return 0
    fi
    d="$(dirname "$d")"
  done
  return 1
}

ROOT="${1:-}"
if [[ -z "$ROOT" ]]; then
  ROOT="$(find_root "$SCRIPT_DIR")" || ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
fi

SPEC_SKILL="$ROOT/.agents/skills/spec"
VIBE_SKILL="$ROOT/.agents/skills/vibe"
MACHINE="$VIBE_SKILL/state-machine.json"
STATE="$VIBE_SKILL/state.json"
HOOKS="$ROOT/.claude/hooks/hooks.json"
PLUGIN="$ROOT/.claude-plugin/plugin.json"
DEPS="$VIBE_SKILL/reference/deps.json"

ok()   { printf 'ok   %s %s\n' "$1" "$2"; }
warn() { printf 'warn %s %s\n' "$1" "$2"; }
have_jq() { command -v jq >/dev/null 2>&1; }

note_header() { printf '# vibe doctor — %s\n' "$ROOT"; }

# ok when P is a real dir, or a symlink that resolves; warn when broken/absent.
check_link_or_dir() {
  local id="$1" p="$2"
  if [[ -L "$p" ]]; then
    if [[ -e "$p" ]]; then ok "$id" "$p -> $(readlink "$p") (symlink resolves)"
    else warn "$id" "$p -> $(readlink "$p") is a BROKEN symlink"; fi
  elif [[ -d "$p" ]]; then
    ok "$id" "$p is a real directory"
  else
    warn "$id" "$p is absent"
  fi
}

dep_present() {
  local n="$1"
  [[ -d "$HOME/.claude/skills/$n" ]] && return 0
  [[ -d "$HOME/.claude/plugins" ]] || return 1
  find "$HOME/.claude/plugins" -maxdepth 5 -iname "$n" -print -quit 2>/dev/null | grep -q .
}

note_header

# jq — several checks degrade without it.
if have_jq; then ok tool.jq "jq present ($(jq --version 2>/dev/null))"
else warn tool.jq "jq not installed — cursor + manifest checks degrade to unverified"; fi

# core skills present + integrity.
check_link_or_dir core.spec "$SPEC_SKILL"
check_link_or_dir core.vibe "$VIBE_SKILL"

# state machine parses.
if [[ -f "$MACHINE" ]]; then
  if have_jq && ! jq -e . "$MACHINE" >/dev/null 2>&1; then
    warn machine "state-machine.json is present but not valid JSON"
  else
    ok machine "state-machine.json present"
  fi
else
  warn machine "state-machine.json missing at $MACHINE — flow harness incomplete"
fi

# flow cursor: absent is normal (idle); present-but-invalid is a warn.
if [[ -f "$STATE" ]]; then
  if [[ -x "$VIBE_SKILL/scripts/validate-state.sh" ]] && bash "$VIBE_SKILL/scripts/validate-state.sh" >/dev/null 2>&1; then
    ok cursor "flow cursor valid ($(have_jq && jq -r '"\(.flow).\(.phase) feature=\(.feature // "none")"' "$STATE" 2>/dev/null || echo present))"
  else
    warn cursor "flow cursor present but invalid — run validate-state.sh (or reseed from state.example.json)"
  fi
else
  ok cursor "no flow cursor (idle) — normal when not mid-flow"
fi

# Claude adapter wiring (absent under a spec-only install).
if [[ -f "$HOOKS" ]]; then ok adapter.hooks "Claude hooks wired (.claude/hooks/hooks.json)"
else warn adapter.hooks "no .claude/hooks/hooks.json — Claude hooks not wired (spec-only install?)"; fi
if [[ -f "$PLUGIN" ]]; then ok adapter.plugin "plugin manifest present (.claude-plugin/plugin.json)"
else warn adapter.plugin "no .claude-plugin/plugin.json — plugin not registerable (spec-only install?)"; fi

# external dependency manifest + per-dep presence.
if [[ ! -f "$DEPS" ]]; then
  warn deps.manifest "deps.json missing at $DEPS"
elif ! have_jq; then
  warn deps.manifest "deps.json present but jq unavailable — cannot read dependency list"
elif ! jq -e . "$DEPS" >/dev/null 2>&1; then
  warn deps.manifest "deps.json is not valid JSON"
else
  ok deps.manifest "dependency manifest valid ($DEPS)"
  while IFS=$'\t' read -r name kind degrade; do
    [[ -n "$name" ]] || continue
    if dep_present "$name"; then ok "dep.$name" "$kind '$name' present on disk"
    else warn "dep.$name" "$kind '$name' not found — degrade: $degrade"; fi
  done < <(jq -r '.deps[] | [.name, .kind, .degrade] | @tsv' "$DEPS")
fi

exit 0
