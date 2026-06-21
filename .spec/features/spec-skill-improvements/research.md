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

### 3d. OpenSpec — Interoperability, Not Adoption

No canonical "OpenSpec" standard exists at time of writing. The useful framing
is the pattern OpenAPI established: a machine-readable structure sitting
*alongside* prose, not replacing it, enabling tooling to consume requirements
without parsing natural language.

The spec skill already has the right instinct: R-ID conventions, `### Requirement:`
headings, GWT scenarios. The interoperability gap is that these are parseable
only by convention, not by a declared marker. Adding opt-in `<!-- spec:requirement
id="R1" strength="SHALL" -->` HTML-comment markers makes R-IDs addressable by
external tools (Linear, CI dashboards, test-traceability reporters) without
forcing all authors to adopt the overhead.

**Key principle:** spec prose stays canonical; structured markers are a
machine-readable shadow of the same content, not a replacement. Export tooling
reads markers when present, falls back to heuristic heading parsing when absent.

### 3e. The Right Interoperability Model — Spec as Framework, Superpowers as Executor

This is the most important cross-cutting insight, and it corrects a trap
the improvement catalog can fall into.

**Wrong model:** build tools inside the spec skill that do things superpowers
already does. Examples of the trap:
- Build `interview.sh` → re-implements `superpowers:brainstorming`
- Build `score.sh` as quality assessment → re-implements what `superpowers:verification-before-completion` does when given spec artifacts

**Right model:**

```
Spec skill  =  FORMAT + CONSTRAINTS + VALIDATION SCRIPTS
Superpowers =  EXECUTION WITHIN THAT FORMAT
```

The spec skill's job for each authoring phase is to:
1. Supply the **constraint document** (the template, the format rules, the
   example, the validation criteria) — this is what feature.md, strategy.md,
   and the reference guides already do
2. Name the **executor** — which `superpowers:*` reads those constraints and
   does the actual work

