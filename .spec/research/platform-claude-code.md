# Research: Claude Code Platform Capabilities

> What building blocks does Claude Code provide that we can use to build the framework? What are the constraints?

Updated: 2026-03-18

---

## Why This Matters

The engineering agent framework runs entirely on Claude Code. Every phase, gate, routing decision, and enforcement mechanism must be built from Claude Code primitives. Understanding exactly what's available — and what's not — determines what we can build and how.

---

## 1. Skills System

Skills are the primary extension mechanism. A skill is a markdown file that Claude reads and follows.

**Structure:**
```
.agents/skills/<skill-name>/
├── SKILL.md          # The skill definition (required). Claude reads this when the skill is invoked.
├── README.md         # Human documentation (optional)
├── scripts/          # Shell scripts the skill can reference
└── reference/        # Reference material (templates, guides)
```

**How invocation works:**
- User types `/<skill-name>` or `/<skill-name> <arguments>`
- Claude reads `SKILL.md`, which contains instructions and `$ARGUMENTS` placeholder
- The entire skill execution happens within Claude's normal tool-use loop
- Skills can invoke other skills via the `Skill` tool
- Skills can reference their own scripts via `bash ~/.agents/skills/<name>/scripts/foo.sh`

**What skills CAN do:**
- Define multi-step workflows with decision points
- Invoke any tool (Read, Write, Edit, Bash, Agent, AskUserQuestion, etc.)
- Spawn subagents for parallel work
- Read/write files, run commands, search code
- Call other skills via the Skill tool

