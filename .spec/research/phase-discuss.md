# Research: DISCUSS Phase

> How do AI coding frameworks handle clarification, scope alignment, and requirement refinement before speccing?

Updated: 2026-03-13

---

## Framework Analysis

### Superpowers

**How it works:**
- `/superpowers:brainstorm` skill uses a nine-step sequential workflow with a hard gate: no implementation skill may be invoked until user approves a design
- Questions asked **one at a time** with preference for multiple-choice over open-ended
- Performs **autonomous codebase reconnaissance before asking any questions** — questions are informed by what actually exists
- Explicitly requires exploring 2-3 approaches with trade-offs before settling, always leading with a recommendation
- Spec-reviewer subagent adds a second pass (max 5 iterations) before user sees design — internal adversarial loop
- Uses **divergent thinking** (AI's strength) paired with **convergent thinking** (human's strength)
- Follows the **Double Diamond model:** explore → define → ideate → refine
- 29k+ GitHub stars, 143k+ installs — widely validated approach

**Scope handling:** If the request spans multiple independent subsystems, flag it and decompose into sub-projects before continuing. Each sub-project gets its own session.

**Transition:** After user approves the design, the only permitted next step is invoking the `writing-plans` skill. No direct path to code.

**Example:**
```
Agent: "I've reviewed the codebase. You have three auth-related
       files already. Before I design the OAuth flow, one question:
       Should this replace the current session-cookie auth,
       or run alongside it?
       (A) Replace — single auth system going forward
       (B) Alongside — OAuth as an additional option
       (C) Unclear — let's discuss"
```

**Adopt as plugin:** Brainstorm skill for the DISCUSS phase
**Built-in equivalent:** Structured multi-perspective questioning with reconnaissance-first pattern

---

### GSD (Get Stuff Done)

**How it works:**
- `/gsd:discuss-phase` command performs **domain analysis first**, then generates 3-4 "gray areas" specific to the type of thing being built:
  - Visual feature → layout, density, interactions, empty states
  - API/CLI → response format, flags, error handling, verbosity
  - Auth feature → OAuth providers, token storage, session strategy
- Output is `CONTEXT.md`, documenting **locked decisions** (user-specified, non-negotiable) vs. **flexible decisions** (AI determines)
- Batch mode (`--batch`) collapses questions into grouped sets for speed
- Hard boundary: discuss phase clarifies HOW within fixed scope — never WHETHER to add features
- Scope expansion attempts actively redirected to a "deferred ideas" list

**Key insight:** The locked/flexible distinction prevents AI from asking about things it should just decide. Users own non-functional preferences (which database? which auth provider?). AI owns implementation choices (which abstraction? how to structure the service?).

**Transition:** `CONTEXT.md` feeds directly into `/gsd:plan-phase` as authoritative input. The discuss phase output is an artifact, not a conversation transcript.

**Adopt pattern:** Locked/flexible distinction, gray area detection, scope deflection
**Skip:** The full discuss-per-phase loop — we discuss once before speccing

---

### Feature-Dev

**How it works:**
- Dedicated **Phase 3: Clarifying Questions** within 7-phase workflow (Discovery → Codebase Exploration → Clarifying Questions → Architecture Design → Implementation → Quality Review → Summary)
- After codebase exploration in Phase 2, Phase 3 identifies underspecified aspects as an **organized list**
- Explicitly waits for user answers before proceeding — marked "CRITICAL: DO NOT SKIP"
- Categories: edge cases, error handling, integration points, backward compatibility, performance

**Key pattern:** Two rounds of clarification — once before research (Phase 1), once after with codebase context (Phase 3). Context-informed questions are meaningfully better than generic ones.

**Example:**
```
"Before designing the architecture, I need to clarify:
1. OAuth provider: Which OAuth providers? (Google, GitHub, custom?)
2. User data: Store OAuth tokens or just user profile?
3. Existing auth: Replace current auth or add alongside?
4. Sessions: Integrate with existing session management?
5. Error handling: How to handle OAuth failures?"
```

**Adopt as plugin:** Context-informed clarifying questions (post-research)
**Built-in equivalent:** Structured AskUserQuestion with categories

---

### BMAD Method

**How it works:**
- **Analyst Agent ("Mary")** as dedicated requirements persona conducts structured discovery: goals, constraints, edge cases, success metrics
- PRD built section-by-section interactively — not generated in one pass
- Explicit "Open Questions" list in PRD for gaps requiring further clarification
- MVP Scope section with explicit in/out-of-scope

