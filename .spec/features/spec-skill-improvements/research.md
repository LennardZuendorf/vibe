---
type: feature-research
feature: spec-skill-improvements
parent: ../../product.md
updated: 2026-06-21
---

# Spec Skill Improvements — Research & Design Exploration

Discovery artifacts, peer analysis, and creative ideation for v2 of the spec
skill bundle. This document informs the feature spec; it is not the spec itself.

---

## 1. Purpose

The spec skill (v1.8) is complete and dogfood-proven. Four features shipped,
44 tests green, warn-first validation covering SF0–SF12. The question is not
"does it work?" — it does — but "what would make it genuinely excellent?"

This research explores three angles the user named:

- **Specialized subagents** — dedicated roles within the spec authoring process
- **OpenSpec + superpowers plan compatibility** — interop with structured formats
  and the `superpowers:*` delegation ecosystem
- **Skill file improvements** — what v2 SKILL.md metadata should express

Plus creative ideas surfaced during deep reading of the codebase.

---

## 2. Spec Skill v1.8 — Honest Maturity Audit

### Strengths (keep, don't touch)

- **Two-layer model** — root (persistent) vs feature (ephemeral) is the right
  abstraction; no improvement needed here
- **6-step authoring flow** — locate → WHAT → rigor gate → HOW → plan → skip;
  good micro-flow, well-sequenced
- **Warn-first validators** — SF0–SF12 cover the critical structural invariants;
  graduated severity is correct
- **Feature deletion discipline** — CODE IS TRUTH; archive-then-delete is the
  right answer for transient specs
- **Lessons format** — pattern + rule + tags + date is clean and extractable

### Pain Points (verified against dogfood runs)

**Pain 1: Compound is manual and scary.**
The wrap-up sequence (promote → record → update plan → archive → delete) has 5
distinct steps with no tooling. The only merge marker support is prose
documentation — no script extracts `<!-- merge -->` blocks and promotes them.
Result: compound is the most error-prone phase; the "marker pairing" lesson
exists precisely because manual merge went wrong.

**Pain 2: The WHAT interview is agent-dependent.**
Step 2 (interview for WHAT) depends entirely on the receiving agent's ability
to enforce RFC-2119 keywords and Given/When/Then structure. There is no
automated check of interview output before it becomes the authoritative product
spec. A weak interview pass silently produces weak requirements.

**Pain 3: D8 lessons retrieval is a human task.**
`lessons.md` has tagged entries, but "read lessons at session start" means the
agent reads all of them. There is no extraction path for "give me the lessons
most relevant to this feature." With 5 entries it doesn't matter; at 50 it will.

**Pain 4: Superpowers delegation is underdocumented.**
`vibe-feature` mentions `superpowers:brainstorming`, `superpowers:writing-plans`,
and `superpowers:executing-plans` by name. The `spec` skill does not express
which superpowers belong to which authoring step. A consuming agent must read
both SKILL.md files and reason about the handoff contract — this is fragile.

**Pain 5: SKILL.md is opaque to tooling.**
The frontmatter carries `name`, `description`, `user-invocable`, `argument-hint`,
`allowed-tools`, `compatibility`, and a few metadata fields. It does not express:
- Which files it writes (outputs)
- Which files it reads (inputs)
- Which superpowers or subagents it delegates to
- Which flow phases invoke it
- What caveman output level it targets per phase

This makes automated skill discovery and dependency analysis guesswork.

**Pain 6: No output density calibration in spec docs.**
vibe-feature emits `caveman=lite` during `feature.design`. But there is no
definition of what "lite" means for a spec skill output — what sections to emit,
what to compress, what must always be full. A `caveman=full` run produces the
same output as `caveman=lite` because the spec skill has no awareness of levels.

---

## 3. Peer Analysis

### 3a. OpenSpec — The Structured Spec Vision

No formal "OpenSpec" standard exists in this repository or as a single external
tool at time of writing. However, the concept maps cleanly to a pattern visible
in the success of **OpenAPI** (REST), **AsyncAPI** (event-driven), and **GraphQL
SDL** (schema): *machine-readable structured specs alongside human prose*.

For product/tech specs, an "OpenSpec" approach means:

