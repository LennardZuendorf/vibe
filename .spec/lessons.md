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
**Superseded:** 2026-10-13 by D45 — feature design/plans now default to superpowers; the spec skill routes, it does not offer.

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
**Pinned-by:** final whole-branch review, spec v0.4 phase 1 (2026-10-13)
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
**Pinned-by:** final whole-branch review, spec v0.4 phase 1 (2026-10-13)
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

### Replace a table row by its table, never by its leading cell
**Pattern:** A batched doc edit replaced plan row `| F3 | …` by matching the line prefix; the same prefix opened a row in the issue catalog table, which was overwritten with the 7-column plan row — two issue links lost and the table malformed. The check scripts only asserted the new text existed, so they passed.
**Rule:** Anchor a row replacement to its table (header or section) and assert afterwards that every table kept its column count and that the old row text is gone only where intended. A post-edit check must also look for collateral damage, not only for the new text.
**Tags:** spec, docs, plan, tables, batch-edit
**Date:** 2026-10-13

### A new shell script runs the umbrella suite, not just its own
**Pattern:** The pressure-test harness passed shellcheck and its own honesty checks, then broke the flow suite's bash-3.2 portability lint (`"${arr[@]}"` on an empty array under `set -u`). The implementer and two reviewers ran only the spec checks; the defect surfaced two tasks later.
**Rule:** Any change that adds or edits a tracked `*.sh` file — test scripts included — runs `bash tests/run.sh` (all suites) and `git ls-files '*.sh' | grep -Ev '^oracles/(bash|js)/' | xargs shellcheck` (the CI command) before commit; a phase gate later found two SC2015 hits in `spec/tests/run.sh` the same way. Guard array expansions as `${arr[@]+"${arr[@]}"}` for stock macOS bash 3.2.
**Tags:** bash, portability, bash32, testing, spec
**Date:** 2026-10-13

### Run wall-time budgets on a quiet machine before calling a regression
**Pattern:** The engine's 150 ms performance-budget test failed repeatedly while several agents and suites ran in parallel; alone, the slowest command measured 85 ms and the suite went 674/674.
**Rule:** Before treating a wall-time failure as a regression, rerun the suite alone and compare against its logged control. Never run timing suites concurrently with other suites in a gate.
**Tags:** testing, performance, flaky, engine
**Date:** 2026-10-13

### Plan text must fit the tests that already constrain it
**Pattern:** Two plan steps specified exact text that the repo's existing tests rejected: compound orders 435 bytes against a 400-byte per-block budget, and a fallback line that did not contain the plan's own assert string.
**Rule:** When a plan dictates exact content, check it against every existing budget, pin and assert that covers that file before the plan is approved — and make the plan's own asserts match its own text.
**Tags:** planning, orders, testing, superpowers
**Date:** 2026-10-13

### Sweep for the meaning, not one spelling of it
**Pattern:** A fix swept `flow/` for the literal `.spec/features/` to find every line that still made the fallback the default. Lines saying "follow the spec skill feature.md steps 1–4" and injecting "the feature's `product.md` requirements" carried the same meaning without the string and survived until the final review.
**Rule:** When retiring a default, list every phrasing that expresses it (paths, file names, step references, template names) and grep for all of them; then pin the invariant with a test that fails on any unqualified reference, not just the one spelling.
**Tags:** flow, spec, superpowers, grep, review
**Date:** 2026-10-13

### A finding marked must-fix is fixed in its own task
**Pattern:** A task review found a stale safety comment, marked it "must-fix", and parked it as a deferred minor for the final wave. The final whole-branch review then reported it as the branch's only Critical.
**Rule:** A finding that a binding rule makes mandatory (stale safety comment, test that asserts nothing) enters the task's fix loop; only genuinely optional polish is deferred to the final review.
**Tags:** review, sdd, process, comments
**Date:** 2026-10-13

