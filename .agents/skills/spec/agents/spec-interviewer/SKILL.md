---
name: spec-interviewer
description: WHAT-phase interview delegator — injects feature.md Interview constraints, then delegates to superpowers:brainstorming or self-executes.
user-invocable: false
allowed-tools:
  - Read
---

# spec-interviewer

WHAT-phase interview delegator. Injects the spec interview constraints before handing off to `superpowers:brainstorming`. Offers delegation; does not silently invoke it.

## Orders

1. Read `.agents/skills/spec/feature.md` — specifically `§ Interview for WHAT` (steps 1–2: problem/why, scope, requirements with GWT scenarios)
2. Read `.spec/product.md`, `.spec/tech.md`, `.spec/lessons.md` for project context
3. Tell the user: *"I can run this as a `superpowers:brainstorming` session with spec format constraints pre-loaded — the constraint context is `feature.md § Interview for WHAT`. Want me to?"*
4. **On yes:** invoke `superpowers:brainstorming` with the constraint context injected
5. **On no or unavailable:** conduct the WHAT interview directly using `feature.md § Interview for WHAT` as the constraint — same output format, no degraded quality signal

## Output (either path)

A partial `features/<name>/product.md` draft for human review:
- Problem paragraph
- Scope table (Owns / Does not own)
- At least one `### Requirement:` with RFC-2119 keyword + one `#### Scenario:` GWT block

## Invariants

- MUST inject `feature.md § Interview for WHAT` before delegating — never delegate without constraint context
- MUST offer delegation to user (not silently run brainstorming)
- MUST self-execute if user declines or superpowers:brainstorming unavailable
- Output is a draft for review, not a committed file
