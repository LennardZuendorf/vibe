---
type: feature-tech
feature: spec-framework
sibling: product.md
parent: ../../tech.md
updated: 2026-06-06
---

# Feature: Spec Framework — Architecture

The spec framework is a vendorable agent skill plus scripts/templates that manage
the `.spec/` document tree.

**Parent:** [../../tech.md](../../tech.md)
**Requirements:** [product.md](product.md)
**Design:** [design.md](design.md)
**Plan:** [plan.md](plan.md)

---

## Files

```text
.agents/skills/spec/
├── SKILL.md
├── strategy.md
├── feature.md
├── scripts/
│   ├── setup.sh
│   ├── validate.sh
│   └── list-specs.sh
└── reference/
    ├── product.md
    ├── tech.md
    ├── plan.md
    └── templates/
        ├── product.md
        ├── tech.md
        ├── design.md
        ├── plan.md
        ├── feature-product.md
        └── feature-tech.md
```

---

## Contracts

- `setup.sh` resolves templates relative to its own script directory so it works
  when vendored at `.agents/skills/spec` or installed at `~/.agents/skills/spec`.
- `validate.sh` treats `product.md`, `tech.md`, `design.md`, and `plan.md` as
  root entrypoints.
- Feature validation requires `product.md` and `tech.md`; optional `design.md`
  and `plan.md` must still have frontmatter when present.
- `design.md` supports a token-plus-prose structure inspired by
  `google-labs-code/design.md`: YAML token groups such as `colors`,
  `typography`, `rounded`, `spacing`, and `components`, followed by markdown
  rationale.

---

## Merge Behavior

Feature-specific detail stays in `.spec/archive/<feature>/`. Cross-cutting
technical decisions may be promoted into root or branch specs during compound.
Promotable tech blocks use `<!-- merge -->` markers.

---

## Lessons Retrieval (D8)

`lessons.md` is a retrieval surface, not just an append log. Each lesson carries a
one-line `**Tags:**` field (free-form keywords, e.g. `auth, async, migrations`) plus
Pattern/Rule/Date. Keeping it KISS: one file, tagged entries, no external index or
schema.

**Ownership split (D8):**
- **spec-framework** — lesson *format*, `setup.sh` bootstrap template, optional
  future `validate.sh` warn for missing Tags.
- **vibe-flow** — *runtime read* on `*.design` / `*.triage` entry ([R10](../vibe-flow/product.md));
  agents scan Tags by keyword against the work in hand.
- **vibe-compound** — writes new tagged lessons; `regen-active-rules.sh` projects a
  separate top-5 digest into `AGENTS.md` (pinned/date — not tag-based).

If the file grows unwieldy, shard by category into `lessons/<category>.md` — but
not before it actually hurts.

**Known gap:** `setup.sh` lessons bootstrap omits `**Tags:**` — tracked as SF0 in
[plan.md](plan.md).
