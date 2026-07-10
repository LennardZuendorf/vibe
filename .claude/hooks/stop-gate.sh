#!/usr/bin/env bash
# stop-gate.sh — vibe flow gate hook (platform-adapters/3 + flow-mvp/9).
#
# Event: Stop. End-of-turn smell checks over the flow state. Predicates are
# WARN-ONLY (print to stderr, exit 0) EXCEPT the one promoted tooth: in a
# *.verify state the evidence receipt is required, so a missing or stale receipt
# BLOCKS the stop (exit 2). Promote further predicates only after dogfooding
# proves they are crossed by accident, not on purpose ("earn the teeth").
#
# Thin shell: state comes from .agents/skills/vibe/scripts/detect-context.sh snapshot;
# no flow policy is duplicated here.
#
# Warnings relay: warn-only smells print to stderr (exit 0), which Claude Code
# never surfaces to the model. So each warn is also appended to
# .agents/skills/vibe/warnings.log; the UserPromptSubmit inject hook drains that
# log to stdout (injected into context) next turn, then truncates it. The one
# blocking tooth (missing/stale receipt) still uses exit 2, which IS model-visible.
#
# jq-optional (review-fix): the one blocking tooth must fire with or WITHOUT jq —
# docs call it a hard block, so it cannot silently vanish on a jq-less target.
# Without jq the state/feature come from the flat machine-written cursor via the
# same sed fallback detect-context.sh uses, and stop_hook_active is read from
# stdin with sed. The receipt existence/staleness logic is already file + git
# based (no jq), so the block is byte-for-byte identical either way. Only the
# warn-only nudge that needs the machine's `next` array is skipped without jq.
#
# Graceful degrade (R9): missing detect-context.sh / unreadable cursor -> the
# affected check is skipped and the hook exits 0. stop_hook_active passes through
# (no block loops). Outside *.verify the hook is warn-only, exactly as before. An
# unwritable relay log never fails the hook.

set -euo pipefail

STDIN="$(cat 2>/dev/null || true)"   # capture stdin (stop_hook_active lives here)

ROOT="${CLAUDE_PROJECT_DIR:-$PWD}"
DETECT="$ROOT/.agents/skills/vibe/scripts/detect-context.sh"
STATE_FILE="$ROOT/.agents/skills/vibe/state.json"
EVID_REL=".agents/skills/vibe/evidence"
WARN_LOG="$ROOT/.agents/skills/vibe/warnings.log"

have_jq() { command -v jq >/dev/null 2>&1; }

# Re-entry guard: a Stop hook that already fired must not block again (Claude sets
# stop_hook_active on the re-invocation), or the block would loop forever. Read it
# jq-first, else sed over the flat stdin JSON.
if have_jq; then
  STOP_ACTIVE="$(printf '%s' "$STDIN" | jq -r '.stop_hook_active // false' 2>/dev/null || echo false)"
else
  STOP_ACTIVE="$(printf '%s' "$STDIN" | sed -n 's/.*"stop_hook_active"[[:space:]]*:[[:space:]]*\(true\|false\).*/\1/p' | head -n1)"
  [[ -n "$STOP_ACTIVE" ]] || STOP_ACTIVE=false
fi
[[ "$STOP_ACTIVE" == "true" ]] && exit 0

[[ -f "$DETECT" ]] || exit 0

# State + feature resolution, jq-optional. With jq, take the detect-context.sh
# snapshot (also carries the machine `next` for the warn-only nudge). Without jq,
# read the flat cursor directly with the shared sed fallback — the blocking tooth
# needs STATE + FEATURE only, so NEXT is left empty (its predicate is warn-only).
if have_jq; then
  SNAP="$(bash "$DETECT" snapshot 2>/dev/null || echo '{}')"
  STATE="$(printf '%s' "$SNAP" | jq -r '.state // "idle"' 2>/dev/null || echo idle)"
  NEXT="$(printf '%s' "$SNAP" | jq -r '(.next // []) | join(", ")' 2>/dev/null || echo "")"
  FEATURE="$(printf '%s' "$SNAP" | jq -r '.feature // empty' 2>/dev/null || echo "")"
else
  STATE=idle; NEXT=""; FEATURE=""
  if [[ -f "$STATE_FILE" ]]; then
    _flow="$(sed -n 's/.*"flow"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' "$STATE_FILE" | head -n1)"
    _phase="$(sed -n 's/.*"phase"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' "$STATE_FILE" | head -n1)"
    FEATURE="$(sed -n 's/.*"feature"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' "$STATE_FILE" | head -n1)"
    [[ -n "$_flow" && "$_flow" != "null" ]] || _flow=idle
    [[ -n "$_phase" && "$_phase" != "null" ]] || _phase=idle
    if [[ "$_flow" == "$_phase" ]]; then STATE="$_flow"; else STATE="$_flow.$_phase"; fi
  fi
fi

