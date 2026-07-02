# spec-skill-improvements Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Elevate the spec skill from v1.8 to v2.0 by adding composable subagent SKILL.md files, new automation scripts, validator extensions, branch doc templates, and a config-driven setup interview.

**Architecture:** All changes are additive — the two-layer spec model and 6-step authoring flow are untouched. SKILL.md gets enriched YAML frontmatter and new sections; four subagent SKILL.md files land under `agents/`; three bash scripts are added to `scripts/`; `validate.sh` gains four new checks (SF13–SF16); `feature.md` gains output profiles; branch doc templates land in `reference/templates/`.

**Tech Stack:** Bash (3.2-compatible; macOS + Linux), YAML frontmatter, Markdown. No external dependencies beyond coreutils. Tests live in `tests/spec/run.sh` (existing file, pure bash, no bats).

---

## File Map

**New files:**
- `.agents/skills/spec/agents/spec-tracer/SKILL.md`
- `.agents/skills/spec/agents/spec-promoter/SKILL.md`
- `.agents/skills/spec/agents/spec-interviewer/SKILL.md`
- `.agents/skills/spec/agents/spec-health/SKILL.md`
- `.agents/skills/spec/scripts/promote.sh`
- `.agents/skills/spec/scripts/lessons-for.sh`
- `.agents/skills/spec/scripts/scan-merges.sh`
- `.agents/skills/spec/reference/templates/product-topic.md`
- `.agents/skills/spec/reference/templates/tech-topic.md`
- `.agents/skills/spec/reference/templates/plan-topic.md`
- `.agents/skills/spec/reference/templates/research.md`

**Modified files:**
- `.agents/skills/spec/SKILL.md` — frontmatter v2.0, `## Roles`, routing expansion, `## Config`, `## Setup` interview
- `.agents/skills/spec/feature.md` — add `## Output profiles`
- `.agents/skills/spec/scripts/validate.sh` — add SF13, SF14, SF15, SF16
- `.agents/skills/spec/reference/product.md` — OpenSpec `requirements:` docs
- `.agents/skills/spec/reference/plan.md` — OpenSpec `units:` docs
- `tests/spec/run.sh` — new test functions appended

---

## Task 1: SKILL.md v2.0 frontmatter (unit 1)

Replace the existing single-line `allowed-tools` and `metadata.version: "1.8"` with structured YAML fields.

**Files:**
- Modify: `.agents/skills/spec/SKILL.md` (frontmatter block, lines 1–18)
- Test: `tests/spec/run.sh` (append `test_skill_v2_frontmatter_fields`)

- [ ] **Step 1: Write the failing test**

Append to `tests/spec/run.sh` before the final `echo "=== results..."` line:

```bash
echo ""
echo "=== spec-skill-improvements ssi/1: SKILL.md v2.0 frontmatter ==="
test_skill_v2_frontmatter_fields() {
  local skill; skill="$(cat "$SPEC_SKILL/SKILL.md")"
  assert_contains "ssi/1" 'version: "2.0"' "$skill" 'version: "2.0"'
  assert_contains "ssi/1" "allowed-tools list entry" "$skill" '- Edit'
  assert_contains "ssi/1" "allowed-tools Write entry" "$skill" '- Write'
  assert_contains "ssi/1" "context: field present" "$skill" 'context:'
  assert_contains "ssi/1" "agents: field present" "$skill" 'agents:'
  assert_contains "ssi/1" "superpowers: field present" "$skill" 'superpowers:'
  assert_contains "ssi/1" "delegates: field present" "$skill" 'delegates:'
  assert_contains "ssi/1" "caveman: field present" "$skill" 'caveman:'
}
test_skill_v2_frontmatter_fields
```

- [ ] **Step 2: Run test to verify it fails**

```bash
bash tests/spec/run.sh 2>&1 | grep 'ssi/1'
```

Expected: multiple `FAIL [ssi/1]` lines.

- [ ] **Step 3: Replace SKILL.md frontmatter**

Replace the entire frontmatter block (lines 1–18) of `.agents/skills/spec/SKILL.md` with:

```yaml
---
name: spec
description: |
  Maintains `.spec/` design docs across two layers: persistent root specs
  (product, tech, design, plan, lessons, branch docs) kept current with no
  backlog, and branch-scoped `features/<name>/` folders that merge to root,
  archive transiently, then delete before the branch merges.
  Use when scoping a feature, bootstrapping strategy, reviewing architecture,
  updating a spec, validating consistency, or the user mentions spec, PRD,
  design doc, tech design, feature spec, or branch doc.
user-invocable: true
argument-hint: "[strategy|feature [<name>]|interview [<name>]|promote <name>|audit|diff <name>|health|research <name>|lessons-for <tag>|product|tech|design|plan|lessons|setup|validate]"
compatibility: Requires bash. macOS and Linux. bash 3.2-compatible.
metadata:
  author: lennarddib
  version: "2.0"

allowed-tools:
  - Read
  - Edit
  - Write
  - Glob
  - Grep
  - Bash
  - Agent

context:
  - .agents/skills/spec/feature.md
  - .agents/skills/spec/strategy.md
  - .agents/skills/spec/reference/product.md
  - .agents/skills/spec/reference/tech.md
  - .agents/skills/spec/reference/plan.md

agents:
  - spec-tracer:      agents/spec-tracer/SKILL.md
  - spec-promoter:    agents/spec-promoter/SKILL.md
  - spec-interviewer: agents/spec-interviewer/SKILL.md
  - spec-health:      agents/spec-health/SKILL.md

superpowers:
  - superpowers:brainstorming
  - superpowers:writing-plans
  - code-explorer
  - code-architect
  - superpowers:verification-before-completion
  - superpowers:finishing-a-development-branch

outputs:
  - .spec/features/<name>/product.md
  - .spec/features/<name>/tech.md
  - .spec/features/<name>/design.md
  - .spec/features/<name>/plan.md
  - .spec/product.md
  - .spec/tech.md
  - .spec/lessons.md

reads:
  - .spec/product.md
  - .spec/tech.md
  - .spec/design.md
  - .spec/lessons.md
  - .spec/plan.md
  - .spec/.config.yaml

delegates:
  - role: spec-interviewer
    when: feature product.md WHAT phase (steps 1-2)
    superpowers: [superpowers:brainstorming]
  - role: spec-architect
    when: feature tech.md HOW phase (steps 3-4)
    superpowers: [code-explorer, code-architect]
  - role: spec-auditor
    when: validate / audit
    superpowers: []
  - role: spec-compactor
    when: compound (wrap-up, promote, record)
    superpowers: [superpowers:finishing-a-development-branch]

phases:
  - setup.apply
  - strategy.spec
  - feature.design
  - feature.plan
  - feature.compound
  - strategy.compound

caveman:
  lite: design+plan phases; Scope table + req titles; file paths; unit IDs only
  full: impl reference + verify; all sections per template
  ultra: compound receipts; all sections + evidence + traceability matrix
---
```

Keep everything after the frontmatter closing `---` exactly as-is.

- [ ] **Step 4: Run test to verify it passes**

```bash
bash tests/spec/run.sh 2>&1 | grep 'ssi/1'
```

Expected: all `PASS [ssi/1]` lines.

- [ ] **Step 5: Run full validate**

```bash
bash .agents/skills/spec/scripts/validate.sh
```

Expected: `Errors: 0`.

- [ ] **Step 6: Commit**

```bash
git add .agents/skills/spec/SKILL.md tests/spec/run.sh
git commit -m "feat(spec): bump SKILL.md to v2.0 frontmatter with agents/superpowers/caveman fields"
```

---

## Task 2: Branch doc templates (unit 9)

Creates four template files in `reference/templates/`.

**Files:**
- Create: `.agents/skills/spec/reference/templates/product-topic.md`
- Create: `.agents/skills/spec/reference/templates/tech-topic.md`
- Create: `.agents/skills/spec/reference/templates/plan-topic.md`
- Create: `.agents/skills/spec/reference/templates/research.md`
- Test: `tests/spec/run.sh` (append `test_branch_doc_templates_exist`)

- [ ] **Step 1: Write the failing test**

Append to `tests/spec/run.sh` before the final `echo "=== results..."` line:

```bash
echo ""
echo "=== spec-skill-improvements ssi/9: branch doc templates ==="
test_branch_doc_templates_exist() {
  local templates="$SPEC_SKILL/reference/templates"
  for t in product-topic tech-topic plan-topic research; do
    if [[ -f "$templates/${t}.md" ]]; then
      pass "ssi/9" "${t}.md template exists"
    else
      fail "ssi/9" "${t}.md template exists"
    fi
  done
  local pt; pt="$(cat "$templates/product-topic.md" 2>/dev/null || true)"
  assert_contains "ssi/9" "product-topic has type: product-topic" "$pt" 'type: product-topic'
  local tt; tt="$(cat "$templates/tech-topic.md" 2>/dev/null || true)"
  assert_contains "ssi/9" "tech-topic has type: tech-topic" "$tt" 'type: tech-topic'
}
test_branch_doc_templates_exist
```

- [ ] **Step 2: Run test to verify it fails**

```bash
bash tests/spec/run.sh 2>&1 | grep 'ssi/9'
```

Expected: `FAIL [ssi/9]` for each template.

- [ ] **Step 3: Create product-topic.md**

Create `.agents/skills/spec/reference/templates/product-topic.md`:

```markdown
---
type: product-topic
topic: <topic>
parent: product.md
scope: cross-cutting
covers: <what cross-cutting concern this covers>
updated: YYYY-MM-DD
---

# <topic> — Product

One paragraph: why this cross-cutting concern exists and who it affects.

## Principles

- Principle 1
- Principle 2

## Requirements

### Requirement: <title>

<feature> SHALL/MUST <observable behaviour>.

#### Scenario: <title>

**Given** …
**When** …
**Then** …

**When to create:** Only when a concern truly spans every feature — not as a
substitute for a feature folder. Examples: design system, accessibility
conventions, naming conventions that every feature must enforce.
```

- [ ] **Step 4: Create tech-topic.md**

Create `.agents/skills/spec/reference/templates/tech-topic.md`:

```markdown
---
type: tech-topic
topic: <topic>
parent: tech.md
scope: cross-cutting
covers: <what cross-cutting technical concern this governs>
updated: YYYY-MM-DD
---

# <topic> — Tech

One paragraph: what this cross-cutting technical concern governs.

## Contract

<Invariant every feature must satisfy.>

## File Layout

```text
path/to/shared/
├── file.ext     ← purpose
```

## Integration Points

| Feature | How it integrates |
|---|---|
| <name> | … |

## Risks

- Risk 1
```

- [ ] **Step 5: Create plan-topic.md**

Create `.agents/skills/spec/reference/templates/plan-topic.md`:

```markdown
---
type: plan-topic
topic: <topic>
parent: plan.md
scope: cross-cutting
covers: <what multi-feature sequence this sub-plan tracks>
updated: YYYY-MM-DD
---

# <topic> — Sub-plan

One paragraph: why this cross-cutting plan exists separately from root plan.md.

## Sequence

| Step | Feature or sub-task | Gate |
|---|---|---|
| 1 | … | … |

## Current Focus

<What is being worked on now in this sub-plan.>

**When to create:** Only when root `plan.md` grows unwieldy because a
cross-cutting plan (e.g. a multi-feature migration) needs its own sequence.
Default to keeping everything in root `plan.md`.
```