**What skills CANNOT do:**
- Persist state between sessions (must use files)
- Intercept tool calls (that's hooks)
- Run automatically without user invocation (that's hooks)
- Access Claude's internal state or conversation history programmatically

**Relevance for our framework:**
- `/develop` is a skill that orchestrates the entire lifecycle
- `/spec` is a skill that manages specs
- `/setup-framework` will be a skill
- Skills are the "commands" users invoke. Hooks are the "enforcement" layer.

---

## 2. Hooks System

Hooks are shell commands that run in response to Claude Code events. They're the enforcement mechanism.

**Hook types:**

| Hook | When It Fires | Use Case |
|------|--------------|----------|
| `PreToolUse` | Before any tool executes | **Phase gate enforcement.** Block writes during wrong phases. |
| `PostToolUse` | After any tool completes | Post-action validation, logging |
| `SessionStart` | When a session begins (matcher: `startup`) | Load context, show phase status |
| `Stop` | When Claude finishes a response | Post-session learning extraction, auto-commit |
| `SubagentStop` | When a subagent completes | Aggregate subagent results |

**Matcher patterns:**
- `PreToolUse` and `PostToolUse` use tool name matchers: `"Edit|Write|NotebookEdit"`, `"Bash"`, `"Agent"`, etc.
- `SessionStart` uses `"startup"` matcher
- Matchers support regex-style OR: `"Edit|Write"` matches either tool

**Hook data (stdin):**
Hooks receive JSON on stdin with tool-specific data:
```json
// PreToolUse for Edit/Write:
{
  "tool_name": "Write",
  "tool_input": {
    "file_path": "/path/to/file",
    "content": "..."
  }
}
```

**Exit codes:**
| Code | Meaning | Effect |
|------|---------|--------|
| 0 | Allow | Tool proceeds normally |
| 1 | Error | Hook failed (tool still proceeds, error logged) |
| 2 | Block | **Tool is blocked.** Stderr message shown to Claude as the reason. |

**Stderr as feedback:** When exit code is 2, whatever the hook writes to stderr becomes the error message Claude sees. This is how we communicate "you're in RESEARCH phase, writes to source files are not allowed."

**Environment variables in hooks:**
- `CLAUDE_SESSION_ID` — current session ID
- Standard env vars (PATH, HOME, etc.)
- Hooks run in the project root directory

**What hooks CAN do:**
- Block tool calls (exit 2)
- Provide feedback to Claude via stderr
- Read any file (including `.spec/.phase`)
- Run any shell command
- Write files (for logging, state tracking)

**What hooks CANNOT do:**
- Modify the tool call (can only allow or block)
- Access Claude's conversation context
- Modify Claude's next response (only provide stderr feedback)
- Run interactively (no stdin from user)

**Relevance for our framework:**
- `PreToolUse` on `Edit|Write|NotebookEdit` → phase gate enforcement
- `SessionStart` → load phase state, remind about lessons.md
- `Stop` → potential LEARN phase auto-extraction
- `PreToolUse` on `Bash` → could enforce no-destructive-commands during certain phases

---

## 3. CLAUDE.md and Agents.md

These are instruction files that Claude reads at session start.

**Loading order and scoping:**

| File | Scope | Loaded When |
|------|-------|-------------|
| `~/.claude/CLAUDE.md` | User-global | Every session |
| `<project>/CLAUDE.md` | Project root | When working in this project |
| `<project>/Agents.md` | Project root | Same as CLAUDE.md (alias) |
| `<subdir>/CLAUDE.md` | Subdirectory | When working in files in this subdir |

**What goes where:**
- **CLAUDE.md** — Project-level instructions, rules, conventions. "Always run tests before committing." "Use TypeScript strict mode."
- **Agents.md** — Same as CLAUDE.md. Some projects use both (Agents.md for agent-specific instructions).
- Neither should be massive — they're loaded into every session's context.

**Relevance for our framework:**
- `CLAUDE.md` is our top-level framework documentation: what skills exist, what the workflow is, what rules apply
- Should reference skills and hooks, not duplicate their content
- Needs updating when the architecture changes (it's currently stale)

---

## 4. Subagents (Agent Tool)

Subagents are independent Claude instances spawned for specific tasks.

**Subagent types:**

| Type | Tools Available | Use Case |
|------|----------------|----------|
| `general-purpose` (default) | All tools | Implementation tasks, complex multi-step work |
| `Explore` | Read-only (Glob, Grep, Read, WebSearch, WebFetch) | Codebase search, research, fast exploration |
| `Plan` | Read-only | Architecture planning, implementation design |

**Key capabilities:**

- **Model overrides:** `model: "haiku"` for cheap Explore agents, `model: "opus"` for complex work
- **Worktree isolation:** `isolation: "worktree"` gives the agent an isolated git worktree. Changes are on a separate branch. If agent makes changes, the worktree path and branch are returned.
- **Background execution:** `run_in_background: true` — agent runs asynchronously, parent is notified on completion
- **Parallel dispatch:** Multiple Agent calls in the same message run in parallel
- **Fresh context:** Each agent starts with a clean context — no conversation history leakage
- **SendMessage:** Can continue a previously spawned agent with its full context preserved

**Context management:**
- Subagents do NOT inherit the parent's conversation history
- The `prompt` parameter is the ONLY context the agent gets (plus CLAUDE.md, hooks, etc.)
- This means: include everything the agent needs in the prompt (spec content, file paths, task description)
- Agent results are returned to the parent as a single message

**Worktree isolation details:**
- Creates a temporary git worktree (isolated copy of the repo)
- Agent works on its own branch
- If no changes made: worktree auto-cleaned
- If changes made: worktree path and branch returned in result
- Parent can merge the branch or inspect changes
- Multiple agents can work in parallel on different worktrees without conflicts

**Relevance for our framework:**
- **RESEARCH:** Parallel Explore agents (haiku model, read-only, cheap)
- **IMPLEMENT:** general-purpose agents in isolated worktrees per wave task
- **REVIEW:** Parallel review agents (via /simplify or custom)
- **Background test runs:** Spawn background agent to run tests while continuing work
- **VERIFY:** Explore agent to scan codebase against feature spec

---

## 5. Permissions System

Controls which tools Claude can use without asking.

**Configuration in settings.json:**
```json
{
  "permissions": {
    "allow": [
      "Read",
      "Glob",
      "Grep",
      "TodoWrite",
      "Skill(spec)",
      "Skill(develop)",
      "Bash(git status)",
      "Bash(git diff)"
    ],
    "deny": [
      "Bash(rm -rf)"
    ]
  }
}
```

**Permission patterns:**
- Tool name: `"Read"`, `"Write"`, `"Edit"`, `"Bash"`
- Skill-specific: `"Skill(spec)"`, `"Skill(develop)"`
- Bash command patterns: `"Bash(npm test)"`, `"Bash(git *)"`
- MCP tool: `"mcp__<server>__<tool>"`

**How it works:**
- `allow` list: tools/commands that execute without user confirmation
- `deny` list: tools/commands that are always blocked
- Anything not in either list: prompts user for confirmation
- Project settings (`.claude/settings.json`) vs user settings (`~/.claude/settings.json`)

**Relevance for our framework:**
- Allow read tools by default (Read, Glob, Grep) for free exploration
- Allow /spec and /develop skills
- May want to allow test commands: `Bash(npm test)`, `Bash(pytest)`
- Phase-specific permissions could be enforced via hooks (more flexible than static permissions)

---

## 6. MCP Servers

Model Context Protocol servers provide additional tools to Claude.

**Configuration:**
```json
// .claude/settings.json or ~/.claude/settings.json
{
  "mcpServers": {
    "server-name": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-filesystem"],
      "env": {}
    }
  }
}
```

**What MCP servers provide:**
- Additional tools (e.g., database access, browser automation, file system operations)
- Resources (data sources Claude can read)
- Prompts (templates Claude can use)

**Relevance for our framework:**
- Compound Engineering plugin ships 2 MCP servers: Playwright (browser automation) and Context7 (framework docs)
- We don't need custom MCP servers for v1
- Plugin adapters might need to configure MCP servers that plugins depend on

---

## 7. Settings Files

**File locations:**
| File | Scope | Purpose |
|------|-------|---------|
| `.claude/settings.json` | Project (checked in) | Shared project settings: hooks, permissions |
| `.claude/settings.local.json` | Project (gitignored) | Local overrides (personal preferences) |
| `~/.claude/settings.json` | User-global | Global settings across all projects |

**Structure:**
```json
{
  "hooks": {
    "PreToolUse": [...],
    "PostToolUse": [...],
    "SessionStart": [...],
    "Stop": [...]
  },
  "permissions": {
    "allow": [...],
    "deny": [...]
  },
  "mcpServers": {}
}
```

**Merge behavior:** Project settings extend (not replace) user-global settings. Local settings override project settings.

**Relevance for our framework:**
- `.claude/settings.json` is where our hooks and permissions live
- It's checked into git — shared across team
- `/setup-framework` may need to update this file when configuring plugins

---

## 8. Memory System

Claude Code has a built-in auto-memory system.

**How it works:**
- Claude writes notes to `~/.claude/projects/<project>/memory/MEMORY.md` and topic-specific files
- Files under 200 lines are loaded automatically at session start
- Project-scoped: all worktrees/subdirectories in the same git repo share memory
- Claude decides what's worth remembering during normal conversation

**Memory files:**
```
~/.claude/projects/<project>/memory/
├── MEMORY.md            # Main memory entrypoint
├── debugging.md         # Topic: debugging insights
├── api-conventions.md   # Topic: API patterns
└── ...
```

**Relevance for our framework:**
- Auto-memory is orthogonal to our lessons.md — they serve different purposes
- Auto-memory: Claude's own notes about the project (implicit, automatic)
- lessons.md: explicit, structured lessons with rules (part of spec system)
- We should NOT rely on auto-memory for framework state — it's not version-controlled
- All framework state lives in `.spec/` (version-controlled, explicit, shareable)

---

## 9. Built-in Tools

Full inventory of tools available to Claude and subagents.

| Tool | Purpose | Read/Write | Notes |
|------|---------|-----------|-------|
| **Read** | Read file contents | Read | Supports images, PDFs, notebooks |
| **Write** | Create or overwrite files | Write | Requires prior Read of existing files |
| **Edit** | Exact string replacement in files | Write | Preferred over Write for modifications |
| **Glob** | Find files by pattern | Read | `**/*.ts`, `src/**/*.md` |
| **Grep** | Search file contents | Read | Regex, type filters, context lines |
| **Bash** | Execute shell commands | Both | Timeout support, background execution |
| **Agent** | Spawn subagents | Meta | Types: general-purpose, Explore, Plan |
| **WebSearch** | Search the web | Read | Returns search results |
| **WebFetch** | Fetch URL content | Read | Converts HTML to markdown |
| **TodoWrite** | Manage task list | Meta | Visible to user as progress tracker |
| **AskUserQuestion** | Ask user questions | Meta | Multi-choice, multi-select, previews |
| **NotebookEdit** | Edit Jupyter notebooks | Write | Cell-level editing |
| **Skill** | Invoke a skill | Meta | Routes to SKILL.md |

**Tool restrictions by subagent type:**
- `Explore` agents: Read, Glob, Grep, WebSearch, WebFetch, Agent (no Write, Edit, Bash-write)
- `Plan` agents: Same as Explore (read-only)
- `general-purpose` agents: All tools

**Relevance for our framework:**
- Phase gate hooks can only intercept Edit/Write/NotebookEdit (file-modifying tools)
- Bash is harder to gate (can write files via `echo > file`) — hook on Bash would need command parsing
- AskUserQuestion is perfect for the DISCUSS phase (multi-choice, structured)
- TodoWrite for progress tracking during IMPLEMENT
- Agent for all subagent dispatch (RESEARCH, IMPLEMENT, REVIEW)

---

## 10. Plugin System

How plugins are structured and distributed.

**Plugin structure (standard convention):**
```
.agents/skills/<plugin-name>/
├── SKILL.md              # Main skill definition
├── README.md             # Documentation
├── scripts/              # Shell scripts
├── reference/            # Templates, guides
└── ...
```

**Installation:** Copy the skill directory into `.agents/skills/`. No package manager, no registry. Git clone or manual copy.

**What plugins can include:**
- Skills (SKILL.md + supporting files)
- Hook configurations (added to `.claude/settings.json`)
- MCP server configurations
- Templates and reference material

**Known plugins relevant to us:**

| Plugin | Skills Provided | Hooks | MCP Servers |
|--------|----------------|-------|-------------|
| superpowers | brainstorm, writing-plans, tdd, review, debug | Session hooks for phase enforcement | None |
| feature-dev | code-explorer, code-architect, code-reviewer | None | None |
| simplify | simplify (3-agent review) | None | None |
| compound-engineering | ce:plan, ce:work, ce:review, ce:compound, +13 more | Session hooks | Playwright, Context7 |

**Relevance for our framework:**
- `/setup-framework` detects plugins by checking for known paths
- Plugin adapters are routing logic in `/develop` SKILL.md — not separate code
- Plugins don't need to know about us — we route to them
- Feature detection (check if skill exists) > version pinning

---

## Summary: What We Use and How

| Platform Feature | How We Use It | Phase/Component |
|-----------------|---------------|-----------------|
| Skills | `/develop`, `/spec`, `/setup-framework` | All phases |
| PreToolUse hooks | Phase gate enforcement (block writes in wrong phase) | All phases |
| SessionStart hooks | Load phase state, remind about lessons | Session init |
| Stop hooks | LEARN phase auto-extraction (potential) | LEARN |
| Subagents (Explore) | Parallel codebase research, VERIFY scans | RESEARCH, VERIFY |
| Subagents (general-purpose) | Wave-based implementation tasks | IMPLEMENT |
| Subagents (background) | Test runs between waves | IMPLEMENT |
| Worktree isolation | Parallel task execution without conflicts | IMPLEMENT |
| AskUserQuestion | DISCUSS phase structured prompts | DISCUSS |
| TodoWrite | Progress tracking during implementation | IMPLEMENT |
| CLAUDE.md | Framework documentation and rules | Always loaded |
| .spec/.phase file | Phase state persistence | All phases |
| .framework.json | Plugin routing config | All phases |
| Permissions | Allow read tools, skills by default | Session init |

---

## Constraints and Limitations

| Constraint | Impact | Workaround |
|-----------|--------|------------|
| Hooks can only block, not modify | Can't transform tool calls, only allow/deny | Design phases so blocking is sufficient |
| No persistent state between sessions | Phase state must be file-based | `.spec/.phase` file |
| Subagents don't inherit conversation | Must include all context in prompt | Pass spec content + task in prompt |
| Bash hooks are hard to gate | Can't easily prevent `echo > file` during RESEARCH | Accept this limitation; trust phase discipline |
| Skills can't auto-invoke | User must type `/develop` | SessionStart hook can remind |
| No inter-hook communication | Hooks can't pass data to each other | Use files for shared state |
| Hook stderr is the only feedback channel | Limited formatting for block messages | Keep messages concise and actionable |
