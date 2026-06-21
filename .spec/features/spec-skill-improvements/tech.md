---
type: feature-tech
feature: spec-skill-improvements
sibling: product.md
parent: ../../tech.md
updated: 2026-06-21
---

# spec-skill-improvements — Tech Spec

Implementation architecture for each requirement. All changes are additive;
the core two-layer model and 6-step authoring flow are left untouched.

---

## File Layout

```
.agents/skills/spec/
├── SKILL.md                      ← v2.0: allowed-tools, context, subagents, superpowers
│                                    frontmatter; ## Roles manifest section
├── feature.md                    ← add ## Output profiles section
├── strategy.md                   ← no changes
├── subagents/                    ← NEW composable subagent folder
│   ├── spec-tracer/
│   │   └── SKILL.md              ← NEW read-only codebase tracer (parallel-safe)
│   ├── spec-promoter/
│   │   └── SKILL.md              ← NEW compound merge-marker promoter (diff-first)
│   ├── spec-interviewer/
│   │   └── SKILL.md              ← NEW WHAT-phase interview delegator
│   └── spec-health/
│       └── SKILL.md              ← NEW structural health assessor
├── scripts/
│   ├── validate.sh               ← extend with SF13, SF14, SF15, SF16
│   ├── setup.sh                  ← no changes
│   ├── list-specs.sh             ← no changes
│   ├── promote.sh                ← NEW
│   ├── lessons-for.sh            ← NEW
│   └── scan-merges.sh            ← NEW
└── reference/
    ├── product.md                ← document OpenSpec frontmatter convention (opt-in)
    └── templates/
        ├── (existing templates)
        ├── product-topic.md      ← NEW branch doc template
        ├── tech-topic.md         ← NEW branch doc template
        ├── plan-topic.md         ← NEW branch doc template
        └── research.md           ← NEW research template
```

---

## SKILL.md Frontmatter v2.0

Bump to `version: 2.0`. Add the following YAML fields to the existing
frontmatter block (after `metadata:`). Must be valid YAML; existing parsers
that read only `name/description/user-invocable` continue to work.

```yaml
version: 2.0

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

subagents:
  - spec-tracer:     subagents/spec-tracer/SKILL.md
  - spec-promoter:   subagents/spec-promoter/SKILL.md
  - spec-interviewer: subagents/spec-interviewer/SKILL.md
  - spec-health:     subagents/spec-health/SKILL.md

superpowers:
  - superpowers:brainstorming          # WHAT interview (step 2)
  - superpowers:writing-plans          # plan units (step 5)
  - code-explorer                      # HOW codebase trace (step 4)
  - code-architect                     # HOW approach sketch (step 4)
  - superpowers:verification-before-completion  # audit quality framing

outputs:
  - .spec/features/<name>/product.md
  - .spec/features/<name>/tech.md
  - .spec/features/<name>/design.md
  - .spec/features/<name>/plan.md
  - .spec/product.md            # compound only
  - .spec/tech.md               # compound only
  - .spec/lessons.md            # compound only

reads:
  - .spec/product.md
  - .spec/tech.md
  - .spec/design.md
  - .spec/lessons.md
  - .spec/plan.md
  - .spec/features/<name>/

delegates:
  - role: spec-interviewer
    when: feature product.md WHAT phase (steps 1–2)
    superpowers: [superpowers:brainstorming]
  - role: spec-architect
    when: feature tech.md HOW phase (steps 3–4)
    superpowers: [code-explorer, code-architect]
  - role: spec-auditor
    when: validate / audit
    superpowers: []
  - role: spec-compactor
    when: compound (wrap-up, promote, record)
    superpowers: []

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
```

`<name>` is a placeholder; tools reading this field should treat it as a
pattern, not a literal path.

**`allowed-tools:` rationale:** Without explicit allowed-tools, agents in
constrained environments default to Read-only and cannot write spec files.
Listing Edit, Write, Glob, Grep, Bash, Agent here surfaces the full write
surface to the tool runner without needing per-session escalation.