| Authoring step | Spec skill provides | Executor |
|---|---|---|
| WHAT interview (step 2) | `feature.md § Interview for WHAT` as constraint context | `superpowers:brainstorming` |
| HOW sketching (step 4) | `reference/tech.md` + feature template as constraint | `code-explorer`, `code-architect` |
| Plan units (step 5) | `reference/plan.md` + stable ID rules as constraint | `superpowers:writing-plans` |
| Validation | `validate.sh` script (deterministic, not a superpower) | spec skill itself |
| Compound promotion | `promote.sh` script (deterministic, not a superpower) | spec skill itself |
| Quality assessment | `score.sh` metrics (spec-specific; superpowers can't derive RFC-2119 compliance) | `superpowers:verification-before-completion` receives score output as context |

The test for any proposed improvement: *is this something the spec skill uniquely knows (file format, structural rule, domain-specific metric) or is it general-purpose capability a superpower already handles?*

- Unique to spec: R-ID traceability, merge marker validation, stale-link detection,
  RFC-2119 compliance checking, frontmatter schema, Scope table parsing
- Covered by superpowers: dialogue for requirement elicitation, plan unit
  decomposition, implementation execution, code review, quality assessment framing

**Practical implication for this feature:** the roles section in SKILL.md should
express explicit `superpowers:` delegation for every authoring step, not build
competing tools. `spec-interviewer` is not a CLI — it is a named context that
routes to `superpowers:brainstorming` with `feature.md § Interview for WHAT`
injected as the constraint document.

### 3f. Specialized Subagents — Role-Based Architecture

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

**`spec-interviewer`** — WHAT phase delegation context
- Supplies `feature.md § Interview for WHAT` as constraint document for `superpowers:brainstorming`
- Adds RFC-2119 vocabulary reminder (SHALL vs MUST vs SHOULD vs MAY) and GWT scaffolding
- Does NOT build a custom interview CLI — `superpowers:brainstorming` is the dialogue engine
- Outputs: `features/<name>/product.md`

**`spec-architect`** — HOW phase delegation context
- Supplies `reference/tech.md` template and merge-marker convention as constraint
- Delegates to `code-explorer` (trace) and `code-architect` (sketch approach)
- Does NOT trace code itself — `code-explorer` reads the repo; the role frames the output contract
- Outputs: `features/<name>/tech.md`

**`spec-planner`** — Plan units delegation context
- Supplies `reference/plan.md` + stable-ID rules as constraint document for `superpowers:writing-plans`
- Provides the `<name>/n` ID convention, Requirements Trace table format, and verification row template
- Does NOT write plan units itself — `superpowers:writing-plans` does; the role frames the format
- Outputs: `features/<name>/plan.md`

**`spec-auditor`** — Validation role (spec-unique; no superpower substitute)
- Runs `validate.sh` (deterministic, spec-format-aware; superpowers can't derive this)
- Interprets output and provides fix suggestions per warning
- Optionally runs `score.sh` and passes structured metrics to `superpowers:verification-before-completion`
  as additional context for quality assessment
- Outputs: structured validation report to stdout

**`spec-compactor`** — Compound role (spec-unique for promotion; lesson drafting delegates)
- Runs `promote.sh` (deterministic merge-marker extraction; superpowers can't know the format)
- Identifies what "graduates" vs "stays" using the `<!-- merge -->` convention
- For the lesson entry: supplies pattern/rule/tags template as constraint, then delegates to
  `superpowers:finishing-a-development-branch` for narrative polish
- Outputs: merged root specs; draft lesson entry for human review

**Implementation:** Named sections in the main SKILL.md (not separate files), callable as
`/spec interview`, `/spec audit`, `/spec promote`. Each section is a delegation context
(constraint doc + named executor), not a custom tool body.

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

### Improvement 7: `scripts/score.sh` — Spec-Specific Quality Metrics

**What:** `bash .agents/skills/spec/scripts/score.sh` emits machine-readable
quality metrics for the `.spec/` tree across dimensions that `validate.sh`
doesn't cover and that superpowers cannot derive without spec-format knowledge:

| Dimension | Measured how |
|---|---|
| RFC-2119 compliance | % of requirement blocks containing SHALL/MUST/SHOULD/MAY |
| GWT coverage | % of requirements with ≥1 scenario; % of scenarios with Given+When+Then |
| R-ID traceability | % of R-IDs in product.md cited in plan.md |
| Unit verification completeness | % of plan units with non-empty verification row |
| `updated:` freshness | Days since last edit per file |

Emits structured JSON (for CI ingestion) and a human-readable summary.

**Interoperability note:** `score.sh` is NOT a quality assessor — it is a
metrics probe. Quality assessment is done by `superpowers:verification-before-completion`
receiving the score output as structured context:

```
bash .agents/skills/spec/scripts/score.sh > /tmp/spec-score.json
# Agent then: "Review these spec quality metrics and surface the most critical gaps"
# → superpowers:verification-before-completion reads the JSON and assesses
```

`score.sh` knows RFC-2119 and the R-ID format; `verification-before-completion`
knows how to assess and communicate gaps. Neither duplicates the other.

**Effort:** Medium (80-line script). **ROI:** Medium (useful for multi-feature
repos and CI; less critical for dogfood-scale use).

---

### Improvement 8: Constraint Context Documents for Superpowers Delegation

**What** (revised from "build `interview.sh`")**:** Rather than building a
custom interview CLI that duplicates `superpowers:brainstorming`, add explicit
**constraint context documents** to `feature.md` and `strategy.md` that an
agent injects into the superpowers delegation call.

**The wrong approach (what NOT to build):**
```bash
# A 200-line bash interview script that asks questions in a terminal loop
# → duplicates superpowers:brainstorming; lower quality than the real thing
bash .agents/skills/spec/scripts/interview.sh my-feature
```

**The right approach:**
```
Agent at feature.design step 2:
  "I'm about to run the WHAT interview. Constraint document:"
  [injects feature.md § Interview for WHAT: RFC-2119 rules, GWT format,
   Scope table format, rigor gate criteria]
  → delegates to superpowers:brainstorming with those constraints active

Agent at feature.plan step 5:
  "I'm about to write plan units. Constraint document:"
  [injects reference/plan.md: stable-ID convention, Requirements Trace
   table format, verification row template, human-gate reminder]
  → delegates to superpowers:writing-plans with those constraints active
```

**What to actually build:** Short **context snippets** in `feature.md` and
`strategy.md` formatted as ready-to-inject agent briefings — one paragraph per
step that names the executor, the constraint document section to inject, and
the validation criteria the output must pass.

**Why this is better:**
- `superpowers:brainstorming` is purpose-built for Socratic dialogue; a bash
  CLI cannot match its quality
- `superpowers:writing-plans` understands decomposition, dependency mapping,
  and estimation; a custom script would be a poor imitation
- The spec skill's constraint documents improve over time without touching
  the executor — separation of format from execution
- Zero implementation cost compared to a 200-line bash script

**Effort:** Small (text additions to feature.md). **ROI:** High (applies to
every feature, every session; superpowers quality > custom CLI quality).

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

### Theme A: Spec as framework, superpowers as executor

The most important structural principle. The spec skill owns FORMAT + CONSTRAINTS
+ SPEC-SPECIFIC SCRIPTS. Superpowers own EXECUTION within that format. Any
proposed improvement must pass this test: *does the spec skill uniquely know
this, or does a superpower already handle it?*

Spec-unique work: structural validation, R-ID traceability, merge marker
extraction, stale-link detection, RFC-2119 compliance metrics. Superpower
work: dialogue, plan writing, code review, quality assessment framing.
The spec skill provides constraint documents; superpowers read them.

### Theme B: Automate what is mechanical; delegate what requires judgment

The best improvements (promote.sh, lessons-for.sh) automate the mechanical,
error-prone parts (merge marker extraction, tag matching) that are currently
documented-but-manual. They do NOT attempt to automate judgment calls
(requirement quality, scope negotiation, lesson selection) — those stay with
superpowers and the human.

### Theme C: Progressive enrichment over mandatory overhead

OpenSpec markers and spec scoring are high value only as optional enrichment.
The spec skill's power comes from low barrier to entry; improvements must not
raise the floor.

### Theme D: Roles as delegation contexts, not separate tools

Roles (spec-interviewer, spec-planner, spec-compactor) are named sections in
SKILL.md that define the constraint document to inject and the superpower to
call — not custom CLI implementations. Single-file discipline preserved;
role-specific dispatch enabled.

### Theme E: Cavekit compatibility is about output contracts, not truncation

Defining which sections are mandatory at each caveman level is not compression
— it is precision. A lite spec has all required sections at the right depth.
The spec skill's output profiles tell agents WHAT to produce at each level;
superpowers within the level produce it correctly.

---

## 6. Recommended Sequencing

### Tier 1 — High ROI, Low Risk (ship first)

1. **Role profiles** in SKILL.md as delegation contexts (text only; Improvement 1)
2. **Cavekit output profiles** in feature.md (text only; Improvement 4)
3. **Superpowers constraint context snippets** in feature.md per step (text; Improvement 8 revised)
4. **SKILL.md metadata enrichment** with `delegates:` (frontmatter additions; Improvement 5)
5. **Routing expansion** in SKILL.md + argument-hint (text only; Improvement 10)
6. **`scripts/lessons-for.sh`** (40 lines; Improvement 3)
7. **SF13 cross-reference integrity** (extend validate.sh; Improvement 9 partial)

### Tier 2 — Medium ROI, Medium Effort (ship second)

8. **`scripts/promote.sh`** (60 lines; Improvement 2)
9. **SF14 scope conflict detector** (extend validate.sh; Improvement 9)
10. **OpenSpec requirement markers** + **export-requirements.sh** (Improvement 6)

### Tier 3 — Deferred (higher effort, lower urgency)

11. **`scripts/score.sh`** (spec-specific metrics; Improvement 7)
12. **SF15–SF16** validators (cross-feature dep check + unit drift; Improvement 9)

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