# warn — print a warn-only smell to stderr AND queue it to the warnings relay so
# the inject hook can surface it to the model next turn. An unwritable log or a
# missing vibe dir is a silent no-op (never fail the hook).
warn() {
  echo "vibe-gate: $1" >&2
  local dir="$ROOT/.agents/skills/vibe"
  [[ -d "$dir" ]] || return 0
  printf 'gate: %s\n' "$1" >> "$WARN_LOG" 2>/dev/null || true
}

git_changed() {
  command -v git >/dev/null 2>&1 || return 1
  git -C "$ROOT" rev-parse --is-inside-work-tree >/dev/null 2>&1 || return 1
  git -C "$ROOT" status --porcelain 2>/dev/null
}

# Predicate 1 — impl touched src/** but no tests/**.  (warn-only)
# TODO(earn-the-teeth): promote to blocking once dogfood shows it only fires on
# genuine TDD misses, never on doc-only or refactor turns.
if [[ "$STATE" == "feature.impl" || "$STATE" == "quick.fix" ]]; then
  CH="$(git_changed || true)"
  if [[ -n "$CH" ]]; then
    if grep -qE '(^|/)src/' <<<"$CH" && ! grep -qE '(^|/)tests?/' <<<"$CH"; then
      warn "in $STATE, src changed with no test changes — TDD expects a reproducing/covering test. (warn-only)"
    fi
  fi
fi

# Predicate 2 (PROMOTED, flow-mvp/9) — a *.verify state requires a fresh evidence
# receipt before "done". This is the one blocking tooth; every other predicate is
# warn-only. Fixed receipt names under $ROOT/.agents/skills/vibe/evidence/:
# feature-<feature>.md for feature.verify, quick.md for quick.verify.
if [[ "$STATE" == "feature.verify" || "$STATE" == "quick.verify" ]]; then
  receipt=""
  case "$STATE" in
    feature.verify)
      if [[ -n "$FEATURE" ]]; then
        receipt="$ROOT/$EVID_REL/feature-$FEATURE.md"
      else
        # Cursor names no feature -> the receipt path is ambiguous; degrade to
        # warn-only rather than block on ambiguity.
        warn "in feature.verify but the cursor names no feature — cannot resolve the evidence receipt; skipping the gate. (warn-only)"
      fi
      ;;
    quick.verify)
      receipt="$ROOT/$EVID_REL/quick.md"
      ;;
  esac

  if [[ -n "$receipt" ]]; then
    if [[ ! -f "$receipt" ]]; then
      {
        echo "vibe-gate: BLOCKED — $STATE needs an evidence receipt before 'done'."
        echo "  expected: $receipt"
        echo "  it must record the commands you ran and their observed output (per unit ID for a feature)."
        echo "  not verifying? abort with: bash .agents/skills/vibe/scripts/set-state.sh idle"
      } >&2
      exit 2
    fi
    # Receipt present. Staleness is git-derived: any changed path (git status
    # --porcelain, both columns) whose mtime is newer than the receipt means the
    # receipt no longer reflects the tree. No git or a clean tree -> existence-only
    # (pass). The evidence dir is excluded (a receipt is always newer than the
    # tree it describes).
    # Known fail-OPEN cases (never false-block, per the speed-bump posture):
    # porcelain paths are repo-root-relative, so when $ROOT is a subdirectory of
    # an enclosing git repo the -e probe misses and staleness no-ops; C-quoted
    # non-ASCII paths (core.quotePath) are likewise skipped.
    CH="$(git_changed || true)"
    if [[ -n "$CH" ]]; then
      stale=""
      while IFS= read -r line; do
        [[ -n "$line" ]] || continue
        xy="${line:0:2}"
        path="${line:3}"
        case "$xy" in *D*) continue ;; esac                 # skip deletions
        case "$xy" in R*|C*) path="${path##* -> }" ;; esac   # rename/copy -> new path
        case "$path" in "$EVID_REL"/*) continue ;; esac      # exclude the evidence dir
        f="$ROOT/$path"
        if [[ -e "$f" && "$f" -nt "$receipt" ]]; then
          stale="$path"
          break
        fi
      done <<< "$CH"
      if [[ -n "$stale" ]]; then
        {
          echo "vibe-gate: BLOCKED — the evidence receipt is stale."
          echo "  receipt: $receipt"
          echo "  changed after it was written: $stale"
          echo "  re-run verification and rewrite the receipt with fresh commands + output."
          echo "  not verifying? abort with: bash .agents/skills/vibe/scripts/set-state.sh idle"
        } >&2
        exit 2
      fi
    fi
  fi
fi

# Predicate 3 — stuck phase / forgotten set-state.sh.  (warn-only)
# TODO(earn-the-teeth): needs a turn counter, which the cursor deliberately omits
# (prompt-cache discipline). Surface the next legal states as a nudge for now.
if [[ "$STATE" != "idle" && -n "$NEXT" ]]; then
  warn "still in $STATE — when this phase's exit is met, advance with set-state.sh (next: $NEXT). (warn-only)"
fi

exit 0
