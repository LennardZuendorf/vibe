# Research: REVIEW Phase

> How do AI coding frameworks validate implementation correctness, code quality, and spec compliance?

Updated: 2026-03-13

---

## Framework Analysis

### Simplify (Bundled — `/simplify`)

**How it works:**
- Three parallel agents, fan-out/fan-in pattern (introduced Claude Code v2.1.63)
- **Code Reuse Agent:** Searches for duplicated logic, redundant patterns, existing utilities that could replace new code
- **Code Quality Agent:** Examines readability, structure, conventions; flags redundant state, parameter sprawl, copy-paste variations, leaky abstractions
- **Efficiency Agent:** Checks for unnecessary work, missed concurrency, hot-path bloat, memory leaks, overly broad operations

**Mechanics:**
1. Run `git diff` to identify the change surface (recently changed files only, not whole codebase)
2. All three agents receive the complete diff simultaneously, execute in parallel
3. Main agent aggregates findings, applies valid fixes, silently skips false positives
4. Developer reviews final result with `git diff` before committing

**Key design insight:** Narrowly scoped to changed files. Structural and logical enhancements, never changes what code does — only how. Reported 3-5 issues caught per feature branch; 20-30% token reduction in future sessions from simplified code.

**Status:** This is our bundled default review provider

---

### Superpowers

**How it works:**
- **Two-stage sequential review with hard gates:**
  - Stage 1: **Spec Compliance** — Does the code match what was specified?
  - Stage 2: **Code Quality** — Is the code clean, testable, maintainable?
- Both stages must pass. Stage 2 only runs after Stage 1 passes.
- Used inside the larger `subagent-driven-development` skill.

**Stage 1 — Spec Compliance Reviewer:**
- Fresh subagent with zero implementation context — reads spec and code cold
- Explicitly instructed to be skeptical, read actual code, NOT trust implementer's summary
- Checks for: missing requirements, extra/unneeded work, misunderstandings
- Does not defer to implementer's description of what was done

**Stage 2 — Code Quality Reviewer (only after Stage 1 passes):**
- Assesses patterns, error handling, type safety, naming, test coverage
- For Rails projects: eight specific domain areas checked with explicit WRONG/RIGHT patterns
- Examines separation of concerns, architecture soundness, production readiness

**Gate enforcement:** If either reviewer finds issues, implementer fixes and reviewer checks again. It's a loop, not a single pass. Critical issues block entirely.