**`context:` rationale:** Lists constraint documents the skill injects before
delegating to executors. Allows tooling to pre-load the right reference docs
without the agent having to discover them.

---

## SKILL.md — `## Roles` Section

Add a new section **before** `## Routing` in SKILL.md body. Structure:

```markdown
## Roles

Four composable delegation contexts. Invoke via `/spec <role>` or by
phase-routing below. Each role names an executor and the constraint document to
inject — roles are NOT custom tools; they are wiring between spec constraints
and the appropriate superpowers executor.

### Role: spec-interviewer

**Executor:** `superpowers:brainstorming`
**Phase:** feature.design steps 1–2 (WHAT interview)
**Constraint document to inject:** `feature.md § Interview for WHAT` — covers
RFC-2119 vocabulary, Scope table format, GWT scenario structure, and rigor gate
**Inputs (spec skill reads):** root product.md + tech.md + lessons.md
**Outputs (spec skill writes):** features/<name>/product.md
**Validation criteria on output:** RFC-2119 keyword in every requirement body;
≥1 GWT scenario per requirement; Scope table has Owns + Does not own columns

RFC-2119 usage: SHALL = mandatory feature behaviour; MUST = implementation
invariant; SHOULD = strong recommendation; MAY = optional. Spec skill validates
these; `superpowers:brainstorming` applies them during dialogue.

### Role: spec-architect

**Executor:** `code-explorer` (trace) + `code-architect` (sketch)
**Phase:** feature.design steps 3–4 (HOW sketch)
**Constraint document to inject:** `reference/tech.md` + feature-tech template
**Inputs (spec skill reads):** root tech.md + feature product.md + codebase
**Outputs (spec skill writes):** features/<name>/tech.md (+ design.md if full)
**Validation criteria on output:** All cited paths exist in repo; only sections
with real content written; no UX opinions in tech.md

### Role: spec-planner

**Executor:** `superpowers:writing-plans`
**Phase:** feature.plan step 5 (plan units)
**Constraint document to inject:** `reference/plan.md` — covers stable-ID
convention (`<name>/n`), Requirements Trace table format, verification row
template, same-feature-only dependency rule, and human-gate reminder
**Inputs (spec skill reads):** feature product.md + tech.md
**Outputs (spec skill writes):** features/<name>/plan.md
**Validation criteria on output:** All unit IDs follow `<name>/n`; each unit
cites ≥1 R-ID; verification column is non-empty per unit

### Role: spec-auditor

**Executor:** `validate.sh` (spec-unique; no superpower substitute)
**Phase:** any (standalone via `/spec audit`)
**Inputs:** .spec/ tree
**Outputs:** structured validation report (stdout); optionally passes `score.sh`
JSON to `superpowers:verification-before-completion` for quality assessment framing
**Procedure:** Run validate.sh → surface warnings → (optionally) run score.sh →
pass metrics JSON as context to superpowers:verification-before-completion

### Role: spec-compactor

**Executor:** `promote.sh` (spec-unique extraction); `superpowers:finishing-a-development-branch` (lesson narrative)
**Phase:** feature.compound
**Constraint document for lesson:** `strategy.md § Lessons` format — pattern,
rule, tags, date
**Inputs:** features/<name>/tech.md (merge markers); root tech.md; lessons.md
**Outputs:** root tech.md (promoted blocks via promote.sh); draft lesson entry
for human review; plan.md DONE row
**Invariant:** lesson is reviewed by human before appending; never auto-committed
```

---

## `## Routing` Extension

Add to the existing Routing table in SKILL.md:

| Argument | Action |
|---|---|
| `interview [<name>]` | Load spec-interviewer role; run steps 1–2 for `<name>` |
| `promote <name>` | Run `scripts/promote.sh <name>` |
| `audit` | Load spec-auditor role; run `validate.sh` + `score.sh` |
| `score` | Run `scripts/score.sh` |
| `lessons-for <tag>` | Run `scripts/lessons-for.sh <tag>` |