```yaml
# Inside product.md frontmatter — OpenSpec-compatible extensions
requirements:
  - id: R1
    title: "Requirement title"
    strength: SHALL
    scenarios:
      - id: R1-S1
        given: "context"
        when: "action"
        then: "outcome"
    tests:
      - path: tests/spec/run.sh
        case: test_r1_scenario1
```

**Value:** Requirements become queryable. A `scripts/export.sh` can emit
JSON/YAML for:
- CI traceability dashboards
- External PM tools (Linear, Jira)
- LLM context injection (structured is more token-efficient than prose)
- Automated test coverage gap detection (which R-IDs have no test cases?)

**Cost:** Doubles the authoring burden if not carefully scaffolded. Must be
additive (prose stays canonical; structured is derived/optional).

**Recommendation:** Implement as optional frontmatter enrichment; keep prose
canonical. Export script reads both and merges.

### 3b. Cavekit — Output Density Management

The vibe harness defines three **caveman** levels:

| Level | Description | When used |
|---|---|---|
| `lite` | Compressed; key facts only; no boilerplate | Design, plan phases |
| `full` | Standard verbosity; complete sections | Impl, verify phases |
| `ultra` | Maximum detail; all receipts shown | Compound (receipts mode) |

Currently the spec skill emits the same output regardless of caveman level —
the receiving agent is expected to compress. This is backward from ideal.

A **Cavekit-aware** spec skill would define **output profiles** per phase:

**`lite` profile** (feature.design):
- Mandatory: frontmatter, Scope table, Requirements list (titles + strength only)
- Compressed: scenarios as one-line summaries, tech.md sections as bullet points
- Omitted: verbose prose, examples, rationale paragraphs

**`full` profile** (feature.plan, feature.impl reference):
- All sections; standard Requirement+Scenario blocks; full tech.md

**`ultra` profile** (feature.compound receipts):
- All sections + validation evidence + unit traceability matrix

The Cavekit insight: **compress at the source (spec authoring), not at the
sink (agent reading)**. Smaller specs are faster to load, cheaper to inject,
and force the author to be precise.

### 3c. Superpowers Ecosystem — Delegation Map

`vibe-feature` references these superpowers inline:

| Step | Superpower | Role |
|---|---|---|
| feature.design | `superpowers:brainstorming` | Dialogue partner for WHAT interview |
| feature.design | `code-explorer` | Trace codebase for HOW section |
| feature.design | `code-architect` | Sketch approaches in tech.md |
| feature.plan | `superpowers:writing-plans` | Plan unit structure, ID assignment |
| feature.plan | `code-architect` | Validate plan feasibility |
| feature.impl | `superpowers:executing-plans` | Drive unit-by-unit impl |
| feature.impl | `superpowers:test-driven-development` | Write tests first |
| feature.verify | `superpowers:verification-before-completion` | Evidence gathering |
| feature.verify | `superpowers:requesting-code-review` | Hand to code-reviewer |

The `spec` skill (v1.8) has no `delegates:` list in its frontmatter and does
not explicitly document which superpowers it can invoke for its own steps.

**Gap:** The spec feature.md says "Delegate tracing to `code-explorer`; sketch
approaches with `code-architect` when needed" — but this is prose, not
machine-readable. A skill-discovery tool cannot parse this.

**Pattern to adopt:** Add `delegates:` frontmatter as a structured list with
`when:` conditions per delegate:

```yaml
delegates:
  - superpower: code-explorer
    when: feature tech.md HOW tracing
  - superpower: code-architect
    when: feature tech.md approach sketching
  - superpower: superpowers:brainstorming
    when: feature product.md WHAT interview
```

This makes the delegation contract explicit for both human readers and future
tooling (skill dependency graphs, auto-composition).

### 3d. Specialized Subagents — Role-Based Architecture

Currently the spec skill is monolithic: one SKILL.md handles all authoring
phases. But the phases have very different cognitive profiles:

| Phase | Cognitive mode | Skills needed |
|---|---|---|
| WHAT interview | Socratic dialogue | RFC-2119, GWT, boundary negotiation |
| HOW sketching | Code archaeology | File tracing, contract reading |
| Plan writing | Structured decomposition | Unit ID assignment, dependency mapping |
| Validation | Rule enforcement | Structural checks, traceability |
| Compound | Synthesis + cleanup | Promotion logic, lesson extraction |

A **subagent architecture** would split these into specialized roles that the
`spec` skill composes:

