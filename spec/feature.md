# Feature layer — ephemeral per-feature specs

Fallback feature flow. Run this only when the spec skill's Routing table sends you here.

The fallback applies only when superpowers is not installed, the user says *self*, `.vibe/spec.json` sets `"featureLayer": "vibe"`, or OpenSpec interchange is required. The default feature layer is `superpowers:brainstorming` then `superpowers:writing-plans` (see [SKILL.md](SKILL.md) § Routing).

Give each **named** unit of work `.spec/features/<name>/`. These specs are **branch-scoped**. Create them in DESIGN. Read them in IMPL. Wrap them up in COMPOUND: promote cross-cutting decisions, then move the folder to `archive/<name>/`. Prompt the user to delete the archive once validation passes. The archive is gone **before the branch merges**. Never read `archive/` for active work — **CODE IS TRUTH**. Keep global, long-living rules in root files: [strategy.md](strategy.md). Compound order: [SKILL.md](SKILL.md) § Layers (Rule 6).

> **Config note:** If `.spec/.config.yaml` sets `suggest-superpowers: false`, skip the "Superpower tip" callout in this file (optional executors only). Run that step yourself. See [SKILL.md § Config](SKILL.md#config) for the full config reference.

## Feature authoring flow

Run steps 1–5 for every named feature. Step 6 is the escape hatch.

### 1. Locate & name

Confirm the feature name with the user. Read root `.spec/product.md`, `.spec/tech.md`, and `.spec/lessons.md` before writing anything. Create `.spec/features/<name>/` when the name is settled.

### 2. Interview for WHAT

Run a dialogue, not a form fill. Cover:

- **Problem / why** — one paragraph; push back on vague goals
- **Scope** — Owns / Does not own / Deferred table; explicit boundaries vs neighbour features
- **Requirements** — `### Requirement: <title>` with SHALL/MUST language; each requirement gets `#### Scenario:` blocks in Given/When/Then form

Reject vague requirements. If the user cannot state observable behaviour, keep interviewing.

Write to `product.md`. Follow [reference/product.md](reference/product.md) § feature product. Omit empty sections.

### 3. Rigor gate

Choose **lite** (product + tech + plan) vs **full** (+ `design.md`):

| Need `design.md` when… | Skip when… |
|---|---|
| UI layout, interaction flows, visual language | Pure backend / script / config |
| API or data contract needs a human-readable spec | Contracts fit in `tech.md` alone |
| Migration or rollout has UX-facing steps | No user-visible surface |

If unsure, choose lite. Never create `design.md` "just in case."

### 4. Sketch HOW

Trace the codebase (existing files, contracts, KTDs). Write **only sections with content** to `tech.md`: paths, interfaces, file layout, risks. Write no speculative boilerplate.

> **Superpower tip:** Delegate tracing to `code-explorer` and approach sketching to `code-architect`. Inject [reference/tech.md](reference/tech.md) § feature tech as the constraint so they know which sections to populate and where merge markers go. Tell the user: *"I will use `code-explorer` to trace the codebase and `code-architect` to sketch the approach. This grounds the HOW section in code."*

When step 3 chose full, write `design.md`. Template: [reference/templates/feature-design.md](reference/templates/feature-design.md). Guide and token format: [reference/design.md](reference/design.md).

### 5. Plan units

Write `plan.md` with stable `feature/n` unit IDs (`### <name>/1`, `### <name>/2`, … per [reference/plan.md](reference/plan.md)):

- Each unit cites requirement IDs from `product.md`
- Each unit names test scenarios and verification evidence (command, test path, behaviour check)
- Same-feature dependencies only — cross-feature order is a whole-feature gate in root `.spec/plan.md` Feature Sequence
- The root Feature Sequence row is added at **COMPOUND** (the promote step), not
  here: root `.spec/plan.md` is write-blocked during `feature.plan` under the
  flow's write invariants. Treat it as a compound-step obligation (see § Merged
  during COMPOUND below).

**Human gate:** confirm units before implementation.

### 6. Skip conditions

Route to `vibe` quick, not this flow, when **all** hold:

- Atomic change — one file or one bounded fix
- No architectural decisions
- No new requirements needing Scope negotiation

If any condition fails, stay in this flow.

---

## Output density

Output density follows the vibe flow's single `style` note (see the flow's `SKILL.md § Style`):
concise, no filler; compress receipts and summaries but never reasoning; security and
irreversible-action warnings stay in full prose; code, paths, and commands stay byte-exact. There
are no per-phase output levels to select — write precise, minimal spec sections in every phase.

Keep the mandatory shape. Drop the padding:

- **product.md** — frontmatter, problem paragraph, Scope table, requirement titles + strength, GWT
  scenarios. Cut rationale essays and worked examples.
- **tech.md** — frontmatter, file paths, interface signatures, risks as one-line bullets. Cut
  implementation narrative and decision history.
- **plan.md** — frontmatter, unit IDs with requirement IDs, verification (command + evidence). Cut
  restated prose.

Compound receipts add the traceability matrix (unit ID → test path → pass/fail), the `validate.sh`
evidence summary, and a draft lesson entry (pattern, rule, tags) for the human to edit before promoting.

---

## Examples in this repo

