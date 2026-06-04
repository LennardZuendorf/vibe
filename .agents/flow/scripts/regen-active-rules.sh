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
FLOW_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
REPO_ROOT="$(cd "$FLOW_DIR/../.." && pwd)"

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
    # Skip anything inside HTML comment blocks (e.g. the format template).
    /<!--/ { in_comment=1 }
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
  [[ -f "$file" ]] || { warn "WARN: $file not found; skipping."; return 0; }

  local block
  block="$START
<!-- Generated from .spec/lessons.md by regen-active-rules.sh. Do not edit by hand;
     edit lessons.md and re-run during compound. Top $CAP, pinned first. -->

### Active Rules

$digest
$END"

  local tmp; tmp="$(mktemp "${file}.XXXXXX")"
  trap 'rm -f "$tmp"' RETURN

  if grep -qF "$START" "$file" && grep -qF "$END" "$file"; then
    # Replace existing block in place.
    awk -v start="$START" -v end="$END" -v block="$block" '
      $0 == start { print block; skip=1; next }
      $0 == end   { skip=0; next }
      !skip       { print }
    ' "$file" > "$tmp"
  else
    # Append a fresh block, separated by a blank line.
    cat "$file" > "$tmp"
    printf '\n%s\n' "$block" >> "$tmp"
  fi

  mv -f "$tmp" "$file"
  trap - RETURN
  echo "regen-active-rules: updated $(basename "$file")"
}

DIGEST="$(build_digest)"
for t in "${TARGETS[@]}"; do
  write_block "$t" "$DIGEST"
done
