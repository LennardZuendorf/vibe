#!/usr/bin/env bash
# Preserve the owned legacy workflow before its discovery name becomes toolkit.
set -euo pipefail
ROOT="${1:?usage: migrate-discovery.sh PROJECT_ROOT}"
SKILLS="${2:-$ROOT/.agents/skills}"
OLD="$SKILLS/vibe"
NEW="$SKILLS/flow"
RUNTIME="$ROOT/.vibe/run/flow"
# A name alone is not ownership. Foreign skills and the new toolkit are untouched.
owned_legacy() {
  [[ -f "$OLD/state-machine.json" && -f "$OLD/scripts/set-state.sh" ]] \
    && grep -qE '^name: (vibe|flow)$' "$OLD/SKILL.md" 2>/dev/null
}
if ! owned_legacy; then exit 0; fi
# Never create a lock or preserve state through a foreign runtime link.
check_runtime_paths() {
  local p
  for p in "$ROOT/.vibe" "$ROOT/.vibe/run" "$RUNTIME" "$RUNTIME/evidence" "$RUNTIME/state.json"; do
    if [[ -L "$p" ]]; then echo "migration: conflict: runtime path is a symlink: $p" >&2; return 1; fi
  done
}
check_runtime_paths
mkdir -p "$RUNTIME"
LOCK="$RUNTIME/runtime.lock"
LOCK_TOKEN="migration-$$-$RANDOM-$(date -u +%s)"
# Noclobber may open existing nonregular entries (notably FIFOs) and block.
if [[ -e "$LOCK" || -L "$LOCK" ]]; then
  echo "migration: runtime busy; preserve existing holder and retry" >&2
  exit 1
fi
if ! (set -o noclobber; printf '%s\n' "$LOCK_TOKEN" > "$LOCK") 2>/dev/null; then
  echo "migration: runtime busy; preserve existing holder and retry" >&2
  exit 1
fi
release_lock() {
  if [[ -f "$LOCK" && ! -L "$LOCK" ]] && [[ "$(cat "$LOCK")" == "$LOCK_TOKEN" ]]; then rm -f "$LOCK"; fi
}
trap release_lock EXIT
# Namespace and adoption state are authoritative only after acquiring the shared lock.
if ! owned_legacy; then exit 0; fi
check_runtime_paths
if [[ -e "$RUNTIME/runs" || -L "$RUNTIME/runs" ]]; then
  echo "migration: adopted runtime records exist; refuse legacy state preservation; use explicit recovery" >&2
  exit 1
fi
if [[ -e "$RUNTIME/state.json" ]]; then
  if [[ ! -f "$RUNTIME/state.json" ]]; then
    echo "migration: conflict: canonical cursor is not a regular file" >&2; exit 1
  fi
  helper_dir="${BASH_SOURCE[0]%/*}"
  [[ "$helper_dir" != "${BASH_SOURCE[0]}" ]] || helper_dir=.
  # shellcheck source=flow/scripts/runtime-state.sh
  source "$helper_dir/runtime-state.sh"
  kind="$(runtime_cursor_kind "$RUNTIME/state.json")"
  if [[ "$kind" != legacy ]]; then
    echo "migration: versioned, corrupt or unverified canonical cursor; preserve state for explicit recovery" >&2
    exit 1
  fi
fi
if [[ -e "$NEW" || -L "$NEW" ]]; then
  echo "migration: conflict: both legacy vibe workflow and flow exist; preserve both and reconcile before retry" >&2
  exit 1
fi
if [[ -f "$OLD/state.json" && -e "$RUNTIME/state.json" ]] \
  && ! cmp -s "$OLD/state.json" "$RUNTIME/state.json"; then
  echo "migration: conflict: canonical and legacy cursor differ; preserve both and reconcile before retry" >&2
  exit 1
fi
if [[ -d "$OLD/evidence" ]]; then
  while IFS= read -r receipt; do
    rel="${receipt#"$OLD/evidence/"}"
    if [[ -e "$RUNTIME/evidence/$rel" ]] && ! cmp -s "$receipt" "$RUNTIME/evidence/$rel"; then
      echo "migration: conflict: evidence differs: $rel" >&2; exit 1
    fi
  done < <(find "$OLD/evidence" -type f)
fi
mkdir -p "$RUNTIME"
if [[ -f "$OLD/state.json" && ! -e "$RUNTIME/state.json" ]]; then
  cp -p "$OLD/state.json" "$RUNTIME/state.json"
fi
if [[ -d "$OLD/evidence" ]]; then
  mkdir -p "$RUNTIME/evidence"
  cp -Rp "$OLD/evidence/." "$RUNTIME/evidence/"
fi
# Rename the entry, retaining all resources and user additions. For a source
# alias this moves only the symlink, never its external target or live files.
mv "$OLD" "$NEW"
echo "migration: preserved legacy cursor, receipts and resources before toolkit discovery"
