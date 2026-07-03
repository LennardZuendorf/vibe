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

BLOCKS=""
BLOCK_COUNT=0
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
    BLOCKS="${BLOCKS}${current_block}"$'\n'
    ((BLOCK_COUNT++)) || true
  elif [[ $in_block -eq 1 ]]; then
    current_block="${current_block}${line}"$'\n'
  fi
done < "$FEATURE_TECH"

[[ $in_block -eq 1 ]] && { echo "ERROR: unclosed <!-- merge --> block in $FEATURE_TECH" >&2; exit 1; }

if [[ $BLOCK_COUNT -eq 0 ]]; then
  echo "WARN: no <!-- merge --> blocks found in $FEATURE_TECH" >&2
  exit 0
fi

if [[ $DRY_RUN -eq 1 ]]; then
  echo "--- would promote $BLOCK_COUNT block(s) to $TARGET ---"
  printf '%s' "$BLOCKS"
  exit 0
fi

TMPFILE="$(mktemp "$(dirname "$TARGET")/.promote.XXXXXX")"
cat "$TARGET" > "$TMPFILE"
printf '\n%s\n' "$BLOCKS" >> "$TMPFILE"
mv "$TMPFILE" "$TARGET"

echo "PROMOTED $BLOCK_COUNT block(s) from $FEATURE_TECH → $TARGET"