Update `argument-hint` to include: `interview [<name>]|promote <name>|audit|score|lessons-for <tag>`.

---

## `feature.md` — `## Output profiles` Section

Add **after** `## Feature authoring flow` and **before** `## Examples`:

```markdown
## Output profiles

Match caveman level to phase; compress at the source, not the sink.

### Lite (feature.design, feature.plan)

Produce precise, minimal spec sections — not truncated prose but no padding.

| File | Mandatory | Compressed | Omit |
|---|---|---|---|
| product.md | Frontmatter, problem paragraph, Scope table, req titles + strength | Scenarios → one-line Each/When/Then | Rationale paragraphs, examples |
| tech.md | Frontmatter, file paths, interface signatures | Risks → one-line bullets | Implementation narrative, decision history |
| plan.md | Frontmatter, unit ID list with R-IDs | Verification table → command only | Full evidence prose |

### Full (feature.plan reference, feature.impl, feature.verify)

All sections per template; standard Requirement+Scenario blocks; full prose.
No compression. This is the default for inter-phase reading.

### Ultra (feature.compound receipts)

All sections plus:
- Unit traceability matrix (unit ID → test path → pass/fail)
- Validation evidence summary (validate.sh run output)
- Draft lesson entry (pattern, rule, tags — human edits before promoting)
```

---

## `scripts/promote.sh`

**Contract:** `promote.sh <feature-name> [--dry-run] [--target <file>]`

**Algorithm:**
1. Compute `FEATURE_TECH=".spec/features/$1/tech.md"`; assert file exists
2. Compute `TARGET="${target:-.spec/tech.md}"`; assert file exists
3. Extract all `<!-- merge --> … <!-- /merge -->` blocks:
   - Use awk state machine: set `in_block=1` on `<!-- merge -->`, collect
     lines, close on `<!-- /merge -->`
   - Validate: start marker must appear before end marker; if not, print error
     naming the file and exit 1 (never continue to write)
   - Collect all blocks into a BLOCKS variable
4. If `--dry-run`: print `"--- would promote to $TARGET ---"` then print
   BLOCKS; exit 0
5. Write to `TMPFILE=$(mktemp)`; copy TARGET to TMPFILE; append BLOCKS;
   `mv "$TMPFILE" "$TARGET"` (atomic rename)
6. Print: `"PROMOTED $n block(s) from $FEATURE_TECH → $TARGET"`

**Safety invariants (active rules):**
- Validate marker pairing before rewriting (active rule)
- Write via temp + atomic rename (active rule)
- Never mangle on reversed markers — exit non-zero, leave target byte-unchanged

