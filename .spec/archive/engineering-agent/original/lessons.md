# Lessons

Mistakes made and rules to prevent repeating them. Review at the start of every session.

<!-- Format for each lesson:
### [Short description]
**Pattern:** What went wrong and why
**Rule:** The concrete rule that prevents this
**Date:** YYYY-MM-DD
-->

### Don't reimplement what plugins already do
**Pattern:** GSD rebuilt everything from scratch (50+ files, custom CLI, 12 agents) when existing skills already handled most of the work. This creates maintenance burden and fragility.
**Rule:** Always check if an existing plugin/skill handles a capability before building it. Orchestrate first, build only what's missing.
**Date:** 2026-03-13

### Spec framework assumes greenfield — rework projects need more file types
**Pattern:** Deployed on a Flutter rework project (DIB Travel). product.md and tech.md were insufficient — agents needed business context (who is this company?), current state documentation (what exists and why it's broken?), reference material (API surface map of 102 endpoints), and visual assets (screenshots of current platform and competitor UX). These don't fit any existing spec type.
**Rule:** Support optional file types beyond product/tech/plan: `context.md` for business context, `docs/` for reference material, `reference/` for visual assets, and `research/` for codebase audit artifacts. When bootstrapping a rework project, check for these needs explicitly.
**Date:** 2026-03-18

### Design system docs break the product/tech separation — and that's correct
**Pattern:** Design tokens (colors, typography, spacing) are simultaneously product decisions and technical values. Forcing design-language.md into pure product (no code allowed) or pure tech (no UX opinions) creates artificial splits that make the doc less useful. A color like `#00b054` is both brand identity and a hex value for the theme file.
**Rule:** Allow a `design` scope type for design system docs. These may contain both product concerns (what it looks like) and tech concerns (exact values). This is a principled exception, not a relaxation of the product/tech rule.
**Date:** 2026-03-18

### For rework projects, plan phases should map to product goals
**Pattern:** The plan template implies milestones should be independently structured. But for a rework project with 5 clear product goals from the PM, the plan phases mapped 1:1 to those goals. Trying to "derive" independent milestones added no value and broke traceability.
**Rule:** When product.md defines clear, sequenced goals (common in rework projects), plan.md phases should map 1:1 to those goals. The plan adds task breakdown and exit criteria, not new structure. Independent milestone derivation is for projects without pre-defined goals.
**Date:** 2026-03-18
