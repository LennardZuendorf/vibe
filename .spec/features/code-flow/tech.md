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
    ├── code-amend/SKILL.md
    └── code-setup/SKILL.md
```

---

## State Machine

`state-machine.json` contains:

- allowed states and transitions
- required `code-*` skill for each phase
- optional caveman communication level (`lite`, `full`, `ultra`)
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
code-setup -> spec + skill-creator
  Caveman: lite
  Output: .agents/flow/*, .agents/skills/code-*/, AGENTS.md, CLAUDE.md, baseline .spec/

code-strategy -> superpowers:brainstorming -> spec
  Caveman: lite
  Output: .spec/product.md, .spec/tech.md, .spec/design.md, .spec/plan.md

code-feature -> superpowers:brainstorming + writing-plans -> spec
  Caveman: lite during design, full during implementation
  Output: .spec/features/<feature>/{product,tech,design,plan}.md

code-quick -> systematic-debugging or TDD -> verification-before-completion
  Caveman: ultra for triage, full for non-trivial fixes
  Output: workspace edits, optional .spec/quick/<slug>.md

code-verify -> verification-before-completion + systematic-debugging
  Caveman: full
  Output: evidence summary, no spec edits unless failures change scope

code-compound -> spec + finishing-a-development-branch
  Caveman: lite
  Output: lessons, promoted root specs, archived feature folder

code-amend -> spec + receiving-code-review or brainstorming
  Caveman: lite
  Output: targeted spec changes or revised flow state
```

## External Skill Matrix

| Code Skill | Primary External Skills | Caveman Level |
|---|---|---|
| `code-setup` | `spec`, `skill-creator`, optional `superpowers:brainstorming` | `lite` |
| `code-strategy` | `superpowers:brainstorming`, `spec`, optional explorer subagents | `lite` |
| `code-feature` | `superpowers:brainstorming`, `superpowers:writing-plans`, `spec`, `superpowers:test-driven-development`, `superpowers:subagent-driven-development`, `superpowers:executing-plans` | `lite` for design, `full` for implementation |
| `code-quick` | `superpowers:systematic-debugging`, `superpowers:test-driven-development`, `superpowers:verification-before-completion` | `ultra` for triage, `full` for work |
| `code-verify` | `superpowers:verification-before-completion`, `superpowers:requesting-code-review`, `superpowers:systematic-debugging` | `full` |
| `code-compound` | `spec`, `superpowers:finishing-a-development-branch` | `lite` |
| `code-amend` | `spec`, `superpowers:receiving-code-review`, `superpowers:brainstorming` | `lite` |
