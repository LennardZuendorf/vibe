# Writing Product Specs

Product specs answer two questions: **What** are we building and **Why** does it matter? They describe the user's experience, not the developer's implementation. If you catch yourself writing code, component names, or architecture details — stop. That belongs in a tech spec.

## The Product Mindset

When writing a product spec, you are the user's advocate. You describe what they see, what they do, and what happens. You explain the reasoning behind design choices. You define success from the user's perspective.

A good product spec lets someone who has never seen the codebase understand exactly what the product does and why each decision was made.

## Structure

### Entrypoint: `product.md`

The product entrypoint is the single page that answers "what is this project?" It contains:

- **One-liner** — what the product is in one sentence
- **Design principles** — 3-5 guiding rules that resolve ambiguity
- **Target user** — who this is for (be specific)
- **Feature overview** — table of features with priority and links to branch docs
- **Implementation phases** — high-level phases with exit criteria
- **Non-goals** — what you are explicitly NOT building
- **Key product decisions** — numbered list of decisions with rationale
- **Branch documents table** — links to all product branch docs with summaries

Design principles are the most important section. When you face an ambiguous decision during implementation, the principles should resolve it. "Editor-first, agent-assisted" immediately tells you the editor panel takes priority over the chat panel.

### Branch Docs: `product-{topic}.md`

Branch docs deep-dive into a specific product area. Create one when:
- A feature area needs more than ~100 lines of detail
- Multiple implementation tasks will reference this area independently
- The content doesn't fit naturally into an existing branch doc

Each branch doc has:
- **Frontmatter** with type, parent, scope, covers, updated
- **Summary paragraph** explaining what this doc covers
- **Parent/sibling links** for navigation
- **Detailed sections** describing the feature area
- **Open Questions** for unresolved decisions

## Style Rules

**Do:**
- Describe what the user sees and does
- Use concrete examples: "The file tree shows `.md` files in alphabetical order"
- State decisions with rationale: "Auto-save uses VS Code style (debounce 1.5s) because explicit save dialogs interrupt flow"
- Use tables for structured feature comparisons
- Include keyboard shortcuts and interaction patterns
- Write non-goals to prevent scope creep

**Don't:**
- Reference components, hooks, atoms, or any code constructs
- Mention file paths in the codebase
- Describe technical architecture or data flow
- Use words like "implement", "render", "component", "API"
- Write pseudocode or code examples of any kind

**Example — Describing a Feature:**
```markdown
## Auto-Save

Notes save automatically using VS Code-style behavior:
- Debounce 1.5s after the last keystroke
- Save on blur, tab switch, and window focus loss
- Cmd+S works but is rarely needed
- No save dialogs, ever

Dirty indicator (dot on tab title) appears immediately on edit, disappears on save.
```

Notice: no mention of atoms, debounce implementations, event handlers, or file I/O. Pure user experience.

## Cross-References

Product specs should reference their tech counterparts:
```markdown
**Tech implementation:** For code and architecture details, see [tech-infrastructure.md](tech-infrastructure.md).
```

This creates a clean bridge: product says WHAT, tech says HOW, and the reader can navigate between them.

## When to Update vs Create

- **Update** an existing branch doc when adding details to an existing feature area
- **Create** a new branch doc when a genuinely new product area emerges that doesn't belong in any existing doc
- **Never** create a branch doc for something small enough to be a section in an existing doc

## Template

See [templates/product.md](templates/product.md) for entrypoint template and [templates/product-xxx.md](templates/product-xxx.md) for branch template.
