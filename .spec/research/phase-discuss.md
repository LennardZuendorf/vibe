# Research: DISCUSS Phase

> How do AI coding frameworks handle clarification, scope alignment, and requirement refinement before speccing?

Updated: 2026-03-13

---

## Framework Analysis

### GSD (Get Stuff Done)

**How it works:**
- Four-step loop per phase: **discuss → plan → execute → verify**
- `/gsd:discuss-phase` command for adaptive questioning before planning
- Clarifies HOW to implement, not WHETHER to add features
- Generates **3-4 phase-specific "gray areas"** based on domain analysis
- Avoids technical details, architecture choices, and scope expansion

**Key patterns:**
- **Phase-scoped discussion:** Questions are specific to the current phase, not generic
- **Gray area detection:** Automatically identifies ambiguous aspects of the request
- **Focused scope:** Deliberately avoids expanding scope during discussion
- **Domain-aware:** Questions are generated based on actual codebase analysis, not generic templates

**Adopt pattern:** Gray area detection — automatically surface what's ambiguous
**Skip:** The full discuss-per-phase loop — we discuss once before speccing

---

### Superpowers

**How it works:**
- `/superpowers:brainstorm` skill activates before writing code
- Refines rough ideas through **Socratic questions**
- Uses **divergent thinking** (AI's strength) paired with **convergent thinking** (human's strength)
- Follows the **Double Diamond model:** explore → define → ideate → refine
- Explores alternatives, presents design in sections for validation
- Saves a **design document** before proceeding

**Key patterns:**
- **Socratic method:** Questions that surface hidden assumptions, not just fill in blanks
- **Diverge-converge:** Generate multiple interpretations before narrowing to one
- **Double Diamond:** Two rounds of diverge/converge — first on problem definition, then on solution
- **Design document output:** Discussion produces a persistent artifact, not just conversation
- **29k+ GitHub stars, 143k+ installs** — widely validated approach

**Adopt as plugin:** Brainstorm skill for the DISCUSS phase
**Built-in equivalent:** Structured multi-perspective questioning

---

### Feature-Dev

**How it works:**
- Phase 1 ("Discovery"): Clarifies the feature request — what problem, what constraints, what requirements
- Phase 3 ("Clarifying Questions"): After codebase exploration, identifies what's still underspecified
- Questions focus on: edge cases, error handling, integration points, backward compatibility, performance

**Key patterns:**
- **Two rounds of clarification:** Once before research, once after (with codebase context)
- **Context-informed questions:** Phase 3 questions are based on what the code-explorer found
- **Specificity:** Questions target concrete technical concerns, not abstract ones
- **Structured Q&A:** Clear question format that expects concise answers

**Adopt as plugin:** Context-informed clarifying questions (post-research)
**Built-in equivalent:** Structured AskUserQuestion with categories

---

### Cursor / Windsurf

**How it works:**
- Cursor: Mainly relies on user providing context in the chat
- Windsurf: "Cascade" engine asks clarifying questions when the request is ambiguous
- Neither has a formal discussion phase — more ad-hoc

**Skip:** No structured patterns worth adopting

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

**Key patterns:**
- **Separate problem from solution:** First understand what you're solving, then how
- **Explore before committing:** Generate multiple interpretations before picking one
- **Human as convergence engine:** AI is good at diverging (generating options), human is good at converging (picking the right one)

**Adopt built-in:** The diverge-converge structure for our built-in DISCUSS phase

---

## Synthesis: Recommendations for Our DISCUSS Phase

### Built-in Default Provider

```
1. Frame the problem (diverge):
   - "Here are N ways to interpret this request: [A, B, C]"
   - "Which interpretation matches your intent?"

2. Surface gray areas:
   - Scope: "Should this apply to X, Y, or both?"
   - Constraints: "Any performance/compatibility requirements?"
   - Priorities: "If we can't do everything, what's most important?"
   - Edge cases: "What should happen when [unusual condition]?"

3. Clarify approach (converge):
   - "Based on your answers, here's my understanding: [summary]"
   - "Anything I'm missing before I write specs?"

4. Persist decisions:
   - Write key decisions to .spec/research/agreements.md
   - These become inputs to the SPEC phase
```

### Plugin Enhancements

| Plugin | Enhancement |
|--------|-------------|
| **superpowers** | Full brainstorm skill with Socratic questioning, diverge-converge, design document output |
| **feature-dev** | Context-informed clarifying questions based on codebase exploration findings |

### Key Design Decisions

1. **Separate phase from RESEARCH.** Research is about understanding the codebase. DISCUSS is about aligning with the user on what to build. Different goals.
2. **Structured, not freeform.** Built-in uses AskUserQuestion with categories (scope, constraints, priorities, edge cases). Not open-ended chat.
3. **Persist outcomes.** Discussion results are written to files — they survive compaction and inform the SPEC phase.
4. **Diverge before converging.** Present multiple interpretations of the request before asking the user to pick one. Surfaces hidden assumptions.
5. **Context-aware when possible.** If RESEARCH phase found relevant code, reference it in questions ("I found ThemeContext — should we extend it or create a new system?").

### Interaction Example

```
Agent: Before I write specs, let me make sure I understand what you want.

  I see two ways to interpret "add dark mode":
  A) A simple toggle that swaps CSS variables (quick, limited)
  B) A full theme system with custom colors per theme (flexible, more work)

  Which direction?

User: B, but keep it simple for v1.

Agent: Got it. A few more questions:
  1. Scope: Entire app, or just the main content area?
  2. Default: Follow OS preference, or always start light?
  3. Persistence: Where should we save the preference?

User: Entire app, follow OS, localStorage.

Agent: Clear. I'll spec a theme system with CSS custom properties,
  OS preference detection, and localStorage persistence.
  Moving to SPEC phase.
```

---

## Sources

- [Superpowers (GitHub)](https://github.com/obra/superpowers)
- [Superpowers: How I'm using coding agents (blog.fsck.com)](https://blog.fsck.com/2025/10/09/superpowers/)
- [Superpowers Complete Guide](https://pasqualepillitteri.it/en/news/215/superpowers-claude-code-complete-guide)
- [GSD Framework](https://github.com/gsd-build/get-shit-done)
- [Feature-Dev Plugin](https://github.com/anthropics/claude-code/tree/main/plugins/feature-dev)
