#!/usr/bin/env bash
# tests/spec/run.sh — behaviour tests for the spec-framework SF0–SF4 units.
# Pure bash; no bats dependency. Each test cites its plan unit ID (SF0…SF3, D9).
#
# Usage: bash tests/spec/run.sh
# Exit 0 = all pass; non-zero = at least one failure.

set -uo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
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

echo ""
echo "=== results: $PASS passed, $FAIL failed ==="
[[ $FAIL -eq 0 ]]
