---
type: feature-plan
feature: spec-skill-improvements
parent: ../../plan.md
updated: 2026-06-21
---

# spec-skill-improvements — Implementation Plan

Stable unit IDs: `spec-skill-improvements/n`. IDs never renumber on reorder;
add a new ID for new work. All units are additive — no existing spec or script
is deleted; only extended or supplemented.

**Prerequisite:** spec skill bundle DONE (gate from root plan.md).

---

## Unit Table

| ID | Title | Requires | Status |
|---|---|---|---|
| spec-skill-improvements/1 | SKILL.md metadata enrichment | — | planned |
| spec-skill-improvements/2 | Subagent role profiles in SKILL.md | 1 | planned |
| spec-skill-improvements/3 | Routing expansion + argument-hint | 1, 2 | planned |
| spec-skill-improvements/4 | feature.md output profiles (Cavekit) | — | planned |
| spec-skill-improvements/5 | `scripts/promote.sh` | — | planned |
| spec-skill-improvements/6 | `scripts/lessons-for.sh` | — | planned |
| spec-skill-improvements/7 | validate.sh SF13 — stale link checker | — | planned |
| spec-skill-improvements/8 | validate.sh SF14 — scope conflict detector | — | planned |

Deferred (tier-3, not in this feature):
- `scripts/score.sh` (spec-specific quality metrics for superpowers context; Improvement 7)
- Superpowers constraint context snippets in feature.md (text additions; Improvement 8 revised)
- SF15 cross-feature unit dep check (extends I1)
- SF16 unit ID drift detector (requires history file)

Explicitly NOT in scope: `scripts/interview.sh`. WHAT-phase interviewing is
done by `superpowers:brainstorming` with `feature.md § Interview for WHAT` as
constraint context. No custom interview CLI is needed or wanted.

---

## Unit Detail

### spec-skill-improvements/1 — SKILL.md metadata enrichment

**Requirements:** R-metadata (SKILL.md metadata enrichment)

**Scope:** Edit `.agents/skills/spec/SKILL.md` frontmatter only.

**Changes:**
- Add `outputs:`, `reads:`, `delegates:`, `phases:`, `caveman:` YAML fields
  after `metadata:` block
- `delegates:` lists four roles with `when:` and `superpowers:` sub-fields
- `caveman:` has three keys: `lite`, `full`, `ultra`

**Verification:**
```bash
# Frontmatter fields present
grep -q 'delegates:' .agents/skills/spec/SKILL.md
grep -q 'phases:' .agents/skills/spec/SKILL.md
grep -q 'caveman:' .agents/skills/spec/SKILL.md
grep -q 'outputs:' .agents/skills/spec/SKILL.md
grep -q 'reads:' .agents/skills/spec/SKILL.md
```

**Test:** `tests/spec/run.sh` → add `test_skill_metadata_fields` covering
presence of each new frontmatter key.

---

### spec-skill-improvements/2 — Subagent role profiles in SKILL.md

**Requirements:** R-roles (Subagent role profiles as superpowers delegation contexts)

**Scope:** Edit `.agents/skills/spec/SKILL.md` body — add `## Roles` section
before `## Routing`.

**Changes:**
- Add `## Roles` section with subsections per tech.md spec:
  `### Role: spec-interviewer`, `spec-architect`, `spec-planner`, `spec-auditor`, `spec-compactor`
- Each subsection has: Executor, Phase, Constraint document to inject, Inputs, Outputs, Validation criteria
- spec-interviewer: executor = `superpowers:brainstorming`; constraint = feature.md § Interview for WHAT
- spec-planner: executor = `superpowers:writing-plans`; constraint = reference/plan.md
- spec-auditor: executor = validate.sh (no superpower); feeds score.sh JSON to verification-before-completion
- spec-compactor: executor = promote.sh + superpowers:finishing-a-development-branch for lesson narrative

**Verification:**
```bash
grep -q '## Roles' .agents/skills/spec/SKILL.md
grep -q '### Role: spec-interviewer' .agents/skills/spec/SKILL.md
grep -q '### Role: spec-auditor' .agents/skills/spec/SKILL.md
```

