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
