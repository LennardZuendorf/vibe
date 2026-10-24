#!/usr/bin/env bash
# vibe-spec: literal-file
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

# ── SF12: R-ID traceability fires under any awk (mawk \b regression) ──────────
# The old emit_r_ids used \b inside awk match(), which mawk (Debian default awk)
# does not support, so under mawk the R-ID trace check silently never fired. A
# product.md R-id NOT cited in plan.md must warn regardless of which awk the host
# ships. Discriminating: reverting to the \b pattern makes this pass under gawk
# but fail under mawk — here it must warn on whatever awk runs the suite.
test_sf12_r_id_trace_fires() {
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

### Requirement: Thing SHALL happen (R7)

#### Scenario: Happy path

- **Given** x
- **When** y
- **Then** z
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
| — | thing | myfeature/1 |

### myfeature/1 — first slice

**Dependencies:** —
EOF
  local out; out="$( cd "$d" && bash "$VALIDATE" 2>&1 )"
  assert_contains SF12 "untraced R-id warns under any awk (mawk \\b regression)" "$out" "R7 in product.md not cited in plan.md"
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
  for t in ${expected[@]+"${expected[@]}"}; do
    if [[ -f "$SPEC_SKILL/reference/templates/$t" ]]; then
      pass SF17 "template exists: $t"
    else
      fail SF17 "template exists: $t"
    fi
  done
}