### A headless CLI inside a read loop eats the loop's input
**Pattern:** The pressure harness ran `claude -p` inside `while read … done <scenarios.tsv`. The headless CLI reads stdin when it is not a terminal, so the first session would have swallowed the remaining scenario rows and the loop would have ended after one scenario. The bug shipped through two reviews and was caught only before the first live run.
**Rule:** Every command that may read stdin, when called inside a `while read` loop, gets `</dev/null`. Live harnesses also keep each session's transcript and stderr on disk; a silenced stderr hides auth and rate-limit failures.
**Tags:** bash, harness, testing, pressure, spec
**Date:** 2026-10-14

### A skill description must name the small work it covers
**Pattern:** After the rewrite, the spec skill passed every design-shaped pressure scenario but failed "Quick fix: … Just fix it" with and without superpowers, plus "Add a --verbose flag" without them (GREEN 9/12). The agent read the file and wrote straight away. Naming one-line fixes, typos, new flags, and "quick"/"just fix it" in the description took the run to 12/12 twice.
**Rule:** The description is the only text an agent sees before choosing. It names the smallest work it must catch, in the user's words, and says to invoke before reading files. A body row (Routing, Red Flags) does not catch what the description misses.
**Tags:** skills, spec, triggers, pressure, superpowers
**Date:** 2026-10-14

### A compiled, passing plan is not a proven guard
**Pattern:** Every Phase 2 code block was compiled and tested in a scratch workspace before the plan was written, yet task reviews found guards that pass without guarding in six of eight tasks: a schema validator that ignored `$ref` siblings, contracts that did not pin required sets, a vocabulary list missing half the state names, a path-dependency branch no test reached, a harness `cwd` that escaped the sandbox, and a boundary test with no decoy to stop.
**Rule:** For every guard a plan adds, plant the defect it exists to catch and watch it fail before the plan ships. "It compiles and the tests pass" proves the code runs, not that the check discriminates.
**Tags:** testing, planning, review, guards, rust, contracts
**Date:** 2026-10-14

### A backtracking matcher needs a bound before it reaches a hook
**Pattern:** The hand-written glob matcher backtracked without memoization: `*a*a*a*a*b` against 200 characters took 4.9 s, and six `**/` segments took 6.8 s. It was meant to run on every path inside a hook.
**Rule:** Any recursive wildcard matcher memoizes failed (pattern, subject) positions, and a test pins a hostile pattern under a generous time bound.
**Tags:** rust, performance, glob, hooks
**Date:** 2026-10-14

### Derive a banned vocabulary from its source, never from memory
**Pattern:** The core crate's "no tool vocabulary" test listed `feature.` and `strategy.` but missed `quick.*`, `setup.*` and the `agents-md` channel.
**Rule:** A guard against a set of names reads the set from the file that defines it (state machine, channel config) and asserts a floor on how many it derived.
**Tags:** testing, guards, rust, flow
**Date:** 2026-10-14

### Floors and fixtures must hold at every stage of a plan
**Pattern:** A source-file floor of 5 failed at the stage that had 3 files, and fixture files named `*.sh` broke the repo-wide CI shellcheck once tracked. Both surfaced only when intermediate stages were built and files were staged.
**Rule:** Build each intermediate task state of a plan, and dry-run the whole phase with every new file staged, before handing the plan over. Fixture files that are not real scripts never use a script extension.
**Tags:** planning, testing, ci, shellcheck
**Date:** 2026-10-14

### The write guard suffix-matches nested spec paths
**Pattern:** Corpus files under `tests/parity/corpus/**/tree/.spec/plan.md` are blocked for Edit/Write in every flow state, because the policy rule matches the `.spec/plan.md` suffix.
**Rule:** Never hand-edit generated corpus trees; change the generator and regenerate. A real fix belongs to the flow policy (anchor root-spec rules at the repo root).
**Tags:** flow, policy, parity, spec
**Date:** 2026-10-14

