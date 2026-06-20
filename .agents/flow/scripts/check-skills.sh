#!/usr/bin/env bash
# check-skills.sh — detect-and-warn skill availability for a flow state (OPEN-6,
# vibe-flow/3). The flow delegates to external skills (`superpowers:*`), the
# bundled `spec` skill, and feature-dev subagents. A missing delegate must WARN
# and degrade, never hard-fail the session.
#
#   check-skills.sh                 # check delegates of the current cursor state
#   check-skills.sh <flow.phase>    # check delegates of an explicit state
#   check-skills.sh caveman <level> # print the 1-line caveman fallback definition
#                                   #   (used when the upstream caveman skill is absent)
#
# A bash script cannot introspect Claude Code's installed-skill registry, so this
# verifies what is checkable on disk (the bundled `spec` skill, `vibe-*` shims) and
# flags the rest as external/assumed-installed. Always exits 0.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
FLOW_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
AGENTS_DIR="$(cd "$FLOW_DIR/.." && pwd)"
MACHINE="$FLOW_DIR/state-machine.json"
STATE="$FLOW_DIR/state.json"

warn() { echo "check-skills: WARN — $1" >&2; }
note() { echo "check-skills: $1"; }

have_jq() { command -v jq >/dev/null 2>&1; }

current_state() {
  if have_jq && [[ -f "$STATE" ]] && jq -e . "$STATE" >/dev/null 2>&1; then
    local flow phase
    flow=$(jq -r '.flow // "idle"' "$STATE")
    phase=$(jq -r '.phase // "idle"' "$STATE")
    if [[ "$flow" == "$phase" ]]; then echo "$flow"; else echo "$flow.$phase"; fi
  else
    echo "idle"
  fi
}

# ── caveman fallback ─────────────────────────────────────────────────────────
# When the upstream caveman skill is not installed, the flow still names a level;
# emit its 1-line definition straight from the machine so the agent can apply it.
caveman_fallback() {
  local level="${1:-}"
  if [[ -z "$level" ]]; then
    echo "usage: check-skills.sh caveman <lite|full|ultra>" >&2
    return 0
  fi
  if have_jq && [[ -f "$MACHINE" ]]; then
    local def
    def=$(jq -r --arg l "$level" '.caveman_levels[$l] // empty' "$MACHINE" 2>/dev/null)
    if [[ -n "$def" ]]; then
      echo "caveman[$level]: $def"
      return 0
    fi
  fi
  # Hard-coded floor if the machine is unreadable.
  case "$level" in
    lite)  echo "caveman[lite]: No filler or hedging; full sentences." ;;
    full)  echo "caveman[full]: Drop articles; fragments OK; short synonyms." ;;
    ultra) echo "caveman[ultra]: Arrows (X -> Y); one word where one does." ;;
    *)     echo "caveman[$level]: output compression only; never reasoning depth." ;;
  esac
}

# ── skill availability for a state ─────────────────────────────────────────────
check_state() {
  local state="${1:-$(current_state)}"

  if ! have_jq; then
    warn "jq not installed — cannot read delegates; assuming all external skills present."
    return 0
  fi
  if [[ ! -f "$MACHINE" ]]; then
    warn "state-machine.json not found at $MACHINE; skipping skill check."
    return 0
  fi

  local delegates
  delegates=$(jq -r --arg s "$state" '.states[$s].delegates[]? // empty' "$MACHINE" 2>/dev/null)
  if [[ -z "$delegates" ]]; then
    note "state '$state' delegates to no external skills."
    return 0
  fi

  local missing_external=0
  while IFS= read -r d; do
    [[ -z "$d" ]] && continue
    case "$d" in
      spec)
        if [[ -f "$AGENTS_DIR/skills/spec/SKILL.md" ]]; then
          note "ok: bundled skill 'spec' present."
        else
          warn "bundled skill 'spec' missing at $AGENTS_DIR/skills/spec — flow degrades to inline guidance."
        fi
        ;;
      superpowers:*)
        warn "external skill '$d' is assumed-installed (cannot verify from shell). If absent, follow the inline orders without it."
        missing_external=$((missing_external + 1))
        ;;
      code-explorer|code-architect|code-reviewer)
        warn "feature-dev subagent '$d' is assumed-installed (cannot verify from shell). If absent, the orchestrator does the step inline."
        missing_external=$((missing_external + 1))
        ;;
      *)
        warn "delegate '$d' is unrecognised; treat as assumed-installed and degrade gracefully if absent."
        missing_external=$((missing_external + 1))
        ;;
    esac
  done <<< "$delegates"

  if [[ "$missing_external" -gt 0 ]]; then
    note "$missing_external external delegate(s) unverifiable — missing ones degrade to inline orders, never a hard fail."
  fi
}

case "${1:-}" in
  caveman) caveman_fallback "${2:-}" ;;
  *)       check_state "${1:-}" ;;
esac

exit 0
