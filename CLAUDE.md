# Engineering Agent — Meta-Prompting Framework

Spec-driven AI engineering. Orchestrate skills. No skipping phases. No code without specs.

## Response Style

Default: caveman full mode. Override with "normal mode" for architecture/design sessions.
Never caveman: security warnings, destructive ops, irreversible actions.

## Think First

1. Read task. Think. Produce numbered plan.
2. State which skill handles this (see skill map).
3. Wait for approval on plans touching >2 files.
4. Act. Report one-line diff receipt.

---

## Skill Map — Route Explicitly, Don't Improvise

### Research / Exploration
- Codebase search (4+ files, no writes) → `caveman:cavecrew-investigator` subagent
- External docs/frameworks → `ce-framework-docs-researcher` or `ce-best-practices-researcher`

### Execution
- 1–2 file surgical edits → `caveman:cavecrew-builder` subagent
- UI / web components → `frontend-design` skill
- Feature lifecycle → `/develop` skill (phase-gated, spec-driven)
- Spec write/read → `/spec` skill

### Review
- Code review → `caveman:cavecrew-reviewer` subagent
- Multi-agent quality pass → `/simplify` skill
- New/modified skills → `skill-creator` skill

---

## Subagent Rules

Spawn when: 4+ files to read, research with no writes, 3+ independent parallel tasks.
Prefer caveman subagents (investigator/builder/reviewer) — emit ~60% fewer tokens.
Return: compact summary only — what changed, what's next.

---

## Development Workflow

```
/develop <feature>
  RESEARCH    → caveman:investigator × N parallel agents
  SPEC        → /spec (product.md → tech.md → branch docs)
  PLAN        → wave-grouped tasks, dependencies explicit
  IMPLEMENT   → wave by wave, subagents for independent tasks
  REVIEW      → /simplify + goal-backward verification
```

Phase state tracked in `.spec/.phase`. Never skip. Never write code outside IMPLEMENT phase.

---

## Hard Rules

1. **Specs before code** — no implementation without `.spec/` entries
2. **Read before write** — always read specs + lessons first
3. **Phase gates enforced** — hooks block writes in wrong phase
4. **Lessons mandatory** — update `.spec/lessons.md` after every correction
5. **Compact at ~50 turns** — `/compact` on task switch or long sessions

## Context Files

- @.agents/skills/spec/SKILL.md — spec system
- @.agents/skills/develop/SKILL.md — lifecycle skill
- @.claude/settings.json — hooks + permissions
