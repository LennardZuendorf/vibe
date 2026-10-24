# shellcheck shell=bash
# tests/skills/cases/spec-docs.sh — Track B lints: spec docs (skill-revision/8-11).
# Sourced by run.sh after lib.sh. Bash 3.2 safe.

# sd_absent <id> <file> <needle>… — none of the needles may appear in the file.
sd_absent() {
  local id="$1" f="$2" n
  shift 2
  for n in "$@"; do
    if grep -qF -- "$n" "$f"; then fail "$id" "$f must not contain: $n"; else pass "$id" "$f has no: $n"; fi
  done
}
# sd_present <id> <file> <needle>… — every needle must appear in the file.
sd_present() {
  local id="$1" f="$2" n
  shift 2
  for n in "$@"; do
    if grep -qF -- "$n" "$f"; then pass "$id" "$f has: $n"; else fail "$id" "$f must contain: $n"; fi
  done
}
# sd_slugs FILE — GitHub-style heading slugs, one per line, outside code fences.
sd_slugs() {
  awk '/^[ \t]*```/ { f = !f; next } !f && /^#+ / { print }' "$1" \
    | sed -e 's/^#* *//' \
    | tr '[:upper:]' '[:lower:]' \
    | sed -e 's/[^a-z0-9 _-]//g' -e 's/ /-/g'
}
# sd_anchors FILE LAYOUT_ROOT — every relative link#fragment resolves to a heading, in source and layout.
sd_anchors() {
  local f="$1" root="$2" bad=0 t frag path sdir ldir dir slug
  sdir="$(dirname "$f")"
  ldir="$(dirname "$(_layout_path "$f" "$root")")"
  while IFS= read -r t; do
    [[ -z "$t" ]] && continue
    path="${t%%#*}"; frag="${t#*#}"
    for dir in "$sdir" "$ldir"; do
      if [[ -z "$path" ]]; then
        if [[ "$dir" == "$sdir" ]]; then path_f="$f"; else path_f="$(_layout_path "$f" "$root")"; fi
      else
        path_f="$dir/$path"
      fi
      [[ -f "$path_f" ]] || continue
      slug="$(sd_slugs "$path_f" | grep -Fx -- "$frag" || true)"
      [[ -n "$slug" ]] || { echo "$f: anchor #$frag not found in $path_f"; bad=1; }
    done
  done <<EOA
$(awk '/^[ \t]*```/ { f = !f; next } !f { print }' "$f" | grep -Eo '\]\([^)[:space:]]*#[^)[:space:]]+\)' | sed -e 's/^](//' -e 's/)$//' | grep -Ev '^[A-Za-z][A-Za-z0-9+.-]*:' || true)
EOA
  return "$bad"
}
# sd_links_ok <id> <file> — links and anchors resolve in source and layout.
sd_links_ok() {
  local out
  if out="$(resolve_links "$1" "$REPO_ROOT"; sd_anchors "$1" "$REPO_ROOT")" && [[ -z "$out" ]]; then
    pass "$2" "$1 links and anchors resolve"
  else
    fail "$2" "$1 links and anchors resolve"; printf '%s\n' "$out" | sed 's/^/        /'
  fi
}

_SD_ANCHOR_T="$(mktemp -d)"
printf '# Title\n\n## Wrap-up\n\n[a](#wrap-up) [b](#gone)\n' > "$_SD_ANCHOR_T/a.md"
assert_eq "skill-revision/8" "sd_anchors flags a missing heading only" "1" \
  "$(sd_anchors "$_SD_ANCHOR_T/a.md" "$_SD_ANCHOR_T" | sort -u | grep -c gone)"
rm -rf "$_SD_ANCHOR_T"

# --- skill-revision/8 — spec/feature.md ---------------------------------------
_f8="spec/feature.md"
sd_absent "skill-revision/8" "$_f8" 'Superpower tip' 'suggest-superpowers' '.config.yaml' 'SKILL.md#config' \
  '/code:feature' 'merge-feature.sh' 'validate.sh' '§ Style' 'row added at COMPOUND' 'docs/spec/'
sd_present "skill-revision/8" "$_f8" '### 0. Size check' 'Wrap-up: spec SKILL.md § Wrap-up.' \
  'Config: spec reference/setup.md § Config.' \
  "exists with status \`IN PROGRESS\` before design" \
  "Write only the template's REQUIRED sections; omit empty ones."
_size_line="$(grep -nF '### 0. Size check' "$_f8" | head -n 1 | cut -d: -f1)"
_one_line="$(grep -nF '### 1.' "$_f8" | head -n 1 | cut -d: -f1)"
if [[ -n "$_size_line" && -n "$_one_line" && "$_size_line" -lt "$_one_line" ]]; then
  pass "skill-revision/8" "size check comes before step 1"
else
  fail "skill-revision/8" "size check comes before step 1"
fi
sd_links_ok "$_f8" "skill-revision/8"