**Test:** `tests/spec/run.sh` → add `test_skill_roles_section` asserting
all four role headers are present.

---

### spec-skill-improvements/3 — Routing expansion + argument-hint

**Requirements:** R-routing (argument-hint and routing expansion)

**Scope:** Edit `.agents/skills/spec/SKILL.md` — update `argument-hint` and
`## Routing` table.

**Changes:**
- Update `argument-hint:` to include new routes
- Add rows to `## Routing` `$ARGUMENTS:` table for: `interview`, `promote`,
  `audit`, `score`, `lessons-for`

**Prerequisite:** Units 1, 2 (roles and metadata must exist before routing
references them).

**Verification:**
```bash
grep -q 'interview' .agents/skills/spec/SKILL.md
grep -q 'promote' .agents/skills/spec/SKILL.md
grep -q 'audit' .agents/skills/spec/SKILL.md
grep -q 'lessons-for' .agents/skills/spec/SKILL.md
```

**Test:** `tests/spec/run.sh` → add `test_skill_routing_new_routes` asserting
new route names appear in SKILL.md routing table.

---

### spec-skill-improvements/4 — feature.md output profiles (Cavekit)

**Requirements:** R-cavekit (Cavekit-aware output profiles)

**Scope:** Edit `.agents/skills/spec/feature.md` — add `## Output profiles`
section after `## Feature authoring flow`.

**Changes:**
- Add `## Output profiles` with three subsections: Lite, Full, Ultra
- Each subsection contains a table: mandatory / compressed / omit per file type

**Verification:**
```bash
grep -q '## Output profiles' .agents/skills/spec/feature.md
grep -q 'Lite' .agents/skills/spec/feature.md
grep -q 'Ultra' .agents/skills/spec/feature.md
```

**Test:** `tests/spec/run.sh` → add `test_feature_output_profiles` asserting
section and all three level headers exist.

---

### spec-skill-improvements/5 — `scripts/promote.sh`

**Requirements:** R-promote (Compound promotion automation)

**Scope:** Create `.agents/skills/spec/scripts/promote.sh`.

**Changes:**
- New file: full implementation per tech.md spec
- `set -euo pipefail`; shellcheck-clean
- Validates markers; dry-run flag; atomic rename; graceful exit on no-markers

**Verification:**
```bash
bash .agents/skills/spec/scripts/promote.sh --help 2>&1 || true
# Dry-run with a valid feature:
mkdir -p /tmp/spec-test/.spec/features/feat
echo '<!-- merge -->'  >> /tmp/spec-test/.spec/features/feat/tech.md
echo '## section'      >> /tmp/spec-test/.spec/features/feat/tech.md
echo '<!-- /merge -->' >> /tmp/spec-test/.spec/features/feat/tech.md
echo '# root'          > /tmp/spec-test/.spec/tech.md
(cd /tmp/spec-test && bash "$OLDPWD/.agents/skills/spec/scripts/promote.sh" feat --dry-run)
# Should print: --- would promote 1 block(s) to .spec/tech.md ---
rm -rf /tmp/spec-test
```

**Test:** `tests/spec/run.sh` → add:
- `test_promote_dry_run_no_mutation` — dry-run leaves target unchanged
- `test_promote_reversed_markers_refused` — reversed markers exit non-zero, target unchanged
- `test_promote_valid_blocks` — valid blocks are appended to target

**Shellcheck:** `shellcheck .agents/skills/spec/scripts/promote.sh` must
exit 0.

---

### spec-skill-improvements/6 — `scripts/lessons-for.sh`

**Requirements:** R-lessons (Tag-based lessons extraction)

**Scope:** Create `.agents/skills/spec/scripts/lessons-for.sh`.

**Changes:**
- New file: full implementation per tech.md spec
- Supports `--format markdown|inject|json`; defaults to markdown
- Exits 0 with empty output on no-match

