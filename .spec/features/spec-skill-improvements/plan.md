---
type: feature-plan
feature: spec-skill-improvements
parent: ../../plan.md
updated: 2026-06-29
implemented: 2026-06-29
---

# spec-skill-improvements — Implementation Plan

Stable unit IDs: `spec-skill-improvements/n`. IDs never renumber on reorder;
add a new ID for new work. All units are additive — no existing spec or script
is deleted; only extended or supplemented.

**Prerequisite:** spec skill bundle DONE (gate from root plan.md).

---

## Unit Table

| ID | Title | Priority | Requires | Status |
|---|---|---|---|---|
| spec-skill-improvements/1 | SKILL.md v2.0 frontmatter (allowed-tools, context, subagents, superpowers) | HIGH | — | done |
| spec-skill-improvements/2 | `## Roles` manifest section in SKILL.md | HIGH | 1 | done |
| spec-skill-improvements/3 | Routing expansion (`diff`, `health`, `research`, `lessons`, `interview`, `promote`) | MEDIUM | 1, 2 | done |
| spec-skill-improvements/4 | `feature.md` output profiles (Cavekit lite/full/ultra) | MEDIUM | — | done |
| spec-skill-improvements/5 | `scripts/promote.sh` | MEDIUM | — | done |
| spec-skill-improvements/6 | `scripts/lessons-for.sh` | MEDIUM | — | done |
| spec-skill-improvements/7 | `validate.sh` SF13 — stale link checker | MEDIUM | — | done |
| spec-skill-improvements/8 | `validate.sh` SF14 — scope conflict detector | MEDIUM | — | done |
| spec-skill-improvements/9 | Branch doc templates (product-topic, tech-topic, plan-topic, research) | HIGH | — | done |
| spec-skill-improvements/10 | `scripts/scan-merges.sh` | MEDIUM | — | done |
| spec-skill-improvements/11 | OpenSpec machine-readable frontmatter (requirements:, units:) | MEDIUM | — | done |
| spec-skill-improvements/12 | `agents/spec-tracer/SKILL.md` | MEDIUM | 1 | done |
| spec-skill-improvements/13 | `agents/spec-promoter/SKILL.md` | MEDIUM | 1, 5 | done |
| spec-skill-improvements/14 | `agents/spec-interviewer/SKILL.md` | LOW | 1, 2 | done |
| spec-skill-improvements/15 | `agents/spec-health/SKILL.md` | LOW | 1, 7, 8, 16 | done |
| spec-skill-improvements/16 | `validate.sh` SF15 (root length) + SF16 (lessons tags) | LOW | — | done |
| spec-skill-improvements/17 | Composable agents/ folder wiring in root SKILL.md | STRUCTURAL | 12, 13, 14, 15 | done |
| spec-skill-improvements/18 | Interactive setup interview + `.spec/.config.yaml` | HIGH | 1 | done |
| spec-skill-improvements/19 | Config read at session start (SKILL.md `## Config` + behavior adjustments) | HIGH | 18 | done |

Explicitly NOT in scope: `scripts/interview.sh`. WHAT-phase interviewing is
done by `superpowers:brainstorming` with `feature.md § Interview for WHAT` as
constraint context. No custom interview CLI is needed or wanted.

Deferred (tier-3, not in this feature):
- `scripts/score.sh` (spec-specific quality metrics for superpowers context)

---

## Unit Detail

### spec-skill-improvements/1 — SKILL.md v2.0 frontmatter

**Requirements:** R-metadata, R-allowed-tools, R-skill-manifest

**Scope:** Edit `.agents/skills/spec/SKILL.md` frontmatter only.

**Changes:**
- Bump `version:` to `2.0`
- Add `allowed-tools:` list: Read, Edit, Write, Glob, Grep, Bash, Agent
- Add `context:` list: feature.md, strategy.md, reference/product.md, reference/tech.md, reference/plan.md
- Add `agents:` map: spec-tracer, spec-promoter, spec-interviewer, spec-health → their SKILL.md paths
- Add `superpowers:` list: brainstorming, writing-plans, code-explorer, code-architect, verification-before-completion
- Add `outputs:`, `reads:`, `delegates:`, `phases:`, `caveman:` YAML fields

**Verification:**
```bash
grep -q 'version: 2.0' .agents/skills/spec/SKILL.md
grep -q 'allowed-tools:' .agents/skills/spec/SKILL.md
grep -q 'context:' .agents/skills/spec/SKILL.md
grep -q 'agents:' .agents/skills/spec/SKILL.md
grep -q 'superpowers:' .agents/skills/spec/SKILL.md
grep -q 'delegates:' .agents/skills/spec/SKILL.md
grep -q 'caveman:' .agents/skills/spec/SKILL.md
```

