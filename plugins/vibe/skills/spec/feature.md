# Feature layer — ephemeral per-feature specs

Fallback feature flow. Run this only when the spec skill's Routing table sends you here.

The fallback applies only when superpowers is not installed, your human partner says *self*, `.vibe/spec.json` sets `"featureLayer": "vibe"`, or OpenSpec interchange is required. The default feature layer is `superpowers:brainstorming` then `superpowers:writing-plans` (see [SKILL.md](SKILL.md) § Routing).

Give each **named** unit of work `<root>/features/<feature>/` (`<root>` is the spec root; `vibe-spec root` prints it). These specs are **branch-scoped**. Create them in design. Read them in implementation. Wrap them up at the end: promote cross-cutting decisions, then remove the folder. The folder is gone **before the branch merges**. Never read `archive/` for active work — **CODE IS TRUTH**. Keep global, long-living rules in root files: [strategy.md](strategy.md).

Wrap-up: spec SKILL.md § Wrap-up.

Config: spec reference/setup.md § Config.

## Feature authoring flow

Run step 0 first. Run steps 1–5 for every named feature that passes it.

### 0. Size check

Use the Feature, bounded or Typo row of SKILL.md § Routing instead when all hold:

- One file or one bounded fix
- No architectural decision
- No new requirement

If any condition fails, stay in this flow.

### 1. Locate & name

Confirm the feature name with your human partner. Read the spec root's `product.md`, `tech.md`, and `lessons.md` before writing anything. The root Feature Sequence row exists with status `IN PROGRESS` before design; add it in strategy. If it is missing, stop and route to strategy. Create `<root>/features/<feature>/` when the name is settled.

### 2. Interview for WHAT

Run a dialogue, not a form fill. Cover:

- **Problem / why** — one paragraph; push back on vague goals
- **Scope** — Owns / Does not own / Deferred table; explicit boundaries vs neighbour features
- **Requirements** — `### Requirement: <title>` with SHALL/MUST language; each requirement gets `#### Scenario:` blocks in Given/When/Then form

Reject vague requirements. If your human partner cannot state observable behaviour, keep interviewing.

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

Trace the codebase (existing files, contracts, KTDs). Write **only sections with content** to `tech.md`: paths, interfaces, file layout, risks. Write no speculative boilerplate. Write only the template's REQUIRED sections; omit empty ones.

Delegate tracing to `feature-dev:code-explorer` and approach sketching to `feature-dev:code-architect` when they are installed and `suggest` is not false. Give them [reference/tech.md](reference/tech.md) § feature tech as the constraint. Without them, do the step yourself.

When step 3 chose full, write `design.md`. Template: [reference/templates/feature-design.md](reference/templates/feature-design.md). Guide and token format: [reference/design.md](reference/design.md).

### 5. Plan units

Write `plan.md` with stable `<feature>/n` unit IDs (`### <feature>/1`, `### <feature>/2`, … per [reference/plan.md](reference/plan.md)):

- Each unit cites requirement IDs from `product.md`
- Each unit names test scenarios and verification evidence (command, test path, behaviour check)
- Same-feature dependencies only — cross-feature order is a whole-feature gate in the root `plan.md` Feature Sequence

**Human gate:** your human partner confirms units before implementation.

---

## Output density

Output density follows the vibe flow's single `style` note (see the flow skill's `style` note):
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

Compound receipts add the traceability matrix (unit ID → test path → pass/fail), the `vibe-spec validate`
evidence summary, and a draft lesson entry (pattern, rule, tags) for your human partner to edit before promoting.

---

## Examples in this repo

Feature folders are ephemeral. Write them at design, consume them at implementation, delete them
before the branch merges. This repo keeps none. The filled-in root docs in the spec root
(product / tech / design / plan / lessons) are the standing worked example.

## Anatomy of `features/<feature>/`

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
feature: <feature>
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
 design     implementation  wrap-up    wrap-up      wrap-up
```

1. **Created during design.** Steps 1–4 of the authoring flow → `product.md`, `tech.md`, optional `design.md`.
2. **Planned before implementation.** Step 5 → `plan.md` with stable IDs; your human partner confirms the units.
3. **Consumed during implementation.** Read feature specs; cite unit IDs in commits and tests; revise with targeted fixes if reality diverges.
4. **Verified against plan.** Evidence checked per unit verification table — not agent assertions alone.
5. **Merged, archived, deleted at wrap-up.** Cross-cutting blocks from `features/<feature>/tech.md` promote into root `tech.md` (or topic docs). Feature-only detail does not promote. The folder is gone **before the branch merges**. Never read `archive/` for active work (CODE IS TRUTH).

Wrap-up: spec SKILL.md § Wrap-up.

Without the flow, use the same lifecycle: create the folder when scoping, remove it when done.

## Marking content for promotion (tech)

Wrap **cross-cutting** sections so `vibe-spec promote <feature>` can promote them into root `tech.md`:

```markdown
<!-- merge -->
## Single routing contract
All workflow phases and adapters call the same flow reader — one JSON shape everywhere.
<!-- /merge -->
```

Keep feature-only sections (one file's line count, one hook's exit code table) **outside** merge markers. Wrap-up discards them.

Details: [reference/tech.md](reference/tech.md) (merge markers, feature `tech.md` sections).

## What stays vs what graduates

| Stays in feature folder (discarded or archived) | Graduates to root / topic doc |
|---|---|
| Per-hook matcher strings, per-command LOC estimates | "Every hook delegates to keystone" |
| Feature file list for this build | Global file layout diagram |
| One feature's user-visible copy | Reusable design principles in root `product.md` |

If everything you wrote applies to **every** future feature, that content belongs in [strategy.md](strategy.md) / root topic docs. Move it up. Never bury it in a feature folder. Layer rules: [SKILL.md](SKILL.md) § Layers.

## Archive and delete

Wrap-up step 4 moves a fallback folder to `archive/<feature>/` as a transient safety net for a CI failure right after wrap-up. It is **never** a store for active work. Never read `archive/` while building — **CODE IS TRUTH**.

Once validation passes, ask your human partner to delete the archive. The folder is gone **before the branch merges**. Keep an archive past the branch only with a stated reason (rejected alternatives, plan archaeology worth standalone history).

**Delete checklist:** root `plan.md` Feature Sequence row → DONE with live-surface link; no links pointing at `features/<feature>/` or `archive/<feature>/`; never restore the folder if validation complains. A wrapped-up feature is absent from `features/`.

## Templates and validation

- Feature templates: [feature-product](reference/templates/feature-product.md), [feature-tech](reference/templates/feature-tech.md), [feature-plan](reference/templates/feature-plan.md), optional [feature-design](reference/templates/feature-design.md)
- Writing guides: [product](reference/product.md), [tech](reference/tech.md), [plan](reference/plan.md), [design](reference/design.md)

Global layer handoff: [strategy.md](strategy.md).
