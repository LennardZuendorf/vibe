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

### A single-source parity test must pin prose to the code, not to a sibling doc
**Pattern:** flow-legibility's write invariants live in three places: enforced in `detect-context.sh decide`, restated in the `<!-- vibe:doctrine -->` block, and rendered in the AGENTS.md template. The first "single-source" test only compared the doctrine block and the template *to each other*, and only on the union of state *names* — so it passed even when a rule was reassigned to the wrong state (e.g. lessons.md writable in a state `decide` actually blocks). Two prose copies agreeing proves nothing about the policy they both claim to describe; the code was never in the loop.
**Rule:** Any doc that restates a code-enforced policy must be parity-tested against the CODE, per rule, not against a sibling doc. Extract each rule's allow-set from the enforcer (`detect-context.sh decide`) and assert the prose's per-rule state set matches it exactly — a set-equality check, not a name-union. A prose↔prose test only catches copy-paste rot; it silently permits rule-reassignment drift, which is the failure that actually misleads an agent.
**Tags:** spec, single-source, discriminating-tests, doctrine, parity
**Date:** 2026-07-18

### Injected context is append-only — budget it and class it by trigger
**Pattern:** The flow's injection design treated hook output as if it were a refreshable status line. It is not: `UserPromptSubmit` stdout enters the conversation layer, which is append-only, so a byte-stable per-turn order accumulates one copy per turn for the whole session — and the `Stop` gate's stuck-phase predicate fired *every* non-idle turn, queueing a redundant `vibe-warn:` line into every subsequent prompt. Two adjacent errors came from the same wrong mental model: `doctrine.sh` appended a live `Cursor:` line to `SessionStart`, whose output is *replayed* rather than re-run on `--resume` (so it is stale by construction), and the write invariants were restated in seven prose locations while the guard already enforced them — against documented guidance that a bloated instruction file makes Claude "ignore your actual instructions".
**Rule:** Treat every injection channel as a context budget with an explicit line cap, and class payload by trigger: *level* (every turn — the minimum that must always be true), *edge* (only when the cursor changes — orders, contracts, retrieved lessons), *event* (only when something happened — drift, warnings). Never put live state on `SessionStart`; it replays stale on resume. Never state in always-on prose what a hook enforces — render it in the guard's verdict at the moment of violation, where it is transient, free, and actionable. Lint the budgets in CI; discipline will not hold them.
**Tags:** injection, context-budget, hooks, prompt-cache, doctrine, agents-md
**Date:** 2026-08-10

### A guard is only as strong as the capability it bans, not the spelling it matches
**Pattern:** The duplicate-primitive scan failed review three times running. Each round closed exactly the hole reported and left a smaller one of the same kind: it began matching three *spellings* of "re-derive a primitive path"; inverted to banning *ingredients* (filename literals, layout constants) — and that round exported a `cursorPath()` helper to avoid taking a waiver, creating a legal spelling of the forbidden thing; then banned the helper *identifiers* — and widened ingredients to multi-token regexes while the matcher still ran **per line**, so ordinary Prettier wrapping evaded. Separately, nothing scanned `engine/tests/`, so `await import('../tests/helpers.mjs')` reached a working duplicate cursor reader in one line. Every round believed it was done, and every round's fix was the previous round's next hole.
**Rule:** When a guard is evaded, do not add a pattern for the evasion — that buys exactly one round. Close the *capability*: state the invariant as "no module may obtain X by any means unless allowlisted", then find the mechanical property that makes it true regardless of syntax. Match against the whole comment-stripped file, never per line. Close the scanned set under whatever relation the attacker can traverse (here, module resolution — which covers static `import`, `import()`, `require`, and `createRequire` without naming any of them). Pin exemptions by exact line *and occurrence count*, never by file. And require the implementer to produce the list of evasions it tried against its **own** fix, including the ones that failed — the round that finally held was the first to produce that artifact.
**Tags:** js-core, discriminating-tests, static-analysis, guards, structural-closure, mutation-testing
**Date:** 2026-08-13