**Test:** `tests/spec/run.sh` → add `test_skill_v2_frontmatter_fields` covering
presence of each new frontmatter key and the version bump.

---

### spec-skill-improvements/2 — `## Roles` manifest section in SKILL.md

**Requirements:** R-roles (Subagent role profiles as superpowers delegation contexts)

**Scope:** Edit `.agents/skills/spec/SKILL.md` body — add `## Roles` section
before `## Routing`. This section is a routing manifest: it names executors
and constraint documents. The actual subagent SKILL.md files live under
`agents/` and are authored in units 12-15.

**Changes:**
- Add `## Roles` section with subsections:
  `### Role: spec-interviewer`, `spec-architect`, `spec-planner`, `spec-auditor`, `spec-compactor`
- Each subsection: Executor, Phase, Constraint document to inject, Inputs,
  Outputs, Validation criteria on output
- spec-interviewer: executor = `superpowers:brainstorming`; constraint = `feature.md § Interview for WHAT`
- spec-planner: executor = `superpowers:writing-plans`; constraint = `reference/plan.md`
- spec-auditor: executor = validate.sh (no superpower substitute); passes JSON to verification-before-completion
- spec-compactor: executor = promote.sh for extraction; lesson is human-reviewed before appending

**Verification:**
```bash
grep -q '## Roles' .agents/skills/spec/SKILL.md
grep -q 'spec-interviewer' .agents/skills/spec/SKILL.md
grep -q 'spec-planner' .agents/skills/spec/SKILL.md
grep -q 'spec-auditor' .agents/skills/spec/SKILL.md
grep -q 'spec-compactor' .agents/skills/spec/SKILL.md
```

**Test:** `tests/spec/run.sh` → add `test_skill_roles_section` asserting
all five role headers are present.

---

### spec-skill-improvements/3 — Routing expansion

**Requirements:** R-routing, R-routing-extended

**Scope:** Edit `.agents/skills/spec/SKILL.md` — update `argument-hint` and
`## Routing` table.

**Changes:**
- Update `argument-hint:` to full route list
- Add rows to Routing table for: `interview [<name>]`, `promote <name>`,
  `audit`, `lessons-for <tag>`, `diff <name>`, `health`, `research <name>`,
  `lessons <tag>` (alias for lessons-for)

**Prerequisite:** Units 1, 2.

**Verification:**
```bash
grep -q 'interview' .agents/skills/spec/SKILL.md
grep -q 'promote' .agents/skills/spec/SKILL.md
grep -q 'audit' .agents/skills/spec/SKILL.md
grep -q 'lessons-for' .agents/skills/spec/SKILL.md
grep -q 'diff' .agents/skills/spec/SKILL.md
grep -q 'health' .agents/skills/spec/SKILL.md
grep -q 'research' .agents/skills/spec/SKILL.md
```

**Test:** `tests/spec/run.sh` → add `test_skill_routing_new_routes` asserting
all route names appear in SKILL.md routing table.

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

### spec-skill-improvements/9 — Branch doc templates

**Requirements:** R-branch-templates

**Priority:** HIGH — small, structural, no new concepts.

**Scope:** Create four template files under
`.agents/skills/spec/reference/templates/`:
- `product-topic.md`
- `tech-topic.md`
- `plan-topic.md`
- `research.md`

**Changes:** New files only; content per tech.md spec. No changes to existing
templates or scripts.

**Verification:**
```bash
ls .agents/skills/spec/reference/templates/product-topic.md
ls .agents/skills/spec/reference/templates/tech-topic.md
ls .agents/skills/spec/reference/templates/plan-topic.md
ls .agents/skills/spec/reference/templates/research.md
grep -q 'type: product-topic' .agents/skills/spec/reference/templates/product-topic.md
grep -q 'type: tech-topic' .agents/skills/spec/reference/templates/tech-topic.md
```

**Test:** `tests/spec/run.sh` → add `test_branch_doc_templates_exist` asserting
all four template files are present and have valid frontmatter types.

---

### spec-skill-improvements/10 — `scripts/scan-merges.sh`

**Requirements:** R-scan-merges

**Priority:** MEDIUM.

**Scope:** Create `.agents/skills/spec/scripts/scan-merges.sh`.

**Changes:**
- New file: full implementation per tech.md spec
- Scans all feature tech.md files or one named feature
- Formats as table, json, or plain
- Exits 1 on unclosed marker; exits 0 otherwise
- Read-only; never modifies files

