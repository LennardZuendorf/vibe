# Research: REVIEW Phase

> How do AI coding frameworks validate implementation correctness, code quality, and spec compliance?

Updated: 2026-03-13

---

## Framework Analysis

### Simplify (Bundled)

**How it works:**
- Spawns **3 parallel review agents**, each with a different perspective:
  - **Reuse agent:** Finds duplicate code, missed abstractions, existing utilities that could be used
  - **Quality agent:** Checks for bugs, edge cases, error handling, naming, clarity
  - **Efficiency agent:** Identifies performance issues, unnecessary computation, memory leaks
- Each agent returns focused findings
- Findings are merged into actionable items
- Agent fixes the issues found

**Key patterns:**
- **Multi-perspective review:** Three distinct lenses catch different types of issues
- **Parallel execution:** All three agents run simultaneously
- **Focused mandates:** Each agent only looks for its type of issue (no overlap)
- **Actionable output:** Findings are concrete (file, line, issue, fix)

**Status:** This is our bundled default review provider

---

### Superpowers

**How it works:**
- **Dual-stage review:**
  - Stage 1: **Spec compliance** — Does the code match what was specified?
  - Stage 2: **Code quality** — Is the code clean, testable, maintainable?
- Both stages must pass before the review is considered complete
- Uses the `review` skill which can be invoked standalone

**Key patterns:**
- **Two-stage gate:** Compliance first, quality second. Both must pass.
- **Spec compliance is explicit:** Re-reads specs and verifies each requirement is implemented
- **Staged feedback:** If stage 1 fails, stage 2 doesn't run (no point checking quality if requirements aren't met)

**Adopt as plugin:** Two-stage review gating
**Built-in equivalent:** Add spec compliance check before /simplify runs

---

### Feature-Dev

**How it works:**
- Phase 6 ("Quality Review") spawns **3 `code-reviewer` agents in parallel:**
  - Focus 1: Simplicity, DRY principles, elegance
  - Focus 2: Bugs and functional correctness
  - Focus 3: Project conventions
- Each reviewer produces findings with **confidence scores (0-100)**
- Findings below the confidence threshold are flagged but not blocking
- Phase 7 produces a final summary

**Key patterns:**
- **Confidence scoring:** Each finding has a 0-100 confidence score
- **Threshold-based blocking:** Below threshold = warning, above = must fix
- **Convention awareness:** One reviewer specifically checks project conventions
- **Three specialized focuses:** Similar to Simplify but with different lens labels

**Adopt as plugin:** Confidence scoring for review findings
**Built-in equivalent:** Simplify's three-perspective review + our own spec compliance check

---

### GSD (Get Stuff Done)

**How it works:**
- **Goal-backward verification:** Instead of "what tasks did we do?", ask "what must be TRUE for this feature to work?"
- Defines observable conditions, then verifies each one in the code
- Tests observable behaviors, not implementation details

**Key patterns:**
- **Goal-backward thinking:** Start from the desired end state and work backward
- **Observable conditions:** "The toggle appears in settings" not "ToggleButton component exists"
- **Behavior over implementation:** Test what the user sees, not how it's coded

**Adopt built-in:** Goal-backward verification as the first step of every review

---

### CodeRabbit / Ellipsis / Greptile (AI Code Review Tools)

**How they work:**
- **CodeRabbit:** Automated PR review with line-by-line comments. Uses AST analysis + LLM reasoning. Learns project conventions over time. Free for open source.
- **Ellipsis:** PR review + bug detection. Focuses on security vulnerabilities and common error patterns.
- **Greptile:** Codebase-aware review. Understands the full repo context, not just the diff.

**Key patterns:**
- **Diff-focused review:** Review only what changed, not the entire codebase
- **Convention learning:** Tools that learn project patterns over time
- **Security focus:** Dedicated checks for OWASP top 10, injection, XSS
- **Codebase-aware:** Understanding context beyond just the changed files

**Skip for now:** These are PR-level tools, not phase-level review tools
**Adopt pattern:** Diff-focused review — review changed files, not everything

---

### GitHub Copilot Code Review

