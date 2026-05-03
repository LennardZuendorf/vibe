# Engineering Agent — Distilled Insights for shards-code

The prior "Engineering Agent" design overlaps with shards-code in goal but diverges in surface. Below: what's worth keeping. Each entry is a quote or short reference, source-tagged.

---

## Design Principles Worth Keeping

From the prior `tech.md` Design Philosophy section — these map 1:1 onto shards-code's stated principles and the prose is sharper than what's in the new spec:

- **File-based communication.** "All state lives in `.spec/` files. No in-memory state survives session boundaries. Phases read files, not conversation history." (`tech.md`)
- **Shell over code.** "Prefer bash scripts over complex application code. The framework is glue, not a product." (`tech.md`)
- **Hooks enforce, skills orchestrate.** "PreToolUse hooks prevent writes in wrong phases. Skills handle the workflow logic." (`tech.md`) — shards-code softens this to "warn on most, block on two" but the framing is identical.
- **Subagents are disposable.** "Each subagent gets a focused task + minimal context. They return compact summaries, not raw data." (`tech.md`)

From the prior `product.md` Design Principles:

- **Orchestrate, don't reimplement.** "Existing plugins are maintained by their authors. We route to them, not rebuild them. When they ship updates, we get them for free." (`product.md`) — shards-code's Principle #1, stated more crisply here.
- **Front-load the thinking.** "All research, discussion, spec writing, and planning happens in the Design Cluster before any code is written." (`product.md`) — directly applies to shards-code's `DESIGN` sub-cluster within `/code:feature`.
- **Verify, don't rewrite.** "Before implementation, agents scan the codebase to confirm the feature spec is still valid. If something has changed, they flag it — they don't silently rewrite the spec." (`product.md` Decision #4) — defines what `IMPL:VERIFY` should do in shards-code.
- **Cherry-pick the best patterns.** "We adopt proven patterns (GSD's waves, Superpowers' pressure resistance, Compound Engineering's learning phase) as built-in defaults." (`product.md`)

---

## External References / Inspirations

shards-code should not lose sight of what it builds on:

- **GSD (Getting Shit Done)** — wave grouping, gap closure, plan immutability, fresh subagents per task. (ccforeveryone.com/gsd)
- **HumanLayer FIC (Frequent Intentional Compaction)** — "files are the memory, not conversation history." A 10-line spec outweighs 10,000 tokens of chat. Validated on 300K+ LOC Rust codebases. (github.com/humanlayer/advanced-context-engineering-for-coding-agents)
- **Superpowers** — Socratic brainstorming (diverge → converge → commit), TDD enforcement, pressure resistance.
- **Compound Engineering** — the "compound step" / LEARN phase. Treated as a dedicated phase, not a checkbox in review.
- **Simplify** — multi-agent parallel review with three perspectives: Reuse, Quality, Efficiency. shards-code's `IMPL:REVIEW` defers to `ce-code-review` instead, but the three-perspective pattern is a useful fallback.
- **Feature-Dev** — explorer / architect / reviewer agent specializations.

---

## Structural Patterns That Carry Over

### State file format

The prior design used `CLUSTER:PHASE:FEATURE` (e.g., `IMPL:VERIFY:dark-mode`). shards-code uses `<workflow>:<phase>[:<feature>]` (e.g., `feature:IMPL:VERIFY:dark-mode`). The prior format is a strict subset; the new format adds the workflow prefix to distinguish `quick`, `strategy`, and `feature`.

### Feature spec lifecycle

