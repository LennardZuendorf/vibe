#!/usr/bin/env bash
# merge-agents.sh — provision a repo's AGENTS.md from the vibe template, merging
# only inside the managed `vibe:instructions` markers, and (optionally) create
# adapter symlinks (CLAUDE.md, WARP.md, …) that point at AGENTS.md.
#
#   merge-agents.sh [TARGET_ROOT]          # merge template -> TARGET_ROOT/AGENTS.md
#   merge-agents.sh link <adapter> [ROOT]  # symlink <adapter> -> AGENTS.md (relative)
#
# Merge cases (deterministic, idempotent):
#   1. No AGENTS.md            -> copy the template.
#   2. vibe:instructions block -> replace content between the markers.
#   3. vibe:constitution block -> migrate: replace the legacy region with the
#                                 vibe:instructions region (one-time upgrade).
#   4. No markers but body == template body (normalized) -> wrap (copy template).
#   5. No markers, divergent   -> append the managed block (WARN; never clobber).
# Content OUTSIDE the markers (user preamble, the vibe:active-rules block) is never
# touched. Re-running with an up-to-date template is a no-op (byte compare).
#
# Graceful: missing template -> exit 1 with a clear message (this is a setup
# prerequisite, not a session-time degrade). A real-file adapter is reported, not
# clobbered.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# Template resolves relative to this script (vendored) or the global install.
TEMPLATE="$SCRIPT_DIR/../reference/templates/AGENTS.md"
if [[ ! -f "$TEMPLATE" && -f "$HOME/.agents/skills/vibe/reference/templates/AGENTS.md" ]]; then
  TEMPLATE="$HOME/.agents/skills/vibe/reference/templates/AGENTS.md"
fi

I_START="<!-- vibe:instructions:start -->"
I_END="<!-- vibe:instructions:end -->"
C_START="<!-- vibe:constitution:start -->"
C_END="<!-- vibe:constitution:end -->"
AR_START="<!-- vibe:active-rules:start -->"
AR_END="<!-- vibe:active-rules:end -->"
# The template's branded title line, above the managed markers. vibe writes it
# when it creates the file; unmerge removes it when nothing else vibe-owned keeps
# the file alive. Exact-match only — a user's own heading is never touched.
TITLE_LINE="# AGENTS.md — vibe Engineering Guide"

warn() { echo "merge-agents: WARN — $1" >&2; }
note() { echo "merge-agents: $1"; }
die()  { echo "merge-agents: ERROR — $1" >&2; exit 1; }

# marker_line FILE MARKER — line number of the first line that equals MARKER
# exactly, empty if none. grep -n + head, awk-free so the unmerge path runs on an
# awk-less target. Returns 0 even when nothing matches (no set -e trip).
marker_line() {
  local file="$1" marker="$2" hit
  hit="$(grep -nxF -- "$marker" "$file" 2>/dev/null | head -n1 || true)"
  [[ -n "$hit" ]] && printf '%s\n' "${hit%%:*}"
  return 0
}

# assert_not_reversed FILE START END — the one content-safety guard, shared by
# merge and unmerge for every managed block. If the pair is present as exact lines
# but reversed (end before start), replace/strip would silently drop trailing
# content — refuse rather than mangle. Absent or substring-only markers pass (the
# caller no-ops). Never re-implement this check elsewhere.
assert_not_reversed() {
  local file="$1" start="$2" end="$3" s_line e_line
  grep -qF "$start" "$file" && grep -qF "$end" "$file" || return 0
  s_line="$(marker_line "$file" "$start")"
  e_line="$(marker_line "$file" "$end")"
  [[ -n "$s_line" && -n "$e_line" ]] || return 0
  if (( s_line >= e_line )); then
    die "$file has reversed markers ($start line $s_line, $end line $e_line) — fix by hand"
  fi
}

# strip_managed_block FILE START END — remove the inclusive START..END region in
# place via grep -n + sed range delete (awk-free, so unmerge runs on an awk-less
# target). Assumes assert_not_reversed already validated the pair; no-ops when the
# block is absent or the markers are not exact lines. Writes via temp + rename.
strip_managed_block() {
  local file="$1" start="$2" end="$3"
  grep -qF "$start" "$file" && grep -qF "$end" "$file" || return 0
  local s_line e_line
  s_line="$(marker_line "$file" "$start")"
  e_line="$(marker_line "$file" "$end")"
  [[ -n "$s_line" && -n "$e_line" ]] || return 0
  local tmp; tmp="$(mktemp "${file}.XXXXXX")"
  sed "${s_line},${e_line}d" "$file" > "$tmp"
  mv -f "$tmp" "$file"
}

