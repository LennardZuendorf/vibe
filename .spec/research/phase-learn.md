# Research: LEARN Phase

> Should learning be a distinct phase? How do AI coding frameworks capture, store, and apply knowledge across sessions?

Updated: 2026-03-14

---

## The Core Question

Our current lifecycle embeds learning in two places:
- **RESEARCH phase** reads `lessons.md` (apply past learnings)
- **REVIEW phase** updates `lessons.md` (capture new learnings)

The question is whether learning deserves its own phase — a dedicated step between REVIEW and DONE where the agent systematically extracts, categorizes, and persists knowledge for future sessions.

**Compound Engineering answers yes.** Their explicit "Compound" phase is the strongest precedent, and the data supports it.

---

## Compound Engineering Plugin (Primary Cross-Reference)

**Source:** [EveryInc/compound-engineering-plugin](https://github.com/EveryInc/compound-engineering-plugin) (9,300+ stars)
**Authors:** Kieran Klaassen, Dan Shipper (Every, Inc.)
**Article:** [Compound Engineering: How Every Codes With Agents](https://every.to/chain-of-thought/compound-engineering-how-every-codes-with-agents)

### Philosophy

> "In traditional engineering, each feature makes the next feature harder. In compound engineering, each feature makes the next feature easier."

The insight: **learning is not a byproduct of review — it is a distinct activity that requires its own time allocation.** Their 80/20 split (plan+review vs. work+compound) suggests knowledge capture deserves ~10% of total effort.

### The 4-Phase Workflow

| Phase | Command | Time Split | Purpose |
|-------|---------|-----------|---------|
| **Plan** | `/ce:plan` | ~40% | Sub-agents research codebase, commit history, internet best practices in parallel |
| **Work** | `/ce:work` | ~10% | Claude asks clarifying questions, builds the feature and writes tests |
| **Review** | `/ce:review` | ~40% | 15 parallel review agents check security, architecture, performance, simplicity |
| **Compound** | `/ce:compound` | ~10% | Capture learnings into structured wiki documentation |

Additional commands: `/ce:brainstorm`, `/lfg` (full autonomous), `/slfg` (parallel autonomous), `/deepen-plan`

### Plugin Scale

- **28 specialized AI agents** across 5 categories (review: 15, research: 5, design: 3, workflow: 4, docs: 1)
- **13 slash commands**, **20 skills**
- **2 MCP servers**: Playwright (browser automation), Context7 (framework docs for 100+ frameworks)
- Cross-tool support: converts to OpenCode, Codex, Gemini CLI, Copilot, Kiro, Windsurf, and others

### The "Compound" Phase — What It Actually Does

During `/ce:compound`, the agent:
1. Summarizes bugs encountered and their solutions
2. Documents performance issues identified
3. Records new problem-solving approaches developed
4. Stores these in structured wiki files that agents read in future sessions

In the next cycle, the **learnings-researcher** agent and the plan phase automatically consult this accumulated knowledge, creating the compounding feedback loop.

### Cross-Reference: How This Maps to Our Framework

| Compound Engineering | Our Framework | Gap |
|---------------------|---------------|-----|
| Plan (40%) | RESEARCH + DISCUSS + SPEC + PLAN | We split planning into 4 phases — more granular |
| Work (10%) | IMPLEMENT | Similar |
| Review (40%) | REVIEW | Similar (both use parallel review agents) |
| **Compound (10%)** | **Embedded in REVIEW** | **This is the gap — we don't have a dedicated learning phase** |

The distinction from review: **Review asks "is this correct?" Learning asks "what should we never forget?"**

---

## Framework-by-Framework Analysis

### Three Generations of Learning Systems

1. **Manual rule files** (2024–early 2025): Developer-written CLAUDE.md / AGENTS.md / .cursorrules. Learning happens in the developer's head; they manually update rules.

2. **Auto-capture memory** (mid 2025–2026): Claude auto-memory, Copilot agentic memory, Windsurf Cascade memories. Agent detects what's worth remembering and persists it. Continuous but passive.

3. **Structured retrospective phases** (2026+): Compound Engineering's compound step, everything-claude-code's continuous-learning skill, SICA's meta-agent loop. Learning is explicit, distinct, with its own triggers and storage.

**The field is converging on learning as a first-class concern, not an afterthought.**

---

### Claude Code Auto-Memory (Built-in)

- Writes notes to `~/.claude/projects/<project>/memory/MEMORY.md` and topic files
- Continuous during conversation — not a distinct phase
- Captures build commands, debugging insights, architecture notes, style preferences
- Project-scoped: all worktrees/subdirectories share one auto-memory directory
- No confidence scoring or quality filtering
- **Timing:** Continuous (embedded)

---

### everything-claude-code Continuous Learning

**v1 (Stop Hook):**
- At session end, Stop hook evaluates session (minimum 10 messages)
- Scans transcript for extractable patterns
- Writes qualified patterns to `~/.claude/skills/learned/`
- ~50-80% skill activation reliability

**v2 (Instinct-Based):**
- Hooks capture every PreToolUse and PostToolUse event
- Appends structured observations to JSONL store
- Background observer detects patterns, creates "Instincts" with confidence scores (0.3–0.9)
- `/evolve` command aggregates 3+ related instincts into reusable Skill modules
- 100% observation reliability

**Pattern categories:** `error_resolution`, `user_corrections`, `workarounds`, `debugging_techniques`, `project_specific`

**Key innovation:** Confidence-scored instincts that evolve into skills — the closest thing to automated learning with quality control.

---

### Superpowers

- Creator extracted 2,249 markdown files of lessons/corrections from previous conversations
- Clustered by topic, "pressure tested" to determine if new skills were needed
- Most situations already handled by existing skills — only 1-2 required improvements
- Learning baked into skill design (hard gates, hooks, enforcement), not captured per-session
- "Latent space engineering" — influencing the model's internal state, not just providing facts
- **Timing:** Design-time (manual, by framework author)

---

### GitHub Copilot Agentic Memory

- Repository-scoped memories built from observing agent interactions
- Memories include cited code locations for verification
- Cross-agent sharing: memory from coding agent used by review agent
- **Self-correcting:** Before applying any memory, agent verifies accuracy by checking cited code. If code contradicts memory, stores corrected version.
- Human confirmation gate before saving (in VS)
- **Key innovation:** Self-healing memory pool — prevents stale learnings from poisoning future sessions

---

### OpenAI Codex CLI Memory

- Background pipeline at startup: processes completed sessions
- Phase 1: Extracts `raw_memory` and `rollout_summary` from each eligible session, stores in SQLite
- Phase 2: Spawns consolidation sub-agent to update `MEMORY.md`, `memory_summary.md`, and `skills/`
- "Polluted" sessions (e.g., using web search) excluded from memory
- **Timing:** Asynchronous at session startup (background, architecturally distinct)

---

### Kiro Steering Files

- Markdown docs in `.kiro/steering/` — product.md, structure.md, tech.md
- Three inclusion modes: `always`, `fileMatch`, `manual`
- Grow continuously with the codebase
- Initial files auto-generated, ongoing updates manual
- **Timing:** Continuous (embedded in workflow)

---

### Mem0 — Universal Memory Layer

- Three memory types: episodic (interaction events), semantic (extracted knowledge), procedural (behavioral patterns)
- Extraction via LLM: converts messages into entities and relation triplets
- Conflict Detector flags overlapping/contradictory nodes; Update Resolver decides add/merge/invalidate/skip
- 26% accuracy uplift over OpenAI memory, 91% lower p95 latency, 90% token cost savings
- Supports Neo4j, Memgraph, Amazon Neptune
- **Timing:** Continuous (memory layer, any phase)

---

### SICA (Self-Improving Coding Agent) — Research

- Meta-agent loop: agent edits its own code to improve performance
- Benchmarking → meta-improvement cycles
- Tool synthesis: agent proposes, implements, and validates new utilities
- 17–53% performance gains on SWE-bench Verified
- Safety: sandbox containment, human-in-the-loop for critical modifications
- **Timing:** Distinct phase (explicit meta-improvement after benchmarking)

---

### DSPy / GEPA — Automated Prompt Optimization

- MIPROv2: bootstraps traces, filters for high-scoring trajectories
- GEPA: reflective prompt evolution using genetic-Pareto optimization
- Leverages any textual feedback: eval logs, code traces, failed parses, error messages
- 93% accuracy on MATH benchmark (vs 67% basic)
- **Timing:** Distinct optimization phase (batch, after data collection)

---

### Devin AI

- Knowledge system: tips, docs, instructions persisting across sessions
- Session Insights provides improved prompts for similar future tasks
- Cross-session memory largely manual ("amnesiac contractor")
- **Timing:** End of session (partially embedded)

---

### Ralph Loop / Addy Osmani Architecture

- Four memory channels: git history, progress log, task state, AGENTS.md
- AGENTS.md organized: Patterns & Conventions, Gotchas, Style/Preferences, Recent Learnings
- Real-time correction: developer stops loop, adds notes to AGENTS.md
- Key advice: keep AGENTS.md pruned to prevent context bloat
- **Timing:** Continuous within loop (embedded)

---

## Cross-Cutting Patterns

### Learning Storage Format Spectrum

| Format | Examples | Pros | Cons |
|--------|----------|------|------|
| Plain markdown | CLAUDE.md, AGENTS.md, MEMORY.md, Kiro steering | Human-readable, version-controllable | No structure, context bloat risk |
| Structured markdown with metadata | everything-claude-code v1, Compound Engineering | Categorized, searchable | No machine-readable semantics |
| JSONL / SQLite | everything-claude-code v2, Codex CLI | Machine-readable, queryable, scored | Not human-friendly |
| Knowledge graph | Mem0, Copilot agentic memory | Rich relationships, conflict detection | Complex infrastructure |

**Our position:** Structured markdown with metadata (lessons.md with Pattern/Rule/Date). Right balance of human-readable and structured.

### When Learning Happens

| Timing | Examples | Trade-offs |
|--------|----------|------------|
| Continuous (during work) | Claude auto-memory, Windsurf, Mem0 | Low friction, but noisy |
| Post-session | everything-claude-code v1, Codex CLI | Clean extraction, but delayed |
| **Explicit phase** | **Compound Engineering**, SICA, DSPy | **Highest quality, but requires discipline** |
| Post-iteration in loop | Ralph, Addy Osmani | Good for automated loops |

**Compound Engineering's approach is strongest** because it's intentional and time-boxed.

### Quality Control Mechanisms

1. **No filter** — save everything (early CLAUDE.md approach)
2. **Heuristic filter** — minimum session length, pattern type matching (everything-claude-code v1)
3. **Confidence scoring** — 0.3–0.9 scores with evolution thresholds (everything-claude-code v2)
4. **Human curation gate** — save suggestions requiring approval (Copilot VS, Devin)
5. **Self-healing** — citation verification, contradiction detection (Copilot agentic memory)
6. **Automated optimization** — benchmark-driven pruning (DSPy, SICA)

### Memory Decay and Maintenance

Several frameworks flag the problem of stale learnings:
- **Claude Code:** "Outdated or incorrect information in memory files can lead Claude astray, causing more harm than having no memory at all"
- **Copilot:** Self-correcting memories that verify citations against current code
- **Superpowers:** "Pressure testing" whether lessons are still needed — most were already handled
- **Addy Osmani:** Keep AGENTS.md pruned — archive obsolete information

**A LEARN phase needs a pruning mechanism, not just accumulation.**

### Pre-mortem vs. Post-mortem Mapping

| Traditional | Agent Lifecycle | Phase |
|-------------|----------------|-------|
| Pre-mortem ("What could go wrong?") | Read lessons.md before working | RESEARCH |
| Post-mortem ("What went wrong?") | Extract learnings after completing work | LEARN |
| Retrospective ("What should we change?") | Update process rules, spec improvements | LEARN |

---

## Synthesis: Recommendations for Our LEARN Phase

### Should LEARN Be a Distinct Phase?

**Yes.** Three arguments:

1. **Compound Engineering proves it works.** Their most successful innovation isn't the review agents — it's the dedicated compound step. The plugin has 9,300+ stars and is the most popular Claude Code plugin.

2. **Review and learning have different goals.** Review asks "is this correct?" Learning asks "what should we never forget?" Combining them dilutes both. Review focus drifts toward "what did we learn?" instead of "does this work?"

3. **Embedded learning gets skipped.** When learning is a checkbox in the review checklist, it's the first thing cut when time is short. A distinct phase with its own gate makes skipping architecturally harder.

### Proposed Phase Position

```
RESEARCH → DISCUSS → SPEC → PLAN → IMPLEMENT → REVIEW → LEARN → DONE
```

LEARN runs after REVIEW because:
- Review may produce its own learnings (bugs found, spec gaps discovered)
- Review fixes should be complete before extracting patterns
- Learning needs the full picture: what was planned, what was built, what broke, what was fixed

### What LEARN Should Do

```
Step 1: Extract learnings (automated)
   - Scan session for: user corrections, test failures and fixes,
     spec divergences, review findings, blocked paths
   - Categorize by type: error_resolution, user_correction,
     workaround, debugging_technique, process_improvement,
     spec_gap, pattern_discovery

Step 2: Update lessons.md (write)
   For each extracted learning:
   - Write structured entry: Pattern, Rule, Date, Category, Confidence
   - Reference specific file paths where relevant
   - Cross-reference related existing lessons

Step 3: Prune stale lessons (maintain)
   - Check existing lessons against current codebase
   - If referenced files/patterns no longer exist → mark for removal
   - If a lesson contradicts current code → flag for review
   - Present pruning candidates to user for confirmation

Step 4: Update specs if needed (sync)
   - If implementation revealed spec gaps during REVIEW → update specs
   - If lessons suggest process changes → note in lessons.md
   - Run spec validation after any spec updates

Step 5: Present summary (report)
   - New lessons added: N
   - Stale lessons pruned: N
   - Specs updated: list
   - "Learning complete. Ready to mark DONE?"
```

### LEARN Phase Exit Criteria

- [ ] All session learnings extracted and categorized
- [ ] lessons.md updated with new entries (Pattern/Rule/Date format)
- [ ] Stale lessons identified and pruned (with user confirmation)
- [ ] Specs synced if review revealed gaps
- [ ] User confirms learning summary

### Integration with Compound Engineering Plugin

If compound-engineering is installed as a plugin, it can serve as the LEARN phase provider:

```json
{
  "learn": {
    "provider": "compound-engineering",
    "config": {
      "wiki_path": ".spec/wiki/",
      "auto_prune": true
    }
  }
}
```

When compound-engineering provides the LEARN phase:
- Uses its `learnings-researcher` agent for extraction
- Stores in its wiki format (alongside our lessons.md)
- Its `/ce:compound` command maps to our LEARN phase gate

When no plugin is installed (built-in default):
- Agent performs the 5-step process above
- Stores in lessons.md (our existing format)
- Simpler, no wiki infrastructure needed

### Key Design Decisions

1. **Distinct phase, not embedded.** Compound Engineering's strongest innovation. Worth the ceremony.
2. **After REVIEW, before DONE.** Review findings feed into learning. Learning doesn't block review.
3. **Pruning is mandatory.** Accumulation without decay leads to context bloat and stale rules poisoning future sessions (Claude Code's own warning).
4. **Structured markdown, not JSONL.** Human-readable, version-controllable, git-friendly. Our lessons.md format with added Category and Confidence fields.
5. **File-path references.** Lessons that reference specific files can be automatically invalidated when those files change (Copilot's self-correcting memory pattern).
6. **10% time budget.** Compound Engineering's allocation. Quick and intentional, not a ceremony.
7. **Plugin-compatible.** compound-engineering can provide this phase, but the built-in default works with zero plugins.
8. **Session corrections are highest-value.** User corrections during the session are the most reliable signal for learning (everything-claude-code v2 finding).

### Enhanced Lessons Format

Current format:
```markdown
### [Short description]
**Pattern:** What went wrong and why
**Rule:** The concrete rule that prevents this
**Date:** When learned
```

Proposed enhanced format:
```markdown
### [Short description]
**Category:** error_resolution | user_correction | workaround | process_improvement | spec_gap | pattern_discovery
**Pattern:** What went wrong and why
**Rule:** The concrete rule that prevents this
**References:** File paths or spec sections this applies to
**Confidence:** HIGH (verified in code) | MEDIUM (inferred) | LOW (uncertain)
**Date:** When learned
```

The enhanced format enables:
- Automated pruning (check References against current files)
- Category-based filtering (surface relevant lessons by task type)
- Confidence-based prioritization (HIGH lessons always loaded, LOW lessons loaded on-demand)

---

## Sources

### Compound Engineering (Primary Reference)
- [EveryInc/compound-engineering-plugin (GitHub)](https://github.com/EveryInc/compound-engineering-plugin)
- [Compound Engineering: How Every Codes With Agents](https://every.to/chain-of-thought/compound-engineering-how-every-codes-with-agents)
- [Compound Engineering Guide](https://every.to/guides/compound-engineering)
- [Learning from Every's Compound Engineering (Will Larson)](https://lethain.com/everyinc-compound-engineering/)
- [Ry Walker Research](https://rywalker.com/research/compound-engineering-plugin)
- [VibeSparking Overview](https://www.vibesparking.com/en/blog/ai/2026-01-03-compound-engineering-plugin-claude-code/)
- [Torq Software Notes](https://reading.torqsoftware.com/notes/software/ai-ml/agentic-coding/2026-01-19-compound-engineering-claude-code/)

### Memory and Learning Systems
- [Claude Code Memory Documentation](https://code.claude.com/docs/en/memory)
- [everything-claude-code Continuous Learning Skill](https://github.com/affaan-m/everything-claude-code/blob/main/skills/continuous-learning/SKILL.md)
- [everything-claude-code Continuous Learning v2](https://playbooks.com/skills/affaan-m/everything-claude-code/continuous-learning-v2)
- [GitHub Copilot Agentic Memory](https://docs.github.com/en/copilot/concepts/agents/copilot-memory)
- [Building an Agentic Memory System for Copilot](https://github.blog/ai-and-ml/github-copilot/building-an-agentic-memory-system-for-github-copilot/)
- [OpenAI Codex CLI Memory System](https://deepwiki.com/openai/codex/3.7-memory-system)
- [Mem0: Universal Memory Layer](https://mem0.ai/research)
- [Mem0 Paper (arXiv)](https://arxiv.org/abs/2504.19413)

### Frameworks
- [Superpowers Plugin](https://github.com/obra/superpowers)
- [Kiro Steering Files](https://kiro.dev/)
- [GSD (Get Stuff Done)](https://medium.com/@richardhightower/what-is-gsd-spec-driven-development-without-the-ceremony-570216956a84)
- [Devin AI Documentation](https://docs.devin.ai/essential-guidelines/when-to-use-devin)
- [Ralph Autonomous Agent Loop](https://github.com/snarktank/ralph)
- [DSPy Framework](https://dspy.ai/)
- [SICA: Self-Improving Coding Agent](https://www.researchgate.net/publication/390991089_A_Self_Improving_Coding_Agent)

### Articles and Analysis
- [Self-Improving Coding Agents (Addy Osmani)](https://addyosmani.com/blog/self-improving-agents/)
- [Complete Guide to AI Agent Memory Files](https://medium.com/data-science-collective/the-complete-guide-to-ai-agent-memory-files-claude-md-agents-md-and-beyond-49ea0df5c5a9)
- [Self-Evolving Agents Cookbook (OpenAI)](https://cookbook.openai.com/examples/partners/self_evolving_agents/autonomous_agent_retraining)
- [Agentic Engineering Patterns (Simon Willison)](https://simonwillison.net/guides/agentic-engineering-patterns/better-code/)
- [Humans and Agents in Software Engineering Loops (Martin Fowler)](https://martinfowler.com/articles/exploring-gen-ai/humans-and-agents.html)
