#!/usr/bin/env bash
# scripts/scan-merges.sh — report <!-- merge --> blocks across .spec/features/
set -euo pipefail

FEATURE=""
FORMAT="table"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --format) FORMAT="${2:?--format requires table|json|plain}"; shift 2 ;;
    --format=*) FORMAT="${1#--format=}"; shift ;;
    -*) echo "Unknown option: $1" >&2; exit 1 ;;
    *) FEATURE="$1"; shift ;;
  esac
done

SPEC_DIR="${SPEC_DIR:-.spec}"

if [[ -n "$FEATURE" ]]; then
  target_files=("$SPEC_DIR/features/$FEATURE/tech.md")
else
  target_files=()
  for f in "$SPEC_DIR"/features/*/tech.md; do
    [[ -f "$f" ]] && target_files+=("$f")
  done
fi

if [[ ${#target_files[@]} -eq 0 ]]; then
  echo "No feature tech.md files found." >&2
  exit 0
fi

UNCLOSED=0
results=()

for f in "${target_files[@]}"; do
  [[ -f "$f" ]] || continue
  feat="$(basename "$(dirname "$f")")"
  lineno=0
  in_block=0
  block_start=0
  preview=""
  while IFS= read -r line; do
    ((lineno++)) || true
    if [[ "$line" == "<!-- merge -->" ]]; then
      if [[ $in_block -eq 1 ]]; then
        echo "ERROR: unclosed <!-- merge --> at line $block_start in $f (nested open at $lineno)" >&2
        UNCLOSED=1
        continue
      fi
      in_block=1
      block_start=$lineno
      preview=""
    elif [[ "$line" == "<!-- /merge -->" ]]; then
      if [[ $in_block -ne 1 ]]; then
        echo "ERROR: <!-- /merge --> without opener at line $lineno in $f" >&2
        UNCLOSED=1
        continue
      fi
      in_block=0
      results+=("${feat}|${f}|${block_start}|${lineno}|${preview}")
    elif [[ $in_block -eq 1 && -z "$preview" && -n "$line" ]]; then
      preview="${line:0:60}"
    fi
  done < "$f"
  if [[ $in_block -eq 1 ]]; then
    echo "ERROR: unclosed <!-- merge --> block starting at line $block_start in $f" >&2
    UNCLOSED=1
  fi
done

if [[ ${#results[@]} -eq 0 && $UNCLOSED -eq 0 ]]; then
  echo "No merge blocks found."
  exit 0
fi

case "$FORMAT" in
  json)
    printf '['
    first=1
    for r in "${results[@]}"; do
      IFS='|' read -r feat file start end preview <<< "$r"
      [[ $first -eq 0 ]] && printf ','
      printf '{"feature":"%s","file":"%s","start":%s,"end":%s,"preview":"%s"}' \
        "$feat" "$file" "$start" "$end" "$preview"
      first=0
    done
    printf ']\n'
    ;;
  plain)
    for r in "${results[@]}"; do
      IFS='|' read -r feat file start end preview <<< "$r"
      echo "${file}:${start}-${end}: ${preview}"
    done
    ;;
  *)
    printf '%-25s %-45s %-12s %s\n' "feature" "file" "lines" "preview"
    printf '%-25s %-45s %-12s %s\n' "-------" "----" "-----" "-------"
    for r in "${results[@]}"; do
      IFS='|' read -r feat file start end preview <<< "$r"
      printf '%-25s %-45s %-12s %s\n' "$feat" "$file" "${start}-${end}" "${preview}"
    done
    ;;
esac

[[ $UNCLOSED -eq 0 ]]