### Control a red timing gate against a clean base before blaming load or the branch
**Pattern:** The engine perf budget failed (`doctor` 194–252 ms > 150 ms). It was blamed first on load, then on `target/`. A clean `origin/main` export failed identically; the cost is `validate-state.sh` (81 ms, ~10 `jq` spawns) under `doctor`, independent of the branch.
**Rule:** When a timing gate fails, profile the slow command's subprocesses and rerun the same gate on a clean base export before attributing the cause.
**Tags:** testing, performance, verification
**Date:** 2026-10-14

### Enumerate the oracle's shell semantics before porting a line
**Pattern:** Task reviews of the F16 ports found five parity defects, each in a shell construct the port had read as plain logic:
- glob `*.md` matching dirs and dangling links;
- the raw `$@` tail including a leading `--`;
- `basename "$(dirname …)"`;
- bash arithmetic in `-gt`;
- a feature name interpolated into a `grep -E` pattern.
**Rule:** Before porting a bash line, name each construct's exact semantics (glob, argv tail, arithmetic, regex interpolation, coreutils formatting). Add a corpus case for each one. Mimic it, or pin the divergence on purpose with a reason.
**Tags:** rust, parity, oracle, bash, review
**Date:** 2026-10-15

### Line-based Markdown scanners must skip fenced code
**Pattern:** The F16 `plan` query counted `- [ ]` and `## ` lines inside fenced examples. This repo's own plans hold fenced headings and checkboxes, so a fully ticked task would report `in-progress` forever. The plan text mandated the defect.
**Rule:** Any line scanner over Markdown tracks ``` and ~~~ fences and ignores structure inside them. Test it against this repo's real plans, not only synthetic fixtures.
**Tags:** markdown, parsing, plans, testing
**Date:** 2026-10-15

### Reuse the path-display helper instead of rejoining components
**Pattern:** The `plan`/`feature` JSON rebuilt paths by joining `Path::components()` with `/`. For a spec root outside the project, the `RootDir` component produced `//Users/...`. `Root::display` already handled that case.
**Rule:** Render a path through one shared helper that handles both the inside-project and the outside-project case. Test with a `--root` outside the project.
**Tags:** rust, paths, json, contracts
**Date:** 2026-10-15

### Sandbox git repos in tests inherit the user's signing config
**Pattern:** Five `inject-triggers/6` adapter asserts failed while the SSH signing agent was down. The test's throwaway `git commit` in its sandbox inherited global `commit.gpgsign=true`, failed silently (`>/dev/null 2>&1`), and left staged files that the `git status --porcelain` assertions then saw.
**Rule:** Every test that runs `git init` in a sandbox sets `commit.gpgsign false` (and `tag.gpgsign false`) in that repo next to `user.email`/`user.name`, and never discards a setup commit's exit status.
**Tags:** testing, git, sandbox, environment
**Date:** 2026-10-15

### Concurrent tests that share a deterministic scratch dir race under load
**Pattern:** `common::project(name)` reused one directory per name; parallel tests clobbered each other.
**Rule:** Give every test fixture a unique directory (process id plus an atomic counter).
**Tags:** rust, tests, sandbox
**Date:** 2026-10-15

### Checks that read git history must detect shallow clones
**Pattern:** SF29 and `trace` read a depth-1 CI checkout as full history: every file got the boundary commit's date, and every unit looked uncited.
**Rule:** Check `rev-parse --is-shallow-repository` first, and stay silent or say so.
**Tags:** spec, validate, git
**Date:** 2026-10-15

### Never split free text on separator bytes; use NUL
**Pattern:** A `%x1f`/`%x1e` git log format broke on commit messages that contained those bytes.
**Rule:** Use `git log -z`, because git refuses NUL in messages, and validate each record's header (hex sha).
**Tags:** git, rust, parsing
**Date:** 2026-10-15

### Give each file one owning check per problem
**Pattern:** In layered trees, legacy SF20/SF21 errored on files SF30 already reported, and SF26/SF33 repeated SF20.
**Rule:** When a new layout or check covers a file, route that file away from the old check. Test that a bad input yields exactly one finding.
**Tags:** spec, validate, layered
**Date:** 2026-10-15

