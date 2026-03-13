# Research: SPEC Phase

> How do AI coding frameworks handle design specification before implementation?

Updated: 2026-03-13

---

## Framework Analysis

### Our Spec System (Existing — The Backbone)

**What exists:**
- Full spec management system at `.agents/skills/spec/`
- Six document types: product entrypoint, tech entrypoint, plan entrypoint, lessons, branch docs, sub-plans
- Frontmatter-based metadata (`type`, `scope`, `parent`, `children`, `updated`) enabling graph traversal
- Strict product/tech separation with explicit style rules and good/bad examples
- Cross-reference discipline: bidirectional links between parents, branches, and siblings
- Validation script: checks naming, frontmatter, broken links, orphaned children
- Templates for all document types
- Progressive disclosure: load entrypoints first, branch docs only when relevant

**This is non-negotiable.** The SPEC phase always uses `/spec`. No plugin can replace it.

**What our system does better than everyone else:**
1. **Hard product/tech separation with enforcement** — No other framework has explicit style rules with good/bad examples for what belongs in each doc type
2. **Progressive disclosure architecture** — Entrypoint → branch doc hierarchy with navigation rules. Directly addresses the "curse of instructions" (AI performance degrades with too many requirements in one prompt)
3. **Lessons as first-class infrastructure** — The only framework that treats error memory as spec infrastructure
4. **Automated structural validation** — Cross-reference checking, naming, orphaned children
5. **Bidirectional cross-references** — Creates a navigable graph, not a flat file dump
6. **Frontmatter-based metadata** — Enables programmatic querying of the spec graph

---

### GSD (Get Stuff Done)

**Spec-related patterns:**
- Externalizes state into discrete files: `PROJECT.md`, `REQUIREMENTS.md`, `ROADMAP.md`, `CONTEXT.md`, `STATE.md`, `MILESTONE.md`, `DISCOVERY.md`, `RESEARCH.md`, `UAT.md`, `VALIDATION.md`
- `PLAN_N_M.md` files are XML-formatted **executable prompts**, not documentation — each is 2-3 tasks fitting 50% of a context window
- Does NOT enforce product vs. tech separation
- `CONTEXT.md` per milestone archives what was known at decision time — temporal snapshots
- `STATE.md` as explicit current-state file (richer than our `.spec/.phase`)

**What GSD does better than us:**
- XML task format provides unambiguous machine-parseable execution units (our plan tasks are prose checkboxes)
- `CONTEXT.md` per milestone captures temporal state that our living-document approach loses on update
- `STATE.md` is richer than our binary `.phase` file

**Skip:** XML format, monolithic requirements doc
**Consider:** Temporal context snapshots, richer state tracking

---

### Feature-Dev

**Spec-related patterns:**
- `code-architect` agent produces architecture blueprints with mandatory sections:
  1. Patterns & Conventions Found (with **`file:line` references**)
  2. Architecture Decision (chosen approach + rationale + trade-offs)
  3. Component Design (file path, responsibilities, dependencies, interfaces)
  4. Implementation Map (specific files to create/modify)
  5. Data Flow (entry-to-output with transformation steps)
  6. Build Sequence (phased checklist)
  7. Critical Details (error handling, state management, testing, performance, security)
- Makes **decisive choices** — picks one approach and commits, no hedging
- Proposals are ephemeral (session-only), not persistent files

**What feature-dev does better than us:**
- `file:line` references make findings directly verifiable
- Build sequence automatically derived from architecture proposal
- Decisive single-approach commitment prevents wishy-washy "we could do A or B" specs

**Adopt pattern:** `file:line` references in tech specs, decisive commitment language
**Gap:** No product/tech separation, no persistence

---

### Superpowers

**Spec-related patterns:**
- Brainstorm skill uses **Socratic questioning** to elicit a spec from conversation
- Presents design in digestible chunks for approval before implementation
- In v5.0: **subagent reviews planning docs for completeness** — catches TBD sections left by main agent
- Skills trigger automatically on task recognition, no explicit invocation needed
- TDD enforcement: failing tests derived from specs must exist before code

**What Superpowers does better than us:**
- Socratic elicitation — surfaces ambiguity through questions rather than writing specs from a description
- Automated completeness checking (not just structural validation)
- Spec → test derivation: test stubs come directly from spec content

**Adopt pattern:** Completeness checking as a spec gate
**Gap:** No structured persistent spec system

---

### BMAD Method

**What it is:**
- Uses specialized personas (Analyst, PM, Architect, Product Owner, Scrum Master, Developer, QA)
- Each persona produces a document that feeds the next: Project Brief → PRD → Architecture Design → User Stories → Implementation
- `project-context.md` acts as a constitution guiding all agent decisions

