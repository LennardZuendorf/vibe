#!/usr/bin/env bash
# regen-active-rules.sh — project a capped top-5 digest of .spec/lessons.md into
# the managed active-rules block of CLAUDE.md and AGENTS.md.
#
# This is generated output (like a committed lockfile), NOT a second source of
# truth. lessons.md stays canonical. Run during a *.compound state.
#
# Markers (everything outside them is user-owned and never touched):
#   <!-- vibe:active-rules:start -->
#   ... generated ...
#   <!-- vibe:active-rules:end -->
#
# Selection: pinned first (entries with **Pinned-by:** — pinning is deliberately
# expensive), then most recent by **Date:**. Hard cap of 5.
#
# Deterministic, idempotent, marker-aware. Graceful degrade: missing lessons.md
# or no entries -> warn and leave an empty managed block; never hard-fail.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SKILL_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
REPO_ROOT="$(cd "$SKILL_DIR/../../.." && pwd)"

LESSONS="$REPO_ROOT/.spec/lessons.md"
TARGETS=("$REPO_ROOT/CLAUDE.md" "$REPO_ROOT/AGENTS.md")
CAP=5
START="<!-- vibe:active-rules:start -->"
END="<!-- vibe:active-rules:end -->"

warn() { echo "regen-active-rules: $1" >&2; }

# Build the digest body (markdown lines) from lessons.md.
build_digest() {
  if [[ ! -f "$LESSONS" ]]; then
    warn "WARN: $LESSONS not found; writing empty digest."
    echo "_No lessons recorded yet._"
    return 0
  fi

  local rows
  rows="$(awk '
    function emit() {
      if (have) printf "%d\t%s\t%s\t%s\n", pinned, (date=="" ? "0000-00-00" : date), title, rule
    }
    # Skip HTML comment blocks (e.g. the trailing format template). Only a line
    # that STARTS with <!-- opens a block, so a Rule/Pattern body may contain an
    # inline <!-- ... --> token (e.g. a marker example) without being dropped.
    /^[[:space:]]*<!--/ { in_comment=1 }
    in_comment { if ($0 ~ /-->/) in_comment=0; next }
    /^### / { emit(); have=1; title=substr($0,5); rule=""; date=""; pinned=0; next }
    /^\*\*Rule:\*\*/      { r=$0; sub(/^\*\*Rule:\*\* */,"",r); rule=r; next }
    /^\*\*Date:\*\*/      { d=$0; sub(/^\*\*Date:\*\* */,"",d); date=d; next }
    /^\*\*Pinned-by:\*\*/ { pinned=1; next }
    END { emit() }
  ' "$LESSONS" \
    | sort -t$'\t' -k1,1nr -k2,2r \
    | head -n "$CAP")"

  if [[ -z "$rows" ]]; then
    warn "WARN: no lessons parsed; writing empty digest."
    echo "_No lessons recorded yet._"
    return 0
  fi

  # Format: pinned entries get a 📌, then "**title** — rule".
  awk -F'\t' '{
    pin = ($1 == "1") ? "📌 " : ""
    printf "- %s**%s** — %s\n", pin, $3, $4
  }' <<< "$rows"
}

# Replace the content between markers in $1 with the digest. Append a managed
# block if the markers are absent. Idempotent.
write_block() {
  local file="$1" digest="$2"
  local target="$file"
  if [[ -L "$file" ]]; then
    local link
    link="$(readlink "$file")"
    if [[ "$link" == /* ]]; then
      target="$link"
    else
      target="$(cd "$(dirname "$file")" && pwd)/$link"
    fi
  fi
  [[ -f "$target" ]] || { warn "WARN: $target not found; skipping."; return 0; }

  local block
  block="$START
<!-- Generated from .spec/lessons.md by regen-active-rules.sh. Do not edit by hand;
     edit lessons.md and re-run during compound. Top $CAP, pinned first. -->

### Active Rules

$digest
$END"

  # Temp files live beside $target (the mv destination), not $file (which may be
  # a symlink pointing elsewhere) — so the final mv is always a same-directory
  # atomic rename. The RETURN trap is the sole cleanup path.
  local tmp blockfile
  tmp="$(mktemp "${target}.XXXXXX")"
  blockfile="$(mktemp "${target}.block.XXXXXX")"
  trap 'rm -f "$tmp" "$blockfile"' RETURN
  printf '%s\n' "$block" > "$blockfile"

  if grep -qF "$START" "$target" && grep -qF "$END" "$target"; then
    awk -v start="$START" -v end="$END" -v blockfile="$blockfile" '
      $0 == start {
        while ((getline line < blockfile) > 0) print line
        close(blockfile)
        skip = 1
        next
      }
      $0 == end { skip = 0; next }
      !skip { print }
    ' "$target" > "$tmp"
  else
    cat "$target" > "$tmp"
    printf '\n' >> "$tmp"
    cat "$blockfile" >> "$tmp"
  fi

  mv -f "$tmp" "$target"
  echo "regen-active-rules: updated $(basename "$target")"
}

DIGEST="$(build_digest)"
seen=""
for t in "${TARGETS[@]}"; do
  resolved="$t"
  if command -v realpath >/dev/null 2>&1; then
    resolved="$(realpath "$t" 2>/dev/null || echo "$t")"
  elif command -v python3 >/dev/null 2>&1; then
    resolved="$(python3 -c 'import os,sys; print(os.path.realpath(sys.argv[1]))' "$t" 2>/dev/null || echo "$t")"
  fi
  if printf '%s\n' "$seen" | grep -Fxq "$resolved"; then
    continue
  fi
  seen="${seen}${resolved}"$'\n'
  write_block "$t" "$DIGEST"
done
