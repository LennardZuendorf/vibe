# Research: Real-World Deployment — DIB Travel Rework Project

Updated: 2026-03-18

---

## Context

The engineering-agent spec framework was deployed on a real project: reworking a corporate travel app (Flutter, existing codebase, not greenfield). This produced 7 concrete findings about gaps in the framework, all stemming from the same root cause: **the framework was designed for greenfield projects, but most projects are reworks of existing codebases.**

---

## Finding 1: `context.md` — Business Context Upstream of Product

**What happened:** Created `.spec/context.md` for business context (company identity, market position, customer profile, geographic focus). This answers "what world does this product exist in?" — which is upstream of product.md's "what are we building?"

**Why it matters:** `product.md` assumes you know why the company exists. For a corporate travel app, knowing the company serves Nordic municipalities and SMBs, positions as a digital TMC (not Amex GBT), and aspires to OTA-grade UX is essential context. Without it, agents write features for the wrong audience.

**Gap:** The spec framework has no file type for business/domain context. `product.md` is about the app. `tech.md` is about architecture. Neither covers "what business is this for?"

**Recommendation:** Add `context.md` as an optional entrypoint. It holds company/domain/market context that's stable across the project lifetime and informs product decisions. Product.md can then focus purely on the app.

---

## Finding 2: `research/` — Codebase Audit Artifacts

**What happened:** Created `.spec/research/` with 8 detailed reports (architecture, state management, API layer, Flutter best practices, testing gaps, code quality, DI/config, executive summary) produced by parallel subagents during RESEARCH phase.

**Why it matters:** Before writing meaningful specs for a rework, you need deep understanding of the current state. These findings are too detailed for tech.md but too important to lose. They're referenced by tech.md and plan.md but don't belong inside them.

**Gap:** The framework already acknowledges `research/` as a RESEARCH phase output, but doesn't formally define it as a supported directory type. The `/spec` skill doesn't cover it.

**Recommendation:** Formally define `research/` as a supported subdirectory for discovery/audit artifacts. Especially critical for rework projects where understanding current state is prerequisite to planning future state.

**Note:** This aligns with our existing Design Cluster model — RESEARCH produces artifacts in `.spec/research/` that inform SPEC and PLAN.

---

## Finding 3: `docs/` — Reference Documentation

**What happened:** Created `.spec/docs/api-surface.md` — a complete map of 102 API endpoints with HTTP methods, paths, file locations, request/response models, and HTTP client details.

**Why it matters:** This is neither a product spec nor a tech spec — it's factual reference material. It's exhaustive, meant to be consulted during implementation, not read end-to-end. It doesn't describe what we're building or how; it documents what exists.

**Gap:** The spec framework has no concept of reference documentation. Everything is either a spec (product/tech/plan) or a lesson. API maps, data dictionaries, integration docs, and environment configs don't fit any existing type.

**Recommendation:** Add `docs/` as a supported subdirectory for reference material. These files support specs but aren't specs — they're factual inventories consulted during implementation.

---

## Finding 4: `reference/` — Visual Assets

**What happened:** Created `.spec/reference/` to store screenshots of the current web platform and competitor apps.

**Why it matters:** Design specs need visual references. "Match the web design" is useless without showing what the web design looks like. Screenshots of competitor UX as a benchmark, plus screenshots of the existing platform, are essential context for design-related specs.

**Gap:** The spec framework has no concept of visual assets. Everything is markdown. But design work requires images.

**Recommendation:** Add `reference/` as a supported subdirectory for visual assets (screenshots, mockups, design exports, competitor examples). Branch docs like `product-design.md` can reference files in this directory.

---

## Finding 5: Design Language Docs Cross the Product/Tech Line

**What happened:** Created `product-design-language.md` that captures design tokens (colors, typography, spacing) AND component patterns (buttons, cards, inputs) AND mobile adaptation notes. It straddles the product/tech boundary.

**Why it matters:** Design tokens are inherently cross-cutting. The color `#00b054` is simultaneously a product decision (brand identity) and a technical value (hex code in theme file). Forcing this into pure product OR pure tech creates artificial splits that make the doc less useful.

**Gap:** The spec framework enforces a hard line between product specs (zero code) and tech specs (zero UX opinions). Design systems break this rule by nature — they are the interface between product and tech.

**Recommendation:** Add `design` as a recognized scope type alongside `product` and `tech`. Design docs are explicitly allowed to contain both "what it looks like" (product concern) and "exact values to implement" (tech concern). This is a principled exception, not a relaxation of the rule.

---

## Finding 6: Plan Phases Map 1:1 to Product Goals

**What happened:** `plan.md` phases were structured as 1:1 mappings to the 5 goals in `product.md`, not independently derived milestones.

**Why it matters:** The plan template implies milestones should be independently structured. But for rework projects with clear sequenced goals from a PM, the plan should directly reflect those goals. This makes tracing trivial: product goal → plan phase → tech tasks.

**Gap:** The plan template doesn't acknowledge that for rework/refactor projects, phases often map 1:1 to product goals. It presents milestones as if they should always be derived fresh.

**Recommendation:** Note in the global plan template that goal-driven plan phases (mapping 1:1 to product goals) are valid and preferred when the product spec defines clear sequenced goals.

---

## Finding 7: Current State in Product Specs

**What happened:** `product.md` included a "Current State" section describing what's broken, alongside the forward-looking vision and goals.

**Why it matters:** For rework projects, "what we build" is meaningless without "what exists and why it's inadequate." An agent reading only the future vision won't understand why the architecture needs to change. The current state section provides the contrast that makes the vision concrete.

**Gap:** The product spec template is forward-looking only. It describes what to build, not what exists. For greenfield this is fine. For reworks, the gap between current and target is the entire point.

**Recommendation:** Add an optional "Current State" or "Starting Point" section to the product.md template. Essential for rework projects, can be omitted for greenfield.

---

## Synthesis: Greenfield vs Rework

The spec framework was designed for greenfield. Rework projects (arguably more common) need additional file types:

| Need | Greenfield | Rework |
|------|-----------|--------|
| Business context | Often obvious | Must be documented (context.md) |
| Current state | Doesn't exist | Must be audited (research/, current state in product.md) |
| Reference material | Minimal | Extensive (docs/ for API maps, data dictionaries) |
| Visual assets | Optional | Essential (reference/ for screenshots, competitor benchmarks) |
| Design system | Can build fresh | Must document existing (design scope docs) |
| Plan structure | Derive milestones | Often maps to product goals |

**The framework should handle both without requiring different configurations.** The solution is additional optional file types and template sections, not a separate rework mode.

---

## Impact on Spec Directory Structure

```
.spec/
├── context.md                     # NEW: optional business/domain context
├── product.md                     # Entrypoint: what & why (+ current state for reworks)
├── tech.md                        # Entrypoint: how
├── plan.md                        # Entrypoint: implementation roadmap
├── product-design.md              # Branch: functional design
├── product-design-language.md     # Branch: design system (design scope — crosses product/tech)
├── lessons.md                     # Accumulated learnings
│
├── research/                      # FORMALIZED: discovery/audit artifacts from RESEARCH phase
│   ├── architecture.md
│   ├── api-layer.md
│   └── ...
│
├── docs/                          # NEW: reference documentation
│   ├── api-surface.md
│   ├── data-dictionary.md
│   └── ...
│
├── reference/                     # NEW: visual assets
│   ├── current-web-platform.png
│   ├── competitor-ux.png
│   └── ...
│
├── features/                      # Feature specs (existing)
│   └── ...
├── archive/                       # Archived features (existing)
│   └── ...
```
