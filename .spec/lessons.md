# Lessons

Mistakes made and rules to prevent repeating them. Written during `compound`,
read on entry to `*.design` and `*.triage` so past mistakes shape new work.
Tags make entries retrievable — scan for tags matching the work in hand.

### Spec strictness: warn-first, then migrate
**Pattern:** New validate.sh checks shipped as errors immediately; dogfood repo's legacy feature plans failed validation before migration, blocking the harness from validating itself.
**Rule:** Ship structural validators warn-first; promote warn→error only after live specs are migrated during compound. Pair every validator with a behaviour test in `spec/tests/run.sh`.
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

### Uninstall must surgically invert the install into shared dirs, and the test must discriminate
**Pattern:** `install.sh` copies vibe files into *shared* dirs (`.claude/commands`, `.claude/hooks`) the user may also populate. A naïve `rm -rf "$dir"` uninstall would take the user's co-located files with it. The first cut removed the right files but the uninstall tests only asserted that *shipped* files were gone — a review found that swapping the surgical `remove_shipped` for `rm -rf` still passed every assertion (a false-negative on a data-loss path).
**Rule:** An uninstaller must delete only the paths the installer created (per-file inverse of the copy), never blanket-remove a shared directory; pruning *emptied* dirs is fine. Pair every preservation guarantee with a **discriminating** test — one that fails if the safety code is replaced by the naïve destructive version (drop a user file into each shared dir, run uninstall, assert it survives *and* the shipped file is gone). Reuse the tested marker-pairing guard for the managed instruction block; never re-implement it.
**Tags:** install-tooling, uninstall, data-safety, shared-dirs, discriminating-tests
**Date:** 2026-07-03

### The dogfood repo is a privileged target — eval on a fresh, non-git install
**Pattern:** `orders.sh` (the flow's headline per-turn feature) worked in every in-repo test and on the source repo, but a README-only stranger eval installing into a fresh `mktemp -d` found it silently returned `state=unknown` for every state. Root cause: it located the skills dir by searching upward for a `.spec`/`.git` marker, and a fresh install target has neither — while the source repo (and the dogfood repo the tests run in) always does. The bug was invisible precisely because every test environment was privileged.
**Rule:** A tool that will be *installed elsewhere* must be tested from a representative fresh target (a bare `mktemp -d`, no `.git`, no `.spec`), not just the source/dogfood repo. Prefer self-location relative to the script's own path over repo-root markers the target may lack. Run a periodic "stranger" eval (fresh agent, docs-only, throwaway sandbox) as a release gate — it exercises the install-target reality the in-repo suites cannot.
**Tags:** release-docs, stranger-eval, self-location, install-target, dogfood, orders
**Date:** 2026-07-03

### Compound is where drift is born — enforce it mechanically
**Pattern:** flow-mvp shipped and merged (PR #14) with every unit still marked NOT STARTED in its own `plan.md`, no root `plan.md` Feature Sequence row, and no compound at all — the feature folder was never archived and its Delivered note never written. The gates object, a quick-flow compound state, the precedence section, the evidence-receipt verify tooth, and caveman demotion were all demonstrably live in the tree, yet the `.spec/` memory said the work had never begun. End-of-feature discipline is exactly the moment attention lapses (the work "feels done"), and nothing in the harness forced the compound — the audit caught it weeks later.
**Rule:** Compound must be mechanically enforced, not trusted to discipline. A drift check (`spec/scripts/check-drift.sh`, CI-wired after `validate.sh`) fails when a directory under `.spec/features/` has no row in the root `.spec/plan.md`, and flags any `NOT STARTED` unit left in a feature `plan.md`. Hand-written assertion counts are errored the same way — they rot silently. A green suite is not a compounded feature; make the missing-compound state impossible to merge past.
**Tags:** compound, drift, dogfood, ci
**Date:** 2026-07-09

<!-- Format for each lesson:
### [Short description]
**Pattern:** What went wrong and why
**Rule:** The concrete rule that prevents this
**Tags:** comma, separated, keywords
**Date:** YYYY-MM-DD
-->