```bash
#!/usr/bin/env bash
# scripts/promote.sh — extract merge markers from feature tech.md and promote
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

# Extract blocks; validate marker order
BLOCKS=""
BLOCK_COUNT=0
in_block=0
FIRST_CLOSE=-1
FIRST_OPEN=-1
lineno=0
while IFS= read -r line; do
  ((lineno++)) || true
  if [[ "$line" == "<!-- merge -->" ]]; then
    [[ $in_block -eq 1 ]] && { echo "ERROR: nested <!-- merge --> at line $lineno in $FEATURE_TECH" >&2; exit 1; }
    in_block=1
    [[ $FIRST_OPEN -eq -1 ]] && FIRST_OPEN=$lineno
    current_block=""
  elif [[ "$line" == "<!-- /merge -->" ]]; then
    if [[ $in_block -ne 1 ]]; then
      echo "ERROR: <!-- /merge --> without opener at line $lineno in $FEATURE_TECH" >&2
      exit 1
    fi
    in_block=0
    BLOCKS="${BLOCKS}${current_block}"$'\n'
    ((BLOCK_COUNT++)) || true
    [[ $FIRST_CLOSE -eq -1 ]] && FIRST_CLOSE=$lineno
  elif [[ $in_block -eq 1 ]]; then
    current_block="${current_block}${line}"$'\n'
  fi
done < "$FEATURE_TECH"

[[ $in_block -eq 1 ]] && { echo "ERROR: unclosed <!-- merge --> block in $FEATURE_TECH" >&2; exit 1; }

if [[ $BLOCK_COUNT -eq 0 ]]; then
  echo "WARN: no <!-- merge --> blocks found in $FEATURE_TECH" >&2
  exit 0
fi

if [[ $DRY_RUN -eq 1 ]]; then
  echo "--- would promote $BLOCK_COUNT block(s) to $TARGET ---"
  echo "$BLOCKS"
  exit 0
fi

TMPFILE="$(mktemp)"
cat "$TARGET" > "$TMPFILE"
printf '\n%s\n' "$BLOCKS" >> "$TMPFILE"
mv "$TMPFILE" "$TARGET"

echo "PROMOTED $BLOCK_COUNT block(s) from $FEATURE_TECH → $TARGET"
```

---

## `scripts/lessons-for.sh`

**Contract:** `lessons-for.sh <tag> [<tag>...] [--format markdown|inject|json]`

**Algorithm:**
1. Read `.spec/lessons.md`; split on `### ` headings
2. For each lesson, extract its `**Tags:**` line; check if any supplied tag
   appears in it (case-insensitive substring match)
3. Collect matching lessons
4. Format and print per `--format` flag (default: markdown)
5. Exit 0 always; empty stdout when no matches

**Format contracts:**
- `markdown` — verbatim lesson blocks separated by `---`
- `inject` — `<!-- lessons: <tags> -->\n<blocks>\n<!-- /lessons -->`
- `json` — `[{"title":…,"pattern":…,"rule":…,"tags":[…],"date":…}]`

```bash
#!/usr/bin/env bash
# scripts/lessons-for.sh — extract lessons matching tags from lessons.md
set -euo pipefail

FORMAT="markdown"
TAGS=()

for arg in "$@"; do
  case "$arg" in
    --format) : ;;        # handled by shift below; placeholder
    markdown|inject|json) FORMAT="$arg" ;;
    *) TAGS+=("$arg") ;;
  esac
done
# Handle --format <value> pair
for i in "${!@}"; do
  if [[ "${@:$i:1}" == "--format" ]]; then
    FORMAT="${@:$((i+1)):1}"
  fi
done

LESSONS_FILE=".spec/lessons.md"
[[ -f "$LESSONS_FILE" ]] || exit 0
[[ ${#TAGS[@]} -gt 0 ]] || { echo "usage: lessons-for.sh <tag> [<tag>...] [--format markdown|inject|json]" >&2; exit 1; }

# Read and parse lessons (pure bash, no awk dependency)
current_block=""
matched_blocks=()
in_lesson=0

while IFS= read -r line || [[ -n "$line" ]]; do
  if [[ "$line" =~ ^###\  ]]; then
    if [[ $in_lesson -eq 1 && -n "$current_block" ]]; then
      tags_line="$(echo "$current_block" | grep '^\*\*Tags:\*\*' | head -1 || true)"
      for tag in "${TAGS[@]}"; do
        if echo "$tags_line" | grep -qi "$tag"; then
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

# Check final block
if [[ $in_lesson -eq 1 && -n "$current_block" ]]; then
  tags_line="$(echo "$current_block" | grep '^\*\*Tags:\*\*' | head -1 || true)"
  for tag in "${TAGS[@]}"; do
    if echo "$tags_line" | grep -qi "$tag"; then
      matched_blocks+=("$current_block")
      break
    fi
  done
fi

[[ ${#matched_blocks[@]} -eq 0 ]] && exit 0

case "$FORMAT" in
  inject)
    echo "<!-- lessons: ${TAGS[*]} -->"
    for block in "${matched_blocks[@]}"; do echo "$block"; echo; done
    echo "<!-- /lessons -->"
    ;;
  json)
    echo "["
    first=1
    for block in "${matched_blocks[@]}"; do
      [[ $first -eq 0 ]] && echo ","
      title="$(echo "$block" | head -1 | sed 's/^### //')"
      pattern="$(echo "$block" | grep '^\*\*Pattern:\*\*' | sed 's/\*\*Pattern:\*\* //')"
      rule="$(echo "$block" | grep '^\*\*Rule:\*\*' | sed 's/\*\*Rule:\*\* //')"
      tags="$(echo "$block" | grep '^\*\*Tags:\*\*' | sed 's/\*\*Tags:\*\* //')"
      date="$(echo "$block" | grep '^\*\*Date:\*\*' | sed 's/\*\*Date:\*\* //')"
      printf '{"title":"%s","pattern":"%s","rule":"%s","tags":"%s","date":"%s"}' \
        "$title" "$pattern" "$rule" "$tags" "$date"
      first=0
    done
    echo "]"
    ;;
  *)  # markdown
    for block in "${matched_blocks[@]}"; do echo "$block"; echo "---"; done
    ;;
esac
```

