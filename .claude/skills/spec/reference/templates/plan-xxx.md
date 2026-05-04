---
type: branch
parent: plan.md
scope: {topic}
covers: {feature-specific milestones, tasks, validation criteria}
updated: {YYYY-MM-DD}
---

# Plan: {Feature/Area Name}

<!--
FILENAME: plan-{topic}.md
Examples: plan-editor.md, plan-auth.md, plan-file-sync.md
-->

{One-paragraph summary: what this sub-plan covers and why it needs its own plan separate from the main plan.md.}

**Parent plan:** [plan.md](plan.md)
**Product spec:** [product-{topic}.md](product-{topic}.md)
**Tech spec:** [tech-{topic}.md](tech-{topic}.md)

---

## Scope

What this sub-plan covers:
- {Feature area 1}
- {Feature area 2}

What stays in the main plan:
- {Items that remain in plan.md}

---

## Pre-Implementation Checklist

- [ ] Product spec reviewed: [product-{topic}.md](product-{topic}.md)
- [ ] Tech spec reviewed: [tech-{topic}.md](tech-{topic}.md)
- [ ] Dependencies from main plan satisfied
- [ ] Architecture decisions resolved

---

## Milestones

| Milestone | Goal | Sessions | Risk |
|-----------|------|----------|------|
| **{topic}-M1** | {Goal} | {N} | {Low/Med/High} |
| **{topic}-M2** | {Goal} | {N-N} | {Low/Med/High} |

---

## {topic}-M1: {Name}

**Goal:** {One sentence.}
**Sessions:** {N} | **Risk:** {Level}
**Depends on:** {Main plan milestone or "none"}

Tasks:
- [ ] {Concrete, verifiable task}
- [ ] {Concrete, verifiable task}

**Done when:** {Validation criteria.}

---

## {topic}-M2: {Name}

**Goal:** {One sentence.}
**Sessions:** {N-N} | **Risk:** {Level}
**Depends on:** {topic}-M1

Tasks:
- [ ] {Task}
- [ ] {Task}

**Done when:** {Validation criteria.}

---

## Progress

| Milestone | Status | Sessions Used | Estimate |
|-----------|--------|---------------|----------|
| {topic}-M1 | NOT STARTED | 0 | {N} |
| {topic}-M2 | NOT STARTED | 0 | {N-N} |
