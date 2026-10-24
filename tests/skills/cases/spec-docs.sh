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

# --- skill-revision/10 — reference guides and templates agree with the validator ---
# sd_fence FILE HEADING [FENCE] — the first fenced block under the exact "## HEADING" line.
# FENCE is the fence string (default three backticks); a four-backtick fence holds inner fences.
sd_fence() {
  awk -v h="## $2" -v fence="${3:-\`\`\`}" '
    !f && /^## / { s = $0; next }
    s == h && index($0, fence) == 1 && substr($0, length(fence) + 1, 1) != "`" { if (f) exit; f = 1; next }
    s == h && f { print }
  ' "$1"
}
# sd_tpl_fill TEMPLATE FEATURE — a template with its placeholders replaced by fixed words.
sd_tpl_fill() {
  sed -e "s/{name}/$2/g" -e 's/{Name}/Demo/g' -e 's/{topic}/demo/g' -e 's/{YYYY-MM-DD}/2026-10-24/g' -e 's/{[^}]*}/filled/g' "$1"
}

_ref="spec/reference"
_tpl="$_ref/templates"
for _t in product tech design plan product-topic tech-topic plan-topic; do
  if awk '/<!--/ { f = 1 } f { print } f && /-->/ { exit }' "$_tpl/$_t.md" | grep -qF 'flat layout only'; then
    pass "skill-revision/10" "$_tpl/$_t.md is marked flat layout only"
  else
    fail "skill-revision/10" "$_tpl/$_t.md must carry a 'flat layout only' mark"
  fi
done
sd_present "skill-revision/10" "$_tpl/feature-product.md" "Do NOT: code, APIs, algorithms, plan units. File paths appear only in the Scope table's Owns row."
sd_present "skill-revision/10" "$_tpl/product-topic.md" '{topic}' 'updated: {YYYY-MM-DD}'
sd_present "skill-revision/10" "$_tpl/tech-topic.md" '{topic}' 'updated: {YYYY-MM-DD}'
sd_present "skill-revision/10" "$_tpl/plan-topic.md" '{topic}' 'updated: {YYYY-MM-DD}'
sd_absent "skill-revision/10" "$_tpl/product-topic.md" '<topic>' '2026-06-29'
sd_absent "skill-revision/10" "$_ref/product.md" 'warn-only check compares' 'validate.sh'
sd_present "skill-revision/10" "$_ref/product.md" 'No tool reads this list.'
sd_present "skill-revision/10" "$_ref/plan.md" 'NOT STARTED | IN PROGRESS | BLOCKED | DONE'
sd_absent "skill-revision/10" "$_ref/plan.md" 'validate.sh'
for _p in "$_ref/plan.md" "$_tpl/plan.md"; do
  if grep -qE '\|[[:space:]]*ACTIVE[[:space:]]*\||/ ACTIVE /' "$_p"; then fail "skill-revision/10" "$_p uses ACTIVE as a status value"; else pass "skill-revision/10" "$_p has no ACTIVE status value"; fi
done
sd_absent "skill-revision/10" "$_ref/design.md" 'same template'
sd_links_ok "$_ref/design.md" "skill-revision/10"
sd_links_ok "$_ref/plan.md" "skill-revision/10"
sd_links_ok "$_ref/product.md" "skill-revision/10"

# Behaviour: files written as the docs teach pass the validator.
_vs="$(sd_bin || true)"
if [[ -z "$_vs" ]]; then
  fail "skill-revision/10" "docs-to-validator fixture needs the vibe-spec binary (cargo build -p vibe-spec)"