**What BMAD does better than us:**
- Explicit role separation during spec creation (PM writes requirements without knowing implementation)
- **User stories with acceptance criteria** bridge product → implementation in a directly testable way
- `project-context.md` constitution concept is stronger than `lessons.md` for cross-cutting conventions

**Adopt pattern:** User stories / acceptance criteria format for testable requirements
**Skip:** Full persona system — too ceremonial for solo developers

---

### ADRs (Architecture Decision Records)

**Format:** Title, Status (Proposed → Accepted → Deprecated → Superseded), Context, Decision, Consequences

**Relationship to our system:**
Our tech specs record decisions in "Key Patterns" and "Risks & Mitigations" sections. But these are embedded, not standalone per-decision records. The ADR pattern adds:
- **Superseded tracking:** Marking old decisions and pointing to replacements
- **Single-decision focus:** Each ADR is one decision with rationale
- **Status lifecycle:** Decisions can be deprecated/superseded

**Our `lessons.md` partially covers this.** Full ADR system is overkill for our use case.

---

### Google Design Docs / Engineering RFCs

**Google's design doc anatomy:**
1. Context & Scope — objective facts, no opinions
2. Goals & Non-Goals — explicit non-goals
3. The Design — emphasizing trade-offs, system-context diagrams, API sketches
4. **Alternatives Considered** — rejected designs with trade-off rationale
5. **Cross-Cutting Concerns** — security, privacy, observability

**Uber's ERD evolution:** Scaled from informal RFCs to **tiered templates** — lightweight for team-scoped changes, heavyweight for org-wide changes.

**What Google/Uber do better than us:**
- Alternatives Considered as a **mandatory** section (prevents re-litigation of closed decisions)
- Cross-cutting concerns as their own section (security, privacy, observability)
- **Tiered templates** — spec depth matches work complexity (we use same structure for bug fixes and major features)

---

### Cursor/Windsurf Rules

**What they are:**
- `.cursorrules` / `.windsurfrules` are behavioral contracts for AI agents
- Imperative: "always do X, never do Y"
- Our `CLAUDE.md` serves the same role

**Key insight:** Specs are descriptive ("the system uses pattern X"), rules are imperative ("always use pattern X"). These are complementary layers. Tool like Specifys.ai exports PRD → Tech Spec → `.cursorrules` — showing demand for a spec-to-rules bridge.

**Gap:** No mechanism for spec content to update `CLAUDE.md` conventions

---

### AI-Specific Spec Best Practices

