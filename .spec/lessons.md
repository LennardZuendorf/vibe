# Lessons

Mistakes made and rules to prevent repeating them. Written during `compound`,
read on entry to `*.design` and `*.triage` so past mistakes shape new work.
Tags make entries retrievable — scan for tags matching the work in hand.

### Spec strictness: warn-first, then migrate
**Pattern:** New validate.sh checks shipped as errors immediately; dogfood repo's legacy feature plans failed validation before migration, blocking the harness from validating itself.
**Rule:** Ship structural validators warn-first; promote warn→error only after live specs are migrated during compound. Pair every validator with a behaviour test in `tests/spec/run.sh`.
**Tags:** spec, validate, templates, dogfood
**Date:** 2026-06-06

### Single-source the per-turn orders; don't duplicate them in the machine
**Pattern:** The per-state inject orders lived in BOTH `state-machine.json` (`inject` strings) and the `vibe-*` skill bodies. Two copies of the same orders drift apart, and nothing flags the divergence.
**Rule:** Author the orders once, in the linked skill, as a machine-extractable `<!-- vibe:orders:<state> -->` block; carry `inject: null` in the machine for skill-owning states (only `idle` keeps an inline fallback); resolve via `orders.sh` (cursor → `skill` link → block → interpolate `<feature>` only, so the prompt cache stays byte-stable). Hooks are thin shells over that resolver.
**Tags:** vibe-flow, d12, inject, prompt-cache, single-source
**Date:** 2026-06-18

### A Claude Code plugin cannot bundle skills outside ./skills/
**Pattern:** vibe's skills are canonically under `.agents/skills/`, but the plugin manifest has no `skills` path field and bans `../` in component paths (`commands`/`agents`/`hooks`/`mcpServers` only). Pointing the plugin at `.agents/skills/` is impossible.
**Rule:** Let the *plugin* carry only the Claude-specific runtime wiring it uniquely provides (`commands` + `hooks` via `${CLAUDE_PLUGIN_ROOT}`); deliver the platform-neutral core (the `spec`/`vibe` skill and `.agents/skills/vibe`) as project files through `install.sh`. "Single install" = run the installer. Keeps adapters thin and the core canonical.
**Tags:** platform-adapters, plugin, install, claude-code, adapters
**Date:** 2026-06-18

### An installer must preserve per-project runtime state across a re-copy
**Pattern:** `install.sh` copied the core then unconditionally `rm`'d the target's `.agents/skills/vibe/state.json` and re-seeded from the template, silently resetting a live mid-flow cursor to idle on every re-run — while advertising "idempotent". The cursor is the one file that is per-project runtime state, not managed core.
**Rule:** A provisioner that refreshes managed files must snapshot per-project runtime state (the flow cursor) before the copy and restore it after, seeding only when genuinely absent. "Idempotent" has to hold for *user* state, not just managed files; pin it with a regression test that a live cursor (`feature.impl <feature>`) survives a re-install.
**Tags:** platform-adapters, install, idempotency, cursor, state
**Date:** 2026-06-20

### Marker-bounded merge must validate marker pairing before rewriting
**Pattern:** `merge-agents.sh` entered its replace path on a substring `grep` for both markers regardless of order; a file with the markers reversed (end before start) made the awk silently drop all trailing user content — breaking the script's own "never touch content outside the markers" guarantee.
**Rule:** Any tool that rewrites a region between managed markers must confirm the markers exist as exact lines AND that start precedes end before mutating — refuse (never mangle) on reversed/overlapping markers, and always write via temp + atomic rename. Pair it with a test that a reversed-marker file is left byte-untouched.
**Tags:** agent-instructions, merge, markers, content-safety, atomic
**Date:** 2026-06-20

### Skill design: promote superpowers proactively, remain self-sufficient
**Pattern:** When a skill can delegate to superpowers or subagents, two failure modes appear: (1) the skill silently self-executes every step, never surfacing that a better executor exists — users never learn the tools; (2) the skill hard-couples to superpowers and fails or degrades silently when they're unavailable.
**Rule:** Skills SHOULD proactively offer their optimal executor at each step — "I can use X for this, want me to?" — and MUST self-execute from their constraint documents if the user declines or the executor is unavailable. The order is always: offer first, self-suffice second. Never silently skip the offer; never block on the answer.
**Tags:** spec, superpowers, subagents, skill-design, interoperability
**Date:** 2026-06-21

### Script self-location: search for markers, don't count hops
**Pattern:** `orders.sh` and `regen-active-rules.sh` resolved the repo root with a fixed number of `dirname` hops tuned to the old `.agents/skills` nesting; after the `flow/` move the same scripts silently hit generic fallbacks or overshot the root when invoked via their canonical path — the symlinked invocation still worked, masking the breakage.
**Rule:** Scripts reachable through compat symlinks must locate the repo root by upward marker search (`.spec`/`.git`), never fixed hop counts; pin with path-parity tests asserting byte-identical output via both real and symlinked invocation.
**Tags:** monorepo-split, symlinks, self-location, path-parity, prompt-cache
**Date:** 2026-07-03

<!-- Format for each lesson:
### [Short description]
**Pattern:** What went wrong and why
**Rule:** The concrete rule that prevents this
**Tags:** comma, separated, keywords
**Date:** YYYY-MM-DD
-->