else
  _fx="$(mktemp -d)"
  (
    cd "$_fx" && git init -q . && "$_vs" init >/dev/null
    _r="docs/spec"
    mkdir -p "$_r/features/demo" "$_r/features/example"
    for _k in product tech plan; do
      sd_tpl_fill "$REPO_ROOT/$_tpl/feature-$_k.md" demo > "$_r/features/demo/$_k.md"
      sed -e 's/demo/example/g' -e 's/filled/other/g' "$_r/features/demo/$_k.md" > "$_r/features/example/$_k.md"
    done
    sd_tpl_fill "$REPO_ROOT/$_tpl/research.md" demo > "$_r/features/demo/research.md"
    sd_fence "$REPO_ROOT/$_ref/plan.md" 'Example (feature plan)' '````' > "$_r/features/example/plan.md"
    printf '\n## Feature Sequence\n\n| Order | Feature | Deliverable | Test | Status | Starts when |\n|---:|---|---|---|---|---|\n| 1 | demo | files | `t` | IN PROGRESS | — |\n| 2 | example | files | `t` | IN PROGRESS | demo DONE |\n' >> "$_r/plan.md"
  )
  _val="$(cd "$_fx" && "$_vs" validate 2>&1 | sed 's/\x1b\[[0-9;]*m//g' || true)"
  _warns="$(printf '%s\n' "$_val" | grep -E 'WARN|ERROR' || true)"
  assert_eq "skill-revision/10" "validate prints no warning for a root built from the docs" "" "$_warns"
  (cd "$_fx" && "$_vs" check-drift >/dev/null 2>&1); assert_rc "skill-revision/10" "check-drift exits 0 on the fixture" 0 "$?"
  _sm="$(cd "$_fx" && "$_vs" scan-merges demo 2>&1 | grep -c 'features/demo' || true)"
  assert_eq "skill-revision/10" "the filled feature tech template has no live merge block" "0" "$_sm"
  # The token example of reference/design.md validates as the root design.md.
  _tok="$(awk '/Example — token frontmatter/ { f = 1 } f && /^```yaml/ { g = 1; next } g && /^```/ { exit } g { print }' "$REPO_ROOT/$_ref/design.md")"
  (cd "$_fx" && { printf '%s\n' "$_tok"; printf '\n# Design\n\nProse.\n'; } > docs/spec/design.md)
  _dv="$(cd "$_fx" && "$_vs" validate 2>&1 | sed 's/\x1b\[[0-9;]*m//g' | grep -E 'WARN|ERROR' || true)"
  assert_eq "skill-revision/10" "the design token example passes as the root design.md" "" "$_dv"
  rm -rf "$_fx"
fi

# --- skill-revision/11 — spec/README.md and spec/AGENTS.md ---------------------
_rd="spec/README.md"
_ag="spec/AGENTS.md"
for _f in "$_rd" "$_ag"; do
  sd_absent "skill-revision/11" "$_f" 'needs only' 'zero runtime' 'no build step' 'offered not required' 'validate.sh'
  if grep -qE 'becomes.*v0\.4|v0\.4.*becomes' "$_f"; then fail "skill-revision/11" "$_f still says it becomes something in v0.4"; else pass "skill-revision/11" "$_f has no future-v0.4 roadmap"; fi
done
sd_present "skill-revision/11" "$_rd" 'vibe-spec' 'using-spec' 'hooks/' 'agents/' 'docs/superpowers' 'layered' \
  'legacy (flat root only)'
if grep -E '^\|.*scripts/' "$_rd" | grep -qF 'legacy (flat root only)'; then
  pass "skill-revision/11" "README marks scripts/ legacy (flat root only)"
else
  fail "skill-revision/11" "README must mark scripts/ legacy (flat root only)"
fi
sd_present "skill-revision/11" "$_ag" 'cargo test -p vibe-spec' 'bash spec/tests/run.sh' 'bash spec/tests/agents.sh' \
  'bash spec/tests/pressure/run.sh'
if grep -F 'bash spec/tests/pressure/run.sh' "$_ag" | grep -qF 'claude CLI'; then
  pass "skill-revision/11" "AGENTS says the pressure run needs the claude CLI"
else
  fail "skill-revision/11" "AGENTS must say the pressure run needs the claude CLI"
fi
sd_links_ok "$_rd" "skill-revision/11"