- [ ] **Step 6: Create research.md**

Create `.agents/skills/spec/reference/templates/research.md`:

```markdown
---
type: feature-research
feature: <name>
parent: product.md
updated: YYYY-MM-DD
---

# <name> — Research

Discovery artifacts for the feature. Deleted with the feature folder at
wrapup — never promoted to root. If a finding should persist, extract it
to a lesson (lessons.md) or root tech.md section.

## Question

<What are we trying to learn?>

## Findings

### Finding: <title>

<What was found and where.>

## Decision

<What the research led to — and what was ruled out.>
```

- [ ] **Step 7: Run test to verify it passes**

```bash
bash tests/spec/run.sh 2>&1 | grep 'ssi/9'
```

Expected: all `PASS [ssi/9]`.

- [ ] **Step 8: Commit**

```bash
git add .agents/skills/spec/reference/templates/
git add tests/spec/run.sh
git commit -m "feat(spec): add branch doc and research templates to reference/templates/"
```

---

## Task 3: feature.md output profiles (unit 4)

Adds `## Output profiles` section to `feature.md`.

**Files:**
- Modify: `.agents/skills/spec/feature.md`
- Test: `tests/spec/run.sh` (append `test_feature_output_profiles`)

- [ ] **Step 1: Write the failing test**

Append to `tests/spec/run.sh` before the final `echo "=== results..."` line:

```bash
echo ""
echo "=== spec-skill-improvements ssi/4: feature.md output profiles ==="
test_feature_output_profiles() {
  local f; f="$(cat "$SPEC_SKILL/feature.md")"
  assert_contains "ssi/4" "## Output profiles section" "$f" '## Output profiles'
  assert_contains "ssi/4" "Lite profile defined" "$f" '### Lite'
  assert_contains "ssi/4" "Full profile defined" "$f" '### Full'
  assert_contains "ssi/4" "Ultra profile defined" "$f" '### Ultra'
}
test_feature_output_profiles
```

- [ ] **Step 2: Run test to verify it fails**

```bash
bash tests/spec/run.sh 2>&1 | grep 'ssi/4'
```

Expected: all `FAIL [ssi/4]`.

- [ ] **Step 3: Read feature.md to find insertion point**

```bash
grep -n '## ' .agents/skills/spec/feature.md | head -20
```

Find the line number of `## Feature authoring flow` and the section after it. Insert `## Output profiles` after `## Feature authoring flow` section ends.

- [ ] **Step 4: Add output profiles section**

In `.agents/skills/spec/feature.md`, append this section after `## Feature authoring flow` (before the next `## ` heading):

```markdown
## Output profiles

Match caveman level to phase; compress at the source, not the sink.

### Lite (feature.design, feature.plan)

Produce precise, minimal spec sections — not truncated prose but no padding.

| File | Mandatory | Compressed | Omit |
|---|---|---|---|
| product.md | Frontmatter, problem paragraph, Scope table, req titles + RFC strength | Scenarios → one-line Given/When/Then | Rationale paragraphs, examples |
| tech.md | Frontmatter, file paths, interface signatures | Risks → one-line bullets | Implementation narrative, decision history |
| plan.md | Frontmatter, unit ID list with R-IDs | Verification table → command only | Full evidence prose |

### Full (feature.plan reference, feature.impl, feature.verify)

All sections per template; standard Requirement+Scenario blocks; full prose.
No compression. Default for inter-phase reading and reference.

### Ultra (feature.compound receipts)

All sections plus:
- Unit traceability matrix (unit ID → test path → pass/fail)
- Validation evidence summary (validate.sh run output)
- Draft lesson entry (pattern, rule, tags — human edits before promoting)
```

- [ ] **Step 5: Run test to verify it passes**

```bash
bash tests/spec/run.sh 2>&1 | grep 'ssi/4'
```

Expected: all `PASS [ssi/4]`.

- [ ] **Step 6: Commit**

```bash
git add .agents/skills/spec/feature.md tests/spec/run.sh
git commit -m "feat(spec): add lite/full/ultra output profiles to feature.md"
```

---

## Task 4: scripts/promote.sh (unit 5)

Creates the merge-marker extraction script with atomic rename and dry-run support.

**Files:**
- Create: `.agents/skills/spec/scripts/promote.sh`
- Test: `tests/spec/run.sh` (append promote tests)

- [ ] **Step 1: Write the failing tests**

Append to `tests/spec/run.sh` before the final `echo "=== results..."` line:

```bash
echo ""
echo "=== spec-skill-improvements ssi/5: promote.sh ==="
PROMOTE="$SPEC_SKILL/scripts/promote.sh"

test_promote_dry_run_no_mutation() {
  local d; d="$(mktmp)"
  mkdir -p "$d/.spec/features/feat"
  printf '# Root\n' > "$d/.spec/tech.md"
  printf '<!-- merge -->\n## Section\ncontent\n<!-- /merge -->\n' \
    > "$d/.spec/features/feat/tech.md"
  local before after out
  before="$(cat "$d/.spec/tech.md")"
  out="$(cd "$d" && bash "$PROMOTE" feat --dry-run 2>&1)"
  after="$(cat "$d/.spec/tech.md")"
  assert_contains "ssi/5" "dry-run prints preview" "$out" "would promote"
  if [[ "$before" == "$after" ]]; then
    pass "ssi/5" "dry-run leaves target unchanged"
  else
    fail "ssi/5" "dry-run leaves target unchanged"
  fi
  rm -rf "$d"
}

test_promote_reversed_markers_refused() {
  local d; d="$(mktmp)"
  mkdir -p "$d/.spec/features/bad"
  printf '# Root\n' > "$d/.spec/tech.md"
  printf '<!-- /merge -->\ncontent\n<!-- merge -->\n' \
    > "$d/.spec/features/bad/tech.md"
  local before after rc
  before="$(cat "$d/.spec/tech.md")"
  cd "$d" && bash "$PROMOTE" bad 2>/dev/null; rc=$?; cd - >/dev/null
  after="$(cat "$d/.spec/tech.md")"
  if [[ $rc -ne 0 ]]; then pass "ssi/5" "reversed markers exit non-zero"
  else fail "ssi/5" "reversed markers exit non-zero"; fi
  if [[ "$before" == "$after" ]]; then pass "ssi/5" "reversed markers leave target unchanged"
  else fail "ssi/5" "reversed markers leave target unchanged"; fi
  rm -rf "$d"
}

test_promote_valid_blocks() {
  local d; d="$(mktmp)"
  mkdir -p "$d/.spec/features/feat"
  printf '# Root\n' > "$d/.spec/tech.md"
  printf '<!-- merge -->\n## Section A\nline1\n<!-- /merge -->\n' \
    > "$d/.spec/features/feat/tech.md"
  local out after
  out="$(cd "$d" && bash "$PROMOTE" feat 2>&1)"
  after="$(cat "$d/.spec/tech.md")"
  assert_contains "ssi/5" "promote reports success" "$out" "PROMOTED"
  assert_contains "ssi/5" "target contains promoted content" "$after" "## Section A"
  rm -rf "$d"
}

test_promote_dry_run_no_mutation
test_promote_reversed_markers_refused
test_promote_valid_blocks
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
bash tests/spec/run.sh 2>&1 | grep 'ssi/5'
```