**Verification:**
```bash
# With a feature that has merge blocks:
bash .agents/skills/spec/scripts/scan-merges.sh spec-skill-improvements
# Exit 0; table output shows blocks found
# Run shellcheck
shellcheck .agents/skills/spec/scripts/scan-merges.sh
```

**Test:** `tests/spec/run.sh` → add:
- `test_scan_merges_finds_blocks` — reports blocks in feature with markers
- `test_scan_merges_unclosed_exits_nonzero` — unclosed marker triggers exit 1
- `test_scan_merges_empty_feature` — feature with no markers exits 0, empty output

---

### spec-skill-improvements/11 — OpenSpec machine-readable frontmatter

**Requirements:** R-openspec

**Priority:** MEDIUM.

**Scope:** Edit `.agents/skills/spec/reference/product.md` and
`.agents/skills/spec/reference/plan.md` to document the optional
`requirements:` and `units:` frontmatter fields. No changes to validate.sh
(convention is warn-only, opt-in).

**Changes:**
- `reference/product.md`: Add `## OpenSpec frontmatter (optional)` section
  documenting the `requirements:` list format with id, title, strength, scenarios
- `reference/plan.md`: Add `## OpenSpec frontmatter (optional)` section
  documenting the `units:` list format with id, title, status, requires
- Note in both: "opt-in; validate.sh does not error on absence"

**Verification:**
```bash
grep -q 'requirements:' .agents/skills/spec/reference/product.md
grep -q 'units:' .agents/skills/spec/reference/plan.md
grep -q 'opt-in' .agents/skills/spec/reference/product.md
```

**Test:** `tests/spec/run.sh` → add `test_openspec_docs_present` asserting
the optional frontmatter sections exist in both reference files.

---

### spec-skill-improvements/12 — `agents/spec-tracer/SKILL.md`

**Requirements:** R-composable

**Priority:** MEDIUM.

**Scope:** Create `.agents/skills/spec/agents/spec-tracer/SKILL.md`.

**Invariants:**
- `allowed-tools:` must be `[Read, Glob, Grep]` — no write tools
- Must be parallel-safe (no state written; safe to run concurrently)
- Output is a structured trace document, not a product or tech spec

**Verification:**
```bash
ls .agents/skills/spec/agents/spec-tracer/SKILL.md
grep -q 'allowed-tools' .agents/skills/spec/agents/spec-tracer/SKILL.md
grep -q 'Read' .agents/skills/spec/agents/spec-tracer/SKILL.md
# Confirm no write tools listed:
grep -v 'Edit\|Write\|Bash' .agents/skills/spec/agents/spec-tracer/SKILL.md | grep -q 'allowed-tools'
```

**Test:** `tests/spec/run.sh` → add `test_spec_tracer_read_only` asserting
SKILL.md lists no write tools in allowed-tools.

---

### spec-skill-improvements/13 — `agents/spec-promoter/SKILL.md`

**Requirements:** R-composable

**Priority:** MEDIUM.

**Depends on:** spec-skill-improvements/5 (promote.sh must exist)

**Scope:** Create `.agents/skills/spec/agents/spec-promoter/SKILL.md`.

**Invariants:**
- MUST run promote.sh with `--dry-run` first; MUST show output to user
- MUST require explicit human confirmation before live run
- `allowed-tools:` is `[Read, Bash]`

**Verification:**
```bash
ls .agents/skills/spec/agents/spec-promoter/SKILL.md
grep -q 'dry-run' .agents/skills/spec/agents/spec-promoter/SKILL.md
grep -q 'confirm' .agents/skills/spec/agents/spec-promoter/SKILL.md
```

**Test:** `tests/spec/run.sh` → add `test_spec_promoter_diff_first` asserting
the SKILL.md contains dry-run + confirmation language.

---

### spec-skill-improvements/14 — `agents/spec-interviewer/SKILL.md`

**Requirements:** R-composable

**Priority:** LOW.

**Scope:** Create `.agents/skills/spec/agents/spec-interviewer/SKILL.md`.

**Invariants:**
- MUST inject `feature.md § Interview for WHAT` before delegating
- MUST offer delegation (not silently invoke brainstorming)
- `allowed-tools:` is `[Read]`

**Verification:**
```bash
ls .agents/skills/spec/agents/spec-interviewer/SKILL.md
grep -q 'Interview for WHAT' .agents/skills/spec/agents/spec-interviewer/SKILL.md
grep -q 'brainstorming' .agents/skills/spec/agents/spec-interviewer/SKILL.md
```

**Test:** `tests/spec/run.sh` → add `test_spec_interviewer_constraint_injection`
asserting constraint reference and delegation offer language are present.

---