# ── SF17, skill-revision/5: SKILL.md stays within its word budget; detail lives in reference/setup.md ──
test_sf17_skill_size() {
  local words; words="$(wc -w < "$SPEC_SKILL/SKILL.md" | tr -d ' ')"
  if [[ "$words" -le 1400 ]]; then pass SF17 "SKILL.md at most 1400 words ($words)"
  else fail SF17 "SKILL.md at most 1400 words ($words)"; fi
  if [[ -f "$SPEC_SKILL/reference/setup.md" ]]; then pass SF17 "setup, config and templates live in reference/setup.md"
  else fail SF17 "setup, config and templates live in reference/setup.md"; fi
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
test_sf12_r_id_trace_fires
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

# ── skill-revision/3: SKILL.md frontmatter holds only keys a harness reads ───
test_skill_v2_frontmatter_fields() {
  local fm keys bad="" k compat
  local allowed=" name description user-invocable argument-hint allowed-tools compatibility "
  fm="$(awk 'NR==1&&$0!="---"{exit} NR>1&&$0=="---"{exit} NR>1{print}' "$SPEC_SKILL/SKILL.md")"
  keys="$(grep -Eo '^[A-Za-z][A-Za-z0-9_-]*:' <<<"$fm" | sed 's/:$//' | tr '\n' ' ')"
  for k in $keys; do
    [[ "$allowed" == *" $k "* ]] || bad="$bad $k"
  done
  if [[ -z "$bad" && -n "$keys" ]]; then pass "skill-revision/3" "SKILL.md frontmatter keys are the allowlist ($keys)"
  else fail "skill-revision/3" "SKILL.md frontmatter keys are the allowlist (unexpected:$bad)"; fi
  compat="$(sed -n 's/^compatibility:[[:space:]]*//p' <<<"$fm")"
  assert_contains "skill-revision/3" "compatibility names the vibe-spec binary" "$compat" "vibe-spec"
  assert_not_contains "skill-revision/3" "SKILL.md frontmatter dropped retired caveman machinery" "$fm" "caveman"
}

# ── spec-skill-improvements: Unit 2 — ## Roles section reconciled to real subagents ─
# Roles, delegates:, and agents/ dirs must name the SAME four subagents. Phantom
# names (spec-architect/planner/auditor/compactor) must not reappear anywhere in SKILL.md.
test_skill_roles_section() {
  local skill; skill="$(cat "$SPEC_SKILL/SKILL.md")"
  assert_contains "skill-revision/6" "SKILL.md has ## Roles and dispatch" "$skill" "## Roles and dispatch"
  assert_contains "spec-skill-improvements/2" "Roles name spec-promoter" "$skill" "spec-promoter"
  assert_contains "spec-skill-improvements/2" "Roles name spec-health" "$skill" "spec-health"
  assert_not_contains "spec-skill-improvements/2" "no phantom spec-architect" "$skill" "spec-architect"
  assert_not_contains "spec-skill-improvements/2" "no phantom spec-planner" "$skill" "spec-planner"
  assert_not_contains "spec-skill-improvements/2" "no phantom spec-auditor" "$skill" "spec-auditor"
  assert_not_contains "spec-skill-improvements/2" "no phantom spec-compactor" "$skill" "spec-compactor"
}

# ── fix-7: every Roles/delegates name has a real agents/<name>.md file ──
test_skill_roles_have_agent_dirs() {
  local skill; skill="$(cat "$SPEC_SKILL/SKILL.md")"
  local agent
  for agent in spec-promoter spec-health; do
    assert_contains "fix-7" "SKILL.md names real subagent $agent" "$skill" "$agent"
    if [[ -f "$SPEC_SKILL/agents/$agent.md" ]]; then
      pass "fix-7" "agents/$agent.md exists"
    else
      fail "fix-7" "agents/$agent.md exists"
    fi
  done
}

# ── spec-skill-improvements: Unit 3 — Routing expansion ──────────────────────
test_skill_routing_new_routes() {
  local skill; skill="$(cat "$SPEC_SKILL/SKILL.md")"
  assert_contains "spec-skill-improvements/3" "SKILL.md routing has interview" "$skill" "interview"
  assert_contains "spec-skill-improvements/3" "SKILL.md routing has promote" "$skill" "promote"
  # shellcheck disable=SC2016 # literal backticks
  if grep -qE '^\| `audit` \|' "$SPEC_SKILL/SKILL.md"; then fail "skill-revision/4" "SKILL.md has no audit argument row (health runs validate + check-drift)"
  else pass "skill-revision/4" "SKILL.md has no audit argument row (health runs validate + check-drift)"; fi
  assert_contains "spec-skill-improvements/3" "SKILL.md routing has lessons-for" "$skill" "lessons-for"
  assert_contains "spec-skill-improvements/3" "SKILL.md routing has diff" "$skill" "diff"
  assert_contains "spec-skill-improvements/3" "SKILL.md routing has health" "$skill" "health"
  assert_contains "spec-skill-improvements/3" "SKILL.md routing has research" "$skill" "research"
}

# ── spec-skill-improvements: Unit 4 — feature.md output density (style note) ──
test_feature_output_profiles() {
  local fm; fm="$(cat "$SPEC_SKILL/feature.md")"
  assert_contains "spec-skill-improvements/4" "feature.md has ## Output density" "$fm" "## Output density"
  assert_contains "spec-skill-improvements/4" "feature.md output density cites the flow style note" "$fm" "§ Style"
  assert_not_contains "spec-skill-improvements/4" "feature.md dropped retired caveman levels" "$fm" "caveman"
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

# ── spec-skill-improvements: Unit 7 — SF13 stale directory-link checker ──────
# SF13 owns only non-.md directory links; stale .md links are the main checker's job.
test_sf13_stale_link_warns() {
  local d; d="$(mktmp)"; mkdir -p "$d/.spec"
  cat > "$d/.spec/product.md" <<'EOF'
---
type: entrypoint
scope: product
children: []
updated: 2026-01-01
---
[old](features/deleted/)
EOF
  local out; out="$(cd "$d" && bash "$VALIDATE" 2>&1)"
  assert_contains "spec-skill-improvements/7" "SF13 warns on stale feature directory link" "$out" "SF13"
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
[here](features/existing/)
EOF
  local out; out="$(cd "$d" && bash "$VALIDATE" 2>&1)"
  assert_not_contains "spec-skill-improvements/7" "SF13 silent on valid directory link" "$out" "SF13"
  rm -rf "$d"
}

# ── fix-9d: a stale .md link is reported once (broken-link error), never as SF13 ─
# Discriminates against the old code, which double-reported .md links as both.
test_sf13_md_link_no_double_report() {
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
  assert_contains "fix-9d" "stale .md link errors as broken link" "$out" "broken link to 'features/deleted/product.md'"
  assert_not_contains "fix-9d" "stale .md link not double-reported by SF13" "$out" "SF13"
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
  # Assert the full conflict message: naming both features in one SF14 line is
  # discriminating (feature names alone also appear in section headers).
  assert_contains "spec-skill-improvements/8" "SF14 warns naming both conflicting features" "$out" "SF14: scope conflict — 'alpha' and 'beta' both own: state machine"
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

# js-core/8 fix round 2 (re-review Minor 1). The fix above — `${results[@]+…}`
# instead of a bare `"${results[@]}"` — is a real bash-3.2 defect fix: under
# `set -u`, bash < 4.4 ABORTS on an empty-array slice, and the ONLY finding
# being an unclosed marker leaves `results` empty. But the fixture that was
# meant to pin it asserts only `rc != 0`, and the ABORTED script also exits 1.
# It cannot tell "aborted mid-output" from "reported the finding".
#
# The visible damage is the OUTPUT, so assert that: `--format json` emitted a
# bare `[` (invalid JSON, truncated by the abort) before the fix and `[]` after,
# while stderr carries the diagnosis either way. Note the discrimination domain
# honestly: on bash >= 4.4 the construct is simply not broken, so no fixture can
# separate the two spellings there — this goes red on the macOS CI leg (stock
# /bin/bash 3.2, which runs every suite through `$BASH` dispatch) and on any
# other 3.2 target, which is exactly where the defect lives.
test_scan_merges_unclosed_emits_valid_json() {
  local d; d="$(mktmp)"; mkdir -p "$d/.spec/features/feat"
  printf '<!-- merge -->\n## unclosed block\n' > "$d/.spec/features/feat/tech.md"
  local out err rc=0
  err="$d/err.txt"
  # $BASH, not PATH's `bash`: the defect only exists on bash < 4.4, and the
  # macOS leg exercises it by starting the aggregator on /bin/bash 3.2. A
  # PATH-resolved `bash` would hand this fixture whatever else is installed
  # (GitHub's macOS images also carry Homebrew bash 5) and the pin would go
  # vacuous on the only leg that can see the bug — the same failure the
  # aggregator's own $BASH dispatch was added to fix.
  out="$( (cd "$d" && "${BASH:-bash}" "$SPEC_SKILL/scripts/scan-merges.sh" --format json feat) 2>"$err" )" || rc=$?
  if [[ "$out" == "[]" ]]; then
    pass "spec-skill-improvements/10" "unclosed marker still emits complete JSON (not a truncated '[')"
  else
    fail "spec-skill-improvements/10" "unclosed marker still emits complete JSON (not a truncated '[')"
    echo "        expected: []"
    echo "        got:      $out"
  fi
  local errbody; errbody="$(cat "$err" 2>/dev/null || true)"
  assert_contains "spec-skill-improvements/10" "unclosed marker reports the finding on stderr" \
    "$errbody" "unclosed <!-- merge --> block"
  if [[ $rc -ne 0 ]]; then pass "spec-skill-improvements/10" "unclosed marker exits non-zero (json format)"
  else fail "spec-skill-improvements/10" "unclosed marker exits non-zero (json format)"; fi
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

# ── spec-skill-improvements/13, skill-revision/6: spec-promoter previews first, applies on "apply: yes" ──
test_spec_promoter_diff_first() {
  local promoter; promoter="$(cat "$SPEC_SKILL/agents/spec-promoter.md")"
  assert_contains "spec-skill-improvements/13" "spec-promoter has dry-run language" "$promoter" "--dry-run"
  assert_contains "skill-revision/6" "spec-promoter applies only on 'apply: yes'" "$promoter" "apply: yes"
}

# ── skill-revision/6: every field spec-health cites is a key or value its commands emit ──
test_spec_health_output_levels() {
  local f tok bad=0 n=0 emit
  emit="$(cat "$REPO_ROOT/crates/vibe-spec/src/report.rs" "$REPO_ROOT/crates/vibe-spec/src/drift.rs")"
  f="$SPEC_SKILL/agents/spec-health.md"
  # shellcheck disable=SC2016 # literal backticks
  for tok in $(grep -E '^[0-9]+\. Reply with' -A3 "$f" | grep -oE '`[a-z]+`' | tr -d '`' | sort -u); do
    n=$((n + 1))
    if ! grep -qF "\"$tok\"" <<<"$emit"; then bad=$((bad + 1)); echo "        not emitted: $tok"; fi
  done
  if [[ $n -lt 4 ]]; then fail "skill-revision/6" "spec-health cites the emitted fields (examined $n, floor 4)"
  elif [[ $bad -eq 0 ]]; then pass "skill-revision/6" "spec-health cites only fields report.rs/drift.rs emit ($n)"
  else fail "skill-revision/6" "spec-health cites $bad field(s) no command emits"; fi
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
    python3 -c "[print(f'## Section {i}\nLine.') for i in range(170)]"
  } > "$d/.spec/product.md"
  local out; out="$(cd "$d" && bash "$VALIDATE" 2>&1)"
  assert_contains "spec-skill-improvements/16" "SF15 warns on >300-line root spec" "$out" "SF15"
  rm -rf "$d"
}

# ── fix-8: SF15 threshold is 300 — a ~250-line root spec must NOT warn ───────
# Discriminates against the old 200-line threshold, which would flag this file.
test_sf15_under_threshold_silent() {
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
    python3 -c "[print(f'## Section {i}\nLine.') for i in range(120)]"
  } > "$d/.spec/product.md"
  local out; out="$(cd "$d" && bash "$VALIDATE" 2>&1)"
  assert_not_contains "fix-8" "SF15 silent on ~245-line root spec (threshold 300)" "$out" "SF15"
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
  for agent in spec-promoter spec-health; do
    if [[ -f "$SPEC_SKILL/agents/$agent.md" ]]; then
      pass "spec-skill-improvements/17" "agent file exists: $agent"
    else
      fail "spec-skill-improvements/17" "agent file exists: $agent"
    fi
  done
  local skill; skill="$(cat "$SPEC_SKILL/SKILL.md")"
  assert_contains "spec-skill-improvements/17" "root SKILL.md agents: map has spec-health" "$skill" "spec-health"
}

# ── skill-revision/5: the setup interview lives in reference/setup.md ───────
test_setup_section_has_interview_flow() {
  local setup; setup="$(cat "$SPEC_SKILL/reference/setup.md" 2>/dev/null)"
  assert_contains "skill-revision/5" "reference/setup.md Setup has Q1" "$setup" "Q1"
  assert_contains "skill-revision/5" "reference/setup.md Setup has Q2" "$setup" "Q2"
  assert_contains "skill-revision/5" "reference/setup.md Setup has Q3" "$setup" "Q3"
  assert_not_contains "skill-revision/5" "reference/setup.md Setup keeps three questions" "$setup" "Q4"
  assert_contains "skill-revision/5" "reference/setup.md Setup writes .vibe/spec.json" "$setup" ".vibe/spec.json"
}

# ── skill-revision/5: one config file; SKILL.md § Config points to reference/setup.md ──
test_config_section_present() {
  local setup skill; setup="$(cat "$SPEC_SKILL/reference/setup.md" 2>/dev/null)"; skill="$(cat "$SPEC_SKILL/SKILL.md")"
  assert_contains "skill-revision/5" "SKILL.md has ## Config" "$skill" "## Config"
  assert_contains "skill-revision/5" "SKILL.md Config points to reference/setup.md" "$skill" "(reference/setup.md)"
  assert_contains "skill-revision/5" "reference/setup.md has ## Config" "$setup" "## Config"
  assert_contains "skill-revision/5" "reference/setup.md Config names .vibe/spec.json" "$setup" ".vibe/spec.json"
}

# shellcheck disable=SC2016 # literal backticks
test_config_defaults_documented() {
  local setup; setup="$(cat "$SPEC_SKILL/reference/setup.md" 2>/dev/null)"
  assert_contains "skill-revision/5" "reference/setup.md documents featureLayer" "$setup" '`featureLayer`'
  assert_contains "skill-revision/5" "reference/setup.md documents suggest" "$setup" '`suggest`'
  assert_contains "skill-revision/5" "reference/setup.md documents superpowers" "$setup" '`superpowers`'
  assert_not_contains "skill-revision/5" "reference/setup.md dropped the retired caveman key" "$setup" "caveman"
}

# ── flow-mvp/4: hybrid plan template gains agentic-workers grammar ──────────
test_flow_mvp_4_template_hybrid_grammar() {
  local tpl; tpl="$(cat "$SPEC_SKILL/reference/templates/feature-plan.md")"
  assert_contains "flow-mvp/4" "template has 'For agentic workers' line" "$tpl" "For agentic workers"
  assert_contains "flow-mvp/4" "template has ## Global Constraints" "$tpl" "## Global Constraints"
  assert_contains "flow-mvp/4" "template has a Steps checkbox block" "$tpl" "**Steps:**"
  assert_contains "flow-mvp/4" "template Steps block has checkboxes" "$tpl" "- [ ]"
  assert_contains "flow-mvp/11" "template has per-unit **Interfaces:** block" "$tpl" "**Interfaces:**"
  # simplify: Interfaces + Steps are handover-mode-only; interactive impl may omit them,
  # but the core blocks (Goal/Requirements/Files/Test scenarios/Verification) always stay.
  assert_contains "simplify/plan-grammar" "template marks Interfaces/Steps as handover-mode-only" "$tpl" "HANDOVER-MODE ONLY"
  assert_contains "simplify/plan-grammar" "template keeps Test scenarios as a core block" "$tpl" "**Test scenarios:**"
  assert_contains "simplify/plan-grammar" "template keeps Verification as a core block" "$tpl" "**Verification:**"
}

# ── fix-1: SF14 detects conflicts in the row-label Scope shape (template shape) ─
# The shipped feature-product.md template uses '| **Owns** | items |'. The old
# check only matched the '| Owns |' column-header shape, so it missed the template
# entirely. This test fails against the old code (no SF14 warning emitted).
test_sf14_row_label_shape_conflict() {
  local d; d="$(mktmp)"; mkdir -p "$d/.spec/features/gamma" "$d/.spec/features/delta"
  cat > "$d/.spec/product.md" <<'EOF'
---
type: entrypoint
scope: product
children: []
updated: 2026-01-01
---
# P
EOF
  cat > "$d/.spec/features/gamma/product.md" <<'EOF'
---
type: feature-product
feature: gamma
sibling: tech.md
parent: ../../product.md
updated: 2026-01-01
---
## Scope

| | |
|---|---|
| **Owns** | state machine, cursor file |
| **Does not own** | adapters |
EOF
  cat > "$d/.spec/features/gamma/tech.md" <<'EOF'
---
type: feature-tech
feature: gamma
sibling: product.md
parent: ../../tech.md
updated: 2026-01-01
---
# Tech
EOF
  cat > "$d/.spec/features/delta/product.md" <<'EOF'
---
type: feature-product
feature: delta
sibling: tech.md
parent: ../../product.md
updated: 2026-01-01
---
## Scope

| | |
|---|---|
| **Owns** | state machine, routing hooks |
| **Does not own** | adapters |
EOF
  cat > "$d/.spec/features/delta/tech.md" <<'EOF'
---
type: feature-tech
feature: delta
sibling: product.md
parent: ../../tech.md
updated: 2026-01-01
---
# Tech
EOF
  local out; out="$(cd "$d" && bash "$VALIDATE" 2>&1)"
  # Full message is discriminating: only fires if the row-label shape is parsed.
  assert_contains "fix-1" "SF14 catches row-label (template) shape conflict" "$out" "SF14: scope conflict — 'delta' and 'gamma' both own: state machine"
  rm -rf "$d"
}

# ── fix-2: link checker examines every link on a line, not only the last ────────
# Old greedy '\[.*?\]\(...\)' + greedy sed extracted only the last target, so a
# broken FIRST link on a multi-link line slipped through. Fails against old code.
test_link_checker_first_of_two_broken() {
  local d; d="$(mktmp)"; mkdir -p "$d/.spec"
  cat > "$d/.spec/product.md" <<'EOF'
---
type: entrypoint
scope: product
children: []
updated: 2026-01-01
---
See [gone](missing-doc.md) and [here](tech.md) for details.
EOF
  cat > "$d/.spec/tech.md" <<'EOF'
---
type: entrypoint
scope: tech
children: []
updated: 2026-01-01
---
# T
EOF
  local out; out="$(cd "$d" && bash "$VALIDATE" 2>&1)"
  assert_contains "fix-2" "first of two links on a line is checked" "$out" "broken link to 'missing-doc.md'"
  rm -rf "$d"
}

# ── fix-3: branch-doc scope/covers checks fire on the real -topic types ─────────
# A topic doc authored WITHOUT scope/covers must be flagged; old code gated on the
# dead 'type: branch' and never fired for template-authored topic docs.
test_topic_doc_scope_covers_checked() {
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
  cat > "$d/.spec/product-auth.md" <<'EOF'
---
type: product-topic
parent: product.md
updated: 2026-01-01
---
# Auth — Product
EOF
  local out; out="$(cd "$d" && bash "$VALIDATE" 2>&1)"
  assert_contains "fix-3" "topic doc missing scope: warns (check fires on product-topic)" "$out" "branch doc missing 'scope:'"
  assert_contains "fix-3" "topic doc missing covers: warns" "$out" "branch doc missing 'covers:'"
  rm -rf "$d"
}

test_topic_doc_complete_passes() {
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
  cat > "$d/.spec/tech-infra.md" <<'EOF'
---
type: tech-topic
parent: tech.md
scope: infra
covers: ci, deploy
updated: 2026-01-01
---
# Infra — Tech
EOF
  local out; out="$(cd "$d" && bash "$VALIDATE" 2>&1)"
  assert_not_contains "fix-3" "complete topic doc raises no branch-field warning" "$out" "branch doc missing"
  rm -rf "$d"
}

# ── fix-4: every templates/ link in reference/*.md resolves on disk ─────────────
# Guards the product-xxx.md / tech-xxx.md rot (real files are *-topic.md).
test_reference_template_links_resolve() {
  local ref dir link target missing=0
  for ref in "$SPEC_SKILL/reference"/*.md; do
    [[ -f "$ref" ]] || continue
    dir="$(dirname "$ref")"
    while IFS= read -r target; do
      [[ "$target" == templates/* ]] || continue
      target="${target%%#*}"
      if [[ ! -e "$dir/$target" ]]; then
        fail "fix-4" "reference link resolves: $(basename "$ref") → $target"
        missing=1
      fi
    done < <(grep -oE '\[[^]]*\]\([^)]+\)' "$ref" | sed 's/.*(\([^)]*\)).*/\1/')
  done
  [[ $missing -eq 0 ]] && pass "fix-4" "all templates/ links in reference/*.md resolve"
}

# ── fix-5: promote.sh is idempotent — a re-run must not duplicate blocks ─────────
test_promote_idempotent_rerun() {
  local d; d="$(mktmp)"; mkdir -p "$d/.spec/features/feat"
  printf '<!-- merge -->\n## promoted section\n<!-- /merge -->\n' > "$d/.spec/features/feat/tech.md"
  echo "# root" > "$d/.spec/tech.md"
  (cd "$d" && bash "$SPEC_SKILL/scripts/promote.sh" feat) >/dev/null
  (cd "$d" && bash "$SPEC_SKILL/scripts/promote.sh" feat) >/dev/null
  local count; count="$(grep -c "promoted section" "$d/.spec/tech.md")"
  assert_contains "fix-5" "promote re-run leaves a single copy" "count=$count" "count=1"
  rm -rf "$d"
}

# ── fix-6: setup.sh writes a commented default .config.yaml when absent ──────────
test_setup_writes_config() {
  local d; d="$(mktmp)"; ( cd "$d" && bash "$SETUP" >/dev/null 2>&1 )
  if [[ -f "$d/.spec/.config.yaml" ]]; then
    pass "fix-6" "setup.sh created .spec/.config.yaml"
  else
    fail "fix-6" "setup.sh created .spec/.config.yaml"
  fi
  local body; body="$(cat "$d/.spec/.config.yaml" 2>/dev/null || true)"
  assert_contains "fix-6" ".config.yaml documents vibe-flow key" "$body" "vibe-flow"
  assert_not_contains "fix-6" ".config.yaml dropped the retired caveman key" "$body" "caveman"
  assert_contains "fix-6" ".config.yaml documents suggest-superpowers key" "$body" "suggest-superpowers"
  rm -rf "$d"
}

# ── fix-6: setup.sh never overwrites an existing .config.yaml ────────────────────
test_setup_config_preserved() {
  local d; d="$(mktmp)"; mkdir -p "$d/.spec"
  printf '# sentinel: preserved\n' > "$d/.spec/.config.yaml"
  ( cd "$d" && bash "$SETUP" >/dev/null 2>&1 )
  local body; body="$(cat "$d/.spec/.config.yaml")"
  assert_contains "fix-6" "existing .config.yaml preserved across setup" "$body" "# sentinel: preserved"
  rm -rf "$d"
}

# ── Run new tests ────────────────────────────────────────────────────────────
echo ""
echo "=== spec-skill-improvements v2.0 tests ==="
test_skill_v2_frontmatter_fields
test_skill_roles_section
test_skill_roles_have_agent_dirs
test_skill_routing_new_routes
test_feature_output_profiles
test_promote_dry_run_no_mutation
test_promote_reversed_markers_refused
test_promote_valid_blocks
test_promote_idempotent_rerun
test_lessons_for_match
test_lessons_for_no_match
test_lessons_for_inject_format
test_sf13_stale_link_warns
test_sf13_valid_links_pass
test_sf13_md_link_no_double_report
test_sf14_conflict_warns
test_sf14_no_conflict_passes
test_sf14_row_label_shape_conflict
test_link_checker_first_of_two_broken
test_topic_doc_scope_covers_checked
test_topic_doc_complete_passes
test_reference_template_links_resolve
test_setup_writes_config
test_setup_config_preserved
test_branch_doc_templates_exist
test_scan_merges_finds_blocks
test_scan_merges_unclosed_exits_nonzero
test_scan_merges_unclosed_emits_valid_json
test_scan_merges_empty_feature
test_openspec_docs_present
test_spec_promoter_diff_first
test_spec_health_output_levels
test_sf15_long_root_warns
test_sf15_under_threshold_silent
test_sf16_lesson_no_tags_warns
test_sf16_lesson_with_tags_passes
test_subagents_folder_wiring
test_setup_section_has_interview_flow
test_config_section_present
test_config_defaults_documented
test_flow_mvp_4_template_hybrid_grammar

# ── install-agnostic paths: skill files reference their scripts skill-relative ──
# Per agentskills.io, skill docs reference bundled scripts/files relative to the skill
# (`scripts/validate.sh`, or `${CLAUDE_SKILL_DIR}/scripts/…` in `!` commands), correct in
# EVERY install model. Ban what the matcher catches: `.agents/skills/spec` and `~/.agents/skills/` paths.
# R1: spec/README.md and spec/AGENTS.md are human-facing install/layout docs, excluded by exact path.
test_skill_script_paths_relative() {
  local f rel bad=0 scanned=0 excluded=0 total floor names=()
  local s; for s in "$REPO_ROOT"/spec/scripts/*.sh; do names+=("$(basename "$s")"); done
  total="$(find "$REPO_ROOT/spec" -name '*.md' -not -path '*/tests/*' | wc -l | tr -d ' ')"
  while IFS= read -r f; do
    rel="${f#"$REPO_ROOT"/}"
    case "$rel" in
      spec/README.md|spec/AGENTS.md) excluded=$((excluded + 1)); continue ;;
    esac
    scanned=$((scanned + 1))
    # shellcheck disable=SC2088
    if grep -qF '.agents/skills/spec' "$f" || grep -qF '~/.agents/skills/' "$f"; then
      bad=$((bad + 1)); echo "        offender: $rel"
    fi
  done < <(find "$REPO_ROOT/spec" -name '*.md' -not -path '*/tests/*')
  floor=10
  if [[ ${#names[@]} -lt 7 || $scanned -lt $floor || $((scanned + excluded)) -ne $total || $excluded -ne 2 ]]; then
    fail "install-agnostic-paths" "guard examined too little (scanned=$scanned excluded=$excluded total=$total, scripts=${#names[@]})"
  elif [[ $bad -eq 0 ]]; then
    pass "install-agnostic-paths" "$scanned spec docs reference ${#names[@]} scripts skill-relative ($excluded install docs excluded)"
  else
    fail "install-agnostic-paths" "$bad spec doc(s) hard-code an install-absolute script path"
  fi
}
test_skill_script_paths_relative

# ── F1: spec scripts resolve the project root from cwd, never from their own path ──
test_f1_subdir_invocation() {
  local out rc
  # The legacy bash validator reads only `.spec/`; probe a fixture project, not this repo.
  local fx; fx="$(mktemp -d)"; mkdir -p "$fx/.git" "$fx/.spec" "$fx/flow"
  out="$(cd "$fx/flow" && bash "$REPO_ROOT/spec/scripts/validate.sh" 2>&1)"; rc=$?
  rm -rf "$fx"
  if grep -qF "Validating .spec/" <<<"$out" && ! grep -qF "No .spec/ directory found" <<<"$out"; then
    pass F1 "validate.sh from flow/ resolves the project .spec/"
  else
    fail F1 "validate.sh from flow/ resolves the project .spec/ (rc=$rc)"
  fi
}
test_f1_subdir_invocation

test_f1_plugin_cache_layout() {
  local tmp proj cache s out rc bad=0 marker
  tmp="$(mktemp -d)"; proj="$tmp/proj"; cache="$tmp/cache/spec"
  mkdir -p "$proj/src/deep" "$cache"
  cp -R "${F1_SCRIPTS_DIR:-$REPO_ROOT/spec/scripts}" "$cache/scripts"
  (cd "$proj" && git init -q . && bash "$cache/scripts/setup.sh" >/dev/null 2>&1)
  if [[ ! -d "$proj/.spec" ]]; then
    rm -rf "$tmp"; fail F1 "fixture setup failed: git init + setup.sh did not create .spec/"; return
  fi
  mkdir -p "$proj/.spec/features/f1feat"
  printf '%s\n' '# Lessons' '' '### f1-fixture-lesson' '**Tags:** alpha' >"$proj/.spec/lessons.md"
  printf '%s\n' '# Tech' '' '<!-- merge -->' 'f1-merge-preview' '<!-- /merge -->' >"$proj/.spec/features/f1feat/tech.md"
  for s in validate list-specs lessons-for scan-merges; do
    case "$s" in
      validate) marker="lessons.md: checked" ;;
      list-specs) marker="- lessons.md [lessons]" ;;
      lessons-for) marker="f1-fixture-lesson" ;;
      scan-merges) marker="f1-merge-preview" ;;
    esac
    if [[ "$s" == lessons-for ]]; then
      out="$(cd "$proj/src/deep" && bash "$cache/scripts/$s.sh" alpha 2>&1)"; rc=$?
    else
      out="$(cd "$proj/src/deep" && bash "$cache/scripts/$s.sh" 2>&1)"; rc=$?
    fi
    if [[ "$out" != *"$marker"* ]] || grep -qF "No .spec/ directory found" <<<"$out"; then
      bad=$((bad + 1)); echo "        offender: $s (rc=$rc, missing marker: $marker)"
    fi
  done
  rm -rf "$tmp"
  if [[ $bad -eq 0 ]]; then pass F1 "4 scripts from a plugin-cache path + nested cwd read the project .spec/ (positive markers)"
  else fail F1 "$bad script(s) failed to read the project .spec/ from a nested cwd"
  fi
}
test_f1_plugin_cache_layout

test_f1_git_boundary() {
  local tmp out s bad=0
  tmp="$(mktemp -d)"; mkdir -p "$tmp/outer/.spec/features/f1feat" "$tmp/outer/inner/sub"
  printf '%s\n' '<!-- merge -->' 'f1-merge-preview' '<!-- /merge -->' >"$tmp/outer/.spec/features/f1feat/tech.md"
  printf '%s\n' '### f1-fixture-lesson' '**Tags:** alpha' >"$tmp/outer/.spec/lessons.md"
  (cd "$tmp/outer/inner" && git init -q .)
  for s in validate list-specs lessons-for scan-merges; do
    out="$(cd "$tmp/outer/inner/sub" && bash "$REPO_ROOT/spec/scripts/$s.sh" alpha 2>&1)" || true
    case "$s" in
      validate|list-specs) [[ "$out" == *"No .spec/ directory found"* ]] || { bad=$((bad + 1)); echo "        offender: $s"; } ;;
      *) [[ "$out" != *"f1-merge-preview"* && "$out" != *"f1-fixture-lesson"* ]] || { bad=$((bad + 1)); echo "        offender: $s"; } ;;
    esac
  done
  rm -rf "$tmp"
  if [[ $bad -eq 0 ]]; then pass F1 "root search stops at the .git boundary for all 4 scripts"
  else fail F1 "root search crossed the .git boundary in $bad script(s)"
  fi
}
test_f1_git_boundary

# ── SF17/F25, skill-revision/3: description is trigger-first, ≤500 chars, no angle brackets ──
test_sf17_description_budget() {
  local desc chars
  desc="$(awk '/^description:/{f=1; sub(/^description:[[:space:]]*\|?[[:space:]]*/,""); if(length) print; next} f && /^[a-z_-]+:/{exit} f{sub(/^[[:space:]]+/,""); print}' "$SPEC_SKILL/SKILL.md" | tr '\n' ' ' | sed 's/[[:space:]]*$//')"
  chars="${#desc}"
  if [[ $chars -gt 20 && $chars -le 500 ]] && grep -qE '^(You MUST use this|Use when)' <<<"$desc" \
    && ! grep -qF 'Maintains' <<<"$desc" && ! grep -q '[<>]' <<<"$desc"; then
    pass SF17 "description is trigger-first, within 500 chars, no angle brackets ($chars chars)"
  else
    fail SF17 "description is trigger-first, within 500 chars, no angle brackets ($chars chars)"
  fi
}
test_sf17_description_budget