# Print the inclusive region between START and END markers in FILE.
extract_region() {
  local file="$1" start="$2" end="$3"
  awk -v s="$start" -v e="$end" '
    $0 == s { grab = 1 }
    grab { print }
    $0 == e && grab { exit }
  ' "$file"
}

# Replace the inclusive START..END region in FILE with the contents of BLOCKFILE.
replace_region() {
  local file="$1" start="$2" end="$3" blockfile="$4" out="$5"
  awk -v s="$start" -v e="$end" -v bf="$blockfile" '
    $0 == s {
      while ((getline line < bf) > 0) print line
      close(bf)
      skip = 1
      next
    }
    $0 == e && skip { skip = 0; next }
    !skip { print }
  ' "$file" > "$out"
}

# Normalize for the wrap/stub comparison: strip CR and trailing spaces, squeeze
# internal blank runs to one, drop leading/trailing blank lines. awk-free (sed +
# cat -s) so the unmerge stub check runs on an awk-less target.
normalize() {
  sed 's/\r$//; s/[[:space:]]*$//' "$1" \
    | cat -s \
    | sed '/./,$!d' \
    | sed -e :a -e '/^\n*$/{$d;N;ba' -e '}'
}

# ── adapter symlink mode ───────────────────────────────────────────────────────
link_adapter() {
  local adapter="${1:-}" root="${2:-.}"
  [[ -n "$adapter" ]] || die "usage: merge-agents.sh link <adapter> [TARGET_ROOT]"
  local target="AGENTS.md"
  ( cd "$root" || die "target root not found: $root"
    if [[ -L "$adapter" ]]; then
      if [[ "$(readlink "$adapter")" == "$target" ]]; then
        note "skip: $adapter already symlinks $target"; exit 0
      fi
      warn "$adapter symlinks $(readlink "$adapter"), not $target — relinking."
      ln -sf "$target" "$adapter"; note "relinked $adapter -> $target"; exit 0
    fi
    if [[ -e "$adapter" ]]; then
      warn "$adapter is a real file — not replacing. Show a diff and confirm before symlinking."
      exit 1
    fi
    ln -s "$target" "$adapter"
    note "linked $adapter -> $target"
  )
}

# ── merge mode ─────────────────────────────────────────────────────────────────
merge() {
  local root="${1:-.}"
  local target="$root/AGENTS.md"
  [[ -f "$TEMPLATE" ]] || die "template not found at $TEMPLATE"

  if [[ ! -f "$target" ]]; then
    cp "$TEMPLATE" "$target"
    note "created $target from template"
    return 0
  fi

  local tmp; tmp="$(mktemp "${target}.XXXXXX")"
  local block; block="$(mktemp "${target}.block.XXXXXX")"
  local tcore=""
  trap 'rm -f "$tmp" "$block" ${tcore:+"$tcore"}' RETURN
  extract_region "$TEMPLATE" "$I_START" "$I_END" > "$block"
  [[ -s "$block" ]] || die "template is missing its vibe:instructions markers"

  if grep -qF "$I_START" "$target" && grep -qF "$I_END" "$target"; then
    # Guard the cardinal invariant: reversed markers (end before start) would make
    # replace_region silently drop trailing content — refuse rather than mangle.
    # (Same shared guard the unmerge path uses.)
    assert_not_reversed "$target" "$I_START" "$I_END"
    replace_region "$target" "$I_START" "$I_END" "$block" "$tmp"
    if cmp -s "$tmp" "$target"; then
      note "no-op: $target instructions already up to date"
    else
      mv -f "$tmp" "$target"; note "merged template into $target (vibe:instructions block)"
    fi
    return 0
  fi

  if grep -qF "$C_START" "$target" && grep -qF "$C_END" "$target"; then
    replace_region "$target" "$C_START" "$C_END" "$block" "$tmp"
    mv -f "$tmp" "$target"
    note "migrated legacy vibe:constitution block -> vibe:instructions in $target"
    return 0
  fi

  # No markers. Wrap if the file IS the template body (normalized), else append.
  tcore="$(mktemp "${target}.tcore.XXXXXX")"   # cleaned by the RETURN trap
  # template core = inner instructions body (markers + active-rules stripped)
  awk -v s="$I_START" -v e="$I_END" '
    $0 == s { ins = 1; next }
    $0 == e { ins = 0; next }
    ins && $0 ~ /^<!-- Managed by vibe-setup/ { skipc = 1 }
    ins && skipc && $0 ~ /-->$/ { skipc = 0; next }
    ins && !skipc { print }
  ' "$TEMPLATE" > "$tcore"
  if diff -q <(normalize "$tcore") <(normalize "$target") >/dev/null 2>&1; then
    cp "$TEMPLATE" "$tmp" && mv -f "$tmp" "$target"
    note "wrapped unmarked-equivalent $target in vibe:instructions markers (no duplicate body)"
    return 0
  fi

  warn "$target has no vibe:instructions markers and diverges from the template."
  warn "appending the managed block; review and move your content as needed."
  { printf '\n'; cat "$block"; } >> "$target"
  note "appended vibe:instructions block to $target"
}

