# Research: RESEARCH Phase

> How do AI coding frameworks handle codebase exploration and context gathering before writing code?

Updated: 2026-03-13

---

## Framework Analysis

### GSD (Get Stuff Done)

**How it works:**
- Interviews the user first, then spawns **parallel research agents** with distinct mandates
- Each agent gets a **fresh 200k token context window** — no degradation from prior work
- Agents search by different axes: file type, architectural layer, dependency graph
- Research output is structured and merged into a summary
- Context stays clean in the main session while agents do the heavy lifting

**Key patterns:**
- **Typed concern segregation:** 4 parallel agents with distinct concern types — tech/architecture, code quality, implementation concerns, testing/infra. Each agent has a non-overlapping scope to prevent duplicate work.
- **"Dumb zone" theory:** >40% context utilization = degraded output. All design decisions flow from keeping the orchestrator below this threshold.
- **Aggressive parallelism:** 3-4 agents dispatched simultaneously
- **Fresh context per agent:** Prevents context rot — task 50 has same quality as task 1
- **Disk-based output:** All research artifacts written to markdown files, never kept in conversation memory
- **Structured output:** Agents return compact summaries (<500 tokens each), not raw file contents

**Adopt:** Typed concern agents, fresh contexts, disk-based summaries, 40% context ceiling
**Skip:** The full GSD CLI infrastructure — we just need the patterns

---

### Feature-Dev

**How it works:**
- Phase 2 ("Codebase Exploration") launches **2-3 `code-explorer` agents in parallel**
- Each agent is tasked with a different aspect: similar existing features, architectural patterns, relevant abstractions
- Agents return **file references and implementation traces**
- Claude then reads the identified files to build actual understanding
- This happens BEFORE any architecture proposals

**Key patterns:**
- **Specialized agent type:** `code-explorer` is purpose-built for codebase analysis — strictly read-only
- **4-stage structured analysis:** Discovery → Flow Tracing → Architecture Mapping → Detail Extraction
- **Mandatory `file:line` references:** All findings include specific file paths with line numbers, making them directly verifiable
- **Two-pass approach:** Agents find references → main agent reads and understands
- **Pattern extraction:** Identifies conventions, module boundaries, abstraction layers
- **2-3 parallel instances per research run**, each with a distinct aspect

**Adopt as plugin:** Route to feature-dev's explorer agents when installed
**Built-in equivalent:** Parallel Explore agents with similar mandates + `file:line` reference requirement

---

### Superpowers

**How it works:**
- Uses `brainstorm` skill for **divergent requirement interpretation** before convergence
- Research is more about understanding WHAT to build than HOW
- Runs research in forked Explore agents (`context: fork` in skill frontmatter)
- Each forked agent gets isolated context — no conversation history bleed

**Key patterns:**
- **Divergent-convergent thinking:** Generate multiple interpretations of the problem before narrowing
- **Forked context:** Skills with `context: fork` run in isolation
- **Brainstorm before explore:** Understand the problem space before searching the codebase
- **Ultrathink:** Extended thinking mode for deeper analysis

**Adopt as plugin:** Brainstorm skill for requirements discovery
**Built-in equivalent:** Structured problem framing before codebase search

---

### Aider

**How it works:**
- Builds a **repository map** using tree-sitter to extract code structure
- Maps function signatures, class definitions, imports across the entire repo
- Uses this map to identify relevant files before making changes
- Sends only relevant context to the LLM, not the whole repo

**Key patterns:**
- **AST-based mapping:** Uses tree-sitter for structural code understanding
- **Relevance scoring:** Ranks files by relevance to the current task
- **Minimal context:** Only sends what's needed — optimizes token usage
- **Persistent map:** The repo map persists across interactions

**Adopt pattern:** The idea of building a structural map before diving into code
**Skip:** AST tooling — Claude Code's Glob/Grep is sufficient for our needs

---

### Cursor / Windsurf

**How it works:**
- **Cursor:** Uses embeddings-based codebase indexing for semantic search
- **Windsurf (Cascade):** Maintains a "Cascade" context engine that tracks file relationships
- Both do automatic context gathering — user doesn't need to specify files
- Rules files (`.cursorrules`, `.windsurfrules`) guide exploration priorities

**Key patterns:**
- **Semantic search:** Embeddings allow "find code that does X" rather than "grep for Y"
- **Automatic context:** The tool decides what's relevant
- **Rules-guided exploration:** Project-specific rules file tells the tool what matters

**Skip for now:** Embeddings/indexing requires infrastructure we don't have
**Adopt pattern:** Project-specific guidance (we have this via `.spec/` and `CLAUDE.md`)

---

### Claude Code (Vanilla)