**`spec-interviewer`** — WHAT phase specialist
- Deep RFC-2119 vocabulary (SHALL vs MUST vs SHOULD vs MAY semantics)
- Given/When/Then structure enforcer
- Pushes back on vague requirements ("users can filter things" → "users SHALL
  filter search results by one or more of: date range, content type, author")
- Suggests Scope table boundaries based on neighbouring features
- Outputs: `features/<name>/product.md`

**`spec-architect`** — HOW phase specialist
- Delegates to `code-explorer` and `code-architect`
- Knows the feature tech.md sections and which to populate
- Marks `<!-- merge -->` blocks automatically for cross-cutting decisions
- Validates that tech.md cites real file paths
- Outputs: `features/<name>/tech.md`

**`spec-auditor`** — Validation specialist
- Runs and interprets `validate.sh` output
- Provides fix suggestions per warning
- Tracks validation history (which warnings are new vs recurring)
- Integrates with CI
- Outputs: structured validation report

**`spec-compactor`** — Compound specialist
- Extracts `<!-- merge -->` blocks and promotes to root
- Identifies what "graduates" vs "stays"
- Drafts the lesson entry based on compound evidence
- Prompts for archive → delete
- Outputs: merged root specs + `lessons.md` entry

**Implementation:** Not necessarily separate SKILL.md files — can be sections
in the main SKILL.md with dedicated subsections per role, callable as
`/spec interview`, `/spec audit`, `/spec promote`.

---

## 4. Improvement Catalog

### Improvement 1: Subagent Profiles in SKILL.md

**What:** Add dedicated `## Role: spec-interviewer`, `## Role: spec-architect`,
`## Role: spec-auditor`, `## Role: spec-compactor` sections in SKILL.md with
explicit cognitive mode, delegation chain, input/output contracts, and
validation criteria.

**Why:** Reduces the monolithic burden on the agent invoking `/spec`. Each role
can be invoked discretely when only that phase is needed (e.g., `/spec audit`
without re-running the WHAT interview).

**How:** Extend SKILL.md § Routing with role-based routes; add `## Roles`
section before `## Routing`. Add `roles:` to frontmatter listing available
roles with their `argument-hint` keywords.

**Effort:** Small (text changes; no scripts). **ROI:** High (every feature uses
all four phases).

---

### Improvement 2: `scripts/promote.sh` — Compound Automation

**What:** `bash .agents/skills/spec/scripts/promote.sh <name>` extracts
`<!-- merge -->` / `<!-- /merge -->` blocks from `features/<name>/tech.md` and
offers a dry-run preview before atomic promotion to root `tech.md`.

**Why:** Compound is currently manual and the most error-prone phase (lesson
about reversed markers). Automating the mechanical part reduces risk.

**Contract:**
```
promote.sh <feature-name> [--dry-run] [--target <root-file>]
```
- Default target: `.spec/tech.md`
- Validates markers exist, are paired, and start-precedes-end (lesson: never mangle)
- Dry-run shows the diff before applying
- Writes via temp + atomic rename
- Prints: `PROMOTED <n> blocks from features/<name>/tech.md → .spec/tech.md`
- Errors hard on malformed markers (warn on no markers found)

**Effort:** Medium (50–80 line script). **ROI:** High (used every compound run).

---

### Improvement 3: `scripts/lessons-for.sh` — D8 Extraction

**What:** `bash .agents/skills/spec/scripts/lessons-for.sh <tag> [<tag>...]`
reads `.spec/lessons.md`, extracts lessons whose `**Tags:**` include any
supplied tag, and prints them in a format ready for prompt injection.

**Why:** D8 in vibe-flow requires "read relevant lessons at session start." With
5 lessons this is trivial; at 50 it degrades into noise. Tag-based extraction
narrows the signal.

**Contract:**
```
lessons-for.sh <tag> [<tag>...] [--format json|markdown|inject]
```
- `markdown` (default): prints matching lessons verbatim
- `inject`: wraps in `<!-- lessons: <tags> -->` delimiters for hook injection
- `json`: structured export (title, pattern, rule, tags, date)
- Returns exit 0 with empty output when no matches (graceful)

**Effort:** Small (40–60 line script). **ROI:** Medium now, high at scale.

---

### Improvement 4: Cavekit-Aware Output Profiles in feature.md

**What:** Add a `## Output profiles` section to `feature.md` defining what
the spec skill emits at each caveman level, so agents can set `caveman=<level>`
and get predictably compressed output.

**Lite profile:**
- product.md: Scope table + requirement titles (no scenarios)
- tech.md: File paths + contract signatures only (no prose rationale)
- plan.md: Unit IDs + requirement refs only (no verification rows)

**Full profile:**
- All sections per template; standard Requirement+Scenario blocks; full prose

**Ultra profile (compound receipts):**
- All sections + validation evidence rows + unit traceability matrix + lesson draft

**Why:** The current skill injects the same verbosity regardless of phase. This
wastes tokens in impl (the agent reads the full spec) and loses precision in
design (compressed output forces precision).

**How:** Add the `## Output profiles` section to `feature.md`; reference it
from `SKILL.md` § Routing (which already mentions `argument-hint`).

**Effort:** Small (text changes only). **ROI:** Medium (affects all phases).

---

### Improvement 5: SKILL.md Metadata Enrichment

**What:** Extend the SKILL.md frontmatter with structured fields:

```yaml
outputs:
  - .spec/features/<name>/product.md
  - .spec/features/<name>/tech.md
  - .spec/features/<name>/design.md
  - .spec/features/<name>/plan.md
  - .spec/product.md      # during compound
  - .spec/tech.md         # during compound
  - .spec/lessons.md      # during compound

reads:
  - .spec/product.md
  - .spec/tech.md
  - .spec/lessons.md
  - .spec/plan.md
  - .spec/features/<name>/

delegates:
  - role: spec-interviewer
    when: feature product.md WHAT phase
    superpowers: [brainstorming]
  - role: spec-architect
    when: feature tech.md HOW phase
    superpowers: [code-explorer, code-architect]
  - role: spec-auditor
    when: validate phase
    superpowers: []
  - role: spec-compactor
    when: compound phase
    superpowers: []

phases:
  - setup.apply
  - strategy.spec
  - feature.design
  - feature.plan
  - feature.compound
  - strategy.compound

caveman:
  lite: design and plan phases; compress scenarios and prose
  full: impl reference and verify; all sections
  ultra: compound receipts; all sections + evidence + traceability
```

**Why:** Enables automated skill dependency graphs, context pre-loading, and
skill-discovery tools. Makes the spec skill self-describing.

**How:** Extend SKILL.md frontmatter (YAML); update validate.sh to check
`delegates:` entries resolve to real skill paths or known superpowers.

**Effort:** Small (frontmatter additions + one validator). **ROI:** Medium now,
high as the ecosystem scales.

---

### Improvement 6: OpenSpec-Compatible Requirement Metadata

**What:** Add optional structured metadata to requirement blocks so they are
machine-queryable alongside the human-readable prose:

```markdown
### Requirement: Fast search results

<!-- spec:requirement id="R3" strength="SHALL" -->
Search results SHALL appear within 200ms of the user's keypress on cached data.
<!-- /spec:requirement -->

#### Scenario: Cache hit
<!-- spec:scenario id="R3-S1" -->
**Given** a warm search cache
**When** the user types a search term
**Then** results appear within 200ms
<!-- /spec:scenario -->
```

A companion `scripts/export-requirements.sh` reads these markers and emits
structured JSON/YAML for tooling integration.

**Why:** Requirements-as-data enables traceability automation, test coverage
gap detection, and external tool integration — without breaking the prose-first
authoring experience.

**Why not:** Adds authoring overhead. Must be opt-in and well-scaffolded.

**Recommendation:** Ship the marker convention as an opt-in pattern in
`feature.md` and `reference/product.md`; implement `export-requirements.sh`
as an optional script; add SF17 validator that checks marker IDs match
frontmatter requirement IDs when both are present.

**Effort:** Medium (convention + script + validator). **ROI:** Medium (unlocks
future tooling).

---

### Improvement 7: `scripts/score.sh` — Spec Quality Scorecard

**What:** `bash .agents/skills/spec/scripts/score.sh` produces a quality
scorecard for the current `.spec/` tree with dimensions:

| Dimension | Weight | How measured |
|---|---|---|
| Completeness | 30% | All required sections present; no empty placeholders |
| Precision | 25% | RFC-2119 keywords in all requirements; GWT in all scenarios |
| Traceability | 25% | All R-IDs cited in plan; all plan units have verification |
| Freshness | 10% | `updated:` dates within current feature's active period |
| Lesson capture | 10% | At least one lesson per completed feature |

Emits a score (0–100) and dimension breakdown. Designed for trend tracking:
run in CI, log the score, surface regressions.

**Why:** `validate.sh` catches structural errors; `score.sh` measures quality.
Structural validity ≠ authoring quality. A spec with correct frontmatter but
vague requirements scores low on precision.

**Effort:** Medium (100-line script). **ROI:** Medium (motivational + CI signal).

---

### Improvement 8: `scripts/interview.sh` — Guided Requirement Builder

**What:** Interactive CLI that walks the WHAT phase of feature authoring step
by step, validating each input before proceeding:

```
$ bash .agents/skills/spec/scripts/interview.sh my-feature

Step 1: Problem / Why
> Describe the problem this feature solves:
  → [user types]
  ✓ Accepted (38 words, specific outcome named)

Step 2: Scope — What does this feature OWN?
> List owned responsibilities (one per line, empty to finish):
  → [user types]
  ✓ Accepted (3 items)

Step 3: Scope — What does this feature NOT OWN (but might be confused with)?
> List explicit non-ownerships:
  → [user types]

Step 4: Requirements
> Requirement title:
  → [user types]
> Strength (SHALL/MUST/SHOULD/MAY):
  → [user types]
> Description (must use chosen strength word):
  → [user types]
  ✓ RFC-2119 keyword found: SHALL
> Add a scenario? (y/n): y
> Given:  [user types]
> When:   [user types]
> Then:   [user types]
  ✓ GWT complete

...
Writing .spec/features/my-feature/product.md ...
Done.
```

**Why:** The 6-step authoring flow depends on the agent's ability to enforce
RFC-2119 + GWT. A scaffolded CLI makes quality requirements accessible to
humans writing specs directly (not via agent) and acts as a validation harness
for agent-generated output.

**When not:** When the agent is running the interview interactively — it doesn't
need the CLI. This is for human-first authoring and offline spec work.

**Effort:** Medium-large (150-200 line bash script). **ROI:** Medium (high
value for human authors; lower for pure-agent flows).

---

### Improvement 9: SF13–SF16 Validators

**What:** Four new validation units for `validate.sh`:

**SF13: Cross-reference integrity after feature deletion**
- Scan root specs for markdown links pointing to `features/<name>/` or
  `archive/<name>/` when those folders don't exist
- WARN with exact file+line for each stale link
- Prevents "linking to deleted features" (common compound cleanup error)

**SF14: Scope conflict detection**
- Parse all feature Scope "Owns" tables
- Warn if two active features claim the same responsibility keyword
- Helps prevent boundary drift during multi-feature builds

**SF15: Cross-feature unit dependency check** (extends I1)
- SF12 catches duplicate IDs; SF15 catches cross-feature citations in unit deps
- Plan units MAY only cite same-feature unit IDs as predecessors
- Cross-feature order is a whole-feature gate in root plan only

**SF16: Stable unit ID drift**
- Compare current plan unit IDs against `.spec/.unit-id-history.json` (generated,
  gitignored)
- WARN when a unit ID disappears without being marked DONE
- WARN when a unit's title changes significantly (potential renumbering)
- Helps catch "renumbered on reorder" mistakes per D9

**Effort:** Medium (extend validate.sh; add history file). **ROI:** High for
multi-feature repos; medium for single-feature dogfood use.

---

### Improvement 10: `argument-hint` and Routing Expansion

**What:** Extend the `argument-hint` and `## Routing` table to cover the new
capabilities:

```yaml
argument-hint: "[strategy|feature [<name>]|product|tech|design|plan|lessons|setup|validate|interview [<name>]|promote <name>|audit|score|export|lessons-for <tag>]"
```

New routes in `## Routing`:

| Argument | Action |
|---|---|
| `interview <name>` | Run `scripts/interview.sh <name>` — guided WHAT interview |
| `promote <name>` | Run `scripts/promote.sh <name>` — compound promotion |
| `audit` | Run `scripts/validate.sh` + `scripts/score.sh` in sequence |
| `score` | Run `scripts/score.sh` — quality scorecard |
| `export` | Run `scripts/export-requirements.sh` — structured data export |
| `lessons-for <tag>` | Run `scripts/lessons-for.sh <tag>` — filtered lessons |

**Why:** Every new script is useless if it's not discoverable. Surfacing them
in `argument-hint` and the routing table makes them first-class citizens.

**Effort:** Tiny (text changes). **ROI:** High (discoverability multiplier for
all other improvements).

---

## 5. Cross-Cutting Themes

### Theme A: Source over convention

The best improvements (promote.sh, lessons-for.sh) automate what is currently
documented-but-manual. The spec skill has excellent conventions; the gap is
mechanical automation that enforces them.

### Theme B: Progressive enrichment over mandatory overhead

Requirements-as-data (OpenSpec markers) and spec scoring are high value only
if they're optional enrichment, not additional mandatory structure. The spec
skill's power comes from its low barrier to entry; improvements must not raise
the floor.

### Theme C: Subagents as named sections, not separate files

Creating separate SKILL.md files for each role (spec-interviewer, spec-auditor,
etc.) adds file-management overhead. Defining them as named sections in the
existing SKILL.md, callable via `/spec <role>`, preserves the single-file
discipline while enabling role-specific dispatch.

### Theme D: Cavekit compatibility is about output contracts, not compression

The right move is not to compress spec output (brevity is not always good for
specs) but to define **which sections are mandatory** at each caveman level.
A lite spec has all the required sections at appropriate depth — it's precise,
not truncated.

---

## 6. Recommended Sequencing

### Tier 1 — High ROI, Low Risk (ship first)

1. **Subagent profiles** in SKILL.md (text only; Improvement 1)
2. **Cavekit output profiles** in feature.md (text only; Improvement 4)
3. **SKILL.md metadata enrichment** (frontmatter additions; Improvement 5)
4. **Routing expansion** in SKILL.md + argument-hint (text only; Improvement 10)
5. **SF13 cross-reference integrity** (extend validate.sh; Improvement 9 partial)
6. **`scripts/lessons-for.sh`** (40 lines; Improvement 3)

### Tier 2 — Medium ROI, Medium Effort (ship second)

7. **`scripts/promote.sh`** (60 lines; Improvement 2)
8. **SF14–SF15 scope/dependency validators** (extend validate.sh; Improvement 9)
9. **OpenSpec requirement markers** + **export-requirements.sh** (Improvement 6)

### Tier 3 — Higher Effort, Future Work

10. **`scripts/score.sh`** (Improvement 7)
11. **`scripts/interview.sh`** (Improvement 8)
12. **SF16 unit ID drift** (requires history file; Improvement 9 partial)

---

## 7. Open Questions

**Q1: Who owns the superpowers namespace?**
The `superpowers:*` identifiers in `vibe-feature` suggest an external registry.
Does adding `superpowers:spec-interview` or `superpowers:spec-audit` to the
`delegates:` list require coordinating with that registry, or is it a local
convention? If local, the cost is near-zero; if external, it needs coordination.

**Q2: Should promote.sh be in the spec skill or vibe-compound?**
`vibe-compound` owns the compound procedure; `spec` owns the merge marker
format. The promotion script sits at the boundary. Recommendation: ship it in
`spec/scripts/` (spec owns the format) and have `vibe-compound` call it.

**Q3: How does score.sh interact with warn-first policy?**
The Lesson on warn-first validation says: promote warnings to errors only after
live specs migrate. Should `score.sh` start at warn-only and promote later, or
is it always advisory (never a gate)? Recommendation: always advisory; gates
stay in `validate.sh`.

**Q4: Is the OpenSpec requirement marker syntax compatible with existing validators?**
The `<!-- spec:requirement -->` HTML comment markers would not conflict with
existing `validate.sh` checks, but they should be compatible with `SF10`
(requirement format). Validate.sh should ignore the markers when checking
RFC-2119 keywords — the prose inside the markers is the authoritative text.

**Q5: What is the right cadence for archiving vs promoting to lessons?**
Currently lessons are written during compound. With deeper compound automation
(`promote.sh`), should lessons be drafted automatically from the promotion
log? Or does the durable-rule quality of a good lesson require human judgment?
Recommendation: keep lessons human-drafted; compound automation provides the
raw material (what promoted, what was discarded) but does not draft the lesson.

---

*Research completed: 2026-06-21. Informs `spec-skill-improvements` product.md and tech.md.*
