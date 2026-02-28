# Creating New Spec Documents

## When to create a new spec

Create a new branch doc when:
- A feature area is large enough to need its own dedicated spec (100+ lines of detail)
- The content doesn't fit naturally into an existing branch doc
- Multiple implementation tasks will reference this spec independently

**Don't create a new doc for:**
- Small additions (add a section to an existing branch doc instead)
- Temporary specs or experiments
- Anything that should live in code comments or CLAUDE.md

## Naming convention

```
{area}-{topic}.md
```

- **area**: `product` or `tech`
- **topic**: short, semantic name for the feature area

Examples: `tech-search.md`, `product-onboarding.md`, `tech-collaboration.md`

## Template

```markdown
---
type: branch
parent: {product.md or tech.md}
scope: {short-identifier}
covers: {comma-separated list of what this doc covers}
updated: {YYYY-MM-DD}
---

# {Title}

{One-paragraph summary of what this doc covers and its relationship to the parent.}

**Parent:** [{parent}]({parent}) | **Sibling:** [{sibling}]({sibling})

{For tech docs:}
**UX specs:** For what this looks/behaves like, see [product-design.md](product-design.md).

{For product docs:}
**Tech implementation:** For code and architecture, see [{tech-doc}]({tech-doc}).

---

## 1. {First section}

...

---

## N. Open Questions

1. **{Question}** — {Context and recommendation if any.}
```

## After creating

1. **Link from parent entrypoint.** Add the new doc to the parent's `children:` frontmatter list AND its "Branch Documents" table.

2. **Link from siblings.** If the new doc is closely related to an existing branch, add a sibling cross-reference in both docs' headers.

3. **Update CLAUDE.md.** Add the new doc to the "Branch Documents" table in the project root CLAUDE.md.

4. **Update the spec skill map.** Edit `.claude/skills/spec/SKILL.md` to add the new doc to the "Quick map" tree and "Routing table".

## Checklist

```
- [ ] File created with correct frontmatter
- [ ] Named with {area}-{topic}.md convention
- [ ] Parent entrypoint updated (children list + branch table)
- [ ] Sibling cross-references added
- [ ] CLAUDE.md branch table updated
- [ ] Skill SKILL.md map and routing table updated
- [ ] No content duplicated from existing docs (link instead)
```