**How it works:**
- **Agent tool:** Spawns subagents for complex, multi-step research
- **Explore subagent type:** Specialized for codebase exploration (read-only)
- **Glob tool:** Fast file pattern matching
- **Grep tool:** Regex content search with filtering
- **WebSearch/WebFetch:** External research
- Subagents can run in parallel, in background, and in isolated worktrees

**Key patterns:**
- **Three thoroughness levels:** quick, medium, very thorough (for Explore agents)
- **Parallel dispatch:** Multiple Agent calls in a single message
- **Background execution:** Non-blocking research while other work continues
- **Compact summaries:** Subagents return focused results, not raw data

**This IS our built-in default.** Everything else builds on these primitives.

---

### HumanLayer FIC (Frequent Intentional Compaction)

**How it works:**
- Formalizes context management during research: target **40-60% context utilization**
- Research artifacts written to markdown files
- Each phase starts with a **clean context seeded from the previous phase's output file**
- Files are the memory, not conversation history

**Key insight:** This is exactly our philosophy — "treat sessions as disposable, treat files as permanent." FIC gives it a name and specific utilization targets.

**Adopt built-in:** The 40-60% target and clean-context-per-phase pattern.

---

## Universal Finding

**Research must be isolated from the main context.** Every framework that scales enforces this — subagents explore, main agent receives summaries. Raw search output must never enter the main context.

---

## Synthesis: Recommendations for Our RESEARCH Phase

### Built-in Default Provider

```
1. Read context:
   - .spec/lessons.md (prevent past mistakes)
   - .spec/product.md + tech.md (understand existing architecture)
   - CLAUDE.md (project conventions)

2. Spawn 3-4 parallel Explore agents with TYPED CONCERNS (non-overlapping):
   - Agent 1 (tech/architecture): "Find code related to [feature], map architecture layers"
   - Agent 2 (quality/tests): "Find related tests, fixtures, examples, coverage gaps"
   - Agent 3 (implementation): "Find configuration, types, interfaces, integration points"
   - Agent 4 (external): WebSearch for APIs/libraries (only if needed)

   Rules for subagent output:
   - Each agent returns structured summary UNDER 500 TOKENS
   - All file references must use `file:line` format (directly verifiable)
   - Raw search output must NEVER enter the main context
   - Agents write to disk, orchestrator reads summaries

3. Merge findings into .spec/research/{topic}.md:
   - Exists (don't rebuild): [file:line paths]
   - Must build: [gap analysis]
   - Patterns & constraints: [conventions found]
   - Risks: HIGH (verified in code) / MEDIUM (inferred) / LOW (uncertain)
   - Open questions: [what needs user input]

4. Present to user for confirmation
```

### Plugin Enhancements

| Plugin | Enhancement |
|--------|-------------|
| **feature-dev** | Replace generic Explore agents with `code-explorer` agents that trace execution paths and map architecture layers |
| **superpowers** | Add brainstorm step before exploration — divergent problem interpretation, then convergent search |

### Key Design Decisions

1. **Always persist to files.** Research goes to `.spec/research/*.md` regardless of provider. This survives compaction and session boundaries.
2. **Typed concern agents (from GSD).** Each agent has a non-overlapping concern type (tech, quality, implementation, external). Prevents duplicate work.
3. **`file:line` references mandatory (from Feature-Dev).** All findings include specific file paths with line numbers. Makes findings directly verifiable.
4. **500-token summary cap.** Subagents return structured summaries under 500 tokens. Raw search output never enters main context.
5. **Two-pass pattern.** Agents find references → main agent reads and understands. Don't dump raw file contents.
6. **Parallel by default.** Always dispatch 3-4 agents simultaneously. Sequential research is too slow.
7. **40-60% context target (from FIC).** Keep orchestrator context utilization in this range. Above 40% = degraded quality.
8. **Files are the memory.** Each phase starts clean, seeded from previous phase's output files. Sessions are disposable.

---

## Sources

- [GSD Framework (GitHub)](https://github.com/gsd-build/get-shit-done)
- [GSD v2](https://github.com/gsd-build/gsd-2)
- [GSD context budget management](https://zread.ai/gsd-build/get-shit-done/19-context-budget-management)
- [Superpowers (GitHub)](https://github.com/obra/superpowers)
- [Feature-Dev code-explorer agent](https://github.com/anthropics/claude-code/blob/main/plugins/feature-dev/agents/code-explorer.md)
- [Beating context rot with GSD - The New Stack](https://thenewstack.io/beating-the-rot-and-getting-stuff-done/)
- [Superpowers Complete Guide](https://pasqualepillitteri.it/en/news/215/superpowers-claude-code-complete-guide)
- [HumanLayer FIC (Frequent Intentional Compaction)](https://humanlayer.dev/blog/frequent-intentional-compaction)