Prior pattern (still applies to shards-code's `/code:feature`):

```
.spec/features/<name>/      ← created during DESIGN:SPEC
  product.md
  tech.md
  research.md (optional)

→ consumed during IMPL:VERIFY + IMPL:WORK (read-only after SPEC)
→ merged into global specs during IMPL:COMPOUND
→ moved to .spec/archive/<name>/ after merge
```

### Cross-cutting merge rules

From `product-design.md` ("What Gets Merged vs Archived"):

| Content | Action |
|---|---|
| Cross-cutting architecture decisions | Merge into global `tech.md` |
| New design patterns | Merge into `product-design.md` or `tech.md` |
| Feature-specific implementation detail | Archive only |
| Lessons learned | Already in `lessons.md` |
| Feature product requirements | Archive only |

This is the contract `bin/merge-feature.sh` should implement during `IMPL:COMPOUND`.

### Spec file types beyond product/tech/plan

From the prior `product.md` and lesson #2 in `lessons.md`:

- `context.md` — business/domain context, upstream of product. Essential for rework projects.
- `lessons.md` — accumulated mistakes; read at session start, written only during COMPOUND.
- `product-design-*.md` with `design` scope — design system docs that cross product/tech (e.g., `#00b054` is both brand identity and a hex value).
- `research/` — discovery artifacts, not specs.
- `docs/` — exhaustive reference material (API maps, data dictionaries).
- `reference/` — visual assets (screenshots, mockups).
- `features/` and `archive/` — see above.

shards-code's `/spec` skill already supports most of these. Worth confirming during Pass 2.

### VERIFY phase contract

From `product-design.md` "VERIFY Phase" section, the checklist:

1. Read feature spec from `.spec/features/<name>/`.
2. Scan codebase for changes since spec was written:
   - Referenced file paths still exist?
   - Interfaces / APIs match spec?
   - Dependencies still available?
3. If drift: amend feature spec with **targeted fixes**, not a rewrite.
4. If major drift: flag to user, may need a mini Design Cluster.

This is the contract for `IMPL:VERIFY` in `/code:feature`. Recommended scope from `questions.md` Q1: file existence + interface match (skip running tests — `IMPL:WORK` will run them).

### Wave-based execution

Plan tasks grouped into dependency waves. Tasks within a wave have no dependencies on each other → parallel subagents. Waves run sequentially. shards-code's plan templates should preserve this.

### Goal-backward verification

From the prior REVIEW phase: don't ask "did we do the tasks?" — ask "what must be TRUE for this feature to work?" then verify each condition. Applies to shards-code's `IMPL:REVIEW`.

### Mini bootstrap for unplanned features

From `product-design.md`: when a new feature emerges mid-project, run a scoped DESIGN cluster (just research + discuss + spec for that feature), amend the global plan, then continue normally. shards-code's `/code:feature <new-name>` already implements this implicitly by always starting at `feature:DESIGN:RESEARCH:<name>`.

---

## Hook Patterns Worth Reusing

From `original/check-phase.sh`:

- `set -euo pipefail` for safe bash.
- `INPUT=$(cat)` to read tool input JSON from stdin.
- Multi-location script discovery (try `$PWD`, `$HOME`, then git root) for portability across machines.
- Exit 0 = allow when the framework isn't installed → **graceful degradation by default**. shards-code should keep this: if `bin/detect-context.sh` is missing, hooks should exit 0, not fail the user's session.
- Exit 2 = hard block, stderr shown to Claude. shards-code uses this only for the two structural blocks.

From `original/settings.json`:

- Permissions allowlist with `Skill(<name>)` entries so the listed skills don't prompt.
- `SessionStart` matcher `"startup"` with an inline `command` is fine for a one-liner; switch to a script when logic grows.
- `PreToolUse` matcher `"Edit|Write|NotebookEdit"` is the right gate.

---

## Decisions From `questions.md` Worth Adopting

The prior design surfaced 15 open questions. Several have recommended answers that map directly to shards-code:

| Q | Recommendation | Relevance to shards-code |
|---|----------------|--------------------------|
| Q1: VERIFY scope | File + interface (no test runs) | `IMPL:VERIFY` contract |
| Q3: Feature spec isolation | Feature spec primary + global read-only | Justifies the "global specs outside COMPOUND" hard block |
| Q6: Plugin version compat | Feature detection (check skill exists before delegating) | Matches `detect-context.sh` "skill not installed → drop + warn" |
| Q9: Plugin uninstalled | Warn but don't block | Matches shards-code's gentle-enforcement principle |
| Q11: Merge conflict during VERIFY | Flag with suggestion (don't auto-rewrite) | `IMPL:VERIFY` should propose, not silently amend |
| Q13: `context.md` for rework projects | Detect greenfield vs rework, scaffold accordingly | `/code:strategy` could ask this |

---

## Lessons (Verbatim)

These lessons are portable wisdom. They were learned while building Engineering Agent but apply to any framework of this shape. shards-code should re-ingest them rather than re-learn them.

> ### Don't reimplement what plugins already do
> **Pattern:** GSD rebuilt everything from scratch (50+ files, custom CLI, 12 agents) when existing skills already handled most of the work. This creates maintenance burden and fragility.
> **Rule:** Always check if an existing plugin/skill handles a capability before building it. Orchestrate first, build only what's missing.
> **Date:** 2026-03-13

> ### Spec framework assumes greenfield — rework projects need more file types
> **Pattern:** Deployed on a Flutter rework project (DIB Travel). product.md and tech.md were insufficient — agents needed business context, current state documentation, reference material (API surface map of 102 endpoints), and visual assets.
> **Rule:** Support optional file types beyond product/tech/plan: `context.md`, `docs/`, `reference/`, `research/`. When bootstrapping a rework project, check for these needs explicitly.
> **Date:** 2026-03-18

> ### Design system docs break the product/tech separation — and that's correct
> **Pattern:** Design tokens (colors, typography, spacing) are simultaneously product decisions and technical values. Forcing design-language.md into pure product or pure tech creates artificial splits.
> **Rule:** Allow a `design` scope type for design system docs. These may contain both product concerns and tech concerns. Principled exception, not a relaxation.
> **Date:** 2026-03-18

> ### For rework projects, plan phases should map to product goals
> **Pattern:** The plan template implies milestones should be independently structured. But for a rework project with 5 clear product goals from the PM, the plan phases mapped 1:1 to those goals. Trying to "derive" independent milestones added no value and broke traceability.
> **Rule:** When product.md defines clear, sequenced goals, plan.md phases should map 1:1 to those goals. Independent milestone derivation is for projects without pre-defined goals.
> **Date:** 2026-03-18

---

## What to Deliberately Leave Behind

For honesty: these aspects of Engineering Agent are NOT being carried forward into shards-code, on purpose.

- **`.framework.json` plugin routing config.** shards-code hard-codes the routing in `bin/detect-context.sh`. If config-driven routing comes back, it's `v1.1` (`.shards/config.json`), not `v1`.
- **`/setup-framework` interactive installer.** Replaced by a single `install.sh` that just symlinks files.
- **The 5/6/8-phase linear `/develop` lifecycle.** Replaced by three commands (`quick`, `strategy`, `feature`), each with its own internal phase structure.
- **Pluggable provider per phase.** shards-code picks specific tools per phase: CE for strategy/plan/review/ship/compound, Superpowers for brainstorming/TDD/subagents, `/spec` for SPEC. Not configurable in v1.
- **Hard-block phase gates everywhere.** Replaced by warnings on stderr + exactly two hard blocks (lessons.md outside COMPOUND, global specs outside COMPOUND, plus direct `.phase` edits).
