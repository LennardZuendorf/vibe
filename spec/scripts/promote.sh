#!/usr/bin/env bash
# scripts/promote.sh — extract merge markers from feature tech.md and promote to root spec
set -euo pipefail

FEATURE="${1:?usage: promote.sh <feature-name> [--dry-run] [--target <file>]}"
DRY_RUN=0
TARGET=".spec/tech.md"

shift
while [[ $# -gt 0 ]]; do
  case "$1" in
    --dry-run) DRY_RUN=1 ;;
    --target)  TARGET="${2:?--target requires a path}"; shift ;;
    *) echo "Unknown option: $1" >&2; exit 1 ;;
  esac
  shift
done

FEATURE_TECH=".spec/features/$FEATURE/tech.md"
[[ -f "$FEATURE_TECH" ]] || { echo "ERROR: $FEATURE_TECH not found" >&2; exit 1; }
[[ -f "$TARGET" ]]       || { echo "ERROR: $TARGET not found" >&2; exit 1; }

BLOCKS=()
in_block=0
lineno=0
current_block=""
while IFS= read -r line; do
  ((lineno++)) || true
  if [[ "$line" == "<!-- merge -->" ]]; then
    [[ $in_block -eq 1 ]] && { echo "ERROR: nested <!-- merge --> at line $lineno in $FEATURE_TECH" >&2; exit 1; }
    in_block=1
    current_block=""
  elif [[ "$line" == "<!-- /merge -->" ]]; then
    if [[ $in_block -ne 1 ]]; then
      echo "ERROR: <!-- /merge --> without opener at line $lineno in $FEATURE_TECH" >&2
      exit 1
    fi
    in_block=0
    BLOCKS+=("$current_block")
  elif [[ $in_block -eq 1 ]]; then
    current_block="${current_block}${line}"$'\n'
  fi
done < "$FEATURE_TECH"

[[ $in_block -eq 1 ]] && { echo "ERROR: unclosed <!-- merge --> block in $FEATURE_TECH" >&2; exit 1; }

BLOCK_COUNT=${#BLOCKS[@]}
if [[ $BLOCK_COUNT -eq 0 ]]; then
  echo "WARN: no <!-- merge --> blocks found in $FEATURE_TECH" >&2
  exit 0
fi

# Idempotency: skip any block whose exact content already exists in the target.
# Read target preserving trailing newlines so multi-line substring checks are exact.
target_content="$(cat "$TARGET"; printf x)"; target_content="${target_content%x}"
to_append=""
appended=0
skipped=0
for block in "${BLOCKS[@]}"; do
  if [[ "$target_content" == *"$block"* ]]; then
    skipped=$((skipped + 1))
    continue
  fi
  to_append="${to_append}"$'\n'"${block}"
  target_content="${target_content}"$'\n'"${block}"
  appended=$((appended + 1))
done

if [[ $DRY_RUN -eq 1 ]]; then
  if [[ $appended -eq 0 ]]; then
    echo "--- nothing to promote: all $BLOCK_COUNT block(s) already present in $TARGET ---"
  else
    echo "--- would promote $appended block(s) to $TARGET ($skipped already present) ---"
    printf '%s' "$to_append"
  fi
  exit 0
fi

if [[ $appended -eq 0 ]]; then
  echo "No new blocks to promote — all $BLOCK_COUNT already present in $TARGET"
  exit 0
fi

TMPFILE="$(mktemp "$(dirname "$TARGET")/.promote.XXXXXX")"
cat "$TARGET" > "$TMPFILE"
printf '%s' "$to_append" >> "$TMPFILE"
mv "$TMPFILE" "$TARGET"

echo "PROMOTED $appended block(s) from $FEATURE_TECH → $TARGET ($skipped already present)"
