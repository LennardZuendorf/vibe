# Engineering Agent — Archived Prior Art

This directory holds a previous design that lived in `shards-code` before the repo was repurposed.

The prior framework — internally called **"Engineering Agent"** — attempted the same broad goal as shards-code (a meta-prompting framework for spec-driven AI development) but with a different surface area:

- A single `/develop` command driving a 5-phase lifecycle (`RESEARCH → SPEC → PLAN → IMPLEMENT → REVIEW`)
- Two clusters (`DESIGN` and `IMPL`) with 8 phases total
- A `.framework.json` config file routing each phase to a pluggable provider
- Bundled `/spec` skill and intended adapters for `superpowers`, `feature-dev`, `simplify`, `compound-engineering`
- A `/setup-framework` command for interactive plugin selection

shards-code took a different turn: three commands (`/code:quick`, `/code:strategy`, `/code:feature`), a single `bin/detect-context.sh` keystone, gentle hook enforcement, and orchestration of Compound Engineering + Superpowers + the bundled `/spec` skill.

## What's in here

- **`insights.md`** — the distilled keepers. Patterns, principles, prose, references, and decisions from the prior design that are still relevant to shards-code. Read this before consulting the originals.
- **`original/`** — verbatim copies of the prior repo's source files at the moment of archival. Useful as reference but should not be loaded as live specs.

## What was dropped

- `.spec/research/` (11 phase analyses, agreements doc, deployment findings, platform reference) — out of date relative to shards-code's design.
- The `/develop` skill source itself was never present in this repo; the bundled `/spec` skill remains live at `.agents/skills/spec/`.

## Why this exists

Two reasons: (1) **lessons are portable** — the lessons file in `original/lessons.md` contains four entries that directly inform shards-code's first design principle ("Don't reimplement what plugins already do"), and (2) **prior decisions deserve respect** — several open questions in `original/questions.md` had recommended answers that shards-code can adopt or deliberately diverge from.