Expected: all `FAIL [ssi/5]` (script doesn't exist).

- [ ] **Step 3: Create promote.sh**

Create `.agents/skills/spec/scripts/promote.sh`:

```bash
#!/usr/bin/env bash
# promote.sh — extract <!-- merge --> blocks from feature tech.md and promote to root spec
set -euo pipefail

FEATURE="${1:?usage: promote.sh <feature-name> [--dry-run] [--target <file>]}"
DRY_RUN=0
TARGET=".spec/tech.md"

shift
while [[ $# -gt 0 ]]; do
  case "$1" in
    --dry-run) DRY_RUN=1 ;;
    --target)  TARGET="${2:?--target requires a path}"; shift ;;
    *) echo "Unknown option: $1" >&2; exit 1 ;;
  esac
  shift
done

FEATURE_TECH=".spec/features/$FEATURE/tech.md"
[[ -f "$FEATURE_TECH" ]] || { echo "ERROR: $FEATURE_TECH not found" >&2; exit 1; }
[[ -f "$TARGET" ]]       || { echo "ERROR: $TARGET not found" >&2; exit 1; }

BLOCKS=""
BLOCK_COUNT=0
in_block=0
current_block=""
lineno=0

while IFS= read -r line; do
  lineno=$((lineno + 1))
  if [[ "$line" == "<!-- merge -->" ]]; then
    if [[ $in_block -eq 1 ]]; then
      echo "ERROR: nested <!-- merge --> at line $lineno in $FEATURE_TECH" >&2
      exit 1
    fi
    in_block=1
    current_block=""
  elif [[ "$line" == "<!-- /merge -->" ]]; then
    if [[ $in_block -ne 1 ]]; then
      echo "ERROR: <!-- /merge --> without opener at line $lineno in $FEATURE_TECH" >&2
      exit 1
    fi
    in_block=0
    BLOCKS="${BLOCKS}${current_block}"$'\n'
    BLOCK_COUNT=$((BLOCK_COUNT + 1))
  elif [[ $in_block -eq 1 ]]; then
    current_block="${current_block}${line}"$'\n'
  fi
done < "$FEATURE_TECH"

if [[ $in_block -eq 1 ]]; then
  echo "ERROR: unclosed <!-- merge --> block in $FEATURE_TECH" >&2
  exit 1
fi

if [[ $BLOCK_COUNT -eq 0 ]]; then
  echo "WARN: no <!-- merge --> blocks found in $FEATURE_TECH" >&2
  exit 0
fi

if [[ $DRY_RUN -eq 1 ]]; then
  echo "--- would promote $BLOCK_COUNT block(s) to $TARGET ---"
  echo "$BLOCKS"
  exit 0
fi

TMPFILE="$(mktemp "$(dirname "$TARGET")/.promote.XXXXXX")"
cat "$TARGET" > "$TMPFILE"
printf '\n%s\n' "$BLOCKS" >> "$TMPFILE"
mv "$TMPFILE" "$TARGET"

echo "PROMOTED $BLOCK_COUNT block(s) from $FEATURE_TECH → $TARGET"
```

- [ ] **Step 4: Make executable and shellcheck**

```bash
chmod +x .agents/skills/spec/scripts/promote.sh
shellcheck .agents/skills/spec/scripts/promote.sh
```

Expected: no output (clean).

- [ ] **Step 5: Run tests to verify they pass**

```bash
bash tests/spec/run.sh 2>&1 | grep 'ssi/5'
```

Expected: all `PASS [ssi/5]`.

- [ ] **Step 6: Commit**

```bash
git add .agents/skills/spec/scripts/promote.sh tests/spec/run.sh
git commit -m "feat(spec): add scripts/promote.sh for merge-marker compound extraction"
```

---

## Task 5: scripts/lessons-for.sh (unit 6)

Tag-filtered lesson extractor. Bash 3.2-compatible (no `declare -A`).

**Files:**
- Create: `.agents/skills/spec/scripts/lessons-for.sh`
- Test: `tests/spec/run.sh` (append lessons-for tests)

- [ ] **Step 1: Write the failing tests**

Append to `tests/spec/run.sh` before the final `echo "=== results..."` line:

```bash
echo ""
echo "=== spec-skill-improvements ssi/6: lessons-for.sh ==="
LESSONS_FOR="$SPEC_SKILL/scripts/lessons-for.sh"

test_lessons_for_match() {
  local d; d="$(mktmp)"
  mkdir -p "$d/.spec"
  cat > "$d/.spec/lessons.md" <<'EOF'
# Lessons

### Test lesson one
**Pattern:** x
**Rule:** y
**Tags:** foo, bar
**Date:** 2026-01-01

### Unrelated lesson
**Pattern:** a
**Rule:** b
**Tags:** other
**Date:** 2026-01-02
EOF
  local out
  out="$(cd "$d" && bash "$LESSONS_FOR" foo 2>&1)"
  assert_contains "ssi/6" "matching lesson returned" "$out" "Test lesson one"
  assert_not_contains "ssi/6" "non-matching lesson excluded" "$out" "Unrelated lesson"
  rm -rf "$d"
}

test_lessons_for_no_match() {
  local d; d="$(mktmp)"; mkdir -p "$d/.spec"
  printf '# Lessons\n\n### Only lesson\n**Tags:** alpha\n**Date:** 2026-01-01\n' \
    > "$d/.spec/lessons.md"
  local out rc
  out="$(cd "$d" && bash "$LESSONS_FOR" nonexistent 2>&1)"; rc=$?
  if [[ $rc -eq 0 ]]; then pass "ssi/6" "no-match exits 0"
  else fail "ssi/6" "no-match exits 0"; fi
  if [[ -z "$out" ]]; then pass "ssi/6" "no-match produces empty output"
  else fail "ssi/6" "no-match produces empty output (got: $out)"; fi
  rm -rf "$d"
}

test_lessons_for_inject_format() {
  local d; d="$(mktmp)"; mkdir -p "$d/.spec"
  printf '# Lessons\n\n### Inject lesson\n**Tags:** spec\n**Date:** 2026-01-01\n' \
    > "$d/.spec/lessons.md"
  local out
  out="$(cd "$d" && bash "$LESSONS_FOR" spec --format inject 2>&1)"
  assert_contains "ssi/6" "inject format has opening delimiter" "$out" '<!-- lessons:'
  assert_contains "ssi/6" "inject format has closing delimiter" "$out" '<!-- /lessons -->'
  rm -rf "$d"
}

test_lessons_for_match
test_lessons_for_no_match
test_lessons_for_inject_format
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
bash tests/spec/run.sh 2>&1 | grep 'ssi/6'
```

Expected: all `FAIL [ssi/6]`.

- [ ] **Step 3: Create lessons-for.sh**

Create `.agents/skills/spec/scripts/lessons-for.sh`:

```bash
#!/usr/bin/env bash
# lessons-for.sh — extract lessons matching one or more tags from lessons.md
set -euo pipefail

FORMAT="markdown"
TAGS=()
LESSONS_FILE=".spec/lessons.md"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --format) FORMAT="${2:?--format requires a value (markdown|inject|json)}"; shift 2 ;;
    markdown|inject|json) FORMAT="$1"; shift ;;
    -*) echo "Unknown option: $1" >&2; exit 1 ;;
    *) TAGS+=("$1"); shift ;;
  esac
done

if [[ ${#TAGS[@]} -eq 0 ]]; then
  echo "usage: lessons-for.sh <tag> [<tag>...] [--format markdown|inject|json]" >&2
  exit 1
fi

[[ -f "$LESSONS_FILE" ]] || exit 0

current_block=""
matched_blocks=()
in_lesson=0

while IFS= read -r line || [[ -n "$line" ]]; do
  if [[ "$line" =~ ^###\  ]]; then
    if [[ $in_lesson -eq 1 && -n "$current_block" ]]; then
      tags_line="$(printf '%s' "$current_block" | grep '^\*\*Tags:\*\*' | head -1 || true)"
      for tag in "${TAGS[@]}"; do
        if printf '%s' "$tags_line" | grep -qi "$tag"; then
          matched_blocks+=("$current_block")
          break
        fi
      done
    fi
    current_block="$line"
    in_lesson=1
  elif [[ $in_lesson -eq 1 ]]; then
    current_block="${current_block}"$'\n'"$line"
  fi
done < "$LESSONS_FILE"

if [[ $in_lesson -eq 1 && -n "$current_block" ]]; then
  tags_line="$(printf '%s' "$current_block" | grep '^\*\*Tags:\*\*' | head -1 || true)"
  for tag in "${TAGS[@]}"; do
    if printf '%s' "$tags_line" | grep -qi "$tag"; then
      matched_blocks+=("$current_block")
      break
    fi
  done
fi

[[ ${#matched_blocks[@]} -eq 0 ]] && exit 0

case "$FORMAT" in
  inject)
    echo "<!-- lessons: ${TAGS[*]} -->"
    for block in "${matched_blocks[@]}"; do printf '%s\n\n' "$block"; done
    echo "<!-- /lessons -->"
    ;;
  json)
    echo "["
    first=1
    for block in "${matched_blocks[@]}"; do
      [[ $first -eq 0 ]] && echo ","
      title="$(printf '%s' "$block" | head -1 | sed 's/^### //')"
      tags="$(printf '%s' "$block" | grep '^\*\*Tags:\*\*' | sed 's/\*\*Tags:\*\* //')"
      printf '{"title":"%s","tags":"%s"}' "$title" "$tags"
      first=0
    done
    echo "]"
    ;;
  *)
    for block in "${matched_blocks[@]}"; do printf '%s\n---\n' "$block"; done
    ;;
esac
```

- [ ] **Step 4: Make executable and shellcheck**

```bash
chmod +x .agents/skills/spec/scripts/lessons-for.sh
shellcheck .agents/skills/spec/scripts/lessons-for.sh
```

Expected: no output.

- [ ] **Step 5: Run tests to verify they pass**

```bash
bash tests/spec/run.sh 2>&1 | grep 'ssi/6'
```

Expected: all `PASS [ssi/6]`.

- [ ] **Step 6: Commit**

```bash
git add .agents/skills/spec/scripts/lessons-for.sh tests/spec/run.sh
git commit -m "feat(spec): add scripts/lessons-for.sh for tag-filtered lesson extraction"
```

---

## Task 6: scripts/scan-merges.sh (unit 10)

Read-only merge-block scanner across feature tech.md files.

**Files:**
- Create: `.agents/skills/spec/scripts/scan-merges.sh`
- Test: `tests/spec/run.sh` (append scan-merges tests)

- [ ] **Step 1: Write the failing tests**

Append to `tests/spec/run.sh` before the final `echo "=== results..."` line:

```bash
echo ""
echo "=== spec-skill-improvements ssi/10: scan-merges.sh ==="
SCAN_MERGES="$SPEC_SKILL/scripts/scan-merges.sh"

test_scan_merges_finds_blocks() {
  local d; d="$(mktmp)"
  mkdir -p "$d/.spec/features/feat"
  printf '<!-- merge -->\n## Section\ncontent\n<!-- /merge -->\n' \
    > "$d/.spec/features/feat/tech.md"
  local out rc
  out="$(cd "$d" && bash "$SCAN_MERGES" feat 2>&1)"; rc=$?
  if [[ $rc -eq 0 ]]; then pass "ssi/10" "scan exits 0 with valid blocks"
  else fail "ssi/10" "scan exits 0 with valid blocks (rc=$rc)"; fi
  assert_contains "ssi/10" "scan reports block count" "$out" "1"
  rm -rf "$d"
}

test_scan_merges_unclosed_exits_nonzero() {
  local d; d="$(mktmp)"
  mkdir -p "$d/.spec/features/bad"
  printf '<!-- merge -->\ncontent without close\n' \
    > "$d/.spec/features/bad/tech.md"
  local rc
  cd "$d" && bash "$SCAN_MERGES" bad 2>/dev/null; rc=$?; cd - >/dev/null
  if [[ $rc -ne 0 ]]; then pass "ssi/10" "unclosed marker exits non-zero"
  else fail "ssi/10" "unclosed marker exits non-zero (got 0)"; fi
  rm -rf "$d"
}

test_scan_merges_empty_feature() {
  local d; d="$(mktmp)"
  mkdir -p "$d/.spec/features/empty"
  printf '# Tech\nNo merge blocks.\n' > "$d/.spec/features/empty/tech.md"
  local out rc
  out="$(cd "$d" && bash "$SCAN_MERGES" empty 2>&1)"; rc=$?
  if [[ $rc -eq 0 ]]; then pass "ssi/10" "no-blocks exits 0"
  else fail "ssi/10" "no-blocks exits 0 (rc=$rc)"; fi
  assert_contains "ssi/10" "no-blocks message" "$out" "No merge blocks"
  rm -rf "$d"
}

test_scan_merges_finds_blocks
test_scan_merges_unclosed_exits_nonzero
test_scan_merges_empty_feature
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
bash tests/spec/run.sh 2>&1 | grep 'ssi/10'
```

Expected: all `FAIL [ssi/10]`.

- [ ] **Step 3: Create scan-merges.sh**

Create `.agents/skills/spec/scripts/scan-merges.sh`:

```bash
#!/usr/bin/env bash
# scan-merges.sh — report <!-- merge --> blocks in feature tech.md files
set -euo pipefail

FEATURE=""
FORMAT="table"
SPEC_DIR=".spec"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --format) FORMAT="${2:?--format requires a value (table|json|plain)}"; shift 2 ;;
    table|json|plain) FORMAT="$1"; shift ;;
    -*) echo "Unknown option: $1" >&2; exit 1 ;;
    *) FEATURE="$1"; shift ;;
  esac
done

if [[ -n "$FEATURE" ]]; then
  FILES=("$SPEC_DIR/features/$FEATURE/tech.md")
else
  FILES=($SPEC_DIR/features/*/tech.md 2>/dev/null) || FILES=()
fi

EXIT_CODE=0
RESULTS=()

for f in "${FILES[@]}"; do
  [[ -f "$f" ]] || continue
  feature_name="$(basename "$(dirname "$f")")"
  in_block=0
  block_start=0
  block_count=0
  preview=""
  lineno=0

  while IFS= read -r line; do
    lineno=$((lineno + 1))
    if [[ "$line" == "<!-- merge -->" ]]; then
      if [[ $in_block -eq 1 ]]; then
        echo "ERROR: nested <!-- merge --> at line $lineno in $f" >&2
        EXIT_CODE=1
        break
      fi
      in_block=1
      block_start=$lineno
      block_count=$((block_count + 1))
      preview=""
    elif [[ "$line" == "<!-- /merge -->" ]]; then
      if [[ $in_block -ne 1 ]]; then
        echo "ERROR: <!-- /merge --> without opener at line $lineno in $f" >&2
        EXIT_CODE=1
        break
      fi
      RESULTS+=("$feature_name|$f|$block_start-$lineno|${preview:0:60}")
      in_block=0
    elif [[ $in_block -eq 1 && -z "$preview" && -n "$line" ]]; then
      preview="$line"
    fi
  done < "$f"

  if [[ $in_block -eq 1 ]]; then
    echo "ERROR: unclosed <!-- merge --> (opened at line $block_start) in $f" >&2
    EXIT_CODE=1
  fi
done

[[ $EXIT_CODE -ne 0 ]] && exit 1

if [[ ${#RESULTS[@]} -eq 0 ]]; then
  echo "No merge blocks found."
  exit 0
fi

case "$FORMAT" in
  json)
    echo "["
    first=1
    for r in "${RESULTS[@]}"; do
      IFS='|' read -r feat file range pv <<< "$r"
      [[ $first -eq 0 ]] && echo ","
      printf '{"feature":"%s","file":"%s","range":"%s","preview":"%s"}' \
        "$feat" "$file" "$range" "$pv"
      first=0
    done
    echo "]"
    ;;
  plain)
    for r in "${RESULTS[@]}"; do
      IFS='|' read -r feat file range pv <<< "$r"
      echo "$file:$range: $pv"
    done
    ;;
  *)
    printf '%-24s %-42s %-12s %s\n' "FEATURE" "FILE" "LINES" "PREVIEW"
    printf '%0.s-' {1..95}; echo
    for r in "${RESULTS[@]}"; do
      IFS='|' read -r feat file range pv <<< "$r"
      printf '%-24s %-42s %-12s %s\n' "$feat" "$file" "$range" "$pv"
    done
    echo ""
    echo "Total blocks: ${#RESULTS[@]}"
    ;;
esac
```

- [ ] **Step 4: Make executable and shellcheck**

```bash
chmod +x .agents/skills/spec/scripts/scan-merges.sh
shellcheck .agents/skills/spec/scripts/scan-merges.sh
```

Expected: no output.

- [ ] **Step 5: Run tests to verify they pass**

```bash
bash tests/spec/run.sh 2>&1 | grep 'ssi/10'
```

Expected: all `PASS [ssi/10]`.

- [ ] **Step 6: Commit**

```bash
git add .agents/skills/spec/scripts/scan-merges.sh tests/spec/run.sh
git commit -m "feat(spec): add scripts/scan-merges.sh for merge-block reporting"
```

---

## Task 7: validate.sh SF13 — stale link checker (unit 7)

Extend `validate.sh` with SF13: warn when root spec files link to `features/` or `archive/` paths that don't exist. Use portable `grep -oE` (no GNU `-P`).

**Files:**
- Modify: `.agents/skills/spec/scripts/validate.sh`
- Test: `tests/spec/run.sh` (append SF13 tests)

- [ ] **Step 1: Write the failing tests**

Append to `tests/spec/run.sh` before the final `echo "=== results..."` line:

```bash
echo ""
echo "=== spec-skill-improvements ssi/7: SF13 stale feature link checker ==="

test_sf13_stale_link_warns() {
  local d; d="$(mktmp)"; mkdir -p "$d/.spec"
  cat > "$d/.spec/product.md" <<'EOF'
---
type: entrypoint
scope: product
children: []
updated: 2026-06-29
---
# P
[old feature](features/deleted-thing/product.md)
EOF
  local out
  out="$(cd "$d" && bash "$VALIDATE" 2>&1)"
  assert_contains "ssi/7" "SF13 warns on stale feature link" "$out" "SF13"
  rm -rf "$d"
}

test_sf13_valid_links_pass() {
  local d; d="$(mktmp)"; mkdir -p "$d/.spec/features/real-thing"
  cat > "$d/.spec/product.md" <<'EOF'
---
type: entrypoint
scope: product
children: []
updated: 2026-06-29
---
# P
[real feature](features/real-thing/product.md)
EOF
  touch "$d/.spec/features/real-thing/product.md"
  local out
  out="$(cd "$d" && bash "$VALIDATE" 2>&1)"
  assert_not_contains "ssi/7" "SF13 silent when link target exists" "$out" "SF13"
  rm -rf "$d"
}

test_sf13_stale_link_warns
test_sf13_valid_links_pass
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
bash tests/spec/run.sh 2>&1 | grep 'ssi/7'
```

Expected: all `FAIL [ssi/7]`.

- [ ] **Step 3: Read validate.sh to find the insertion point**

```bash
grep -n 'check_\|^}' .agents/skills/spec/scripts/validate.sh | tail -30
```

Find the last `check_*` function and the main call sequence at the bottom of the file.

- [ ] **Step 4: Add check_sf13_stale_feature_links function**

In `.agents/skills/spec/scripts/validate.sh`, add this function before the main validation call sequence:

```bash
check_sf13_stale_feature_links() {
  local root_files=(".spec/product.md" ".spec/tech.md" ".spec/design.md" ".spec/plan.md")
  for f in "${root_files[@]}"; do
    [[ -f "$f" ]] || continue
    grep -oE '\(features/[^)]+\)|\(archive/[^)]+\)' "$f" 2>/dev/null \
      | sed 's/^(//;s/)$//' \
      | while IFS= read -r target; do
          [[ -e ".spec/$target" ]] || yellow "SF13: stale link in $f → $target (not found)"
        done
  done
}
```

Then add `check_sf13_stale_feature_links` to the main call sequence after the orphaned-children check. Search for `check_sf` calls at the bottom and append there.

- [ ] **Step 5: Shellcheck validate.sh**

```bash
shellcheck .agents/skills/spec/scripts/validate.sh
```

Expected: no output.

- [ ] **Step 6: Run tests to verify they pass**

```bash
bash tests/spec/run.sh 2>&1 | grep 'ssi/7'
```

Expected: all `PASS [ssi/7]`.

- [ ] **Step 7: Commit**

```bash
git add .agents/skills/spec/scripts/validate.sh tests/spec/run.sh
git commit -m "feat(spec): add SF13 stale feature link checker to validate.sh"
```

---

## Task 8: validate.sh SF14 — scope conflict detector (unit 8)

Extend `validate.sh` with SF14: warn when two feature `product.md` Scope "Owns" tables claim the same keyword. Bash 3.2-compatible (temp file as portable map).

**Files:**
- Modify: `.agents/skills/spec/scripts/validate.sh`
- Test: `tests/spec/run.sh` (append SF14 tests)

- [ ] **Step 1: Write the failing tests**

Append to `tests/spec/run.sh` before the final `echo "=== results..."` line:

```bash
echo ""
echo "=== spec-skill-improvements ssi/8: SF14 scope conflict detector ==="

test_sf14_conflict_warns() {
  local d; d="$(mktmp)"
  mkdir -p "$d/.spec/features/feat-a" "$d/.spec/features/feat-b"
  cat > "$d/.spec/features/feat-a/product.md" <<'EOF'
---
type: feature-product
feature: feat-a
sibling: tech.md
parent: ../../product.md
updated: 2026-06-29
---
## Scope
| Owns | Does not own |
|---|---|
| state machine | adapters |
EOF
  cat > "$d/.spec/features/feat-b/product.md" <<'EOF'
---
type: feature-product
feature: feat-b
sibling: tech.md
parent: ../../product.md
updated: 2026-06-29
---
## Scope
| Owns | Does not own |
|---|---|
| state machine | hooks |
EOF
  local out
  out="$(cd "$d" && bash "$VALIDATE" 2>&1)"
  assert_contains "ssi/8" "SF14 warns on scope conflict" "$out" "SF14"
  rm -rf "$d"
}

test_sf14_no_conflict_passes() {
  local d; d="$(mktmp)"
  mkdir -p "$d/.spec/features/feat-a" "$d/.spec/features/feat-b"
  cat > "$d/.spec/features/feat-a/product.md" <<'EOF'
---
type: feature-product
feature: feat-a
sibling: tech.md
parent: ../../product.md
updated: 2026-06-29
---
## Scope
| Owns | Does not own |
|---|---|
| routing table | state machine |
EOF
  cat > "$d/.spec/features/feat-b/product.md" <<'EOF'
---
type: feature-product
feature: feat-b
sibling: tech.md
parent: ../../product.md
updated: 2026-06-29
---
## Scope
| Owns | Does not own |
|---|---|
| state machine | routing table |
EOF
  local out
  out="$(cd "$d" && bash "$VALIDATE" 2>&1)"
  assert_not_contains "ssi/8" "SF14 silent when no conflict" "$out" "SF14"
  rm -rf "$d"
}

test_sf14_conflict_warns
test_sf14_no_conflict_passes
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
bash tests/spec/run.sh 2>&1 | grep 'ssi/8'
```

Expected: all `FAIL [ssi/8]`.

- [ ] **Step 3: Add check_sf14_scope_conflicts function**

In `.agents/skills/spec/scripts/validate.sh`, add after `check_sf13_stale_feature_links`:

```bash
check_sf14_scope_conflicts() {
  local feature_dirs
  feature_dirs=(.spec/features/*/product.md)
  local seen_file
  seen_file="$(mktemp "${TMPDIR:-/tmp}/.sf14.XXXXXX")"
  for f in "${feature_dirs[@]}"; do
    [[ -f "$f" ]] || continue
    local feature
    feature="$(basename "$(dirname "$f")")"
    local in_owns=0
    while IFS= read -r line; do
      echo "$line" | grep -q "| Owns"         && in_owns=1 && continue
      echo "$line" | grep -q "| Does not own" && in_owns=0 && continue
      if [[ $in_owns -eq 1 ]] && echo "$line" | grep -qE '^\|[^|]+\|'; then
        local item
        item="$(echo "$line" | awk -F'|' '{print $2}' | tr '[:upper:]' '[:lower:]' | xargs)"
        [[ ${#item} -lt 5 || -z "$item" ]] && continue
        local existing
        existing="$(grep "^${item}	" "$seen_file" 2>/dev/null || true)"
        if [[ -n "$existing" ]]; then
          local other
          other="$(echo "$existing" | awk '{print $2}')"
          yellow "SF14: scope conflict — '${other}' and '${feature}' both own: ${item}"
        else
          printf '%s\t%s\n' "$item" "$feature" >> "$seen_file"
        fi
      fi
    done < "$f"
  done
  rm -f "$seen_file"
}
```

Add `check_sf14_scope_conflicts` to the call sequence after `check_sf13_stale_feature_links`.

- [ ] **Step 4: Shellcheck**

```bash
shellcheck .agents/skills/spec/scripts/validate.sh
```

Expected: no output.

- [ ] **Step 5: Run tests to verify they pass**

```bash
bash tests/spec/run.sh 2>&1 | grep 'ssi/8'
```

Expected: all `PASS [ssi/8]`.

- [ ] **Step 6: Commit**

```bash
git add .agents/skills/spec/scripts/validate.sh tests/spec/run.sh
git commit -m "feat(spec): add SF14 scope conflict detector to validate.sh"
```

---

## Task 9: validate.sh SF15 + SF16 (unit 16)

SF15 warns when root specs exceed 200 lines. SF16 warns when lesson entries lack `**Tags:**`.

**Files:**
- Modify: `.agents/skills/spec/scripts/validate.sh`
- Test: `tests/spec/run.sh` (append SF15/SF16 tests)

- [ ] **Step 1: Write the failing tests**

Append to `tests/spec/run.sh` before the final `echo "=== results..."` line:

```bash
echo ""
echo "=== spec-skill-improvements ssi/16: SF15 root length + SF16 lessons tags ==="

test_sf15_long_root_warns() {
  local d; d="$(mktmp)"; mkdir -p "$d/.spec"
  {
    printf '---\ntype: entrypoint\nscope: product\nchildren: []\nupdated: 2026-06-29\n---\n'
    for i in $(seq 1 100); do printf '## Section %d\nLine content.\n' "$i"; done
  } > "$d/.spec/product.md"
  local out
  out="$(cd "$d" && bash "$VALIDATE" 2>&1)"
  assert_contains "ssi/16" "SF15 warns on root spec >200 lines" "$out" "SF15"
  rm -rf "$d"
}

test_sf16_lesson_no_tags_warns() {
  local d; d="$(mktmp)"; mkdir -p "$d/.spec"
  cat > "$d/.spec/product.md" <<'EOF'
---
type: entrypoint
scope: product
children: []
updated: 2026-06-29
---
# P
EOF
  cat > "$d/.spec/lessons.md" <<'EOF'
# Lessons

### Missing tags lesson
**Pattern:** x
**Rule:** y
**Date:** 2026-06-29
EOF
  local out
  out="$(cd "$d" && bash "$VALIDATE" 2>&1)"
  assert_contains "ssi/16" "SF16 warns on tagless lesson" "$out" "SF16"
  rm -rf "$d"
}

test_sf16_lesson_with_tags_passes() {
  local d; d="$(mktmp)"; mkdir -p "$d/.spec"
  cat > "$d/.spec/product.md" <<'EOF'
---
type: entrypoint
scope: product
children: []
updated: 2026-06-29
---
# P
EOF
  cat > "$d/.spec/lessons.md" <<'EOF'
# Lessons

### Tagged lesson
**Pattern:** x
**Rule:** y
**Tags:** spec, validate
**Date:** 2026-06-29
EOF
  local out
  out="$(cd "$d" && bash "$VALIDATE" 2>&1)"
  assert_not_contains "ssi/16" "SF16 silent when tags present" "$out" "SF16"
  rm -rf "$d"
}

test_sf15_long_root_warns
test_sf16_lesson_no_tags_warns
test_sf16_lesson_with_tags_passes
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
bash tests/spec/run.sh 2>&1 | grep 'ssi/16'
```

Expected: all `FAIL [ssi/16]`.

- [ ] **Step 3: Add SF15 and SF16 functions to validate.sh**

In `.agents/skills/spec/scripts/validate.sh`, add after `check_sf14_scope_conflicts`:

```bash
check_sf15_root_spec_length() {
  local max_lines="${SPEC_ROOT_MAX_LINES:-200}"
  local root_files=(".spec/product.md" ".spec/tech.md" ".spec/design.md"
                    ".spec/plan.md" ".spec/lessons.md")
  for f in "${root_files[@]}"; do
    [[ -f "$f" ]] || continue
    local count
    count="$(wc -l < "$f")"
    if [[ $count -gt $max_lines ]]; then
      yellow "SF15: $f is $count lines (>$max_lines); consider splitting into a branch doc"
    fi
  done
}

check_sf16_lessons_tags() {
  local lessons=".spec/lessons.md"
  [[ -f "$lessons" ]] || return 0
  local current_title=""
  local has_tags=0
  while IFS= read -r line; do
    if [[ "$line" =~ ^###\  ]]; then
      if [[ -n "$current_title" && $has_tags -eq 0 ]]; then
        yellow "SF16: lesson '$current_title' has no **Tags:** line (invisible to lessons-for.sh)"
      fi
      current_title="${line#\#\#\# }"
      has_tags=0
    elif [[ "$line" =~ ^\*\*Tags:\*\* ]]; then
      has_tags=1
    fi
  done < "$lessons"
  if [[ -n "$current_title" && $has_tags -eq 0 ]]; then
    yellow "SF16: lesson '$current_title' has no **Tags:** line (invisible to lessons-for.sh)"
  fi
}
```

Add both to the call sequence: `check_sf15_root_spec_length` then `check_sf16_lessons_tags`.

- [ ] **Step 4: Shellcheck**

```bash
shellcheck .agents/skills/spec/scripts/validate.sh
```

- [ ] **Step 5: Run tests to verify they pass**

```bash
bash tests/spec/run.sh 2>&1 | grep 'ssi/16'
```

Expected: all `PASS [ssi/16]`.

- [ ] **Step 6: Run full validate**

```bash
bash .agents/skills/spec/scripts/validate.sh
```

Expected: `Errors: 0`.

- [ ] **Step 7: Commit**

```bash
git add .agents/skills/spec/scripts/validate.sh tests/spec/run.sh
git commit -m "feat(spec): add SF15 root length + SF16 lessons tags checks to validate.sh"
```

---

## Task 10: OpenSpec machine-readable frontmatter docs (unit 11)

Append `## OpenSpec frontmatter (optional)` sections to `reference/product.md` and `reference/plan.md`.

**Files:**
- Modify: `.agents/skills/spec/reference/product.md`
- Modify: `.agents/skills/spec/reference/plan.md`
- Test: `tests/spec/run.sh` (append `test_openspec_docs_present`)

- [ ] **Step 1: Write the failing test**

Append to `tests/spec/run.sh` before the final `echo "=== results..."` line:

```bash
echo ""
echo "=== spec-skill-improvements ssi/11: OpenSpec docs ==="

test_openspec_docs_present() {
  local rp; rp="$(cat "$SPEC_SKILL/reference/product.md")"
  assert_contains "ssi/11" "product.md has requirements: docs" "$rp" 'requirements:'
  assert_contains "ssi/11" "product.md marks opt-in" "$rp" 'opt-in'
  local rplan; rplan="$(cat "$SPEC_SKILL/reference/plan.md")"
  assert_contains "ssi/11" "plan.md has units: docs" "$rplan" 'units:'
}

test_openspec_docs_present
```

- [ ] **Step 2: Run test to verify it fails**

```bash
bash tests/spec/run.sh 2>&1 | grep 'ssi/11'
```

- [ ] **Step 3: Append to reference/product.md**

Read `.agents/skills/spec/reference/product.md` first to find the end. Append:

````markdown
## OpenSpec frontmatter (optional)

Machine-readable requirement metadata. Opt-in per file — specs without it pass
validation unchanged.

```yaml
requirements:
  - id: R-1
    title: "Login with email"
    strength: SHALL
    scenarios: 2
  - id: R-2
    title: "Password reset flow"
    strength: SHALL
    scenarios: 1
```

Fields: `id` (R-N), `title` (short name matching `### Requirement:` heading),
`strength` (SHALL/MUST/SHOULD/MAY), `scenarios` (expected count of `#### Scenario:` blocks).

`validate.sh` does not require these fields. When present, a future opt-in check
may warn when `scenarios:` count mismatches actual GWT blocks in the file body.
````

- [ ] **Step 4: Append to reference/plan.md**

Read `.agents/skills/spec/reference/plan.md` first. Append:

````markdown
## OpenSpec frontmatter (optional)

Machine-readable unit metadata. Opt-in per file — plans without it pass
validation unchanged.

```yaml
units:
  - id: "spec-skill-improvements/1"
    title: "SKILL.md metadata enrichment"
    status: planned
    requires: []
  - id: "spec-skill-improvements/2"
    title: "Subagent role profiles"
    status: planned
    requires: ["spec-skill-improvements/1"]
```

Fields: `id` (`<name>/n`), `title`, `status` (planned/in-progress/done),
`requires` (list of upstream unit IDs — same-feature only).

Allows tooling to compute dependency graphs without parsing markdown tables.
`validate.sh` does not require these fields.
````

- [ ] **Step 5: Run test to verify it passes**

```bash
bash tests/spec/run.sh 2>&1 | grep 'ssi/11'
```

Expected: all `PASS [ssi/11]`.

- [ ] **Step 6: Commit**

```bash
git add .agents/skills/spec/reference/product.md .agents/skills/spec/reference/plan.md \
        tests/spec/run.sh
git commit -m "feat(spec): add OpenSpec optional requirements:/units: frontmatter docs"
```

---

## Task 11: SKILL.md `## Roles` section (unit 2)

Add `## Roles` manifest section before `## Routing` in SKILL.md body.

**Files:**
- Modify: `.agents/skills/spec/SKILL.md`
- Test: `tests/spec/run.sh` (append `test_skill_roles_section`)

**Prerequisite:** Task 1 complete.

- [ ] **Step 1: Write the failing test**

Append to `tests/spec/run.sh` before the final `echo "=== results..."` line:

```bash
echo ""
echo "=== spec-skill-improvements ssi/2: ## Roles section ==="

test_skill_roles_section() {
  local skill; skill="$(cat "$SPEC_SKILL/SKILL.md")"
  assert_contains "ssi/2" "## Roles section present" "$skill" '## Roles'
  assert_contains "ssi/2" "spec-interviewer role" "$skill" 'spec-interviewer'
  assert_contains "ssi/2" "spec-architect role" "$skill" 'spec-architect'
  assert_contains "ssi/2" "spec-planner role" "$skill" 'spec-planner'
  assert_contains "ssi/2" "spec-auditor role" "$skill" 'spec-auditor'
  assert_contains "ssi/2" "spec-compactor role" "$skill" 'spec-compactor'
}

test_skill_roles_section
```

- [ ] **Step 2: Run test to verify it fails**

```bash
bash tests/spec/run.sh 2>&1 | grep 'ssi/2'
```

- [ ] **Step 3: Find insertion point**

```bash
grep -n '## Routing' .agents/skills/spec/SKILL.md
```

Note the line number. Insert the `## Roles` block immediately before that line.

- [ ] **Step 4: Insert ## Roles section before ## Routing**

In `.agents/skills/spec/SKILL.md`, insert this block immediately before the `## Routing` line:

```markdown
## Roles

Five composable delegation contexts. Invoke via `/spec <role>` or by
phase-routing below. Each role names an executor and a constraint document to
inject — roles are NOT custom tools; they are wiring between spec constraints
and the appropriate executor.

### Role: spec-interviewer

**Executor:** `superpowers:brainstorming`
**Phase:** feature.design steps 1–2 (WHAT interview)
**Constraint document:** `feature.md § Interview for WHAT`
**Inputs:** root product.md + tech.md + lessons.md
**Outputs:** `features/<name>/product.md` draft (human reviews before writing)
**Validation criteria:** RFC-2119 keyword in every requirement; ≥1 GWT scenario
per requirement; Scope table has Owns + Does not own columns
**Self-execute fallback:** 5-question dialogue (problem → scope → scenario →
failure modes → done signal) using the constraint document as guide

> Proactive offer: "I can use `superpowers:brainstorming` for this — want me to?"
> Self-execute if declined or unavailable.

### Role: spec-architect

**Executor:** `code-explorer` (trace) + `code-architect` (sketch)
**Phase:** feature.design steps 3–4 (HOW sketch)
**Constraint document:** `reference/tech.md` + feature-tech template
**Inputs:** root tech.md + feature product.md + codebase
**Outputs:** `features/<name>/tech.md` (+ design.md if full)
**Validation criteria:** all cited paths exist in repo; no UX opinions in tech.md
**Self-execute fallback:** read codebase with Read/Glob/Grep; sketch HOW section
directly using reference/tech.md as format constraint

> Proactive offer: "I can use `code-explorer` + `code-architect` for this — want me to?"
> Self-execute if declined or unavailable.

### Role: spec-planner

**Executor:** `superpowers:writing-plans`
**Phase:** feature.plan step 5 (plan units)
**Constraint document:** `reference/plan.md` — stable-ID convention, Trace table format
**Inputs:** feature product.md + tech.md
**Outputs:** `features/<name>/plan.md`
**Validation criteria:** all unit IDs follow `<name>/n`; each unit cites ≥1 R-ID;
verification column is non-empty per unit

> Proactive offer: "I can use `superpowers:writing-plans` for this — want me to?"
> Self-execute if declined or unavailable.

### Role: spec-auditor

**Executor:** `validate.sh` (spec-unique; no superpower substitute)
**Phase:** any (standalone via `/spec audit`)
**Inputs:** `.spec/` tree
**Outputs:** structured validation report; optionally passes output to
`superpowers:verification-before-completion` for quality framing
**Procedure:** `bash .agents/skills/spec/scripts/validate.sh` → surface warnings

### Role: spec-compactor

**Executor:** `scripts/promote.sh` (extraction) + `superpowers:finishing-a-development-branch` (lesson narrative)
**Phase:** feature.compound
**Constraint document:** `strategy.md § Lessons` format (pattern, rule, tags, date)
**Inputs:** `features/<name>/tech.md` merge markers; root tech.md; lessons.md
**Outputs:** root tech.md (promoted blocks); draft lesson for human review; plan.md DONE row
**Invariant:** lesson reviewed by human before appending; never auto-committed

```

- [ ] **Step 5: Run test to verify it passes**

```bash
bash tests/spec/run.sh 2>&1 | grep 'ssi/2'
```

Expected: all `PASS [ssi/2]`.

- [ ] **Step 6: Commit**

```bash
git add .agents/skills/spec/SKILL.md tests/spec/run.sh
git commit -m "feat(spec): add ## Roles delegation manifest to SKILL.md"
```

---

## Task 12: SKILL.md routing expansion (unit 3)

Extend the `## Routing` table with new routes.

**Files:**
- Modify: `.agents/skills/spec/SKILL.md`
- Test: `tests/spec/run.sh` (append `test_skill_routing_new_routes`)

**Prerequisite:** Task 11 complete.

- [ ] **Step 1: Write the failing test**

Append to `tests/spec/run.sh` before the final `echo "=== results..."` line:

```bash
echo ""
echo "=== spec-skill-improvements ssi/3: routing expansion ==="

test_skill_routing_new_routes() {
  local skill; skill="$(cat "$SPEC_SKILL/SKILL.md")"
  assert_contains "ssi/3" "interview route" "$skill" '`interview'
  assert_contains "ssi/3" "promote route" "$skill" '`promote'
  assert_contains "ssi/3" "audit route" "$skill" '`audit`'
  assert_contains "ssi/3" "diff route" "$skill" '`diff'
  assert_contains "ssi/3" "health route" "$skill" '`health`'
  assert_contains "ssi/3" "research route" "$skill" '`research'
  assert_contains "ssi/3" "lessons-for route" "$skill" 'lessons-for'
}

