---
type: feature-tech
feature: code-flow
sibling: product.md
parent: ../../tech.md
updated: 2026-05-14
---

# Feature: Code Flow — Architecture

The code flow is a platform-neutral runtime layer under `.agents/flow` plus a
family of `code-*` agent skills.

**Parent:** [../../tech.md](../../tech.md)
**Requirements:** [product.md](product.md)
**Design:** [design.md](design.md)

---

## Files

```text
.agents/
├── flow/
│   ├── state-machine.json
│   ├── state.example.json
│   └── scripts/
│       ├── detect-context.sh
│       ├── set-state.sh
│       └── validate-state.sh
└── skills/
    ├── code-strategy/SKILL.md
    ├── code-feature/SKILL.md
    ├── code-quick/SKILL.md
    ├── code-verify/SKILL.md
    ├── code-compound/SKILL.md
    └── code-amend/SKILL.md
```

---

## State Machine

`state-machine.json` contains:

- allowed states and transitions
- required `code-*` skill for each phase
- canonical `.spec/` paths for planned outputs
- allowed write surfaces
- exit predicates used by adapters or verification skills

Mutable cursors use `state.json` in target projects. This framework repo may
ship `state.example.json` and static definitions, while target cursors should be
gitignored unless the user explicitly wants to share state.

---

## Skill Shim Pattern

Each `code-*` skill follows the same internal sequence:

1. Read `.agents/flow/state.json` and relevant root or feature specs.
2. Confirm or transition state through `set-state.sh`.
3. Delegate to the specialized skill with path injection.
4. Run the relevant validation or verification.
5. Summarize evidence and next legal states.

---

## Delegation Examples

```text
code-strategy -> superpowers:brainstorming -> spec
  Output: .spec/product.md, .spec/tech.md, .spec/design.md, .spec/plan.md

code-feature -> superpowers:brainstorming + writing-plans -> spec
  Output: .spec/features/<feature>/{product,tech,design,plan}.md

code-verify -> verification-before-completion + systematic-debugging
  Output: evidence summary, no spec edits unless failures change scope

code-compound -> spec + finishing-a-development-branch
  Output: lessons, promoted root specs, archived feature folder
```