---

## `validate.sh` Extensions

### SF13 — Cross-reference integrity

Insert after the existing orphaned-children check. Logic:

```bash
check_sf13_stale_feature_links() {
  local root_files=(".spec/product.md" ".spec/tech.md" ".spec/design.md" ".spec/plan.md")
  for f in "${root_files[@]}"; do
    [[ -f "$f" ]] || continue
    while IFS= read -r link; do
      # Extract relative path targets pointing to features/ or archive/
      if [[ "$link" =~ features/([^/)]+) || "$link" =~ archive/([^/)]+) ]]; then
        local target
        target="$(echo "$link" | grep -oP '(?<=\()features/[^)]+|(?<=\()archive/[^)]+' | head -1)"
        [[ -z "$target" ]] && continue
        local abs_target=".spec/$target"
        [[ -e "$abs_target" ]] || yellow "SF13: stale link in $f → $target (not found)"
      fi
    done < <(grep -nP '\[.*?\]\(.*?\)' "$f" || true)
  done
}
```

Add `check_sf13_stale_feature_links` call in the main validation run sequence.

### SF14 — Scope conflict detection

```bash
check_sf14_scope_conflicts() {
  local feature_dirs=(.spec/features/*/product.md)
  declare -A owns_map
  for f in "${feature_dirs[@]}"; do
    [[ -f "$f" ]] || continue
    local feature
    feature="$(basename "$(dirname "$f")")"
    local in_owns=0
    while IFS= read -r line; do
      [[ "$line" =~ "| Owns" ]] && in_owns=1 && continue
      [[ "$line" =~ "| Does not own" ]] && in_owns=0 && continue
      if [[ $in_owns -eq 1 && "$line" =~ ^\|[^|]+\| ]]; then
        local item
        item="$(echo "$line" | awk -F'|' '{print $2}' | tr '[:upper:]' '[:lower:]' | xargs)"
        [[ -z "$item" ]] && continue
        if [[ -v "owns_map[$item]" ]]; then
          yellow "SF14: scope conflict — both '${owns_map[$item]}' and '$feature' own: $item"
        else
          owns_map["$item"]="$feature"
        fi
      fi
    done < "$f"
  done
}
```

---

## Routing Extension (Updated)

Add to the existing Routing table in SKILL.md (replacing the earlier partial
table):