### spec-skill-improvements/15 — `agents/spec-health/SKILL.md`

**Requirements:** R-composable

**Priority:** LOW.

**Scope:** Create `.agents/skills/spec/agents/spec-health/SKILL.md`.

**Invariants:**
- MUST run validate.sh and capture output
- MUST check: feature-plan-sequence alignment, oversized root specs, missing unit IDs
- `allowed-tools:` is `[Read, Glob, Grep, Bash]`
- Output is prioritised list: CRITICAL / WARN / INFO

**Verification:**
```bash
ls .agents/skills/spec/agents/spec-health/SKILL.md
grep -q 'validate.sh' .agents/skills/spec/agents/spec-health/SKILL.md
grep -q 'CRITICAL' .agents/skills/spec/agents/spec-health/SKILL.md
```

**Test:** `tests/spec/run.sh` → add `test_spec_health_output_levels` asserting
CRITICAL/WARN/INFO levels are documented in SKILL.md.

---

### spec-skill-improvements/16 — `validate.sh` SF15 + SF16

**Requirements:** R-sf15-sf16

**Priority:** LOW.

**Scope:** Extend `.agents/skills/spec/scripts/validate.sh`.

**Changes:**
- SF15: add `check_sf15_root_spec_length()` — warn when root spec > 200 lines
  (configurable via `SPEC_ROOT_MAX_LINES` env)
- SF16: add `check_sf16_lessons_tags()` — warn when lesson entry has no `**Tags:**`

**Verification:**
```bash
# SF15: create oversized root spec
python3 -c "print('---\ntype: entrypoint\nscope: product\nupdated: 2026-01-01\n---\n'); [print(f'## Section {i}\nLine.') for i in range(100)]" > /tmp/spec-sf15/.spec/product.md
(cd /tmp/spec-sf15 && bash "$OLDPWD/.agents/skills/spec/scripts/validate.sh") 2>&1 | grep -q 'SF15'
# SF16: create lesson without Tags
(cd /tmp/spec-sf15 && echo $'### Missing tags\n**Pattern:** test\n**Rule:** test\n**Date:** 2026' >> .spec/lessons.md)
bash .agents/skills/spec/scripts/validate.sh 2>&1 | grep -q 'SF16'
```

**Test:** `tests/spec/run.sh` → add:
- `test_sf15_long_root_warns` — root spec >200 lines triggers WARN
- `test_sf16_lesson_no_tags_warns` — tagless lesson triggers WARN
- `test_sf16_lesson_with_tags_passes` — tagged lesson passes silently

---

### spec-skill-improvements/18 — Interactive setup interview + `.spec/.config.yaml`

**Requirements:** R-setup-interview

**Priority:** HIGH.

**Depends on:** spec-skill-improvements/1 (v2.0 frontmatter defines allowed-tools
that includes Edit/Write — needed to create `.spec/.config.yaml`).

**Scope:** Two changes:
1. Replace the `## Setup` section in `.agents/skills/spec/SKILL.md` with the
   4-question interview flow per tech.md spec
2. Add `## Config` subsection documenting `.spec/.config.yaml` format and how
   the skill reads it at session start

**Changes:**
- `SKILL.md § Setup` — full replacement with interview flow (Q1-Q4), summary +
  confirm step, post-setup next-steps, and re-run branch
- `SKILL.md` — add `## Config` section before `## Setup, Templates, Validation`
  documenting config keys, defaults when absent, and behavior adjustments per key
- No changes to `setup.sh` — mechanical script is unchanged; interview is agent-layer only

**Invariants:**
- Interview MUST be conversational; agent explains each choice before asking
- `setup.sh` MUST NOT be called until the user confirms the summary
- `.spec/.config.yaml` MUST be written even when user accepts all defaults
- Re-run MUST NOT repeat the full interview when config already exists
- `suggest-superpowers: false` MUST suppress all "Superpower tip" callouts
  across feature.md, strategy.md, and any subagent files

**Verification:**
```bash
grep -q '## Config' .agents/skills/spec/SKILL.md
grep -q '.config.yaml' .agents/skills/spec/SKILL.md
grep -q 'Q1' .agents/skills/spec/SKILL.md
grep -q 'suggest-superpowers' .agents/skills/spec/SKILL.md
# Setup section no longer just a one-liner:
wc -l .agents/skills/spec/SKILL.md | awk '{print $1}' # should be substantially longer
```

**Test:** `tests/spec/run.sh` → add:
- `test_config_defaults_when_absent` — skill reads absent config and applies defaults
- `test_setup_section_has_interview_flow` — SKILL.md Setup section contains Q1-Q4 markers

---

### spec-skill-improvements/17 — Composable agents/ folder wiring

