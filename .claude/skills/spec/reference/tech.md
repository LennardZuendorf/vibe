# Writing Tech Specs

Tech specs answer one question: **How** do we build it? They describe architecture, code patterns, data flow, and implementation details. Code examples are welcome. UX opinions are not — reference the product spec for that.

## The Tech Mindset

When writing a tech spec, you are speaking to the developer who will implement this (often a future version of yourself or another agent). You describe what code goes where, what patterns to follow, what already exists and shouldn't be rebuilt, and what the risks are.

A good tech spec lets someone start implementing immediately without guessing at architecture. It eliminates "where does this go?" and "how does this connect?" questions.

## Structure

### Entrypoint: `tech.md`

The tech entrypoint is the architecture overview. It contains:

- **Design philosophy** — 2-3 technical principles (e.g., "Inherit everything from upstream. Only build what's new.")
- **Architecture overview** — directory tree showing project structure, highlighting new/modified/inherited parts
- **Tech stack** — what's inherited and what's added, with versions
- **Build vs Inherit vs Integrate** — table showing lines of code by source, so you know what you're actually writing
- **Key patterns** — brief description of each major pattern with links to branch docs for details
- **Risks & mitigations** — table of technical risks and how to handle them
- **Branch documents table** — links to all tech branch docs with summaries

The architecture overview is the most important section. A clear directory tree with annotations ("NEW", "inherited", "extended") immediately tells you where new code goes and what to leave alone.

### Branch Docs: `tech-{topic}.md`

Branch docs deep-dive into a specific technical area. Create one when:
- An area has enough implementation detail to warrant its own document
- Multiple milestones or tasks reference this area
- The codebase area is complex enough that implementation guidance prevents mistakes

Each branch doc has:
- **Frontmatter** with type, parent, scope, covers, updated
- **Summary paragraph** explaining what this doc covers technically
- **Parent/sibling links** for navigation
- **Detailed sections** with code examples, API surfaces, data flow descriptions
- **Open Questions** for unresolved implementation decisions

## Style Rules

**Do:**
- Include real code examples with file path comments
- Show interface/type definitions for key data structures
- Use tables for API surfaces, IPC channels, configuration options
- Reference actual file paths in the codebase
- Describe data flow with concrete steps
- List what already exists and should NOT be rebuilt
- State technical decisions with trade-off rationale

**Don't:**
- Describe user experience or interaction design
- Use words like "intuitive", "clean", "user-friendly"
- Make UX decisions — reference the product spec instead
- Write aspirational architecture — describe what you're actually building

**Example — Describing an Implementation:**
```markdown
## File Watcher

Agent writes to disk -> main process detects change via `fs.watch` -> renderer reloads content into tiptap.

Conflict prevention:
- Auto-save triggers before every agent query
- Own-write tracking: main process records timestamps of writes it initiated
- Writes within 500ms of own-write are ignored (prevents echo reloads)

```typescript
// apps/electron/src/main/services/files.ts
const ownWrites = new Map<string, number>(); // path -> timestamp

function onFileChange(filePath: string) {
  const ownWrite = ownWrites.get(filePath);
  if (ownWrite && Date.now() - ownWrite < 500) return; // skip own write
  notifyRenderer('file:changed', { path: filePath });
}
```
```

Notice: real file paths, real code, concrete algorithm. No UX opinions about how the user perceives the reload.

## Cross-References

Tech specs should reference their product counterparts:
```markdown
**UX specs:** For what this looks and behaves like, see [product-design.md](product-design.md).
```

And link to inherited architecture docs when relevant:
```markdown
## Inherited Architecture Docs
| Document | Covers |
|----------|--------|
| [`apps/electron/AGENTS.md`](../apps/electron/AGENTS.md) | Electron app architecture |
```

## When to Update vs Create

- **Update** an existing branch doc when adding implementation detail to an existing technical area
- **Create** a new branch doc when a genuinely new technical area emerges (new service, new integration layer, new subsystem)
- **Never** create a branch doc for a single component or utility — that's too granular

## Template

See [templates/tech.md](templates/tech.md) for entrypoint template and [templates/tech-xxx.md](templates/tech-xxx.md) for branch template.
