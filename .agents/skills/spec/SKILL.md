---
name: spec
description: Navigate and maintain design specs in .spec/. Use before writing code or making decisions to load architecture, requirements, or implementation details. Also use when specs need updating after implementation changes.
user-invocable: true
argument-hint: [product|tech|update|validate]
allowed-tools: Read, Bash(bash .claude/skills/spec/scripts/validate.sh)
---

# Spec System

`.spec/` contains all design documentation. Product specs describe *what and why*. Tech specs describe *how*. Strict separation — product docs have zero code, tech docs have zero UX opinions.

## Current state

!`ls -1 .spec/*.md 2>/dev/null | while read f; do name=$(basename "$f"); type=$(grep -m1 "^type:" "$f" 2>/dev/null | sed 's/type: //'); scope=$(grep -m1 "^scope:" "$f" 2>/dev/null | sed 's/scope: //'); echo "- $name ($type: $scope)"; done`

## Routing

Based on `$ARGUMENTS`:

- **`product`** -> Read `.spec/product.md`. If the task needs UX detail, load the relevant product branch doc.
- **`tech`** -> Read `.spec/tech.md`. Then load the relevant tech branch doc.
- **`update`** -> Read [reference/updating.md](reference/updating.md), then make changes.
- **`validate`** -> Run `bash .claude/skills/spec/scripts/validate.sh`.
- **No argument** -> Determine the right entrypoint from conversation context. When uncertain, read both entrypoints (not branches).

| Task type | Load first | Then load |
|-----------|-----------|-----------|
| UI, layout, modes, UX, shortcuts | `product.md` | Relevant product branch |
| Feature scoping, priorities, phases | `product.md` | — |
| Code, components, architecture, infra | `tech.md` | Relevant tech branch |
| Full context for a new feature | `product.md` + `tech.md` | Relevant branch(es) |

## Rules

1. Always start with the entrypoint. Branch docs assume you've read the parent.
2. Never load all spec files at once.
3. Product docs: no code, no implementation details. *What and why.*
4. Tech docs: code welcome, reference product docs for the *why.*

## Maintaining specs

- **Updating** existing specs: [reference/updating.md](reference/updating.md)
- **Creating** new spec docs: [reference/creating.md](reference/creating.md)
- **Templates**: [reference/templates/product-branch.md](reference/templates/product-branch.md) and [reference/templates/tech-branch.md](reference/templates/tech-branch.md)
- **Validating** consistency: Run `bash .claude/skills/spec/scripts/validate.sh`