test_skill_routing_new_routes
```

- [ ] **Step 2: Run test to verify it fails**

```bash
bash tests/spec/run.sh 2>&1 | grep 'ssi/3'
```

- [ ] **Step 3: Find the Routing table in SKILL.md**

```bash
grep -n '## Routing\|validate\|_(none)_' .agents/skills/spec/SKILL.md | head -20
```

Find the last row in the `$ARGUMENTS` routing table (before the `_(none)_` row or the end of the table).

- [ ] **Step 4: Add new rows to ## Routing table**

In the `## Routing` section `**$ARGUMENTS:**` table, add these rows after the existing `validate` row (before `_(none)_`):

```markdown
| `interview [<name>]` | Load spec-interviewer role; run WHAT steps 1–2 for `<name>` |
| `promote <name>` | Run `scripts/promote.sh <name>` via spec-promoter agent |
| `audit` | Load spec-auditor role; run `validate.sh` |
| `diff <name>` | Run `scripts/scan-merges.sh <name>` — show pending merge blocks |
| `health` | Invoke spec-health agent — structural assessment of `.spec/` tree |
| `research <name>` | Open `.spec/features/<name>/research.md`; suggest spec-tracer |
| `lessons-for <tag>` | Run `scripts/lessons-for.sh <tag>` — tag-filtered lesson output |
```

