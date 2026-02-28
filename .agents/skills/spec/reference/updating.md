# Updating Spec Documents

## When to update

Update `.spec/` documents when:
- A design decision changes a spec
- Implementation reveals the spec was wrong or incomplete
- An open question gets resolved
- A phase is completed or priorities shift

**Do NOT update for:**
- Session-specific context or in-progress work
- Temporary experiments
- Anything that belongs in code comments

## Update workflow

1. **Read the relevant spec first.** Never edit a spec you haven't read in this session.

2. **Identify the right file.** Changes go in branch docs, not entrypoints, unless it's a top-level change (e.g. new phase, new non-goal).

3. **Make the edit.** Use the Edit tool on `.spec/<filename>`. Keep the same style and structure as existing content.

4. **Bump the `updated:` date** in the YAML frontmatter to today's date.

5. **Check cross-references.** If your change affects something referenced by another doc, update that reference too. Common links:
   - `product.md` links to product branch docs
   - `tech.md` links to tech branch docs
   - Branch docs cross-link each other as siblings

6. **If a new branch doc is needed**, see [creating.md](creating.md) instead.

## Resolving open questions

Each spec has an "Open Questions" section. When a question is resolved:

1. Remove it from the open questions list
2. Add the decision to the relevant section of the spec
3. If the decision has cross-cutting impact, check sibling docs

## Style rules

- Product docs: no code, no implementation details. Describe *what* and *why*.
- Tech docs: code examples welcome. Describe *how*. Reference product docs for the *why*.
- Keep tables aligned and consistent with existing formatting.
- Use `->` not `→` in markdown (renders consistently).
- Use actual directory structure for file paths.
- Names (atoms, components, IPC channels, etc.) must be consistent across all docs.