**Standout: Advanced Elicitation techniques** — a menu of named re-examination techniques applied after initial analysis:

| Technique | What It Does |
|-----------|-------------|
| **Pre-mortem** | "Assume the project failed — work backward to find why" |
| **First Principles** | Strip assumptions, rebuild from ground truth |
| **Inversion** | "How would we guarantee failure?" then avoid those |
| **Red Team vs Blue Team** | Attack your own work, then defend it |
| **Socratic Questioning** | Challenge every claim with "why?" and "how do you know?" |
| **Constraint Removal** | Drop all constraints, see what changes, add back selectively |
| **Stakeholder Mapping** | Re-evaluate from each stakeholder's perspective |
| **Six Thinking Hats** | Facts / feelings / cautions / benefits / creativity / process |

These are offered at decision points, not applied automatically. Standard questions surface *known* ambiguities; advanced elicitation surfaces *unknown* assumptions.

**Adopt pattern:** Pre-mortem for high-risk features, elicitation techniques as optional augmentation
**Skip:** Full persona system — too ceremonial for solo developers

---

### Spec Kit

**How it works:**
- Mandates `[NEEDS CLARIFICATION: specific question]` markers anywhere a prompt is underspecified
- Maximum **three clarification markers per spec** — forces decisions rather than deferral chains
- `/speckit.clarify` and `/speckit.analyze` commands for explicit clarification or consistency checks
- `constitution.md` provides persistent non-negotiable principles as a constraint lens

**Adopt pattern:** Inline clarification markers in specs, max limit to force decisions
**Skip:** Full Spec Kit infrastructure

---

### Design Thinking (General Pattern)

**Double Diamond model applied to AI coding:**

```
Phase 1: Discover          Phase 2: Define
  ╱ diverge ╲               ╱ diverge ╲
 /  explore   \             /  ideate    \
/  multiple    \           /  multiple    \
\  interpretations/       \  solutions   /
 \  of problem  /          \  to problem /
  ╲ converge ╱              ╲ converge ╱
   problem                   solution
   definition                selection
```

**Key insight:** Separate problem from solution. First understand what you're solving (diverge on problem space), then how (diverge on solution space). Human as convergence engine — AI generates options, human picks.

**Adopt built-in:** Diverge-converge structure

---

### Cursor / Windsurf / Canvas / Artifacts

**Cursor:** Relies on user to tag and add context manually — de facto clarification via file attachment. Asks permission before multi-file changes. No structured discuss phase.

**Windsurf:** Cascade makes autonomous assumptions and reads codebase to infer context. Faster, less controllable. No structured discussion.

**Canvas/Artifacts:** Post-generation refinement, not pre-spec clarification. Philosophically opposite to the pre-spec approach.

**Skip:** No structured patterns worth adopting for DISCUSS phase

---

## Cross-Cutting Patterns

### The Reconnaissance-First Pattern (Superpowers, Feature-Dev, GSD)

All three frameworks that handle clarification well share one pattern: **codebase exploration precedes questioning**. The agent reads what exists before asking what's needed. Effects:
1. Questions are specific to the actual codebase, not generic
2. Many "questions" are answered by the code itself and never need to be asked

**Rule:** Do not ask users what already has a definitive answer in the codebase.

### The One-Question Discipline (Superpowers)

Only framework that explicitly enforces **one question at a time**. Research supports this — presenting a list creates a "questionnaire feel" that breaks conversational dynamic and discourages thoughtful answers. Single questions create dialogue; lists create checkbox completion.

**Tension:** GSD's batch mode allows grouping for users who prioritize speed over depth.

### The Locked/Flexible Distinction (GSD)

Two categories: **locked decisions** (user-owned) and **flexible decisions** (AI-owned). Prevents the anti-pattern where AI asks about things it should just decide and users answer questions they don't care about.

### Scope as Separate Concern (GSD, BMAD)

Discussion clarifies HOW within fixed WHAT. If the WHAT is wrong, that's a separate conversation. Scope expansion is redirected (GSD) or formalized as in/out-of-scope (BMAD).

### The LLM Bias Problem

LLMs are trained to answer, not to ask. RLHF reward signals favor "complete-looking" responses over clarifying questions. This creates structural bias: AI makes "reasonable" assumptions silently when requirements are underspecified.