| Argument | Action |
|---|---|
| `interview [<name>]` | Load spec-interviewer subagent; run steps 1–2 for `<name>` |
| `promote <name>` | Run `scripts/promote.sh <name>` via spec-promoter subagent |
| `audit` | Load spec-auditor role; run `validate.sh` + pass to verification-before-completion |
| `score` | Run `scripts/score.sh` (deferred) |
| `lessons-for <tag>` | Run `scripts/lessons-for.sh <tag>` |
| `diff <name>` | Run `scripts/scan-merges.sh <name>` — show pending merge blocks |
| `health` | Invoke spec-health subagent — structural assessment of .spec/ tree |
| `research <name>` | Open `.spec/features/<name>/research.md`; suggest spec-tracer for discovery |

Update `argument-hint:` to: `strategy|feature <name>|interview [<name>]|promote <name>|audit|diff <name>|health|research <name>|lessons-for <tag>|validate|setup`.

---

## Composable Subagent Architecture

Thin SKILL.md root routes to `subagents/<name>/SKILL.md` for each
specialised role. The root SKILL.md holds the routing manifest and phase
wiring; the subagent files hold the deep context for each role.

**Folder contract:**
- Each subagent folder is `subagents/<role-name>/SKILL.md`
- Subagent SKILL.md files are self-contained; they can be invoked standalone
- They inherit the parent's `allowed-tools:` unless they declare their own

### subagents/spec-tracer/SKILL.md

**Purpose:** Read-only codebase tracer for HOW phase (feature.design step 4).
Finds existing files, interfaces, and contracts relevant to a feature. Feeds
output to spec-architect for the HOW sketch.

**Allowed-tools:** Read, Glob, Grep (read-only; never Edit/Write)

**Parallel-safe:** yes — multiple spec-tracer instances can run concurrently
since they only read.

**Contract:**
```yaml
name: spec-tracer
description: Read-only codebase tracer for spec HOW phase
user-invocable: false
allowed-tools: [Read, Glob, Grep]
```

**Orders:** Given a feature name and product.md requirements, trace:
1. Relevant existing files (Glob patterns from tech.md contracts)
2. Interface signatures (Grep for function/type declarations)
3. Cross-references in root tech.md that apply
4. Gaps (requirements with no existing code path)

Output: structured trace document injected into spec-architect context.

### subagents/spec-promoter/SKILL.md

**Purpose:** Compound-phase merge-marker extractor. Shows diff first, then
executes `promote.sh` with human confirmation.

**Allowed-tools:** Read, Bash (for promote.sh execution)

**Diff-first invariant:** MUST print the would-promote blocks (dry-run output)
and receive explicit human confirmation before executing the live promote.

**Contract:**
```yaml
name: spec-promoter
description: Compound merge-marker extractor with diff-first confirmation
user-invocable: false
allowed-tools: [Read, Bash]
```

**Orders:**
1. Run `bash .agents/skills/spec/scripts/promote.sh <name> --dry-run`
2. Show output to user: "These blocks will promote to root tech.md — confirm?"
3. On explicit confirmation: run without `--dry-run`
4. Report: n blocks promoted, paths written

### subagents/spec-interviewer/SKILL.md

**Purpose:** WHAT-phase interview delegator. Injects constraint context then
hands off to `superpowers:brainstorming`.

**Allowed-tools:** Read (constraint loading only)

**Delegation contract:** Must inject `feature.md § Interview for WHAT` as
context before invoking brainstorming. Offers the delegation to the user;
does not silently run it.

**Contract:**
```yaml
name: spec-interviewer
description: WHAT-phase interview — injects constraints, delegates to superpowers:brainstorming
user-invocable: false
allowed-tools: [Read]
delegates-to: superpowers:brainstorming
```

**Orders:**
1. Read `.agents/skills/spec/feature.md` § Interview for WHAT
2. Read `.spec/product.md`, `.spec/tech.md`, `.spec/lessons.md`
3. Tell user: *"I can run this as a `superpowers:brainstorming` session with
   spec format constraints pre-loaded — want me to?"*
4. On yes: invoke `superpowers:brainstorming` with constraint context

### subagents/spec-health/SKILL.md