**Verification:**
```bash
# Single-tag match
echo '### Test lesson'$'\n''**Pattern:** test'$'\n''**Rule:** test'$'\n''**Tags:** foo, bar'$'\n''**Date:** 2026-01-01' > /tmp/test-lessons.md
(SPEC_DIR=/tmp; bash .agents/skills/spec/scripts/lessons-for.sh foo) | grep -q 'Test lesson'
# No-match exit 0
bash .agents/skills/spec/scripts/lessons-for.sh nonexistent-tag 2>/dev/null; echo "exit: $?"
```

**Test:** `tests/spec/run.sh` → add:
- `test_lessons_for_match` — tag match returns correct lesson block
- `test_lessons_for_no_match` — no match returns empty + exit 0
- `test_lessons_for_inject_format` — inject format adds delimiters

**Shellcheck:** `shellcheck .agents/skills/spec/scripts/lessons-for.sh` must
exit 0.

---

### spec-skill-improvements/7 — validate.sh SF13

**Requirements:** R-sf13 (SF13 cross-reference integrity)

**Scope:** Extend `.agents/skills/spec/scripts/validate.sh`.

**Changes:**
- Add `check_sf13_stale_feature_links()` function per tech.md spec
- Call it in the main validation sequence after orphaned-children check
- Add WARN counter increment per stale link found

**Verification:**
```bash
# Create a stale link scenario
mkdir -p /tmp/spec-sf13/.spec
echo '---'$'\n''type: entrypoint'$'\n''scope: product'$'\n''updated: 2026-01-01'$'\n''---'$'\n''[old](features/deleted/product.md)' > /tmp/spec-sf13/.spec/product.md
(cd /tmp/spec-sf13 && bash "$OLDPWD/.agents/skills/spec/scripts/validate.sh") 2>&1 | grep -q 'SF13'
rm -rf /tmp/spec-sf13
```

**Test:** `tests/spec/run.sh` → add:
- `test_sf13_stale_link_warns` — stale feature link triggers WARN
- `test_sf13_valid_links_pass` — valid links (folder exists) pass silently

---

### spec-skill-improvements/8 — validate.sh SF14

**Requirements:** R-sf14 (SF14 scope conflict detection)

**Scope:** Extend `.agents/skills/spec/scripts/validate.sh`.

**Changes:**
- Add `check_sf14_scope_conflicts()` function per tech.md spec
- Call it after SF13 in the validation sequence
- WARN on each conflicting ownership pair

**Verification:**
```bash
# Two features claiming the same scope item
mkdir -p /tmp/spec-sf14/.spec/features/{a,b}
printf '---\ntype: feature-product\nfeature: a\nsibling: tech.md\nparent: ../../product.md\nupdated: 2026-01-01\n---\n## Scope\n| Owns | Does not own |\n|---|---|\n| state machine | adapters |\n' > /tmp/spec-sf14/.spec/features/a/product.md
printf '---\ntype: feature-product\nfeature: b\nsibling: tech.md\nparent: ../../product.md\nupdated: 2026-01-01\n---\n## Scope\n| Owns | Does not own |\n|---|---|\n| state machine | hooks |\n' > /tmp/spec-sf14/.spec/features/b/product.md
(cd /tmp/spec-sf14 && bash "$OLDPWD/.agents/skills/spec/scripts/validate.sh") 2>&1 | grep -q 'SF14'
rm -rf /tmp/spec-sf14
```

**Test:** `tests/spec/run.sh` → add:
- `test_sf14_conflict_warns` — conflicting Owns terms trigger WARN
- `test_sf14_no_conflict_passes` — distinct Owns terms pass silently

---

## Requirements Trace

| Requirement | Unit(s) | Validator |
|---|---|---|
| Specialized subagent role profiles | 2, 3 | test_skill_roles_section |
| Compound promotion automation | 5 | test_promote_*, shellcheck |
| Tag-based lessons extraction | 6 | test_lessons_for_*, shellcheck |
| Cavekit-aware output profiles | 4 | test_feature_output_profiles |
| SKILL.md metadata enrichment | 1 | test_skill_metadata_fields |
| SF13 cross-reference integrity | 7 | test_sf13_* |
| SF14 scope conflict detection | 8 | test_sf14_* |
| Routing expansion + argument-hint | 3 | test_skill_routing_new_routes |
