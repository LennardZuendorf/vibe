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
- **Mandate-based agents:** Each agent has a specific research focus (e.g., "find all database-related code", "find test patterns", "find config files")
- **Aggressive parallelism:** 3+ agents dispatched simultaneously
- **Fresh context per agent:** Prevents context rot — task 50 has same quality as task 1
- **Structured output:** Agents return compact summaries, not raw file contents

**Adopt:** Mandate-based parallel agents, fresh contexts, structured summaries
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
- **Specialized agent type:** `code-explorer` is purpose-built for codebase analysis
- **Trace-based exploration:** Follows execution paths, maps architecture layers
- **Two-pass approach:** Agents find references → main agent reads and understands
- **Pattern extraction:** Identifies conventions, module boundaries, abstraction layers

**Adopt as plugin:** Route to feature-dev's explorer agents when installed
**Built-in equivalent:** Parallel Explore agents with similar mandates

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

## Synthesis: Recommendations for Our RESEARCH Phase

### Built-in Default Provider

```
1. Read context:
   - .spec/lessons.md (prevent past mistakes)
   - .spec/product.md + tech.md (understand existing architecture)
   - CLAUDE.md (project conventions)

2. Spawn 3+ parallel Explore agents with mandates:
   - Agent 1: "Find existing code related to [feature]" (Glob + Grep)
   - Agent 2: "Find related tests, fixtures, examples" (Glob for test patterns)
   - Agent 3: "Find configuration, types, interfaces" (Grep for type definitions)
   - Agent 4: (if external) WebSearch for APIs/libraries

3. Merge findings into .spec/research/{topic}.md:
   - Exists (don't rebuild): [file paths]
   - Must build: [gap analysis]
   - Patterns & constraints: [conventions found]
   - Risks: HIGH (verified) / MEDIUM (inferred) / LOW (uncertain)
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
2. **Mandate-based agents.** Each research agent gets a specific focus, not a generic "explore everything."
3. **Two-pass pattern.** Agents find references → main agent reads and understands. Don't dump raw file contents.
4. **Parallel by default.** Always dispatch 3+ agents simultaneously. Sequential research is too slow.
5. **Compact summaries.** Agents return structured findings, not raw grep output. ~95% context reduction.

---

## Sources

- [GSD Framework (GitHub)](https://github.com/gsd-build/get-shit-done)
- [GSD v2](https://github.com/gsd-build/gsd-2)
- [Superpowers (GitHub)](https://github.com/obra/superpowers)
- [Feature-Dev code-explorer agent](https://github.com/anthropics/claude-code/blob/main/plugins/feature-dev/agents/code-explorer.md)
- [Beating context rot with GSD - The New Stack](https://thenewstack.io/beating-the-rot-and-getting-stuff-done/)
- [Superpowers Complete Guide](https://pasqualepillitteri.it/en/news/215/superpowers-claude-code-complete-guide)