**Psychological design:** Skills run "BEFORE any response" to structurally prevent skipping. Makes it architecturally impossible to skip stages (Cialdini's persuasion principles applied to AI compliance).

**Adopt as plugin:** Two-stage review gating
**Built-in equivalent:** Add spec compliance check before /simplify runs

---

### Claude Code Review Plugin (Official Anthropic)

**How it works:**
- 4-5 parallel specialized agents + independent confidence scorers per finding
- **CLAUDE.md Compliance Auditor (×2, redundant):** Two agents independently audit against `CLAUDE.md` and `REVIEW.md` guidelines. Redundancy ensures thoroughness.
- **Bug Detector:** Scans for bugs introduced by the diff only (not pre-existing)
- **History Analyzer:** Examines git blame and commit history for context
- **Independent Confidence Scorers:** One scorer per identified issue, each scoring 0-100

**Confidence scoring system:**

| Score | Meaning |
|-------|---------|
| 0 | Not confident, false positive |
| 25 | Somewhat confident, might be real |
| 50 | Moderately confident, minor |
| 75 | Highly confident, important |
| 100 | Absolutely certain |

Default threshold: 80. Only issues ≥80 reported. Adjustable via config.

**Focus:** Deliberately limited to correctness (logic errors, security vulnerabilities, regressions). NOT formatting, style, or test coverage. Explicit architectural choice to reduce false positives.

**Performance:** 84% of large PRs (>1000 lines) get findings, averaging 7.5 issues. Less than 1% of findings marked incorrect by engineers. $15-25 per review.

**Key principle:** "People are very sensitive to false positives — if we focus on logic errors and actual bugs, the false positive rate is low because when you know about a bug, you should definitely fix it."

---

### Feature-Dev (code-reviewer agent)

**How it works:**
- Phase 6 ("Quality Review") spawns **3 `code-reviewer` agents in parallel:**
  - Focus 1: Simplicity, DRY principles, elegance
  - Focus 2: Bugs and functional correctness
  - Focus 3: Project conventions
- Each reviewer produces findings with **confidence scores (0-100)**
- Findings below confidence threshold are flagged but not blocking

**Severity tiers:**
- **CRITICAL (Security):** Hardcoded credentials, injection, auth bypasses, exposed secrets
- **HIGH (Code Quality):** Large functions, deep nesting, missing error handling, debug statements
- **MEDIUM (Performance):** Inefficient algorithms, unnecessary re-renders, large bundles
- **LOW (Best Practices):** TODOs without tickets, poor naming, magic numbers

**Verdict format:** Reviews conclude with severity table and explicit verdict: **Approve / Warning / Block** based on issue counts.

**Adopt as plugin:** Confidence scoring for review findings
**Built-in equivalent:** Simplify's three-perspective review + our spec compliance check

---

### GSD (Get Stuff Done)

**How it works:**
- **Goal-backward verification:** Instead of "did we complete all tasks?", ask "what must be TRUE for this feature to work?"
- Defines observable conditions, then verifies each in the code
- Tests observable behaviors, not implementation details

**The contrast:**
- **Forward (traditional):** "Build authentication system" → generate task list → check off tasks
- **Goal-backward:** "Users can securely access accounts" → derive what must be true → verify those conditions hold

**Verification artifacts:** Phase-specific `VERIFICATION.md` and `UAT.md` files, distinct from `PLAN.md`. Plans track what was attempted; verification tracks whether goals were achieved.

**Key principles:**
1. Outcome focus: verify outcomes, not task completion
2. Measurable criteria: "user can log in with email/password" replaces "authentication works"
3. Atomic validation: individual task commits enable independent verification

**Adopt built-in:** Goal-backward verification as the first step of every review

---

### Amazon Kiro

**How it works:**
- Spec-driven review with agent hooks as automated quality gates
- Specs generate explicit acceptance criteria in EARS notation
- Agent hooks trigger automatically on file save/create/delete events
- Steering files in `.kiro/steering/` provide persistent project context

**Spec compliance:** Most explicit of any IDE tool — specs generate measurable acceptance criteria directly verifiable.

**Adopt pattern:** Acceptance criteria derived from specs, hook-based automated gates

---

### GitHub Copilot Code Review

**How it works:**
- Agentic architecture (rebuilt 2025): retrieves context intelligently, explores repo beyond just the diff
- Catches issues as it reads (old approach waited until end, causing "forgetting")
- Memory across reviews — doesn't treat each PR as isolated
- Clusters similar findings into single cohesive unit (reduces cognitive load)
- Batch autofixes: resolve entire class of issues with one click
- Integrates with deterministic tools (CodeQL, ESLint) — AI layers on top of static analysis

**Scale:** 60M+ reviews, 10× growth since April 2025, handles 1 in 5 code reviews on GitHub.

---

### CodeRabbit / Greptile / Ellipsis

**CodeRabbit:** PR-bot with dynamic multi-agent task graph. ~44% bug catch rate. Reliable on syntax errors, security, style; weak on intent mismatches and cross-service dependencies.

**Greptile:** Full codebase indexing — 82% bug catch rate (highest), higher false positive rate as trade-off. Reviews diff in context of entire repository.

**Ellipsis:** Review-and-fix in one step — reads reviewer comments and automatically generates commits with fixes applied. Developer reviews the fix commit rather than the issue.

---

## Cross-Cutting Patterns

### Multi-Agent Fan-Out / Fan-In Architecture

Dominant pattern across all sophisticated review tools:

```
Diff/Code
    ↓
[Agent A]  [Agent B]  [Agent C]  [Agent D]   ← parallel, different lenses
    ↓           ↓           ↓          ↓
        [Synthesizer / Aggregator]
                ↓
         Ranked findings
                ↓
        Filtered output (threshold)
```

**Why parallel works:** Each agent receives only context relevant to its lens, avoiding context dilution. Agents with narrow focus are more accurate than a single agent holding all perspectives.

**Aggregation strategies:**
- Confidence scoring per finding, threshold filtering (Claude Code Review)
- Adversarial validation — agents challenge each other (diffray)
- Deduplication before report (diffray, Anthropic Code Review)
- Silent skip of low-confidence findings (varies by tool)

### Confidence Scoring (State of the Art)

0-100 per-finding confidence with configurable threshold (default 80):

| Tier | Score | Action |
|------|-------|--------|
| High confidence | ≥80 | Report / must fix |
| Moderate | 50-79 | Silently skip or low-priority |
| Low | <50 | Drop entirely |

**Critical design principle (Anthropic):** Focus on logic errors only. False positives erode trust in the review process itself.

### Spec Compliance Checking (5 Approaches)

| Approach | Framework | How |
|----------|-----------|-----|
| **Skeptical independent reviewer** | Superpowers | Fresh subagent reads spec + code cold, no implementation context |
| **Document-based compliance** | Claude Code Review | Two auditors check against CLAUDE.md/REVIEW.md guidelines |
| **Acceptance criteria verification** | Kiro | Specs generate EARS-notation criteria, hooks verify on every change |
| **Goal-backward verification** | GSD | Enumerate what must be TRUE, verify each condition |
| **Critic agent at phase transition** | QuantumBlack/McKinsey | Dedicated critic checks phase's definition-of-done criteria |

### Fix-and-Retry Loop (Evaluator-Reflect-Refine)

```
Generator → output → Evaluator → critique → Refiner → improved output → Evaluator ...
  Loop exits when: score ≥ threshold OR max_iterations reached
```

**Production safeguards:**
- Max iterations circuit breaker (prevents infinite loops, runaway costs)
- Rollback to snapshot if max iterations reached without passing
- **Never allow fix loop to modify test files** (prevent "cheating agent")
- Aggressive error log truncation before feedback (prevent context exhaustion)
- Test suite re-run after each fix round to catch regressions from fixes

### False Positive Handling Strategies

| Strategy | Mechanism | Used By |
|----------|-----------|---------|
| Confidence threshold | Only report ≥80 confidence | Claude Code Review, feature-dev |
| Adversarial validation | Agents challenge each other | diffray |
| Narrow scope | Only correctness/bugs, not style | Anthropic Code Review |
| Deduplication | Consolidate similar findings | Copilot, diffray |
| Changed-files-only | Review diff, not full codebase | /simplify |
| Verification step | Check finding against actual code behavior | Anthropic Code Review |
| Human final review | AI doesn't approve/block; human decides | Copilot, Code Review |

---

## Synthesis: Recommendations for Our REVIEW Phase

### Built-in Default Provider

```
Step 1: Deterministic checks (fast, cheap, parallel)
   - Run test suite
   - Run linters / type checkers
   - Validate spec files (validate.sh)
   - Check for debug code, console.logs, TODOs

Step 2: Goal-backward spec compliance (judgment, sequential)
   - Re-read product spec fresh (no implementation context)
   - Enumerate: "What must be TRUE for this feature to work?"
   - Verify each condition against actual code (not task summaries)
   - Re-read tech spec: does code follow described architecture?
   - Flag divergences

Step 3: Multi-perspective code quality (parallel agents)
   - Run /simplify (bundled):
     - Reuse perspective: duplicate code, missed abstractions
     - Quality perspective: bugs, edge cases, error handling
     - Efficiency perspective: performance, unnecessary computation
   - Aggregate findings, apply fixes, skip false positives

Step 4: Self-review checklist
   Hard gates (must all pass):
   - [ ] All plan tasks checked off
   - [ ] Test suite passes
   - [ ] Spec validation passes
   - [ ] No debug code, console.logs, TODOs

   Outcome verification (goal-backward):
   - [ ] Each product spec requirement verifiably observable in code
   - [ ] Each tech spec architectural decision reflected in implementation
   - [ ] Feature works end-to-end as described

   Quality gates (applied):
   - [ ] /simplify run and applied
   - [ ] Code follows existing patterns
   - [ ] Specs still accurate (update if diverged)
   - [ ] Lessons learned → update lessons.md

Step 5: Present to user
   - Summary of what was built
   - Test results
   - Review findings and fixes applied
```

### Review Flow

```
Deterministic checks (tests, lints, validation)
        ↓
   Pass? ──No──→ Fix → retry
        ↓ Yes
Goal-backward spec compliance
        ↓
   Pass? ──No──→ Fix implementation or update spec → retry
        ↓ Yes
Run /simplify (parallel quality review)
        ↓
   Issues? ──Yes──→ Apply fixes → re-test
        ↓ No
Self-review checklist
        ↓
   All clear? ──No──→ Fix remaining items
        ↓ Yes
Present to user → DONE
```

### Fix-and-Retry Design

- Max 3 rounds (enough to fix genuine issues, not enough to spiral)
- Explicit rollback if max iterations reached
- Each round: evaluate → fix → re-test → re-evaluate
- **Never modify test files during fix loop** (prevent cheating agent)
- Truncate error logs before feeding back (prevent context exhaustion)
- Fixes trace back to review findings only — no new features

### Chaining Multiple Review Providers

REVIEW is unique — it can chain multiple providers:

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

When multiple providers configured:
1. Built-in checks always run first (deterministic, goal-backward, spec compliance)
2. Each configured provider runs in sequence
3. All findings collected and presented together
4. Fixes applied between providers if needed

### Plugin Enhancements

| Plugin | Enhancement |
|--------|-------------|
| **simplify** | Multi-agent review with 3 perspectives (default — bundled) |
| **superpowers** | Two-stage gating: spec compliance must pass before quality review. Loop until both pass. |
| **feature-dev** | Confidence-scored findings (0-100). Threshold-based blocking. Approve/Warning/Block verdict. |

### Key Design Decisions

1. **Deterministic first, judgment second.** Run tests/lints before AI review. Cheap, fast, catches obvious failures.
2. **Goal-backward verification (from GSD).** Ask "what must be TRUE?" not "what tasks did we do?" Requirements > style.
3. **Spec compliance before quality (from Superpowers).** Checking quality on non-compliant code wastes effort. Compliance first, always.
4. **Skeptical independent reader (from Superpowers).** Fresh context, no implementation bias. Read actual code, don't trust summaries.
5. **Simplify is the default.** Bundled, always available. Three-perspective review covers most needs.
6. **Confidence scoring for plugin reviews (from Anthropic).** 0-100 per finding, threshold 80. False positives erode trust.
7. **Providers can chain.** Unlike other phases (one provider), REVIEW supports multiple sequential providers.
8. **Fixes only.** During REVIEW, no new features. Every change traces to a review finding.
9. **Never modify tests in fix loop.** Prevents the "cheating agent" anti-pattern.
10. **Lessons are mandatory.** If you learned something during review, update `lessons.md` before DONE.

### Review Phase Artifact

The REVIEW phase should produce a durable record documenting:
1. What was verified and how
2. Which spec requirements confirmed implemented (with file references)
3. What quality issues were found and fixed
4. What lessons were learned

This serves as exit evidence for the REVIEW gate, context for future sessions, and input to `lessons.md`.

---

## Sources

- [Claude Code /simplify guide (claudefa.st)](https://claudefa.st/blog/guide/mechanics/simplify-batch-commands)
- [Piebald-AI system prompt: skill-simplify.md](https://github.com/Piebald-AI/claude-code-system-prompts/blob/main/system-prompts/skill-simplify.md)
- [Anthropic launches multi-agent code review — The New Stack](https://thenewstack.io/anthropic-launches-a-multi-agent-code-review-tool-for-claude-code/)
- [Claude Code 2.1.63 /simplify — SuperGok](https://supergok.com/claude-code-2-1-63-simplify-command/)
- [Superpowers (GitHub)](https://github.com/obra/superpowers)
- [Superpowers for Claude Code — Medium](https://medium.com/@manavghosh/superpowers-for-claude-code-2d0f93f52922)
- [Superpowers: enforces what you should do — ddewhurst](https://ddewhurst.com/blog/superpowers-claude-code-plugin-enforces-what-you-should-do/)
- [Three-stage code review gist](https://gist.github.com/marostr/4ff8fff0b930a615998097a36a4eae37)
- [Claude Code Review plugin README](https://github.com/anthropics/claude-code/blob/main/plugins/code-review/README.md)
- [Code Review — Claude Code Docs](https://code.claude.com/docs/en/code-review)
- [Code Review for Claude Code — Anthropic Blog](https://claude.com/blog/code-review)
- [Feature-Dev code-reviewer](https://github.com/affaan-m/everything-claude-code/blob/main/agents/code-reviewer.md)
- [GSD agent skill (agentskills.so)](https://agentskills.so/skills/ctsstc-get-shit-done-skills-gsd)
- [Kiro docs: Specs](https://kiro.dev/docs/specs/)
- [Beyond Vibe Coding: Amazon Kiro — InfoQ](https://www.infoq.com/news/2025/08/aws-kiro-spec-driven-agent/)
- [About GitHub Copilot code review — GitHub Docs](https://docs.github.com/en/copilot/concepts/agents/code-review)
- [60M Copilot code reviews — GitHub Blog](https://github.blog/ai-and-ml/github-copilot/60-million-copilot-code-reviews-and-counting/)
- [State of AI Code Review Tools 2025 — devtoolsacademy](https://www.devtoolsacademy.com/blog/state-of-ai-code-review-tools-2025/)
- [AI Code Review Benchmarks — Greptile](https://www.greptile.com/benchmarks)
- [CodeRabbit Review 2026 — UCStrategies](https://ucstrategies.com/news/coderabbit-review-2026-fast-ai-code-reviews-but-a-critical-gap-enterprises-cant-ignore/)
- [Multi-Agent Code Review — diffray](https://diffray.ai/multi-agent-code-review/)
- [Scaling Code Review: Multi-Agent Systems — rkoots](https://rkoots.github.io/blog/2026/03/09/bringing-code-review-to-claude-code/)
- [Common workflow patterns for AI agents — Claude Blog](https://claude.com/blog/common-workflow-patterns-for-ai-agents-and-when-to-use-them)
- [Evaluator reflect-refine loop — AWS](https://docs.aws.amazon.com/prescriptive-guidance/latest/agentic-ai-patterns/evaluator-reflect-refine-loop-patterns.html)
- [Sandboxed fix loop — DEV Community](https://dev.to/kowshik_jallipalli_a7e0a5/the-sandboxed-ralph-wiggum-loop-securely-letting-agents-fix-code-until-tests-pass-30h5)
- [Self-Healing Pipelines with AI Agents — Dagger](https://dagger.io/blog/automate-your-ci-fixes-self-healing-pipelines-with-ai-agents)
- [ADK LoopAgent — Google Developer Experts](https://medium.com/google-developer-experts/build-ai-agents-that-self-correct-until-its-right-adk-loopagent-f620bf351462)
- [Agentic workflows for software development — QuantumBlack/McKinsey](https://medium.com/quantumblack/agentic-workflows-for-software-development-dc8e64f4a79d)
- [Spec-Driven Development — Augment Code](https://www.augmentcode.com/guides/what-is-spec-driven-development)
- [Confidence Scoring — multimodal.dev](https://www.multimodal.dev/post/using-confidence-scoring-to-reduce-risk-in-ai-driven-decisions)
- [Confidence Threshold — LlamaIndex](https://www.llamaindex.ai/glossary/what-is-confidence-threshold)
- [Demystifying evals for AI agents — Anthropic Engineering](https://www.anthropic.com/engineering/demystifying-evals-for-ai-agents)
