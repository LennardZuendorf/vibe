# shellcheck shell=bash
# shellcheck disable=SC2016  # needles are literal markdown, backticks included
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

# sd_bin — path of the built vibe-spec binary (built once; empty when cargo is absent).
sd_bin() {
  local b="$REPO_ROOT/target/debug/vibe-spec"
  if [[ ! -x "$b" ]] && command -v cargo >/dev/null 2>&1; then
    (cd "$REPO_ROOT" && cargo build -q --locked -p vibe-spec) >/dev/null 2>&1 || true
  fi
  [[ -x "$b" ]] && printf '%s\n' "$b"
}
# sd_lessons_fence FILE — the first fenced block under "## Lessons".
sd_lessons_fence() {
  awk '
    /^## / { s = $0; next }
    s == "## Lessons" && /^```/ { if (f) exit; f = 1; next }
    s == "## Lessons" && f { print }
  ' "$1"
}

# --- skill-revision/9 — spec/strategy.md and the lesson format ----------------
_f9="spec/strategy.md"
sd_absent "skill-revision/9" "$_f9" 'merge: true' 'merge-feature.sh' 'scripts/validate.sh' 'Superpower tip' \
  'suggest-superpowers' '.config.yaml' 'SKILL.md#config' 'docs/spec/'
if grep -qiF 'branch doc' "$_f9"; then fail "skill-revision/9" "$_f9 must not say 'branch doc'"; else pass "skill-revision/9" "$_f9 has no 'branch doc'"; fi
sd_present "skill-revision/9" "$_f9" 'Wrap-up: spec SKILL.md § Wrap-up.' 'Config: spec reference/setup.md § Config.' \
  '**Pinned-by:**' '`vibe-spec validate`' '`<!-- merge -->`' '`<!-- /merge -->`' '`vibe-spec promote <feature>`' \
  'superpowers specs and plans by default' '`vibe:rules`'
_fence="$(sd_lessons_fence "$_f9")"
for _k in '### ' '**Pattern:**' '**Rule:**' '**Tags:**' '**Date:**'; do
  assert_contains "skill-revision/9" "§ Lessons example shows $_k" "$_fence" "$_k"
done
sd_links_ok "$_f9" "skill-revision/9"

# Behaviour: the § Lessons example, filled in, passes the validator and the lessons query.
_vs="$(sd_bin || true)"
if [[ -z "$_vs" ]]; then
  fail "skill-revision/9" "lesson round-trip needs the vibe-spec binary (cargo build -p vibe-spec)"
else
  _rt="$(mktemp -d)"
  (
    cd "$_rt" && git init -q . && "$_vs" init >/dev/null
    printf '\n%s\n' "$_fence" \
      | sed -e 's/^\*\*Tags:\*\*.*/**Tags:** roundtrip/' -e 's/YYYY-MM-DD/2026-10-24/' -e 's/<[^>]*>/filled/g' \
      >> docs/spec/lessons.md
  )
  _val="$(cd "$_rt" && "$_vs" validate 2>&1 || true)"
  _got="$(cd "$_rt" && "$_vs" lessons-for roundtrip 2>&1 || true)"
  if [[ "$_val" == *SF16* ]]; then fail "skill-revision/9" "filled example trips SF16"; else pass "skill-revision/9" "filled example passes SF16"; fi
  assert_contains "skill-revision/9" "lessons-for returns the filled example" "$_got" "**Tags:** roundtrip"
  rm -rf "$_rt"
fi