The `argument-hint` frontmatter was already updated in Task 1.

- [ ] **Step 5: Run test to verify it passes**

```bash
bash tests/spec/run.sh 2>&1 | grep 'ssi/3'
```

- [ ] **Step 6: Commit**

```bash
git add .agents/skills/spec/SKILL.md tests/spec/run.sh
git commit -m "feat(spec): expand ## Routing table with interview/promote/audit/diff/health/research/lessons-for"
```

---

## Task 13: agents/spec-tracer/SKILL.md (unit 12)

Read-only codebase tracer subagent. `parallel-safe: true`; only Read/Glob/Grep in `allowed-tools`.

**Files:**
- Create: `.agents/skills/spec/agents/spec-tracer/SKILL.md`
- Test: `tests/spec/run.sh` (append `test_spec_tracer_read_only`)

- [ ] **Step 1: Write the failing test**

Append to `tests/spec/run.sh` before the final `echo "=== results..."` line:

```bash
echo ""
echo "=== spec-skill-improvements ssi/12: spec-tracer SKILL.md ==="

test_spec_tracer_read_only() {
  local f="$SPEC_SKILL/agents/spec-tracer/SKILL.md"
  if [[ -f "$f" ]]; then pass "ssi/12" "spec-tracer/SKILL.md exists"
  else fail "ssi/12" "spec-tracer/SKILL.md exists"; return; fi
  local body; body="$(cat "$f")"
  assert_contains "ssi/12" "has allowed-tools" "$body" 'allowed-tools'
  assert_contains "ssi/12" "lists Read" "$body" 'Read'
  assert_not_contains "ssi/12" "no Edit in allowed-tools" "$body" '- Edit'
  assert_not_contains "ssi/12" "no Write in allowed-tools" "$body" '- Write'
  assert_contains "ssi/12" "parallel-safe declared" "$body" 'parallel-safe'
}

test_spec_tracer_read_only
```