# ── unmerge mode (uninstall) ────────────────────────────────────────────────────
# Remove BOTH managed regions (vibe:instructions and vibe:active-rules),
# preserving everything outside them. Reuses the shared marker-pairing guard:
# reversed markers in EITHER block are refused before any write, so a refusal
# leaves the file byte-identical. If nothing but whitespace remains, the file was
# a vibe-created stub (target had no AGENTS.md) — delete it so uninstall leaves no
# trace. Writes go via temp + atomic rename.
unmerge() {
  local root="${1:-.}"
  local target="$root/AGENTS.md"
  [[ -f "$target" ]] || { note "no AGENTS.md at $target — nothing to remove"; return 0; }

  local had_i=0 had_ar=0
  grep -qF "$I_START"  "$target" && grep -qF "$I_END"  "$target" && had_i=1
  grep -qF "$AR_START" "$target" && grep -qF "$AR_END" "$target" && had_ar=1
  if [[ "$had_i" -eq 0 && "$had_ar" -eq 0 ]]; then
    note "no vibe managed blocks in $target — left untouched"; return 0
  fi

  # Pre-flight both blocks BEFORE mutating, so a reversed-marker refusal (die)
  # leaves the file untouched rather than half-stripped.
  assert_not_reversed "$target" "$I_START"  "$I_END"
  assert_not_reversed "$target" "$AR_START" "$AR_END"

  # Pure stub: the file is (normalized) the template vibe would create, with no
  # user content added around the managed blocks — including the template's own
  # title line, which sits above the markers. This is the "target had none" case:
  # remove it wholesale, the clean inverse of the fresh install that created it.
  if [[ -f "$TEMPLATE" ]] && diff -q <(normalize "$target") <(normalize "$TEMPLATE") >/dev/null 2>&1; then
    rm -f "$target"
    note "removed vibe-created AGENTS.md stub at $target (untouched template — target had none)"
    return 0
  fi

  strip_managed_block "$target" "$I_START"  "$I_END"
  strip_managed_block "$target" "$AR_START" "$AR_END"

  # Stripping the blocks can strand the vibe-branded title line vibe wrote above
  # them (user added prose outside the markers, or the active-rules block was
  # regenerated, so the file no longer matches the pristine stub). Remove an
  # exact-match leading title line and the blank lines it leaves; a user's own
  # heading (any other text) is left in place.
  if [[ "$(head -n1 "$target")" == "$TITLE_LINE" ]]; then
    local tmp; tmp="$(mktemp "${target}.XXXXXX")"
    sed '1d' "$target" | sed '/./,$!d' > "$tmp"
    mv -f "$tmp" "$target"
  fi

  # A file now holding only whitespace was created entirely by vibe — remove it.
  if ! grep -q '[^[:space:]]' "$target"; then
    rm -f "$target"
    note "removed vibe-created AGENTS.md stub at $target (no user content remained)"
    return 0
  fi

  note "removed vibe managed blocks from $target (user content preserved)"
}

case "${1:-}" in
  link)    shift; link_adapter "${1:-}" "${2:-.}" ;;
  unmerge) shift; unmerge "${1:-.}" ;;
  *)       merge "${1:-.}" ;;
esac