**How it works:**
- AI-powered PR review built into GitHub
- Reviews diffs and suggests improvements
- Can auto-fix certain issues
- Integrates with GitHub Actions

**Skip:** PR-level tool, not relevant to our phase-level review

---

## Synthesis: Recommendations for Our REVIEW Phase

### Built-in Default Provider

```
1. Goal-backward verification (from GSD):
   - Re-read product spec
   - List observable conditions: "What must be TRUE?"
   - Verify each condition in the code (not assumed — actually checked)

2. Spec compliance check:
   - Re-read tech spec
   - For each architectural decision, verify it's implemented as specified
   - Flag any divergences

3. Run test suite:
   - Execute project tests
   - Fix any failures before proceeding

4. Run /simplify (bundled):
   - Reuse perspective: duplicate code, missed abstractions
   - Quality perspective: bugs, edge cases, error handling
   - Efficiency perspective: performance, unnecessary computation
   - Apply fixes

5. Self-review checklist:
   - [ ] All plan tasks checked off
   - [ ] Every spec requirement verifiably implemented
   - [ ] Specs still accurate (update if implementation diverged)
   - [ ] No debug code, console.logs, or TODOs left behind
   - [ ] Code follows existing patterns in the codebase
   - [ ] Lessons learned? Update .spec/lessons.md

6. Validate specs:
   - Run validate.sh
   - Ensure cross-references are intact

7. Present to user:
   - Summary of what was built
   - Test results
   - Review findings and fixes applied
```

### Review Flow Diagram

```
Goal-backward verification
        ↓
   Pass? ──No──→ Fix implementation → retry
        ↓ Yes
Spec compliance check
        ↓
   Pass? ──No──→ Fix or update spec → retry
        ↓ Yes
Run test suite
        ↓
   Pass? ──No──→ Fix tests → retry
        ↓ Yes
Run /simplify
        ↓
   Issues? ──Yes──→ Apply fixes → re-test
        ↓ No
Self-review checklist
        ↓
   All clear? ──No──→ Fix remaining items
        ↓ Yes
Present to user → DONE
```

### Plugin Enhancements

| Plugin | Enhancement |
|--------|-------------|
| **simplify** | Multi-agent review with 3 perspectives (default — bundled) |
| **superpowers** | Two-stage gating: spec compliance must pass before quality review runs |
| **feature-dev** | Confidence-scored findings (0-100). Threshold-based blocking. |

### Chaining Multiple Review Providers

The REVIEW phase is unique — it can chain multiple providers:

```json
{
  "review": {
    "providers": ["simplify", "feature-dev"],
    "config": {
      "confidence_threshold": 80
    }
  }
}
```

When multiple providers are configured:
1. Built-in checks always run first (goal-backward, spec compliance, tests)
2. Each configured provider runs in sequence
3. All findings are collected and presented together
4. Fixes are applied between providers if needed

### Key Design Decisions

1. **Goal-backward verification first (from GSD).** Ask "what must be TRUE?" before checking code quality. Requirements > style.
2. **Spec compliance is non-negotiable.** Always runs, regardless of configured provider. This is the backbone.
3. **Tests must pass.** No amount of code review compensates for failing tests.
4. **Simplify is the default.** Bundled and always available. Three-perspective review covers most needs.
5. **Providers can chain.** Unlike other phases (one provider), REVIEW supports multiple sequential providers.
6. **Fixes only.** During REVIEW, no new features. Every change traces to a review finding.
7. **Lessons are mandatory.** If you learned something during review, update `lessons.md` before marking DONE.
8. **Spec loop-back.** If review reveals spec gaps, loop back to RESEARCH (not just fix forward).

---

## Sources

- [Simplify skill](https://github.com/anthropics/claude-code/tree/main/plugins/simplify) (reference)
- [Feature-Dev code-reviewer](https://github.com/anthropics/claude-code/blob/main/plugins/feature-dev/agents/code-reviewer.md)
- [Superpowers (GitHub)](https://github.com/obra/superpowers)
- [GSD Framework](https://github.com/gsd-build/get-shit-done)
- [CodeRabbit](https://coderabbit.ai/)
