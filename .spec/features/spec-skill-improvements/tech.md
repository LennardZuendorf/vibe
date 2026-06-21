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
├── SKILL.md                    ← extend frontmatter + add ## Roles section
├── feature.md                  ← add ## Output profiles section
├── strategy.md                 ← no changes
├── scripts/
│   ├── validate.sh             ← extend with SF13, SF14
│   ├── setup.sh                ← no changes
│   ├── list-specs.sh           ← no changes
│   ├── promote.sh              ← NEW
│   ├── lessons-for.sh          ← NEW
│   └── score.sh                ← NEW (deferred tier-3)
└── reference/
    ├── product.md              ← document OpenSpec marker convention (opt-in)
    └── templates/              ← no changes
```

---

## SKILL.md Frontmatter Extensions

Add the following YAML fields to the existing frontmatter block (after
`metadata:`). Must be valid YAML; must not break existing parsers that read
only name/description/user-invocable.

```yaml
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

---

## SKILL.md — `## Roles` Section

Add a new section **before** `## Routing` in SKILL.md body. Structure:

```markdown
## Roles

Four composable roles. Invoke via `/spec <role>` or by phase-routing below.
Each role has a defined cognitive mode and explicit I/O contract.

### Role: spec-interviewer

**Cognitive mode:** Socratic dialogue. Push back on vague requirements.
**Phase:** feature.design steps 1–2 (WHAT interview)
**Inputs:** root product.md + tech.md + lessons.md
**Outputs:** features/<name>/product.md
**Delegates:** superpowers:brainstorming (dialogue), no code tools
**Validation:** RFC-2119 keyword in every requirement; ≥1 GWT scenario per req

RFC-2119 usage: SHALL = mandatory feature behaviour; MUST = implementation
invariant; SHOULD = strong recommendation, deviation needs justification; MAY =
optional. Choose at authoring time; do not change without explicit scope review.

### Role: spec-architect

**Cognitive mode:** Code archaeology + contract sketching.
**Phase:** feature.design steps 3–4 (HOW sketch)
**Inputs:** root tech.md + feature product.md + codebase
**Outputs:** features/<name>/tech.md (+ design.md if rigor gate = full)
**Delegates:** code-explorer (trace), code-architect (sketch)
**Validation:** All cited paths exist in repo; only filled sections written

### Role: spec-auditor

**Cognitive mode:** Rule enforcement + gap analysis.
**Phase:** any (invocable standalone via `/spec audit`)
**Inputs:** .spec/ tree
**Outputs:** structured validation report (stdout)
**Delegates:** none
**Procedure:** Run validate.sh; run score.sh; surface critical warnings with
fix suggestions; track which warnings are recurring vs new

### Role: spec-compactor

**Cognitive mode:** Synthesis + cleanup.
**Phase:** feature.compound
**Inputs:** features/<name>/tech.md (merge markers); root tech.md; lessons.md
**Outputs:** root tech.md (promoted blocks); lessons.md (new entry); plan.md (DONE row)
**Delegates:** promote.sh (extraction); none for lesson draft
**Invariants:** Never promotes feature-only detail; lesson is human-drafted
from compactor output, not auto-generated
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
