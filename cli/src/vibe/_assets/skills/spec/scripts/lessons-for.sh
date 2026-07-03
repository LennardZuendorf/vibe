#!/usr/bin/env bash
# scripts/lessons-for.sh — extract lessons matching tags from lessons.md
set -euo pipefail

FORMAT="markdown"
TAGS=()

while [[ $# -gt 0 ]]; do
  case "$1" in
    --format) FORMAT="${2:?--format requires a value (markdown|inject|json)}"; shift 2 ;;
    --format=*) FORMAT="${1#--format=}"; shift ;;
    markdown|inject|json) FORMAT="$1"; shift ;;
    -*) echo "Unknown option: $1" >&2; exit 1 ;;
    *) TAGS+=("$1"); shift ;;
  esac
done

LESSONS_FILE="${SPEC_DIR:-.spec}/lessons.md"
[[ -f "$LESSONS_FILE" ]] || exit 0
[[ ${#TAGS[@]} -gt 0 ]] || { echo "usage: lessons-for.sh <tag> [<tag>...] [--format markdown|inject|json]" >&2; exit 1; }

current_block=""
matched_blocks=()
in_lesson=0

while IFS= read -r line || [[ -n "$line" ]]; do
  if [[ "$line" =~ ^###[[:space:]] ]]; then
    if [[ $in_lesson -eq 1 && -n "$current_block" ]]; then
      tags_line="$(printf '%s\n' "$current_block" | grep '^\*\*Tags:\*\*' | head -1 || true)"
      for tag in "${TAGS[@]}"; do
        if printf '%s\n' "$tags_line" | grep -qi "$tag"; then
          matched_blocks+=("$current_block")
          break
        fi
      done
    fi
    current_block="$line"
    in_lesson=1
  elif [[ $in_lesson -eq 1 ]]; then
    current_block="${current_block}"$'\n'"$line"
  fi
done < "$LESSONS_FILE"

if [[ $in_lesson -eq 1 && -n "$current_block" ]]; then
  tags_line="$(printf '%s\n' "$current_block" | grep '^\*\*Tags:\*\*' | head -1 || true)"
  for tag in "${TAGS[@]}"; do
    if printf '%s\n' "$tags_line" | grep -qi "$tag"; then
      matched_blocks+=("$current_block")
      break
    fi
  done
fi

[[ ${#matched_blocks[@]} -eq 0 ]] && exit 0

case "$FORMAT" in
  inject)
    echo "<!-- lessons: ${TAGS[*]} -->"
    for block in "${matched_blocks[@]}"; do printf '%s\n\n' "$block"; done
    echo "<!-- /lessons -->"
    ;;
  json)
    printf '['
    first=1
    for block in "${matched_blocks[@]}"; do
      [[ $first -eq 0 ]] && printf ','
      title="$(printf '%s\n' "$block" | head -1 | sed 's/^### //')"
      pattern="$(printf '%s\n' "$block" | grep '^\*\*Pattern:\*\*' | sed 's/\*\*Pattern:\*\* //' || true)"
      rule="$(printf '%s\n' "$block" | grep '^\*\*Rule:\*\*' | sed 's/\*\*Rule:\*\* //' || true)"
      tags="$(printf '%s\n' "$block" | grep '^\*\*Tags:\*\*' | sed 's/\*\*Tags:\*\* //' || true)"
      date="$(printf '%s\n' "$block" | grep '^\*\*Date:\*\*' | sed 's/\*\*Date:\*\* //' || true)"
      printf '{"title":"%s","pattern":"%s","rule":"%s","tags":"%s","date":"%s"}' \
        "$title" "$pattern" "$rule" "$tags" "$date"
      first=0
    done
    printf ']\n'
    ;;
  *)
    for block in "${matched_blocks[@]}"; do printf '%s\n---\n' "$block"; done
    ;;
esac