test_f25_voice() {
  local f hits=0 scanned=0
  for f in "$SPEC_SKILL"/SKILL.md "$SPEC_SKILL"/feature.md "$SPEC_SKILL"/strategy.md "$SPEC_SKILL"/reference/*.md; do
    [[ -f "$f" ]] || continue; scanned=$((scanned + 1))
    hits=$((hits + $(grep -ciE '\b(you may want to|consider (using|adding)|it might be|feel free|optionally consider)\b' "$f" || true)))
  done
  grep -qxF 'Fallback feature flow. Run this only when the spec skill'"'"'s Routing table sends you here.' "$SPEC_SKILL/feature.md" || hits=$((hits + 1))
  if [[ $scanned -lt 7 ]]; then fail F25 "voice check examined $scanned files (floor 7)"
  elif [[ $hits -eq 0 ]]; then pass F25 "skill docs use imperative voice ($scanned files)"
  else fail F25 "$hits hedge phrase(s) or missing fallback header"; fi
}
test_f25_voice

# ── Task P4-5: spec hook shim + generated plugin hooks (own suite) ──────────
test_hooks_suite() {
  local out rc
  out="$(bash "$REPO_ROOT/spec/tests/hooks.sh" 2>&1)"; rc=$?
  printf '%s\n' "$out"
  if [[ $rc -eq 0 ]]; then pass HOOKS "spec/tests/hooks.sh passes"; else fail HOOKS "spec/tests/hooks.sh passes"; fi
}
test_hooks_suite

# ── Task P4-6: agents + plugin validation (own suites) ──────────────────────
test_agents_suites() {
  local out rc
  out="$(bash "$REPO_ROOT/spec/tests/agents.sh" 2>&1)"; rc=$?
  printf '%s\n' "$out"
  if [[ $rc -eq 0 ]]; then pass AGENTS "spec/tests/agents.sh passes"; else fail AGENTS "spec/tests/agents.sh passes"; fi
  out="$(bash "$REPO_ROOT/spec/tests/plugin-validate.sh" 2>&1)"; rc=$?
  printf '%s\n' "$out"
  if [[ $rc -eq 0 ]]; then pass PLUGIN "spec/tests/plugin-validate.sh passes"; else fail PLUGIN "spec/tests/plugin-validate.sh passes"; fi
}
test_agents_suites

echo ""
echo "=== results: $PASS passed, $FAIL failed ==="
[[ $FAIL -eq 0 ]]
