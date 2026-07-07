#!/usr/bin/env bash
# spec/tests/run.sh — behaviour tests for the spec-framework SF0–SF4 units.
# Pure bash; no bats dependency. Each test cites its plan unit ID (SF0…SF3, D9).
#
# Usage: bash spec/tests/run.sh
# Exit 0 = all pass; non-zero = at least one failure.

set -uo pipefail

# Repo root by upward marker search (.spec / .git) — depth- and symlink-agnostic:
# resolves the physical path so real and symlinked invocations converge.
_find_repo_root() {
  local d; d="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"
  while [[ "$d" != "/" ]]; do
    [[ -d "$d/.spec" || -e "$d/.git" ]] && { printf '%s\n' "$d"; return 0; }
    d="$(dirname "$d")"
  done
  return 1
}
REPO_ROOT="$(_find_repo_root)" || { echo "cannot locate repo root (.spec/.git)" >&2; exit 1; }
SPEC_SKILL="$REPO_ROOT/.agents/skills/spec"
SETUP="$SPEC_SKILL/scripts/setup.sh"
VALIDATE="$SPEC_SKILL/scripts/validate.sh"
LIST="$SPEC_SKILL/scripts/list-specs.sh"

PASS=0
FAIL=0

pass() { echo "  PASS [$1] $2"; PASS=$((PASS + 1)); }
fail() { echo "  FAIL [$1] $2"; FAIL=$((FAIL + 1)); }

# assert_contains <id> <desc> <haystack> <needle>
assert_contains() {
  if [[ "$3" == *"$4"* ]]; then pass "$1" "$2"; else
    fail "$1" "$2"; echo "        expected to contain: $4"; fi
}
# assert_not_contains <id> <desc> <haystack> <needle>
assert_not_contains() {
  if [[ "$3" != *"$4"* ]]; then pass "$1" "$2"; else
    fail "$1" "$2"; echo "        expected NOT to contain: $4"; fi
}

mktmp() { mktemp -d "${TMPDIR:-/tmp}/spec-test.XXXXXX"; }

# ── SF0: setup.sh lessons bootstrap carries **Tags:** ───────────────────────
test_sf0_setup_tags() {
  local d; d="$(mktmp)"; ( cd "$d" && bash "$SETUP" >/dev/null 2>&1 )
  local body; body="$(cat "$d/.spec/lessons.md" 2>/dev/null || true)"
  assert_contains SF0 "setup.sh lessons.md template includes **Tags:**" "$body" '**Tags:**'
  rm -rf "$d"
}

# ── SF0: validate.sh warns on a tag-less lesson entry ───────────────────────
test_sf0_validate_tagless_warn() {
  local d; d="$(mktmp)"; mkdir -p "$d/.spec"
  cat > "$d/.spec/product.md" <<'EOF'
---
type: entrypoint
scope: product
children: []
updated: 2026-06-06
---
# P
EOF
  cat > "$d/.spec/lessons.md" <<'EOF'
# Lessons

### Tagless lesson
**Pattern:** x
**Rule:** y
**Date:** 2026-06-06
EOF
  local out; out="$( cd "$d" && bash "$VALIDATE" 2>&1 )"
  assert_contains SF0 "validate.sh warns on lesson missing **Tags:**" "$out" "Tags"
  rm -rf "$d"
}

# ── SF0: validate.sh stays quiet on a properly tagged lesson ────────────────
test_sf0_validate_tagged_ok() {
  local d; d="$(mktmp)"; mkdir -p "$d/.spec"
  cat > "$d/.spec/product.md" <<'EOF'
---
type: entrypoint
scope: product
children: []
updated: 2026-06-06
---
# P
EOF
  cat > "$d/.spec/lessons.md" <<'EOF'
# Lessons

### Tagged lesson
**Pattern:** x
**Rule:** y
**Tags:** alpha, beta
**Date:** 2026-06-06
EOF
  local out; out="$( cd "$d" && bash "$VALIDATE" 2>&1 )"
  assert_not_contains SF0 "validate.sh no tag-warning when **Tags:** present" "$out" "missing '**Tags:**'"
  rm -rf "$d"
}

# ── SF1: design writing guide exists and SKILL.md links it ──────────────────
test_sf1_design_guide() {
  if [[ -f "$SPEC_SKILL/reference/design.md" ]]; then
    pass SF1 "reference/design.md writing guide exists"
  else
    fail SF1 "reference/design.md writing guide exists"
  fi
  local skill; skill="$(cat "$SPEC_SKILL/SKILL.md")"
  assert_contains SF1 "SKILL.md links reference/design.md guide" "$skill" "reference/design.md)"
}

# ── SF2: list-specs.sh maps design.md to area 'design' (not unknown) ─────────
test_sf2_list_design_area() {
  local d; d="$(mktmp)"; mkdir -p "$d/.spec/features/foo"
  cat > "$d/.spec/design.md" <<'EOF'
---
type: entrypoint
scope: design
updated: 2026-06-06
---
# D
EOF
  cat > "$d/.spec/features/foo/design.md" <<'EOF'
---
type: feature-design
updated: 2026-06-06
---
# FD
EOF
  local out; out="$( cd "$d" && bash "$LIST" 2>&1 )"
  assert_contains SF2 "root design.md tagged [design]" "$out" "design.md [design]"
  assert_not_contains SF2 "root design.md not [unknown]" "$out" "design.md [unknown]"
  assert_contains SF2 "feature design.md surfaced" "$out" "features/foo/design.md"
  rm -rf "$d"
}