Feature folders are ephemeral. Write them at design, consume them at impl, delete them
before the branch merges. This repo keeps none. The filled-in root
docs under [`.spec/`](../.spec) (product / tech / design / plan / lessons) are
the standing worked example.

- [`spec/`](SKILL.md) — spec framework (units `spec/n`, delivered — wrapped up; no feature folder)

## Anatomy of `features/<name>/`

| File | Required | Purpose |
|---|---|---|
| `product.md` | yes | What this feature does (requirements, UX) |
| `tech.md` | yes | How it is built (paths, contracts, implementation) |
| `design.md` | optional | UI/UX or design-system fragment for this feature |
| `plan.md` | recommended | Stable unit IDs, dependencies, verification — see [reference/plan.md](reference/plan.md) |
| `research.md` | optional | Discovery artifacts |

**Frontmatter** (shape; adjust paths to your tree):

```yaml
---
type: feature-product   # or feature-tech
feature: <name>
sibling: tech.md          # or product.md
parent: ../../product.md # or ../../tech.md
updated: YYYY-MM-DD
---
```

Full conventions: [reference/product.md](reference/product.md) § feature product, [reference/tech.md](reference/tech.md) § feature tech.

## Lifecycle

```
Created  →  Consumed  →  Merged  →  Archived  →  Deleted (before merge)
   ↑           ↑           ↑           ↑              ↑
 DESIGN      IMPL       COMPOUND    COMPOUND       after validation
 phase       phase      phase       phase          (agent prompts)
```

1. **Created during DESIGN.** Steps 1–4 of the authoring flow → `product.md`, `tech.md`, optional `design.md`.
2. **Planned before IMPL.** Step 5 → `plan.md` with stable IDs; human gate on units.
3. **Consumed during IMPL.** Read feature specs; cite unit IDs in commits and tests; revise with targeted fixes if reality diverges.
4. **Verified against plan.** Evidence checked per unit verification table — not agent assertions alone.
5. **Merged during COMPOUND.** Cross-cutting blocks from `features/<name>/tech.md` promote into root `tech.md` (or branch docs). Feature-only detail does not promote.

   > **Compound note:** The spec skill owns the compound sequence ([SKILL.md § Layers](SKILL.md), Rule 6): promote `<!-- merge -->` blocks → add the root `.spec/plan.md` Feature Sequence row (if absent) and mark it DONE → record a tagged lesson ([strategy.md § Lessons](strategy.md) format) → move the folder to `archive/<name>/` → run `validate.sh` → prompt the user to delete the archive. The archive move is the flow's own work. `superpowers:finishing-a-development-branch` handles only the git close-out (branch merge/lifecycle) after the spec work is done. Never hand it the full compound procedure. It does not know the spec format.

6. **Archived then deleted.** At wrapup, move `.spec/features/<name>/` to `archive/<name>/` as a transient safety net. After validation passes, prompt the user to delete the archive. The folder is gone **before the branch merges**. Never read `archive/` for active work (CODE IS TRUTH). See § Archive and delete.

Without a `/code:feature` workflow, use the same lifecycle: create the folder when scoping, remove it when done.

## Marking content for promotion (tech)

Wrap **cross-cutting** sections so COMPOUND tooling (e.g. `bin/merge-feature.sh` in your project) can promote them into root `tech.md`:

```markdown
<!-- merge -->
## Single routing contract
All workflow phases and adapters call the same flow reader — one JSON shape everywhere.
<!-- /merge -->
```

Keep feature-only sections (one file's line count, one hook's exit code table) **outside** merge markers. Wrap-up discards them.

Details: [reference/tech.md](reference/tech.md) (merge markers, feature `tech.md` sections).

## What stays vs what graduates

| Stays in feature folder (discarded or archived) | Graduates to root / branch |
|---|---|
| Per-hook matcher strings, per-command LOC estimates | "Every hook delegates to keystone" |
| Feature file list for this build | Global file layout diagram |
| One feature's user-visible copy | Reusable design principles in root `product.md` |

If everything you wrote applies to **every** future feature, that content belongs in [strategy.md](strategy.md) / root branch docs. Move it up. Never bury it in a feature folder. Layer rules: [SKILL.md](SKILL.md) § Layers.

## Archive and delete

After promote, move `.spec/features/<name>/` to `archive/<name>/`. The archive is a **transient safety net** for a CI failure right after wrapup. It is **never** a store for active work. Never read `archive/` while building — **CODE IS TRUTH**.

Once validation passes, **prompt the user to delete the archive**. The folder is gone **before the branch merges**. Keep an archive past the branch only with a stated reason (rejected alternatives, plan archaeology worth standalone history). Otherwise delete it.

**Delete checklist:** root `plan.md` Feature Sequence row → DONE with live-surface link; no links pointing at `features/<name>/` or `archive/<name>/`; never restore the folder if validation complains. A wrapped-up feature is absent from `features/`.

Wrap-up sequence: [SKILL.md](SKILL.md) § Layers (Rule 6).

## Templates and validation

- Feature templates: [feature-product](reference/templates/feature-product.md), [feature-tech](reference/templates/feature-tech.md), [feature-plan](reference/templates/feature-plan.md), optional [feature-design](reference/templates/feature-design.md)
- Writing guides: [product](reference/product.md), [tech](reference/tech.md), [plan](reference/plan.md), [design](reference/design.md)

Global layer handoff: [strategy.md](strategy.md).
