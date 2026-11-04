# Spec setup and config

Read this on `/spec setup`, or when a route depends on a config key.

## Config

One file holds every key: `.vibe/spec.json` in the project root. A missing file or
key means the default. A legacy `<root>/.config.yaml` is read only for a key that
`.vibe/spec.json` lacks. No binary reads these three keys; you read them.

| Key | Default | Effect |
|---|---|---|
| `featureLayer` | `"superpowers"` | `"vibe"` → fallback feature folders (`<root>/features/<feature>/`) |
| `suggest` | `true` | `false` → do not dispatch `feature-dev:code-explorer` or `feature-dev:code-architect` |
| `superpowers` | `{}` | `{"<skill>": false}` → that delegate is unavailable; `brainstorming` or `writing-plans` false → fallback |

The same file holds keys that `vibe-spec` reads (`root`, `tooth`, `migrate`).
Keep them when you write the file.

## Setup

On `/spec setup`:

1. `.vibe/spec.json` present → show the three keys, update only the one your human
   partner names, confirm, write.
2. Absent → ask three questions, conversationally:
   - **Q1 — Delegates:** "Detected: <superpowers skills in your skill list>. Turn any of them off? none | <names>" → `superpowers`
   - **Q2 — Feature layer:** "Detected: <superpowers skills in your skill list>. Use the fallback layer instead? yes | no" → `featureLayer`
   - **Q3 — Suggestions:** dispatch the optional explorers (`feature-dev:code-explorer`, `feature-dev:code-architect`) when a step can use them? Default: yes → `suggest`
3. Show the summary. Write nothing before your human partner confirms.
4. On confirm: write `.vibe/spec.json` (merge the keys; keep every other key). Run
   `vibe-spec init` only when `vibe-spec root` exits 3 (no spec root). Report what
   it created. Next: `/spec strategy` or `/spec feature <feature>`.

## Flat layout

A flat root (`vibe-spec root --json` prints `"layout": "flat"`) keeps every root doc
at the top of the spec root. On a layered root, follow [layered.md](layered.md) instead.

| Doc | Holds | Guide |
|---|---|---|
| `<root>/product.md` | story, users, requirements, non-goals | [product.md](product.md) |
| `<root>/tech.md` | stack, layout, contracts, risks | [tech.md](tech.md) |
| `<root>/design.md` | design language, UX conventions | [design.md](design.md) |
| `<root>/plan.md` | Feature Sequence, gates, current focus | [plan.md](plan.md) |
| `<root>/lessons.md` | tagged lessons | — |
| `<root>/{product,tech}-<topic>.md` | cross-cutting topic docs | [product.md](product.md) |

## Templates (flat layout only)

Copy into the spec root:

| Template | For |
|---|---|
| [product.md](templates/product.md) · [tech.md](templates/tech.md) · [design.md](templates/design.md) · [plan.md](templates/plan.md) | root docs |
| [product-topic.md](templates/product-topic.md) · [tech-topic.md](templates/tech-topic.md) · [plan-topic.md](templates/plan-topic.md) | cross-cutting topic docs (`type: product-topic` / `tech-topic` / `plan-topic`) |
| [feature-product.md](templates/feature-product.md) · [feature-tech.md](templates/feature-tech.md) · [feature-design.md](templates/feature-design.md) · [feature-plan.md](templates/feature-plan.md) · [research.md](templates/research.md) | fallback feature folder (any layout) |
