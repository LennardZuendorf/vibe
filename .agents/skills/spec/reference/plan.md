# Writing Plans

Plans sequence work. They reference product, tech, design, and feature specs — they do not duplicate them.

## Two layers

| Layer | File | Scope |
|---|---|---|
| **Root** | `.spec/plan.md` | Milestones, feature map, boundaries, unit-prefix registry, critical path, spec-vs-repo gaps |
| **Feature** | `.spec/features/<name>/plan.md` | Unit tables, dependencies, verification, feature-scoped open questions |

**Rule:** root plan = *what ships when* across features. Feature plan = *how this feature is sliced* for implementation. If you are writing step-by-step tasks for one feature, they belong in the feature plan.

Optional branch plans (`plan-{topic}.md`) follow the root pattern for cross-cutting workstreams (e.g. infrastructure rollout).

---

## When to write

| Moment | Write |
|---|---|
| Strategy / roadmap | Root `plan.md` — milestones, feature list, critical path |
| Feature design complete | Feature `plan.md` — units with stable IDs |
| Scope changes mid-flight | Amend the relevant plan; add units, never renumber |
| Compound | Archive feature plan with the feature; promote cross-cutting sequencing into root `plan.md` |

Human gate: root plan milestones and feature unit tables should be approved before `impl`.

---

## Milestones (root)

Milestones are delivery phases — **not** implementation units.

| Convention | Example |
|---|---|
| Sequential | M0, M1, M2, … |
| Parallel tracks | M4a, M4b (same phase, independent streams) |
| Exit criteria | Observable behaviour or artifact — not "units done" |

Each milestone names which features participate and what "done" means. Feature plans map their units to milestone IDs in the unit table's **Milestone** column.

---

## Unit IDs (feature)

Stable, citeable identifiers for implementation slices. Used in commits, tests, and verify evidence during `impl`.

### Format

```
{PREFIX}{N}
```

| Part | Rule |
|---|---|
| **PREFIX** | 2–4 uppercase letters from the feature slug (`spec-framework` → `SF`, `agent-instructions` → `AI`) |
| **N** | Non-negative integer, assigned once, **never renumbered** on reorder |

### Rules

1. **One prefix per feature** — register in root plan's unit-prefix table.
2. **Never renumber** — deprioritized work stays in the table; add new IDs for new work.
3. **One unit = one reviewable slice** — testable, committable, independently verifiable.
4. **Cite during impl** — commit subjects and test names reference the unit (`feat(flow): VF1 add orders blocks`).
5. **Aliases allowed** — when restructuring, document equivalence (`VF1 = legacy U8`) in the feature plan; do not delete old IDs from git history references.
6. **Cross-feature deps** — document in both the feature plan dependency table and root critical path.

### Unit table columns

| Column | Purpose |
|---|---|
| **ID** | Stable prefix + number |
| **Summary** | One line — what ships |
| **Depends on** | Prior units (same or other features) |
| **Milestone** | Root milestone this unit rolls into |

### Verification table

Pair each unit with observable evidence — command, test path, script output, or behaviour check. Verify phase checks this table, not agent assertions.

---

## Feature boundaries

Every feature `product.md` includes a **Scope** table:

| | |
|---|---|
| **Owns** | Paths, scripts, contracts this feature may write |
| **Does not own** | Neighbour features and root concerns — explicit negatives prevent bleed |

Root `plan.md` summarizes boundaries across features (diagram or table). Feature plans do not redefine product scope — they reference `product.md`.

---

## Standard feature doc headers

Consistent cross-links across the feature folder:

```markdown
**Parent:** [../../product.md](../../product.md)   # or ../../tech.md in tech.md
**Architecture:** [tech.md](tech.md)              # product.md
**Requirements:** [product.md](product.md)        # tech.md
**Design:** [design.md](design.md)                # when present
**Plan:** [plan.md](plan.md)                      # when present
**Related:** [../other-feature/product.md](../other-feature/product.md)  # when coupled
```

---

## Root plan sections

1. **Features** — table linking every active feature's product, tech, plan
2. **Feature boundaries** — who owns what (compressed from feature Scope tables)
3. **Milestones** — M0… with goals, participating features, exit criteria
4. **Unit prefixes** — registry mapping prefix → feature → plan file
5. **Critical path** — hard cross-feature dependencies
6. **Spec vs implementation** — honest drift inventory with owning unit
7. **Current focus** — active milestone and next gate

Do **not** duplicate feature unit tables in the root plan.

---

## Feature plan sections

1. **Unit prefix** — local declaration + link to root registry
2. **Units** — full unit table
3. **Dependencies** — within-feature and cross-feature blocks
4. **Spec vs implementation** — feature-scoped gaps
5. **Verification** — evidence per unit
6. **Open questions** — planning blockers only

Template: [templates/feature-plan.md](templates/feature-plan.md)

---

## Frontmatter

**Root plan:**
```yaml
---
type: plan
parent: product.md
children: []   # feature plan paths when registered
updated: YYYY-MM-DD
---
```

**Feature plan:**
```yaml
---
type: feature-plan
feature: <name>
parent: ../../plan.md
updated: YYYY-MM-DD
---
```

List feature plans in root `children:` when you want validate to track them (optional but recommended).

---

## Anti-slop

Plans sequence decisions — they are not implementation dumps.

- **Repo-relative paths only** — link `.spec/features/<name>/plan.md`, not absolute paths.
- **No process exhaust** — no brainstorming, triage notes, or "how we got here" in the plan file.
- **No qualifiers or hedges** — units state what ships; delete "maybe", "TBD unless".
- **Omit empty sections** — skip placeholder tables and headers with no rows.
- **Decisions, not code** — no code snippets, pseudocode, or pasted API shapes; link to `tech.md`.

---

## Style rules

- Reference specs by link — never paste requirements or architecture inline.
- Prefer tables for units, milestones, and boundaries.
- Keep "Current focus" to 2–3 sentences — it rot fast; bump `updated:` when it changes.
- Open questions belong in the plan that owns the blocker; close them by decision or new unit.

---

## Example (multi-feature repo)

Root `.spec/plan.md` might show:

| Prefix | Feature | Plan |
|---|---|---|
| SF | spec-framework | `.agents/skills/spec/` + root `plan.md` M0 (wrapped up) |
| VF | vibe-flow | features/vibe-flow/plan.md |
| AI | agent-instructions | features/agent-instructions/plan.md |

Critical path excerpt:

```
SF0 (spec skill) → VF0 (state machine) → VF1 (per-turn orders) → AI0 (wrap AGENTS.md)
```

Feature `vibe-flow/plan.md` unit row:

| ID | Summary | Depends on | Milestone |
|---|---|---|---|
| VF1 | Per-turn orders in vibe-* skills; strip frozen injects | VF0 | M2 |

This pattern scales: add features, register prefixes, extend milestones — without rewriting history.