- [ ] **Step 2: Run test to verify it fails**

```bash
bash tests/spec/run.sh 2>&1 | grep 'ssi/12'
```

- [ ] **Step 3: Create the directory and SKILL.md**

```bash
mkdir -p .agents/skills/spec/agents/spec-tracer
```

Create `.agents/skills/spec/agents/spec-tracer/SKILL.md`:

```markdown
---
name: spec-tracer
description: Read-only codebase tracer for spec HOW phase — maps existing files, interfaces, and contracts relevant to a feature requirement set
user-invocable: false
parallel-safe: true
allowed-tools:
  - Read
  - Glob
  - Grep
---

# spec-tracer

Read-only codebase tracer for feature.design step 4 (HOW phase). Safe to run
concurrently with spec-interviewer because it only reads — never writes.

## Input contract

Requires from parent context:
- Feature name
- Feature `product.md` requirements (or a summary of what the feature does)

## Orders

Run in this sequence — each step feeds the next:

1. **Read context** — read root `.spec/tech.md` for cross-cutting contracts
   that apply to this feature's domain

2. **Glob for relevant files** — search the repo for files whose names or paths
   match keywords from the feature requirements; start specific, widen only if
   needed

3. **Grep for interface signatures** — find function/type/class/variable
   declarations relevant to requirements; note file path and line number

4. **Map gaps** — for each requirement, assess: does an existing code path
   satisfy it, or is new code needed?

5. **Produce trace document** (stdout; parent skill injects into spec-architect)

## Output format

```markdown
## Codebase Trace: <feature-name>

### Relevant existing files
| File | Purpose |
|---|---|
| `path/to/file.sh` | … |

### Interface signatures
| Symbol | File:line | Notes |
|---|---|---|
| `function_name` | `path/to/file.sh:42` | … |

### Root tech.md cross-references
- Section "…" applies because …

### Gaps (new code required)
- Requirement "…" — no matching code path found
```

Output only — do not write to any file.
```

- [ ] **Step 4: Run test to verify it passes**

```bash
bash tests/spec/run.sh 2>&1 | grep 'ssi/12'
```

- [ ] **Step 5: Commit**

```bash
git add .agents/skills/spec/agents/spec-tracer/SKILL.md tests/spec/run.sh
git commit -m "feat(spec): add agents/spec-tracer/SKILL.md — read-only parallel codebase tracer"
```

---

## Task 14: agents/spec-promoter/SKILL.md (unit 13)

Diff-first merge promoter agent. MUST dry-run first; MUST confirm before live write.

**Files:**
- Create: `.agents/skills/spec/agents/spec-promoter/SKILL.md`
- Test: `tests/spec/run.sh` (append `test_spec_promoter_diff_first`)

**Prerequisite:** Task 4 (promote.sh) complete.

- [ ] **Step 1: Write the failing test**

Append to `tests/spec/run.sh` before the final `echo "=== results..."` line:

```bash
echo ""
echo "=== spec-skill-improvements ssi/13: spec-promoter SKILL.md ==="

test_spec_promoter_diff_first() {
  local f="$SPEC_SKILL/agents/spec-promoter/SKILL.md"
  if [[ -f "$f" ]]; then pass "ssi/13" "spec-promoter/SKILL.md exists"
  else fail "ssi/13" "spec-promoter/SKILL.md exists"; return; fi
  local body; body="$(cat "$f")"
  assert_contains "ssi/13" "dry-run mentioned" "$body" 'dry-run'
  assert_contains "ssi/13" "confirm step present" "$body" 'confirm'
  assert_contains "ssi/13" "references promote.sh" "$body" 'promote.sh'
}

test_spec_promoter_diff_first
```

- [ ] **Step 2: Run test to verify it fails**

```bash
bash tests/spec/run.sh 2>&1 | grep 'ssi/13'
```

- [ ] **Step 3: Create the directory and SKILL.md**

```bash
mkdir -p .agents/skills/spec/agents/spec-promoter
```

Create `.agents/skills/spec/agents/spec-promoter/SKILL.md`:

```markdown
---
name: spec-promoter
description: Compound merge-marker extractor — shows scan diff first, executes promote.sh on explicit confirmation
user-invocable: false
allowed-tools:
  - Read
  - Bash
---

# spec-promoter

Extracts `<!-- merge -->` blocks from a feature's `tech.md` and promotes them
to root `.spec/tech.md`. Always shows a scan preview before any write.

## Orders

1. **Scan** — run the read-only scan and show the output:
   ```bash
   bash .agents/skills/spec/scripts/scan-merges.sh <feature-name>
   ```
   If scan exits non-zero (unclosed marker): stop here, report the error,
   do not proceed to confirmation.

2. **Confirm** — ask the user:
   > "These blocks will be promoted to `.spec/tech.md`. Confirm? (yes/no)"

3. **Execute** (only on explicit yes/y/confirm/proceed):
   ```bash
   bash .agents/skills/spec/scripts/promote.sh <feature-name>
   ```
   Report: "N block(s) promoted to .spec/tech.md".

4. **Decline** (any other answer): "Skipped promotion — no files changed."

## Safety invariants

- MUST show scan output before asking
- MUST NOT run promote.sh without explicit confirmation
- MUST report what was done or skipped
- Dry-run is implicit via scan-merges.sh; promote.sh is only run live
```

- [ ] **Step 4: Run test to verify it passes**

```bash
bash tests/spec/run.sh 2>&1 | grep 'ssi/13'
```

- [ ] **Step 5: Commit**

```bash
git add .agents/skills/spec/agents/spec-promoter/SKILL.md tests/spec/run.sh
git commit -m "feat(spec): add agents/spec-promoter/SKILL.md — diff-first merge promoter"
```

---

## Task 15: agents/spec-interviewer/SKILL.md (unit 14)

WHAT-phase interview delegator. Injects `feature.md § Interview for WHAT` as constraint; offers `superpowers:brainstorming`; self-executes 5-question dialogue if declined.

**Files:**
- Create: `.agents/skills/spec/agents/spec-interviewer/SKILL.md`
- Test: `tests/spec/run.sh` (append `test_spec_interviewer_constraint_injection`)