# ── SF3: validate.sh passes a well-formed token doc ─────────────────────────
test_sf3_design_tokens_ok() {
  local d; d="$(mktmp)"; mkdir -p "$d/.spec"
  cat > "$d/.spec/design.md" <<'EOF'
---
type: entrypoint
scope: design
children: []
updated: 2026-06-06
colors:
  primary: "#000000"
typography:
  body:
    fontSize: 16px
---
# D
EOF
  local out rc
  out="$( cd "$d" && bash "$VALIDATE" 2>&1 )"; rc=$?
  assert_contains SF3 "valid token doc passes (exit 0)" "rc=$rc" "rc=0"
  assert_not_contains SF3 "valid token doc raises no token warning" "$out" "empty token group"
  rm -rf "$d"
}

# ── SF3: validate.sh warns on an empty token group ──────────────────────────
test_sf3_design_tokens_malformed_warn() {
  local d; d="$(mktmp)"; mkdir -p "$d/.spec"
  cat > "$d/.spec/design.md" <<'EOF'
---
type: entrypoint
scope: design
children: []
updated: 2026-06-06
colors:
typography:
  body:
    fontSize: 16px
---
# D
EOF
  local out; out="$( cd "$d" && bash "$VALIDATE" 2>&1 )"
  assert_contains SF3 "empty token group warns" "$out" "empty token group"
  rm -rf "$d"
}

# ── SF4: lint_design_md skips when VIBE_DESIGN_LINT unset ───────────────────
test_sf4_design_lint_skips() {
  local d; d="$(mktmp)"; mkdir -p "$d/.spec"
  cat > "$d/.spec/product.md" <<'EOF'
---
type: entrypoint
scope: product
children: []
updated: 2026-06-06
---
# P
EOF
  cat > "$d/.spec/design.md" <<'EOF'
---
type: entrypoint
scope: design
children: []
updated: 2026-06-06
colors:
  primary: "#000000"
---
# D
EOF
  local out; out="$( cd "$d" && bash "$VALIDATE" 2>&1 )"
  assert_not_contains SF4 "lint_design_md silent when VIBE_DESIGN_LINT unset" "$out" "design.md lint"
  assert_not_contains SF4 "lint_design_md does not invoke npx when unset" "$out" "npx"
  rm -rf "$d"
}

# ── SF10: RFC-2119 keyword in requirement body (not only title) ─────────────
test_sf10_rfc_in_body_ok() {
  local d; d="$(mktmp)"; mkdir -p "$d/.spec"
  cat > "$d/.spec/product.md" <<'EOF'
---
type: entrypoint
scope: product
children: []
updated: 2026-06-06
---
# P

## Requirements

### Requirement: Plain title

The system SHALL behave correctly.

#### Scenario: Happy path

- **Given** x
- **When** y
- **Then** z
EOF
  local out; out="$( cd "$d" && bash "$VALIDATE" 2>&1 )"
  assert_not_contains SF10 "RFC keyword in body passes without warning" "$out" "missing RFC-2119 keyword"
  rm -rf "$d"
}

# ── SF10: missing RFC-2119 in title and body still warns ────────────────────
test_sf10_rfc_missing_warn() {
  local d; d="$(mktmp)"; mkdir -p "$d/.spec"
  cat > "$d/.spec/product.md" <<'EOF'
---
type: entrypoint
scope: product
children: []
updated: 2026-06-06
---
# P

## Requirements

### Requirement: Plain title

The system behaves correctly.

#### Scenario: Happy path

- **Given** x
- **When** y
- **Then** z
EOF
  local out; out="$( cd "$d" && bash "$VALIDATE" 2>&1 )"
  assert_contains SF10 "missing RFC keyword warns" "$out" "missing RFC-2119 keyword"
  rm -rf "$d"
}

# ── SF11: ## Milestones is not treated as a milestone section ───────────────
test_sf11_milestones_heading_ok() {
  local d; d="$(mktmp)"; mkdir -p "$d/.spec"
  cat > "$d/.spec/plan.md" <<'EOF'
---
type: entrypoint
scope: plan
children: []
updated: 2026-06-06
---
# Plan

## Milestones

| Milestone | Goal |
|---|---|
| M0 | Bootstrap |

### SF1 — Example unit

Detail belongs in feature plan, not here when under M0 — but under Milestones it is fine.
EOF
  local out; out="$( cd "$d" && bash "$VALIDATE" 2>&1 )"
  assert_not_contains SF11 "## Milestones does not trigger embed warning" "$out" "milestone section embeds unit heading"
  rm -rf "$d"
}

# ── SF11: ## M0 still flags embedded unit headings (new feature/n shape) ─────
test_sf11_m0_embed_warn() {
  local d; d="$(mktmp)"; mkdir -p "$d/.spec"
  cat > "$d/.spec/plan.md" <<'EOF'
---
type: entrypoint
scope: plan
children: []
updated: 2026-06-06
---
# Plan

## M0 — Bootstrap

### vibe-flow/1 — Example unit

This unit detail should live in a feature plan.
EOF
  local out; out="$( cd "$d" && bash "$VALIDATE" 2>&1 )"
  assert_contains SF11 "## M0 embeds feature/n unit heading warns" "$out" "milestone section embeds unit heading"
  rm -rf "$d"
}

