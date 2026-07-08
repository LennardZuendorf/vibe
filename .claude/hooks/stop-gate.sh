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
# Graceful degrade (R9): missing jq / detect-context.sh / unreadable cursor -> the
# affected check is skipped and the hook exits 0. stop_hook_active passes through
# (no block loops). Outside *.verify the hook is warn-only, exactly as before.

set -euo pipefail

STDIN="$(cat 2>/dev/null || true)"   # capture stdin (stop_hook_active lives here)

ROOT="${CLAUDE_PROJECT_DIR:-$PWD}"
DETECT="$ROOT/.agents/skills/vibe/scripts/detect-context.sh"
EVID_REL=".agents/skills/vibe/evidence"

command -v jq >/dev/null 2>&1 || exit 0

# Re-entry guard: a Stop hook that already fired must not block again (Claude sets
# stop_hook_active on the re-invocation), or the block would loop forever.
if [[ "$(printf '%s' "$STDIN" | jq -r '.stop_hook_active // false' 2>/dev/null || echo false)" == "true" ]]; then
  exit 0
fi

[[ -f "$DETECT" ]] || exit 0

SNAP="$(bash "$DETECT" snapshot 2>/dev/null || echo '{}')"
STATE="$(printf '%s' "$SNAP" | jq -r '.state // "idle"' 2>/dev/null || echo idle)"
NEXT="$(printf '%s' "$SNAP" | jq -r '(.next // []) | join(", ")' 2>/dev/null || echo "")"
FEATURE="$(printf '%s' "$SNAP" | jq -r '.feature // empty' 2>/dev/null || echo "")"

warn() { echo "vibe-gate: $1" >&2; }

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