**Requirements:** R-composable

**Priority:** STRUCTURAL (depends on 12, 13, 14, 15).

**Scope:** Wire the `agents/` folder into root SKILL.md routing.

**Changes:**
- Confirm `agents:` map in frontmatter (from unit 1) references correct paths
- Confirm `## Routing` table (from unit 3) references subagent invocations for
  `interview`, `promote`, `diff`, `health`
- No new files — this is integration verification that units 1, 3, 12-15 work together

**Verification:**
```bash
# All subagent SKILL.md files present
ls .agents/skills/spec/agents/spec-tracer/SKILL.md
ls .agents/skills/spec/agents/spec-promoter/SKILL.md
ls .agents/skills/spec/agents/spec-interviewer/SKILL.md
ls .agents/skills/spec/agents/spec-health/SKILL.md
# Root SKILL.md subagents: map populated
grep -A 5 'agents:' .agents/skills/spec/SKILL.md | grep -q 'spec-tracer'
```

**Test:** `tests/spec/run.sh` → add `test_subagents_folder_wiring` asserting
all four subagent paths in SKILL.md frontmatter map to existing files.

---

### spec-skill-improvements/19 — Config read at session start

**Requirements:** R-config-read

**Priority:** HIGH.

**Depends on:** spec-skill-improvements/18 (config format defined by setup interview).

**Scope:** Edit `.agents/skills/spec/SKILL.md`:
1. Add `## Config` section documenting `.spec/.config.yaml` keys, defaults, and
   behavior adjustments applied per key
2. Add config-read step to session-start flow (before delegating to any role)

**Changes:**
- `SKILL.md § Config` — new section before `## Setup`: documents all config keys,
  defaults when absent, and per-key behavior adjustments (suggest-superpowers,
  vibe-flow, caveman, superpowers.*)
- `SKILL.md § Session start` or preamble — add: "Check for `.spec/.config.yaml`;
  load and apply config adjustments silently"
- `suggest-superpowers: false` propagation note in feature.md, strategy.md preamble

**Invariants:**
- Absent or partial config MUST use documented defaults without error
- Config read MUST happen before any offer or execution so `suggest-superpowers`
  suppression takes effect from the first step

**Verification:**
```bash
grep -q '## Config' .agents/skills/spec/SKILL.md
grep -q '.config.yaml' .agents/skills/spec/SKILL.md
grep -q 'suggest-superpowers' .agents/skills/spec/SKILL.md
grep -q 'defaults' .agents/skills/spec/SKILL.md
```

**Test:** `tests/spec/run.sh` → add:
- `test_config_section_present` — SKILL.md contains `## Config` section with all four key types
- `test_config_defaults_documented` — each key has a documented default value

---

## Requirements Trace

| Requirement | Unit(s) | Validator |
|---|---|---|
| R-metadata: SKILL.md metadata enrichment | 1 | test_skill_v2_frontmatter_fields |
| R-allowed-tools: allowed-tools expansion | 1 | test_skill_v2_frontmatter_fields |
| R-skill-manifest: agents/superpowers declaration | 1 | test_skill_v2_frontmatter_fields |
| R-roles: Subagent role profiles (delegation manifest) | 2, 3 | test_skill_roles_section |
| R-routing: Routing expansion + argument-hint | 3 | test_skill_routing_new_routes |
| R-routing-extended: diff/health/research/lessons | 3 | test_skill_routing_new_routes |
| R-cavekit: Cavekit-aware output profiles | 4 | test_feature_output_profiles |
| R-promote: Compound promotion automation | 5, 13 | test_promote_*, test_spec_promoter_diff_first |
| R-lessons: Tag-based lessons extraction | 6 | test_lessons_for_*, shellcheck |
| R-sf13: SF13 cross-reference integrity | 7 | test_sf13_* |
| R-sf14: SF14 scope conflict detection | 8 | test_sf14_* |
| R-branch-templates: Branch doc templates | 9 | test_branch_doc_templates_exist |
| R-scan-merges: scan-merges.sh | 10 | test_scan_merges_* |
| R-openspec: OpenSpec machine-readable frontmatter | 11 | test_openspec_docs_present |
| R-composable: Composable subagent architecture | 12, 13, 14, 15, 17 | test_subagents_folder_wiring |
| R-sf15-sf16: SF15 root length + SF16 lessons tags | 16 | test_sf15_*, test_sf16_* |
| R-setup-interview: Interactive setup interview + config write | 18 | test_setup_section_has_interview_flow |
| R-config-read: Config read at session start + behavior adjustments | 19 | test_config_section_present, test_config_defaults_documented |
