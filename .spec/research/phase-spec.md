# Research: SPEC Phase

> How do AI coding frameworks handle design specification before implementation?

Updated: 2026-03-13

---

## Framework Analysis

### Our Spec System (Existing — The Backbone)

**What exists:**
- Full spec management system at `.agents/skills/spec/`
- Three entrypoint types: `product.md` (what/why), `tech.md` (how), `plan.md` (roadmap)
- Branch docs for deep-dives: `product-{topic}.md`, `tech-{topic}.md`, `plan-{topic}.md`
- `lessons.md` for accumulated mistakes
- Frontmatter with `type`, `scope`, `parent`, `children`, `updated`
- Cross-reference system with bidirectional links
- Validation script: checks frontmatter, naming, broken links, orphaned children
- Templates for all document types
- Strict product/tech separation: product specs have ZERO code, tech specs have ZERO UX opinions

**This is non-negotiable.** The SPEC phase always uses `/spec`. No plugin can replace it.

**What's good:**
- Strong separation of concerns (product vs tech)
- Validation catches drift and inconsistency
- Templates enforce consistent structure
- Cross-references keep documents connected
- Progressive disclosure — load only what's relevant

---

### GSD

**Spec-related patterns:**
- Spec-driven development is a core principle
- Plans reference specs as the source of truth
- But GSD doesn't have a formal spec writing system — it focuses on plans and execution
- Specs are more like "context engineering" artifacts than formal design documents

**Skip:** GSD doesn't add to our spec system

---

### Feature-Dev

**Spec-related patterns:**
- Phase 4 ("Architecture Design") produces an architecture blueprint
- `code-architect` agent generates: patterns found, architecture decision, component design, implementation map, data flow, build sequence
- Makes **decisive choices** — picks one approach and commits (doesn't present multiple options by default)
- Output is comprehensive but not split into product/tech

**Adopt pattern:** The architecture blueprint structure is useful for tech specs
**Gap:** Feature-dev doesn't separate product from tech concerns

---

### Superpowers

**Spec-related patterns:**
- Brainstorm skill produces a "design document" before coding
- No formal spec system — the design doc is more of a summary than a structured spec
- The "plan for dumb executor" principle means specs need to be extremely explicit

**Adopt pattern:** Explicitness principle — specs should be unambiguous enough for a fresh agent to follow
**Gap:** No structured spec system to adopt

---

### ADR (Architecture Decision Records)

**What it is:**
- Lightweight docs that capture architectural decisions
- Format: Title, Status, Context, Decision, Consequences
- Stored in `docs/adr/` or similar directory
- Each ADR is a single decision, not a full design doc

**Key patterns:**
- **Decision log:** Each ADR captures ONE decision with rationale
- **Status tracking:** Proposed → Accepted → Deprecated → Superseded
- **Consequences section:** Forces you to think about trade-offs
- **Chronological:** ADRs are numbered and dated

**Adopt pattern:** Our `lessons.md` serves a similar purpose (capturing learnings).
**Consider:** Adding a "Product Decisions" section to product specs (we already have this)
**Skip:** Full ADR system — our specs already capture decisions inline

---

### RFC Processes (Uber, Meta, Google)

**How it works:**
- RFC = Request For Comments — formal proposal before building
- Sections typically: Summary, Motivation, Detailed Design, Alternatives Considered, Open Questions
- Reviewed by peers before approval
- Time-boxed review period

**Key patterns:**
- **Alternatives Considered:** Forces exploration of multiple approaches
- **Open Questions:** Explicitly tracks what's unresolved
- **Peer review:** Multiple perspectives before committing
- **Approval gate:** Can't build until RFC is approved

**Already adopted:**
- Our specs have "Open Questions" sections
- Phase gates serve as approval mechanism
- Product/tech separation forces different perspectives

---

## Synthesis: Recommendations for Our SPEC Phase

### What We Already Have (No Changes Needed)

1. Full spec system with templates, validation, cross-refs
2. Product/tech separation (sacred rule)
3. Open Questions tracking
4. Lessons tracking
5. Phase gate enforcement (can't write non-spec files during SPEC phase)

### What the Orchestrator Adds

The SPEC phase in `/develop` wraps our existing `/spec` skill with orchestration:

```
1. Check if .spec/ exists → if not, run /spec setup

2. Write specs in mandatory order:
   a. product.md or product-{feature}.md (WHAT and WHY)
   b. tech.md or tech-{feature}.md (HOW)

3. Feed in research findings:
   - Reference .spec/research/*.md when writing specs
   - Ensure specs address risks identified in research
   - Resolve open questions from DISCUSS phase

4. Validate: bash validate.sh

5. Present to user: "Are these specs accurate?"
```

### Integration Points with Other Phases

| Phase | How It Uses Specs |
|-------|------------------|
| RESEARCH | Reads existing specs to understand context before exploring |
| DISCUSS | Decisions from discussion become spec content |
| SPEC | Writes/updates product and tech specs |
| PLAN | Reads specs to create implementation plan |
| IMPLEMENT | Re-reads specs before each milestone to prevent drift |
| REVIEW | Verifies implementation matches spec requirements |

### Key Design Decisions

1. **SPEC phase is not pluggable.** No plugin can replace `/spec`. It's the backbone.
2. **Orchestrator adds context, not functionality.** The `/develop` skill feeds research findings into spec writing and enforces the write order.
3. **Explicitness principle (from Superpowers).** Specs should be clear enough that a fresh agent with no conversation history can understand them.
4. **Research-informed specs.** Specs reference findings from the RESEARCH phase — they don't start from scratch.
5. **Validation is mandatory.** `validate.sh` runs before the phase can advance.

---

## Sources

- `.agents/skills/spec/SKILL.md` (local, existing)
- `.agents/skills/spec/reference/product.md` (local, existing)
- `.agents/skills/spec/reference/tech.md` (local, existing)
- [Feature-Dev code-architect agent](https://github.com/anthropics/claude-code/blob/main/plugins/feature-dev/agents/code-architect.md)
- [Superpowers (GitHub)](https://github.com/obra/superpowers)