# ── SF11: feature plan with feature/n headings is accepted ───────────────────
test_sf11_feature_plan_new_shape_ok() {
  local d; d="$(mktmp)"; mkdir -p "$d/.spec/features/myfeature"
  cat > "$d/.spec/product.md" <<'EOF'
---
type: entrypoint
scope: product
children: []
updated: 2026-06-06
---
# P
EOF
  cat > "$d/.spec/features/myfeature/product.md" <<'EOF'
---
type: feature-product
feature: myfeature
updated: 2026-06-06
---
# Feature
EOF
  cat > "$d/.spec/features/myfeature/tech.md" <<'EOF'
---
type: feature-tech
feature: myfeature
updated: 2026-06-06
---
# Tech
EOF
  cat > "$d/.spec/features/myfeature/plan.md" <<'EOF'
---
type: feature-plan
feature: myfeature
parent: ../../plan.md
updated: 2026-06-06
---
# Plan

## Requirements Trace

| ID | Requirement | Units |
|---|---|---|
| R1 | thing | myfeature/1 |

### myfeature/1 — first slice

**Dependencies:** —
EOF
  local out; out="$( cd "$d" && bash "$VALIDATE" 2>&1 )"
  assert_not_contains SF11 "feature/n heading accepted (no missing-units warn)" "$out" "missing '### <feature>/<n>'"
  rm -rf "$d"
}

# ── I1: cross-feature unit dependency in a feature plan warns ────────────────
test_i1_cross_feature_dep_warn() {
  local d; d="$(mktmp)"; mkdir -p "$d/.spec/features/myfeature"
  cat > "$d/.spec/product.md" <<'EOF'
---
type: entrypoint
scope: product
children: []
updated: 2026-06-06
---
# P
EOF
  cat > "$d/.spec/features/myfeature/product.md" <<'EOF'
---
type: feature-product
feature: myfeature
updated: 2026-06-06
---
# Feature
EOF
  cat > "$d/.spec/features/myfeature/tech.md" <<'EOF'
---
type: feature-tech
feature: myfeature
updated: 2026-06-06
---
# Tech
EOF
  cat > "$d/.spec/features/myfeature/plan.md" <<'EOF'
---
type: feature-plan
feature: myfeature
parent: ../../plan.md
updated: 2026-06-06
---
# Plan

## Requirements Trace

| ID | Requirement | Units |
|---|---|---|
| R1 | thing | myfeature/1 |

### myfeature/1 — first slice

**Dependencies:** otherfeature/2
EOF
  local out; out="$( cd "$d" && bash "$VALIDATE" 2>&1 )"
  assert_contains I1 "cross-feature unit dep warns" "$out" "cross-feature unit dependency 'otherfeature/2'"
  rm -rf "$d"
}

# ── I1: same-feature dependency does not warn ───────────────────────────────
test_i1_same_feature_dep_ok() {
  local d; d="$(mktmp)"; mkdir -p "$d/.spec/features/myfeature"
  cat > "$d/.spec/product.md" <<'EOF'
---
type: entrypoint
scope: product
children: []
updated: 2026-06-06
---
# P
EOF
  cat > "$d/.spec/features/myfeature/product.md" <<'EOF'
---
type: feature-product
feature: myfeature
updated: 2026-06-06
---
# Feature
EOF
  cat > "$d/.spec/features/myfeature/tech.md" <<'EOF'
---
type: feature-tech
feature: myfeature
updated: 2026-06-06
---
# Tech
EOF
  cat > "$d/.spec/features/myfeature/plan.md" <<'EOF'
---
type: feature-plan
feature: myfeature
parent: ../../plan.md
updated: 2026-06-06
---
# Plan

## Requirements Trace

| ID | Requirement | Units |
|---|---|---|
| R1 | thing | myfeature/1, myfeature/2 |

### myfeature/1 — first slice

**Dependencies:** —

### myfeature/2 — second slice

**Dependencies:** myfeature/1
EOF
  local out; out="$( cd "$d" && bash "$VALIDATE" 2>&1 )"
  assert_not_contains I1 "same-feature dep does not warn" "$out" "cross-feature unit dependency"
  rm -rf "$d"
}

# ── SF17: SKILL.md frontmatter has required discovery fields ─────────────────
test_sf17_skill_frontmatter() {
  local fm; fm="$(awk '/^---$/{c++; next} c==1{print; if(/^---$/){exit}}' "$SPEC_SKILL/SKILL.md")"
  assert_contains SF17 "SKILL.md frontmatter has name:" "$fm" "name: spec"
  assert_contains SF17 "SKILL.md frontmatter has description:" "$fm" "description:"
}

# ── SF17: argument-hint covers every routing keyword ─────────────────────────
test_sf17_argument_hint() {
  local hint; hint="$(awk '/^argument-hint:/{sub(/^argument-hint:[[:space:]]*/,""); print; exit}' "$SPEC_SKILL/SKILL.md")"
  local arg
  for arg in strategy feature product tech design plan lessons setup validate; do
    assert_contains SF17 "argument-hint includes '$arg'" "$hint" "$arg"
  done
  assert_not_contains SF17 "argument-hint omits spurious 'spec' route" "$hint" "|spec"
}