**Mitigation across frameworks:**
- Hard gates with explicit "wait for user" instructions (Feature-Dev)
- Phase state tracking that blocks writing until clarification completes (Superpowers, our framework)
- Explicit prompting: "List your uncertainties before generating code" (VibeVibe pattern)

---

## Synthesis: Recommendations for Our DISCUSS Phase

### Built-in Default Provider

```
1. Frame the problem (diverge):
   - Read existing specs + codebase (reconnaissance-first)
   - "Here are N ways to interpret this request: [A, B, C]"
   - "Which interpretation matches your intent?"

2. Surface gray areas (informed by codebase):
   - Scope: "Should this apply to X, Y, or both?"
   - Constraints: "Any performance/compatibility requirements?"
   - Priorities: "If we can't do everything, what's most important?"
   - Edge cases: "What should happen when [unusual condition]?"
   - Only ask questions that cannot be answered by reading code/specs

3. Clarify approach (converge):
   - "Based on your answers, here's my understanding: [summary]"
   - Locked decisions (user-specified, non-negotiable)
   - Flexible decisions (AI determines during SPEC)
   - Deferred ideas (out of scope this session)
   - "Anything I'm missing before I write specs?"

4. Persist decisions:
   - Write key decisions to .spec/research/agreements.md
   - These become inputs to the SPEC phase
```

### Question Generation Heuristics

Apply these filters in order:
1. **Already answered?** Read specs and codebase first. Skip if answerable.
2. **Decision-reversible?** If wrong choice can be changed later cheaply, skip.
3. **User-owned?** Preferences, constraints, stakeholder requirements → ask. Implementation choices → don't ask.
4. **Would this change the spec?** If two different answers produce meaningfully different specs, ask. Otherwise skip.
5. **Enumerable options?** If yes, offer multiple choice. If no, ask open-ended.

| Category | Ask About | Skip |
|----------|-----------|------|
| Scope | What is explicitly out of scope? | What features to add |
| Integration | Which existing systems must this touch? | How to integrate (AI decides) |
| Constraints | Non-negotiable tech, legal, or business limits | Performance targets (can infer) |
| Audience | Who uses this? What's their context? | How the UI looks (derive from product.md) |
| Edge cases | What should happen when X fails? | Happy path (usually clear) |
| Success criteria | How will we know this works? | Implementation tests (AI writes these) |

### Ambiguity Detection Signals

**Linguistic:** Vague verbs ("improve," "optimize," "better"), undefined pronouns ("it," "them"), missing actors ("users should..."), missing constraints ("make it fast"), implicit choices ("add login")

**Structural:** Feature request with no success criterion, touches 3+ subsystems, contradicts something in specs, assumes something untrue about codebase

### Scope Negotiation Protocol

1. Name the scope boundary explicitly
2. Confirm: "Should we scope to X only, or discuss scoping first?"
3. If deferring: add to deferred ideas list in research.md
4. Never silently expand scope

### Transition Criteria to SPEC

Discussion is complete when:
- All questions with decision-reversing impact are answered
- Scope is explicit (what's in, what's deferred)
- At least one success criterion is articulated
- Locked/flexible distinction is documented
- User has confirmed the summary is accurate

### Plugin Enhancements

| Plugin | Enhancement |
|--------|-------------|
| **superpowers** | Full brainstorm skill with Socratic questioning, one-at-a-time discipline, diverge-converge, design document output |
| **feature-dev** | Context-informed clarifying questions based on codebase exploration findings |

### Key Design Decisions

1. **Separate phase from RESEARCH.** Research is about understanding the codebase. DISCUSS is about aligning with the user on what to build. Different goals.
2. **Reconnaissance-first.** Read code and specs before asking anything. Never ask what's already answerable.
3. **Locked/flexible distinction (from GSD).** Separate user-owned decisions from AI-owned implementation choices.
4. **Structured, not freeform.** Built-in uses AskUserQuestion with categories (scope, constraints, priorities, edge cases). Not open-ended chat.
5. **Persist outcomes.** Discussion results written to files — survive compaction, inform SPEC phase.
6. **Diverge before converging.** Present multiple interpretations before asking user to pick one. Surfaces hidden assumptions.
7. **Scope deflection (from GSD).** Scope expansion redirected to deferred list, never silently absorbed.
8. **Advanced elicitation optional (from BMAD).** Pre-mortem / inversion for high-risk features. Not default.

