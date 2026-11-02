#!/usr/bin/env bash
# resolve-target.sh — sourced helper: resolve a managed file to the physical path
# a write should land on, so a symlinked document stays a symlink.
#
#   resolve_target FILE ROOT
#     stdout  the physical path (symlinked parents resolved, links followed)
#     rc 0    FILE is a regular file, or does not exist, and sits inside ROOT
#     rc 1    a link dangles, loops, or resolves outside ROOT; the reason goes to
#             stderr and names FILE
#
# Callers create their temp file next to the resolved path and `mv` onto it, so
# the rename is atomic and the link survives. No `readlink -f` (absent on macOS
# bash 3.2 hosts): a hop-capped `readlink` loop plus `cd -P` on the parent.

resolve_target() {
  local file="$1" root="$2" orig="$1" hops=0 dir base link rootp
  while [[ -L "$file" ]]; do
    hops=$((hops + 1))
    if (( hops > 40 )); then
      echo "resolve-target: symlink loop at $orig" >&2; return 1
    fi
    dir="$(cd -P "$(dirname "$file")" 2>/dev/null && pwd -P)" \
      || { echo "resolve-target: dangling link $orig (parent directory missing)" >&2; return 1; }
    link="$(readlink "$file")"
    case "$link" in
      /*) file="$link" ;;
      *)  file="$dir/$link" ;;
    esac
  done
  if (( hops > 0 )) && [[ ! -f "$file" ]]; then
    echo "resolve-target: $orig is a dangling link or not a regular file ($file)" >&2
    return 1
  fi
  dir="$(cd -P "$(dirname "$file")" 2>/dev/null && pwd -P)" \
    || { echo "resolve-target: cannot resolve the directory of $orig" >&2; return 1; }
  base="$(basename "$file")"
  rootp="$(cd -P "$root" 2>/dev/null && pwd -P)" \
    || { echo "resolve-target: root not found: $root" >&2; return 1; }
  case "$dir/$base" in
    "$rootp"/*) ;;
    *) echo "resolve-target: $orig resolves outside $rootp ($dir/$base)" >&2; return 1 ;;
  esac
  printf '%s\n' "$dir/$base"
}