### "Path resolves" means is_file inside the project
**Pattern:** SF36 accepted `#anchor` (empty path resolves to the project dir) and directories, because it used `.exists()`.
**Rule:** Strip fragments before the emptiness check, then require `is_file()` and canonical containment.
**Tags:** spec, validate, paths
**Date:** 2026-10-15

### Fixtures must have the target's real shape
**Pattern:** A one-part layered fixture hid SF32 false duplicates that fire on every multi-part tree (customer-portal's target).
**Rule:** Model fixtures on the real target, including repeated structures.
**Tags:** spec, validate, layered, tests
**Date:** 2026-10-15

### A git pathspec prunes merge and empty commits
**Pattern:** Scoping `trace`'s `git log` with `-- .` at the repo top silently dropped merge and empty commits (history simplification), so their unit citations vanished.
**Rule:** Pass no pathspec when the scope is the whole repository. When a real sub-path scope is needed, accept or document that merges are pruned.
**Tags:** git, trace, rust
**Date:** 2026-10-15

### Normalise the base path, not just the target
**Pattern:** The layered link resolver applied `..` to the link target but compared it against an unnormalised `--root docs/../docs/spec`, so valid absolute links read as outside the root.
**Rule:** Normalise both sides of a path comparison lexically before `strip_prefix`/`starts_with`. Test with a root given through `..`.
**Tags:** spec, validate, paths, layered
**Date:** 2026-10-15

### Compare Windows paths in their plain form
**Pattern:** Test helpers built expected paths with `fs::canonicalize`, which returns `\\?\C:\…` on Windows, while the binary printed `C:\…`; four `root` tests failed only on the Windows CI legs.
**Rule:** Strip the verbatim prefix in test helpers, then build expected paths one component per `join` (a verbatim base silently turned `/` into `\`; a plain one does not). Push early so the Windows legs run.
**Tags:** rust, tests, windows, paths
**Date:** 2026-10-16

### Shellcheck versions disagree; write for the strictest
**Pattern:** `out="$(cd X && cmd || true)"` passed local shellcheck 0.11.0 but CI's runner shellcheck flagged SC2015 (info) and failed the job before any suite ran.
**Rule:** Never write `A && B || C` in shell; put `|| true` outside the command substitution. Treat a CI shellcheck finding as authoritative over the local version.
**Tags:** shell, ci, shellcheck
**Date:** 2026-10-16

### A status edit is a spec edit: bump `updated:`
**Pattern:** Closing the F2–F17 rows changed `.spec/plan.md` without touching its `updated:` field, so `vibe-spec validate` gained an SF29 warning (updated older than the last commit).
**Rule:** Bump `updated:` in the same commit as any edit to a spec doc, status cells included. Run `vibe-spec validate --json` before committing a spec edit.
**Tags:** spec, compound, sf29
**Date:** 2026-10-16

### A glob port sorts by the glob's words, not bare names
**Pattern:** `text::sub_dirs` sorted bare names; the bash oracle's `features/*/` glob sorts `name/`, so `spec-delta` precedes `spec`.
**Rule:** When porting a glob, sort by the glob's expanded words (including the trailing `/`) and pin a prefix-pair case.
**Tags:** rust, parity, sorting
**Date:** 2026-10-16

### Skipping is failing open
**Pattern:** `walk_files` skipped unreadable dirs (the pre-merge gate passed) and symlinked entries (import dropped requirements, exit 0); `unwrap_or_default` on a read turned an unreadable target into empty content that the write then truncated (data loss).
**Rule:** A read or walk error that feeds a gate or a write must abort, never default or skip silently. Test each with `chmod 000` and a symlink.
**Tags:** rust, spec, safety, io
**Date:** 2026-10-17

### Plan every write in memory
**Pattern:** Multi-file writes were not all-or-nothing (a second write failure left the first written), and `--dry-run` planned against disk instead of the merged bytes.
**Rule:** Plan every write in memory, keep originals, restore on failure. Dry-run executes the same plan and gates only the writes.
**Tags:** rust, spec, promote, atomicity
**Date:** 2026-10-17

### Lstat every path component before a delete
**Pattern:** `promote --done` deleted through a symlinked artifact directory (leaf-only lstat).
**Rule:** Before an irreversible delete, lstat every path component from the project root down and refuse any symlink.
**Tags:** rust, spec, safety, symlink
**Date:** 2026-10-18

### Probe root-discovering commands in their own spec tree
**Pattern:** A reviewer probe ran `import` in a scratch dir without `.spec`; root discovery walked up and imported into the repo's real `.spec/`.
**Rule:** Probes of root-discovering commands create their own `.spec` in the scratch tree, or run outside the repo.
**Tags:** spec, tests, discovery
**Date:** 2026-10-18

### Grep for scripted edits before claiming them
**Pattern:** A fix-round edit silently failed to apply (the search text did not match the rustfmt-formatted file) and the round reported done; the re-review caught the missing test.
**Rule:** After a scripted edit, grep for the new content before claiming it landed.
**Tags:** tooling, review
**Date:** 2026-10-16

### Check the signing agent before a commit batch
**Pattern:** Commits were blocked when the SSH signing agent (Bitwarden) was locked.
**Rule:** Run `ssh-add -l` before a commit batch. On failure leave the work staged and report; never disable signing.
**Tags:** git, signing
**Date:** 2026-10-17

### Compare file identity, not spelling, before merging two writes
**Pattern:** `promote` decided that a merge target and a delta target were one file by comparing their path text. The same file under two paths (a symlinked directory) got two writes, and the second overwrote the first while the command reported success.
**Rule:** Compare file identity (device and inode, or canonical path) before merging two writes to one file; fall back to the lexical path only when a file is missing.
**Tags:** rust, spec, safety, symlink
**Date:** 2026-10-18

### Scope hook paths by project identity
**Pattern:** `validate --file` compared raw payload paths, so `..` and cwd-relative paths returned an empty result without an error.
**Rule:** Normalise every hook path through one identity function and make it project-relative before filtering.
**Tags:** rust, spec, hooks
**Date:** 2026-10-19

### Write runtime markers atomically, never through links
**Pattern:** The session marker was written through a symlinked `.vibe` and a symlinked `spec-session`, so it truncated or created files outside the project.
**Rule:** `lstat` the directory and the file, skip on a symlink, and write with temp file plus rename.
**Tags:** rust, safety, symlink
**Date:** 2026-10-19

### Give every hook a timeout
**Pattern:** The spec hook entries had no `timeout`, while the flow entries had 10.
**Rule:** Set `"timeout"` on every hook entry in `settings.json` and generated `hooks.json`, and assert it in a test.
**Tags:** hooks, tooling
**Date:** 2026-10-19

### Route agents through the engine
**Pattern:** The `spec-promoter` agent edited root docs by hand and never ran `vibe-spec promote`, which applies merge blocks and deltas.
**Rule:** An agent that owns a workflow runs the engine command and works only the checklist it prints.
**Tags:** agents, spec
**Date:** 2026-10-19

### Match ownership markers by exact line and restore multi-file writes
**Pattern:** `setup` matched its ownership marker as a substring, so a foreign file could be overwritten. Its multi-file writes had no restore. The hooks path from a subdirectory skipped the symlink walk.
**Rule:** Match the marker as a whole line, make multi-file writes all-or-nothing, and make the hooks path absolute before the symlink walk. Normalise lexically: `git rev-parse --path-format=absolute` resolves symlinks and hides a symlinked hooks dir.
**Tags:** rust, safety, git
**Date:** 2026-10-19

### Probe in a dir with its own `git init`
**Pattern:** A reviewer probe ran in a plain subdirectory inside the repo. Git resolved the enclosing repo and the probe installed a hook into the shared git dir.
**Rule:** Create probe dirs outside the repo tree and run `git init` in each before any git-aware command.
**Tags:** tests, git, process
**Date:** 2026-10-19