**Prerequisites:** Task 1 (frontmatter), Task 11 (## Roles section).

- [ ] **Step 1: Write the failing test**

Append to `tests/spec/run.sh` before the final `echo "=== results..."` line:

```bash
echo ""
echo "=== spec-skill-improvements ssi/14: spec-interviewer SKILL.md ==="

test_spec_interviewer_constraint_injection() {
  local f="$SPEC_SKILL/agents/spec-interviewer/SKILL.md"
  if [[ -f "$f" ]]; then pass "ssi/14" "spec-interviewer/SKILL.md exists"
  else fail "ssi/14" "spec-interviewer/SKILL.md exists"; return; fi
  local body; body="$(cat "$f")"
  assert_contains "ssi/14" "references Interview for WHAT constraint" "$body" 'Interview for WHAT'
  assert_contains "ssi/14" "mentions superpowers:brainstorming" "$body" 'superpowers:brainstorming'
  assert_contains "ssi/14" "has 5-question self-execute fallback" "$body" '5-question'
}

test_spec_interviewer_constraint_injection
```

- [ ] **Step 2: Run test to verify it fails**

```bash
bash tests/spec/run.sh 2>&1 | grep 'ssi/14'
```

- [ ] **Step 3: Create the directory and SKILL.md**

```bash
mkdir -p .agents/skills/spec/agents/spec-interviewer
```

Create `.agents/skills/spec/agents/spec-interviewer/SKILL.md`:

```markdown
---
name: spec-interviewer
description: WHAT-phase interview delegator — injects feature.md constraints, offers superpowers:brainstorming, self-executes 5-question dialogue if declined
user-invocable: false
delegates-to: superpowers:brainstorming
allowed-tools:
  - Read
---

# spec-interviewer

Conducts the WHAT-phase dialogue for a new feature. Offers to delegate to
`superpowers:brainstorming` before running the interview directly.

## Orders

1. Read `.agents/skills/spec/feature.md` § Interview for WHAT (the constraint
   document defining RFC-2119 vocabulary, Scope table format, GWT structure)
2. Read `.spec/product.md`, `.spec/tech.md`, `.spec/lessons.md`
3. Offer delegation:
   > "I can run this WHAT interview as a `superpowers:brainstorming` session
   > with spec format constraints pre-loaded — RFC-2119 keywords, GWT scenarios,
   > Scope table boundaries. That gives you a structured dialogue rather than a
   > form fill. Want me to use that?"
4. On yes: invoke `superpowers:brainstorming` with the constraint context from step 1
5. On no (self-execute fallback): run the 5-question dialogue directly

## Self-execute fallback (5-question dialogue)

Ask these in order; wait for answer before next:

**Q1 — Problem:** "What problem does this feature solve? Who has it, and when
does it hurt them?"

**Q2 — Scope:** "What does this feature own — what decision is uniquely its to
make? And what explicitly does it NOT own (hand off to another feature)?"

**Q3 — Scenario:** "Walk me through the core scenario: what does the user/system
do, and what does the feature do in response? Give me a concrete example."

**Q4 — Failure modes:** "What can go wrong? What should the feature do when it
can't complete the happy path?"

**Q5 — Done signal:** "How will you know this feature is done? What's the
observable output that proves it works?"

After Q5: synthesise answers into a product.md draft using `feature.md § Interview
for WHAT` format. Show draft; ask for approval before writing the file.
```

- [ ] **Step 4: Run test to verify it passes**

```bash
bash tests/spec/run.sh 2>&1 | grep 'ssi/14'
```

- [ ] **Step 5: Commit**

```bash
git add .agents/skills/spec/agents/spec-interviewer/SKILL.md tests/spec/run.sh
git commit -m "feat(spec): add agents/spec-interviewer/SKILL.md — WHAT-phase interview delegator"
```

---

## Task 16: agents/spec-health/SKILL.md (unit 15)

Structural health assessor. Runs validate.sh + deeper structural checks; outputs CRITICAL/WARN/INFO list.

**Files:**
- Create: `.agents/skills/spec/agents/spec-health/SKILL.md`
- Test: `tests/spec/run.sh` (append `test_spec_health_output_levels`)

**Prerequisites:** Tasks 7, 8, 9 (SF13–SF16 in validate.sh).

- [ ] **Step 1: Write the failing test**

Append to `tests/spec/run.sh` before the final `echo "=== results..."` line:

```bash
echo ""
echo "=== spec-skill-improvements ssi/15: spec-health SKILL.md ==="

test_spec_health_output_levels() {
  local f="$SPEC_SKILL/agents/spec-health/SKILL.md"
  if [[ -f "$f" ]]; then pass "ssi/15" "spec-health/SKILL.md exists"
  else fail "ssi/15" "spec-health/SKILL.md exists"; return; fi
  local body; body="$(cat "$f")"
  assert_contains "ssi/15" "validate.sh referenced" "$body" 'validate.sh'
  assert_contains "ssi/15" "CRITICAL level defined" "$body" 'CRITICAL'
  assert_contains "ssi/15" "WARN level defined" "$body" 'WARN'
  assert_contains "ssi/15" "INFO level defined" "$body" 'INFO'
}

test_spec_health_output_levels
```

- [ ] **Step 2: Run test to verify it fails**

```bash
bash tests/spec/run.sh 2>&1 | grep 'ssi/15'
```

- [ ] **Step 3: Create the directory and SKILL.md**

```bash
mkdir -p .agents/skills/spec/agents/spec-health
```

Create `.agents/skills/spec/agents/spec-health/SKILL.md`:

```markdown
---
name: spec-health
description: Structural health assessor for .spec/ tree — runs validate.sh plus deeper structural checks; outputs prioritised CRITICAL/WARN/INFO list
user-invocable: true
allowed-tools:
  - Read
  - Glob
  - Grep
  - Bash
---

# spec-health

Structural health assessor. Runs `validate.sh` then performs deeper checks
that require reading multiple files for cross-cutting issues validate.sh misses.

## Orders

Run in this exact sequence:

1. **Baseline** — run and capture validate.sh output:
   ```bash
   bash .agents/skills/spec/scripts/validate.sh 2>&1
   ```
   Any ERROR from validate.sh → CRITICAL in output. Any WARN → WARN in output.

2. **Plan alignment** — read `.spec/plan.md` Feature Sequence section; glob
   `.spec/features/*/`; for each feature folder not in Feature Sequence, add
   WARN: "feature folder has no plan.md entry".

3. **Unit IDs** — for each `.spec/features/*/plan.md` that exists, grep for
   `| spec-skill-improvements/` or any `<name>/n` pattern; if no unit IDs found,
   add WARN: "plan.md has no unit IDs (lessons-for and traceability won't work)".

4. **Root spec size** — for each root spec (product.md, tech.md, plan.md,
   lessons.md): if over 300 lines, add WARN: "root spec over 300 lines — high
   risk of feature-level detail accumulating".

5. **Merge blocks** — run `scan-merges.sh` with no args; if any blocks found,
   add INFO: "N pending merge blocks across features — run `/spec promote` to
   promote them".

6. **Produce health report**

## Output format

```markdown
## Spec Health Report — YYYY-MM-DD

### CRITICAL (must fix before merging)
- [item] — [reason]

### WARN (should fix; may cause tool failures)
- [item] — [reason]

### INFO (optional improvements)
- [item] — [reason]

### Summary
Errors: N  Warnings: N  Info: N
Overall: [HEALTHY / NEEDS ATTENTION / CRITICAL]
```

If no issues found in any category, omit that section and write
`### Summary\nAll checks passed. Spec tree is healthy.`
```

- [ ] **Step 4: Run test to verify it passes**

```bash
bash tests/spec/run.sh 2>&1 | grep 'ssi/15'
```

- [ ] **Step 5: Commit**

```bash
git add .agents/skills/spec/agents/spec-health/SKILL.md tests/spec/run.sh
git commit -m "feat(spec): add agents/spec-health/SKILL.md — structural health assessor"
```

---

## Task 17: Composable agents/ folder wiring (unit 17)

Integration verification: confirm all four subagent paths in SKILL.md frontmatter map to existing files.

**Files:**
- Test: `tests/spec/run.sh` (append `test_subagents_folder_wiring`)

**Prerequisites:** Tasks 1, 12, 13, 14, 15, 16 all complete.

- [ ] **Step 1: Write the test**

Append to `tests/spec/run.sh` before the final `echo "=== results..."` line:

```bash
echo ""
echo "=== spec-skill-improvements ssi/17: composable agents wiring ==="

test_subagents_folder_wiring() {
  local skill; skill="$(cat "$SPEC_SKILL/SKILL.md")"
  assert_contains "ssi/17" "agents: map in frontmatter" "$skill" 'agents:'
  assert_contains "ssi/17" "spec-tracer entry" "$skill" 'spec-tracer'
  assert_contains "ssi/17" "spec-promoter entry" "$skill" 'spec-promoter'
  assert_contains "ssi/17" "spec-interviewer entry" "$skill" 'spec-interviewer'
  assert_contains "ssi/17" "spec-health entry" "$skill" 'spec-health'
  for agent in spec-tracer spec-promoter spec-interviewer spec-health; do
    local f="$SPEC_SKILL/agents/$agent/SKILL.md"
    if [[ -f "$f" ]]; then pass "ssi/17" "$agent/SKILL.md exists on disk"
    else fail "ssi/17" "$agent/SKILL.md exists on disk"; fi
  done
}

test_subagents_folder_wiring
```

- [ ] **Step 2: Run test**

```bash
bash tests/spec/run.sh 2>&1 | grep 'ssi/17'
```

Expected: all `PASS [ssi/17]` (all prerequisites complete).

- [ ] **Step 3: Run full validate**

```bash
bash .agents/skills/spec/scripts/validate.sh
```

Expected: `Errors: 0`.

- [ ] **Step 4: Commit**

```bash
git add tests/spec/run.sh
git commit -m "feat(spec): add ssi/17 wiring integration test for composable agents/ folder"
```

---

## Task 18: Interactive setup interview + `.spec/.config.yaml` (unit 18)

Replace `## Setup` section in SKILL.md with 4-question interview flow; add `## Config` section.

**Files:**
- Modify: `.agents/skills/spec/SKILL.md`
- Test: `tests/spec/run.sh` (append setup interview tests)

**Prerequisite:** Task 1 (v2.0 frontmatter) complete.

- [ ] **Step 1: Write the failing tests**

Append to `tests/spec/run.sh` before the final `echo "=== results..."` line:

```bash
echo ""
echo "=== spec-skill-improvements ssi/18: setup interview + config ==="

test_setup_section_has_interview_flow() {
  local skill; skill="$(cat "$SPEC_SKILL/SKILL.md")"
  assert_contains "ssi/18" "## Config section present" "$skill" '## Config'
  assert_contains "ssi/18" ".config.yaml referenced" "$skill" '.config.yaml'
  assert_contains "ssi/18" "Q1 question present" "$skill" 'Q1'
  assert_contains "ssi/18" "Q2 question present" "$skill" 'Q2'
  assert_contains "ssi/18" "Q3 question present" "$skill" 'Q3'
  assert_contains "ssi/18" "Q4 question present" "$skill" 'Q4'
  assert_contains "ssi/18" "suggest-superpowers key documented" "$skill" 'suggest-superpowers'
}

test_setup_section_has_interview_flow
```

- [ ] **Step 2: Run test to verify it fails**

```bash
bash tests/spec/run.sh 2>&1 | grep 'ssi/18'
```

- [ ] **Step 3: Find the ## Setup section in SKILL.md**

```bash
grep -n '## Setup\|## Config' .agents/skills/spec/SKILL.md
```

Note the line range of the existing `## Setup` section. You will replace it.

- [ ] **Step 4: Add ## Config section before ## Setup**

In `.agents/skills/spec/SKILL.md`, find the `## Setup` heading and insert this `## Config` section immediately before it:

```markdown
## Config

At session start, check for `.spec/.config.yaml`. If present, load and apply
adjustments silently before any delegation or authoring step.

**Config file format:**

```yaml
# .spec/.config.yaml — generated by /spec setup
vibe-flow: true             # bool — is vibe-flow feature-dev in use?
caveman: auto               # auto|lite|full — static default (ignored when vibe-flow: true)
superpowers:
  brainstorming: true       # superpowers:brainstorming available?
  writing-plans: true       # superpowers:writing-plans available?
  code-explorer: true       # code-explorer available?
  code-architect: true      # code-architect available?
  verification: true        # superpowers:verification-before-completion available?
  finishing: true           # superpowers:finishing-a-development-branch available?
suggest-superpowers: true   # bool — show proactive executor tips at each step?
setup-date: YYYY-MM-DD
```

**Defaults when file absent or field missing:**
- `vibe-flow: false` — assume manual invocation
- `caveman: full` — all sections, no compression
- all `superpowers.*: true` — assume available; user will decline when not
- `suggest-superpowers: true` — proactive tips on by default

**Behavior adjustments per key:**
- `suggest-superpowers: false` → suppress all "Superpower tip" callouts; self-execute every step without offering
- `vibe-flow: true` → note that caveman level is auto-managed by flow cursor; don't ask
- `caveman: lite` → apply lite output profile by default; tell the user once
- `superpowers.brainstorming: false` → skip brainstorming offer in spec-interviewer; go straight to self-execute

```

- [ ] **Step 5: Replace ## Setup section content**

In `.agents/skills/spec/SKILL.md`, replace the existing `## Setup` section body (keep the `## Setup` heading) with:

```markdown
## Setup

When the user runs `/spec setup`:

1. **Check for `.spec/.config.yaml`:**
   - If present: show current settings; ask "Which setting would you like
     to update?" → update only the named setting; re-confirm; write; done.
   - If absent: run the full interview below.

2. **Interview (4 questions — conversational, not a form):**

   **Q1 — Workflow orchestration:**
   "Are you using vibe-flow's feature dev skills (vibe-feature, vibe-compound,
   etc.) to manage your workflow, or will you run /spec commands directly?
   With vibe-flow, caveman level and phase routing are handled by the flow
   cursor. Without it, you drive everything manually."
   → Answer: yes (vibe-flow) | no (manual)

   **Q2 — Caveman level** (skip if Q1 = vibe-flow):
   "What level of spec detail do you want by default?
     lite — scope table + requirement titles + unit IDs only (fast, low noise)
     full — all sections per template (recommended for most projects)
     auto — match to current phase when detectable, fall back to full"
   → Answer: lite | full | auto

   **Q3 — Available superpowers:**
   "Which AI skills/superpowers are available in your environment?
     all   — brainstorming, writing-plans, code-explorer, code-architect,
             verification, finishing (I'll offer all of them)
     none  — self-sufficient mode; I'll execute every step directly
     custom — I'll list them; tell me which ones you have"
   → If custom: list each, ask yes/no per superpower
   → Answer: all | none | {per-superpower booleans}

   **Q4 — Proactive suggestions:**
   "Should I suggest superpowers at each authoring step — 'I can use X for
   this, want me to?' — or stay quiet and just execute?
   Recommended: yes. You can always decline."
   → Answer: yes | no

3. **Show summary and confirm:**
   "Here's what I'll configure:
     Workflow: vibe-flow / manual
     Caveman:  [level] / auto-managed
     Superpowers: all / none / [list]
     Suggest tips: yes / no
   Confirm? (yes to write, no to re-ask)"

4. **On confirm:** write `.spec/.config.yaml`; then run:
   ```bash
   bash .agents/skills/spec/scripts/setup.sh
   ```
   Report what was created.

5. **After setup, surface next steps:**
   "Your .spec/ is ready. Suggested next step:
     /spec strategy   — start writing root product.md + tech.md
     /spec feature <name>  — scope your first named feature"
```

- [ ] **Step 6: Run tests to verify they pass**

```bash
bash tests/spec/run.sh 2>&1 | grep 'ssi/18'
```

Expected: all `PASS [ssi/18]`.

- [ ] **Step 7: Run full validate**

```bash
bash .agents/skills/spec/scripts/validate.sh
```

Expected: `Errors: 0`.

- [ ] **Step 8: Commit**

```bash
git add .agents/skills/spec/SKILL.md tests/spec/run.sh
git commit -m "feat(spec): add ## Config section and 4-question setup interview to SKILL.md"
```

---

## Task 19: Config read at session start (unit 19)

Add config-read step to the session-start flow in SKILL.md preamble. Propagate `suggest-superpowers` suppression note to feature.md.

**Files:**
- Modify: `.agents/skills/spec/SKILL.md`
- Modify: `.agents/skills/spec/feature.md`
- Test: `tests/spec/run.sh` (append config session-start tests)

**Prerequisite:** Task 18 complete.

- [ ] **Step 1: Write the failing tests**

Append to `tests/spec/run.sh` before the final `echo "=== results..."` line:

```bash
echo ""
echo "=== spec-skill-improvements ssi/19: config read at session start ==="

test_config_section_present() {
  local skill; skill="$(cat "$SPEC_SKILL/SKILL.md")"
  assert_contains "ssi/19" "## Config section present" "$skill" '## Config'
  assert_contains "ssi/19" "defaults documented" "$skill" 'defaults'
  assert_contains "ssi/19" "suggest-superpowers key present" "$skill" 'suggest-superpowers'
  assert_contains "ssi/19" "vibe-flow key present" "$skill" 'vibe-flow'
}

test_feature_config_propagation() {
  local f; f="$(cat "$SPEC_SKILL/feature.md")"
  assert_contains "ssi/19" "feature.md references suggest-superpowers" "$f" 'suggest-superpowers'
}

test_config_section_present
test_feature_config_propagation
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
bash tests/spec/run.sh 2>&1 | grep 'ssi/19'
```

Note: `test_config_section_present` may already pass if Task 18 is done. `test_feature_config_propagation` should fail.

- [ ] **Step 3: Add config-read note to SKILL.md Agent quick start**

In `.agents/skills/spec/SKILL.md`, find the `## Agent quick start` section. Add step 0 before the existing step 1:

```markdown
0. **Load config** — check for `.spec/.config.yaml`; if present, read and apply
   behavior adjustments (see [## Config](#config)) silently before any other step.
```

- [ ] **Step 4: Add suggest-superpowers note to feature.md**

In `.agents/skills/spec/feature.md`, find the `## Feature authoring flow` section header or preamble. Add this note immediately after the section header:

```markdown
> **Config note:** If `.spec/.config.yaml` sets `suggest-superpowers: false`,
> suppress all "Superpower tip" callouts below and self-execute every step
> without offering. Load config before step 1.
```

- [ ] **Step 5: Run tests to verify they pass**

```bash
bash tests/spec/run.sh 2>&1 | grep 'ssi/19'
```

Expected: all `PASS [ssi/19]`.

- [ ] **Step 6: Run full test suite**

```bash
bash tests/spec/run.sh
```

Expected: `0 failed`.

- [ ] **Step 7: Run validate**

```bash
bash .agents/skills/spec/scripts/validate.sh
```

Expected: `Errors: 0`.

- [ ] **Step 8: Commit**

```bash
git add .agents/skills/spec/SKILL.md .agents/skills/spec/feature.md tests/spec/run.sh
git commit -m "feat(spec): wire config read into session start and propagate suggest-superpowers to feature.md"
```

---

## Self-Review

**Spec coverage check:**

| Unit | Task | Covered |
|---|---|---|
| ssi/1 SKILL.md v2.0 frontmatter | Task 1 | ✓ |
| ssi/2 ## Roles section | Task 11 | ✓ |
| ssi/3 Routing expansion | Task 12 | ✓ |
| ssi/4 feature.md output profiles | Task 3 | ✓ |
| ssi/5 promote.sh | Task 4 | ✓ |
| ssi/6 lessons-for.sh | Task 5 | ✓ |
| ssi/7 SF13 stale link checker | Task 7 | ✓ |
| ssi/8 SF14 scope conflict | Task 8 | ✓ |
| ssi/9 branch doc templates | Task 2 | ✓ |
| ssi/10 scan-merges.sh | Task 6 | ✓ |
| ssi/11 OpenSpec docs | Task 10 | ✓ |
| ssi/12 spec-tracer SKILL.md | Task 13 | ✓ |
| ssi/13 spec-promoter SKILL.md | Task 14 | ✓ |
| ssi/14 spec-interviewer SKILL.md | Task 15 | ✓ |
| ssi/15 spec-health SKILL.md | Task 16 | ✓ |
| ssi/16 SF15+SF16 | Task 9 | ✓ |
| ssi/17 agents/ wiring | Task 17 | ✓ |
| ssi/18 setup interview + config write | Task 18 | ✓ |
| ssi/19 config read at session start | Task 19 | ✓ |

All 19 units covered.