### A check that examines nothing must fail loudly, never pass quietly
**Pattern:** This feature produced the same shape five times. A test harness reported `ok` from a bare `return`. Thirty-one parity tests asserted nothing when `jq` was absent and counted as passes. The bash-3.2 lint enumerated **zero** files in a git tree with an empty index, and twelve nonexistent paths when its root was a repo subdirectory — passing in both, so a release tarball could be declared clean having scanned nothing. The jq half of the matrix could vanish entirely with both CI legs green. And while verifying a fix, the controller's own mutation regex matched a line a refactor had renamed: the suite ran unmutated and returned green, which looked exactly like a caught mutant.
**Rule:** Every guard must assert its own population — a floor on what it examined, and that the things it examined exist. Absence of findings is only evidence when presence of *input* is proven. Extend this to your own verification: after planting a mutant, confirm it actually landed (`git diff --numstat`) before believing the result, and prefer structural floors over hand-written counts, which rot. A green that cannot distinguish "checked and clean" from "checked nothing" is not a green.
**Tags:** js-core, vacuous-tests, guards, ci, mutation-testing, verification
**Date:** 2026-08-13

### Porting a script destroys the oracle that proves the port — freeze it first
**Pattern:** `runGuardHook` and `runGateHook` — the two hooks that can *block* a tool call or a turn — were ported by overwriting their bash originals in place. Every other command kept its oracle and was parity-tested against it; these two silently could not be, and no per-unit review noticed, because from inside each unit nothing was missing. The whole-branch review restored the oracles from `main` and found 30 real divergences: the engine blocked where bash did not, a latent block-loop that could wedge a session.
**Rule:** When replacing an implementation, copy its predecessor into the test tree as a frozen oracle **before** the first line of the replacement is written, and make differential comparison part of the suite. Prioritise by blast radius: anything that can block, delete, or halt gets its oracle frozen first. Where an oracle disagrees with *itself* across its own code paths, follow the fail-safe branch and pin the divergence with a control — losing one turn of enforcement is recoverable, wedging a session is not.
**Tags:** js-core, porting, parity, oracles, blocking-hooks, differential-testing
**Date:** 2026-08-13

### A control case that depends on an unset variable must delete it, not merely not set it
**Pattern:** The jq-half gate's "bare" spawn existed to prove that a run with a skipped jq half exits non-zero. `runCommand` builds its child env from `{...process.env}`, so on the one CI leg that runs the whole suite under `VIBE_NO_JQ=1`, the bare spawn inherited the opt-in and was not bare — it took the deliberately-skipped branch, exited 0, and failed its own assertion. The test's comment said isolation was handled "without touching this runner's own PATH": PATH was isolated, the environment never was. The same class had just been fixed one file over for `CLAUDE_PROJECT_DIR`.
**Rule:** A test whose meaning is "variable X is not set" must **delete** X from the child environment, not rely on the parent not having it — and the deletion belongs on the shared spawn helper so every call site can use it. Then run the suite under each ambient variable it reads, in both states, as its own leg. Related: run every CI leg locally, byte-exact, before pushing — the round that shipped this bug ran none of them, and the round that fixed it ran all ten.
**Tags:** js-core, hermeticity, test-isolation, environment, ci
**Date:** 2026-08-13

### A comment asserting a safety property becomes load-bearing — test it or delete it
**Pattern:** `install.sh` claimed "Source-only artifacts (co-located tests…) never ship" while scrubbing only `vibe/tests` and not `vibe/engine/tests`, so 8,313 lines of test code across 17 files landed in every user repo. The comment was not merely stale: the primitive scan's one exemption was *argued on that premise*, so a false comment was silently holding up a correctness argument. Across these rounds, four other comments were found asserting protections the code had just been proven not to provide — including two reason codes promising exactly the guarantee that had been demonstrated absent.
**Rule:** Treat a comment that asserts a safety or scope property as an untested claim, and either give it a test or delete it. When a fix proves a comment wrong, correct the comment in the same commit — a stale comment is worse than none, because the next reader (and the next reviewer's reasoning) will rely on it. Prefer assertions that are structural over lists of names: scrub every `tests/` directory at any depth and assert *no test artifact anywhere* reaches a target, rather than naming two directories that the next co-located suite will silently escape.
**Tags:** js-core, install, comments, false-invariants, packaging
**Date:** 2026-08-13

<!-- Format for each lesson:
### [Short description]
**Pattern:** What went wrong and why
**Rule:** The concrete rule that prevents this
**Tags:** comma, separated, keywords
**Date:** YYYY-MM-DD
-->