### Interaction Examples

**Simple feature (Feature-Dev style):**
```
Agent: [After reading codebase]
I found your existing auth system uses session cookies (src/auth/session.ts).
One question before I write the OAuth spec:

Should OAuth be an *alternative* login option, or should it *replace*
the current session-cookie auth?

(A) Alternative — both methods coexist
(B) Replacement — migrate to OAuth only
(C) Start with alternative, plan migration later
```

**Vague request (Superpowers decomposition):**
```
User: "Improve the search"

Agent: Before I write any specs, I need to understand the scope.
I see three separate concerns in the search system:
1. Result relevance (ranking algorithm)
2. Result display (UI/UX)
3. Search speed (indexing performance)

Which of these is the primary concern right now?
```

**High-stakes (BMAD pre-mortem):**
```
Agent: [After standard clarification]
One more question before we finalize scope.

If this feature shipped and users consistently avoided using it,
what would be the most likely reason?
```

**Summary (GSD locked/flexible):**
```
Agent: Here's what I understand before writing specs:

LOCKED (your decisions):
- Auth provider: GitHub OAuth only
- Token storage: Store in DB (not client-only)
- Session: Integrate with existing Redis sessions

FLEXIBLE (I'll determine during spec):
- Token refresh strategy
- Error message copy
- Retry logic on OAuth failure

DEFERRED (out of scope this session):
- Google OAuth support
- Magic link fallback

Does this match your intent?
```

---

## Sources

- [Superpowers (GitHub)](https://github.com/obra/superpowers)
- [superpowers/skills/brainstorming/SKILL.md](https://github.com/obra/superpowers/blob/main/skills/brainstorming/SKILL.md)
- [Superpowers: How I'm using coding agents (blog.fsck.com)](https://blog.fsck.com/2025/10/09/superpowers/)
- [Stop Babysitting Your AI Agents: The Superpowers Breakthrough](https://colinmcnamara.com/blog/stop-babysitting-your-ai-agents-superpowers-breakthrough)
- [GSD Framework](https://github.com/gsd-build/get-shit-done)
- [GSD Discussion and Planning Commands (DeepWiki)](https://deepwiki.com/gsd-build/get-shit-done/4.3-discussion-and-planning-commands)
- [What Is GSD? — Medium](https://medium.com/@richardhightower/what-is-gsd-spec-driven-development-without-the-ceremony-570216956a84)
- [Feature-Dev Plugin](https://github.com/anthropics/claude-code/tree/main/plugins/feature-dev)
- [How to Use Feature-Dev in Claude Code (Leanware)](https://www.leanware.co/insights/how-to-use-feature-dev-claude-code)
- [Learning from Claude Code's own plugins (Substack)](https://tgvashworth.substack.com/p/learning-from-claude-codes-own-plugins)
- [BMAD Method Documentation](https://docs.bmad-method.org/)
- [Advanced Elicitation — BMAD Method](https://docs.bmad-method.org/explanation/advanced-elicitation/)
- [BMAD: The Agile Framework That Makes AI Predictable](https://dev.to/extinctsion/bmad-the-agile-framework-that-makes-ai-actually-predictable-5fe7)
- [OpenSpec vs Spec Kit — Big Hat Group](https://www.bighatgroup.com/blog/openspec-vs-speckit-spec-driven-ai-development/)
- [Spec Kit (DeepWiki)](https://deepwiki.com/github/spec-kit/3-spec-driven-development)
- [Teaching AI to Clarify: Handling Assumptions and Ambiguity](https://shanechang.com/p/training-llms-smarter-clarifying-ambiguity-assumptions/)
- [Confirming Requirements with AI — VibeVibe](https://www.vibevibe.cn/en/Advanced/03-prd-doc-driven/02-discuss-with-ai.html)
- [The Double Diamond — David Theil](https://david-theil.medium.com/the-double-diamond-a-model-for-divergent-and-convergent-thinking-4c902a546796)
- [AI as Mirror — Rubber Duck Theory](https://ashitaorbis.com/posts/007-ai-as-mirror/)
- [Windsurf vs Cursor — Builder.io](https://www.builder.io/blog/windsurf-vs-cursor)
- [ChatGPT Canvas vs Claude Artifacts — Skim AI](https://skimai.com/chatgpt-canvas-vs-claude-artifacts-which-is-better-for-enterprise-ai-collaboration/)