From [Addy Osmani's research](https://addyosmani.com/blog/good-spec/):
- Use meaningful subheadings, code blocks, tables, summaries — LLMs parse structured text better
- One real code snippet beats three paragraphs describing it
- Include executable commands verbatim (`npm test -- --coverage`, not "run the tests")
- Define three-tier boundaries: Always / Ask First / Never
- Break specs into modular cross-referenced documents (performance drops with too many requirements in one prompt)
- Include verification steps in the spec

**Our progressive disclosure model directly addresses the "curse of instructions."** This is a genuine architectural advantage.

---

## Identified Gaps (8 Total)

### Gap 1: No "Alternatives Considered" in Tech Specs
**Impact:** Future agents re-examine rejected approaches because the spec doesn't explain why they were ruled out.
**Recommendation:** Add optional `## Alternatives Considered` section to tech spec template.

### Gap 2: No Tiered Spec Scope (Lightweight vs. Heavyweight)
**Impact:** Same structure for 2-line bug fixes and multi-month redesigns. Overhead is disproportionate for small changes.
**Recommendation:** Define two tiers:
- **Lightweight:** Inline section in `plan.md` + brief `tech.md` update. For < 1 session, no architectural impact.
- **Full:** Current product-{topic}.md + tech-{topic}.md + plan-{topic}.md. For 3+ milestones or architectural decisions.

### Gap 3: No Spec Completeness Gate (Only Structural Validation)
**Impact:** Specs with TBD markers or empty sections pass validation and enter PLAN phase.
**Recommendation:** Add content-level validation: check for empty sections, TBD markers, missing required sections, unresolved Open Questions. Superpowers v5 added this and called it "a dramatic improvement."

### Gap 4: No Cross-Cutting Concerns Section
**Impact:** Security and observability considerations discovered during implementation rather than designed in.
**Recommendation:** Add optional `## Cross-Cutting Concerns` section to tech spec template (Security, Privacy, Observability, Error Handling Strategy).

### Gap 5: No Temporal Context Snapshots
**Impact:** History of why a decision was made at a particular point is lost when specs are updated.
**Recommendation:** When a tech spec section is materially revised, add a brief `> Changed from: [old approach] because: [reason]` note.

### Gap 6: No Acceptance Criteria / Test Derivation Bridge
**Impact:** Test coverage depends on implementer's interpretation, not the spec itself.
**Recommendation:** Add optional `## Acceptance Criteria` section (given/when/then format) to branch docs. Plan should map each criterion to a test case.

### Gap 7: No Spec-to-Rules Export Path
**Impact:** Conventions in `tech.md` don't propagate to `CLAUDE.md` behavioral layer.
**Recommendation:** During REVIEW, check if new conventions should be added to `CLAUDE.md`.

### Gap 8: `.phase` File Is Too Simple
**Impact:** On resumption, agent knows phase but not milestone, relevant files, or last-completed task.
**Recommendation:** Extend to structured JSON: current phase, milestone, last task, relevant spec files to load.

---

## Synthesis: Recommendations for Our SPEC Phase

### What We Already Have (No Changes Needed)

1. Full spec system with templates, validation, cross-refs
2. Product/tech separation (sacred rule)
3. Open Questions tracking
4. Lessons tracking
5. Phase gate enforcement

### What the Orchestrator Adds

```
1. Check if .spec/ exists → if not, run /spec setup

2. Write specs in mandatory order:
   a. product.md or product-{feature}.md (WHAT and WHY)
   b. tech.md or tech-{feature}.md (HOW)

3. Feed in research findings:
   - Reference .spec/research/*.md when writing specs
   - Ensure specs address risks identified in research
   - Resolve open questions from DISCUSS phase

4. Completeness check (NEW — from Superpowers):
   - Check for empty sections, TBD markers
   - Verify Open Questions that should be resolved are resolved

5. Validate: bash validate.sh

6. Present to user: "Are these specs accurate?"
```

### Integration Points with Other Phases

| Phase | How It Uses Specs |
|-------|------------------|
| RESEARCH | Reads existing specs to understand context before exploring |
| DISCUSS | Decisions from discussion become spec content |
| SPEC | Writes/updates product and tech specs |
| PLAN | Reads specs to create implementation plan |
| IMPLEMENT | Re-reads specs before each milestone to prevent drift |
| REVIEW | Verifies implementation matches spec requirements; updates CLAUDE.md if new conventions found |

### Key Design Decisions

1. **SPEC phase is not pluggable.** No plugin can replace `/spec`. It's the backbone.
2. **Orchestrator adds context, not functionality.** Feeds research findings in, enforces write order.
3. **Explicitness principle (from Superpowers).** Specs should be unambiguous for a fresh agent.
4. **Research-informed specs.** Reference RESEARCH findings, don't start from scratch.
5. **Validation is mandatory.** `validate.sh` runs before advancing.
6. **Completeness checking (NEW).** Content-level validation, not just structural.

### Gaps to Address During Implementation

| Gap | Priority | When to Address |
|-----|----------|----------------|
| Alternatives Considered section | P1 | M1 (update tech spec template) |
| Tiered spec scope | P2 | M2 (routing logic in /develop) |
| Completeness gate | P1 | M2 (add to SPEC phase exit criteria) |
| Cross-cutting concerns section | P2 | M1 (update tech spec template) |
| Temporal context snapshots | P3 | Future (nice-to-have) |
| Acceptance criteria bridge | P2 | M2 (add to product branch template) |
| Spec-to-rules export | P3 | M4 (REVIEW phase enhancement) |
| Rich .phase file | P1 | M1 (extend phase tracking) |

---

## Sources

- `.agents/skills/spec/SKILL.md` (local, existing)
- `.agents/skills/spec/reference/product.md` (local, existing)
- `.agents/skills/spec/reference/tech.md` (local, existing)
- [GSD Framework](https://github.com/gsd-build/get-shit-done)
- [GSD v2](https://github.com/gsd-build/gsd-2)
- [Beating context rot with GSD - The New Stack](https://thenewstack.io/beating-the-rot-and-getting-stuff-done/)
- [Feature-Dev code-architect](https://github.com/anthropics/claude-code/blob/main/plugins/feature-dev/agents/code-architect.md)
- [Superpowers (GitHub)](https://github.com/obra/superpowers)
- [Superpowers v5 (blog)](https://blog.fsck.com/2026/03/09/superpowers-5/)
- [BMAD Method](https://github.com/bmad-code-org/BMAD-METHOD)
- [Design Docs at Google](https://www.industrialempathy.com/posts/design-docs-at-google/)
- [Software Engineering RFCs — Pragmatic Engineer](https://newsletter.pragmaticengineer.com/p/software-engineering-rfc-and-design)
- [ADR Templates](https://adr.github.io/adr-templates/)
- [How to Write a Good Spec for AI Agents — Addy Osmani](https://addyosmani.com/blog/good-spec/)
- [AI Coding Rules for Cursor & Windsurf](https://deeplearning.fr/ai-coding-assistant-rules-for-windsurf-and-cursor/)
- [Spec-Driven Development Map — Vishal Mysore](https://medium.com/@visrow/spec-driven-development-is-eating-software-engineering-a-map-of-30-agentic-coding-frameworks-6ac0b5e2b484)