# ── SF17: SKILL.md internal links resolve under skill bundle ─────────────────
test_sf17_skill_links() {
  local skill dir link target missing=0
  skill="$(cat "$SPEC_SKILL/SKILL.md")"
  dir="$SPEC_SKILL"
  while IFS= read -r link; do
    [[ "$link" == http* ]] && continue
    [[ "$link" == \#* ]] && continue
    target="${link%%#*}"
    [[ -z "$target" ]] && continue
    if [[ ! -e "$dir/$target" ]]; then
      fail SF17 "SKILL.md link resolves: $link"
      missing=1
    fi
  done < <(printf '%s' "$skill" | grep -oE '\[[^]]+\]\([^)]+\)' | sed 's/.*(\([^)]*\)).*/\1/')
  [[ $missing -eq 0 ]] && pass SF17 "SKILL.md internal links resolve"
}

# ── SF17: template inventory matches reference/templates/ ────────────────────
test_sf17_templates() {
  local expected=(
    product.md tech.md design.md plan.md
    feature-product.md feature-tech.md feature-plan.md feature-design.md
  )
  local t
  for t in "${expected[@]}"; do
    if [[ -f "$SPEC_SKILL/reference/templates/$t" ]]; then
      pass SF17 "template exists: $t"
    else
      fail SF17 "template exists: $t"
    fi
  done
}

# ── SF17: SKILL.md stays within skill size budget ────────────────────────────
test_sf17_skill_size() {
  local lines; lines="$(wc -l < "$SPEC_SKILL/SKILL.md" | tr -d ' ')"
  if [[ "$lines" -lt 500 ]]; then
    pass SF17 "SKILL.md under 500 lines ($lines)"
  else
    fail SF17 "SKILL.md under 500 lines ($lines)"
  fi
}

# ── SF17: companion entrypoints exist ────────────────────────────────────────
test_sf17_companion_docs() {
  for f in strategy.md feature.md; do
    if [[ -f "$SPEC_SKILL/$f" ]]; then pass SF17 "companion doc exists: $f"
    else fail SF17 "companion doc exists: $f"; fi
  done
}

echo "=== spec-framework SF0–SF4 behaviour tests ==="
test_sf0_setup_tags
test_sf0_validate_tagless_warn
test_sf0_validate_tagged_ok
test_sf1_design_guide
test_sf2_list_design_area
test_sf3_design_tokens_ok
test_sf3_design_tokens_malformed_warn
test_sf4_design_lint_skips

echo ""
echo "=== spec-framework SF10–SF11 behaviour tests ==="
test_sf10_rfc_in_body_ok
test_sf10_rfc_missing_warn
test_sf11_milestones_heading_ok
test_sf11_m0_embed_warn
test_sf11_feature_plan_new_shape_ok

echo ""
echo "=== interdependence (I1) behaviour tests ==="
test_i1_cross_feature_dep_warn
test_i1_same_feature_dep_ok

echo ""
echo "=== spec-framework SF17 skill-bundle tests ==="
test_sf17_skill_frontmatter
test_sf17_argument_hint
test_sf17_skill_links
test_sf17_templates
test_sf17_skill_size
test_sf17_companion_docs

# ── spec-skill-improvements: Unit 1 — SKILL.md v2.0 frontmatter ─────────────
test_skill_v2_frontmatter_fields() {
  local skill; skill="$(cat "$SPEC_SKILL/SKILL.md")"
  assert_contains "spec-skill-improvements/1" "SKILL.md version 2.0" "$skill" "version: 2.0"
  assert_contains "spec-skill-improvements/1" "SKILL.md has allowed-tools:" "$skill" "allowed-tools:"
  assert_contains "spec-skill-improvements/1" "SKILL.md has context:" "$skill" "context:"
  assert_contains "spec-skill-improvements/1" "SKILL.md has agents:" "$skill" "agents:"
  assert_contains "spec-skill-improvements/1" "SKILL.md has superpowers:" "$skill" "superpowers:"
  assert_contains "spec-skill-improvements/1" "SKILL.md has delegates:" "$skill" "delegates:"
  assert_contains "spec-skill-improvements/1" "SKILL.md has caveman:" "$skill" "caveman:"
}

# ── spec-skill-improvements: Unit 2 — ## Roles section ──────────────────────
test_skill_roles_section() {
  local skill; skill="$(cat "$SPEC_SKILL/SKILL.md")"
  assert_contains "spec-skill-improvements/2" "SKILL.md has ## Roles" "$skill" "## Roles"
  assert_contains "spec-skill-improvements/2" "SKILL.md has spec-interviewer role" "$skill" "spec-interviewer"
  assert_contains "spec-skill-improvements/2" "SKILL.md has spec-planner role" "$skill" "spec-planner"
  assert_contains "spec-skill-improvements/2" "SKILL.md has spec-auditor role" "$skill" "spec-auditor"
  assert_contains "spec-skill-improvements/2" "SKILL.md has spec-compactor role" "$skill" "spec-compactor"
}

# ── spec-skill-improvements: Unit 3 — Routing expansion ──────────────────────
test_skill_routing_new_routes() {
  local skill; skill="$(cat "$SPEC_SKILL/SKILL.md")"
  assert_contains "spec-skill-improvements/3" "SKILL.md routing has interview" "$skill" "interview"
  assert_contains "spec-skill-improvements/3" "SKILL.md routing has promote" "$skill" "promote"
  assert_contains "spec-skill-improvements/3" "SKILL.md routing has audit" "$skill" "audit"
  assert_contains "spec-skill-improvements/3" "SKILL.md routing has lessons-for" "$skill" "lessons-for"
  assert_contains "spec-skill-improvements/3" "SKILL.md routing has diff" "$skill" "diff"
  assert_contains "spec-skill-improvements/3" "SKILL.md routing has health" "$skill" "health"
  assert_contains "spec-skill-improvements/3" "SKILL.md routing has research" "$skill" "research"
}

# ── spec-skill-improvements: Unit 4 — feature.md output profiles ─────────────
test_feature_output_profiles() {
  local fm; fm="$(cat "$SPEC_SKILL/feature.md")"
  assert_contains "spec-skill-improvements/4" "feature.md has ## Output profiles" "$fm" "## Output profiles"
  assert_contains "spec-skill-improvements/4" "feature.md has Lite profile" "$fm" "### Lite"
  assert_contains "spec-skill-improvements/4" "feature.md has Ultra profile" "$fm" "### Ultra"
}

# ── spec-skill-improvements: Unit 5 — promote.sh dry-run no mutation ─────────
test_promote_dry_run_no_mutation() {
  local d; d="$(mktmp)"; mkdir -p "$d/.spec/features/feat"
  printf '<!-- merge -->\n## promoted section\n<!-- /merge -->\n' > "$d/.spec/features/feat/tech.md"
  echo "# root" > "$d/.spec/tech.md"
  local before; before="$(cat "$d/.spec/tech.md")"
  (cd "$d" && bash "$SPEC_SKILL/scripts/promote.sh" feat --dry-run) >/dev/null
  local after; after="$(cat "$d/.spec/tech.md")"
  assert_contains "spec-skill-improvements/5" "dry-run leaves target unchanged" "$before" "$after"
  rm -rf "$d"
}

test_promote_reversed_markers_refused() {
  local d; d="$(mktmp)"; mkdir -p "$d/.spec/features/feat"
  printf '<!-- /merge -->\n## bad\n<!-- merge -->\n' > "$d/.spec/features/feat/tech.md"
  echo "# root" > "$d/.spec/tech.md"
  local before; before="$(cat "$d/.spec/tech.md")"
  local rc=0
  (cd "$d" && bash "$SPEC_SKILL/scripts/promote.sh" feat) >/dev/null 2>&1 || rc=$?
  local after; after="$(cat "$d/.spec/tech.md")"
  if [[ $rc -ne 0 ]]; then pass "spec-skill-improvements/5" "reversed markers exit non-zero"
  else fail "spec-skill-improvements/5" "reversed markers exit non-zero"; fi
  assert_contains "spec-skill-improvements/5" "reversed markers: target byte-unchanged" "$before" "$after"
  rm -rf "$d"
}

test_promote_valid_blocks() {
  local d; d="$(mktmp)"; mkdir -p "$d/.spec/features/feat"
  printf '<!-- merge -->\n## promoted section\n<!-- /merge -->\n' > "$d/.spec/features/feat/tech.md"
  echo "# root" > "$d/.spec/tech.md"
  (cd "$d" && bash "$SPEC_SKILL/scripts/promote.sh" feat) >/dev/null
  local after; after="$(cat "$d/.spec/tech.md")"
  assert_contains "spec-skill-improvements/5" "valid blocks appended to target" "$after" "promoted section"
  rm -rf "$d"
}

# ── spec-skill-improvements: Unit 6 — lessons-for.sh ───────────────────────
test_lessons_for_match() {
  local d; d="$(mktmp)"; mkdir -p "$d/.spec"
  printf '### Test lesson\n**Pattern:** p\n**Rule:** r\n**Tags:** spec, validate\n**Date:** 2026-01-01\n' > "$d/.spec/lessons.md"
  local out; out="$(cd "$d" && bash "$SPEC_SKILL/scripts/lessons-for.sh" validate)"
  assert_contains "spec-skill-improvements/6" "lessons-for tag match returns lesson" "$out" "Test lesson"
  rm -rf "$d"
}

test_lessons_for_no_match() {
  local d; d="$(mktmp)"; mkdir -p "$d/.spec"
  printf '### Test lesson\n**Pattern:** p\n**Rule:** r\n**Tags:** spec\n**Date:** 2026-01-01\n' > "$d/.spec/lessons.md"
  local out rc=0
  out="$(cd "$d" && bash "$SPEC_SKILL/scripts/lessons-for.sh" nonexistent 2>/dev/null)" || rc=$?
  if [[ $rc -eq 0 ]]; then pass "spec-skill-improvements/6" "no-match exit 0"
  else fail "spec-skill-improvements/6" "no-match exit 0"; fi
  assert_not_contains "spec-skill-improvements/6" "no-match empty output" "$out" "Test lesson"
  rm -rf "$d"
}

test_lessons_for_inject_format() {
  local d; d="$(mktmp)"; mkdir -p "$d/.spec"
  printf '### Test lesson\n**Pattern:** p\n**Rule:** r\n**Tags:** spec\n**Date:** 2026-01-01\n' > "$d/.spec/lessons.md"
  local out; out="$(cd "$d" && bash "$SPEC_SKILL/scripts/lessons-for.sh" spec --format inject)"
  assert_contains "spec-skill-improvements/6" "inject format has open delimiter" "$out" "<!-- lessons:"
  assert_contains "spec-skill-improvements/6" "inject format has close delimiter" "$out" "<!-- /lessons -->"
  rm -rf "$d"
}

# ── spec-skill-improvements: Unit 7 — SF13 stale link checker ───────────────
test_sf13_stale_link_warns() {
  local d; d="$(mktmp)"; mkdir -p "$d/.spec"
  cat > "$d/.spec/product.md" <<'EOF'
---
type: entrypoint
scope: product
children: []
updated: 2026-01-01
---
[old](features/deleted/product.md)
EOF
  local out; out="$(cd "$d" && bash "$VALIDATE" 2>&1)"
  assert_contains "spec-skill-improvements/7" "SF13 warns on stale feature link" "$out" "SF13"
  rm -rf "$d"
}

test_sf13_valid_links_pass() {
  local d; d="$(mktmp)"; mkdir -p "$d/.spec/features/existing"
  cat > "$d/.spec/product.md" <<'EOF'
---
type: entrypoint
scope: product
children: []
updated: 2026-01-01
---
[here](features/existing/product.md)
EOF
  touch "$d/.spec/features/existing/product.md"
  local out; out="$(cd "$d" && bash "$VALIDATE" 2>&1)"
  assert_not_contains "spec-skill-improvements/7" "SF13 silent on valid links" "$out" "SF13"
  rm -rf "$d"
}

# ── spec-skill-improvements: Unit 8 — SF14 scope conflict ───────────────────
test_sf14_conflict_warns() {
  local d; d="$(mktmp)"; mkdir -p "$d/.spec/features/alpha" "$d/.spec/features/beta"
  cat > "$d/.spec/product.md" <<'EOF'
---
type: entrypoint
scope: product
children: []
updated: 2026-01-01
---
# P
EOF
  cat > "$d/.spec/features/alpha/product.md" <<'EOF'
---
type: feature-product
feature: alpha
sibling: tech.md
parent: ../../product.md
updated: 2026-01-01
---
## Scope
| Owns | Does not own |
|---|---|
| state machine | adapters |
EOF
  cat > "$d/.spec/features/alpha/tech.md" <<'EOF'
---
type: feature-tech
feature: alpha
sibling: product.md
parent: ../../tech.md
updated: 2026-01-01
---
# Tech
EOF
  cat > "$d/.spec/features/beta/product.md" <<'EOF'
---
type: feature-product
feature: beta
sibling: tech.md
parent: ../../product.md
updated: 2026-01-01
---
## Scope
| Owns | Does not own |
|---|---|
| state machine | hooks |
EOF
  cat > "$d/.spec/features/beta/tech.md" <<'EOF'
---
type: feature-tech
feature: beta
sibling: product.md
parent: ../../tech.md
updated: 2026-01-01
---
# Tech
EOF
  local out; out="$(cd "$d" && bash "$VALIDATE" 2>&1)"
  assert_contains "spec-skill-improvements/8" "SF14 warns on conflicting Owns terms" "$out" "SF14"
  assert_contains "spec-skill-improvements/8" "SF14 message names first feature" "$out" "alpha"
  assert_contains "spec-skill-improvements/8" "SF14 message names second feature" "$out" "beta"
  rm -rf "$d"
}

test_sf14_no_conflict_passes() {
  local d; d="$(mktmp)"; mkdir -p "$d/.spec/features/alpha" "$d/.spec/features/beta"
  cat > "$d/.spec/product.md" <<'EOF'
---
type: entrypoint
scope: product
children: []
updated: 2026-01-01
---
# P
EOF
  cat > "$d/.spec/features/alpha/product.md" <<'EOF'
---
type: feature-product
feature: alpha
sibling: tech.md
parent: ../../product.md
updated: 2026-01-01
---
## Scope
| Owns | Does not own |
|---|---|
| state machine | adapters |
EOF
  cat > "$d/.spec/features/alpha/tech.md" <<'EOF'
---
type: feature-tech
feature: alpha
sibling: product.md
parent: ../../tech.md
updated: 2026-01-01
---
# Tech
EOF
  cat > "$d/.spec/features/beta/product.md" <<'EOF'
---
type: feature-product
feature: beta
sibling: tech.md
parent: ../../product.md
updated: 2026-01-01
---
## Scope
| Owns | Does not own |
|---|---|
| routing hooks | adapters |
EOF
  cat > "$d/.spec/features/beta/tech.md" <<'EOF'
---
type: feature-tech
feature: beta
sibling: product.md
parent: ../../tech.md
updated: 2026-01-01
---
# Tech
EOF
  local out; out="$(cd "$d" && bash "$VALIDATE" 2>&1)"
  assert_not_contains "spec-skill-improvements/8" "SF14 silent on distinct Owns terms" "$out" "SF14"
  rm -rf "$d"
}

# ── spec-skill-improvements: Unit 9 — branch doc templates ──────────────────
test_branch_doc_templates_exist() {
  local templates_dir="$SPEC_SKILL/reference/templates"
  for t in product-topic.md tech-topic.md plan-topic.md research.md; do
    if [[ -f "$templates_dir/$t" ]]; then pass "spec-skill-improvements/9" "template exists: $t"
    else fail "spec-skill-improvements/9" "template exists: $t"; fi
  done
  local product_topic; product_topic="$(cat "$templates_dir/product-topic.md")"
  local tech_topic; tech_topic="$(cat "$templates_dir/tech-topic.md")"
  assert_contains "spec-skill-improvements/9" "product-topic.md has type: product-topic" "$product_topic" "type: product-topic"
  assert_contains "spec-skill-improvements/9" "tech-topic.md has type: tech-topic" "$tech_topic" "type: tech-topic"
}

# ── spec-skill-improvements: Unit 10 — scan-merges.sh ───────────────────────
test_scan_merges_finds_blocks() {
  local d; d="$(mktmp)"; mkdir -p "$d/.spec/features/feat"
  printf '# Tech\n<!-- merge -->\n## promoted\n<!-- /merge -->\nmore text\n' > "$d/.spec/features/feat/tech.md"
  local out; out="$(cd "$d" && bash "$SPEC_SKILL/scripts/scan-merges.sh" feat)"
  assert_contains "spec-skill-improvements/10" "scan-merges finds blocks" "$out" "feat"
  rm -rf "$d"
}

test_scan_merges_unclosed_exits_nonzero() {
  local d; d="$(mktmp)"; mkdir -p "$d/.spec/features/feat"
  printf '<!-- merge -->\n## unclosed block\n' > "$d/.spec/features/feat/tech.md"
  local rc=0
  (cd "$d" && bash "$SPEC_SKILL/scripts/scan-merges.sh" feat) >/dev/null 2>&1 || rc=$?
  if [[ $rc -ne 0 ]]; then pass "spec-skill-improvements/10" "unclosed marker exits non-zero"
  else fail "spec-skill-improvements/10" "unclosed marker exits non-zero"; fi
  rm -rf "$d"
}

test_scan_merges_empty_feature() {
  local d; d="$(mktmp)"; mkdir -p "$d/.spec/features/feat"
  echo "# Tech only, no merge blocks" > "$d/.spec/features/feat/tech.md"
  local rc=0
  (cd "$d" && bash "$SPEC_SKILL/scripts/scan-merges.sh" feat) >/dev/null || rc=$?
  if [[ $rc -eq 0 ]]; then pass "spec-skill-improvements/10" "no-block feature exits 0"
  else fail "spec-skill-improvements/10" "no-block feature exits 0"; fi
  rm -rf "$d"
}

# ── spec-skill-improvements: Unit 11 — OpenSpec frontmatter docs ─────────────
test_openspec_docs_present() {
  local product_ref; product_ref="$(cat "$SPEC_SKILL/reference/product.md")"
  local plan_ref; plan_ref="$(cat "$SPEC_SKILL/reference/plan.md")"
  assert_contains "spec-skill-improvements/11" "reference/product.md has requirements: section" "$product_ref" "requirements:"
  assert_contains "spec-skill-improvements/11" "reference/plan.md has units: section" "$plan_ref" "units:"
  assert_contains "spec-skill-improvements/11" "reference/product.md notes opt-in" "$product_ref" "opt-in"
}

# ── spec-skill-improvements: Unit 12 — spec-tracer read-only ─────────────────
test_spec_tracer_read_only() {
  local tracer; tracer="$(cat "$SPEC_SKILL/agents/spec-tracer/SKILL.md")"
  assert_contains "spec-skill-improvements/12" "spec-tracer has allowed-tools" "$tracer" "allowed-tools"
  assert_contains "spec-skill-improvements/12" "spec-tracer lists Read" "$tracer" "Read"
  assert_not_contains "spec-skill-improvements/12" "spec-tracer has no Edit in allowed-tools" "$(grep -A5 'allowed-tools' "$SPEC_SKILL/agents/spec-tracer/SKILL.md" | head -5)" "Edit"
  assert_not_contains "spec-skill-improvements/12" "spec-tracer has no Write in allowed-tools" "$(grep -A5 'allowed-tools' "$SPEC_SKILL/agents/spec-tracer/SKILL.md" | head -5)" "Write"
}

# ── spec-skill-improvements: Unit 13 — spec-promoter diff-first ──────────────
test_spec_promoter_diff_first() {
  local promoter; promoter="$(cat "$SPEC_SKILL/agents/spec-promoter/SKILL.md")"
  assert_contains "spec-skill-improvements/13" "spec-promoter has dry-run language" "$promoter" "dry-run"
  assert_contains "spec-skill-improvements/13" "spec-promoter has confirmation language" "$promoter" "confirm"
}

# ── spec-skill-improvements: Unit 14 — spec-interviewer constraint ────────────
test_spec_interviewer_constraint_injection() {
  local interviewer; interviewer="$(cat "$SPEC_SKILL/agents/spec-interviewer/SKILL.md")"
  assert_contains "spec-skill-improvements/14" "spec-interviewer references Interview for WHAT" "$interviewer" "Interview for WHAT"
  assert_contains "spec-skill-improvements/14" "spec-interviewer references brainstorming" "$interviewer" "brainstorming"
}

# ── spec-skill-improvements: Unit 15 — spec-health output levels ─────────────
test_spec_health_output_levels() {
  local health; health="$(cat "$SPEC_SKILL/agents/spec-health/SKILL.md")"
  assert_contains "spec-skill-improvements/15" "spec-health has CRITICAL level" "$health" "CRITICAL"
  assert_contains "spec-skill-improvements/15" "spec-health has WARN level" "$health" "WARN"
  assert_contains "spec-skill-improvements/15" "spec-health has INFO level" "$health" "INFO"
}

# ── spec-skill-improvements: Unit 16 — SF15 + SF16 ───────────────────────────
test_sf15_long_root_warns() {
  local d; d="$(mktmp)"; mkdir -p "$d/.spec"
  {
    cat <<'EOF'
---
type: entrypoint
scope: product
children: []
updated: 2026-01-01
---
EOF
    python3 -c "[print(f'## Section {i}\nLine.') for i in range(110)]"
  } > "$d/.spec/product.md"
  local out; out="$(cd "$d" && bash "$VALIDATE" 2>&1)"
  assert_contains "spec-skill-improvements/16" "SF15 warns on long root spec" "$out" "SF15"
  rm -rf "$d"
}

test_sf16_lesson_no_tags_warns() {
  local d; d="$(mktmp)"; mkdir -p "$d/.spec"
  cat > "$d/.spec/product.md" <<'EOF'
---
type: entrypoint
scope: product
children: []
updated: 2026-01-01
---
# P
EOF
  cat > "$d/.spec/lessons.md" <<'EOF'
### Missing tags
**Pattern:** test
**Rule:** test
**Date:** 2026-01-01
EOF
  local out; out="$(cd "$d" && bash "$VALIDATE" 2>&1)"
  assert_contains "spec-skill-improvements/16" "SF16 warns on tagless lesson" "$out" "SF16"
  rm -rf "$d"
}

test_sf16_lesson_with_tags_passes() {
  local d; d="$(mktmp)"; mkdir -p "$d/.spec"
  cat > "$d/.spec/product.md" <<'EOF'
---
type: entrypoint
scope: product
children: []
updated: 2026-01-01
---
# P
EOF
  cat > "$d/.spec/lessons.md" <<'EOF'
### Tagged lesson
**Pattern:** test
**Rule:** test
**Tags:** foo, bar
**Date:** 2026-01-01
EOF
  local out; out="$(cd "$d" && bash "$VALIDATE" 2>&1)"
  assert_not_contains "spec-skill-improvements/16" "SF16 silent on tagged lesson" "$out" "SF16"
  rm -rf "$d"
}

# ── spec-skill-improvements: Unit 17 — subagents folder wiring ───────────────
test_subagents_folder_wiring() {
  for agent in spec-tracer spec-promoter spec-interviewer spec-health; do
    if [[ -f "$SPEC_SKILL/agents/$agent/SKILL.md" ]]; then
      pass "spec-skill-improvements/17" "subagent SKILL.md exists: $agent"
    else
      fail "spec-skill-improvements/17" "subagent SKILL.md exists: $agent"
    fi
  done
  local skill; skill="$(cat "$SPEC_SKILL/SKILL.md")"
  assert_contains "spec-skill-improvements/17" "root SKILL.md agents: map has spec-tracer" "$skill" "spec-tracer"
}

# ── spec-skill-improvements: Unit 18 — setup interview flow ──────────────────
test_setup_section_has_interview_flow() {
  local skill; skill="$(cat "$SPEC_SKILL/SKILL.md")"
  assert_contains "spec-skill-improvements/18" "SKILL.md Setup has Q1" "$skill" "Q1"
  assert_contains "spec-skill-improvements/18" "SKILL.md Setup has Q2" "$skill" "Q2"
  assert_contains "spec-skill-improvements/18" "SKILL.md Setup has Q3" "$skill" "Q3"
  assert_contains "spec-skill-improvements/18" "SKILL.md Setup has Q4" "$skill" "Q4"
  assert_contains "spec-skill-improvements/18" "SKILL.md Setup references .config.yaml" "$skill" ".config.yaml"
}

# ── spec-skill-improvements: Unit 19 — config section + defaults ─────────────
test_config_section_present() {
  local skill; skill="$(cat "$SPEC_SKILL/SKILL.md")"
  assert_contains "spec-skill-improvements/19" "SKILL.md has ## Config section" "$skill" "## Config"
  assert_contains "spec-skill-improvements/19" "SKILL.md Config references .config.yaml" "$skill" ".config.yaml"
  assert_contains "spec-skill-improvements/19" "SKILL.md Config has suggest-superpowers" "$skill" "suggest-superpowers"
}

test_config_defaults_documented() {
  local skill; skill="$(cat "$SPEC_SKILL/SKILL.md")"
  assert_contains "spec-skill-improvements/19" "SKILL.md Config documents vibe-flow key" "$skill" "vibe-flow"
  assert_contains "spec-skill-improvements/19" "SKILL.md Config documents caveman key" "$skill" "caveman"
  assert_contains "spec-skill-improvements/19" "SKILL.md Config documents suggest-superpowers key" "$skill" "suggest-superpowers"
}

# ── flow-mvp/4: hybrid plan template gains agentic-workers grammar ──────────
test_flow_mvp_4_template_hybrid_grammar() {
  local tpl; tpl="$(cat "$SPEC_SKILL/reference/templates/feature-plan.md")"
  assert_contains "flow-mvp/4" "template has 'For agentic workers' line" "$tpl" "For agentic workers"
  assert_contains "flow-mvp/4" "template has ## Global Constraints" "$tpl" "## Global Constraints"
  assert_contains "flow-mvp/4" "template has a Steps checkbox block" "$tpl" "**Steps:**"
  assert_contains "flow-mvp/4" "template Steps block has checkboxes" "$tpl" "- [ ]"
}

# ── Run new tests ────────────────────────────────────────────────────────────
echo ""
echo "=== spec-skill-improvements v2.0 tests ==="
test_skill_v2_frontmatter_fields
test_skill_roles_section
test_skill_routing_new_routes
test_feature_output_profiles
test_promote_dry_run_no_mutation
test_promote_reversed_markers_refused
test_promote_valid_blocks
test_lessons_for_match
test_lessons_for_no_match
test_lessons_for_inject_format
test_sf13_stale_link_warns
test_sf13_valid_links_pass
test_sf14_conflict_warns
test_sf14_no_conflict_passes
test_branch_doc_templates_exist
test_scan_merges_finds_blocks
test_scan_merges_unclosed_exits_nonzero
test_scan_merges_empty_feature
test_openspec_docs_present
test_spec_tracer_read_only
test_spec_promoter_diff_first
test_spec_interviewer_constraint_injection
test_spec_health_output_levels
test_sf15_long_root_warns
test_sf16_lesson_no_tags_warns
test_sf16_lesson_with_tags_passes
test_subagents_folder_wiring
test_setup_section_has_interview_flow
test_config_section_present
test_config_defaults_documented
test_flow_mvp_4_template_hybrid_grammar

echo ""
echo "=== results: $PASS passed, $FAIL failed ==="
[[ $FAIL -eq 0 ]]