**Purpose:** Structural health assessor. Reads the full `.spec/` tree and
produces a prioritised list of structural problems beyond what `validate.sh`
checks — staleness, balance, missing sections, spec drift.

**Allowed-tools:** Read, Glob, Grep, Bash (validate.sh run)

**Contract:**
```yaml
name: spec-health
description: Structural health assessor for .spec/ tree
user-invocable: true
allowed-tools: [Read, Glob, Grep, Bash]
```

**Orders:**
1. Run `validate.sh` and capture output
2. Read root product.md, tech.md, plan.md, lessons.md
3. Check: are all features in plan.md Feature Sequence?
4. Check: do any feature folders have no corresponding plan entry?
5. Check: are any feature folder plans missing unit IDs?
6. Check: do any root specs exceed 300 lines (over-loaded)?
7. Report a prioritised health list: CRITICAL / WARN / INFO items

---

## Branch Doc Templates

New template files under `reference/templates/`. Mirror the existing
`feature-product.md` / `feature-tech.md` pattern but for cross-cutting docs.

### `reference/templates/product-topic.md`

```markdown
---
type: product-topic
topic: <topic>
parent: product.md
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
```

**When to create:** Only when a concern truly spans every feature — not as a
substitute for a feature folder. Examples: design system, accessibility
conventions, naming conventions that every feature enforces.

### `reference/templates/tech-topic.md`

```markdown
---
type: tech-topic
topic: <topic>
parent: tech.md
updated: YYYY-MM-DD
---

# <topic> — Tech

One paragraph: what this cross-cutting technical concern governs.

## Contract

<Invariant every feature must satisfy.>

## File Layout

```
path/to/shared/
├── file.ext     ← purpose
```

## Integration Points

| Feature | How it integrates |
|---|---|
| <name> | … |
```

### `reference/templates/plan-topic.md`

```markdown
---
type: plan-topic
topic: <topic>
parent: plan.md
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
```

**When to create:** Only when root `plan.md` grows unwieldy because a
cross-cutting plan (e.g. a multi-feature migration) needs its own sequence.
Default to keeping everything in root `plan.md`.

### `reference/templates/research.md`

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

---

## `scripts/scan-merges.sh`

**Contract:** `scan-merges.sh [<feature-name>] [--format table|json|plain]`

**Purpose:** Report all `<!-- merge -->` / `<!-- /merge -->` blocks across the
.spec/ tree, or for a single feature. Structured alternative to grepping for
merge markers manually.

**Algorithm:**
1. If `<feature-name>` given: scan `.spec/features/<name>/tech.md` only
2. Else: scan all `.spec/features/*/tech.md`
3. For each file: collect block count, line ranges, first-line preview of each block
4. Detect unclosed markers; report as error
5. Format output per `--format` (default: table)

**Format contracts:**
- `table` — human-readable: `feature | file | blocks | first-line-preview`
- `json` — `[{"feature":…,"file":…,"blocks":[{"start":n,"end":n,"preview":…}]}]`
- `plain` — one line per block: `<file>:<start>-<end>: <preview>`

**Exit codes:**
- `0` — scan complete (even if no blocks found)
- `1` — unclosed marker detected (file name and line printed to stderr)

**Safety:** Read-only. Never modifies files. Safe to run at any time.

---

## `validate.sh` Extensions — SF15 and SF16

### SF15 — Root spec length check

Insert after SF14. Warn when any root spec file exceeds 200 lines.

```bash
check_sf15_root_spec_length() {
  local max_lines=200
  local root_files=(".spec/product.md" ".spec/tech.md" ".spec/design.md"
                    ".spec/plan.md" ".spec/lessons.md")
  for f in "${root_files[@]}"; do
    [[ -f "$f" ]] || continue
    local count
    count="$(wc -l < "$f")"
    if [[ $count -gt $max_lines ]]; then
      yellow "SF15: $f is $count lines (>${max_lines}); consider splitting \
into a branch doc (product-{topic}.md, tech-{topic}.md)"
    fi
  done
}
```

Rationale: root specs that grow past 200 lines usually contain feature-level
detail that belongs in a feature folder or branch doc. The 200-line threshold
is a heuristic; adjust via env `SPEC_ROOT_MAX_LINES` if needed.

### SF16 — Lessons entries without Tags

Insert after SF15. Warn on any lesson block in `lessons.md` that lacks a
`**Tags:**` line.

```bash
check_sf16_lessons_tags() {
  local lessons=".spec/lessons.md"
  [[ -f "$lessons" ]] || return 0
  local current_title=""
  local has_tags=0
  while IFS= read -r line; do
    if [[ "$line" =~ ^###\  ]]; then
      if [[ -n "$current_title" && $has_tags -eq 0 ]]; then
        yellow "SF16: lesson '$current_title' in lessons.md has no **Tags:** line"
      fi
      current_title="${line#\#\#\# }"
      has_tags=0
    elif [[ "$line" =~ ^\*\*Tags:\*\* ]]; then
      has_tags=1
    fi
  done < "$lessons"
  # Check final entry
  if [[ -n "$current_title" && $has_tags -eq 0 ]]; then
    yellow "SF16: lesson '$current_title' in lessons.md has no **Tags:** line"
  fi
}
```

**Tags rationale:** `lessons-for.sh` uses `**Tags:**` lines for extraction.
Entries without tags are invisible to D8 injection hooks.

---

## OpenSpec Frontmatter Format

Optional machine-readable overlay. Fields are added to existing frontmatter;
their absence does not affect validate.sh (SF0-SF16 do not require them).

### product.md — `requirements:` list

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

**Purpose:** allows tooling to extract requirement IDs without parsing prose.
The `scenarios:` count is informational; validate.sh SF-opt (warn-only, opt-in)
can check it against actual GWT blocks.

### plan.md — `units:` list

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

**Purpose:** allows tooling to compute dependency graphs and status summaries
without parsing markdown tables.

**Adoption rule:** OpenSpec frontmatter is opt-in per file. Reference documents
(`reference/product.md`, `reference/plan.md`) note the convention as optional.
Validate.sh does not error on absence; a future opt-in SF check may warn on
malformed entries when the field is present.

---

## Risks

**Risk: promote.sh atomicity on NFS / cross-device mounts**
`mv` is not atomic across devices. The temp file must be created on the same
filesystem as the target. Mitigate: use `mktemp --tmpdir="$(dirname "$TARGET")"`.

**Risk: lessons-for.sh bash associative arrays on macOS bash 3**
macOS ships bash 3.2 which lacks `declare -A`. Mitigate: use Python-style
awk for the JSON path; use only basic bash for markdown/inject formats.
`promote.sh` and `validate.sh` already use only bash 3-compatible constructs.

**Risk: SF14 false positives on partial keyword matches**
Substring matching can flag unrelated items (e.g., "state" matching "flow state"
and "state machine"). Mitigate: normalize to full table-row text; require
matches be longer than 5 characters; document false-positive rate in test.

**Risk: SKILL.md frontmatter size**
Adding ~30 lines of YAML to frontmatter increases the skill header. Confirm
that Claude Code's skill parser handles multi-field frontmatter without
truncating (it does, but worth testing with a larger-than-current example).

<!-- merge -->
## Cross-cutting contracts promoted from spec-skill-improvements

### Promote script contract
All compound tooling in this repo SHOULD call `bash .agents/skills/spec/scripts/promote.sh <feature>`
rather than implementing merge-marker extraction independently. The promote.sh
script is the single source of the active rule: "validate marker pairing before
rewriting."

### Lessons extraction contract
All D8 hook implementations SHOULD call `bash .agents/skills/spec/scripts/lessons-for.sh <tag>`
rather than reading lessons.md directly, so that formatting and tag matching
stay consistent.
<!-- /merge -->
